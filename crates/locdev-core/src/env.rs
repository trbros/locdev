//! Shared environment variables: what the daemon keeps, what a project gets
//! and what `env sync` would write. Secret values never travel in these
//! types unless a client asked to reveal one (`EnvVar::value`).

use serde::{Deserialize, Serialize};

/// The profile every project inherits unless it opts out.
pub const GLOBAL_PROFILE: &str = "global";

/// Prefixes that frameworks inline into browser bundles. A secret may never
/// use one: whatever has it ends up in code anyone can download.
pub const PUBLIC_PREFIXES: &[&str] = &[
    "NEXT_PUBLIC_",
    "VITE_",
    "NUXT_PUBLIC_",
    "PUBLIC_",
    "REACT_APP_",
    "GATSBY_",
    "EXPO_PUBLIC_",
    "VUE_APP_",
];

/// Variables that would break how LocDev runs processes, or let a shared
/// value load code into every one of them.
const RESERVED: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "PWD",
    "HOST",
    "PORT",
    "LOCDEV",
    "LOCDEV_PROJECT",
    "LOCDEV_HOME",
    "NODE_OPTIONS",
    "PHPRC",
    "PHP_INI_SCAN_DIR",
];
const RESERVED_PREFIXES: &[&str] = &["LD_", "DYLD_", "__VITE_", "LOCDEV_"];

/// Longest value accepted, in bytes.
pub const MAX_VALUE: usize = 32 * 1024;

/// The public prefix `key` starts with, if any.
pub fn public_prefix(key: &str) -> Option<&'static str> {
    PUBLIC_PREFIXES.iter().copied().find(|p| key.starts_with(p))
}

/// Why `key` can't be a shared variable, if it can't.
pub fn check_key(key: &str) -> Result<(), String> {
    let mut chars = key.chars();
    let valid_start = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !valid_start
        || key.len() > 128
        || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "`{key}` is not a valid variable name (letters, digits and _, not starting with a digit)"
        ));
    }
    if RESERVED.contains(&key) || RESERVED_PREFIXES.iter().any(|p| key.starts_with(p)) {
        return Err(format!(
            "{key} is set by LocDev or the system and can't be shared"
        ));
    }
    Ok(())
}

/// Why `value` can't be stored, if it can't.
pub fn check_value(value: &str) -> Result<(), String> {
    if value.len() > MAX_VALUE {
        return Err(format!("values are limited to {} KiB", MAX_VALUE / 1024));
    }
    if value.contains('\0') {
        return Err("values can't contain NUL characters".into());
    }
    Ok(())
}

