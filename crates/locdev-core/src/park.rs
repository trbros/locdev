//! Parked folders: folders whose project subfolders become sites on their
//! own (`~/Development/shop` → `shop.test`), kept in step as projects come
//! and go. Discovery lives in `locdev-frameworks::discover`; the daemon owns
//! the list (`parked.toml`), the scans and the file watching.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::ProjectKind;

/// The deepest a parked folder is searched (1 = its direct subfolders).
pub const MAX_DEPTH: u8 = 3;

/// One parked folder and its rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedDir {
    /// Absolute and canonical.
    pub path: PathBuf,
    /// Off: no scans and no watching; its sites stay as they are.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// How many levels below the folder to look (1 to `MAX_DEPTH`). A
    /// project's own subfolders are never searched.
    #[serde(default = "one")]
    pub depth: u8,
    /// Folder names (or paths relative to the parked folder) to skip; `*`
    /// matches any run of characters (`archive-*`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// Projects the user doesn't want as sites, relative to the folder.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored: Vec<String>,
    #[serde(default)]
    pub on_new: OnNew,
    #[serde(default)]
    pub on_missing: OnMissing,
}

fn yes() -> bool {
    true
}

fn one() -> u8 {
    1
}

impl ParkedDir {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            enabled: true,
            depth: 1,
            exclude: Vec::new(),
            ignored: Vec::new(),
            on_new: OnNew::default(),
            on_missing: OnMissing::default(),
        }
    }
}

/// What happens to a project that shows up in a parked folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnNew {
    /// It becomes a site right away (stopped; nothing runs on its own).
    #[default]
    Register,
    /// It is listed, and becomes a site only when the user adds it.
    Suggest,
}

/// What happens to a site from a parked folder whose folder is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnMissing {
    /// The site is removed from LocDev (its files are already gone).
    #[default]
    Unregister,
    /// The site stays, marked as missing, until it is removed by hand.
    Keep,
}

/// Where a project found in a parked folder stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParkedState {
    /// A site that came from this folder.
    Registered,
    /// Already a site, added by hand (left as it is).
    Manual,
    /// Waiting for the user to add it (`on_new = suggest`).
    Suggested,
    /// The user doesn't want it as a site.
    Ignored,
    /// Could not become a site (no free name, invalid name…); see `note`.
    Failed,
}

/// A project found in a parked folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedProject {
    pub path: PathBuf,
    /// Relative to the parked folder, with `/`.
    pub rel: String,
    /// The site's name (registered) or the one it would get.
    pub name: String,
    pub kind: ProjectKind,
    /// "Laravel + Vite", "Next.js"…
    pub framework: String,
    pub state: ParkedState,
    /// Why the name differs from the folder's, or why it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Something a scan could not read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkError {
    pub path: PathBuf,
    pub message: String,
}

/// A parked folder with what its last scan found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedView {
    #[serde(flatten)]
    pub dir: ParkedDir,
    pub exists: bool,
    /// Changes are picked up as they happen (else only by rescans).
    pub watching: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch_error: Option<String>,
    /// Unix milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_scan: Option<u64>,
    #[serde(default)]
    pub projects: Vec<ParkedProject>,
    #[serde(default)]
    pub errors: Vec<ParkError>,
}

/// The rules the user can change, all optional (unset = keep).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParkOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_new: Option<OnNew>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_missing: Option<OnMissing>,
}

impl ParkOptions {
    pub fn apply(&self, dir: &mut ParkedDir) -> Result<(), String> {
        if let Some(depth) = self.depth {
            if !(1..=MAX_DEPTH).contains(&depth) {
                return Err(format!("depth must be between 1 and {MAX_DEPTH}"));
            }
            dir.depth = depth;
        }
        if let Some(exclude) = &self.exclude {
            let mut clean = Vec::new();
            for pattern in exclude {
                let pattern = pattern.trim().trim_matches('/').replace('\\', "/");
                if pattern.is_empty() {
                    continue;
                }
                if pattern.split('/').any(|part| part == "..") {
                    return Err(format!("'{pattern}' can't go outside the folder"));
                }
                if !clean.contains(&pattern) {
                    clean.push(pattern);
                }
            }
            dir.exclude = clean;
        }
        if let Some(enabled) = self.enabled {
            dir.enabled = enabled;
        }
        if let Some(on_new) = self.on_new {
            dir.on_new = on_new;
        }
        if let Some(on_missing) = self.on_missing {
            dir.on_missing = on_missing;
        }
        Ok(())
    }
}

/// Whether `pattern` (with `*` wildcards) matches all of `text`.
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || text.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    if !text.ends_with(last) {
        return false;
    }
    for middle in &parts[1..parts.len() - 1] {
        match rest.find(middle) {
            Some(at) => rest = &rest[at + middle.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_matches("archive", "archive"));
        assert!(!glob_matches("archive", "archive-2"));
        assert!(glob_matches("archive-*", "archive-2024"));
        assert!(glob_matches("*-old", "shop-old"));
        assert!(glob_matches("a*b*c", "a-x-b-y-c"));
        assert!(!glob_matches("a*b*c", "a-x-c"));
        assert!(glob_matches("*", "anything"));
        assert!(
            !glob_matches("ab*ba", "aba"),
            "overlapping ends don't match"
        );
    }

    #[test]
    fn options_validate_and_clean() {
        let mut dir = ParkedDir::new("/srv".into());
        let bad = ParkOptions {
            depth: Some(4),
            ..Default::default()
        };
        assert!(bad.apply(&mut dir).is_err());
        let escape = ParkOptions {
            exclude: Some(vec!["../etc".into()]),
            ..Default::default()
        };
        assert!(escape.apply(&mut dir).is_err());

        let good = ParkOptions {
            depth: Some(2),
            exclude: Some(vec![
                " old/ ".into(),
                "old".into(),
                "".into(),
                "a\\b".into(),
            ]),
            on_new: Some(OnNew::Suggest),
            ..Default::default()
        };
        good.apply(&mut dir).unwrap();
        assert_eq!(dir.depth, 2);
        assert_eq!(dir.exclude, ["old", "a/b"]);
        assert_eq!(dir.on_new, OnNew::Suggest);
        assert!(dir.enabled, "unset options are kept");
    }

    #[test]
    fn defaults_when_read() {
        let dir: ParkedDir = toml::from_str("path = \"/srv\"").unwrap();
        assert_eq!(dir, ParkedDir::new("/srv".into()));
    }
}
