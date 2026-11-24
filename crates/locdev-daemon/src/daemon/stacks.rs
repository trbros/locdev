//! Framework adapters in the daemon: each project's stack (with the user's
//! choices), what a process needs from it (port variables, the Python
//! virtual environment), the `stack` requests and the signed definitions.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use locdev_core::frameworks::{Prerequisite, StackReport};
use locdev_core::model::{Project, Stack};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_frameworks::{Catalog, Overrides, PackageManager, info};
use tracing::info;

use super::Daemon;
use crate::fsutil::find_program;

/// How long a project's stack (read from its files) is trusted.
const STACK_TTL: Duration = Duration::from_secs(3);

impl Daemon {
    pub(super) fn catalog(&self) -> Arc<Catalog> {
        self.definitions.active()
    }

    /// The project's stack with the user's choices, cached for a few seconds.
    pub(super) fn stack_of(&self, project: &Project) -> Stack {
        let key = format!(
            "{}\0{:?}\0{:?}\0{:?}",
            project.path.display(),
            project.adapter,
            project.stacks,
            project.hidden_stacks
        );
        let mut cache = self.stacks.lock().unwrap();
        if let Some((at, stack)) = cache.get(&key)
            && at.elapsed() < STACK_TTL
        {
            return stack.clone();
        }
        let stack = locdev_frameworks::analyze(
            &self.catalog(),
            &project.path,
            Overrides {
                adapter: project.adapter.as_deref(),
                add: &project.stacks,
                hide: &project.hidden_stacks,
            },
        );
        cache.insert(key, (Instant::now(), stack.clone()));
        stack
    }

