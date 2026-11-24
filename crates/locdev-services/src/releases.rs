//! Versions LocDev can download, with the checksum each publisher states.
//! A service without a trusted build for this platform says why instead.
//!
//! - PostgreSQL: theseus-rs/postgresql-binaries (built from the official
//!   source, with SHA-256 files and provenance attestations), every OS.
//! - MariaDB: the MariaDB Foundation's REST API (bintar, SHA-256), Linux.
//! - MongoDB: fastdl.mongodb.org community builds (`.sha256` next to each),
//!   Ubuntu/Debian, macOS and Windows.
//! - Valkey: download.valkey.io Ubuntu builds (`.sha256`), Linux.
//! - MySQL and Redis publish no SHA-256 for their binaries: system installs only.

use locdev_core::services::{ServiceKind, ServiceRelease};
use locdev_runtimes::download::{checksum_for, fetch_text};
use serde_json::Value;

/// A downloadable build: where it is and the SHA-256 to expect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub release: ServiceRelease,
    pub sha256: String,
    pub zip: bool,
}

/// Why `kind` can't be downloaded on this machine, if it can't.
pub fn unavailable(kind: ServiceKind) -> Option<String> {
    let (os, _) = (std::env::consts::OS, std::env::consts::ARCH);
    match kind {
        ServiceKind::Mysql => Some(
            "Oracle publishes MD5 sums only, which LocDev doesn't accept; use a system MySQL or MariaDB".into(),
        ),
        ServiceKind::Redis => Some(
            "Redis publishes no prebuilt binaries; use a system Redis, or Valkey (a compatible fork LocDev can install)".into(),
        ),
        ServiceKind::Sqlite => Some("SQLite is a file; there is nothing to install or run".into()),
        ServiceKind::Valkey if os != "linux" => Some("Valkey publishes builds for Linux only; use Homebrew or a system install".into()),
        ServiceKind::Mariadb if os == "macos" => Some("MariaDB publishes no macOS builds; use Homebrew or a system install".into()),
        ServiceKind::Mongodb if os == "linux" && mongo_target().is_none() => {
            Some("MongoDB publishes builds per distribution; this one isn't covered (Ubuntu 22.04+/Debian 12)".into())
        }
        _ if target_triple().is_none() => Some("no builds for this CPU architecture".into()),
        _ => None,
    }
}

fn target_triple() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return None,
    })
}

fn os_release() -> (String, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let get = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")))
            .map(|v| v.trim_matches('"').to_string())
            .unwrap_or_default()
    };
    (get("ID"), get("VERSION_ID"))
}

/// MongoDB's name for this Linux distribution (`ubuntu2404`).
fn mongo_target() -> Option<String> {
    let (id, version) = os_release();
    let major: u32 = version.split('.').next()?.parse().ok()?;
    match id.as_str() {
        // Newer Ubuntus run the latest LTS build.
        "ubuntu" if major >= 24 => Some("ubuntu2404".into()),
        "ubuntu" if major >= 22 => Some("ubuntu2204".into()),
        "debian" if major >= 12 => Some("debian12".into()),
        _ => None,
    }
}

fn github_json(url: &str) -> Result<Value, String> {
    let text = fetch_text(url).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{url}: {e}"))
}

