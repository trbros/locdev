//! Databases and caches LocDev runs for projects: installations (found on
//! the system or downloaded and verified), instances with their own data,
//! port, config and logs, and cold backups. The daemon supervises the
//! processes; this crate only prepares and describes them.

pub mod packages;
pub mod releases;
pub mod spec;

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use locdev_core::services::{InstallSource, Instance, InstanceBackup, ServiceInstall, ServiceKind};
use serde::{Deserialize, Serialize};

pub use spec::Dirs;

/// Where everything lives: `bin/<kind>/<version>` for downloads,
/// `instances/<name>` for data, `backups/<name>` for copies.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn install_dir(&self, kind: ServiceKind, version: &str) -> PathBuf {
        self.root.join("bin").join(kind.id()).join(version)
    }

    pub fn dirs(&self, instance: &str) -> Dirs {
        Dirs::new(self.root.join("instances").join(instance))
    }

    pub fn backups(&self, instance: &str) -> PathBuf {
        self.root.join("backups").join(instance)
    }
}

/// Installations on this machine, LocDev's own first. System ones are
/// read, never changed.
pub fn discover(layout: &Layout) -> Vec<ServiceInstall> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for kind in ServiceKind::ALL {
        if kind == ServiceKind::Sqlite {
            continue;
        }
        let managed = layout.root.join("bin").join(kind.id());
        for dir in read_dirs(&managed) {
            let bin = dir.join("bin");
            if let Some(version) = spec::version_of(kind, &bin) {
                seen.insert(bin.clone());
                out.push(ServiceInstall {
                    kind,
                    version,
                    source: InstallSource::Managed,
                    bin_dir: bin,
                });
            }
        }
        for bin in system_dirs(kind) {
            let canonical = std::fs::canonicalize(&bin).unwrap_or(bin.clone());
            if !seen.insert(canonical) {
                continue;
            }
            if let Some(version) = spec::version_of(kind, &bin) {
                out.push(ServiceInstall {
                    kind,
                    version,
                    source: InstallSource::System,
                    bin_dir: bin,
                });
            }
        }
    }
    out
}

fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

/// Folders where the system keeps a service's server program.
fn system_dirs(kind: ServiceKind) -> Vec<PathBuf> {
    let (server, _) = spec::programs(kind);
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    // Distributions keep some servers off PATH.
    dirs.extend(
        [
            "/usr/sbin",
            "/usr/local/sbin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
        ]
        .map(PathBuf::from),
    );
    // Homebrew's versioned formulae (`mysql@8.4`, `postgresql@16`) are keg-only.
    if let Some(prefix) = packages::brew_prefix(kind) {
        for root in [
            "/opt/homebrew/opt",
            "/usr/local/opt",
            "/home/linuxbrew/.linuxbrew/opt",
        ] {
            dirs.extend(
                read_dirs(Path::new(root))
                    .into_iter()
                    .filter(|d| {
                        d.file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
                    })
                    .map(|d| d.join("bin")),
            );
        }
    }
    match kind {
        ServiceKind::Postgres => {
            dirs.extend(
                read_dirs(Path::new("/usr/lib/postgresql"))
                    .into_iter()
                    .map(|d| d.join("bin")),
            );
            dirs.extend(
                read_dirs(Path::new("C:\\Program Files\\PostgreSQL"))
                    .into_iter()
                    .map(|d| d.join("bin")),
            );
        }
        ServiceKind::Mysql | ServiceKind::Mariadb => {
            dirs.extend(
                read_dirs(Path::new("C:\\Program Files\\MySQL"))
                    .into_iter()
                    .map(|d| d.join("bin")),
            );
            dirs.extend(
                read_dirs(Path::new("C:\\Program Files"))
                    .into_iter()
                    .filter(|d| d.to_string_lossy().contains("MariaDB"))
                    .map(|d| d.join("bin")),
            );
        }
        _ => {}
    }
    let mut out = Vec::new();
    for dir in dirs {
        if spec::find(&dir, server).is_some() && !out.contains(&dir) {
            out.push(dir);
        }
    }
    out
}

