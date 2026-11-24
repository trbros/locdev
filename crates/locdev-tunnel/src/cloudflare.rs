//! Cloudflare Tunnel in Quick Tunnel mode (`cloudflared tunnel --url`):
//! a random `*.trycloudflare.com` address, no account and no domain.
//! Facts from the docs (checked 2026-11-04):
//! <https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/>
//! and from cloudflared's source (`cmd/cloudflared/tunnel/cmd.go`,
//! `metrics/metrics.go`, `RELEASE_NOTES`).

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use locdev_core::tunnel::TunnelProvider;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const ID: &str = "cloudflare";
const REPO: &str = "cloudflare/cloudflared";
/// `--allowed-mail` (e-mail login in front of a Quick Tunnel) arrived here.
pub const MIN_VERSION: &str = "2026.9.2";

pub fn provider() -> TunnelProvider {
    TunnelProvider {
        id: ID.into(),
        name: "Cloudflare Tunnel (Quick Tunnel)".into(),
        description: "A random trycloudflare.com address for the site, through Cloudflare's network. No account and no domain.".into(),
        docs: "https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/".into(),
        license: "Apache-2.0 (cloudflared)".into(),
        account: false,
        limits: vec![
            "For testing and development, with no uptime guarantee; use is subject to Cloudflare's Online Services Terms of Use.".into(),
            "Up to 200 requests at the same time (more get 429); Server-Sent Events don't work.".into(),
            "The address changes every time the tunnel starts and stops working when it stops.".into(),
            "Anyone with the address can open the site, unless you limit it to e-mail addresses (they confirm with a one-time PIN).".into(),
        ],
        program: None,
        installable: asset().is_some(),
        install_note: asset()
            .is_none()
            .then(|| "Cloudflare publishes no cloudflared build for this system.".into()),
    }
}

/// The release file for this machine and whether it's a .tgz.
pub fn asset() -> Option<(&'static str, bool)> {
    asset_for(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn asset_for(os: &str, arch: &str) -> Option<(&'static str, bool)> {
    Some(match (os, arch) {
        ("linux", "x86_64") => ("cloudflared-linux-amd64", false),
        ("linux", "aarch64") => ("cloudflared-linux-arm64", false),
        ("linux", "arm") => ("cloudflared-linux-arm", false),
        ("linux", "x86") => ("cloudflared-linux-386", false),
        ("macos", "x86_64") => ("cloudflared-darwin-amd64.tgz", true),
        ("macos", "aarch64") => ("cloudflared-darwin-arm64.tgz", true),
        ("windows", "x86_64") => ("cloudflared-windows-amd64.exe", false),
        ("windows", "x86") => ("cloudflared-windows-386.exe", false),
        _ => return None,
    })
}

pub fn executable_name() -> &'static str {
    if cfg!(windows) {
        "cloudflared.exe"
    } else {
        "cloudflared"
    }
}

/// `name: hex` lines under "SHA256 Checksums" in a release's notes.
pub fn checksums_in_notes(body: &str) -> BTreeMap<String, String> {
    body.lines()
        .filter_map(|line| {
            let (name, hex) = line.trim().split_once(':')?;
            let hex = hex.trim().to_ascii_lowercase();
            (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()))
                .then(|| (name.trim().to_string(), hex))
        })
        .collect()
}

/// The checksum to verify against: the one in Cloudflare's release notes
/// and the one GitHub computed for the uploaded file must both exist and
/// agree; anything else is refused with the reason.
pub fn expected_sha256(notes: Option<&str>, digest: Option<&str>) -> Result<String, String> {
    let digest = digest
        .and_then(|d| d.strip_prefix("sha256:"))
        .map(str::to_ascii_lowercase);
    match (notes, digest) {
        (Some(notes), Some(digest)) if notes.eq_ignore_ascii_case(&digest) => Ok(digest),
        (Some(_), Some(_)) => Err("the checksum in Cloudflare's release notes doesn't match the file GitHub serves; not installing it (install cloudflared from your package manager instead)".into()),
        (None, _) => Err("Cloudflare's release notes list no checksum for this file; not installing it".into()),
        (_, None) => Err("GitHub reports no checksum for this file; not installing it".into()),
    }
}

