//! Node.js from the official distribution (nodejs.org/dist): the release
//! index for the catalog, `SHASUMS256.txt` to verify every download.

use std::path::{Path, PathBuf};

use locdev_core::model::{JobState, Release};
use serde::Deserialize;

use crate::{RuntimeError, download, extract};

const DIST: &str = "https://nodejs.org/dist";

/// `(platform used in file names, key in index.json "files", archive extension)`.
fn platform() -> Option<(&'static str, &'static str, &'static str)> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => ("linux-x64", "linux-x64", "tar.gz"),
        ("linux", "aarch64") => ("linux-arm64", "linux-arm64", "tar.gz"),
        ("macos", "x86_64") => ("darwin-x64", "osx-x64-tar", "tar.gz"),
        ("macos", "aarch64") => ("darwin-arm64", "osx-arm64-tar", "tar.gz"),
        ("windows", "x86_64") => ("win-x64", "win-x64-zip", "zip"),
        ("windows", "aarch64") => ("win-arm64", "win-arm64-zip", "zip"),
        _ => return None,
    })
}

pub fn supported() -> bool {
    platform().is_some()
}

#[derive(Deserialize)]
struct IndexEntry {
    version: String,
    date: String,
    files: Vec<String>,
    /// `false`, or the LTS codename.
    lts: serde_json::Value,
}

/// Stable releases built for this platform, newest first.
pub fn catalog() -> Result<Vec<Release>, RuntimeError> {
    let (_, key, _) = platform().ok_or(RuntimeError::Unsupported)?;
    let index: Vec<IndexEntry> =
        serde_json::from_str(&download::fetch_text(&format!("{DIST}/index.json"))?)
            .map_err(|e| RuntimeError::Catalog(e.to_string()))?;

    Ok(index
        .into_iter()
        .filter(|entry| !entry.version.contains('-') && entry.files.iter().any(|f| f == key))
        .map(|entry| Release {
            version: entry.version.trim_start_matches('v').to_string(),
            lts: entry.lts.as_str().map(str::to_string),
            date: entry.date,
        })
        .collect())
}

/// The executable inside an installed version's folder.
pub fn executable(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join("node.exe")
    } else {
        dir.join("bin").join("node")
    }
}

/// Downloads, verifies and unpacks `version` into `dest` (which must not
/// exist yet). `scratch` holds the archive while it is processed.
pub fn install(
    version: &str,
    dest: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<(), RuntimeError> {
    let (platform, _, ext) = platform().ok_or(RuntimeError::Unsupported)?;
    let file = format!("node-v{version}-{platform}.{ext}");
    let base = format!("{DIST}/v{version}");

    // The expected hash comes first: no checksum, no install.
    let sums = download::fetch_text(&format!("{base}/SHASUMS256.txt"))?;
    let expected = download::checksum_for(&sums, &file)
        .ok_or_else(|| RuntimeError::Catalog(format!("{file} is not listed in SHASUMS256.txt")))?;
    log(format!("source {base}/{file}"));
    log(format!("expected sha256 {expected} (SHASUMS256.txt)"));

    let archive = scratch.join(&file);
    let actual = download::download(&format!("{base}/{file}"), &archive, &|done, total| {
        progress(JobState::Downloading, done, total)
    })?;

    progress(JobState::Verifying, 0, None);
    if actual != expected {
        let _ = std::fs::remove_file(&archive);
        return Err(RuntimeError::Checksum {
            file,
            expected,
            actual,
        });
    }

    log("sha256 matches".into());

    progress(JobState::Extracting, 0, None);
    let unpacked = scratch.join(format!("node-{version}"));
    if ext == "zip" {
        extract::zip(&archive, &unpacked, true)?;
    } else {
        extract::tar_gz(&archive, &unpacked)?;
    }
    let _ = std::fs::remove_file(&archive);

    if !executable(&unpacked).is_file() {
        return Err(RuntimeError::Archive(
            "the archive has no node executable".into(),
        ));
    }
    std::fs::rename(&unpacked, dest)?;
    Ok(())
}