/// Why `id` can't name a profile, if it can't.
pub fn check_profile_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{id}` is not a valid profile id (lowercase letters, digits and -)"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvProfile {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
}

/// A shared variable. `value` holds non-secret values; for secrets it is
/// `None` unless the client asked to reveal it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub profile: String,
    pub secret: bool,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub description: String,
    /// Unix milliseconds.
    pub updated_at: u64,
}

/// Which shared variables a project takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectEnvSettings {
    /// Take the global profile.
    #[serde(default = "yes")]
    pub inherit_global: bool,
    /// Extra profiles, in order: a later one wins over an earlier one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
    /// Keys this project never gets, whatever profile has them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled: Vec<String>,
}

fn yes() -> bool {
    true
}

impl Default for ProjectEnvSettings {
    fn default() -> Self {
        Self {
            inherit_global: true,
            profiles: Vec::new(),
            disabled: Vec::new(),
        }
    }
}

/// Where secret values are kept and whether they can be read right now.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VaultStatus {
    pub available: bool,
    /// "system keyring" or "key file".
    pub backend: String,
    #[serde(default)]
    pub error: Option<String>,
    pub secrets: usize,
}

/// One change to the shared variables. Never carries a value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvHistoryEntry {
    pub at: u64,
    /// "set", "removed", "revealed", "profile_added", "synced", "restored"…
    pub action: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// What happens to a shared variable for one project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    /// The process gets it.
    Injected,
    /// The project (or the system) already defines it; LocDev stays out.
    Skipped,
    /// A later profile on the project has the same key.
    Overridden,
    /// The project turned this key off.
    Disabled,
    /// Refused for safety (a secret with a browser prefix, a locked vault…).
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvEntry {
    pub key: String,
    pub profile: String,
    pub secret: bool,
    pub status: EntryStatus,
    #[serde(default)]
    pub reason: Option<String>,
    /// Non-secret values only.
    #[serde(default)]
    pub value: Option<String>,
}

/// What kind of file a `.env*` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvFileRole {
    /// `.env`
    Base,
    /// `.env.local`
    Local,
    /// `.env.<mode>`
    Mode,
    /// `.env.<mode>.local`
    ModeLocal,
    /// `.env.example`, `.env.sample`, `.env.dist`, `.env.template`
    Template,
}

/// A `.env*` file in the project and whether the dev server reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvFileInfo {
    pub file: String,
    pub role: EnvFileRole,
    /// The mode it belongs to (`production` for `.env.production`).
    #[serde(default)]
    pub mode: Option<String>,
    /// Read by the dev server LocDev runs.
    pub active: bool,
    /// 1 = wins over every other file; `None` when inactive.
    #[serde(default)]
    pub rank: Option<u8>,
    /// Why it is (or isn't) read.
    pub note: String,
    pub keys: usize,
    /// Keys defined more than once in this file.
    #[serde(default)]
    pub duplicates: Vec<String>,
    /// Not UTF-8 or otherwise unreadable; listed but not analyzed.
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConflictSource {
    /// Defined in one of the project's files.
    File { file: String, active: bool },
    /// Already in the environment LocDev runs processes with.
    System,
    /// Set by LocDev itself for this process (`APP_URL` for Laravel's Vite…).
    Locdev,
    /// Another LocDev profile has the same key.
    Profile { profile: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvConflict {
    pub key: String,
    /// The profile whose variable is affected.
    pub profile: String,
    pub source: ConflictSource,
    /// Whether the shared variable still reaches the process.
    pub injected: bool,
    /// What actually wins, in words.
    pub effect: String,
}

/// Everything about one project's shared variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectEnvReport {
    pub project: String,
    /// "Laravel", "Next.js"… the rules used to read the files.
    pub framework: String,
    pub settings: ProjectEnvSettings,
    pub entries: Vec<EnvEntry>,
    pub files: Vec<EnvFileInfo>,
    pub conflicts: Vec<EnvConflict>,
    /// Framework facts worth knowing (cached config, build-time inlining…).
    pub notes: Vec<String>,
    /// Running with a different set than the one it would get now.
    pub restart_needed: bool,
}

/// A line `env sync` would add.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncLine {
    pub key: String,
    pub profile: String,
    pub secret: bool,
    /// Written without its real value (`.env.example`).
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSkip {
    pub key: String,
    pub reason: String,
}

/// What `env sync` would do to one file. Nothing is written until the
/// client sends `token` back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPlan {
    pub project: String,
    pub file: String,
    pub exists: bool,
    pub add: Vec<SyncLine>,
    pub skip: Vec<SyncSkip>,
    /// The lines to be appended, secrets masked.
    pub preview: String,
    /// Ties the confirmation to this exact file content and these lines.
    pub token: String,
    /// Why nothing can be written at all (not UTF-8, a symlink…).
    #[serde(default)]
    pub refused: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncResult {
    pub project: String,
    pub file: String,
    pub added: Vec<String>,
    /// The encrypted copy taken before writing, if the file existed.
    #[serde(default)]
    pub backup: Option<String>,
}

/// An encrypted copy of a `.env*` file taken before LocDev changed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvBackup {
    pub id: String,
    pub project: String,
    pub file: String,
    pub at: u64,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert!(check_key("STRIPE_SECRET_KEY").is_ok());
        assert!(check_key("_x1").is_ok());
        assert!(check_key("1ABC").is_err());
        assert!(check_key("A-B").is_err());
        assert!(check_key("").is_err());
        assert!(check_key("PATH").is_err());
        assert!(check_key("LD_PRELOAD").is_err());
        assert!(check_key("DYLD_INSERT_LIBRARIES").is_err());
        assert!(check_key("LOCDEV_ANYTHING").is_err());
    }

    #[test]
    fn public_prefixes() {
        assert_eq!(public_prefix("NEXT_PUBLIC_API"), Some("NEXT_PUBLIC_"));
        assert_eq!(public_prefix("VITE_KEY"), Some("VITE_"));
        assert_eq!(public_prefix("STRIPE_SECRET_KEY"), None);
    }

    #[test]
    fn profile_ids() {
        assert!(check_profile_id("staging").is_ok());
        assert!(check_profile_id("next-js2").is_ok());
        assert!(check_profile_id("Staging").is_err());
        assert!(check_profile_id("-x").is_err());
        assert!(check_profile_id("").is_err());
    }

    #[test]
    fn settings_default_inherit() {
        let s: ProjectEnvSettings = serde_json::from_str("{}").unwrap();
        assert!(s.inherit_global);
    }
}