/// What a release offers for this machine.
#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub file: String,
    pub tgz: bool,
    pub sha256: Result<String, String>,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .build()
        .into()
}

/// The latest release (GitHub's API). Blocking.
pub fn latest() -> Result<Release, String> {
    let (file, tgz) = asset().ok_or("Cloudflare publishes no cloudflared build for this system")?;
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let raw = agent()
        .get(&url)
        .header("User-Agent", "LocDev")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))?;
    let release: ApiRelease = serde_json::from_str(&raw).map_err(|e| format!("{url}: {e}"))?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == file)
        .ok_or_else(|| format!("release {} has no {file}", release.tag_name))?;
    if !asset
        .browser_download_url
        .starts_with(&format!("https://github.com/{REPO}/releases/download/"))
    {
        return Err(format!("unexpected download address for {file}"));
    }
    let notes = checksums_in_notes(&release.body);
    Ok(Release {
        version: release.tag_name.clone(),
        url: asset.browser_download_url.clone(),
        file: file.into(),
        tgz,
        sha256: expected_sha256(notes.get(file).map(String::as_str), asset.digest.as_deref()),
    })
}

/// Downloads `release`, checks its sha256 and installs the program under
/// `dir/<version>/`. Blocking. Nothing is kept when the check fails.
pub fn install(release: &Release, dir: &Path) -> Result<PathBuf, String> {
    let expected = release.sha256.clone()?;
    let mut bytes = Vec::new();
    agent()
        .get(&release.url)
        .header("User-Agent", "LocDev")
        .call()
        .map_err(|e| format!("{}: {e}", release.url))?
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", release.url))?;
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != expected {
        return Err(format!(
            "{} doesn't match its checksum (expected {expected}, got {actual}); nothing was installed",
            release.file
        ));
    }
    let target = dir.join(&release.version);
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let exe = target.join(executable_name());
    let program = if release.tgz {
        // The macOS archive holds the program alone.
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_slice()));
        let mut found = None;
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            if path.file_name().and_then(|n| n.to_str()) == Some("cloudflared")
                && entry.header().entry_type().is_file()
            {
                let mut data = Vec::new();
                entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
                found = Some(data);
                break;
            }
        }
        found.ok_or("the archive has no cloudflared program")?
    } else {
        bytes
    };
    std::fs::write(&exe, program).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    Ok(exe)
}

/// `cloudflared version 2026.10.7 (built …)` → `2026.10.7`.
pub fn parse_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && w.contains('.'))
        .map(str::to_string)
}

/// The arguments for one site's Quick Tunnel. The tunnel forwards to the
/// site's own server only (never the proxy, a database or the panel),
/// with the site's domain as Host so frameworks that check it accept it.
pub fn args(target_port: u16, host: &str, metrics_port: u16, allowed: &[String]) -> Vec<String> {
    let mut args = vec![
        "tunnel".to_string(),
        "--no-autoupdate".into(),
        "--metrics".into(),
        format!("127.0.0.1:{metrics_port}"),
        "--url".into(),
        format!("http://127.0.0.1:{target_port}"),
        "--http-host-header".into(),
        host.into(),
    ];
    for email in allowed {
        args.push("--allowed-mail".into());
        args.push(email.clone());
    }
    args
}

