//! Shared environment variables: the requests behind `locdev env …` and
//! the app's Environment pages, and what each project process gets at
//! start. Values reach only the process environment; project files change
//! only through `env sync`, after the user saw the plan.

use std::collections::{BTreeMap, BTreeSet};

use locdev_core::env::{ProjectEnvReport, SyncPlan, SyncResult};
use locdev_core::model::{Check, CheckLevel, DevTool, ProcessRole, Project, ProjectKind};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_env::resolve::{self, Input, Resolution};
use locdev_env::rules::{self, Rules, Scan};
use locdev_env::sync::{self, DEFAULT_FILE};
use locdev_env::{EnvStore, VarChange};
use tracing::info;

use super::Daemon;
use crate::config::KeyStore;

const HISTORY_SHOWN: usize = 100;

fn invalid(message: String) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}

/// Keys LocDev sets itself for a project's process (`servers.rs`).
fn locdev_keys(project: &Project, role: ProcessRole, kind: ProjectKind) -> BTreeSet<String> {
    let mut keys = BTreeSet::from(["__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS".to_string()]);
    let vite_helper = project
        .dev_server
        .as_ref()
        .is_some_and(|d| d.tool == DevTool::Vite);
    if role == ProcessRole::Dev && kind == ProjectKind::Laravel && vite_helper {
        keys.extend(["VITE_DEV_SERVER_CERT", "VITE_DEV_SERVER_KEY", "APP_URL"].map(String::from));
    }
    keys
}

/// Keys in the environment the daemon starts processes with.
fn system_keys() -> BTreeSet<String> {
    std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
        .collect()
}

pub(super) struct Resolved {
    pub rules: Rules,
    pub scan: Scan,
    pub resolution: Resolution,
    pub values: BTreeMap<(String, String), String>,
}

