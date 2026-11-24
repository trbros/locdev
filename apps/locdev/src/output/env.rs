//! Rendering for `locdev env …`. Secret values are never printed unless the
//! user asked to reveal one (`env show --reveal`).

use locdev_core::env::{
    ConflictSource, EntryStatus, EnvBackup, EnvHistoryEntry, EnvProfile, EnvVar, ProjectEnvReport,
    SyncPlan, SyncResult, VaultStatus,
};
use serde_json::json;

use super::{Output, Style, ago, truncate};

const MASK: &str = "********";

impl Output {
    fn shown(&self, var: &EnvVar) -> String {
        match (&var.value, var.secret) {
            (Some(value), _) => truncate(value, 60),
            (None, true) => self.paint(Style::Dim, MASK),
            (None, false) => String::new(),
        }
    }

    fn vault_line(&self, vault: &VaultStatus) {
        let line = match &vault.error {
            Some(error) => self.paint(Style::Red, &format!("vault: {error}")),
            None => self.paint(
                Style::Dim,
                &format!("vault: {} ({} secret(s))", vault.backend, vault.secrets),
            ),
        };
        println!("{line}");
    }

    pub fn env_list(&self, profiles: &[EnvProfile], vars: &[EnvVar], vault: &VaultStatus) {
        if self.json {
            self.print_json(json!({ "profiles": profiles, "vars": vars, "vault": vault }));
            return;
        }
        if vars.is_empty() {
            println!("No shared variables yet. Add one with `locdev env add KEY`.");
        }
        for profile in profiles {
            let in_profile: Vec<&EnvVar> =
                vars.iter().filter(|v| v.profile == profile.id).collect();
            if in_profile.is_empty() {
                continue;
            }
            println!(
                "{}",
                self.paint(Style::Bold, &format!("{} ({})", profile.label, profile.id))
            );
            let width = in_profile.iter().map(|v| v.key.len()).max().unwrap_or(0);
            for var in in_profile {
                let tag = if var.secret {
                    self.paint(Style::Yellow, "secret")
                } else {
                    self.paint(Style::Dim, "plain ")
                };
                println!("  {:<width$}  {tag}  {}", var.key, self.shown(var));
            }
        }
        self.vault_line(vault);
    }

    pub fn env_saved(&self, var: &EnvVar) {
        if self.json {
            self.print_json(var);
            return;
        }
        println!(
            "saved {} in profile `{}` ({})",
            var.key,
            var.profile,
            if var.secret {
                "secret, encrypted"
            } else {
                "plain"
            }
        );
        println!(
            "{}",
            self.paint(
                Style::Dim,
                "Running sites get it after a restart (`locdev reload <site>`)."
            )
        );
    }

    pub fn env_var(&self, var: &EnvVar) {
        if self.json {
            self.print_json(var);
            return;
        }
        println!("key           {}", var.key);
        println!("profile       {}", var.profile);
        println!("secret        {}", if var.secret { "yes" } else { "no" });
        println!(
            "value         {}",
            match &var.value {
                Some(v) => v.clone(),
                None => format!("{MASK}  (--reveal to show)"),
            }
        );
        if !var.description.is_empty() {
            println!("description   {}", var.description);
        }
        println!("updated       {} ago", ago(var.updated_at));
    }

    pub fn env_profiles(&self, profiles: &[EnvProfile], vars: &[EnvVar]) {
        if self.json {
            self.print_json(profiles);
            return;
        }
        for p in profiles {
            let count = vars.iter().filter(|v| v.profile == p.id).count();
            println!(
                "{:<16} {:<20} {count} variable(s)  {}",
                p.id,
                p.label,
                self.paint(Style::Dim, &p.description)
            );
        }
    }

