//! PHP from the projects that publish ready-made builds, with no compiling
//! and no LocDev-hosted copies:
//!
//! - Linux and macOS: StaticPHP's prebuilt binaries (`dl.static-php.dev`,
//!   the static-php-cli project), musl/static, one `php` file per archive.
//!   `versions.json` + `version_<MM>.json` give the newest patch of each line
//!   with its SHA-256; older patches are listed by the server and each has a
//!   `.sha256` file next to it.
//! - Windows: the official builds from windows.php.net (`releases.json`,
//!   with the SHA-256 of every zip). Only the newest patch of each line has a
//!   published checksum there, so only those are offered.
//!
//! Nothing is installed without the publisher's SHA-256 matching.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use locdev_core::model::{JobState, Release};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{RuntimeError, download, extract, version};

pub const STATIC_PHP: &str = "https://dl.static-php.dev/v3/php-bin";
pub const WINDOWS_PHP: &str = "https://downloads.php.net/~windows/releases";

/// StaticPHP extension sets LocDev offers. `bulk` (default) has intl,
/// opcache, sodium, mysqli (WordPress) and imagick, and PDO's sqlite/pgsql
/// drivers through Swoole's hooks instead of `pdo_sqlite`/`pdo_pgsql`;
/// `common` is smaller, with the plain `pdo_*` extensions but none of those.
pub const FLAVORS: &[&str] = &["bulk", "common"];

/// Where PHP builds come from. Overridable in `config.toml` (`[runtimes]`)
/// for a mirror, and by tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpSource {
    /// Base of StaticPHP's `php-bin` folder.
    pub static_url: String,
    /// StaticPHP extension set (one of [`FLAVORS`]).
    pub flavor: String,
    /// Base of the official Windows releases.
    pub windows_url: String,
}

impl Default for PhpSource {
    fn default() -> Self {
        Self {
            static_url: STATIC_PHP.into(),
            flavor: FLAVORS[0].into(),
            windows_url: WINDOWS_PHP.into(),
        }
    }
}

impl PhpSource {
    fn flavor_url(&self) -> Result<String, RuntimeError> {
        if !FLAVORS.contains(&self.flavor.as_str()) {
            return Err(RuntimeError::Catalog(format!(
                "unknown PHP build set '{}' (use {})",
                self.flavor,
                FLAVORS.join(" or ")
            )));
        }
        Ok(format!(
            "{}/{}",
            self.static_url.trim_end_matches('/'),
            self.flavor
        ))
    }
}

/// StaticPHP's platform name for this machine.
fn static_platform() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("macos", "aarch64") => "macos-aarch64",
        _ => return None,
    })
}

/// windows.php.net's architecture suffix for this machine.
fn windows_arch() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("x64"),
        ("windows", "aarch64") => Some("arm64"),
        _ => None,
    }
}

pub fn supported() -> bool {
    static_platform().is_some() || windows_arch().is_some()
}

/// A build that can be installed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub version: String,
    pub file: String,
    pub url: String,
    /// From the publisher's index; `None` means "read `<url>.sha256`".
    pub sha256: Option<String>,
    pub date: String,
}

/// Versions published for this platform, newest first.
pub fn catalog(source: &PhpSource) -> Result<Vec<Release>, RuntimeError> {
    Ok(offers(source)?
        .into_iter()
        .map(|offer| Release {
            version: offer.version,
            lts: None,
            date: offer.date,
        })
        .collect())
}

pub fn offers(source: &PhpSource) -> Result<Vec<Offer>, RuntimeError> {
    let mut offers = if let Some(platform) = static_platform() {
        static_offers(source, platform)?
    } else if let Some(arch) = windows_arch() {
        let url = source.windows_url.trim_end_matches('/');
        windows_offers(
            &download::fetch_text(&format!("{url}/releases.json"))?,
            url,
            arch,
        )?
    } else {
        return Err(RuntimeError::Unsupported);
    };
    offers.sort_by(|a, b| version::compare(&b.version, &a.version));
    offers.dedup_by(|a, b| a.version == b.version);
    Ok(offers)
}

