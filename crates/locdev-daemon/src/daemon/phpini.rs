//! PHP settings per version line (`locdev php`, Runtimes → PHP). LocDev's
//! own ini file (`config/php/<line>/90-locdev.ini`) is added to the sites
//! it runs through PHP_INI_SCAN_DIR; a php.ini LocDev didn't write is only
//! ever read. A change is backed up, applied, checked against what PHP
//! actually loads (rolled back if PHP refuses it) and picked up by
//! restarting only the running sites on that line.

use std::path::{Path, PathBuf};

use locdev_core::model::{ProcessRole, Project, RuntimeKind, now_ms};
use locdev_core::phpini::{PhpConfigView, PhpIniBackup, PhpIniChange, PhpIniPreview};
use locdev_core::protocol::{PhpLine, Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_runtimes::php_ini::{self, FILE_NAME, Probe};
use tracing::{info, warn};

use super::Daemon;
use crate::fsutil::write_atomic;

/// Backups kept per line.
const HISTORY: usize = 30;

/// `8.3.14` → `8.3`.
fn line_of(version: &str) -> String {
    version.split('.').take(2).collect::<Vec<_>>().join(".")
}

fn check_line(line: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = line.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(AppError::invalid(format!(
            "'{line}' is not a PHP version line like 8.3"
        )))
    }
}

/// The inherited PHP_INI_SCAN_DIR (the daemon's own environment).
fn inherited_scan_dir() -> Option<String> {
    std::env::var("PHP_INI_SCAN_DIR")
        .ok()
        .filter(|v| !v.is_empty())
}

impl Daemon {
    fn php_dir(&self, line: &str) -> PathBuf {
        self.paths.config_dir.join("php").join(line)
    }

    fn php_history_dir(&self, line: &str) -> PathBuf {
        self.paths
            .data_dir
            .join("state")
            .join("php-ini-history")
            .join(line)
    }

    /// One installation per line: the first one sites would resolve to.
    fn php_lines(&self) -> Vec<PhpLine> {
        let mut lines: Vec<PhpLine> = Vec::new();
        for v in self.runtimes.versions(RuntimeKind::Php) {
            let line = line_of(&v.version);
            if lines.iter().any(|l| l.line == line) {
                continue;
            }
            let configured = std::fs::read_to_string(self.php_dir(&line).join(FILE_NAME))
                .is_ok_and(|t| !php_ini::parse(&t).is_empty());
            lines.push(PhpLine {
                line,
                version: v.version,
                executable: v.executable,
                configured,
            });
        }
        lines
    }

    fn php_line(&self, line: &str) -> Result<PhpLine, AppError> {
        check_line(line)?;
        self.php_lines()
            .into_iter()
            .find(|l| l.line == line)
            .ok_or_else(|| AppError::not_found(format!("no PHP {line} installed")))
    }

    /// The PHP line a project runs with, when PHP resolves to a version.
    fn php_line_of(&self, project: &Project) -> Option<String> {
        self.runtimes
            .for_project(project)
            .into_iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .and_then(|r| r.version)
            .map(|v| line_of(&v.version))
    }

    /// Adds LocDev's settings for the project's PHP line to a process's
    /// environment, after whatever PHP_INI_SCAN_DIR already holds (PHP's
    /// default folder, the Mailbox's fragment).
    pub(super) fn php_ini_env(&self, project: &Project, env: &mut Vec<(String, String)>) {
        let Some(line) = self.php_line_of(project) else {
            return;
        };
        let dir = self.php_dir(&line);
        if !dir.join(FILE_NAME).is_file() {
            return;
        }
        let existing = env
            .iter()
            .rev()
            .find(|(k, _)| k == "PHP_INI_SCAN_DIR")
            .map(|(_, v)| v.clone())
            .or_else(inherited_scan_dir);
        env.retain(|(k, _)| k != "PHP_INI_SCAN_DIR");
        env.push((
            "PHP_INI_SCAN_DIR".into(),
            php_ini::scan_dir_with(existing.as_deref(), &dir),
        ));
    }

    /// Running sites on a line.
    async fn php_sites(&self, line: &str) -> Vec<Project> {
        let projects = self.registry.lock().await.all().to_vec();
        projects
            .into_iter()
            .filter(|p| self.supervisor.is_active(&ProcessRole::Main.id(&p.name)))
            .filter(|p| self.php_line_of(p).as_deref() == Some(line))
            .collect()
    }

