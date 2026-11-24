//! The shared variables on disk: `env.toml` (profiles, variables, project
//! choices and non-secret values), the vault (secret values) and the change
//! history (`env-history.json`, never a value).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use locdev_core::env::{
    EnvHistoryEntry, EnvProfile, EnvVar, GLOBAL_PROFILE, ProjectEnvSettings, VaultStatus,
    check_key, check_profile_id, check_value, public_prefix,
};
use serde::{Deserialize, Serialize};

use crate::vault::{KeySource, Vault};
use crate::write_private;

const FORMAT: u32 = 1;
const HISTORY_MAX: usize = 500;

/// A variable as kept in `env.toml`; a secret's value is in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredVar {
    pub key: String,
    pub profile: String,
    #[serde(default)]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default)]
    pub updated_at: u64,
}

impl StoredVar {
    pub fn id(&self) -> String {
        vault_id(&self.profile, &self.key)
    }
}

fn vault_id(profile: &str, key: &str) -> String {
    format!("{profile}/{key}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EnvFile {
    #[serde(default)]
    format: u32,
    #[serde(default)]
    profiles: Vec<EnvProfile>,
    #[serde(default, rename = "var")]
    vars: Vec<StoredVar>,
    #[serde(default)]
    projects: BTreeMap<String, ProjectEnvSettings>,
}

impl Default for EnvFile {
    fn default() -> Self {
        Self {
            format: FORMAT,
            profiles: vec![global()],
            vars: Vec::new(),
            projects: BTreeMap::new(),
        }
    }
}

fn global() -> EnvProfile {
    EnvProfile {
        id: GLOBAL_PROFILE.into(),
        label: "Global".into(),
        description: "Every project gets these unless it opts out.".into(),
    }
}

/// What a client asked to change about one variable. `None` keeps the
/// current value of that field.
#[derive(Debug, Clone, Default)]
pub struct VarChange {
    pub value: Option<String>,
    pub secret: Option<bool>,
    pub description: Option<String>,
}

pub struct EnvStore {
    path: PathBuf,
    history_path: PathBuf,
    data: EnvFile,
    history: Vec<EnvHistoryEntry>,
    pub vault: Vault,
}

pub type Result<T> = std::result::Result<T, String>;

impl EnvStore {
    pub fn load(config_dir: &Path, state_dir: &Path, key: Box<dyn KeySource>) -> Result<Self> {
        let path = config_dir.join("env.toml");
        let mut data: EnvFile = match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => EnvFile::default(),
            Err(e) => return Err(e.to_string()),
        };
        if data.format > FORMAT {
            return Err(format!("{} was written by a newer LocDev", path.display()));
        }
        if !data.profiles.iter().any(|p| p.id == GLOBAL_PROFILE) {
            data.profiles.insert(0, global());
        }
        let history_path = state_dir.join("env-history.json");
        let history = std::fs::read(&history_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let vault = Vault::load(&state_dir.join("env-vault.json"), key).map_err(|e| e.0)?;
        Ok(Self {
            path,
            history_path,
            data,
            history,
            vault,
        })
    }

    fn save(&self) -> Result<()> {
        let mut data = self.data.clone();
        data.format = FORMAT;
        let text = toml::to_string_pretty(&data).map_err(|e| e.to_string())?;
        let header = "# Shared environment variables (Managed by LocDev).\n# Secret values are not here: they are encrypted in the LocDev vault.\n\n";
        write_private(&self.path, format!("{header}{text}").as_bytes()).map_err(|e| e.to_string())
    }

    pub fn record(
        &mut self,
        action: &str,
        key: Option<&str>,
        profile: Option<&str>,
        project: Option<&str>,
        detail: Option<String>,
    ) {
        self.history.push(EnvHistoryEntry {
            at: locdev_core::model::now_ms(),
            action: action.into(),
            key: key.map(Into::into),
            profile: profile.map(Into::into),
            project: project.map(Into::into),
            detail,
        });
        if self.history.len() > HISTORY_MAX {
            let extra = self.history.len() - HISTORY_MAX;
            self.history.drain(..extra);
        }
        if let Ok(json) = serde_json::to_vec(&self.history) {
            let _ = write_private(&self.history_path, &json);
        }
    }

    pub fn history(&self, limit: usize) -> Vec<EnvHistoryEntry> {
        self.history.iter().rev().take(limit).cloned().collect()
    }

    pub fn vault_status(&self) -> VaultStatus {
        let secrets = self.data.vars.iter().filter(|v| v.secret).count();
        // Only try the keyring when there is something to read; listing
        // variables must not prompt for an unlock.
        let error = if secrets > 0 {
            self.vault.unlock().err().map(|e| e.0)
        } else {
            None
        };
        VaultStatus {
            available: error.is_none(),
            backend: self.vault.backend().into(),
            error,
            secrets,
        }
    }

    pub fn profiles(&self) -> &[EnvProfile] {
        &self.data.profiles
    }

    pub fn profile_ids(&self) -> BTreeSet<String> {
        self.data.profiles.iter().map(|p| p.id.clone()).collect()
    }

    pub fn stored(&self) -> &[StoredVar] {
        &self.data.vars
    }

    /// Every variable, secret values left out.
    pub fn vars(&self) -> Vec<EnvVar> {
        self.data.vars.iter().map(|v| self.view(v)).collect()
    }

    fn view(&self, v: &StoredVar) -> EnvVar {
        EnvVar {
            key: v.key.clone(),
            profile: v.profile.clone(),
            secret: v.secret,
            value: if v.secret { None } else { v.value.clone() },
            description: v.description.clone(),
            updated_at: v.updated_at,
        }
    }

    /// Every value that can be read now; secrets only while the vault opens.
    pub fn values(&self) -> BTreeMap<(String, String), String> {
        let mut out = BTreeMap::new();
        for v in &self.data.vars {
            let value = if v.secret {
                self.vault.get(&v.id()).ok().flatten()
            } else {
                v.value.clone()
            };
            if let Some(value) = value {
                out.insert((v.profile.clone(), v.key.clone()), value);
            }
        }
        out
    }

    pub fn reveal(&mut self, key: &str, profile: &str) -> Result<String> {
        let var = self.find(key, profile)?.clone();
        let value = if var.secret {
            self.vault
                .get(&var.id())
                .map_err(|e| e.0)?
                .ok_or_else(|| format!("{key}'s value is missing from the vault"))?
        } else {
            var.value.clone().unwrap_or_default()
        };
        if var.secret {
            self.record("revealed", Some(key), Some(profile), None, None);
        }
        Ok(value)
    }

    fn find(&self, key: &str, profile: &str) -> Result<&StoredVar> {
        self.data
            .vars
            .iter()
            .find(|v| v.key == key && v.profile == profile)
            .ok_or_else(|| format!("no variable {key} in profile `{profile}`"))
    }

    /// Creates or changes a variable. A new one needs a value; a new one
    /// with a browser prefix defaults to not secret, anything else to secret.
    pub fn set(&mut self, key: &str, profile: &str, change: VarChange) -> Result<EnvVar> {
        check_key(key)?;
        if !self.data.profiles.iter().any(|p| p.id == profile) {
            return Err(format!("no profile `{profile}`"));
        }
        if let Some(value) = &change.value {
            check_value(value)?;
        }
        let existing = self
            .data
            .vars
            .iter()
            .position(|v| v.key == key && v.profile == profile);
        let was_secret = existing.map(|i| self.data.vars[i].secret);
        let secret = change
            .secret
            .or(was_secret)
            .unwrap_or_else(|| public_prefix(key).is_none());
        if secret && let Some(prefix) = public_prefix(key) {
            return Err(format!(
                "{key} can't be secret: frameworks bundle {prefix} variables into browser code. Rename it or store it as a plain value"
            ));
        }

        // The value to keep, moved between the vault and the file if the
        // secret flag changed.
        let value = match (&change.value, existing) {
            (Some(v), _) => v.clone(),
            (None, Some(i)) => {
                let old = &self.data.vars[i];
                if old.secret {
                    self.vault
                        .get(&old.id())
                        .map_err(|e| e.0)?
                        .unwrap_or_default()
                } else {
                    old.value.clone().unwrap_or_default()
                }
            }
            (None, None) => return Err(format!("{key} needs a value")),
        };

        let id = vault_id(profile, key);
        if secret {
            self.vault.set(&id, &value).map_err(|e| e.0)?;
        } else if was_secret == Some(true) {
            self.vault.remove(&id).map_err(|e| e.0)?;
        }
        let var = StoredVar {
            key: key.into(),
            profile: profile.into(),
            secret,
            value: (!secret).then_some(value),
            description: change.description.unwrap_or_else(|| {
                existing
                    .map(|i| self.data.vars[i].description.clone())
                    .unwrap_or_default()
            }),
            updated_at: locdev_core::model::now_ms(),
        };
        match existing {
            Some(i) => self.data.vars[i] = var.clone(),
            None => self.data.vars.push(var.clone()),
        }
        self.save()?;
        let detail = match (was_secret, secret) {
            (None, _) => Some("created".into()),
            (Some(false), true) => Some("now secret".into()),
            (Some(true), false) => Some("no longer secret".into()),
            _ => None,
        };
        self.record("set", Some(key), Some(profile), None, detail);
        Ok(self.view(&var))
    }

    pub fn remove(&mut self, key: &str, profile: &str) -> Result<()> {
        let var = self.find(key, profile)?.clone();
        if var.secret {
            self.vault.remove(&var.id()).map_err(|e| e.0)?;
        }
        self.data
            .vars
            .retain(|v| !(v.key == key && v.profile == profile));
        self.save()?;
        self.record("removed", Some(key), Some(profile), None, None);
        Ok(())
    }

    pub fn add_profile(&mut self, id: &str, label: &str, description: &str) -> Result<EnvProfile> {
        check_profile_id(id)?;
        let label = label.trim();
        let profile = EnvProfile {
            id: id.into(),
            label: if label.is_empty() {
                id.into()
            } else {
                label.into()
            },
            description: description.trim().into(),
        };
        match self.data.profiles.iter_mut().find(|p| p.id == id) {
            Some(existing) => *existing = profile.clone(),
            None => self.data.profiles.push(profile.clone()),
        }
        self.save()?;
        self.record("profile_saved", None, Some(id), None, None);
        Ok(profile)
    }

    /// Removes an empty profile and drops it from every project.
    pub fn remove_profile(&mut self, id: &str) -> Result<()> {
        if id == GLOBAL_PROFILE {
            return Err("the global profile can't be removed".into());
        }
        if !self.data.profiles.iter().any(|p| p.id == id) {
            return Err(format!("no profile `{id}`"));
        }
        let count = self.data.vars.iter().filter(|v| v.profile == id).count();
        if count > 0 {
            return Err(format!(
                "profile `{id}` still has {count} variable(s); remove them first"
            ));
        }
        self.data.profiles.retain(|p| p.id != id);
        for settings in self.data.projects.values_mut() {
            settings.profiles.retain(|p| p != id);
        }
        self.save()?;
        self.record("profile_removed", None, Some(id), None, None);
        Ok(())
    }

    pub fn project(&self, name: &str) -> ProjectEnvSettings {
        self.data.projects.get(name).cloned().unwrap_or_default()
    }

    pub fn set_project(&mut self, name: &str, mut settings: ProjectEnvSettings) -> Result<()> {
        let known = self.profile_ids();
        let mut seen = BTreeSet::new();
        settings
            .profiles
            .retain(|p| p != GLOBAL_PROFILE && seen.insert(p.clone()));
        if let Some(missing) = settings.profiles.iter().find(|p| !known.contains(*p)) {
            return Err(format!("no profile `{missing}`"));
        }
        for key in &settings.disabled {
            check_key(key)?;
        }
        settings.disabled.sort();
        settings.disabled.dedup();
        if settings == ProjectEnvSettings::default() {
            self.data.projects.remove(name);
        } else {
            self.data.projects.insert(name.into(), settings);
        }
        self.save()?;
        self.record("project_changed", None, None, Some(name), None);
        Ok(())
    }

    /// Forgets a removed project's choices.
    pub fn forget_project(&mut self, name: &str) -> Result<()> {
        if self.data.projects.remove(name).is_some() {
            self.save()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::MemoryKey;

    fn store(dir: &Path) -> EnvStore {
        EnvStore::load(dir, dir, Box::new(MemoryKey::default())).unwrap()
    }

    #[test]
    fn secrets_stay_out_of_env_toml() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let var = s
            .set(
                "STRIPE_SECRET_KEY",
                "global",
                VarChange {
                    value: Some("sk_test_abc".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(var.secret, "secret by default");
        assert_eq!(var.value, None);
        let toml = std::fs::read_to_string(dir.path().join("env.toml")).unwrap();
        assert!(toml.contains("STRIPE_SECRET_KEY"));
        assert!(!toml.contains("sk_test_abc"));
        let history = std::fs::read_to_string(dir.path().join("env-history.json")).unwrap();
        assert!(!history.contains("sk_test_abc"));
        assert_eq!(
            s.reveal("STRIPE_SECRET_KEY", "global").unwrap(),
            "sk_test_abc"
        );
        assert_eq!(s.history(1)[0].action, "revealed");
    }

    #[test]
    fn public_prefix_defaults_plain_and_refuses_secret() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let var = s
            .set(
                "NEXT_PUBLIC_SITE",
                "global",
                VarChange {
                    value: Some("https://x".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!var.secret);
        assert!(
            s.set(
                "NEXT_PUBLIC_SITE",
                "global",
                VarChange {
                    secret: Some(true),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn toggling_secret_moves_the_value() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.set(
            "DSN",
            "global",
            VarChange {
                value: Some("v1".into()),
                secret: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        s.set(
            "DSN",
            "global",
            VarChange {
                secret: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !std::fs::read_to_string(dir.path().join("env.toml"))
                .unwrap()
                .contains("v1")
        );
        assert_eq!(s.values()[&("global".into(), "DSN".into())], "v1");
        s.set(
            "DSN",
            "global",
            VarChange {
                secret: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(s.vault.is_empty());
        assert_eq!(s.vars()[0].value.as_deref(), Some("v1"));
    }

    #[test]
    fn profiles_and_projects() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        assert!(s.remove_profile("global").is_err());
        s.add_profile("laravel", "Laravel", "").unwrap();
        s.set(
            "QUEUE_CONNECTION",
            "laravel",
            VarChange {
                value: Some("redis".into()),
                secret: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(s.remove_profile("laravel").is_err(), "not empty");
        s.set_project(
            "shop",
            ProjectEnvSettings {
                profiles: vec!["laravel".into(), "laravel".into(), "global".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(s.project("shop").profiles, vec!["laravel".to_string()]);
        assert!(
            s.set_project(
                "shop",
                ProjectEnvSettings {
                    profiles: vec!["nope".into()],
                    ..Default::default()
                }
            )
            .is_err()
        );
        s.remove("QUEUE_CONNECTION", "laravel").unwrap();
        s.remove_profile("laravel").unwrap();
        assert!(s.project("shop").profiles.is_empty());

        // Survives a reload.
        s.add_profile("staging", "Staging", "").unwrap();
        let again = store(dir.path());
        assert!(again.profile_ids().contains("staging"));
    }

    #[test]
    fn rejects_bad_input() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        let v = |value: &str| VarChange {
            value: Some(value.into()),
            ..Default::default()
        };
        assert!(s.set("PATH", "global", v("x")).is_err());
        assert!(s.set("A", "nope", v("x")).is_err());
        assert!(s.set("A", "global", v("a\0b")).is_err());
        assert!(s.set("A", "global", VarChange::default()).is_err());
    }
}
