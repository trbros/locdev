//! `locdev env …`: shared environment variables and secrets.

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::env::{GLOBAL_PROFILE, ProjectEnvSettings, SyncPlan};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::vcs::read_secret;
use crate::work::confirm;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum EnvCommand {
    /// Shared variables (secret values hidden) and where the vault key lives.
    #[command(visible_alias = "ls")]
    List {
        /// Only this profile.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Create or change a shared variable. Without VALUE it is read from
    /// standard input, which keeps secrets out of your shell history.
    #[command(visible_alias = "set")]
    Add {
        key: String,
        value: Option<String>,
        #[arg(long, default_value = GLOBAL_PROFILE)]
        profile: String,
        /// Keep the value encrypted in the vault (default, except for browser prefixes).
        #[arg(long, conflicts_with = "plain")]
        secret: bool,
        /// Store the value as plain text in env.toml.
        #[arg(long)]
        plain: bool,
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove a shared variable.
    #[command(visible_alias = "rm")]
    Remove {
        key: String,
        #[arg(long, default_value = GLOBAL_PROFILE)]
        profile: String,
    },
    /// One variable; --reveal prints a secret's value.
    Show {
        key: String,
        #[arg(long, default_value = GLOBAL_PROFILE)]
        profile: String,
        /// Print the value even if it is secret (asks for the LocDev password when one is required).
        #[arg(long)]
        reveal: bool,
    },
    /// Profiles: groups of variables a project can take.
    #[command(subcommand)]
    Profile(ProfileCommand),
    /// Choose which shared variables a site takes.
    Use {
        site: Option<String>,
        /// Take this profile too (later ones win over earlier ones).
        #[arg(long = "add-profile", value_name = "PROFILE")]
        add_profile: Vec<String>,
        #[arg(long = "remove-profile", value_name = "PROFILE")]
        remove_profile: Vec<String>,
        /// Never give this key to the site.
        #[arg(long, value_name = "KEY")]
        disable: Vec<String>,
        /// Undo --disable.
        #[arg(long, value_name = "KEY")]
        enable: Vec<String>,
        /// Take the global profile (on by default).
        #[arg(long, value_name = "BOOL")]
        global: Option<bool>,
    },
    /// What the site's processes get, its .env files and every conflict.
    Check { site: Option<String> },
    /// Only the conflicts: where a shared variable meets one the site defines.
    Conflicts { site: Option<String> },
    /// Append the shared variables a site is missing to one of its .env files.
    ///
    /// Without a site, every registered site. Only adds lines, after showing
    /// them; keeps a backup; never copies secrets into .env.production or
    /// .env.staging, and writes .env.example with empty placeholders.
    Sync {
        site: Option<String>,
        /// Target file (default .env).
        #[arg(long)]
        file: Option<String>,
        /// Show what would change and stop.
        #[arg(long)]
        dry_run: bool,
        /// Do not ask before writing.
        #[arg(short, long)]
        yes: bool,
    },
    /// Encrypted copies of the site's .env files taken before LocDev wrote them.
    Backups { site: Option<String> },
    /// Put a backup back (the current file is backed up first).
    Restore {
        id: String,
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Recent changes to shared variables (never values).
    History,
}

#[derive(Subcommand)]
pub(crate) enum ProfileCommand {
    #[command(visible_alias = "ls")]
    List,
    /// Create a profile or rename it.
    Add {
        id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove an empty profile.
    #[command(visible_alias = "rm")]
    Remove { id: String },
}

pub async fn run(command: EnvCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let name = |site| async { anyhow::Ok(site::resolve(paths, site).await?.project.name) };
    match command {
        EnvCommand::List { profile } => {
            let Response::EnvOverview {
                profiles,
                vars,
                vault,
                ..
            } = daemon_call(paths, Request::EnvOverview).await?
            else {
                bail!("unexpected response from daemon");
            };
            let vars: Vec<_> = vars
                .into_iter()
                .filter(|v| profile.as_ref().is_none_or(|p| &v.profile == p))
                .collect();
            out.env_list(&profiles, &vars, &vault);
        }
        EnvCommand::Add {
            key,
            value,
            profile,
            secret,
            plain,
            description,
        } => {
            let value = match value {
                Some(value) => {
                    if !plain && std::io::stdin().is_terminal() {
                        out.note("note: a value given as an argument stays in your shell history; omit it to type or pipe it instead");
                    }
                    value
                }
                None => {
                    if std::io::stdin().is_terminal() {
                        eprint!("Value for {key}: ");
                    }
                    read_secret()?
                }
            };
            let secret = match (secret, plain) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            };
            let Response::EnvVar { var } = daemon_call(
                paths,
                Request::EnvSet {
                    key,
                    profile,
                    value: Some(value),
                    secret,
                    description,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_saved(&var);
        }
        EnvCommand::Remove { key, profile } => {
            daemon_call(
                paths,
                Request::EnvRemove {
                    key: key.clone(),
                    profile: profile.clone(),
                },
            )
            .await?;
            out.message(&format!("removed {key} from profile `{profile}`"));
        }
        EnvCommand::Show {
            key,
            profile,
            reveal,
        } => {
            let Response::EnvOverview { vars, .. } =
                daemon_call(paths, Request::EnvOverview).await?
            else {
                bail!("unexpected response from daemon");
            };
            let Some(mut var) = vars
                .into_iter()
                .find(|v| v.key == key && v.profile == profile)
            else {
                bail!("no variable {key} in profile `{profile}`");
            };
            if reveal && var.secret {
                let request = |password| Request::EnvReveal {
                    key: key.clone(),
                    profile: profile.clone(),
                    password,
                };
                let value = match daemon_call(paths, request(None)).await {
                    Ok(Response::EnvValue { value }) => value,
                    Err(e) if e.to_string().contains("password") => {
                        eprint!("LocDev password: ");
                        let password = read_secret()?;
                        match daemon_call(paths, request(Some(password))).await? {
                            Response::EnvValue { value } => value,
                            other => bail!("unexpected response from daemon: {other:?}"),
                        }
                    }
                    Err(e) => return Err(e),
                    Ok(other) => bail!("unexpected response from daemon: {other:?}"),
                };
                var.value = Some(value);
            }
            out.env_var(&var);
        }
        EnvCommand::Profile(command) => match command {
            ProfileCommand::List => {
                let Response::EnvOverview { profiles, vars, .. } =
                    daemon_call(paths, Request::EnvOverview).await?
                else {
                    bail!("unexpected response from daemon");
                };
                out.env_profiles(&profiles, &vars);
            }
            ProfileCommand::Add {
                id,
                label,
                description,
            } => {
                let Response::EnvProfile { profile } = daemon_call(
                    paths,
                    Request::EnvSaveProfile {
                        id,
                        label: label.unwrap_or_default(),
                        description: description.unwrap_or_default(),
                    },
                )
                .await?
                else {
                    bail!("unexpected response from daemon");
                };
                out.message(&format!("profile `{}` saved", profile.id));
            }
            ProfileCommand::Remove { id } => {
                daemon_call(paths, Request::EnvRemoveProfile { id: id.clone() }).await?;
                out.message(&format!("profile `{id}` removed"));
            }
        },
        EnvCommand::Use {
            site,
            add_profile,
            remove_profile,
            disable,
            enable,
            global,
        } => {
            let name = name(site).await?;
            let Response::EnvProject(report) =
                daemon_call(paths, Request::EnvProject { name: name.clone() }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let mut settings: ProjectEnvSettings = report.settings;
            settings.profiles.retain(|p| !remove_profile.contains(p));
            settings.profiles.extend(add_profile);
            settings.disabled.retain(|k| !enable.contains(k));
            settings.disabled.extend(disable);
            if let Some(global) = global {
                settings.inherit_global = global;
            }
            let Response::EnvProject(report) =
                daemon_call(paths, Request::EnvSetProject { name, settings }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_report(&report, false);
        }
        EnvCommand::Check { site } => {
            let name = name(site).await?;
            let Response::EnvProject(report) =
                daemon_call(paths, Request::EnvProject { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_report(&report, false);
        }
        EnvCommand::Conflicts { site } => {
            let name = name(site).await?;
            let Response::EnvProject(report) =
                daemon_call(paths, Request::EnvProject { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_report(&report, true);
        }
        EnvCommand::Sync {
            site,
            file,
            dry_run,
            yes,
        } => {
            // No site means every site (unlike other commands, which use the
            // current directory).
            let name = match site {
                Some(site) => Some(name(Some(site)).await?),
                None => None,
            };
            let Response::EnvSyncPlans { plans } = daemon_call(
                paths,
                Request::EnvSyncPlan {
                    name,
                    file: file.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_plans(&plans);
            let todo: Vec<&SyncPlan> = plans
                .iter()
                .filter(|p| p.refused.is_none() && !p.add.is_empty())
                .collect();
            if dry_run || todo.is_empty() {
                if todo.is_empty() {
                    out.note("nothing to add");
                }
                return Ok(ExitCode::SUCCESS);
            }
            if !yes {
                let lines: usize = todo.iter().map(|p| p.add.len()).sum();
                confirm(&format!(
                    "Append {lines} line(s) to {} file(s)? A backup is kept.",
                    todo.len()
                ))?;
            }
            let mut failed = false;
            for plan in todo {
                match daemon_call(
                    paths,
                    Request::EnvSync {
                        name: plan.project.clone(),
                        file: Some(plan.file.clone()),
                        token: plan.token.clone(),
                    },
                )
                .await
                {
                    Ok(Response::EnvSynced(result)) => out.env_synced(&result),
                    Ok(other) => bail!("unexpected response from daemon: {other:?}"),
                    Err(e) => {
                        failed = true;
                        out.error(&e.context(format!("{}: {}", plan.project, plan.file)));
                    }
                }
            }
            if failed {
                return Ok(ExitCode::FAILURE);
            }
        }
        EnvCommand::Backups { site } => {
            let name = name(site).await?;
            let Response::EnvBackups { backups } =
                daemon_call(paths, Request::EnvBackups { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_backups(&backups);
        }
        EnvCommand::Restore { id, site, yes } => {
            let name = name(site).await?;
            if !yes {
                confirm(&format!(
                    "Replace the file with backup {id}? The current file is backed up first."
                ))?;
            }
            daemon_call(
                paths,
                Request::EnvRestore {
                    name,
                    id: id.clone(),
                },
            )
            .await?;
            out.message(&format!("restored {id}"));
        }
        EnvCommand::History => {
            let Response::EnvOverview { history, .. } =
                daemon_call(paths, Request::EnvOverview).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.env_history(&history);
        }
    }
    Ok(ExitCode::SUCCESS)
}
