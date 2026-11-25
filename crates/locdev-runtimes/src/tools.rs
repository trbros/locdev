//! Package managers LocDev can install: pnpm and Yarn from the npm registry
//! (each tarball checked against the registry's `sha512` integrity), and
//! Composer from getcomposer.org (checked against its published SHA-256).
//!
//! Each install gets small launchers that run it with whatever `node`/`php`
//! comes first on the project's PATH, so a package manager always runs on
//! the project's own runtime. pnpm 11+ is the exception: its CLI is a native
//! binary (from `@pnpm/exe.<target>`, checked the same way), which the
//! launchers run directly; it still runs scripts with the project's `node`.

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

/// Whether an install can't run as it is and must be installed again: a
/// pnpm 11+ set up before LocDev placed its native binary, whose launchers
/// hand the placeholder bin (an `sh` script) to `node`.
pub fn needs_reinstall(kind: RuntimeKind, dir: &Path) -> bool {
    if kind != RuntimeKind::Pnpm {
        return false;
    }
    let native = std::fs::read_to_string(dir.join("package.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|manifest| native_package(&manifest).is_some());
    let launcher = std::fs::read_to_string(executable(kind, dir)).unwrap_or_default();
    native && launcher.contains("node ")
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
    fetch_package(package, version, unpacked, scratch, progress, log)?;

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
    match native_package(&manifest) {
        // The CLI is a native binary: placed over each placeholder bin, and
        // the launchers run it directly.
        Some(native) => {
            let binary = fetch_native_binary(&native, version, scratch, progress, log)?;
            for (name, script) in &bins {
                let script = place_native(&binary, unpacked, name, script)?;
                write_launcher(unpacked, dest, name, None, &script)?;
            }
            if let Some(folder) = binary.parent() {
                let _ = std::fs::remove_dir_all(folder);
            }
        }
        None => {
            for (name, script) in &bins {
                write_launcher(unpacked, dest, name, Some("node"), script)?;
            }
        }
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

/// Downloads `package@version` from the registry, checks it against the
/// registry's `sha512` integrity and unpacks it (without the top `package/`
/// folder) into `into`.
fn fetch_package(
    package: &str,
    version: &str,
    into: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<(), RuntimeError> {
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
        let file = package.rsplit('/').next().unwrap_or(package);
        return Err(RuntimeError::Checksum {
            file: format!("{file}-{version}.tgz"),
            expected: integrity.to_string(),
            actual: format!(
                "sha512-{}",
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &digests.sha512)
            ),
        });
    }
    log("sha512 matches the registry's integrity".into());

    progress(JobState::Extracting, 0, None);
    std::fs::create_dir_all(into)?;
    extract::tar_gz(&archive, into)?;
    let _ = std::fs::remove_file(&archive);
    Ok(())
}

/// The package that carries this host's native binary, when the package is
/// a wrapper around one (pnpm 11+: `pnpm` lists `@pnpm/exe.<target>` among
/// its optional dependencies, and its `bin` entries are placeholders its
/// install script replaces with that binary).
fn native_package(manifest: &Value) -> Option<String> {
    let optional = manifest["optionalDependencies"].as_object()?;
    let name = format!("@pnpm/exe.{}", native_target()?);
    optional.contains_key(&name).then_some(name)
}

/// How pnpm names this host's build: `linux-x64`, `linux-arm64-musl`,
/// `darwin-arm64`, `win32-x64`.
fn native_target() -> Option<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "win32",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return None,
    };
    let musl = os == "linux" && is_musl();
    Some(format!("{os}-{arch}{}", if musl { "-musl" } else { "" }))
}

/// Whether this Linux runs on musl (Alpine): its loader is `/lib/ld-musl-*`.
fn is_musl() -> bool {
    std::fs::read_dir("/lib").is_ok_and(|dir| {
        dir.flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("ld-musl-"))
    })
}