    pub fn env_report(&self, report: &ProjectEnvReport, conflicts_only: bool) {
        if self.json {
            if conflicts_only {
                self.print_json(&report.conflicts);
            } else {
                self.print_json(report);
            }
            return;
        }
        if !conflicts_only {
            let mut chain = Vec::new();
            if report.settings.inherit_global {
                chain.push("global".to_string());
            }
            chain.extend(report.settings.profiles.iter().cloned());
            println!(
                "{}  {}",
                self.paint(Style::Bold, &report.project),
                self.paint(
                    Style::Dim,
                    &format!(
                        "{} · profiles: {}",
                        report.framework,
                        if chain.is_empty() {
                            "none".into()
                        } else {
                            chain.join(" → ")
                        }
                    )
                )
            );
            if report.restart_needed {
                println!(
                    "{}",
                    self.paint(
                        Style::Yellow,
                        &format!(
                            "Running with an older set; `locdev reload {}` applies the changes.",
                            report.project
                        )
                    )
                );
            }
            if report.entries.is_empty() {
                println!("No shared variables apply to this site.");
            }
            let width = report
                .entries
                .iter()
                .map(|e| e.key.len())
                .max()
                .unwrap_or(0);
            for entry in &report.entries {
                let (label, style) = match entry.status {
                    EntryStatus::Injected => ("injected  ", Style::Green),
                    EntryStatus::Skipped => ("skipped   ", Style::Yellow),
                    EntryStatus::Overridden => ("overridden", Style::Dim),
                    EntryStatus::Disabled => ("disabled  ", Style::Dim),
                    EntryStatus::Blocked => ("blocked   ", Style::Red),
                };
                let value = match (&entry.value, entry.secret) {
                    (_, true) => self.paint(Style::Dim, MASK),
                    (Some(v), false) => truncate(v, 40),
                    (None, false) => String::new(),
                };
                let reason = entry
                    .reason
                    .as_deref()
                    .map(|r| self.paint(Style::Dim, &format!("  {r}")))
                    .unwrap_or_default();
                println!(
                    "  {}  {:<width$}  {:<10} {value}{reason}",
                    self.paint(style, label),
                    entry.key,
                    entry.profile
                );
            }
            if !report.files.is_empty() {
                println!("{}", self.paint(Style::Bold, "Files"));
                for file in &report.files {
                    let state = match (file.active, file.rank) {
                        (true, Some(rank)) => self.paint(Style::Green, &format!("read #{rank}")),
                        _ => self.paint(Style::Dim, "not read"),
                    };
                    let extra = match (&file.error, file.duplicates.is_empty()) {
                        (Some(e), _) => self.paint(Style::Red, &format!("  {e}")),
                        (None, false) => self.paint(
                            Style::Yellow,
                            &format!("  duplicated: {}", file.duplicates.join(", ")),
                        ),
                        _ => String::new(),
                    };
                    println!(
                        "  {:<24} {state:<8}  {} key(s)  {}{extra}",
                        file.file,
                        file.keys,
                        self.paint(Style::Dim, &file.note)
                    );
                }
            }
            for note in &report.notes {
                println!("{}", self.paint(Style::Dim, &format!("· {note}")));
            }
        }
        if conflicts_only && report.conflicts.is_empty() {
            println!("No conflicts.");
            return;
        }
        if !report.conflicts.is_empty() {
            println!("{}", self.paint(Style::Bold, "Conflicts"));
        }
        for c in &report.conflicts {
            let source = match &c.source {
                ConflictSource::File { file, active: true } => format!("{file} (read)"),
                ConflictSource::File {
                    file,
                    active: false,
                } => format!("{file} (not read)"),
                ConflictSource::System => "system environment".into(),
                ConflictSource::Locdev => "set by LocDev".into(),
                ConflictSource::Profile { profile } => format!("profile `{profile}`"),
            };
            let mark = if c.injected {
                self.paint(Style::Dim, "info")
            } else {
                self.paint(Style::Yellow, "warn")
            };
            println!(
                "  {mark}  {} ({}) vs {source}: {}",
                c.key, c.profile, c.effect
            );
        }
    }

    pub fn env_plans(&self, plans: &[SyncPlan]) {
        if self.json {
            self.print_json(plans);
            return;
        }
        if plans.is_empty() {
            println!("No sites to sync.");
        }
        for plan in plans {
            let header = format!(
                "{} · {}{}",
                plan.project,
                plan.file,
                if plan.exists { "" } else { " (new file)" }
            );
            println!("{}", self.paint(Style::Bold, &header));
            if let Some(why) = &plan.refused {
                println!("  {}", self.paint(Style::Red, &format!("refused: {why}")));
                continue;
            }
            for line in plan.preview.lines() {
                println!("  {}", self.paint(Style::Green, line));
            }
            for skip in &plan.skip {
                println!(
                    "  {}",
                    self.paint(Style::Dim, &format!("skip {}: {}", skip.key, skip.reason))
                );
            }
            if plan.add.is_empty() {
                println!("  {}", self.paint(Style::Dim, "nothing to add"));
            }
        }
    }

    pub fn env_synced(&self, result: &SyncResult) {
        if self.json {
            self.print_json(result);
            return;
        }
        println!(
            "{} · {}: added {}{}",
            result.project,
            result.file,
            if result.added.is_empty() {
                "nothing".into()
            } else {
                result.added.join(", ")
            },
            result
                .backup
                .as_ref()
                .map(|b| format!(" (backup {b})"))
                .unwrap_or_default()
        );
    }

    pub fn env_backups(&self, backups: &[EnvBackup]) {
        if self.json {
            self.print_json(backups);
            return;
        }
        if backups.is_empty() {
            println!("No backups yet; LocDev takes one before it writes a .env file.");
        }
        for b in backups {
            println!(
                "{:<32} {:<20} {:>6} B  {} ago",
                b.id,
                b.file,
                b.size,
                ago(b.at)
            );
        }
    }

    pub fn env_history(&self, history: &[EnvHistoryEntry]) {
        if self.json {
            self.print_json(history);
            return;
        }
        if history.is_empty() {
            println!("No changes yet.");
        }
        for h in history {
            let what = [
                h.key.clone(),
                h.profile.as_ref().map(|p| format!("({p})")),
                h.project.as_ref().map(|p| format!("site {p}")),
                h.detail.clone(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
            println!("{:>6} ago  {:<16} {what}", ago(h.at), h.action);
        }
    }
}
