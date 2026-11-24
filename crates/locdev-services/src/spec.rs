//! What differs between services: their programs, how a data directory is
//! created, how the server starts (always bound to loopback), and how
//! clients connect.

use std::path::{Path, PathBuf};
use std::process::Command;

use locdev_core::services::ServiceKind;

/// The server program (and the one that creates a data directory, if any).
pub fn programs(kind: ServiceKind) -> (&'static str, Option<&'static str>) {
    match kind {
        ServiceKind::Postgres => ("postgres", Some("initdb")),
        ServiceKind::Mariadb => ("mariadbd", Some("mariadb-install-db")),
        ServiceKind::Mysql => ("mysqld", None),
        ServiceKind::Mongodb => ("mongod", None),
        ServiceKind::Valkey => ("valkey-server", None),
        ServiceKind::Redis => ("redis-server", None),
        ServiceKind::Sqlite => ("sqlite3", None),
    }
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Where a program is, given an installation's `bin` folder. MariaDB's
/// tarballs keep `mariadb-install-db` under `scripts/`.
pub fn find(bin_dir: &Path, program: &str) -> Option<PathBuf> {
    let candidates = [
        bin_dir.join(exe(program)),
        bin_dir.join("..").join("scripts").join(program),
        bin_dir.join("..").join("sbin").join(exe(program)),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// The version a server reports (`postgres --version` → `18.1`).
pub fn version_of(kind: ServiceKind, bin_dir: &Path) -> Option<String> {
    let server = find(bin_dir, programs(kind).0)?;
    let output = locdev_core::process::command(server)
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    parse_version(&text)
}

/// The first `N.N[.N]` in a version banner (`Redis server v=8.10.2`, `mariadbd  Ver 11.4.4-MariaDB`).
pub fn parse_version(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit()
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'v')
        {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let candidate = text[start..i].trim_end_matches('.');
            if candidate.contains('.') {
                return Some(candidate.to_string());
            }
        }
        i += 1;
    }
    None
}

/// One argument for the supervisor's shell.
pub fn quote(arg: &str) -> String {
    if cfg!(windows) {
        format!("\"{}\"", arg.replace('"', ""))
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn path(p: &Path) -> String {
    quote(&p.display().to_string())
}

/// An instance's folders.
#[derive(Debug, Clone)]
pub struct Dirs {
    pub root: PathBuf,
    pub data: PathBuf,
    /// Sockets and pid files.
    pub run: PathBuf,
    pub conf: PathBuf,
}

impl Dirs {
    pub fn new(root: PathBuf) -> Self {
        Self {
            data: root.join("data"),
            run: root.join("run"),
            conf: root.join("conf"),
            root,
        }
    }
}

/// The commands that create a fresh data directory, run once, in order.
/// Users are local and passwordless; servers only listen on 127.0.0.1.
pub fn init_commands(
    kind: ServiceKind,
    bin_dir: &Path,
    dirs: &Dirs,
    port: u16,
) -> Result<Vec<Command>, String> {
    let need = |program: &str| {
        find(bin_dir, program)
            .ok_or_else(|| format!("{program} was not found in {}", bin_dir.display()))
    };
    let basedir = bin_dir.parent().unwrap_or(bin_dir).to_path_buf();
    Ok(match kind {
        ServiceKind::Postgres => {
            let mut cmd = locdev_core::process::command(need("initdb")?);
            cmd.arg("-D").arg(&dirs.data).args([
                "-U",
                "locdev",
                "-A",
                "trust",
                "-E",
                "UTF8",
                "--locale=C",
                "--no-instructions",
            ]);
            vec![cmd]
        }
        ServiceKind::Mariadb => {
            let mut cmd = locdev_core::process::command(need("mariadb-install-db")?);
            cmd.arg("--no-defaults")
                .arg(format!("--basedir={}", basedir.display()))
                .arg(format!("--datadir={}", dirs.data.display()))
                .args(["--auth-root-authentication-method=normal", "--skip-test-db"]);
            vec![cmd]
        }
        ServiceKind::Mysql => {
            let mut cmd = locdev_core::process::command(need("mysqld")?);
            cmd.arg("--no-defaults")
                .arg("--initialize-insecure")
                .arg(format!("--basedir={}", basedir.display()))
                .arg(format!("--datadir={}", dirs.data.display()));
            vec![cmd]
        }
        ServiceKind::Valkey | ServiceKind::Redis => {
            std::fs::create_dir_all(&dirs.data).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dirs.conf).map_err(|e| e.to_string())?;
            let conf = format!(
                "# Managed by LocDev\nport {port}\nbind 127.0.0.1 -::1\nprotected-mode yes\ndir {}\ndbfilename dump.rdb\nsave 3600 1 300 100 60 10000\nappendonly no\n",
                dirs.data.display()
            );
            std::fs::write(dirs.conf.join("server.conf"), conf).map_err(|e| e.to_string())?;
            Vec::new()
        }
        ServiceKind::Mongodb => {
            std::fs::create_dir_all(&dirs.data).map_err(|e| e.to_string())?;
            Vec::new()
        }
        ServiceKind::Sqlite => return Err("SQLite is a file; there is no server to set up".into()),
    })
}

/// The command the supervisor runs to serve an instance on `port`.
pub fn start_command(
    kind: ServiceKind,
    bin_dir: &Path,
    dirs: &Dirs,
    port: u16,
) -> Result<String, String> {
    let (server, _) = programs(kind);
    let server = find(bin_dir, server)
        .ok_or_else(|| format!("{server} was not found in {}", bin_dir.display()))?;
    let basedir = bin_dir.parent().unwrap_or(bin_dir);
    let unix = !cfg!(windows);
    Ok(match kind {
        ServiceKind::Postgres => {
            // TCP on loopback only: no Unix socket, whose path has a ~107-byte limit.
            format!(
                "{} -D {} -p {port} -c listen_addresses=127.0.0.1 -c unix_socket_directories=''",
                path(&server),
                path(&dirs.data)
            )
        }
        ServiceKind::Mariadb | ServiceKind::Mysql => {
            let mut cmd = format!(
                "{} --no-defaults --basedir={} --datadir={} --port={port} --bind-address=127.0.0.1 --skip-name-resolve --pid-file={}",
                path(&server),
                path(basedir),
                path(&dirs.data),
                path(&dirs.run.join("server.pid"))
            );
            // The server always opens a socket; keep its path under the limit.
            if unix {
                cmd.push_str(&format!(" --socket={}", path(&socket_path(dirs, port))));
            }
            if kind == ServiceKind::Mysql {
                cmd.push_str(" --mysqlx=OFF");
            }
            cmd
        }
        ServiceKind::Mongodb => {
            let mut cmd = format!(
                "{} --dbpath {} --port {port} --bind_ip 127.0.0.1",
                path(&server),
                path(&dirs.data)
            );
            if unix {
                cmd.push_str(" --nounixsocket");
            }
            cmd
        }
        ServiceKind::Valkey | ServiceKind::Redis => {
            format!("{} {}", path(&server), path(&dirs.conf.join("server.conf")))
        }
        ServiceKind::Sqlite => return Err("SQLite is a file; there is no server to run".into()),
    })
}

/// A Unix socket path short enough for the kernel's limit.
fn socket_path(dirs: &Dirs, port: u16) -> PathBuf {
    let preferred = dirs.run.join("server.sock");
    if preferred.as_os_str().len() < 100 {
        preferred
    } else {
        std::env::temp_dir().join(format!("locdev-svc-{port}.sock"))
    }
}

/// How to connect: a URL and the variables frameworks usually read.
pub fn connection(kind: ServiceKind, port: u16) -> Vec<(String, String)> {
    let kv = |k: &str, v: String| (k.to_string(), v);
    match kind {
        ServiceKind::Postgres => vec![
            kv(
                "DATABASE_URL",
                format!("postgresql://locdev@127.0.0.1:{port}/postgres"),
            ),
            kv("DB_CONNECTION", "pgsql".into()),
            kv("DB_HOST", "127.0.0.1".into()),
            kv("DB_PORT", port.to_string()),
            kv("DB_USERNAME", "locdev".into()),
            kv("DB_PASSWORD", String::new()),
        ],
        ServiceKind::Mariadb | ServiceKind::Mysql => vec![
            kv("DATABASE_URL", format!("mysql://root@127.0.0.1:{port}/")),
            kv(
                "DB_CONNECTION",
                if kind == ServiceKind::Mariadb {
                    "mariadb"
                } else {
                    "mysql"
                }
                .into(),
            ),
            kv("DB_HOST", "127.0.0.1".into()),
            kv("DB_PORT", port.to_string()),
            kv("DB_USERNAME", "root".into()),
            kv("DB_PASSWORD", String::new()),
        ],
        ServiceKind::Mongodb => vec![
            kv("MONGODB_URI", format!("mongodb://127.0.0.1:{port}")),
            kv("DATABASE_URL", format!("mongodb://127.0.0.1:{port}")),
        ],
        ServiceKind::Valkey | ServiceKind::Redis => vec![
            kv("REDIS_URL", format!("redis://127.0.0.1:{port}")),
            kv("REDIS_HOST", "127.0.0.1".into()),
            kv("REDIS_PORT", port.to_string()),
        ],
        ServiceKind::Sqlite => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_from_banners() {
        assert_eq!(
            parse_version("postgres (PostgreSQL) 18.1 (Ubuntu 18.1-1)").as_deref(),
            Some("18.1")
        );
        assert_eq!(
            parse_version("Redis server v=8.10.2 sha=00000000:1").as_deref(),
            Some("8.10.2")
        );
        assert_eq!(
            parse_version("mariadbd  Ver 11.4.4-MariaDB for Linux").as_deref(),
            Some("11.4.4")
        );
        assert_eq!(
            parse_version("db version v8.0.4\nBuild Info").as_deref(),
            Some("8.0.4")
        );
        assert_eq!(parse_version("x86_64 nothing"), None);
    }

    #[cfg(unix)]
    #[test]
    fn quoting_survives_spaces_and_quotes() {
        assert_eq!(quote("/a b/it's"), "'/a b/it'\\''s'");
    }

    #[test]
    fn servers_listen_on_loopback_only() {
        let dirs = Dirs::new(PathBuf::from("/x"));
        let bin = tempfile::tempdir().unwrap();
        for kind in [
            ServiceKind::Postgres,
            ServiceKind::Mariadb,
            ServiceKind::Mongodb,
            ServiceKind::Valkey,
        ] {
            std::fs::write(bin.path().join(exe(programs(kind).0)), "").unwrap();
            let cmd = start_command(kind, bin.path(), &dirs, 7000).unwrap();
            if kind != ServiceKind::Valkey {
                assert!(cmd.contains("127.0.0.1"), "{cmd}");
            }
        }
    }
}