/// Downloads the native package (checked like the wrapper) and returns its
/// binary, in `scratch`.
fn fetch_native_binary(
    package: &str,
    version: &str,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<PathBuf, RuntimeError> {
    let into = scratch.join("native");
    fetch_package(package, version, &into, scratch, progress, log)?;
    let binary = into.join(if cfg!(windows) { "pnpm.exe" } else { "pnpm" });
    if !binary.is_file() {
        return Err(RuntimeError::Archive(format!(
            "{package}@{version} has no pnpm binary"
        )));
    }
    log(format!("native binary from {package}"));
    Ok(binary)
}

/// Puts the native binary where bin `name` is (as pnpm's own install script
/// does: the binary tells pnpm from pnpx by its file name), hard-linked to
/// the first copy to save space. Returns the path the launcher runs,
/// relative to the package.
fn place_native(
    binary: &Path,
    unpacked: &Path,
    name: &str,
    script: &str,
) -> Result<String, RuntimeError> {
    let script = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        script.trim_start_matches("./").to_string()
    };
    let target = unpacked.join(&script);
    let _ = std::fs::remove_file(&target);
    if std::fs::hard_link(binary, &target).is_err() {
        std::fs::copy(binary, &target)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(script)
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
    write_launcher(unpacked, dest, "composer", Some("php"), "composer.phar")?;

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
/// from PATH, or directly when it's a native binary (`None`). Written into
/// `unpacked`, but pointing at `dest`, where the folder ends up.
fn write_launcher(
    unpacked: &Path,
    dest: &Path,
    name: &str,
    interpreter: Option<&str>,
    script: &str,
) -> std::io::Result<()> {
    let dir = unpacked.join(LAUNCHERS);
    std::fs::create_dir_all(&dir)?;
    let run = interpreter.map(|i| format!("{i} ")).unwrap_or_default();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let target = dest
            .join(script)
            .display()
            .to_string()
            .replace('\'', "'\\''");
        let body = format!("#!/bin/sh\n# Managed by LocDev\nexec {run}'{target}' \"$@\"\n");
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
        let body =
            format!("@ECHO off\r\nREM Managed by LocDev\r\n{run}\"%~dp0..\\{relative}\" %*\r\n");
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
    fn native_launchers_run_the_binary_directly() {
        let dir = tempfile::tempdir().unwrap();
        let unpacked = dir.path().join("scratch");
        let dest = dir.path().join("pnpm/12.23.0");
        write_launcher(&unpacked, &dest, "pnpx", None, "pnpx").unwrap();
        let body = std::fs::read_to_string(unpacked.join(LAUNCHERS).join("pnpx")).unwrap();
        assert!(body.contains(&format!("exec '{}' ", dest.join("pnpx").display())));
        assert!(!body.contains("node"));
    }

    #[test]
    fn finds_the_native_package_of_this_host() {
        let Some(target) = native_target() else {
            return;
        };
        let wrapper = serde_json::json!({
            "optionalDependencies": { format!("@pnpm/exe.{target}"): "12.23.0" }
        });
        assert_eq!(
            native_package(&wrapper),
            Some(format!("@pnpm/exe.{target}"))
        );
        // pnpm 10 and earlier: plain JavaScript bins, no native package.
        assert_eq!(
            native_package(&serde_json::json!({ "bin": { "pnpm": "bin/pnpm.cjs" } })),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_pnpm_whose_launcher_runs_the_placeholder_needs_reinstalling() {
        let Some(target) = native_target() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            serde_json::json!({ "optionalDependencies": { format!("@pnpm/exe.{target}"): "12.18.0" } })
                .to_string(),
        )
        .unwrap();
        write_launcher(dir.path(), dir.path(), "pnpm", Some("node"), "pnpm").unwrap();
        assert!(needs_reinstall(RuntimeKind::Pnpm, dir.path()));
        write_launcher(dir.path(), dir.path(), "pnpm", None, "pnpm").unwrap();
        assert!(!needs_reinstall(RuntimeKind::Pnpm, dir.path()));
        assert!(!needs_reinstall(RuntimeKind::Yarn, dir.path()));
    }

    /// Installs a real pnpm 12 from the registry and runs it. Network:
    /// `cargo test -p locdev-runtimes -- --ignored installs_native_pnpm`.
    #[cfg(unix)]
    #[test]
    #[ignore = "downloads pnpm from the npm registry"]
    fn installs_native_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("pnpm/12.23.0");
        let scratch = dir.path().join("scratch");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        install(
            RuntimeKind::Pnpm,
            "12.23.0",
            &dest,
            &scratch,
            &|_, _, _| {},
            &|l| eprintln!("{l}"),
        )
        .unwrap();
        let out = std::process::Command::new(executable(RuntimeKind::Pnpm, &dest))
            .arg("--version")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "12.23.0");
        assert!(!needs_reinstall(RuntimeKind::Pnpm, &dest));
    }

    #[cfg(unix)]
    #[test]
    fn launchers_point_at_the_final_folder() {
        let dir = tempfile::tempdir().unwrap();
        let unpacked = dir.path().join("scratch");
        let dest = dir.path().join("pnpm/9.12.0");
        write_launcher(&unpacked, &dest, "pnpm", Some("node"), "bin/pnpm.cjs").unwrap();
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
