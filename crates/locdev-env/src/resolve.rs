//! What a project's process gets: the profiles it takes, in order, minus
//! what it turned off, minus what it (or the system, or LocDev) already
//! defines. Every decision comes with its reason.

use std::collections::{BTreeMap, BTreeSet};

use locdev_core::deps::EnvConflictPolicy;
use locdev_core::env::{
    ConflictSource, EntryStatus, EnvConflict, EnvEntry, GLOBAL_PROFILE, ProjectEnvSettings,
    public_prefix,
};

use crate::rules::{Rules, Scan};
use crate::store::StoredVar;

pub struct Input<'a> {
    pub settings: &'a ProjectEnvSettings,
    pub vars: &'a [StoredVar],
    /// Profiles that exist.
    pub profiles: &'a BTreeSet<String>,
    /// Every value LocDev can read now, by `(profile, key)`; secrets are
    /// missing while the vault is locked.
    pub values: &'a BTreeMap<(String, String), String>,
    pub rules: &'a Rules,
    pub scan: &'a Scan,
    /// Keys in the environment LocDev starts processes with.
    pub system: &'a BTreeSet<String>,
    /// Keys LocDev itself sets for this project's processes.
    pub locdev: &'a BTreeSet<String>,
    pub policy: EnvConflictPolicy,
}

#[derive(Debug, Default)]
pub struct Resolution {
    pub entries: Vec<EnvEntry>,
    pub conflicts: Vec<EnvConflict>,
    /// What the process gets, sorted by key.
    pub env: Vec<(String, String)>,
    /// Lines for the site's log at start (keys only, never values).
    pub log: Vec<String>,
}

impl Resolution {
    /// The winning profile of each key that a sync may write (not turned
    /// off, not refused for safety), with whether it reached the process.
    pub fn effective(&self) -> impl Iterator<Item = &EnvEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.status, EntryStatus::Injected | EntryStatus::Skipped))
    }
}

/// The profiles a project takes, in precedence order (last wins).
pub fn chain(settings: &ProjectEnvSettings, profiles: &BTreeSet<String>) -> Vec<String> {
    let mut chain = Vec::new();
    if settings.inherit_global {
        chain.push(GLOBAL_PROFILE.to_string());
    }
    for p in &settings.profiles {
        if p != GLOBAL_PROFILE && profiles.contains(p) && !chain.contains(p) {
            chain.push(p.clone());
        }
    }
    chain
}

