//! LocDev's own published releases, read anonymously (no token is ever
//! sent): what the update check compares this daemon/CLI against.

use std::time::Duration;

use locdev_core::update::ReleaseInfo;
use serde::Deserialize;

/// The release asset that names the versions inside a release.
pub const VERSIONS_ASSET: &str = "versions.json";
/// The app updater's manifest (written by tauri-action).
pub const UPDATER_ASSET: &str = "latest.json";
/// Notes longer than this are cut (the release page has the rest).
const NOTES_MAX: usize = 4000;

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Versions {
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub daemon: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(concat!("locdev/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn get(url: &str) -> Result<String, String> {
    agent()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => format!(
                "no published release found ({url}); the repository may be private or have none yet"
            ),
            ureq::Error::StatusCode(403 | 429) => {
                "GitHub's anonymous request limit was reached; try again in an hour".into()
            }
            other => format!("{url}: {other}"),
        })?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("{url}: {e}"))
}

/// Published releases from `{api}/repos/{repo}/releases` (drafts left out),
/// newest first, with the versions of the newest stable one filled in from
/// its `versions.json`. Blocking.
pub fn list(api: &str, repo: &str) -> Result<Vec<ReleaseInfo>, String> {
    let raw = get(&format!("{api}/repos/{repo}/releases?per_page=15"))?;
    let mut releases = parse(&raw, repo, api)?;
    if let Some((index, url)) = releases.iter().enumerate().find_map(|(i, (r, url))| {
        (!r.prerelease)
            .then_some(url.clone())
            .flatten()
            .map(|url| (i, url))
    }) {
        // Unreadable: the versions stay unknown and the tag still counts.
        if let Ok(versions) = get(&url)
            .and_then(|raw| serde_json::from_str::<Versions>(&raw).map_err(|e| e.to_string()))
        {
            releases[index].0.app = versions.app;
            releases[index].0.daemon = versions.daemon;
        }
    }
    Ok(releases.into_iter().map(|(r, _)| r).collect())
}

/// The releases, each with the address of its `versions.json` when it has
/// one from GitHub's download host (or the test API).
pub fn parse(
    raw: &str,
    repo: &str,
    api: &str,
) -> Result<Vec<(ReleaseInfo, Option<String>)>, String> {
    let releases: Vec<ApiRelease> =
        serde_json::from_str(raw).map_err(|e| format!("unexpected answer from GitHub: {e}"))?;
    let download = format!("https://github.com/{repo}/releases/download/");
    let trusted = |url: &str| {
        url.starts_with(&download)
            || (!api.starts_with("https://api.github.com") && url.starts_with(api))
    };
    Ok(releases
        .into_iter()
        // Only app releases (`v1.2.3`): the same repository also carries the
        // `definitions` release of signed framework definitions.
        .filter(|r| !r.draft && is_release_tag(&r.tag_name))
        .map(|r| {
            let asset = |name: &str| {
                r.assets
                    .iter()
                    .find(|a| a.name == name && trusted(&a.browser_download_url))
            };
            let versions = asset(VERSIONS_ASSET).map(|a| a.browser_download_url.clone());
            let updater = asset(UPDATER_ASSET).is_some();
            let mut notes = r.body.unwrap_or_default().trim().to_string();
            if notes.len() > NOTES_MAX {
                let mut cut = NOTES_MAX;
                while !notes.is_char_boundary(cut) {
                    cut -= 1;
                }
                notes.truncate(cut);
                notes.push('…');
            }
            (
                ReleaseInfo {
                    tag: r.tag_name,
                    app: None,
                    daemon: None,
                    published_at: r.published_at,
                    url: r.html_url,
                    notes,
                    prerelease: r.prerelease,
                    updater,
                },
                versions,
            )
        })
        .collect())
}

/// `v` + a dotted version (`v0.17.0`, `v1.0.0-rc.1`).
fn is_release_tag(tag: &str) -> bool {
    tag.strip_prefix('v')
        .and_then(|v| v.split(['-', '+']).next())
        .is_some_and(|core| {
            core.split('.').count() == 3
                && core
                    .split('.')
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_version_tags_are_app_releases() {
        assert!(is_release_tag("v0.17.0"));
        assert!(is_release_tag("v1.0.0-rc.1"));
        assert!(!is_release_tag("definitions"));
        assert!(!is_release_tag("v1.0"));
        assert!(!is_release_tag("vx.y.z"));
    }

    #[test]
    fn drafts_are_left_out_and_assets_must_come_from_github() {
        let raw = r#"[
          {"tag_name":"definitions","html_url":"https://github.com/o/r/releases/tag/definitions","assets":[]},
          {"tag_name":"v0.15.0","draft":true,"html_url":"https://github.com/o/r/releases/tag/v0.15.0","assets":[]},
          {"tag_name":"v0.14.0","prerelease":false,"published_at":"2026-11-04T10:00:00Z","html_url":"https://github.com/o/r/releases/tag/v0.14.0","body":"  Notes  ",
           "assets":[{"name":"versions.json","browser_download_url":"https://github.com/o/r/releases/download/v0.14.0/versions.json"},
                     {"name":"latest.json","browser_download_url":"https://github.com/o/r/releases/download/v0.14.0/latest.json"}]},
          {"tag_name":"v0.13.0","html_url":"https://github.com/o/r/releases/tag/v0.13.0",
           "assets":[{"name":"versions.json","browser_download_url":"https://evil.example/versions.json"}]}
        ]"#;
        let releases = parse(raw, "o/r", "https://api.github.com").unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].0.tag, "v0.14.0");
        assert_eq!(releases[0].0.notes, "Notes");
        assert!(releases[0].0.updater);
        assert!(
            releases[0]
                .1
                .as_deref()
                .unwrap()
                .ends_with("/v0.14.0/versions.json")
        );
        assert_eq!(releases[1].1, None, "an asset elsewhere is ignored");
        assert!(!releases[1].0.updater);
    }
}