#[derive(Deserialize)]
struct Versions {
    #[serde(rename = "version-files")]
    files: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct LineFile {
    #[serde(rename = "php-version")]
    version: String,
    #[serde(rename = "built-at", default)]
    built_at: String,
    #[serde(default)]
    artifacts: BTreeMap<String, Artifacts>,
}

#[derive(Deserialize)]
struct Artifacts {
    cli: Option<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    file: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Listed {
    name: String,
    #[serde(default)]
    last_modified: String,
}

fn static_offers(source: &PhpSource, platform: &str) -> Result<Vec<Offer>, RuntimeError> {
    let base = source.flavor_url()?;
    let versions: Versions =
        serde_json::from_str(&download::fetch_text(&format!("{base}/versions.json"))?)
            .map_err(|e| RuntimeError::Catalog(format!("versions.json: {e}")))?;

    let mut offers = Vec::new();
    for file in versions.files.values() {
        if !safe_name(file) {
            continue;
        }
        let line: LineFile =
            serde_json::from_str(&download::fetch_text(&format!("{base}/{file}"))?)
                .map_err(|e| RuntimeError::Catalog(format!("{file}: {e}")))?;
        offers.extend(newest_offer(&line, &base, platform));
    }

    // Older patches: the server's own listing. Best effort — the newest of
    // each line above is enough to work.
    if let Ok(raw) =
        download::fetch_text_accept(&format!("{base}/?format=json"), "application/json")
        && let Ok(listed) = serde_json::from_str::<Vec<Listed>>(&raw)
    {
        for offer in listed_offers(&listed, &base, platform) {
            if !offers.iter().any(|o: &Offer| o.version == offer.version) {
                offers.push(offer);
            }
        }
    }
    Ok(offers)
}

fn newest_offer(line: &LineFile, base: &str, platform: &str) -> Option<Offer> {
    let cli = line.artifacts.get(platform)?.cli.as_ref()?;
    (safe_name(&cli.file) && is_version(&line.version) && is_sha256(&cli.sha256)).then(|| Offer {
        version: line.version.clone(),
        file: cli.file.clone(),
        url: format!("{base}/{}", cli.file),
        sha256: Some(cli.sha256.to_ascii_lowercase()),
        date: line.built_at.get(..10).unwrap_or_default().to_string(),
    })
}

/// `php-<ver>[-<rev>]-cli-<platform>.tar.gz` files with a `.sha256` next to
/// them; the highest revision of each version wins.
fn listed_offers(listed: &[Listed], base: &str, platform: &str) -> Vec<Offer> {
    let suffix = format!("-cli-{platform}.tar.gz");
    let mut best: BTreeMap<String, (u32, &Listed)> = BTreeMap::new();
    for entry in listed {
        let Some(middle) = entry
            .name
            .strip_prefix("php-")
            .and_then(|rest| rest.strip_suffix(&suffix))
        else {
            continue;
        };
        let (version, revision) = match middle.split_once('-') {
            Some((v, rev)) => match rev.parse::<u32>() {
                Ok(rev) => (v, rev),
                Err(_) => continue,
            },
            None => (middle, 0),
        };
        let checksum = format!("{}.sha256", entry.name);
        if !is_version(version) || !listed.iter().any(|e| e.name == checksum) {
            continue;
        }
        if best.get(version).is_none_or(|(rev, _)| revision > *rev) {
            best.insert(version.to_string(), (revision, entry));
        }
    }
    best.into_iter()
        .map(|(version, (_, entry))| Offer {
            version,
            file: entry.name.clone(),
            url: format!("{base}/{}", entry.name),
            sha256: None,
            date: entry
                .last_modified
                .get(..10)
                .unwrap_or_default()
                .to_string(),
        })
        .collect()
}

/// Thread-safe (`nts`) builds for `arch` from windows.php.net's index.
fn windows_offers(raw: &str, base: &str, arch: &str) -> Result<Vec<Offer>, RuntimeError> {
    let index: BTreeMap<String, Value> = serde_json::from_str(raw)
        .map_err(|e| RuntimeError::Catalog(format!("releases.json: {e}")))?;
    let suffix = format!("-{arch}");
    let mut offers = Vec::new();
    for line in index.values() {
        let Some(version) = line["version"].as_str().filter(|v| is_version(v)) else {
            continue;
        };
        let Some(build) = line.as_object().and_then(|builds| {
            builds
                .iter()
                .find(|(key, _)| key.starts_with("nts-") && key.ends_with(&suffix))
                .map(|(_, build)| build)
        }) else {
            continue;
        };
        let (Some(file), Some(sha256)) = (
            build["zip"]["path"].as_str(),
            build["zip"]["sha256"].as_str(),
        ) else {
            continue;
        };
        if !safe_name(file) || !is_sha256(sha256) {
            continue;
        }
        offers.push(Offer {
            version: version.to_string(),
            file: file.to_string(),
            url: format!("{base}/{file}"),
            sha256: Some(sha256.to_ascii_lowercase()),
            date: build["mtime"]
                .as_str()
                .and_then(|d| d.get(..10))
                .unwrap_or_default()
                .to_string(),
        });
    }
    Ok(offers)
}

fn is_version(v: &str) -> bool {
    v.split('.').count() == 3
        && v.split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_sha256(v: &str) -> bool {
    v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A plain file name: nothing that would change the URL's folder.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !name.starts_with('.')
}

pub fn executable(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join("php.exe")
    } else {
        dir.join("bin").join("php")
    }
}

/// Extensions enabled on Windows, where they ship as DLLs loaded by
/// php.ini (static builds elsewhere have them compiled in).
const WINDOWS_EXTENSIONS: &[&str] = &[
    "bz2",
    "curl",
    "exif",
    "fileinfo",
    "gd",
    "gettext",
    "gmp",
    "intl",
    "mbstring",
    "mysqli",
    "openssl",
    "pdo_mysql",
    "pdo_pgsql",
    "pdo_sqlite",
    "pgsql",
    "sockets",
    "sodium",
    "sqlite3",
    "xsl",
    "zip",
];

/// Writes a php.ini next to php.exe (where PHP looks first) that points at
/// the bundled `ext/` folder and enables the extensions present there.
fn write_windows_ini(dir: &Path) -> Result<(), RuntimeError> {
    let ini = dir.join("php.ini");
    if ini.exists() {
        return Ok(());
    }
    let mut contents = std::fs::read_to_string(dir.join("php.ini-development")).unwrap_or_default();
    contents.push_str("\n; Managed by LocDev\nextension_dir = \"ext\"\n");
    for ext in WINDOWS_EXTENSIONS {
        if dir.join("ext").join(format!("php_{ext}.dll")).is_file() {
            contents.push_str(&format!("extension={ext}\n"));
        }
    }
    if dir.join("ext").join("php_opcache.dll").is_file() {
        contents.push_str("zend_extension=opcache\n");
    }
    std::fs::write(ini, contents)?;
    Ok(())
}

/// CA bundles of the common distributions and macOS, first match wins.
const CA_BUNDLES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    "/etc/ssl/ca-bundle.pem",
    "/etc/ssl/cert.pem",
];

/// php.ini next to the static binary (PHP reads the binary's folder before
/// its compiled-in `/usr/local/etc/php`). StaticPHP builds don't know the
/// system's CA bundle, so HTTPS from PHP (Composer, Guzzle) needs it named.
fn static_ini(ca_bundle: Option<&Path>) -> String {
    let mut ini = String::from("; Managed by LocDev\n");
    if let Some(ca) = ca_bundle {
        let ca = ca.display().to_string().replace('"', "");
        ini.push_str(&format!(
            "curl.cainfo = \"{ca}\"\nopenssl.cafile = \"{ca}\"\n"
        ));
    }
    ini
}

pub fn install(
    source: &PhpSource,
    version: &str,
    dest: &Path,
    scratch: &Path,
    progress: &dyn Fn(JobState, u64, Option<u64>),
    log: &dyn Fn(String),
) -> Result<(), RuntimeError> {
    let offer = offers(source)?
        .into_iter()
        .find(|o| o.version == version)
        .ok_or_else(|| {
            RuntimeError::Catalog(format!("PHP {version} isn't published for this platform"))
        })?;

    // The expected hash comes first: no checksum, no install.
    let (expected, from) = match &offer.sha256 {
        Some(sha) => (sha.clone(), "publisher's index"),
        None => {
            let listing = download::fetch_text(&format!("{}.sha256", offer.url))?;
            let sha = download::checksum_for(&listing, &offer.file).ok_or_else(|| {
                RuntimeError::Catalog(format!("{}.sha256 doesn't list {}", offer.url, offer.file))
            })?;
            (sha, "publisher's .sha256")
        }
    };
    log(format!("source {}", offer.url));
    log(format!("expected sha256 {expected} ({from})"));

    let archive = scratch.join(&offer.file);
    let actual = download::download(&offer.url, &archive, &|done, total| {
        progress(JobState::Downloading, done, total)
    })?;

    progress(JobState::Verifying, 0, None);
    if actual != expected {
        let _ = std::fs::remove_file(&archive);
        return Err(RuntimeError::Checksum {
            file: offer.file,
            expected,
            actual,
        });
    }
    log("sha256 matches".into());

    progress(JobState::Extracting, 0, None);
    let unpacked = scratch.join(format!("php-{version}"));
    if offer.file.ends_with(".zip") {
        // Official Windows zips have no top-level folder, and no php.ini.
        extract::zip(&archive, &unpacked, false)?;
        write_windows_ini(&unpacked)?;
    } else {
        // StaticPHP: a lone `php` at the archive root.
        let bin = unpacked.join("bin");
        extract::tar_gz_flat(&archive, &bin)?;
        let ca = CA_BUNDLES.iter().map(Path::new).find(|p| p.is_file());
        match ca {
            Some(ca) => log(format!("CA bundle {}", ca.display())),
            None => log("no system CA bundle found; set curl.cainfo yourself for HTTPS".into()),
        }
        std::fs::write(bin.join("php.ini"), static_ini(ca))?;
    }
    let _ = std::fs::remove_file(&archive);

    if !executable(&unpacked).is_file() {
        return Err(RuntimeError::Archive(
            "the archive has no php executable".into(),
        ));
    }
    std::fs::rename(&unpacked, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_ini_enables_bundled_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let ext = dir.path().join("ext");
        std::fs::create_dir_all(&ext).unwrap();
        for dll in ["php_mbstring.dll", "php_openssl.dll", "php_opcache.dll"] {
            std::fs::write(ext.join(dll), b"").unwrap();
        }
        std::fs::write(
            dir.path().join("php.ini-development"),
            "memory_limit = 128M\n",
        )
        .unwrap();

        write_windows_ini(dir.path()).unwrap();
        let ini = std::fs::read_to_string(dir.path().join("php.ini")).unwrap();
        assert!(ini.starts_with("memory_limit = 128M"));
        assert!(ini.contains("extension_dir = \"ext\""));
        assert!(ini.contains("extension=mbstring\n") && ini.contains("extension=openssl\n"));
        assert!(!ini.contains("extension=intl"), "only DLLs that exist");
        assert!(ini.contains("zend_extension=opcache"));

        // An existing php.ini is never overwritten.
        std::fs::write(dir.path().join("php.ini"), "custom").unwrap();
        write_windows_ini(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("php.ini")).unwrap(),
            "custom"
        );
    }