pub fn resolve(input: &Input) -> Resolution {
    let chain = chain(input.settings, input.profiles);
    let mut out = Resolution::default();
    for missing in input
        .settings
        .profiles
        .iter()
        .filter(|p| !input.profiles.contains(*p))
    {
        out.log.push(format!(
            "[LocDev] Warning: profile `{missing}` no longer exists; ignored"
        ));
    }

    // Each key's variables in chain order.
    let mut by_key: BTreeMap<&str, Vec<&StoredVar>> = BTreeMap::new();
    for profile in &chain {
        for var in input.vars.iter().filter(|v| &v.profile == profile) {
            by_key.entry(&var.key).or_default().push(var);
        }
    }

    for (key, vars) in by_key {
        let (winner, earlier) = vars.split_last().expect("non-empty");
        for var in earlier {
            out.entries.push(EnvEntry {
                key: key.to_string(),
                profile: var.profile.clone(),
                secret: var.secret,
                status: EntryStatus::Overridden,
                reason: Some(format!(
                    "profile `{}` sets it too and comes later",
                    winner.profile
                )),
                value: None,
            });
            out.conflicts.push(EnvConflict {
                key: key.to_string(),
                profile: var.profile.clone(),
                source: ConflictSource::Profile {
                    profile: winner.profile.clone(),
                },
                injected: false,
                effect: format!("`{}`'s value is used instead", winner.profile),
            });
        }

        let mut entry = EnvEntry {
            key: key.to_string(),
            profile: winner.profile.clone(),
            secret: winner.secret,
            status: EntryStatus::Injected,
            reason: None,
            value: (!winner.secret).then(|| winner.value.clone()).flatten(),
        };
        let mut conflict = |source: ConflictSource, injected: bool, effect: String| {
            out.conflicts.push(EnvConflict {
                key: key.to_string(),
                profile: winner.profile.clone(),
                source,
                injected,
                effect,
            });
        };

        if input.settings.disabled.iter().any(|d| d == key) {
            entry.status = EntryStatus::Disabled;
            entry.reason = Some("turned off for this project".into());
            out.entries.push(entry);
            continue;
        }
        if winner.secret
            && let Some(prefix) = public_prefix(key)
        {
            entry.status = EntryStatus::Blocked;
            entry.reason = Some(format!(
                "a secret with the {prefix} prefix would be bundled into browser code"
            ));
            out.log.push(format!(
                "[LocDev] Warning: {key} is secret but has the browser prefix {prefix}; not injected"
            ));
            out.entries.push(entry);
            continue;
        }

        // The project's own files.
        let defining = input.scan.defining(key);
        let active: Vec<&str> = defining
            .iter()
            .filter(|(_, a)| *a)
            .map(|(f, _)| *f)
            .collect();
        for (file, is_active) in &defining {
            let skip = *is_active || input.policy == EnvConflictPolicy::AnyFile;
            conflict(
                ConflictSource::File {
                    file: file.to_string(),
                    active: *is_active,
                },
                !skip,
                if *is_active {
                    format!(
                        "{file} defines it and {} reads that file; LocDev doesn't inject it, so the project's value is kept",
                        input.rules.loader
                    )
                } else if skip {
                    format!(
                        "{file} defines it (not read in development); skipped by the any-file policy"
                    )
                } else {
                    format!(
                        "{file} defines it but isn't read in development; the shared value is injected"
                    )
                },
            );
        }
        let skipped_by_file = !active.is_empty()
            || (input.policy == EnvConflictPolicy::AnyFile && !defining.is_empty());
        if skipped_by_file {
            let file = active.first().copied().unwrap_or_else(|| defining[0].0);
            entry.status = EntryStatus::Skipped;
            entry.reason = Some(format!("already in {file}"));
            out.log
                .push(format!("[LocDev] Warning: {key} already exists in {file}"));
            out.log.push(format!(
                "[LocDev] Skipping shared variable {key} (profile {}).",
                winner.profile
            ));
            out.entries.push(entry);
            continue;
        }

        if input.locdev.contains(key) {
            conflict(
                ConflictSource::Locdev,
                false,
                "LocDev sets it for this process; its own value is kept".into(),
            );
            entry.status = EntryStatus::Skipped;
            entry.reason = Some("set by LocDev for this process".into());
            out.entries.push(entry);
            continue;
        }
        if input.system.contains(key) {
            conflict(
                ConflictSource::System,
                false,
                "already in the environment LocDev starts processes with; that value is kept"
                    .into(),
            );
            entry.status = EntryStatus::Skipped;
            entry.reason = Some("already set in the system environment".into());
            out.log.push(format!(
                "[LocDev] Warning: {key} is already set in the system environment; skipping the shared value"
            ));
            out.entries.push(entry);
            continue;
        }

        match input.values.get(&(winner.profile.clone(), key.to_string())) {
            Some(value) => {
                out.env.push((key.to_string(), value.clone()));
                out.entries.push(entry);
            }
            None => {
                entry.status = EntryStatus::Blocked;
                entry.reason = Some("the secrets vault is locked".into());
                out.log.push(format!(
                    "[LocDev] Warning: {key} is secret and the vault is locked; not injected"
                ));
                out.entries.push(entry);
            }
        }
    }

    if !out.env.is_empty() {
        let keys: Vec<&str> = out.env.iter().map(|(k, _)| k.as_str()).collect();
        out.log.push(format!(
            "[LocDev] Injected {} shared variable(s): {}",
            keys.len(),
            keys.join(", ")
        ));
    }
    out
}

