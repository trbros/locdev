//! Runtime management: verified downloads of managed versions, detection of
//! versions already on the machine, and the PATH a project runs with.

mod detect;
pub mod download;
pub mod extract;
pub mod lifecycle;
mod node;
mod php;
pub mod php_ini;
pub mod terminal;
pub mod tools;
pub mod version;

use std::path::{Path, PathBuf};

use locdev_core::model::{JobState, Release, RuntimeKind, RuntimeSource, RuntimeVersion};

pub use detect::system;
pub use php::{FLAVORS as PHP_FLAVORS, PhpSource};

#[derive(Debug)]
pub enum RuntimeError {
    Unsupported,
    Network(String),
    Catalog(String),
    Checksum {
        file: String,
        expected: String,
        actual: String,
    },
    Archive(String),
    Io(std::io::Error),
}

impl RuntimeError {
    fn network(url: &str, error: impl std::fmt::Display) -> Self {
        Self::Network(format!("{url}: {error}"))
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(
                f,
                "LocDev can't download this runtime for this platform yet"
            ),
            Self::Network(e) => write!(f, "download failed: {e}"),
            Self::Catalog(e) => write!(f, "release catalog: {e}"),
            Self::Checksum {
                file,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{file} failed verification (expected sha256 {expected}, got {actual}); nothing was installed"
                )
            }
            Self::Archive(e) => write!(f, "archive: {e}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<std::io::Error> for RuntimeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// LocDev-managed runtimes, under `<root>/<kind>/<version>/`.
pub struct Store {
    root: PathBuf,
    /// Where PHP builds come from (StaticPHP / windows.php.net).
    php: PhpSource,
}

impl Store {
    pub fn new(root: PathBuf, php: PhpSource) -> Self {
        Self { root, php }
    }

    /// Whether LocDev can download verified builds of `kind` here.
    pub fn downloads(&self, kind: RuntimeKind) -> bool {
        match kind {
            RuntimeKind::Node => node::supported(),
            RuntimeKind::Php => php::supported(),
            other => tools::supported(other),
        }
    }

    pub fn catalog(&self, kind: RuntimeKind) -> Result<Vec<Release>, RuntimeError> {
        match kind {
            RuntimeKind::Node => node::catalog(),
            RuntimeKind::Php => php::catalog(&self.php),
            other if tools::supported(other) => tools::catalog(other),
            _ => Err(RuntimeError::Unsupported),
        }
    }

    /// What an installed package manager needs to run (`node >=18.12`).
    pub fn tool_meta(&self, kind: RuntimeKind, version: &str) -> tools::ToolMeta {
        tools::meta(&self.kind_dir(kind).join(version))
    }

    fn kind_dir(&self, kind: RuntimeKind) -> PathBuf {
        self.root.join(kind.executable())
    }

    fn executable(&self, kind: RuntimeKind, version: &str) -> PathBuf {
        let dir = self.kind_dir(kind).join(version);
        match kind {
            RuntimeKind::Node => node::executable(&dir),
            RuntimeKind::Php => php::executable(&dir),
            other => tools::executable(other, &dir),
        }
    }

    /// Installed managed versions, newest first.
    pub fn installed(&self, kind: RuntimeKind) -> Vec<RuntimeVersion> {
        let Ok(entries) = std::fs::read_dir(self.kind_dir(kind)) else {
            return Vec::new();
        };
        let mut found: Vec<RuntimeVersion> = entries
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.') && !version::parts(name).is_empty())
            .filter_map(|name| {
                let executable = self.executable(kind, &name);
                executable.is_file().then(|| RuntimeVersion {
                    version: name,
                    source: RuntimeSource::Managed,
                    executable,
                    origin: "locdev".into(),
                    support: None,
                })
            })
            .collect();
        found.sort_by(|a, b| version::compare(&b.version, &a.version));
        found
    }

    /// Downloads, verifies and installs `version`. Safe to interrupt: work
    /// happens in a scratch folder and lands with a single rename.
    pub fn install(
        &self,
        kind: RuntimeKind,
        version: &str,
        progress: &dyn Fn(JobState, u64, Option<u64>),
    ) -> Result<RuntimeVersion, RuntimeError> {
        self.install_logged(kind, version, progress, &|_| {})
    }

    /// Like [`Store::install`], describing each step through `log`.
    pub fn install_logged(
        &self,
        kind: RuntimeKind,
        version: &str,
        progress: &dyn Fn(JobState, u64, Option<u64>),
        log: &dyn Fn(String),
    ) -> Result<RuntimeVersion, RuntimeError> {
        let dest = self.kind_dir(kind).join(version);
        if !dest.exists() {
            let scratch = self.root.join(".tmp").join(format!(
                "{}-{version}-{}",
                kind.executable(),
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&scratch);
            std::fs::create_dir_all(&scratch)?;
            std::fs::create_dir_all(self.kind_dir(kind))?;

            let result = match kind {
                RuntimeKind::Node => node::install(version, &dest, &scratch, progress, log),
                RuntimeKind::Php => {
                    php::install(&self.php, version, &dest, &scratch, progress, log)
                }
                other if tools::supported(other) => {
                    tools::install(other, version, &dest, &scratch, progress, log)
                }
                _ => Err(RuntimeError::Unsupported),
            };
            let _ = std::fs::remove_dir_all(&scratch);
            result?;
            log(format!("installed in {}", dest.display()));
        } else {
            log(format!("already installed in {}", dest.display()));
        }

        progress(JobState::Done, 0, None);
        Ok(RuntimeVersion {
            version: version.to_string(),
            source: RuntimeSource::Managed,
            executable: self.executable(kind, version),
            origin: "locdev".into(),
            support: None,
        })
    }

    pub fn uninstall(&self, kind: RuntimeKind, version: &str) -> Result<(), RuntimeError> {
        if version::parts(version).is_empty() {
            return Err(RuntimeError::Archive(format!(
                "invalid version '{version}'"
            )));
        }
        let dir = self.kind_dir(kind).join(version);
        if dir.is_dir() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// The folder to put first on a project's PATH so `node`/`php` resolve
    /// to `runtime`. Executables with a version suffix (`php8.3`) get a
    /// one-entry shim folder so the plain name works. The shim is a script,
    /// not a symlink: multi-call tools pick the version from the name they
    /// were invoked as, which a symlink would change.
    pub fn path_entry(
        &self,
        kind: RuntimeKind,
        runtime: &RuntimeVersion,
    ) -> std::io::Result<PathBuf> {
        let exe = &runtime.executable;
        let parent = exe.parent().unwrap_or(Path::new("/")).to_path_buf();
        let plain = format!(
            "{}{}",
            kind.executable(),
            if cfg!(windows) { ".exe" } else { "" }
        );
        let name = exe.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        // Windows package managers are `pnpm.cmd`, `composer.bat`…: the plain
        // name already resolves through PATHEXT.
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        if name == plain || (cfg!(windows) && stem == kind.executable()) {
            return Ok(parent);
        }

        let shim =
            self.root
                .join("shims")
                .join(format!("{}-{}", kind.executable(), runtime.version));
        let link = shim.join(&plain);
        std::fs::create_dir_all(&shim)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let quoted = exe.display().to_string().replace('\'', "'\\''");
            let script = format!("#!/bin/sh\n# Managed by LocDev\nexec '{quoted}' \"$@\"\n");
            if std::fs::read_to_string(&link).ok().as_deref() != Some(script.as_str()) {
                std::fs::write(&link, script)?;
                std::fs::set_permissions(&link, std::fs::Permissions::from_mode(0o755))?;
            }
        }
        #[cfg(not(unix))]
        {
            // Windows installs ship `php.exe`/`node.exe`; nothing to shim.
            let _ = &link;
            return Ok(parent);
        }
        #[allow(unreachable_code)]
        Ok(shim)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn shims_versioned_executables() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("runtimes"), PhpSource::default());
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("php8.3"), "").unwrap();
        std::fs::write(bin.join("php"), "").unwrap();

        let versioned = RuntimeVersion {
            version: "8.3.14".into(),
            source: RuntimeSource::System,
            executable: bin.join("php8.3"),
            origin: "system".into(),
            support: None,
        };
        let entry = store.path_entry(RuntimeKind::Php, &versioned).unwrap();
        let shim = std::fs::read_to_string(entry.join("php")).unwrap();
        assert!(
            shim.contains(&format!("exec '{}' \"$@\"", bin.join("php8.3").display())),
            "{shim}"
        );

        let plain = RuntimeVersion {
            executable: bin.join("php"),
            ..versioned
        };
        assert_eq!(store.path_entry(RuntimeKind::Php, &plain).unwrap(), bin);
    }

    #[test]
    fn lists_only_complete_installs() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().to_path_buf(), PhpSource::default());
        std::fs::create_dir_all(dir.path().join("node/22.11.0/bin")).unwrap();
        std::fs::write(dir.path().join("node/22.11.0/bin/node"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("node/20.1.0")).unwrap(); // half-installed
        std::fs::create_dir_all(dir.path().join("node/.tmp")).unwrap();

        let installed = store.installed(RuntimeKind::Node);
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].version, "22.11.0");
    }
}