    /// Trimmed from the real `common/version_85.json` (2026-11-22).
    const LINE_85: &str = r#"{
      "schema-version": 1, "combination": "common", "minor": "8.5",
      "php-version": "8.5.13", "built-at": "2026-11-22T12:31:50Z",
      "artifacts": {
        "linux-x86_64": { "revision": 0, "cli": {
          "file": "php-8.5.13-cli-linux-x86_64.tar.gz", "size": 33163648,
          "sha256": "06CE9FDA757C037871870090F9E85EB7289939D387C19E4F0EFC608BEDC51F55" } },
        "macos-aarch64": { "revision": 0, "fpm": {
          "file": "php-8.5.13-fpm-macos-aarch64.tar.gz", "size": 1,
          "sha256": "120c9da77b7f4d3a4739b787b268b6bf4c41991c5a9e782bbb50d356d72a8658" } }
      }
    }"#;

    #[test]
    fn reads_the_newest_build_of_a_line() {
        let line: LineFile = serde_json::from_str(LINE_85).unwrap();
        let offer = newest_offer(&line, "https://x/common", "linux-x86_64").unwrap();
        assert_eq!(offer.version, "8.5.13");
        assert_eq!(
            offer.url,
            "https://x/common/php-8.5.13-cli-linux-x86_64.tar.gz"
        );
        assert_eq!(
            offer.sha256.as_deref(),
            Some("06ce9fda757c037871870090f9e85eb7289939d387c19e4f0efc608bedc51f55")
        );
        assert_eq!(offer.date, "2026-11-22");
        // No build for the platform, or only other SAPIs: nothing offered.
        assert!(newest_offer(&line, "https://x/common", "linux-aarch64").is_none());
        assert!(newest_offer(&line, "https://x/common", "macos-aarch64").is_none());
    }

    #[test]
    fn listed_builds_need_a_checksum_file() {
        let listed: Vec<Listed> = serde_json::from_str(
            r#"[
            {"name":"php-8.4.20-cli-linux-x86_64.tar.gz","last_modified":"2026-09-01 03:00:00"},
            {"name":"php-8.4.20-cli-linux-x86_64.tar.gz.sha256","last_modified":"2026-09-01 03:00:00"},
            {"name":"php-8.4.21-cli-linux-x86_64.tar.gz","last_modified":"2026-09-10 03:00:00"},
            {"name":"php-8.4.27-cli-linux-x86_64.tar.gz","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.27-cli-linux-x86_64.tar.gz.sha256","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.27-1-cli-linux-x86_64.tar.gz","last_modified":"2026-11-16 03:00:00"},
            {"name":"php-8.4.27-1-cli-linux-x86_64.tar.gz.sha256","last_modified":"2026-11-16 03:00:00"},
            {"name":"php-8.4.27-fpm-linux-x86_64.tar.gz","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.27-fpm-linux-x86_64.tar.gz.sha256","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.27-cli-linux-x86_64-gnu.tar.gz","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.27-cli-linux-x86_64-gnu.tar.gz.sha256","last_modified":"2026-11-13 03:00:00"},
            {"name":"php-8.4.x-cli-linux-x86_64.tar.gz","last_modified":""},
            {"name":"php-8.4.x-cli-linux-x86_64.tar.gz.sha256","last_modified":""}
        ]"#,
        )
        .unwrap();
        let offers = listed_offers(&listed, "https://x/common", "linux-x86_64");
        let found: Vec<(&str, &str)> = offers
            .iter()
            .map(|o| (o.version.as_str(), o.file.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                ("8.4.20", "php-8.4.20-cli-linux-x86_64.tar.gz"),
                // The republished revision replaces the first one.
                ("8.4.27", "php-8.4.27-1-cli-linux-x86_64.tar.gz"),
            ],
            "8.4.21 has no checksum file; other SAPIs, libcs and odd names are ignored"
        );
        assert!(offers.iter().all(|o| o.sha256.is_none()));
        assert_eq!(offers[0].date, "2026-09-01");
    }

    #[test]
    fn windows_builds_are_the_official_nts_zips() {
        // Shape of downloads.php.net/~windows/releases/releases.json.
        let raw = r#"{
          "8.4": {
            "version": "8.4.28",
            "ts-vs17-x64": { "mtime": "2026-11-20T10:00:00+00:00",
              "zip": { "path": "php-8.4.28-Win32-vs17-x64.zip", "size": "32MB",
                       "sha256": "59ccaa095d0f0c3fbb9f7e2bfd86713ebac032d5783cc22bc8934e2285629999" } },
            "nts-vs17-x64": { "mtime": "2026-11-20T10:00:00+00:00",
              "zip": { "path": "php-8.4.28-nts-Win32-vs17-x64.zip", "size": "32MB",
                       "sha256": "c1f9280588b7b6ad7b518969022f0b0175f106fab9d0639ad0e4ce7cd47ac1ed" } },
            "nts-vs17-x86": { "mtime": "2026-11-20T10:00:00+00:00",
              "zip": { "path": "php-8.4.28-nts-Win32-vs17-x86.zip", "size": "30MB",
                       "sha256": "d0370400d24ccdbb0b1d2629f22395acf2a511c1df22eb8422edbb99357cd368" } }
          },
          "8.5": {
            "version": "8.5.13",
            "nts-vs17-x64": { "mtime": "2026-11-20T10:00:00+00:00",
              "zip": { "path": "../evil.zip", "size": "1MB",
                       "sha256": "4ae6c72bc0ca4f6b8f3e9b107874bae0135699f3aabe6c26a1f6103d040eea97" } }
          }
        }"#;
        let offers = windows_offers(raw, "https://w", "x64").unwrap();
        assert_eq!(offers.len(), 1, "a path that leaves the folder is dropped");
        assert_eq!(offers[0].version, "8.4.28");
        assert_eq!(offers[0].url, "https://w/php-8.4.28-nts-Win32-vs17-x64.zip");
        assert_eq!(offers[0].date, "2026-11-20");
        assert!(
            windows_offers(raw, "https://w", "arm64")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn static_ini_points_at_the_ca_bundle() {
        let ini = static_ini(Some(Path::new("/etc/ssl/certs/ca-certificates.crt")));
        assert!(ini.starts_with("; Managed by LocDev\n"));
        assert!(ini.contains("curl.cainfo = \"/etc/ssl/certs/ca-certificates.crt\""));
        assert!(ini.contains("openssl.cafile = \"/etc/ssl/certs/ca-certificates.crt\""));
        assert!(!static_ini(None).contains("cainfo"));
    }

    #[test]
    fn unknown_build_sets_are_refused() {
        let source = PhpSource {
            flavor: "../x".into(),
            ..PhpSource::default()
        };
        assert!(source.flavor_url().is_err());
        assert_eq!(
            PhpSource::default().flavor_url().unwrap(),
            "https://dl.static-php.dev/v3/php-bin/bulk"
        );
    }
}