/// `X.Y.Z` tags, newest first, keeping the newest of each `X.Y` line.
fn newest_per_line(mut versions: Vec<String>, keep: usize) -> Vec<String> {
    let key = |v: &String| {
        v.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    versions.sort_by_key(|v| std::cmp::Reverse(key(v)));
    let mut seen = Vec::new();
    versions
        .into_iter()
        .filter(|v| {
            let line: Vec<&str> = v.split('.').take(2).collect();
            let line = line.join(".");
            if seen.contains(&line) {
                false
            } else {
                seen.push(line);
                true
            }
        })
        .take(keep)
        .collect()
}

/// Downloadable versions, newest first (no network when unavailable).
pub fn list(kind: ServiceKind) -> Result<Vec<ServiceRelease>, String> {
    if let Some(why) = unavailable(kind) {
        return Err(why);
    }
    let release = |version: String, publisher: &str, url: String| ServiceRelease {
        kind,
        version,
        publisher: publisher.into(),
        url,
    };
    Ok(match kind {
        ServiceKind::Postgres => {
            let json = github_json(
                "https://api.github.com/repos/theseus-rs/postgresql-binaries/releases?per_page=100",
            )?;
            let tags: Vec<String> = json
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["prerelease"] == false)
                .filter_map(|r| r["tag_name"].as_str().map(str::to_string))
                .collect();
            // Tags are `<major>.<minor>.<build>`: keep the newest per major.
            let mut by_major: Vec<String> = Vec::new();
            for tag in newest_per_line(tags, 40) {
                let major = tag.split('.').next().unwrap_or_default().to_string();
                if !by_major.iter().any(|t| t.split('.').next() == Some(&major)) {
                    by_major.push(tag);
                }
            }
            by_major
                .into_iter()
                .take(5)
                .map(|tag| {
                    let target = target_triple().unwrap_or_default();
                    release(
                        tag.clone(),
                        "PostgreSQL source, built by theseus-rs/postgresql-binaries",
                        format!("https://github.com/theseus-rs/postgresql-binaries/releases/download/{tag}/postgresql-{tag}-{target}.tar.gz"),
                    )
                })
                .collect()
        }
        ServiceKind::Valkey => {
            let json =
                github_json("https://api.github.com/repos/valkey-io/valkey/releases?per_page=50")?;
            let tags: Vec<String> = json
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["prerelease"] == false)
                .filter_map(|r| {
                    r["tag_name"]
                        .as_str()
                        .map(|t| t.trim_start_matches('v').to_string())
                })
                .filter(|t| {
                    t.split('.').count() == 3 && t.chars().all(|c| c.is_ascii_digit() || c == '.')
                })
                .collect();
            let arch = if std::env::consts::ARCH == "aarch64" {
                "arm64"
            } else {
                "x86_64"
            };
            let codename = valkey_codename();
            newest_per_line(tags, 4)
                .into_iter()
                .map(|v| {
                    let url = format!(
                        "https://download.valkey.io/releases/valkey-{v}-{codename}-{arch}.tar.gz"
                    );
                    release(v, "Valkey (download.valkey.io)", url)
                })
                .collect()
        }
        ServiceKind::Mongodb => {
            let json = github_json("https://api.github.com/repos/mongodb/mongo/tags?per_page=100")?;
            let tags: Vec<String> = json
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|t| t["name"].as_str()?.strip_prefix('r').map(str::to_string))
                .filter(|t| {
                    t.split('.').count() == 3 && t.chars().all(|c| c.is_ascii_digit() || c == '.')
                })
                // Only the long-supported `X.0` lines are production releases on premises.
                .filter(|t| t.split('.').nth(1) == Some("0"))
                .collect();
            newest_per_line(tags, 3)
                .into_iter()
                .map(|v| release(v.clone(), "MongoDB, Inc. (community server)", mongo_url(&v)))
                .collect()
        }
        ServiceKind::Mariadb => {
            let json = github_json("https://downloads.mariadb.org/rest-api/mariadb/")?;
            let majors: Vec<String> = json["major_releases"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|m| m["release_status"] == "Stable")
                .filter_map(|m| m["release_id"].as_str().map(str::to_string))
                .take(4)
                .collect();
            let mut out = Vec::new();
            for major in majors {
                let Ok(json) = github_json(&format!(
                    "https://downloads.mariadb.org/rest-api/mariadb/{major}/"
                )) else {
                    continue;
                };
                let newest = json["releases"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .map(|(v, _)| v.clone())
                    .max_by_key(|v| {
                        v.split('.')
                            .map(|p| p.parse::<u64>().unwrap_or(0))
                            .collect::<Vec<_>>()
                    });
                if let Some(version) = newest
                    && let Some(file) = mariadb_file(&json["releases"][&version])
                {
                    out.push(release(version, "MariaDB Foundation", file.0));
                }
            }
            out
        }
        ServiceKind::Mysql | ServiceKind::Redis | ServiceKind::Sqlite => Vec::new(),
    })
}