/// Downloads, verifies and unpacks a build into `layout`. Blocking.
pub fn install(
    layout: &Layout,
    build: &releases::Build,
    progress: &dyn Fn(u64, Option<u64>),
) -> Result<ServiceInstall, String> {
    let kind = build.release.kind;
    let target = layout.install_dir(kind, &build.release.version);
    if target.join("bin").is_dir() {
        return Err(format!(
            "{} {} is already installed",
            kind.label(),
            build.release.version
        ));
    }
    let staging =
        layout
            .root
            .join("bin")
            .join(format!(".{}-{}.partial", kind.id(), build.release.version));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let archive = staging.join(if build.zip {
        "download.zip"
    } else {
        "download.tar.gz"
    });
    let sha = locdev_runtimes::download::download(&build.release.url, &archive, progress)
        .map_err(|e| e.to_string())?;
    if sha != build.sha256 {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!(
            "the download's SHA-256 ({sha}) doesn't match the publisher's ({}); nothing was installed",
            build.sha256
        ));
    }
    let unpacked = staging.join("files");
    let result = if build.zip {
        locdev_runtimes::extract::zip(&archive, &unpacked, true)
    } else {
        locdev_runtimes::extract::tar_gz(&archive, &unpacked)
    };
    result.map_err(|e| e.to_string())?;
    let version = spec::version_of(kind, &unpacked.join("bin"))
        .ok_or_else(|| format!("the archive has no working {}", spec::programs(kind).0))?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&unpacked, &target).map_err(|e| e.to_string())?;
    let marker = serde_json::json!({ "kind": kind, "version": version, "url": build.release.url, "sha256": sha, "publisher": build.release.publisher });
    let _ = std::fs::write(
        target.join(".locdev-install.json"),
        serde_json::to_vec_pretty(&marker).unwrap_or_default(),
    );
    let _ = std::fs::remove_dir_all(&staging);
    Ok(ServiceInstall {
        kind,
        version: build.release.version.clone(),
        source: InstallSource::Managed,
        bin_dir: target.join("bin"),
    })
}

/// The instances, kept in `services.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Instances {
    #[serde(default, rename = "instance")]
    pub list: Vec<Instance>,
}

impl Instances {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(
            &tmp,
            format!("# Service instances (Managed by LocDev)\n\n{text}"),
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }

    pub fn get(&self, name: &str) -> Option<&Instance> {
        self.list.iter().find(|i| i.name == name)
    }
}

