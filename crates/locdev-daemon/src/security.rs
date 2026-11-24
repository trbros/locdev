//! The optional LocDev password that guards destructive Git operations.
//!
//! Only an Argon2id hash is stored (`security.toml`, readable by this user
//! only), never the password. Wrong passwords lock further attempts with a
//! growing delay, persisted so restarting the daemon doesn't reset it. The
//! password is checked on each request that needs it and never kept.
//!
//! It protects operations started *through LocDev*: Git itself, in any
//! terminal, is not affected.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use locdev_core::git::SecurityStatus;
use locdev_core::model::now_ms;
use locdev_core::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};

use crate::fsutil::write_atomic;

const MIN_LENGTH: usize = 8;
/// Wrong attempts allowed before the lockout starts.
const FREE_ATTEMPTS: u32 = 5;
const LOCK_BASE_MS: u64 = 60_000;
const LOCK_MAX_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct SecurityFile {
    version: u32,
    /// PHC string of the Argon2id hash.
    hash: Option<String>,
    require_password: bool,
    backups: bool,
    failures: u32,
    locked_until: Option<u64>,
}

impl Default for SecurityFile {
    fn default() -> Self {
        Self {
            version: 1,
            hash: None,
            require_password: true,
            backups: true,
            failures: 0,
            locked_until: None,
        }
    }
}

pub struct Security {
    path: PathBuf,
    file: Mutex<SecurityFile>,
}

impl Security {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let file = match std::fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).map_err(|e| {
                AppError::new(ErrorCode::Config, format!("{}: {e}", path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => SecurityFile::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path: path.to_path_buf(),
            file: Mutex::new(file),
        })
    }

    fn save(&self, file: &SecurityFile) -> Result<(), AppError> {
        let raw = toml::to_string_pretty(file).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.path, raw.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn status(&self) -> SecurityStatus {
        let file = self.file.lock().unwrap();
        SecurityStatus {
            password_set: file.hash.is_some(),
            require_password: file.require_password,
            backups: file.backups,
            locked_until: file.locked_until.filter(|t| *t > now_ms()),
        }
    }

    /// Whether destructive operations need the password right now.
    pub fn password_required(&self) -> bool {
        let file = self.file.lock().unwrap();
        file.hash.is_some() && file.require_password
    }

    pub fn backups(&self) -> bool {
        self.file.lock().unwrap().backups
    }

    /// Checks `password` against the stored hash, counting failures.
    pub fn verify(&self, password: Option<&str>) -> Result<(), AppError> {
        let mut file = self.file.lock().unwrap();
        let Some(hash) = file.hash.clone() else {
            return Ok(());
        };
        let now = now_ms();
        if let Some(until) = file.locked_until.filter(|t| *t > now) {
            return Err(AppError::new(
                ErrorCode::Unauthorized,
                format!(
                    "too many wrong passwords; try again in {}s",
                    (until - now).div_ceil(1000)
                ),
            ));
        }
        let Some(password) = password.filter(|p| !p.is_empty()) else {
            return Err(AppError::new(
                ErrorCode::Unauthorized,
                "this operation needs your LocDev password",
            ));
        };
        let ok = PasswordHash::new(&hash)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false);
        if ok {
            file.failures = 0;
            file.locked_until = None;
            self.save(&file)?;
            return Ok(());
        }
        file.failures += 1;
        if file.failures >= FREE_ATTEMPTS {
            let extra = file.failures - FREE_ATTEMPTS;
            let wait = (LOCK_BASE_MS << extra.min(10)).min(LOCK_MAX_MS);
            file.locked_until = Some(now + wait);
        }
        self.save(&file)?;
        Err(AppError::new(ErrorCode::Unauthorized, "wrong password"))
    }

    /// Sets, changes (`current` required) or removes (`new` = None) the password.
    pub fn set_password(&self, current: Option<&str>, new: Option<&str>) -> Result<(), AppError> {
        if self.file.lock().unwrap().hash.is_some() {
            self.verify(current)?;
        }
        let hash = match new {
            Some(new) => {
                if new.chars().count() < MIN_LENGTH {
                    return Err(AppError::invalid(format!(
                        "use at least {MIN_LENGTH} characters"
                    )));
                }
                // A random salt comes from the OS (argon2's getrandom feature).
                Some(
                    Argon2::default()
                        .hash_password(new.as_bytes())
                        .map_err(|e| {
                            AppError::internal(format!("could not hash the password: {e}"))
                        })?
                        .to_string(),
                )
            }
            None => None,
        };
        let mut file = self.file.lock().unwrap();
        file.hash = hash;
        file.failures = 0;
        file.locked_until = None;
        self.save(&file)
    }

    /// Changing the options needs the password when one is set, so the
    /// protection can't be turned off around it.
    pub fn set_options(
        &self,
        password: Option<&str>,
        require_password: bool,
        backups: bool,
    ) -> Result<(), AppError> {
        self.verify(password)?;
        let mut file = self.file.lock().unwrap();
        file.require_password = require_password;
        file.backups = backups;
        self.save(&file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn security() -> (tempfile::TempDir, Security) {
        let dir = tempfile::tempdir().unwrap();
        let s = Security::load(&dir.path().join("security.toml")).unwrap();
        (dir, s)
    }

    #[test]
    fn stores_only_an_argon2id_hash() {
        let (dir, s) = security();
        s.set_password(None, Some("correct horse")).unwrap();
        let raw = std::fs::read_to_string(dir.path().join("security.toml")).unwrap();
        assert!(raw.contains("$argon2id$"), "{raw}");
        assert!(!raw.contains("correct horse"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("security.toml"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(s.verify(Some("correct horse")).is_ok());
        assert_eq!(
            s.verify(Some("wrong")).unwrap_err().code,
            ErrorCode::Unauthorized
        );
        assert!(s.verify(None).is_err());
    }

    #[test]
    fn locks_out_after_repeated_failures_across_restarts() {
        let (dir, s) = security();
        s.set_password(None, Some("correct horse")).unwrap();
        for _ in 0..FREE_ATTEMPTS {
            assert!(s.verify(Some("nope")).is_err());
        }
        let reloaded = Security::load(&dir.path().join("security.toml")).unwrap();
        let error = reloaded.verify(Some("correct horse")).unwrap_err();
        assert!(
            error.message.contains("too many wrong passwords"),
            "{}",
            error.message
        );
        assert!(reloaded.status().locked_until.is_some());
    }

    #[test]
    fn changing_or_removing_needs_the_current_password() {
        let (_dir, s) = security();
        assert!(s.set_password(None, Some("short")).is_err());
        s.set_password(None, Some("first password")).unwrap();
        assert!(
            s.set_password(Some("wrong"), Some("second password"))
                .is_err()
        );
        assert!(
            s.set_options(None, false, true).is_err(),
            "can't turn protection off around it"
        );
        s.set_password(Some("first password"), Some("second password"))
            .unwrap();
        assert!(s.verify(Some("second password")).is_ok());
        s.set_password(Some("second password"), None).unwrap();
        assert!(!s.status().password_set);
        assert!(s.verify(None).is_ok(), "no password, nothing to check");
    }
}