/// The address from cloudflared's metrics server (`/quicktunnel` answers
/// `{"hostname":"…"}`; empty until the tunnel is up).
pub fn hostname_from_metrics(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Quick {
        hostname: String,
    }
    let quick: Quick = serde_json::from_str(body).ok()?;
    let host = quick.hostname.trim().trim_start_matches("https://");
    let valid = host.ends_with(".trycloudflare.com")
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    valid.then(|| format!("https://{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: &str = "### SHA256 Checksums:\n```\ncloudflared-darwin-arm64.tgz: 2136431d5eb467bf447d2ee514b713e192719b143aec1d9e09bf7dc290d5bb12\ncloudflared-linux-amd64: 1f99fb518b83dec2ba8d870a7cc991d82a77e6196295a3d2754ba17ab9f91872\n```\n";

    #[test]
    fn checksums_must_agree() {
        let notes = checksums_in_notes(NOTES);
        assert_eq!(notes.len(), 2);
        // Linux: the notes and GitHub agree (release 2026.10.7).
        assert_eq!(
            expected_sha256(
                notes.get("cloudflared-linux-amd64").map(String::as_str),
                Some("sha256:1F99FB518B83DEC2BA8D870A7CC991D82A77E6196295A3D2754BA17AB9F91872"),
            )
            .unwrap(),
            "1f99fb518b83dec2ba8d870a7cc991d82a77e6196295a3d2754ba17ab9f91872"
        );
        // macOS arm64 in that same release: they don't; refused.
        let refused = expected_sha256(
            notes
                .get("cloudflared-darwin-arm64.tgz")
                .map(String::as_str),
            Some("sha256:3586ccf397e98e5f437e60e7c307ca6ac463e110580503af8bb8e6f9ebf831bd"),
        )
        .unwrap_err();
        assert!(refused.contains("doesn't match"));
        assert!(expected_sha256(None, Some("sha256:00")).is_err());
        assert!(
            expected_sha256(
                notes.get("cloudflared-linux-amd64").map(String::as_str),
                None
            )
            .is_err()
        );
    }

    #[test]
    fn assets_per_system() {
        assert_eq!(
            asset_for("linux", "x86_64"),
            Some(("cloudflared-linux-amd64", false))
        );
        assert_eq!(
            asset_for("macos", "aarch64"),
            Some(("cloudflared-darwin-arm64.tgz", true))
        );
        assert_eq!(
            asset_for("windows", "x86_64"),
            Some(("cloudflared-windows-amd64.exe", false))
        );
        assert_eq!(asset_for("freebsd", "x86_64"), None);
    }

    #[test]
    fn arguments_point_at_the_site_only() {
        let a = args(8100, "shop.test", 9123, &["ana@example.com".into()]);
        assert_eq!(
            a,
            [
                "tunnel",
                "--no-autoupdate",
                "--metrics",
                "127.0.0.1:9123",
                "--url",
                "http://127.0.0.1:8100",
                "--http-host-header",
                "shop.test",
                "--allowed-mail",
                "ana@example.com"
            ]
        );
    }

    #[test]
    fn metrics_and_versions() {
        assert_eq!(
            hostname_from_metrics(r#"{"hostname":"calm-fox-river.trycloudflare.com"}"#).as_deref(),
            Some("https://calm-fox-river.trycloudflare.com")
        );
        assert_eq!(hostname_from_metrics(r#"{"hostname":""}"#), None);
        assert_eq!(
            hostname_from_metrics(r#"{"hostname":"evil.example.com"}"#),
            None
        );
        assert_eq!(
            parse_version("cloudflared version 2026.10.7 (built 2026-10-30-1711 UTC)").as_deref(),
            Some("2026.10.7")
        );
    }

    #[test]
    fn installs_only_a_matching_file() {
        let dir = tempfile::tempdir().unwrap();
        let release = Release {
            version: "2026.10.7".into(),
            url: "https://invalid.example/never-fetched".into(),
            file: "cloudflared-linux-amd64".into(),
            tgz: false,
            sha256: Err("checksums disagree".into()),
        };
        // A refused checksum stops before any download.
        assert_eq!(
            install(&release, dir.path()).unwrap_err(),
            "checksums disagree"
        );
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    }
}