pub fn check_instance_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{name}` is not a valid instance name (lowercase letters, digits and -)"
        ))
    }
}

/// Creates an instance's folders and data directory. Blocking. Never
/// touches an existing data directory.
pub fn init(kind: ServiceKind, bin_dir: &Path, dirs: &Dirs, port: u16) -> Result<(), String> {
    if dirs.data.exists()
        && std::fs::read_dir(&dirs.data)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err(format!(
            "{} already has data; LocDev won't initialize over it",
            dirs.data.display()
        ));
    }
    for dir in [&dirs.root, &dirs.run, &dirs.conf] {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dirs.root, std::fs::Permissions::from_mode(0o700));
    }
    for mut cmd in spec::init_commands(kind, bin_dir, dirs, port)? {
        let output = cmd
            .output()
            .map_err(|e| format!("could not run {:?}: {e}", cmd.get_program()))?;
        if !output.status.success() {
            let text = String::from_utf8_lossy(&output.stderr).to_string()
                + &String::from_utf8_lossy(&output.stdout);
            let tail: Vec<&str> = text.lines().rev().take(8).collect();
            return Err(format!(
                "{} failed: {}",
                Path::new(cmd.get_program())
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            ));
        }
    }
    Ok(())
}

/// Accepts TCP connections on 127.0.0.1:`port`.
pub fn ready(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

/// A cold copy of the data directory (the instance must be stopped).
pub fn backup(layout: &Layout, name: &str) -> Result<InstanceBackup, String> {
    let dirs = layout.dirs(name);
    let dir = layout.backups(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let at = locdev_core::model::now_ms();
    let id = at.to_string();
    let path = dir.join(format!("{id}.tar.gz"));
    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
        file,
        flate2::Compression::fast(),
    ));
    tar.follow_symlinks(false);
    tar.append_dir_all("data", &dirs.data)
        .map_err(|e| e.to_string())?;
    tar.into_inner()
        .and_then(|gz| gz.finish())
        .map_err(|e| e.to_string())?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok(InstanceBackup {
        id,
        instance: name.into(),
        at,
        size,
        path,
    })
}

pub fn backups(layout: &Layout, name: &str) -> Vec<InstanceBackup> {
    let mut list: Vec<InstanceBackup> = std::fs::read_dir(layout.backups(name))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter_map(|e| {
                    let path = e.path();
                    let id = path
                        .file_name()?
                        .to_str()?
                        .strip_suffix(".tar.gz")?
                        .to_string();
                    let at = id.parse().ok()?;
                    Some(InstanceBackup {
                        size: e.metadata().map(|m| m.len()).unwrap_or(0),
                        instance: name.into(),
                        at,
                        id,
                        path,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by_key(|b| std::cmp::Reverse(b.at));
    list
}

/// Puts a backup back (the instance must be stopped). The current data is
/// moved aside, not deleted: `data.before-<id>`.
pub fn restore(layout: &Layout, name: &str, id: &str) -> Result<PathBuf, String> {
    let backup = layout.backups(name).join(format!("{id}.tar.gz"));
    if !backup.is_file() {
        return Err(format!("no backup {id} for {name}"));
    }
    let dirs = layout.dirs(name);
    let aside = dirs
        .root
        .join(format!("data.before-{}", locdev_core::model::now_ms()));
    if dirs.data.exists() {
        std::fs::rename(&dirs.data, &aside).map_err(|e| e.to_string())?;
    }
    let staging = dirs.root.join(".restore");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(&backup).map_err(|e| e.to_string())?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    tar.set_preserve_permissions(true);
    // Our own archive, but entries are still checked to stay inside.
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(format!("unsafe path in backup: {}", path.display()));
        }
        entry.unpack_in(&staging).map_err(|e| e.to_string())?;
    }
    std::fs::rename(staging.join("data"), &dirs.data).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(aside)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_names() {
        assert!(check_instance_name("pg-17").is_ok());
        assert!(check_instance_name("PG").is_err());
        assert!(check_instance_name("../x").is_err());
    }

    #[test]
    fn backup_and_restore_keep_the_old_data() {
        let root = tempfile::tempdir().unwrap();
        let layout = Layout::new(root.path().to_path_buf());
        let dirs = layout.dirs("cache");
        std::fs::create_dir_all(&dirs.data).unwrap();
        std::fs::write(dirs.data.join("dump.rdb"), "v1").unwrap();
        let b = backup(&layout, "cache").unwrap();
        std::fs::write(dirs.data.join("dump.rdb"), "v2").unwrap();
        assert_eq!(backups(&layout, "cache").len(), 1);
        let aside = restore(&layout, "cache", &b.id).unwrap();
        assert_eq!(
            std::fs::read_to_string(dirs.data.join("dump.rdb")).unwrap(),
            "v1"
        );
        assert_eq!(
            std::fs::read_to_string(aside.join("dump.rdb")).unwrap(),
            "v2"
        );
    }

    #[test]
    fn init_refuses_existing_data() {
        let root = tempfile::tempdir().unwrap();
        let layout = Layout::new(root.path().to_path_buf());
        let dirs = layout.dirs("pg");
        std::fs::create_dir_all(&dirs.data).unwrap();
        std::fs::write(dirs.data.join("PG_VERSION"), "18").unwrap();
        assert!(
            init(ServiceKind::Postgres, Path::new("/nowhere"), &dirs, 5432)
                .unwrap_err()
                .contains("already has data")
        );
    }
}
