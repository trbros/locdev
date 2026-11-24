//! Settings the app and the CLI change at runtime (`settings.toml`):
//! automation, notifications, network. `config.toml` stays the file for
//! what is read once at startup (ports, listeners).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use locdev_core::deps::Settings;
use locdev_core::{AppError, ErrorCode};

use crate::fsutil::write_atomic;

pub struct SettingsFile {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsFile {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let current = match std::fs::read_to_string(path) {
            Ok(raw) => toml::from_str::<Settings>(&raw).map_err(|e| {
                AppError::new(ErrorCode::Config, format!("{}: {e}", path.display()))
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path: path.to_path_buf(),
            current: Mutex::new(current),
        })
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    /// Validates and saves; returns the previous settings.
    pub fn set(&self, next: Settings) -> Result<Settings, AppError> {
        if !(1..=8).contains(&next.automation.concurrency) {
            return Err(AppError::invalid(
                "automation.concurrency must be between 1 and 8",
            ));
        }
        if !(1..=50).contains(&next.automation.requests_per_second) {
            return Err(AppError::invalid(
                "automation.requests_per_second must be between 1 and 50",
            ));
        }
        let raw = toml::to_string_pretty(&next).map_err(|e| AppError::internal(e.to_string()))?;
        write_atomic(&self.path, raw.as_bytes())?;
        Ok(std::mem::replace(&mut *self.current.lock().unwrap(), next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::deps::AuditInterval;

    #[test]
    fn round_trip_and_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let file = SettingsFile::load(&path).unwrap();
        assert_eq!(file.get(), Settings::default());

        let mut next = file.get();
        next.automation.audit = AuditInterval::Weekly;
        next.network.offline = true;
        file.set(next.clone()).unwrap();
        assert_eq!(SettingsFile::load(&path).unwrap().get(), next);

        let mut bad = next.clone();
        bad.automation.concurrency = 0;
        assert!(file.set(bad).is_err());
        assert_eq!(
            file.get(),
            next,
            "a rejected change leaves the settings alone"
        );
    }
}
