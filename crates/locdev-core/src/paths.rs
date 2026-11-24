use std::path::{Path, PathBuf};

use crate::AppError;

/// `\\?\C:\dir` (what `canonicalize` returns on Windows) as `C:\dir`: Git,
/// `cmd` and most tools don't take the verbatim form. UNC and other verbatim
/// paths stay as they are.
pub fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

#[cfg(test)]
#[test]
fn plain_strips_only_drive_verbatim_prefixes() {
    assert_eq!(
        plain(PathBuf::from(r"\\?\C:\src\shop")),
        PathBuf::from(r"C:\src\shop")
    );
    assert_eq!(
        plain(PathBuf::from(r"\\?\UNC\host\share")),
        PathBuf::from(r"\\?\UNC\host\share")
    );
    assert_eq!(
        plain(PathBuf::from("/home/dev/shop")),
        PathBuf::from("/home/dev/shop")
    );
}

/// Environment variable that relocates every LocDev directory under one root.
/// Useful for tests and for running several isolated daemons side by side.
pub const HOME_ENV: &str = "LOCDEV_HOME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub home: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self, AppError> {
        if let Some(home) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Ok(Self::under(PathBuf::from(home)));
        }

        let dirs = directories::ProjectDirs::from("dev", "locdev", "locdev")
            .ok_or_else(|| AppError::internal("could not determine the user's home directory"))?;

        let data_dir = dirs.data_local_dir().to_path_buf();
        let runtime_dir = dirs
            .runtime_dir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| data_dir.join("run"));

        Ok(Self {
            home: dirs.config_dir().to_path_buf(),
            config_dir: dirs.config_dir().to_path_buf(),
            log_dir: data_dir.join("logs"),
            runtime_dir,
            data_dir,
        })
    }

    pub fn under(home: PathBuf) -> Self {
        Self {
            config_dir: home.join("config"),
            data_dir: home.join("data"),
            runtime_dir: home.join("run"),
            log_dir: home.join("logs"),
            home,
        }
    }

    /// Creates every directory, restricting the runtime directory (which holds
    /// the IPC socket) to the current user.
    pub fn ensure(&self) -> Result<(), AppError> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.runtime_dir,
            &self.log_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.runtime_dir, std::fs::Permissions::from_mode(0o700))?;
        }

        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn registry_file(&self) -> PathBuf {
        self.config_dir.join("projects.toml")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.runtime_dir.join("locdevd.pid")
    }

    pub fn daemon_log_file(&self) -> PathBuf {
        self.log_dir.join("locdevd.log")
    }
}
