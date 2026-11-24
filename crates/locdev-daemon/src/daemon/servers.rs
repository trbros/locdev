//! Starting a project's processes (its dev server and the optional helper),
//! the per-tool details that make them work behind the proxy, the Processes
//! listing and the per-site HTTPS diagnosis.

use std::time::Duration;

use locdev_core::model::{
    Check, CheckLevel, DevServerMode, DevTool, DnsStatus, ProcessRole, ProcessView, Project,
    ProjectKind, Routing, TlsStatus,
};
use locdev_core::{AppError, ErrorCode, TLD};

use super::Daemon;
use crate::supervisor::{Spec, resolve_command};

impl Daemon {
    /// Starts a project's main process (with its runtimes on PATH) and, when
    /// the user chose so, its helper.
    pub(super) async fn launch(&self, project: &Project) -> Result<(), AppError> {
        self.runtimes.wait_detected().await;
        let spec = self.main_spec(project)?;
        self.supervisor.start(spec)?;
        if project
            .dev_server
            .as_ref()
            .is_some_and(|d| d.mode == DevServerMode::Auto)
            && let Err(error) = self.launch_dev(project).await
        {
            tracing::warn!(project = %project.name, %error, "helper dev server did not start");
        }
        self.launch_auto_aux(project).await;
        Ok(())
    }

    /// Restarts the main process, and the helper if it was running. Runtimes
    /// are resolved first: a missing pin must not take the running site down.
    pub(super) async fn restart(&self, project: &Project) -> Result<(), AppError> {
        self.runtimes.wait_detected().await;
        let spec = self.main_spec(project)?;
        let main = ProcessRole::Main.id(&project.name);
        self.supervisor.stop(&main).await;
        if let Some(port) = spec.port {
            wait_port_free(port).await;
        }
        self.supervisor.start(spec)?;

        let dev = ProcessRole::Dev.id(&project.name);
        if self.supervisor.is_active(&dev) {
            self.supervisor.stop(&dev).await;
            self.launch_dev(project).await?;
        }
        for aux in self.active_aux(project) {
            self.supervisor
                .stop(&super::procs::aux_id(&project.name, &aux))
                .await;
            self.launch_aux(project, &aux).await?;
        }
        Ok(())
    }

    /// Stops every process of the site: helpers and auxiliaries first.
    pub(super) async fn stop_all_of(&self, name: &str) {
        let main = ProcessRole::Main.id(name);
        let others: Vec<String> = self
            .supervisor
            .active_of(name)
            .into_iter()
            .filter(|id| *id != main)
            .collect();
        futures_util::future::join_all(others.iter().map(|id| self.supervisor.stop(id))).await;
        self.supervisor.stop(&main).await;
    }

    fn main_spec(&self, project: &Project) -> Result<Spec, AppError> {
        let command = resolve_command(&project.name, project.command.as_deref(), project.port)?;
        // Shared variables first: LocDev's own (PATH, the proxy hosts) win.
        let (mut env, notes) = self.shared_env(project, ProcessRole::Main);
        env.extend(self.runtimes.env_for(project)?);
        self.adapter_env(project, project.port, &mut env);
        env.extend(proxy_env(project));
        Ok(Spec {
            id: ProcessRole::Main.id(&project.name),
            command,
            cwd: project.path.clone(),
            port: project.port,
            env,
            restart_on_failure: project.restart_on_failure,
            notes,
        })
    }