impl Daemon {
    fn env_store(&self) -> std::sync::MutexGuard<'_, EnvStore> {
        self.env.lock().unwrap()
    }

    pub(super) fn env_resolve(&self, project: &Project, role: ProcessRole) -> Resolved {
        let stack = self.stack_of(project);
        let kind = stack.kind;
        let command = match role {
            ProcessRole::Main => project.command.clone(),
            ProcessRole::Dev => project.dev_server.as_ref().map(|d| d.command.clone()),
        };
        // The adapter's own .env rules when it states them (Rails, Django…);
        // otherwise the built-in ones by kind.
        let rules = match locdev_frameworks::env_spec(&self.catalog(), &stack, &project.path) {
            Some((name, spec)) => {
                let mut notes = spec.notes;
                if spec.loader == "none" && notes.is_empty() {
                    notes.push(format!("{name} doesn't read .env files here."));
                }
                Rules::from_spec(&name, spec.files, &spec.loader, spec.public, notes)
            }
            None => rules::rules(kind, &project.path, command.as_deref()),
        };
        let scan = rules::scan(&project.path, &rules);
        let policy = self.settings.get().env.conflicts;
        let store = self.env_store();
        let settings = store.project(&project.name);
        let values = store.values();
        let resolution = resolve::resolve(&Input {
            settings: &settings,
            vars: store.stored(),
            profiles: &store.profile_ids(),
            values: &values,
            rules: &rules,
            scan: &scan,
            system: &system_keys(),
            locdev: &locdev_keys(project, role, kind),
            policy,
        });
        Resolved {
            rules,
            scan,
            resolution,
            values,
        }
    }

    /// What a process gets at start, and the lines for its log. Remembers
    /// what the main process got, to tell when it needs a restart.
    pub(super) fn shared_env(
        &self,
        project: &Project,
        role: ProcessRole,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let resolution = self.env_resolve(project, role).resolution;
        if role == ProcessRole::Main {
            self.env_applied
                .lock()
                .unwrap()
                .insert(project.name.clone(), resolve::fingerprint(&resolution.env));
        }
        (resolution.env, resolution.log)
    }

    fn env_report(&self, project: &Project) -> ProjectEnvReport {
        let Resolved {
            rules,
            scan,
            resolution,
            ..
        } = self.env_resolve(project, ProcessRole::Main);
        let running = self
            .supervisor
            .is_active(&ProcessRole::Main.id(&project.name));
        let applied = self.env_applied.lock().unwrap().get(&project.name).cloned();
        let restart_needed =
            running && applied.is_some_and(|a| a != resolve::fingerprint(&resolution.env));
        ProjectEnvReport {
            project: project.name.clone(),
            framework: rules.framework,
            settings: self.env_store().project(&project.name),
            entries: resolution.entries,
            files: scan.files,
            conflicts: resolution.conflicts,
            notes: rules.notes,
            restart_needed,
        }
    }

    fn env_plan(&self, project: &Project, file: &str) -> (SyncPlan, Option<Vec<u8>>) {
        let Resolved {
            scan,
            resolution,
            values,
            ..
        } = self.env_resolve(project, ProcessRole::Main);
        sync::plan(
            &project.name,
            &project.path,
            file,
            &resolution,
            &values,
            &scan,
        )
    }

    fn env_overview(&self) -> Response {
        let store = self.env_store();
        Response::EnvOverview {
            profiles: store.profiles().to_vec(),
            vars: store.vars(),
            vault: store.vault_status(),
            history: store.history(HISTORY_SHOWN),
        }
    }

    pub(super) async fn env_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::EnvOverview => Ok(self.env_overview()),

            Request::EnvSet {
                key,
                profile,
                value,
                secret,
                description,
            } => {
                let var = self
                    .env_store()
                    .set(
                        key.trim(),
                        profile.trim(),
                        VarChange {
                            value,
                            secret,
                            description,
                        },
                    )
                    .map_err(invalid)?;
                info!(key = %var.key, profile = %var.profile, secret = var.secret, "shared variable saved");
                Ok(Response::EnvVar { var })
            }

            Request::EnvRemove { key, profile } => {
                self.env_store().remove(&key, &profile).map_err(invalid)?;
                info!(%key, %profile, "shared variable removed");
                Ok(Response::Ok)
            }

            Request::EnvReveal {
                key,
                profile,
                password,
            } => {
                if self.security.password_required() {
                    self.security.verify(password.as_deref())?;
                }
                let value = self
                    .env_store()
                    .reveal(&key, &profile)
                    .map_err(|e| AppError::new(ErrorCode::Config, e))?;
                Ok(Response::EnvValue { value })
            }

            Request::EnvSaveProfile {
                id,
                label,
                description,
            } => {
                let profile = self
                    .env_store()
                    .add_profile(id.trim(), &label, &description)
                    .map_err(invalid)?;
                Ok(Response::EnvProfile { profile })
            }

            Request::EnvRemoveProfile { id } => {
                self.env_store().remove_profile(&id).map_err(invalid)?;
                Ok(Response::Ok)
            }

            Request::EnvProject { name } => {
                let project = self.project(&name).await?;
                Ok(Response::EnvProject(self.env_report(&project)))
            }

            Request::EnvSetProject { name, settings } => {
                let project = self.project(&name).await?;
                self.env_store()
                    .set_project(&project.name, settings)
                    .map_err(invalid)?;
                Ok(Response::EnvProject(self.env_report(&project)))
            }

            Request::EnvSyncPlan { name, file } => {
                let file = file.unwrap_or_else(|| DEFAULT_FILE.into());
                sync::check_target(&file).map_err(invalid)?;
                let projects = match name {
                    Some(name) => vec![self.project(&name).await?],
                    None => self.registry.lock().await.all().to_vec(),
                };
                let plans = projects
                    .iter()
                    .filter(|p| p.path.is_dir())
                    .map(|p| self.env_plan(p, &file).0)
                    .collect();
                Ok(Response::EnvSyncPlans { plans })
            }

            Request::EnvSync { name, file, token } => {
                let project = self.project(&name).await?;
                let file = file.unwrap_or_else(|| DEFAULT_FILE.into());
                // The same plan again: it must still be the one the user saw.
                let (plan, bytes) = self.env_plan(&project, &file);
                if let Some(why) = plan.refused {
                    return Err(invalid(why));
                }
                if plan.token != token {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!(
                            "{file} or the shared variables changed since the preview; review it again"
                        ),
                    ));
                }
                let Some(bytes) = bytes else {
                    return Ok(Response::EnvSynced(SyncResult {
                        project: project.name,
                        file,
                        added: Vec::new(),
                        backup: None,
                    }));
                };
                let path = project.path.join(&file);
                let backup = match std::fs::read(&path) {
                    Ok(current) => {
                        let store = self.env_store();
                        Some(
                            self.env_backups
                                .take(&store.vault, &project.name, &file, &current)
                                .map_err(|e| {
                                    AppError::new(
                                        ErrorCode::Config,
                                        format!(
                                            "no backup could be made, so {file} was left alone: {e}"
                                        ),
                                    )
                                })?
                                .id,
                        )
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.into()),
                };
                sync::write_env_file(&path, &bytes)?;
                let added: Vec<String> = plan.add.iter().map(|l| l.key.clone()).collect();
                self.env_store().record(
                    "synced",
                    None,
                    None,
                    Some(&project.name),
                    Some(format!("{file}: {}", added.join(", "))),
                );
                info!(project = %project.name, %file, added = added.len(), "shared variables synced");
                Ok(Response::EnvSynced(SyncResult {
                    project: project.name,
                    file,
                    added,
                    backup,
                }))
            }

            Request::EnvBackups { name } => {
                let project = self.project(&name).await?;
                Ok(Response::EnvBackups {
                    backups: self.env_backups.list(&project.name),
                })
            }

            Request::EnvRestore { name, id } => {
                let project = self.project(&name).await?;
                let store = self.env_store();
                let (backup, bytes) = self
                    .env_backups
                    .read(&store.vault, &project.name, &id)
                    .map_err(|e| AppError::new(ErrorCode::NotFound, e))?;
                sync::check_target(&backup.file).map_err(invalid)?;
                let path = project.path.join(&backup.file);
                if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
                    return Err(invalid(format!(
                        "{} is a symbolic link; LocDev doesn't write through links",
                        backup.file
                    )));
                }
                // What is there now is kept too, so a restore can be undone.
                if let Ok(current) = std::fs::read(&path) {
                    self.env_backups
                        .take(&store.vault, &project.name, &backup.file, &current)
                        .map_err(|e| AppError::new(ErrorCode::Config, e))?;
                }
                sync::write_env_file(&path, &bytes)?;
                drop(store);
                self.env_store().record(
                    "restored",
                    None,
                    None,
                    Some(&project.name),
                    Some(format!("{} from {id}", backup.file)),
                );
                Ok(Response::Ok)
            }

            other => Err(AppError::internal(format!("not an env request: {other:?}"))),
        }
    }

    /// Whether secret values can be read, and how safely the key is kept.
    pub(super) fn env_vault_check(&self) -> Check {
        let vault = self.env_store().vault_status();
        let check =
            |level, message: String| Check::new("env-vault", "Secrets vault", level, message);
        match (&vault.error, self.config.env.key_store) {
            (Some(error), _) => check(CheckLevel::Error, format!("{error}; secret variables are not injected"))
                .with_remedy("unlock the system keyring, or set [env] key_store = \"file\" in config.toml"),
            (None, KeyStore::File) => check(
                CheckLevel::Warn,
                format!("{} secret(s); the key is in a file (env-vault.key), as sensitive as the secrets", vault.secrets),
            )
            .with_remedy("use [env] key_store = \"keyring\" when this machine has one"),
            (None, KeyStore::Keyring) => check(
                CheckLevel::Ok,
                format!("{} secret(s), key in the {}", vault.secrets, vault.backend),
            ),
        }
    }

    /// A removed project's choices and backups go with it.
    pub(super) fn env_forget(&self, name: &str) {
        let _ = self.env_store().forget_project(name);
        self.env_backups.forget(name);
        self.env_applied.lock().unwrap().remove(name);
    }
}
