//! Upstream support windows: the Node.js release schedule
//! (github.com/nodejs/Release) and PHP's branch states (php.net). A snapshot
//! ships with LocDev so offline machines still get warnings; the daemon
//! refreshes it from the publishers now and then.

use std::collections::BTreeMap;
use std::path::Path;

use locdev_core::model::{RuntimeKind, Support, SupportState};
use serde::{Deserialize, Serialize};

use crate::{RuntimeError, download};

const NODE_SCHEDULE: &str = "https://raw.githubusercontent.com/nodejs/Release/main/schedule.json";
const PHP_STATES: &str = "https://www.php.net/releases/states.php";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifecycle {
    /// When this data was fetched (`YYYY-MM-DD`).
    pub fetched: String,
    /// By major: "22".
    node: BTreeMap<String, NodeLine>,
    /// By minor line: "8.3".
    php: BTreeMap<String, PhpLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NodeLine {
    start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lts: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maintenance: Option<String>,
    end: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    codename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PhpLine {
    start: String,
    active_end: String,
    security_end: String,
}

impl Lifecycle {
    pub fn bundled() -> Self {
        serde_json::from_str(include_str!("../lifecycle.json"))
            .expect("bundled lifecycle.json is valid")
    }

    /// A cached copy, if it exists and parses.
    pub fn load(path: &Path) -> Option<Self> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, raw)?;
        std::fs::rename(tmp, path)
    }

    /// Downloads both schedules. Blocking.
    pub fn fetch(today: &str) -> Result<Self, RuntimeError> {
        let node: BTreeMap<String, NodeLine> =
            serde_json::from_str(&download::fetch_text(NODE_SCHEDULE)?)
                .map_err(|e| RuntimeError::Catalog(format!("Node.js schedule: {e}")))?;
        let states: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
            serde_json::from_str(&download::fetch_text(PHP_STATES)?)
                .map_err(|e| RuntimeError::Catalog(format!("PHP states: {e}")))?;

        let date = |v: &serde_json::Value, key: &str| {
            v[key]
                .as_str()
                .and_then(|s| s.get(..10))
                .map(str::to_string)
        };
        let php = states
            .into_values()
            .flatten()
            .filter_map(|(line, v)| {
                Some((
                    line,
                    PhpLine {
                        start: date(&v, "initial_release")?,
                        active_end: date(&v, "active_support_end")?,
                        security_end: date(&v, "security_support_end")?,
                    },
                ))
            })
            .collect();
        let node = node
            .into_iter()
            .filter_map(|(k, line)| {
                let major = k.strip_prefix('v')?;
                // 0.x lines predate the schedule format that matters here.
                (!major.contains('.')).then(|| (major.to_string(), line))
            })
            .collect();

        Ok(Self {
            fetched: today.to_string(),
            node,
            php,
        })
    }

    /// Support of `version`'s line on `today` (`YYYY-MM-DD`), when the
    /// publisher has a schedule for it.
    pub fn support(&self, kind: RuntimeKind, version: &str, today: &str) -> Option<Support> {
        let parts = crate::version::parts(version);
        let phases: Vec<(SupportState, &str)> = match kind {
            RuntimeKind::Node => {
                let line = self.node.get(&parts.first()?.to_string())?;
                let mut phases = Vec::new();
                // Active until maintenance begins (or, without one, until the end).
                phases.push((
                    SupportState::Active,
                    line.maintenance.as_deref().unwrap_or(&line.end),
                ));
                if line.maintenance.is_some() {
                    phases.push((SupportState::Maintenance, line.end.as_str()));
                }
                phases
            }
            RuntimeKind::Php => {
                let line = self
                    .php
                    .get(&format!("{}.{}", parts.first()?, parts.get(1)?))?;
                vec![
                    (SupportState::Active, line.active_end.as_str()),
                    (SupportState::Security, line.security_end.as_str()),
                ]
            }
            _ => return None,
        };

        let current = phases.iter().find(|(_, until)| today < *until);
        Some(match current {
            Some((state, until)) => Support {
                state: *state,
                until: Some(until.to_string()),
            },
            None => Support {
                state: SupportState::Eol,
                until: phases.last().map(|(_, until)| until.to_string()),
            },
        })
    }

    /// Node.js majors that are (or were) LTS lines.
    pub fn lts_majors(&self) -> Vec<u64> {
        self.node
            .iter()
            .filter(|(_, line)| line.lts.is_some())
            .filter_map(|(major, _)| major.parse().ok())
            .collect()
    }

    /// The Node.js major of an LTS codename (`iron` → 20).
    pub fn codename_major(&self, codename: &str) -> Option<u64> {
        self.node
            .iter()
            .find(|(_, line)| {
                line.codename
                    .as_deref()
                    .is_some_and(|c| c.eq_ignore_ascii_case(codename))
            })
            .and_then(|(major, _)| major.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_node_lines() {
        let data = Lifecycle::bundled();
        let at = |v, day| data.support(RuntimeKind::Node, v, day).unwrap().state;

        assert_eq!(at("18.20.0", "2026-11-02"), SupportState::Eol);
        assert_eq!(at("22.11.0", "2025-01-10"), SupportState::Active);
        assert_eq!(at("20.1.0", "2025-01-10"), SupportState::Maintenance);
        assert!(
            data.support(RuntimeKind::Node, "5.0.0", "2026-01-01")
                .is_none()
        );
        assert!(data.lts_majors().contains(&20));
        assert_eq!(data.codename_major("iron"), Some(20));
    }

    #[test]
    fn classifies_php_lines() {
        let data = Lifecycle::bundled();
        let support = data
            .support(RuntimeKind::Php, "8.3.14", "2026-11-02")
            .unwrap();
        assert_eq!(support.state, SupportState::Security);
        assert_eq!(support.until.as_deref(), Some("2027-12-31"));
        assert_eq!(
            data.support(RuntimeKind::Php, "8.1.30", "2026-11-02")
                .unwrap()
                .state,
            SupportState::Eol
        );
        assert!(
            data.support(RuntimeKind::Pnpm, "9.0.0", "2026-11-02")
                .is_none()
        );
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lifecycle.json");
        let data = Lifecycle::bundled();
        data.save(&path).unwrap();
        assert_eq!(Lifecycle::load(&path), Some(data));
        std::fs::write(&path, "{").unwrap();
        assert_eq!(Lifecycle::load(&path), None);
    }
}