fn valkey_codename() -> &'static str {
    // Ubuntu 24.04 builds need glibc 2.39; older systems get the 22.04 ones.
    let glibc = std::process::Command::new("getconf")
        .arg("GNU_LIBC_VERSION")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .and_then(|t| t.split_whitespace().nth(1).map(str::to_string))
        .and_then(|v| v.split('.').nth(1).and_then(|m| m.parse::<u32>().ok()));
    if glibc.is_some_and(|minor| minor >= 39) {
        "noble"
    } else {
        "jammy"
    }
}

fn mongo_url(version: &str) -> String {
    let arch = if std::env::consts::ARCH == "aarch64" {
        "aarch64"
    } else {
        "x86_64"
    };
    match std::env::consts::OS {
        "macos" => format!(
            "https://fastdl.mongodb.org/osx/mongodb-macos-{}-{version}.tgz",
            if arch == "aarch64" { "arm64" } else { "x86_64" }
        ),
        "windows" => {
            format!("https://fastdl.mongodb.org/windows/mongodb-windows-x86_64-{version}.zip")
        }
        _ => format!(
            "https://fastdl.mongodb.org/linux/mongodb-linux-{arch}-{}-{version}.tgz",
            mongo_target().unwrap_or_default()
        ),
    }
}

/// The bintar (or Windows zip) of a MariaDB release, with its SHA-256.
fn mariadb_file(release: &Value) -> Option<(String, String)> {
    let windows = cfg!(windows);
    let arch = if std::env::consts::ARCH == "aarch64" {
        "aarch64"
    } else {
        "x86_64"
    };
    release["files"].as_array()?.iter().find_map(|f| {
        let name = f["file_name"].as_str()?;
        let ok = if windows {
            name.ends_with("-winx64.zip")
        } else {
            name.contains("-linux-systemd-")
                && name.contains(arch)
                && name.ends_with(".tar.gz")
                && !name.contains("debug")
        };
        ok.then(|| {
            Some((
                f["file_download_url"].as_str()?.to_string(),
                f["checksum"]["sha256sum"].as_str()?.to_string(),
            ))
        })
        .flatten()
    })
}

/// The release with the checksum its publisher states.
pub fn resolve(release: &ServiceRelease) -> Result<Build, String> {
    let zip = release.url.ends_with(".zip");
    let sha256 = match release.kind {
        ServiceKind::Mariadb => {
            let major: Vec<&str> = release.version.split('.').take(2).collect();
            let json = github_json(&format!(
                "https://downloads.mariadb.org/rest-api/mariadb/{}/",
                major.join(".")
            ))?;
            mariadb_file(&json["releases"][&release.version])
                .map(|(_, sha)| sha)
                .ok_or("MariaDB lists no checksum for this file")?
        }
        _ => {
            let listing =
                fetch_text(&format!("{}.sha256", release.url)).map_err(|e| e.to_string())?;
            let file = release.url.rsplit('/').next().unwrap_or_default();
            checksum_for(&listing, file)
                .or_else(|| listing.split_whitespace().next().map(str::to_string))
                .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
                .ok_or("the publisher's checksum file is not a SHA-256")?
        }
    };
    Ok(Build {
        release: release.clone(),
        sha256: sha256.to_lowercase(),
        zip,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_newest_of_each_line() {
        let v = newest_per_line(
            ["9.1.3", "9.1.2", "9.0.4", "8.1.6", "8.1.10", "7.2.9"]
                .map(String::from)
                .to_vec(),
            3,
        );
        assert_eq!(v, vec!["9.1.3", "9.0.4", "8.1.10"]);
    }

    #[test]
    fn no_unverifiable_downloads() {
        assert!(unavailable(ServiceKind::Mysql).unwrap().contains("MD5"));
        assert!(unavailable(ServiceKind::Redis).is_some());
        assert!(list(ServiceKind::Sqlite).is_err());
    }
}
