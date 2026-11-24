//! Package managers LocDev can install: pnpm and Yarn from the npm registry
//! (each tarball checked against the registry's `sha512` integrity), and
//! Composer from getcomposer.org (checked against its published SHA-256).
//!
//! None of them is a native binary: each install gets small launchers that
//! run it with whatever `node`/`php` comes first on the project's PATH, so a
//! package manager always runs on the project's own runtime.

use std::path::{Path, PathBuf};

use locdev_core::model::{JobState, Release, RuntimeKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{RuntimeError, download, extract, version};

const REGISTRY: &str = "https://registry.npmjs.org";
const COMPOSER: &str = "https://getcomposer.org";
/// Abbreviated metadata: versions, dist and engines without readmes.
const ABBREVIATED: &str = "application/vnd.npm.install-v1+json";

/// Folder (inside a version's directory) that holds the launchers.
const LAUNCHERS: &str = "locdev-bin";
/// What a tool needs to run, written next to it at install time.
const META: &str = "locdev-tool.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolMeta {
    /// Runtime constraints from the package (`node: ">=18.12"`, `php: ">=7.2.5"`).
    #[serde(default)]
    pub requires: Vec<(RuntimeKind, String)>,
}

pub fn supported(kind: RuntimeKind) -> bool {
    matches!(
        kind,
        RuntimeKind::Pnpm | RuntimeKind::Yarn | RuntimeKind::Composer
    )
}

/// npm packages a tool is published as. Yarn 1 is `yarn`; Yarn 2+ ships as
/// `@yarnpkg/cli-dist`.
fn packages(kind: RuntimeKind) -> &'static [&'static str] {
    match kind {
        RuntimeKind::Pnpm => &["pnpm"],
        RuntimeKind::Yarn => &["yarn", "@yarnpkg/cli-dist"],
        _ => &[],
    }
}

fn package_for(kind: RuntimeKind, version: &str) -> Option<&'static str> {
    match kind {
        RuntimeKind::Yarn if version::parts(version).first() == Some(&1) => Some("yarn"),
        RuntimeKind::Yarn => Some("@yarnpkg/cli-dist"),
        RuntimeKind::Pnpm => Some("pnpm"),
        _ => None,
    }
}

fn metadata_url(package: &str) -> String {
    format!("{REGISTRY}/{}", package.replace('/', "%2f"))
}

fn fetch_metadata(package: &str) -> Result<Value, RuntimeError> {
    let raw = download::fetch_text_accept(&metadata_url(package), ABBREVIATED)?;
    serde_json::from_str(&raw).map_err(|e| RuntimeError::Catalog(format!("{package}: {e}")))
}