    fn php_history(&self, line: &str) -> Vec<PhpIniBackup> {
        let mut entries: Vec<PhpIniBackup> = std::fs::read_dir(self.php_history_dir(line))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let id = name.strip_suffix(".ini")?.to_string();
                let at = id.parse().ok()?;
                let lines = std::fs::read_to_string(e.path()).ok()?.lines().count();
                Some(PhpIniBackup { id, at, lines })
            })
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.at));
        entries
    }

    fn php_backup(&self, line: &str, text: &str) -> Result<(), AppError> {
        let dir = self.php_history_dir(line);
        std::fs::create_dir_all(&dir)?;
        write_atomic(&dir.join(format!("{}.ini", now_ms())), text.as_bytes())?;
        for old in self.php_history(line).into_iter().skip(HISTORY) {
            let _ = std::fs::remove_file(dir.join(format!("{}.ini", old.id)));
        }
        Ok(())
    }

    /// Probes PHP without and with LocDev's folder. Blocking.
    fn php_probe(php: &Path, dir: &Path) -> Result<(Probe, Probe), String> {
        let inherited = inherited_scan_dir();
        let original = php_ini::probe(php, inherited.as_deref())?;
        let effective = if dir.join(FILE_NAME).is_file() {
            php_ini::probe(
                php,
                Some(&php_ini::scan_dir_with(inherited.as_deref(), dir)),
            )?
        } else {
            original.clone()
        };
        Ok((original, effective))
    }

    async fn php_view(&self, line: &str) -> Result<PhpConfigView, AppError> {
        let php = self.php_line(line)?;
        let dir = self.php_dir(line);
        let file = dir.join(FILE_NAME);
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let exe = php.executable.clone();
        let probe_dir = dir.clone();
        let (original, effective) =
            tokio::task::spawn_blocking(move || Self::php_probe(&exe, &probe_dir))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
                .map_err(|e| AppError::new(ErrorCode::NotRunnable, e))?;
        let mut warnings = php_ini::warnings(&effective, &text);
        warnings.extend(php_ini::not_applied(&effective, &text));
        // LocDev's own file isn't one of "PHP's" files in the view.
        let scanned = original.scanned_files();
        Ok(PhpConfigView {
            line: line.to_string(),
            version: effective.version.clone(),
            executable: php.executable,
            sapi: effective.sapi.clone(),
            loaded_ini: original.loaded.clone().map(PathBuf::from),
            scanned,
            file,
            hash: php_ini::hash(&text),
            directives: php_ini::directives(&original, &effective, &text),
            text,
            warnings,
            sites: self
                .php_sites(line)
                .await
                .into_iter()
                .map(|p| p.name)
                .collect(),
            history: self.php_history(line),
        })
    }

    /// The file a change produces (`None`: no file, PHP's own values).
    fn php_next_text(
        &self,
        line: &str,
        current: &str,
        change: &PhpIniChange,
    ) -> Result<Option<String>, AppError> {
        match change {
            PhpIniChange::Set { values } => {
                let text = php_ini::apply(current, values).map_err(AppError::invalid)?;
                Ok(Some(text))
            }
            PhpIniChange::Text { text, hash } => {
                if *hash != php_ini::hash(current) {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "LocDev's file changed since it was opened; reload it and try again",
                    ));
                }
                Ok(Some(text.clone()))
            }
            PhpIniChange::Restore { id } => {
                if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
                    return Err(AppError::invalid("not a saved version"));
                }
                let path = self.php_history_dir(line).join(format!("{id}.ini"));
                std::fs::read_to_string(&path)
                    .map(Some)
                    .map_err(|_| AppError::not_found(format!("no saved version {id}")))
            }
            PhpIniChange::Reset => Ok(None),
        }
    }

    /// Checks a candidate file with PHP itself, in a scratch folder: syntax
    /// errors and values PHP doesn't take come back as an error.
    async fn php_check(&self, php: &Path, text: &str) -> Result<Vec<String>, AppError> {
        let scratch = tempfile::tempdir()?;
        std::fs::write(scratch.path().join(FILE_NAME), text)?;
        let exe = php.to_path_buf();
        let dir = scratch.path().to_path_buf();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || {
            let inherited = inherited_scan_dir();
            let effective = php_ini::probe(
                &exe,
                Some(&php_ini::scan_dir_with(inherited.as_deref(), &dir)),
            )?;
            let refused = php_ini::not_applied(&effective, &text);
            if !refused.is_empty() {
                return Err(format!("PHP didn't take it: {}", refused.join("; ")));
            }
            Ok(php_ini::warnings(&effective, &text))
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(AppError::invalid)
    }

    pub(super) async fn php_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::PhpConfigLines => Ok(Response::PhpLines {
                lines: self.php_lines(),
            }),
            Request::PhpConfig { line } => Ok(Response::PhpConfig {
                config: self.php_view(&line).await?,
                preview: None,
                restarted: Vec::new(),
            }),
            Request::PhpIniRead { line, path } => {
                let view = self.php_view(&line).await?;
                let allowed = view.loaded_ini.as_ref() == Some(&path)
                    || view.scanned.contains(&path)
                    || path == view.file;
                if !allowed {
                    return Err(AppError::invalid(format!(
                        "{} is not a file PHP {line} loads",
                        path.display()
                    )));
                }
                let bytes = std::fs::read(&path)?;
                if bytes.len() > 1 << 20 {
                    return Err(AppError::invalid("the file is over 1 MB"));
                }
                Ok(Response::PhpIniFile {
                    path,
                    text: String::from_utf8_lossy(&bytes).into_owned(),
                })
            }
            Request::PhpConfigChange {
                line,
                change,
                dry_run,
            } => self.php_change(&line, &change, dry_run).await,
            _ => unreachable!("not a php request"),
        }
    }

    async fn php_change(
        &self,
        line: &str,
        change: &PhpIniChange,
        dry_run: bool,
    ) -> Result<Response, AppError> {
        let php = self.php_line(line)?;
        let file = self.php_dir(line).join(FILE_NAME);
        let current = std::fs::read_to_string(&file).unwrap_or_default();
        let next = self.php_next_text(line, &current, change)?;
        let warnings = match &next {
            Some(text) => self.php_check(&php.executable, text).await?,
            None => Vec::new(),
        };
        let sites = self.php_sites(line).await;
        if dry_run {
            return Ok(Response::PhpConfig {
                config: self.php_view(line).await?,
                preview: Some(PhpIniPreview {
                    before: current,
                    after: next.unwrap_or_default(),
                    restart: sites.into_iter().map(|p| p.name).collect(),
                    warnings,
                }),
                restarted: Vec::new(),
            });
        }

        if !current.is_empty() {
            self.php_backup(line, &current)?;
        }
        let write = |text: Option<&str>| -> Result<(), AppError> {
            match text {
                Some(text) => {
                    std::fs::create_dir_all(file.parent().unwrap())?;
                    write_atomic(&file, text.as_bytes())
                }
                None => match std::fs::remove_file(&file) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
                    _ => Ok(()),
                },
            }
        };
        write(next.as_deref())?;
        // What PHP loads now; anything off puts the previous file back.
        let check = match &next {
            Some(text) => {
                let exe = php.executable.clone();
                let dir = file.parent().unwrap().to_path_buf();
                let text = text.clone();
                tokio::task::spawn_blocking(move || {
                    let (_, effective) = Self::php_probe(&exe, &dir)?;
                    let refused = php_ini::not_applied(&effective, &text);
                    if refused.is_empty() {
                        Ok(())
                    } else {
                        Err(refused.join("; "))
                    }
                })
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
            }
            None => Ok(()),
        };
        if let Err(error) = check {
            let previous = (!current.is_empty()).then_some(current.as_str());
            if let Err(restore) = write(previous) {
                warn!(%restore, "could not restore the previous PHP settings");
            }
            return Err(AppError::invalid(format!(
                "PHP didn't load the new settings ({error}); the previous file was put back"
            )));
        }
        info!(line, "PHP settings changed");

        let mut restarted = Vec::new();
        for site in sites {
            match self.restart(&site).await {
                Ok(()) => restarted.push(site.name),
                Err(error) => {
                    warn!(%error, site = %site.name, "could not restart for new PHP settings")
                }
            }
        }
        Ok(Response::PhpConfig {
            config: self.php_view(line).await?,
            preview: None,
            restarted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        assert_eq!(line_of("8.3.14"), "8.3");
        assert_eq!(line_of("8.4"), "8.4");
        assert!(check_line("8.3").is_ok());
        assert!(check_line("../8").is_err());
        assert!(check_line("8.3.1").is_err());
        assert!(check_line("8").is_err());
    }
}