    /// Starts the helper dev server. Needs a recorded decision other than
    /// "never"; "ask" counts as not decided, so nothing runs.
    pub(super) async fn launch_dev(&self, project: &Project) -> Result<(), AppError> {
        let dev = project.dev_server.as_ref().ok_or_else(|| {
            AppError::new(
                ErrorCode::NotRunnable,
                format!("{} has no helper dev server", project.name),
            )
        })?;
        match dev.mode {
            DevServerMode::Ask => {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "LocDev doesn't run {} for {} until you decide; use `locdev dev mode manual {}` or the app",
                        dev.tool.label(),
                        project.name,
                        project.name
                    ),
                ));
            }
            DevServerMode::Never => {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "managing {} is turned off for {}",
                        dev.tool.label(),
                        project.name
                    ),
                ));
            }
            DevServerMode::Manual | DevServerMode::Auto => {}
        }

        self.runtimes.wait_detected().await;
        let command = resolve_command(&project.name, Some(&dev.command), dev.port)?;
        let (mut env, notes) = self.shared_env(project, ProcessRole::Dev);
        env.extend(self.runtimes.env_for(project)?);
        self.adapter_env(project, dev.port, &mut env);
        env.extend(proxy_env(project));
        env.extend(self.helper_tls_env(project, dev.tool)?);
        self.supervisor.start(Spec {
            id: ProcessRole::Dev.id(&project.name),
            command,
            cwd: project.path.clone(),
            port: dev.port,
            env,
            restart_on_failure: false,
            notes,
        })?;
        Ok(())
    }

    /// Laravel's Vite plugin serves HTTPS itself when it gets a certificate
    /// (`VITE_DEV_SERVER_KEY/CERT`) and the site's address (`APP_URL`). With
    /// the local CA trusted and `*.test` resolving, the helper gets both, so
    /// an HTTPS page loads its assets without mixed content. Nothing in the
    /// project changes; the variables only reach this process.
    fn helper_tls_env(
        &self,
        project: &Project,
        tool: DevTool,
    ) -> Result<Vec<(String, String)>, AppError> {
        let tls = self.tls_status();
        let routed = self.dns.lock().unwrap().routing == Routing::Locdev;
        let laravel = self.stack_of(project).kind == ProjectKind::Laravel;
        let (Some(authority), true, true, true, DevTool::Vite) =
            (&self.authority, tls.trusted, routed, laravel, tool)
        else {
            return Ok(Vec::new());
        };
        let dir = self.paths.data_dir.join("tls").join("sites");
        let (cert, key) = authority.issue_files(&project.name, &dir).map_err(|e| {
            AppError::internal(format!("could not issue a certificate for Vite: {e}"))
        })?;
        Ok(vec![
            ("VITE_DEV_SERVER_CERT".into(), cert.display().to_string()),
            ("VITE_DEV_SERVER_KEY".into(), key.display().to_string()),
            ("APP_URL".into(), format!("https://{}", project.domain)),
        ])
    }

    /// Records the user's decision about the helper. Without a recorded helper,
    /// the one detected from the project's files (or `command`) is used.
    pub(super) async fn set_dev_server(
        &self,
        name: &str,
        mode: DevServerMode,
        command: Option<String>,
    ) -> Result<Project, AppError> {
        let detected = self.stack_of(&self.project(name).await?).dev_server;

        let mut registry = self.registry.lock().await;
        let taken: Vec<u16> = registry.ports_in_use().collect();
        let project = registry.get_mut(name)?;
        let mut dev = match (project.dev_server.clone(), detected, command.as_deref()) {
            (Some(dev), ..) => dev,
            (None, Some(dev), _) => dev,
            (None, None, Some(_)) => locdev_core::model::DevServer {
                tool: DevTool::Vite,
                command: String::new(),
                port: None,
                mode,
            },
            (None, None, None) => {
                return Err(AppError::not_found(format!(
                    "no helper dev server detected for {name}; pass a command"
                )));
            }
        };
        if let Some(command) = command
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
        {
            dev.command = command;
        }
        dev.mode = mode;
        // A port is only needed once it may run.
        if dev.port.is_none() && matches!(mode, DevServerMode::Manual | DevServerMode::Auto) {
            dev.port = Some(self.free_port(&taken)?);
        }
        project.dev_server = Some(dev);
        let project = project.clone();
        self.commit(&registry)?;
        tracing::info!(project = %name, ?mode, "helper dev server decision recorded");
        Ok(project)
    }

    /// Every supervised process of every project, with CPU and memory.
    pub(super) async fn processes(&self) -> Vec<ProcessView> {
        let projects = self.registry.lock().await.all().to_vec();
        let mut views = Vec::new();
        for project in &projects {
            let runtimes: Vec<String> = self
                .runtimes
                .for_project(project)
                .into_iter()
                .filter_map(|r| Some(format!("{} {}", r.kind.executable(), r.version?.version)))
                .collect();
            let mut roles = vec![ProcessRole::Main];
            if project.dev_server.is_some() {
                roles.push(ProcessRole::Dev);
            }
            for role in roles {
                let id = role.id(&project.name);
                let info = self.supervisor.info(&id);
                let (label, command, port) = match role {
                    ProcessRole::Main => (
                        project
                            .command
                            .as_deref()
                            .and_then(|c| c.split_whitespace().next())
                            .unwrap_or("server")
                            .to_string(),
                        project.command.clone(),
                        project.port,
                    ),
                    ProcessRole::Dev => {
                        let dev = project.dev_server.as_ref().expect("filtered above");
                        (
                            dev.tool.label().to_string(),
                            Some(dev.command.clone()),
                            dev.port,
                        )
                    }
                };
                views.push(ProcessView {
                    project: project.name.clone(),
                    role,
                    aux: None,
                    label,
                    command,
                    port,
                    info,
                    cpu: None,
                    memory: None,
                    runtimes: runtimes.clone(),
                    log_file: self.supervisor.log_path(&id).filter(|p| p.is_file()),
                });
            }
            // Auxiliary processes the user turned on, or that are running.
            for aux in self.aux_views(project) {
                if aux.mode == DevServerMode::Ask && aux.info.pid.is_none() {
                    continue;
                }
                let id = super::procs::aux_id(&project.name, &aux.id);
                views.push(ProcessView {
                    project: project.name.clone(),
                    role: ProcessRole::Main,
                    aux: Some(aux.id.clone()),
                    label: aux.name.clone(),
                    command: aux.command.clone(),
                    port: aux.port,
                    info: aux.info.clone(),
                    cpu: None,
                    memory: None,
                    runtimes: runtimes.clone(),
                    log_file: self.supervisor.log_path(&id).filter(|p| p.is_file()),
                });
            }
        }

        let pids: Vec<u32> = views.iter().map(|v| v.info.pid.unwrap_or(0)).collect();
        let usage = tokio::task::spawn_blocking(move || {
            locdev_platform::ports::tree_usage(&pids, Duration::from_millis(250))
        })
        .await
        .unwrap_or_default();
        for (view, usage) in views.iter_mut().zip(usage) {
            if view.info.pid.is_some()
                && let Some((cpu, memory)) = usage
            {
                view.cpu = Some(cpu);
                view.memory = Some(memory);
            }
        }
        views
    }

    /// Everything that has to line up for `https://<site>.test` to work.
    pub(super) fn site_https_checks(
        &self,
        project: &Project,
        tls: &TlsStatus,
        dns: &DnsStatus,
    ) -> Vec<Check> {
        let host = format!("{}.{TLD}", project.name);
        let mut checks = Vec::new();

        checks.push(match (tls.enabled, tls.port) {
            (false, _) => Check::new(
                "https-server",
                "HTTPS listener",
                CheckLevel::Error,
                "HTTPS is disabled",
            )
            .with_remedy("set [tls] enabled = true in config.toml and run `locdev restart`"),
            (_, Some(port)) => Check::new(
                "https-server",
                "HTTPS listener",
                CheckLevel::Ok,
                format!("127.0.0.1:{port}"),
            ),
            (_, None) => Check::new(
                "https-server",
                "HTTPS listener",
                CheckLevel::Error,
                tls.error.clone().unwrap_or_else(|| "not listening".into()),
            )
            .with_remedy("locdev doctor"),
        });

        checks.push(match dns.routing {
            Routing::Locdev => Check::new(
                "dns",
                "*.test resolution",
                CheckLevel::Ok,
                format!("{host} → LocDev"),
            ),
            Routing::Other => Check::new(
                "dns",
                "*.test resolution",
                CheckLevel::Warn,
                "*.test resolves through another tool; it may not reach LocDev",
            )
            .with_remedy("locdev dns status"),
            Routing::None => Check::new(
                "dns",
                "*.test resolution",
                CheckLevel::Warn,
                format!(
                    "{host} doesn't resolve yet; https://{}.localhost works meanwhile",
                    project.name
                ),
            )
            .with_remedy("locdev dns install"),
        });

        checks.push(match &self.authority {
            Some(ca) => match ca.issue(Some(&project.name)) {
                Ok(_) => Check::new(
                    "certificate",
                    "Certificate",
                    CheckLevel::Ok,
                    format!("issued on demand for {host} and *.{host}"),
                ),
                Err(error) => Check::new(
                    "certificate",
                    "Certificate",
                    CheckLevel::Error,
                    error.to_string(),
                ),
            },
            None => Check::new(
                "certificate",
                "Certificate",
                CheckLevel::Error,
                tls.error
                    .clone()
                    .unwrap_or_else(|| "the local CA is unavailable".into()),
            ),
        });

        checks.push(if tls.trusted {
            Check::new(
                "trust",
                "System trust",
                CheckLevel::Ok,
                "the LocDev CA is trusted",
            )
        } else {
            Check::new(
                "trust",
                "System trust",
                CheckLevel::Warn,
                "the LocDev CA is not trusted yet; browsers will warn",
            )
            .with_remedy("locdev tls trust")
        });
        if tls.browsers == Some(false) {
            checks.push(
                Check::new(
                    "trust-browsers",
                    "Browser trust",
                    CheckLevel::Warn,
                    "Chrome/Firefox keep their own store and don't trust the CA yet",
                )
                .with_remedy("locdev tls trust"),
            );
        }

        checks.push(match project.port {
            Some(port) => Check::new(
                "port",
                "Dev server port",
                CheckLevel::Ok,
                format!("proxied to 127.0.0.1:{port}"),
            ),
            None => Check::new(
                "port",
                "Dev server port",
                CheckLevel::Error,
                "the site has no port, so nothing is proxied",
            )
            .with_remedy(format!("locdev edit {} --port <port>", project.name)),
        });

        let running = self
            .supervisor
            .is_active(&ProcessRole::Main.id(&project.name));
        checks.push(Check::new(
            "process",
            "Dev server",
            if running {
                CheckLevel::Ok
            } else {
                CheckLevel::Info
            },
            if running { "running" } else { "not running" },
        ));

        checks.push(Check::new(
            "secure",
            "HTTPS only",
            CheckLevel::Info,
            if project.secure {
                "plain HTTP requests are redirected to HTTPS"
            } else {
                "served over both HTTP and HTTPS"
            },
        ));
        checks
    }
}

/// What every project process gets so its dev server answers the proxy: the
/// site's hostnames for Vite-based tools, which otherwise reject `*.test`
/// (`.localhost` is always allowed). Only this process's environment.
fn proxy_env(project: &Project) -> Vec<(String, String)> {
    vec![(
        "__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS".into(),
        format!("{0}.{TLD},.{0}.{TLD}", project.name),
    )]
}

/// The supervisor sees a process as stopped when its shell exits; the dev
/// server under it can hold the port a moment longer, and starting again
/// right away would fail with "port in use".
async fn wait_port_free(port: u16) {
    for _ in 0..30 {
        if std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_the_site_hosts_for_vite() {
        let project = Project {
            name: "shop".into(),
            ..Default::default()
        };
        assert_eq!(
            proxy_env(&project),
            vec![(
                "__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS".to_string(),
                "shop.test,.shop.test".to_string()
            )]
        );
    }
}