    /// What the adapter adds to a process's environment: the port in the
    /// framework's own variables, and the Python virtual environment first
    /// on PATH. `env` already holds PATH when runtimes were resolved.
    pub(super) fn adapter_env(
        &self,
        project: &Project,
        port: Option<u16>,
        env: &mut Vec<(String, String)>,
    ) {
        env.extend(self.mail_env(project));
        // LocDev's PHP settings first, so a prepend file set there is the one
        // the shim chains (the shim's folder comes last and wins).
        self.php_ini_env(project, env);
        self.php_shim_env(project, env);
        let stack = self.stack_of(project);
        if let Some(port) = port {
            for entry in &stack.port_env {
                let (name, template) = entry.split_once('=').unwrap_or((entry, "{port}"));
                // A fixed value (`ASPNETCORE_ENVIRONMENT=Development`) is a
                // default: the site's own variables keep theirs.
                if !template.contains("{port}") && env.iter().any(|(k, _)| k == name) {
                    continue;
                }
                env.push((
                    name.to_string(),
                    template.replace("{port}", &port.to_string()),
                ));
            }
        }
        if let Some(venv) = &stack.venv {
            let root = project.path.join(venv);
            let bin = if cfg!(windows) {
                root.join("Scripts")
            } else {
                root.join("bin")
            };
            let current = env
                .iter()
                .rev()
                .find(|(k, _)| k == "PATH")
                .map(|(_, v)| std::ffi::OsString::from(v))
                .or_else(|| std::env::var_os("PATH"))
                .unwrap_or_default();
            if let Ok(joined) =
                std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&current)))
            {
                env.push(("PATH".into(), joined.to_string_lossy().into_owned()));
                env.push(("VIRTUAL_ENV".into(), root.display().to_string()));
            }
        }
    }

    fn stack_report(
        &self,
        target: String,
        path: &Path,
        stack: Stack,
        project: Option<&Project>,
    ) -> StackReport {
        let catalog = self.catalog();
        let definition = stack.adapter.as_deref().and_then(|id| catalog.get(id));
        let pm = path
            .join("package.json")
            .is_file()
            .then(|| PackageManager::detect(path));
        // The process PATH LocDev would use: managed runtimes and the venv.
        let mut env = project
            .and_then(|p| self.runtimes.env_for(p).ok())
            .unwrap_or_default();
        if let Some(project) = project {
            self.adapter_env(project, None, &mut env);
        }
        let path_var = env
            .iter()
            .rev()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        let prerequisites = stack
            .requires
            .iter()
            .map(|program| {
                // `python3` is `python` on Windows and in many venvs.
                let found = find_program(program, &path_var).or_else(|| {
                    (program == "python3")
                        .then(|| find_program("python", &path_var))
                        .flatten()
                });
                Prerequisite {
                    program: program.clone(),
                    found,
                }
            })
            .collect();
        StackReport {
            target,
            path: path.to_path_buf(),
            command: project.and_then(|p| p.command.clone()),
            install: definition.and_then(|d| info::with_pm(&d.install, pm)),
            build: definition.and_then(|d| info::with_pm(&d.build, pm)),
            test: definition.and_then(|d| info::with_pm(&d.test, pm)),
            definition: definition.map(info::info),
            prerequisites,
            adapter_override: project.and_then(|p| p.adapter.clone()),
            added: project.map(|p| p.stacks.clone()).unwrap_or_default(),
            hidden: project.map(|p| p.hidden_stacks.clone()).unwrap_or_default(),
            stack,
        }
    }

    /// Which definitions are in use and whether the last update failed.
    pub(super) fn definitions_check(&self) -> locdev_core::model::Check {
        use locdev_core::frameworks::DefinitionsOrigin;
        use locdev_core::model::{Check, CheckLevel};
        let status = self.definitions.status();
        let origin = match status.origin {
            DefinitionsOrigin::Bundled => "bundled",
            DefinitionsOrigin::Downloaded => "downloaded, signature verified",
        };
        let message = format!(
            "revision {} ({origin}), {} frameworks",
            status.revision, status.count
        );
        match status.error {
            Some(error) => Check::new(
                "definitions",
                "Framework definitions",
                CheckLevel::Warn,
                format!("{message}; last update: {error}"),
            )
            .with_remedy("locdev stack definitions --refresh"),
            None => Check::new(
                "definitions",
                "Framework definitions",
                CheckLevel::Ok,
                message,
            ),
        }
    }

    fn definitions_response(&self) -> Response {
        Response::StackDefinitions {
            definitions: self.catalog().frameworks.iter().map(info::info).collect(),
            registry: self.definitions.status(),
        }
    }

    /// Fetches newer definitions on a worker thread.
    pub(super) async fn refresh_definitions(&self, force: bool) -> Result<String, AppError> {
        let registry = self.definitions.clone();
        let outcome = tokio::task::spawn_blocking(move || registry.refresh(force))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
        self.stacks.lock().unwrap().clear();
        Ok(match outcome {
            locdev_frameworks::registry::Outcome::Updated { from, to } => {
                info!(from, to, "framework definitions updated");
                format!("updated from revision {from} to {to}")
            }
            locdev_frameworks::registry::Outcome::UpToDate => "already up to date".into(),
            locdev_frameworks::registry::Outcome::Skipped(r) => {
                format!("revision {r} was rolled back before; skipped")
            }
        })
    }

    pub(super) async fn stack_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::StackDefinitions => Ok(self.definitions_response()),

            Request::StackDetect { name, path } => match (name, path) {
                (Some(name), _) => {
                    let project = self.project(&name).await?;
                    let stack = self.stack_of(&project);
                    Ok(Response::StackReport(self.stack_report(
                        name,
                        &project.path.clone(),
                        stack,
                        Some(&project),
                    )))
                }
                (None, Some(path)) => {
                    let path: PathBuf = std::fs::canonicalize(&path).map_err(|e| {
                        AppError::new(ErrorCode::NotFound, format!("{}: {e}", path.display()))
                    })?;
                    if !path.is_dir() {
                        return Err(AppError::invalid(format!(
                            "{} is not a folder",
                            path.display()
                        )));
                    }
                    let stack =
                        locdev_frameworks::analyze(&self.catalog(), &path, Overrides::default());
                    Ok(Response::StackReport(self.stack_report(
                        path.display().to_string(),
                        &path,
                        stack,
                        None,
                    )))
                }
                (None, None) => Err(AppError::invalid("pass a site name or a folder")),
            },

            Request::StackSet {
                name,
                add,
                remove,
                adapter,
                apply_command,
            } => {
                let catalog = self.catalog();
                for id in add
                    .iter()
                    .chain(&remove)
                    .chain(adapter.iter().filter(|a| !a.is_empty()))
                {
                    if catalog.get(id).is_none() {
                        return Err(AppError::not_found(format!(
                            "no framework definition `{id}` (see `locdev stack list`)"
                        )));
                    }
                }
                let mut registry = self.registry.lock().await;
                let project = registry.get_mut(&name)?;
                for id in add {
                    project.hidden_stacks.retain(|h| h != &id);
                    if !project.stacks.contains(&id) {
                        project.stacks.push(id);
                    }
                }
                for id in remove {
                    if project.stacks.contains(&id) {
                        project.stacks.retain(|s| s != &id);
                    } else if !project.hidden_stacks.contains(&id) {
                        project.hidden_stacks.push(id);
                    }
                }
                if let Some(adapter) = adapter {
                    project.adapter = (!adapter.is_empty()).then_some(adapter);
                }
                let snapshot = project.clone();
                let stack = locdev_frameworks::analyze(
                    &catalog,
                    &snapshot.path,
                    Overrides {
                        adapter: snapshot.adapter.as_deref(),
                        add: &snapshot.stacks,
                        hide: &snapshot.hidden_stacks,
                    },
                );
                if apply_command {
                    let Some(command) = stack.command.clone() else {
                        return Err(AppError::new(
                            ErrorCode::NotRunnable,
                            format!(
                                "{} has no start command LocDev can build for this project; set one with `locdev edit {name} --command`",
                                stack.framework
                            ),
                        ));
                    };
                    project.command = Some(command);
                    project.kind = stack.kind;
                    if project.port.is_none() {
                        let taken: Vec<u16> = registry.ports_in_use().collect();
                        let port = self.free_port(&taken)?;
                        registry.get_mut(&name)?.port = Some(port);
                    }
                }
                let project = registry.get_mut(&name)?.clone();
                self.commit(&registry)?;
                drop(registry);
                self.stacks.lock().unwrap().clear();
                info!(project = %name, adapter = ?project.adapter, "stack changed by hand");
                let stack = self.stack_of(&project);
                Ok(Response::StackReport(self.stack_report(
                    name,
                    &project.path.clone(),
                    stack,
                    Some(&project),
                )))
            }

            Request::RefreshDefinitions { force } => {
                if self.settings.get().network.offline {
                    return Err(AppError::new(
                        ErrorCode::Remote,
                        "LocDev is in offline mode",
                    ));
                }
                // The outcome is in the registry status; errors are too.
                self.refresh_definitions(force).await?;
                Ok(self.definitions_response())
            }

            Request::RollbackDefinitions => {
                self.definitions
                    .rollback()
                    .map_err(|e| AppError::new(ErrorCode::Conflict, e))?;
                self.stacks.lock().unwrap().clear();
                Ok(self.definitions_response())
            }

            other => Err(AppError::internal(format!(
                "not a stack request: {other:?}"
            ))),
        }
    }
}