/// Stable releases, newest first.
pub fn catalog(kind: RuntimeKind) -> Result<Vec<Release>, RuntimeError> {
    if kind == RuntimeKind::Composer {
        return composer_catalog();
    }
    let mut releases = Vec::new();
    for package in packages(kind) {
        let meta = fetch_metadata(package)?;
        let latest = meta["dist-tags"]["latest"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if let Some(versions) = meta["versions"].as_object() {
            for (v, entry) in versions {
                // No pre-releases, and nothing the registry flags as deprecated.
                if v.contains('-')
                    || entry.get("deprecated").is_some()
                    || version::parts(v).len() != 3
                {
                    continue;
                }
                // `@yarnpkg/cli-dist` also publishes old 1.x-numbered builds.
                if kind == RuntimeKind::Yarn && package_for(kind, v) != Some(package) {
                    continue;
                }
                releases.push(Release {
                    version: v.clone(),
                    lts: (*v == latest).then(|| "latest".to_string()),
                    date: String::new(),
                });
            }
        }
    }
    releases.sort_by(|a, b| version::compare(&b.version, &a.version));
    releases.dedup_by(|a, b| a.version == b.version);
    Ok(releases)
}

fn composer_catalog() -> Result<Vec<Release>, RuntimeError> {
    let channels: Value =
        serde_json::from_str(&download::fetch_text(&format!("{COMPOSER}/versions"))?)
            .map_err(|e| RuntimeError::Catalog(format!("Composer versions: {e}")))?;
    let mut releases = Vec::new();
    for channel in ["stable", "2.2", "2"] {
        for entry in channels[channel].as_array().into_iter().flatten() {
            let Some(v) = entry["version"].as_str() else {
                continue;
            };
            if version::parts(v).len() == 3 && !releases.iter().any(|r: &Release| r.version == v) {
                releases.push(Release {
                    version: v.to_string(),
                    lts: entry["lts"]
                        .as_bool()
                        .unwrap_or(false)
                        .then(|| "LTS".to_string()),
                    date: String::new(),
                });
            }
        }
    }
    releases.sort_by(|a, b| version::compare(&b.version, &a.version));
    Ok(releases)
}

/// The launcher a project's PATH should find.
pub fn executable(kind: RuntimeKind, dir: &Path) -> PathBuf {
    let name = kind.executable();
    if cfg!(windows) {
        dir.join(LAUNCHERS).join(format!("{name}.cmd"))
    } else {
        dir.join(LAUNCHERS).join(name)
    }
}

pub fn meta(dir: &Path) -> ToolMeta {
    std::fs::read_to_string(dir.join(META))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn install(
    kind: RuntimeKind,
    version: &str,
    dest: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<(), RuntimeError> {
    let unpacked = scratch.join(format!("{}-{version}", kind.executable()));
    std::fs::create_dir_all(&unpacked)?;
    let meta = match kind {
        RuntimeKind::Composer => {
            install_composer(version, dest, &unpacked, scratch, progress, log)?
        }
        _ => install_npm_package(kind, version, dest, &unpacked, scratch, progress, log)?,
    };
    let raw = serde_json::to_string_pretty(&meta).map_err(std::io::Error::other)?;
    std::fs::write(unpacked.join(META), raw)?;
    std::fs::rename(&unpacked, dest)?;
    Ok(())
}

fn install_npm_package(
    kind: RuntimeKind,
    version: &str,
    dest: &Path,
    unpacked: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<ToolMeta, RuntimeError> {
    let package = package_for(kind, version).ok_or(RuntimeError::Unsupported)?;
    let meta = fetch_metadata(package)?;
    let entry = &meta["versions"][version];
    let tarball = entry["dist"]["tarball"].as_str().unwrap_or_default();
    let integrity = entry["dist"]["integrity"].as_str().unwrap_or_default();
    // The registry names the tarball; it must still come from the registry.
    if !tarball.starts_with(&format!("{REGISTRY}/")) {
        return Err(RuntimeError::Catalog(format!(
            "{package}@{version}: unexpected tarball location '{tarball}'"
        )));
    }
    if integrity.is_empty() {
        return Err(RuntimeError::Catalog(format!(
            "{package}@{version} has no integrity hash"
        )));
    }
    log(format!("source {tarball}"));
    log(format!(
        "expected {}",
        integrity.split_whitespace().next().unwrap_or(integrity)
    ));

    let archive = scratch.join("package.tgz");
    let digests = download::download_digests(tarball, &archive, &|done, total| {
        progress(JobState::Downloading, done, total)
    })?;
    progress(JobState::Verifying, 0, None);
    if !download::integrity_matches(integrity, &digests.sha512)? {
        let _ = std::fs::remove_file(&archive);
        return Err(RuntimeError::Checksum {
            file: format!("{package}-{version}.tgz"),
            expected: integrity.to_string(),
            actual: format!(
                "sha512-{}",
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &digests.sha512)
            ),
        });
    }
    log("sha512 matches the registry's integrity".into());

    progress(JobState::Extracting, 0, None);
    extract::tar_gz(&archive, unpacked)?;
    let _ = std::fs::remove_file(&archive);

    let manifest: Value = std::fs::read_to_string(unpacked.join("package.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .ok_or_else(|| RuntimeError::Archive("the package has no package.json".into()))?;
    let bins = bin_entries(&manifest, kind);
    if bins.is_empty() {
        return Err(RuntimeError::Archive(format!(
            "{package} declares no executable"
        )));
    }
    for (name, script) in &bins {
        write_launcher(unpacked, dest, name, "node", script)?;
    }
    log(format!(
        "launchers: {}",
        bins.iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    let requires = manifest["engines"]["node"]
        .as_str()
        .map(|range| vec![(RuntimeKind::Node, range.to_string())])
        .unwrap_or_default();
    Ok(ToolMeta { requires })
}

/// `bin` is either one path (named after the package) or a name → path map.
/// Paths are relative to the package and must stay inside it.
fn bin_entries(manifest: &Value, kind: RuntimeKind) -> Vec<(String, String)> {
    let entries: Vec<(String, String)> = match &manifest["bin"] {
        Value::String(path) => vec![(kind.executable().to_string(), path.clone())],
        Value::Object(map) => map
            .iter()
            .filter_map(|(name, path)| Some((name.clone(), path.as_str()?.to_string())))
            .collect(),
        _ => Vec::new(),
    };
    entries
        .into_iter()
        .filter(|(name, path)| {
            let safe_name = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
            let safe_path = !path.starts_with('/') && !path.contains("..") && !path.contains('\\');
            safe_name && safe_path
        })
        .collect()
}

fn install_composer(
    version: &str,
    dest: &Path,
    unpacked: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<ToolMeta, RuntimeError> {
    if version::parts(version).len() != 3 {
        return Err(RuntimeError::Catalog(format!(
            "'{version}' is not a Composer release"
        )));
    }
    let url = format!("{COMPOSER}/download/{version}/composer.phar");
    // The expected hash comes first: no checksum, no install.
    let sums = download::fetch_text(&format!("{url}.sha256sum"))?;
    let expected = download::checksum_for(&sums, "composer.phar")
        .ok_or_else(|| RuntimeError::Catalog("composer.phar.sha256sum lists no hash".into()))?;
    log(format!("source {url}"));
    log(format!("expected sha256 {expected}"));

    let phar = scratch.join("composer.phar");
    let actual = download::download(&url, &phar, &|done, total| {
        progress(JobState::Downloading, done, total)
    })?;
    progress(JobState::Verifying, 0, None);
    if actual != expected {
        let _ = std::fs::remove_file(&phar);
        return Err(RuntimeError::Checksum {
            file: "composer.phar".into(),
            expected,
            actual,
        });
    }
    log("sha256 matches getcomposer.org".into());

    progress(JobState::Extracting, 0, None);
    std::fs::rename(&phar, unpacked.join("composer.phar"))?;
    write_launcher(unpacked, dest, "composer", "php", "composer.phar")?;

    // Composer 2.3+ needs PHP 7.2.5; the 2.2 LTS line still runs on 5.3.
    let min_php = if version::compare(version, "2.3.0").is_lt() {
        ">=5.3.2"
    } else {
        ">=7.2.5"
    };
    Ok(ToolMeta {
        requires: vec![(RuntimeKind::Php, min_php.to_string())],
    })
}

/// A script that runs `script` (relative to the package) with `interpreter`
/// from PATH. Written into `unpacked`, but pointing at `dest`, where the
/// folder ends up.
fn write_launcher(
    unpacked: &Path,
    dest: &Path,
    name: &str,
    interpreter: &str,
    script: &str,
) -> std::io::Result<()> {
    let dir = unpacked.join(LAUNCHERS);
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let target = dest
            .join(script)
            .display()
            .to_string()
            .replace('\'', "'\\''");
        let body =
            format!("#!/bin/sh\n# Managed by LocDev\nexec {interpreter} '{target}' \"$@\"\n");
        let path = dir.join(name);
        std::fs::write(&path, body)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        // Relative to the launcher, so the batch file never embeds a path
        // that needs quoting rules of its own.
        let _ = dest;
        let relative = script.replace('/', "\\");
        let body = format!(
            "@ECHO off\r\nREM Managed by LocDev\r\n{interpreter} \"%~dp0..\\{relative}\" %*\r\n"
        );
        std::fs::write(dir.join(format!("{name}.cmd")), body)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bins_safely() {
        let manifest: Value = serde_json::json!({
            "bin": { "pnpm": "bin/pnpm.cjs", "pnpx": "bin/pnpx.cjs", "evil": "../../x", "bad name": "a.js" }
        });
        let bins = bin_entries(&manifest, RuntimeKind::Pnpm);
        assert_eq!(
            bins,
            vec![
                ("pnpm".into(), "bin/pnpm.cjs".into()),
                ("pnpx".into(), "bin/pnpx.cjs".into())
            ]
        );
        let single = serde_json::json!({ "bin": "./bin/yarn.js" });
        assert_eq!(
            bin_entries(&single, RuntimeKind::Yarn),
            vec![("yarn".into(), "./bin/yarn.js".into())]
        );
    }

    #[test]
    fn picks_the_yarn_package_by_major() {
        assert_eq!(package_for(RuntimeKind::Yarn, "1.22.22"), Some("yarn"));
        assert_eq!(
            package_for(RuntimeKind::Yarn, "4.5.0"),
            Some("@yarnpkg/cli-dist")
        );
        assert_eq!(
            metadata_url("@yarnpkg/cli-dist"),
            "https://registry.npmjs.org/@yarnpkg%2fcli-dist"
        );
    }

    #[cfg(unix)]
    #[test]
    fn launchers_point_at_the_final_folder() {
        let dir = tempfile::tempdir().unwrap();
        let unpacked = dir.path().join("scratch");
        let dest = dir.path().join("pnpm/9.12.0");
        write_launcher(&unpacked, &dest, "pnpm", "node", "bin/pnpm.cjs").unwrap();
        let body = std::fs::read_to_string(unpacked.join(LAUNCHERS).join("pnpm")).unwrap();
        assert!(body.contains(&format!(
            "exec node '{}'",
            dest.join("bin/pnpm.cjs").display()
        )));
        assert!(body.contains("Managed by LocDev"));
        assert_eq!(
            executable(RuntimeKind::Pnpm, &dest),
            dest.join("locdev-bin/pnpm")
        );
    }
}