/// A fingerprint of what a process got, to tell when it needs a restart.
pub fn fingerprint(env: &[(String, String)]) -> String {
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    for (k, v) in env {
        ctx.update(k.as_bytes());
        ctx.update(b"\0");
        ctx.update(v.as_bytes());
        ctx.update(b"\0");
    }
    crate::hex(ctx.finish().as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::model::ProjectKind;

    fn var(key: &str, profile: &str, secret: bool, value: &str) -> StoredVar {
        StoredVar {
            key: key.into(),
            profile: profile.into(),
            secret,
            value: (!secret).then(|| value.to_string()),
            description: String::new(),
            updated_at: 0,
        }
    }

    struct Fixture {
        vars: Vec<StoredVar>,
        values: BTreeMap<(String, String), String>,
        profiles: BTreeSet<String>,
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new(vars: Vec<(StoredVar, &str)>) -> Self {
            let values = vars
                .iter()
                .map(|(v, value)| ((v.profile.clone(), v.key.clone()), value.to_string()))
                .collect();
            Self {
                profiles: ["global", "laravel", "staging"].map(String::from).into(),
                vars: vars.into_iter().map(|(v, _)| v).collect(),
                values,
                dir: tempfile::tempdir().unwrap(),
            }
        }

        fn run(
            &self,
            kind: ProjectKind,
            settings: &ProjectEnvSettings,
            system: &[&str],
        ) -> Resolution {
            let rules = crate::rules::rules(kind, self.dir.path(), None);
            let scan = crate::rules::scan(self.dir.path(), &rules);
            let system = system.iter().map(|s| s.to_string()).collect();
            resolve(&Input {
                settings,
                vars: &self.vars,
                profiles: &self.profiles,
                values: &self.values,
                rules: &rules,
                scan: &scan,
                system: &system,
                locdev: &["APP_URL".to_string()].into(),
                policy: EnvConflictPolicy::ActiveFiles,
            })
        }
    }

    fn status(res: &Resolution, key: &str) -> EntryStatus {
        res.entries
            .iter()
            .rfind(|e| e.key == key)
            .map(|e| e.status)
            .unwrap()
    }

    #[test]
    fn inherits_and_overrides_in_order() {
        let fx = Fixture::new(vec![
            (var("SENTRY_DSN", "global", false, "g"), "g"),
            (var("SENTRY_DSN", "laravel", false, "l"), "l"),
            (var("ONLY_GLOBAL", "global", false, "x"), "x"),
        ]);
        let settings = ProjectEnvSettings {
            profiles: vec!["laravel".into()],
            ..Default::default()
        };
        let res = fx.run(ProjectKind::Laravel, &settings, &[]);
        assert_eq!(
            res.env,
            vec![
                ("ONLY_GLOBAL".to_string(), "x".to_string()),
                ("SENTRY_DSN".to_string(), "l".to_string()),
            ]
        );
        assert!(res.entries.iter().any(|e| e.profile == "global"
            && e.key == "SENTRY_DSN"
            && e.status == EntryStatus::Overridden));
        assert!(res.conflicts.iter().any(
            |c| matches!(&c.source, ConflictSource::Profile { profile } if profile == "laravel")
        ));

        let no_global = ProjectEnvSettings {
            inherit_global: false,
            ..Default::default()
        };
        assert!(fx.run(ProjectKind::Laravel, &no_global, &[]).env.is_empty());
    }

    #[test]
    fn active_file_wins_inactive_is_reported() {
        let fx = Fixture::new(vec![
            (var("STRIPE_SECRET_KEY", "global", true, ""), "sk_test_1"),
            (var("SENTRY_DSN", "global", false, "d"), "d"),
        ]);
        std::fs::write(fx.dir.path().join(".env.local"), "STRIPE_SECRET_KEY=mine\n").unwrap();
        std::fs::write(fx.dir.path().join(".env.example"), "SENTRY_DSN=\n").unwrap();
        let res = fx.run(ProjectKind::Next, &ProjectEnvSettings::default(), &[]);

        assert_eq!(status(&res, "STRIPE_SECRET_KEY"), EntryStatus::Skipped);
        assert_eq!(status(&res, "SENTRY_DSN"), EntryStatus::Injected);
        assert_eq!(res.env, vec![("SENTRY_DSN".to_string(), "d".to_string())]);
        assert!(res.log.contains(
            &"[LocDev] Warning: STRIPE_SECRET_KEY already exists in .env.local".to_string()
        ));
        let example = res
            .conflicts
            .iter()
            .find(|c| c.key == "SENTRY_DSN")
            .unwrap();
        assert!(example.injected);
        assert!(matches!(
            &example.source,
            ConflictSource::File { active: false, .. }
        ));
    }

    #[test]
    fn any_file_policy_skips_templates_too() {
        let fx = Fixture::new(vec![(var("SENTRY_DSN", "global", false, "d"), "d")]);
        std::fs::write(fx.dir.path().join(".env.example"), "SENTRY_DSN=\n").unwrap();
        let rules = crate::rules::rules(ProjectKind::Next, fx.dir.path(), None);
        let scan = crate::rules::scan(fx.dir.path(), &rules);
        let res = resolve(&Input {
            settings: &ProjectEnvSettings::default(),
            vars: &fx.vars,
            profiles: &fx.profiles,
            values: &fx.values,
            rules: &rules,
            scan: &scan,
            system: &BTreeSet::new(),
            locdev: &BTreeSet::new(),
            policy: EnvConflictPolicy::AnyFile,
        });
        assert_eq!(status(&res, "SENTRY_DSN"), EntryStatus::Skipped);
        assert!(res.env.is_empty());
    }

    #[test]
    fn disabled_system_locdev_and_blocked() {
        let fx = Fixture::new(vec![
            (var("A", "global", false, "1"), "1"),
            (var("B", "global", false, "2"), "2"),
            (var("APP_URL", "global", false, "x"), "x"),
            (var("NEXT_PUBLIC_TOKEN", "global", true, ""), "secret"),
        ]);
        let settings = ProjectEnvSettings {
            disabled: vec!["A".into()],
            ..Default::default()
        };
        let res = fx.run(ProjectKind::Next, &settings, &["B"]);
        assert_eq!(status(&res, "A"), EntryStatus::Disabled);
        assert_eq!(status(&res, "B"), EntryStatus::Skipped);
        assert_eq!(status(&res, "APP_URL"), EntryStatus::Skipped);
        assert_eq!(status(&res, "NEXT_PUBLIC_TOKEN"), EntryStatus::Blocked);
        assert!(res.env.is_empty());
    }

    #[test]
    fn locked_vault_blocks_secrets_and_logs_no_values() {
        let mut fx = Fixture::new(vec![(var("API_KEY", "global", true, ""), "sk_live_zzz")]);
        fx.values.clear();
        let res = fx.run(ProjectKind::Laravel, &ProjectEnvSettings::default(), &[]);
        assert_eq!(status(&res, "API_KEY"), EntryStatus::Blocked);
        assert!(res.log.iter().all(|l| !l.contains("sk_live")));
        assert!(res.entries.iter().all(|e| e.value.is_none()));
    }

    #[test]
    fn fingerprint_changes_with_values() {
        let a = fingerprint(&[("A".into(), "1".into())]);
        assert_eq!(a, fingerprint(&[("A".into(), "1".into())]));
        assert_ne!(a, fingerprint(&[("A".into(), "2".into())]));
    }
}
