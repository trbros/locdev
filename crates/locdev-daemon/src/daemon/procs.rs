//! A site's auxiliary processes: the queue workers, schedulers and servers
//! its adapters offer, plus the ones the user adds. Like the Vite helper,
//! nothing offered runs until the user decides (`ask` runs nothing), and
//! alternatives that would consume the same queue never run together.

use locdev_core::model::{
    AuxProcessView, DevServerMode, ProcessKind, ProcessOffer, Project, ProjectProcess,
};
use locdev_core::protocol::{ProcessAction, Request, Response};
use locdev_core::{AppError, ErrorCode};
use tracing::info;

use super::Daemon;
use crate::supervisor::{Spec, resolve_command};

/// The supervisor's key for an auxiliary process.
pub(super) fn aux_id(project: &str, process: &str) -> String {
    format!("{project}:{process}")
}

fn check_process_id(id: &str) -> Result<(), AppError> {
    let ok = !id.is_empty()
        && id.len() <= 40
        && !matches!(id, "main" | "dev")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(AppError::invalid(format!(
            "`{id}` is not a valid process name (lowercase letters, digits and -; not main or dev)"
        )))
    }
}

impl Daemon {
    /// Offered processes merged with the user's choices and custom ones.
    pub(super) fn aux_views(&self, project: &Project) -> Vec<AuxProcessView> {
        let offers = self.stack_of(project).processes;
        let mut views: Vec<AuxProcessView> = offers
            .iter()
            .map(|offer| {
                let choice = project
                    .processes
                    .get(&offer.id)
                    .cloned()
                    .unwrap_or_default();
                AuxProcessView {
                    id: offer.id.clone(),
                    name: offer.name.clone(),
                    kind: offer.kind,
                    command: Some(
                        choice
                            .command
                            .clone()
                            .unwrap_or_else(|| offer.command.clone()),
                    ),
                    offered: Some(offer.command.clone()),
                    custom: false,
                    source: Some(offer.source.clone()),
                    mode: choice.mode,
                    port: choice.port,
                    conflicts: offer.conflicts.clone(),
                    note: offer.note.clone(),
                    info: self.supervisor.info(&aux_id(&project.name, &offer.id)),
                }
            })
            .collect();
        for (id, choice) in &project.processes {
            if offers.iter().any(|o| &o.id == id) {
                continue;
            }
            // A decision about a process no longer offered, without a
            // command of its own, has nothing to run; it isn't listed.
            let Some(command) = &choice.command else {
                continue;
            };
            views.push(AuxProcessView {
                id: id.clone(),
                name: choice.label.clone().unwrap_or_else(|| id.clone()),
                kind: ProcessKind::Other,
                command: Some(command.clone()),
                offered: None,
                custom: true,
                source: None,
                mode: choice.mode,
                port: choice.port,
                conflicts: Vec::new(),
                note: None,
                info: self.supervisor.info(&aux_id(&project.name, id)),
            });
        }
        views
    }

    fn processes_response(&self, project: &Project) -> Response {
        Response::ProjectProcesses {
            processes: self.aux_views(project),
        }
    }

