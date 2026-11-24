//! Versions installed through the system's package manager, from the app's
//! terminal (the user sees the command first; `sudo` asks there). For what
//! LocDev can't download with a publisher's SHA-256 (MySQL, Redis), and for
//! more versions where it can. The package manager verifies its packages
//! (Homebrew bottles and winget installers by SHA-256, apt by signature);
//! the installs are system ones afterwards: found, never changed or removed.
//!
//! Names checked 2026-11-23 on formulae.brew.sh, mongodb/homebrew-brew's
//! README, winget-pkgs (`manifests/o/Oracle/MySQL`) and redis.io.

use std::path::Path;

use locdev_core::services::{ServiceKind, ServicePackage};

fn package(id: &str, version: &str, manager: &str, commands: &[&[&str]]) -> ServicePackage {
    ServicePackage {
        id: id.into(),
        version: version.into(),
        manager: manager.into(),
        commands: commands
            .iter()
            .map(|c| c.iter().map(|a| (*a).to_string()).collect())
            .collect(),
    }
}

fn brew(formula: &str, version: &str) -> ServicePackage {
    package(
        formula,
        version,
        "Homebrew",
        &[&["brew", "install", formula]],
    )
}

fn has(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path)
            .any(|dir| dir.join(program).is_file() || dir.join(format!("{program}.exe")).is_file())
    }) || Path::new(&format!("/opt/homebrew/bin/{program}")).is_file()
}

/// What this machine's package manager offers for `kind`.
pub fn packages(kind: ServiceKind) -> Vec<ServicePackage> {
    if cfg!(windows) {
        return windows(kind);
    }
    if cfg!(target_os = "macos") || has("brew") {
        return homebrew(kind);
    }
    if has("apt-get") {
        return apt(kind);
    }
    Vec::new()
}

fn homebrew(kind: ServiceKind) -> Vec<ServicePackage> {
    match kind {
        ServiceKind::Mysql => vec![
            brew("mysql", "latest"),
            brew("mysql@8.4", "8.4 LTS"),
            brew("mysql@8.0", "8.0 (deprecated by Homebrew)"),
        ],
        ServiceKind::Redis => vec![brew("redis", "latest"), brew("redis@8.2", "8.2")],
        ServiceKind::Postgres => vec![
            brew("postgresql@17", "17"),
            brew("postgresql@16", "16"),
            brew("postgresql@15", "15"),
        ],
        ServiceKind::Mongodb => ["8.0", "7.0"]
            .iter()
            .map(|v| {
                let formula = format!("mongodb/brew/mongodb-community@{v}");
                package(
                    &format!("mongodb-community@{v}"),
                    v,
                    "Homebrew (mongodb/brew)",
                    &[
                        &["brew", "tap", "mongodb/brew"],
                        &["brew", "install", &formula],
                    ],
                )
            })
            .collect(),
        ServiceKind::Mariadb => vec![brew("mariadb", "latest")],
        ServiceKind::Valkey => vec![brew("valkey", "latest")],
        ServiceKind::Sqlite => Vec::new(),
    }
}

fn apt(kind: ServiceKind) -> Vec<ServicePackage> {
    // The distribution's own version; one per package.
    let one = |name: &str| {
        vec![package(
            name,
            "the distribution's",
            "apt",
            &[&["sudo", "apt-get", "install", "-y", name]],
        )]
    };
    match kind {
        ServiceKind::Mysql => one("mysql-server"),
        ServiceKind::Redis => one("redis-server"),
        ServiceKind::Postgres => one("postgresql"),
        ServiceKind::Mariadb => one("mariadb-server"),
        _ => Vec::new(),
    }
}

fn windows(kind: ServiceKind) -> Vec<ServicePackage> {
    let winget = |id: &str, version: &str, args: &[&str]| {
        let mut command = vec!["winget", "install", "--id", "Oracle.MySQL", "-e"];
        command.extend_from_slice(args);
        command.extend(["--accept-package-agreements", "--accept-source-agreements"]);
        package(id, version, "winget", &[&command])
    };
    match kind {
        // Oracle's MSI (MySQL Server x.y under Program Files\MySQL). Pinned
        // versions are the newest of each line in winget-pkgs; winget says so
        // when one is gone.
        ServiceKind::Mysql => vec![
            winget("oracle-mysql", "latest", &[]),
            winget("oracle-mysql-8.4", "8.4", &["--version", "8.4.9"]),
            winget("oracle-mysql-8.0", "8.0", &["--version", "8.0.43"]),
        ],
        _ => Vec::new(),
    }
}

/// Homebrew keeps versioned formulae off PATH (`/opt/homebrew/opt/mysql@8.4/bin`).
pub fn brew_prefix(kind: ServiceKind) -> Option<&'static str> {
    Some(match kind {
        ServiceKind::Postgres => "postgresql",
        ServiceKind::Mysql => "mysql",
        ServiceKind::Redis => "redis",
        ServiceKind::Mongodb => "mongodb-community",
        ServiceKind::Mariadb => "mariadb",
        ServiceKind::Valkey => "valkey",
        ServiceKind::Sqlite => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_package_managers_run() {
        for kind in ServiceKind::ALL {
            for set in [homebrew(kind), apt(kind), windows(kind)] {
                for p in set {
                    for c in &p.commands {
                        assert!(["brew", "sudo", "winget"].contains(&c[0].as_str()), "{c:?}");
                    }
                }
            }
        }
        let mysql = windows(ServiceKind::Mysql);
        assert!(
            mysql
                .iter()
                .any(|p| p.commands[0].contains(&"8.4.9".to_string()))
        );
        assert_eq!(
            homebrew(ServiceKind::Mongodb)[0].commands,
            vec![
                vec!["brew", "tap", "mongodb/brew"],
                vec!["brew", "install", "mongodb/brew/mongodb-community@8.0"]
            ]
        );
    }
}
