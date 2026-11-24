//! Updates: the published LocDev releases on GitHub and how this daemon
//! compares to them. The app updates itself through the Tauri updater (its
//! shell, signed `latest.json`); the daemon only reads what was released.

use serde::{Deserialize, Serialize};

/// One published release (drafts never show up).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// `v0.14.0`: the app's version (the release workflow refuses others).
    pub tag: String,
    /// App and daemon/CLI versions, from the release's `versions.json`.
    /// `None` for releases made before it existed (or when it couldn't be read).
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub daemon: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    /// The release page.
    pub url: String,
    /// Release notes, trimmed.
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub prerelease: bool,
    /// Whether it carries the app updater's `latest.json` (so the app can
    /// install it, or go back to it).
    #[serde(default)]
    pub updater: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStatus {
    /// `owner/name` the releases come from.
    pub repo: String,
    /// This daemon's (and CLI's) version.
    pub daemon: String,
    /// The newest stable release, if any was published.
    #[serde(default)]
    pub latest: Option<ReleaseInfo>,
    /// Recent releases, newest first (prereleases included, marked).
    #[serde(default)]
    pub releases: Vec<ReleaseInfo>,
    /// The latest release ships a newer daemon/CLI than this one.
    #[serde(default)]
    pub daemon_update: bool,
    /// When GitHub was last asked (ms since the epoch); `None` = never.
    #[serde(default)]
    pub checked_at: Option<u64>,
    /// Why the last check failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// `v1.2.3` → `1.2.3`; anything else stays as it is.
pub fn tag_version(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// Whether `candidate` is a newer version than `current` (semver order;
/// unparsable versions are never "newer").
pub fn is_newer(candidate: &str, current: &str) -> bool {
    use crate::semver::Version;
    match (
        Version::parse(tag_version(candidate)),
        Version::parse(tag_version(current)),
    ) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// A release tag the app may ask to install: `v` + a version, nothing that
/// could change the URL it goes into.
pub fn valid_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('v') else {
        return false;
    };
    !rest.is_empty()
        && rest.len() <= 40
        && rest.starts_with(|c: char| c.is_ascii_digit())
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
        && crate::semver::Version::parse(rest).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("v0.14.0", "0.13.2"));
        assert!(is_newer("1.0.0", "1.0.0-beta.2"));
        assert!(!is_newer("v0.13.0", "0.13.0"));
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(valid_tag("v0.14.0"));
        assert!(valid_tag("v1.0.0-rc.1"));
        assert!(!valid_tag("0.14.0"));
        assert!(!valid_tag("v0.14.0/../../x"));
        assert!(!valid_tag("v"));
        assert!(!valid_tag("vlatest"));
    }
}