    /// Starts one auxiliary process. It needs a decision other than ask or
    /// never, and none of its alternatives running.
    pub(super) async fn launch_aux(&self, project: &Project, id: &str) -> Result<(), AppError> {
        let views = self.aux_views(project);
        let view = views.iter().find(|v| v.id == id).ok_or_else(|| {
            AppError::not_found(format!("{} has no process `{id}`", project.name))
        })?;
        match view.mode {
            DevServerMode::Ask => {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "LocDev doesn't run {} for {} until you decide; use `locdev process mode {id} manual {}` or the app",
                        view.name, project.name, project.name
                    ),
                ));
            }
            DevServerMode::Never => {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!("{} is turned off for {}", view.name, project.name),
                ));
            }
            DevServerMode::Manual | DevServerMode::Auto => {}
        }
        if let Some(other) = view
            .conflicts
            .iter()
            .find(|c| self.supervisor.is_active(&aux_id(&project.name, c)))
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!(
                    "{} is running and would consume the same jobs; stop it before starting {}",
                    other, view.name
                ),
            ));
        }
        let command = view.command.clone().unwrap_or_default();
        if command.contains("{port}") && view.port.is_none() {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                format!(
                    "{} needs a port; set its mode again to assign one",
                    view.name
                ),
            ));
        }
        let command = resolve_command(&project.name, Some(&command), view.port)?;

        self.runtimes.wait_detected().await;
        let (mut env, notes) = self.shared_env(project, locdev_core::model::ProcessRole::Main);
        env.extend(self.runtimes.env_for(project)?);
        self.adapter_env(project, view.port, &mut env);
        self.supervisor.start(Spec {
            id: aux_id(&project.name, id),
            command,
            cwd: project.path.clone(),
            port: view.port,
            env,
            restart_on_failure: matches!(view.kind, ProcessKind::Worker | ProcessKind::Scheduler),
            notes,
        })?;
        info!(project = %project.name, process = id, "auxiliary process started");
        Ok(())
    }

    /// The processes set to start with the site.
    pub(super) async fn launch_auto_aux(&self, project: &Project) {
        for view in self.aux_views(project) {
            if view.mode == DevServerMode::Auto
                && let Err(error) = self.launch_aux(project, &view.id).await
            {
                tracing::warn!(project = %project.name, process = %view.id, %error, "auxiliary process did not start");
            }
        }
    }

    /// Every auxiliary process of the site that is running.
    pub(super) fn active_aux(&self, project: &Project) -> Vec<String> {
        let prefix = format!("{}:", project.name);
        self.supervisor
            .active_of(&project.name)
            .into_iter()
            .filter_map(|id| id.strip_prefix(&prefix).map(str::to_string))
            .filter(|id| id != "dev")
            .collect()
    }

    pub(super) async fn procs_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::ProjectProcesses { name } => {
                let project = self.project(&name).await?;
                Ok(self.processes_response(&project))
            }

            Request::ControlProcess {
                name,
                process,
                action,
            } => {
                let project = self.project(&name).await?;
                let id = aux_id(&project.name, &process);
                match action {
                    ProcessAction::Start => self.launch_aux(&project, &process).await?,
                    ProcessAction::Stop => {
                        self.supervisor.stop(&id).await;
                    }
                    ProcessAction::Restart => {
                        self.supervisor.stop(&id).await;
                        self.launch_aux(&project, &process).await?;
                    }
                }
                Ok(self.processes_response(&project))
            }

            Request::SetProcess {
                name,
                process,
                mode,
                command,
                label,
            } => {
                check_process_id(&process)?;
                let current = self.project(&name).await?;
                let offer: Option<ProcessOffer> = self
                    .stack_of(&current)
                    .processes
                    .into_iter()
                    .find(|o| o.id == process);
                let command = command.map(|c| c.trim().to_string());
                if offer.is_none()
                    && !current.processes.contains_key(&process)
                    && command.as_deref().is_none_or(str::is_empty)
                {
                    return Err(AppError::invalid(format!(
                        "no process `{process}` is offered for {name}; to add one, give it a command"
                    )));
                }
                // Turning one on turns its alternatives off.
                if matches!(mode, Some(DevServerMode::Auto | DevServerMode::Manual))
                    && let Some(offer) = &offer
                {
                    for other in &offer.conflicts {
                        if self.supervisor.is_active(&aux_id(&name, other)) {
                            return Err(AppError::new(
                                ErrorCode::Conflict,
                                format!(
                                    "{other} is running; stop it before choosing {}",
                                    offer.name
                                ),
                            ));
                        }
                    }
                }

                let mut registry = self.registry.lock().await;
                let taken: Vec<u16> = registry.ports_in_use().collect();
                let project = registry.get_mut(&name)?;
                if let (Some(offer), Some(DevServerMode::Auto | DevServerMode::Manual)) =
                    (&offer, mode)
                {
                    for other in &offer.conflicts {
                        if let Some(choice) = project.processes.get_mut(other)
                            && choice.mode == DevServerMode::Auto
                        {
                            choice.mode = DevServerMode::Manual;
                        }
                    }
                }
                let needs_port = offer.as_ref().is_some_and(|o| o.port)
                    || command.as_deref().is_some_and(|c| c.contains("{port}"));
                let entry = project
                    .processes
                    .entry(process.clone())
                    .or_insert_with(ProjectProcess::default);
                if let Some(mode) = mode {
                    entry.mode = mode;
                }
                match command {
                    Some(c) if c.is_empty() => entry.command = None,
                    Some(c) => entry.command = Some(c),
                    None => {}
                }
                if let Some(label) = label
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                {
                    entry.label = Some(label);
                }
                if needs_port
                    && entry.port.is_none()
                    && matches!(entry.mode, DevServerMode::Manual | DevServerMode::Auto)
                {
                    entry.port = Some(self.free_port(&taken)?);
                }
                let project = project.clone();
                self.commit(&registry)?;
                drop(registry);
                info!(project = %name, %process, ?mode, "process decision recorded");
                Ok(self.processes_response(&project))
            }

            Request::RemoveProcess { name, process } => {
                self.supervisor.stop(&aux_id(&name, &process)).await;
                let mut registry = self.registry.lock().await;
                let project = registry.get_mut(&name)?;
                if project.processes.remove(&process).is_none() {
                    return Err(AppError::not_found(format!(
                        "{name} has no decision about `{process}`"
                    )));
                }
                let project = project.clone();
                self.commit(&registry)?;
                Ok(self.processes_response(&project))
            }

            other => Err(AppError::internal(format!(
                "not a process request: {other:?}"
            ))),
        }
    }
}
