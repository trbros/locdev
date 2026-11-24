//! Daemon state and request dispatch.

use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use locdev_core::deps::{AuditInterval, JobKind, NotificationCategory, Severity};
use locdev_core::git::GithubAuthMethod;
use locdev_core::model::{
    DaemonStatus, DnsStatus, ProcessRole, ProcessState, Project, ProjectView, ProxyStatus,
    RuntimeInfo, RuntimeKind, TerminalRuntimes, TlsStatus, now_ms,
};
use locdev_core::paths::Paths;
use locdev_core::protocol::{JobAction, PROTOCOL_VERSION, Request, Response};
use locdev_core::{AppError, ErrorCode, project_domain, project_name};
use locdev_dns::DnsServer;
use locdev_platform::{resolver, trust};
use locdev_proxy::{Route, Routes};
use locdev_tls::{Authority, CA_NAME, SiteCerts};
use tokio::sync::{Mutex, Notify};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use crate::config::{Config, DnsConfig, ProxyConfig, TlsConfig};
use crate::jobs::{JobControl, Jobs, Spec};
use crate::notify::Notifications;
use crate::registry::Registry;
use crate::runtimes::{Runtimes, validate_pin};
use crate::security::Security;
use crate::settings::SettingsFile;
use crate::supervisor::{LogLimits, Supervisor};

mod actions;
mod config;
mod db;
mod debug;
mod deploy;
mod deps;
mod editors;
mod email;
mod env;
mod extensions;
mod git;
mod github;
mod mail;
mod park;
mod phpini;
mod procs;
mod remote;
mod scaffold;
mod servers;
mod services;
mod stacks;
mod terminal;
mod tinker;
mod tunnels;
mod updates;
mod work;
use crate::{dns, doctor, notify};

const DEFAULT_LOG_LIMIT: usize = 500;

pub struct Daemon {
    pub paths: Paths,
    config: Config,
    started_at: u64,
    /// Until when a LocDev window counts as in front (`ClientPresence`).
    app_in_front: std::sync::Mutex<Option<std::time::Instant>>,
    /// Terminals running project creation and tool installs.
    terminals: terminal::Terminals,
    /// Itself, for work that outlives a request (set by `main`).
    me: OnceLock<std::sync::Weak<Daemon>>,
    registry: Mutex<Registry>,
    pub supervisor: Arc<Supervisor>,
    /// Project stacks, re-read from their files at most every few seconds.
    stacks: std::sync::Mutex<
        std::collections::HashMap<String, (std::time::Instant, locdev_core::model::Stack)>,
    >,
    /// Framework definitions in use (bundled or signed downloads).
    definitions: Arc<locdev_frameworks::registry::Registry>,
    services: services::ServicesState,
    park: park::ParkState,
    tunnels: tunnels::TunnelsState,
    updates: updates::UpdatesState,
    editors: editors::EditorsState,
    debug: debug::DebugState,
    shutdown: Notify,
    routes: Arc<Routes>,
    proxy: std::sync::Mutex<ProxyStatus>,
    dns: std::sync::Mutex<DnsStatus>,
    dns_server: OnceLock<Arc<DnsServer>>,
    pub mail: mail::MailState,
    authority: Option<Arc<Authority>>,
    pub runtimes: Arc<Runtimes>,
    tls: std::sync::Mutex<TlsStatus>,
    pub settings: SettingsFile,
    jobs: Jobs,
    job_wake: Notify,
    job_tokens:
        std::sync::Mutex<std::collections::HashMap<u64, tokio_util::sync::CancellationToken>>,
    pub notifications: Arc<Notifications>,
    registry_cache: deps::RegistryCache,
    limiter: deps::Limiter,
    security: Security,
    /// One Git operation at a time per site.
    git_locks: std::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    github: github::GithubState,
    github_last_check: std::sync::Mutex<u64>,
    action_versions: std::sync::Mutex<actions::VersionCache>,
    env: std::sync::Mutex<locdev_env::EnvStore>,
    env_backups: locdev_env::sync::Backups,
    /// Fingerprint of the shared variables each main process started with.
    env_applied: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl Daemon {
    pub fn load(paths: Paths) -> Result<Self, AppError> {
        let config = Config::load_or_create(&paths.config_file())?;
        let registry = Registry::load(&paths.registry_file())?;
        let routes = Arc::new(Routes::default());
        sync_routes(&routes, &registry);

        let mut tls = TlsStatus {
            enabled: config.tls.enabled,
            requested_port: config.tls.https_port,
            ..Default::default()
        };
        let authority = if config.tls.enabled {
            match Authority::load_or_create(&paths.data_dir.join("tls")) {
                Ok(ca) => {
                    tls.ca_path = Some(ca.cert_path());
                    tls.fingerprint = Some(ca.fingerprint());
                    Some(Arc::new(ca))
                }
                Err(error) => {
                    warn!(%error, "local CA unavailable; HTTPS disabled");
                    tls.error = Some(format!("local CA unavailable: {error}"));
                    None
                }
            }
        } else {
            None
        };

        let runtimes = Arc::new(Runtimes::load(
            &paths.data_dir,
            &paths.config_dir,
            config.runtimes.php_source(),
        )?);

        let supervisor = Arc::new(Supervisor::new(
            paths.log_dir.join("sites"),
            paths.runtime_dir.join("processes.json"),
            LogLimits {
                lines: config.logs.lines,
                file_bytes: config.logs.file_mb * 1024 * 1024,
            },
        ));

        let state = paths.data_dir.join("state");
        let settings = SettingsFile::load(&paths.config_dir.join("settings.toml"))?;
        let jobs = Jobs::load(&state.join("jobs.json"))?;
        let notifications = Arc::new(Notifications::load(&state.join("notifications.json"))?);
        let registry_cache = deps::RegistryCache::load(&state.join("registry-cache.json"));

        let security = Security::load(&paths.config_dir.join("security.toml"))?;
        let key: Box<dyn locdev_env::vault::KeySource> = match config.env.key_store {
            crate::config::KeyStore::Keyring => Box::new(locdev_env::vault::KeyringKey::vault()),
            crate::config::KeyStore::File => Box::new(locdev_env::vault::FileKey(
                paths.config_dir.join("env-vault.key"),
            )),
        };
        let env = locdev_env::EnvStore::load(&paths.config_dir, &state, key)
            .map_err(|e| AppError::new(ErrorCode::Config, format!("shared variables: {e}")))?;

        let source = match (&config.definitions.url, &config.definitions.public_key) {
            (url, key) if !url.is_empty() && !key.is_empty() => {
                locdev_frameworks::registry::Source {
                    url: url.clone(),
                    public_key: key.clone(),
                }
            }
            _ => locdev_frameworks::registry::Source::bundled(),
        };
        let definitions = Arc::new(locdev_frameworks::registry::Registry::load(
            &state.join("definitions"),
            source,
        ));

        let services = services::ServicesState::load(
            paths.data_dir.join("services"),
            paths.config_dir.join("services.toml"),
        )?;

        let park = park::ParkState::load(paths.config_dir.join("parked.toml"))?;

        Ok(Self {
            services,
            park,
            tunnels: Default::default(),
            updates: Default::default(),
            editors: Default::default(),
            debug: debug::DebugState::load(&paths.data_dir.join("observe")),
            definitions,
            env: std::sync::Mutex::new(env),
            env_backups: locdev_env::sync::Backups::new(state.join("env-backups")),
            env_applied: Default::default(),
            security,
            git_locks: Default::default(),
            github: Default::default(),
            github_last_check: Default::default(),
            action_versions: std::sync::Mutex::new(actions::VersionCache::load(
                &state.join("action-versions.json"),
            )),
            settings,
            jobs,
            job_wake: Notify::new(),
            job_tokens: Default::default(),
            notifications,
            registry_cache,
            limiter: deps::Limiter::new(),
            paths,
            config,
            started_at: now_ms(),
            app_in_front: std::sync::Mutex::new(None),
            terminals: terminal::Terminals::default(),
            me: OnceLock::new(),
            registry: Mutex::new(registry),
            supervisor,
            stacks: Default::default(),
            shutdown: Notify::new(),
            routes,
            proxy: std::sync::Mutex::new(ProxyStatus::default()),
            dns: std::sync::Mutex::new(DnsStatus::default()),
            dns_server: OnceLock::new(),
            mail: mail::MailState::default(),
            authority,
            tls: std::sync::Mutex::new(tls),
            runtimes,
        })
    }

    pub fn proxy_config(&self) -> &ProxyConfig {
        &self.config.proxy
    }

    pub fn routes(&self) -> Arc<Routes> {
        self.routes.clone()
    }

    pub fn set_proxy_status(&self, status: ProxyStatus) {
        *self.proxy.lock().unwrap() = status;
    }

    /// Binds the Mailbox; what it captures comes back on the receiver, for
    /// [`Daemon::mail_notifications`].
    pub async fn start_mail(
        &self,
    ) -> tokio::sync::mpsc::UnboundedReceiver<locdev_core::mail::MailSummary> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        mail::start(
            &self.config.mail,
            &self.paths.data_dir.join("mail").join("messages"),
            &self.mail,
            tx,
        )
        .await;
        rx
    }

    pub fn dns_config(&self) -> &DnsConfig {
        &self.config.dns
    }

    pub fn set_dns(&self, status: DnsStatus, server: Option<Arc<DnsServer>>) {
        *self.dns.lock().unwrap() = status;
        if let Some(server) = server {
            let _ = self.dns_server.set(server);
        }
    }

    /// Re-reads the OS resolver configuration and probes the real routing.
    pub async fn refresh_dns(&self) -> DnsStatus {
        let routing = dns::probe(self.dns_server.get().map(Arc::as_ref)).await;
        let backend = resolver::detect();

        let mut status = self.dns.lock().unwrap();
        status.routing = routing;
        status.backend = backend.label().to_string();
        status.installed = resolver::installed(locdev_core::TLD);
        status.points_to = resolver::configured(locdev_core::TLD).map(|a| a.to_string());
        status.conflicts = resolver::conflicts(locdev_core::TLD);
        status.clone()
    }

    pub fn tls_config(&self) -> &TlsConfig {
        &self.config.tls
    }

    /// TLS acceptor that mints site certificates from the local CA.
    pub fn tls_acceptor(&self) -> Option<TlsAcceptor> {
        let authority = self.authority.clone()?;
        match locdev_tls::server_config(SiteCerts::new(authority)) {
            Ok(config) => Some(TlsAcceptor::from(config)),
            Err(error) => {
                warn!(%error, "HTTPS disabled");
                self.tls.lock().unwrap().error = Some(error.to_string());
                None
            }
        }
    }

    pub fn set_https(&self, port: Option<u16>, error: Option<String>) {
        self.routes.set_https_port(port);
        let mut tls = self.tls.lock().unwrap();
        tls.port = port;
        if error.is_some() {
            tls.error = error;
        }
    }

    /// Re-checks whether the OS and browsers trust the CA. Reads trust
    /// stores and runs `certutil`, so it runs off the async threads.
    pub async fn refresh_tls(&self) -> TlsStatus {
        if let Some(ca) = self.authority.clone() {
            let (trusted, browsers) = tokio::task::spawn_blocking(move || {
                (ca.trusted_by_system(), trust::browsers_trust(CA_NAME))
            })
            .await
            .unwrap_or((false, None));
            let mut tls = self.tls.lock().unwrap();
            tls.trusted = trusted;
            tls.browsers = browsers;
        }
        self.tls_status()
    }

    fn tls_status(&self) -> TlsStatus {
        self.tls.lock().unwrap().clone()
    }

    fn dns_status(&self) -> DnsStatus {
        self.dns.lock().unwrap().clone()
    }

    fn proxy_status(&self) -> ProxyStatus {
        let mut status = self.proxy.lock().unwrap().clone();
        status.suffix = self.suffix().to_string();
        status
    }

    /// Site URLs always use `.test`; the proxy still answers `.localhost`
    /// as a fallback while `*.test` doesn't resolve yet.
    fn suffix(&self) -> &'static str {
        locdev_core::TLD
    }

    /// Persists the registry and publishes the new routing table.
    fn commit(&self, registry: &Registry) -> Result<(), AppError> {
        registry.save()?;
        sync_routes(&self.routes, registry);
        Ok(())
    }

    pub async fn autostart(&self) {
        let projects: Vec<Project> = self
            .registry
            .lock()
            .await
            .all()
            .iter()
            .filter(|p| p.auto_start)
            .cloned()
            .collect();

        // Services first: sites usually need their database up.
        self.autostart_instances().await;
        for project in projects {
            if let Err(error) = self.launch(&project).await {
                warn!(project = %project.name, %error, "auto-start failed");
            }
        }
    }

    pub async fn shutdown_requested(&self) {
        self.shutdown.notified().await;
    }

    pub async fn handle(&self, request: Request) -> Response {
        // Anything that shows or starts projects needs the first runtime scan,
        // or system versions would look missing right after boot. `Status`
        // must stay instant: clients use it to know the daemon is up.
        if !matches!(
            request,
            Request::Status
                | Request::Shutdown
                | Request::ProjectLogs { .. }
                | Request::ListProcesses
        ) {
            self.runtimes.wait_detected().await;
        }
        self.dispatch(request).await.unwrap_or_else(Response::Error)
    }

    async fn dispatch(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::Status => Ok(Response::Status(self.status().await)),

            Request::ClientPresence { focused } => {
                // A window that stops reporting (crash, sleep) stops counting.
                *self.app_in_front.lock().unwrap() =
                    focused.then(|| std::time::Instant::now() + std::time::Duration::from_secs(20));
                Ok(Response::Ok)
            }

            Request::Shutdown => {
                info!("shutdown requested over IPC");
                // `notify_one` stores a permit, so the main loop sees it even
                // if it is busy accepting right now.
                self.shutdown.notify_one();
                Ok(Response::Ok)
            }

            Request::Doctor => {
                let dns = self.refresh_dns().await;
                let tls = self.refresh_tls().await;
                let projects = self.registry.lock().await.all().to_vec();
                let mut checks =
                    doctor::run(&self.paths, &projects, &self.proxy_status(), &dns, &tls).await;
                checks.extend(doctor::runtime_checks(
                    &projects,
                    |p| self.runtimes.for_project(p),
                    |resolved| self.runtimes.tool_conflicts(resolved),
                ));
                checks.push(self.env_vault_check());
                checks.push(self.definitions_check());
                Ok(Response::Doctor { checks })
            }

            Request::CheckDns => Ok(Response::Dns(self.refresh_dns().await)),

            Request::CheckTls => Ok(Response::Tls(self.refresh_tls().await)),

            Request::ListRuntimes { refresh } => {
                self.runtimes.wait_detected().await;
                if refresh {
                    self.runtimes.refresh_system().await;
                }
                Ok(Response::Runtimes {
                    runtimes: self.runtime_info().await,
                })
            }

            Request::RuntimeCatalog { kind } => Ok(Response::Catalog {
                releases: self.runtimes.catalog(kind).await?,
            }),

            Request::InstallRuntime { kind, version } => {
                let version = self.runtimes.begin_install(kind, &version).await?;
                self.jobs.enqueue(Spec {
                    kind: JobKind::RuntimeInstall,
                    project: None,
                    input: serde_json::to_value(work::InstallInput { kind, version })
                        .unwrap_or_default(),
                    dedup: format!("install:{}", kind.executable()),
                    automatic: false,
                    max_attempts: 1,
                    not_before: None,
                });
                self.job_wake.notify_one();
                Ok(Response::Runtimes {
                    runtimes: self.runtime_info().await,
                })
            }

            Request::UninstallRuntime {
                kind,
                version,
                force,
            } => {
                let projects = self.registry.lock().await.all().to_vec();
                self.runtimes
                    .uninstall(kind, &version, force, &projects, |name| {
                        matches!(
                            self.supervisor.info(&ProcessRole::Main.id(name)).state,
                            ProcessState::Running | ProcessState::Stopping
                        )
                    })?;
                info!(runtime = kind.executable(), %version, "runtime removed");
                Ok(Response::Runtimes {
                    runtimes: self.runtime_info().await,
                })
            }

            Request::SetDefaultRuntime { kind, version } => {
                self.runtimes.set_default(kind, version)?;
                Ok(Response::Runtimes {
                    runtimes: self.runtime_info().await,
                })
            }

            Request::TerminalRuntimes { enable } => {
                self.runtimes.wait_detected().await;
                let commands = self.runtimes.sync_terminal();
                let dir = self.runtimes.terminal_dir().to_path_buf();
                let path = match enable {
                    Some(true) => locdev_platform::terminal::enable(&dir),
                    Some(false) => locdev_platform::terminal::disable(&dir),
                    None => Ok(locdev_platform::terminal::status(&dir)),
                }
                .map_err(|e| AppError::new(ErrorCode::Io, e.to_string()))?;
                if let Some(on) = enable {
                    info!(enabled = on, files = ?path.files, "terminal runtimes");
                }
                Ok(Response::TerminalRuntimes(TerminalRuntimes {
                    dir,
                    enabled: path.enabled,
                    files: path.files,
                    active: path.active,
                    commands,
                }))
            }

            Request::ListProjects => {
                let registry = self.registry.lock().await;
                let projects = registry.all().iter().map(|p| self.view(p)).collect();
                Ok(Response::Projects { projects })
            }

            Request::AddProject {
                path,
                name,
                command,
                port,
                auto_start,
            } => self.add(path, name, command, port, auto_start).await,

            Request::UpdateProject {
                name,
                command,
                port,
                auto_start,
                node,
                php,
                tools,
                restart_on_failure,
            } => {
                let node = node.map(|v| parse_pin(RuntimeKind::Node, &v)).transpose()?;
                let php = php.map(|v| parse_pin(RuntimeKind::Php, &v)).transpose()?;
                let mut tool_pins = Vec::new();
                for (kind, raw) in tools {
                    if matches!(kind, RuntimeKind::Node | RuntimeKind::Php) {
                        return Err(AppError::invalid(
                            "pin Node.js and PHP with their own fields",
                        ));
                    }
                    tool_pins.push((kind, parse_pin(kind, &raw)?));
                }
                let mut registry = self.registry.lock().await;
                let project = registry.get_mut(&name)?;

                if let Some(node) = node {
                    project.node = node;
                }
                if let Some(php) = php {
                    project.php = php;
                }
                for (kind, pin) in tool_pins {
                    match pin {
                        Some(pin) => project.tools.insert(kind, pin),
                        None => project.tools.remove(&kind),
                    };
                }

                if let Some(command) = command {
                    let command = command.trim();
                    project.command = (!command.is_empty()).then(|| command.to_string());
                }
                if let Some(port) = port {
                    validate_port(port)?;
                    project.port = Some(port);
                }
                if let Some(auto_start) = auto_start {
                    project.auto_start = auto_start;
                }
                if let Some(restart) = restart_on_failure {
                    project.restart_on_failure = restart;
                }

                let project = project.clone();
                self.commit(&registry)?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::DeployPlatforms
            | Request::DeployConnect { .. }
            | Request::DeployDisconnect { .. }
            | Request::DeployRemoteProjects { .. }
            | Request::DeployLinkSite { .. }
            | Request::DeployRemoteStatus { .. }
            | Request::DeployRun { .. }
            | Request::DeployCancel { .. }
            | Request::DeployRollback { .. }
            | Request::DeployLogs { .. } => self.remote_request(request).await,

            Request::UpdateCheck { .. } => self.update_request(request).await,
            Request::ObserveEvents { .. }
            | Request::ObserveEvent { .. }
            | Request::ObserveGroups { .. }
            | Request::ObserveStatus
            | Request::ObserveClear { .. }
            | Request::ObserveDumps { .. } => self.debug_request(request).await,
            Request::EditorList { .. } | Request::EditorOpen { .. } => {
                self.editor_request(request).await
            }

            Request::TunnelProviders
            | Request::TunnelInstall { .. }
            | Request::TunnelList
            | Request::TunnelStart { .. }
            | Request::TunnelStop { .. }
            | Request::TunnelLogs { .. } => self.tunnel_request(request).await,

            Request::PhpConfigLines
            | Request::PhpConfig { .. }
            | Request::PhpConfigChange { .. }
            | Request::PhpIniRead { .. } => self.php_request(request).await,

            Request::ParkList
            | Request::ParkSet { .. }
            | Request::ParkRemove { .. }
            | Request::ParkScan { .. }
            | Request::ParkIgnore { .. } => self.park_request(request).await,

            Request::RemoveProject { name } => {
                self.remove_site(&name).await?;
                Ok(Response::Ok)
            }

            Request::StartProject { name } => {
                let project = self.project(&name).await?;
                self.launch(&project).await?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::StopProject { name } => {
                let project = self.project(&name).await?;
                self.stop_all_of(&name).await;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::RestartProject { name } => {
                let project = self.project(&name).await?;
                self.restart(&project).await?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::ProjectLogs {
                name,
                after,
                limit,
                process,
                aux,
            } => {
                self.project(&name).await?;
                let id = match aux {
                    Some(aux) => procs::aux_id(&name, &aux),
                    None => process.id(&name),
                };
                let lines = self
                    .supervisor
                    .logs(&id, after, limit.unwrap_or(DEFAULT_LOG_LIMIT));
                Ok(Response::Logs { lines })
            }

            Request::ListProcesses => Ok(Response::Processes {
                processes: self.processes().await,
            }),

            Request::SetDevServer {
                name,
                mode,
                command,
                start,
            } => {
                let project = self.set_dev_server(&name, mode, command).await?;
                if start {
                    self.launch_dev(&project).await?;
                }
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::StartDevServer { name } => {
                let project = self.project(&name).await?;
                self.launch_dev(&project).await?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::StopDevServer { name } => {
                let project = self.project(&name).await?;
                self.supervisor.stop(&ProcessRole::Dev.id(&name)).await;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::RestartDevServer { name } => {
                let project = self.project(&name).await?;
                self.supervisor.stop(&ProcessRole::Dev.id(&name)).await;
                self.launch_dev(&project).await?;
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }

            Request::CheckSiteHttps { name } => {
                let project = self.project(&name).await?;
                let tls = self.refresh_tls().await;
                let dns = self.refresh_dns().await;
                Ok(Response::Doctor {
                    checks: self.site_https_checks(&project, &tls, &dns),
                })
            }

            Request::ListDeps { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Deps(self.deps_report(&project).await))
            }

            Request::CheckDeps { name } => {
                self.project(&name).await?;
                if self.settings.get().automation.audit == AuditInterval::Off {
                    info!(project = %name, "dependency check requested while monitoring is off");
                }
                Ok(Response::Job {
                    job: self.queue_audit(&name),
                })
            }

            Request::PlanDepsUpdate {
                name,
                packages,
                allow_major,
                run_scripts,
            } => {
                let project = self.project(&name).await?;
                Ok(Response::DepsPlan(
                    self.plan_update(&project, &packages, allow_major, run_scripts)
                        .await?,
                ))
            }

            Request::UpdateDeps {
                name,
                packages,
                allow_major,
                run_scripts,
            } => {
                let project = self.project(&name).await?;
                // Validate now, so a bad selection fails here, not in the job.
                let plan = self
                    .plan_update(&project, &packages, allow_major, run_scripts)
                    .await?;
                if plan.commands.is_empty() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!("nothing to update: {}", plan.warnings.join("; ")),
                    ));
                }
                let (job, created) = self.jobs.enqueue(Spec {
                    kind: JobKind::DepsUpdate,
                    project: Some(name.clone()),
                    input: serde_json::to_value(work::UpdateInput {
                        packages,
                        allow_major,
                        run_scripts,
                    })
                    .unwrap_or_default(),
                    dedup: format!("update:{name}"),
                    automatic: false,
                    max_attempts: 1,
                    not_before: None,
                });
                if !created {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!("an update for {name} is already queued (job {})", job.id),
                    ));
                }
                self.job_wake.notify_one();
                Ok(Response::Job { job })
            }

            Request::ListJobs { project } => Ok(Response::Jobs {
                jobs: self.jobs.list(project.as_deref()),
            }),

            Request::GetJob { id } => Ok(Response::Job {
                job: self
                    .jobs
                    .get(id)
                    .ok_or_else(|| AppError::not_found(format!("no job {id}")))?,
            }),

            Request::ControlJob { id, action } => {
                let job = match action {
                    JobAction::Cancel => {
                        self.cancel_job(id);
                        self.jobs.get(id)
                    }
                    JobAction::Pause => Some(self.jobs.set_state(id, JobControl::Pause)?),
                    JobAction::Resume => Some(self.jobs.set_state(id, JobControl::Resume)?),
                    JobAction::Retry => Some(self.jobs.set_state(id, JobControl::Retry)?),
                };
                self.job_wake.notify_one();
                Ok(Response::Job {
                    job: job.ok_or_else(|| AppError::not_found(format!("no job {id}")))?,
                })
            }

            Request::ListNotifications { all } => Ok(Response::Notifications {
                notifications: self.notifications.list(all),
                unread: self.notifications.unread(),
            }),

            Request::MarkNotifications {
                ids,
                read,
                dismissed,
            } => {
                self.notifications.mark(&ids, read, dismissed);
                Ok(Response::Ok)
            }

            Request::TestNotification => {
                self.notify(notify::New {
                    category: NotificationCategory::Action,
                    severity: Severity::Info,
                    title: "Notifications work".into(),
                    body: "This is how LocDev will tell you about updates and failures.".into(),
                    project: None,
                    link: None,
                    dedup: format!("test:{}", now_ms()),
                });
                Ok(Response::Ok)
            }

            Request::GetSettings => Ok(Response::Settings(self.settings.get())),

            Request::UpdateSettings { settings } => {
                let before = self.settings.set(settings.clone())?;
                self.settings_changed(&before, &settings);
                if before.observe != settings.observe {
                    self.debug_settings_changed();
                }
                if settings.notifications.native && !before.notifications.native {
                    let notifications = self.notifications.clone();
                    let prefs = settings.notifications.clone();
                    let front = self.app_in_front();
                    tokio::spawn(async move { notifications.deliver(&prefs, front).await });
                }
                Ok(Response::Settings(settings))
            }

            Request::GitStatus { name } => {
                let project = self.project(&name).await?;
                Ok(Response::GitStatus(
                    self.with_repo(&project, |r| r.status()).await?,
                ))
            }

            Request::GitDiff { name, path, staged } => {
                let project = self.project(&name).await?;
                let (text, truncated) = self
                    .with_repo(&project, move |r| r.diff(&path, staged))
                    .await?;
                Ok(Response::Text { text, truncated })
            }

            Request::GitLog {
                name,
                rev,
                skip,
                limit,
            } => {
                let project = self.project(&name).await?;
                let commits = self
                    .with_repo(&project, move |r| {
                        r.log(rev.as_deref(), skip, limit.unwrap_or(50))
                    })
                    .await?;
                Ok(Response::Commits { commits })
            }

            Request::GitShow { name, sha } => {
                let project = self.project(&name).await?;
                Ok(Response::Commit(
                    self.with_repo(&project, move |r| r.show(&sha)).await?,
                ))
            }

            Request::GitBranches { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Branches {
                    branches: self.with_repo(&project, |r| r.branches()).await?,
                })
            }

            Request::GitStashes { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Stashes {
                    stashes: self.with_repo(&project, |r| r.stashes()).await?,
                })
            }

            Request::GitRemotes { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Remotes {
                    remotes: self.with_repo(&project, |r| r.remotes()).await?,
                })
            }

            Request::GitHistory { name } => {
                let project = self.project(&name).await?;
                Ok(Response::GitHistory {
                    reflog: self.with_repo(&project, |r| r.reflog(100)).await?,
                    operations: self.git_operations(&name),
                })
            }

            Request::GitPreview { name, op } => {
                let project = self.project(&name).await?;
                Ok(Response::GitPreview {
                    preview: self.git_preview(&project, op).await?,
                })
            }

            Request::Git { name, op, confirm } => {
                let project = self.project(&name).await?;
                let op = self.worktree_defaults(&project, op)?;
                let result = self.git_run(&project, op.clone(), confirm).await?;
                Ok(Response::GitResult(
                    self.worktree_after(&project, &op, result).await,
                ))
            }

            Request::GitWorktrees { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Worktrees {
                    worktrees: self.git_worktrees(&project).await?,
                })
            }

            Request::GetSecurity => Ok(Response::Security(self.security.status())),

            Request::SetPassword { current, new } => {
                self.security
                    .set_password(current.as_deref(), new.as_deref())?;
                Ok(Response::Security(self.security.status()))
            }

            Request::SetSecurityOptions {
                password,
                require_password,
                backups,
            } => {
                self.security
                    .set_options(password.as_deref(), require_password, backups)?;
                Ok(Response::Security(self.security.status()))
            }

            Request::GithubAccount => Ok(Response::Github {
                account: self.github_account().await?,
            }),

            Request::GithubConnectToken { token } => {
                let token = locdev_github::Token::new(&token).map_err(github::app_error)?;
                Ok(Response::Github {
                    account: Some(self.github_connect(token, GithubAuthMethod::Token).await?),
                })
            }

            Request::GithubConnectGhCli => {
                let token = tokio::task::spawn_blocking(locdev_github::gh_cli_token)
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?
                    .map_err(github::app_error)?;
                Ok(Response::Github {
                    account: Some(self.github_connect(token, GithubAuthMethod::GhCli).await?),
                })
            }

            Request::GithubDeviceStart => {
                Ok(Response::DeviceCode(self.github_device_start().await?))
            }

            Request::GithubDevicePoll => Ok(Response::Github {
                account: self.github_device_poll().await?,
            }),

            Request::GithubDisconnect => {
                self.github_disconnect().await?;
                Ok(Response::Github { account: None })
            }

            Request::Github { name, section } => {
                let project = self.project(&name).await?;
                Ok(Response::GithubData(
                    self.github_data(&project, section).await?,
                ))
            }

            Request::GithubAction { name, action } => {
                let project = self.project(&name).await?;
                let message = self.github_action(&project, action).await?;
                Ok(Response::GitResult(locdev_core::git::GitResult {
                    message,
                    output: String::new(),
                    backup: None,
                }))
            }

            Request::ActionsOverview { name } => {
                let project = self.project(&name).await?;
                self.actions_overview(&project).await
            }
            Request::ActionsTemplate { name, template } => {
                let project = self.project(&name).await?;
                self.actions_template(&project, &template).await
            }
            Request::ActionsRender { workflow } => self.actions_render(workflow).await,
            Request::ActionsValidate { yaml } => {
                Ok(Response::ActionsValidation(self.validate(yaml).await?))
            }
            Request::ActionsRead { name, file } => {
                let project = self.project(&name).await?;
                self.actions_read(&project, file).await
            }
            Request::ActionsPlan { name, file, yaml } => {
                let project = self.project(&name).await?;
                Ok(Response::ActionsPlan(
                    self.actions_plan(&project, file, yaml).await?,
                ))
            }
            Request::ActionsSave {
                name,
                file,
                yaml,
                replace,
            } => {
                let project = self.project(&name).await?;
                self.actions_save(&project, file, yaml, replace).await
            }
            Request::EnvOverview
            | Request::EnvSet { .. }
            | Request::EnvRemove { .. }
            | Request::EnvReveal { .. }
            | Request::EnvSaveProfile { .. }
            | Request::EnvRemoveProfile { .. }
            | Request::EnvProject { .. }
            | Request::EnvSetProject { .. }
            | Request::EnvSyncPlan { .. }
            | Request::EnvSync { .. }
            | Request::EnvBackups { .. }
            | Request::EnvRestore { .. } => self.env_request(request).await,

            Request::StackDefinitions
            | Request::StackDetect { .. }
            | Request::StackSet { .. }
            | Request::RefreshDefinitions { .. }
            | Request::RollbackDefinitions => self.stack_request(request).await,

            Request::ProjectProcesses { .. }
            | Request::ControlProcess { .. }
            | Request::SetProcess { .. }
            | Request::RemoveProcess { .. } => self.procs_request(request).await,

            Request::ListServices { .. }
            | Request::ServiceReleases { .. }
            | Request::InstallService { .. }
            | Request::InstallServicePackage { .. }
            | Request::UninstallService { .. }
            | Request::CreateInstance { .. }
            | Request::ControlInstance { .. }
            | Request::SetInstance { .. }
            | Request::RemoveInstance { .. }
            | Request::BackupInstance { .. }
            | Request::InstanceBackups { .. }
            | Request::RestoreInstance { .. }
            | Request::InstanceLogs { .. } => self.services_request(request).await,

            Request::EmailCatalog { .. }
            | Request::EmailOpen { .. }
            | Request::EmailStat { .. }
            | Request::EmailPreview { .. }
            | Request::EmailBuild { .. }
            | Request::EmailSave { .. }
            | Request::EmailSample { .. }
            | Request::EmailNew { .. }
            | Request::EmailSendTest { .. }
            | Request::EmailHistory { .. }
            | Request::EmailHistoryVersion { .. } => self.email_request(request).await,

            Request::MailboxStatus
            | Request::MailList { .. }
            | Request::MailGet { .. }
            | Request::MailRaw { .. }
            | Request::MailAttachment { .. }
            | Request::MailDelete { .. }
            | Request::MailClear { .. }
            | Request::MailSetup { .. }
            | Request::MailEnable { .. }
            | Request::MailSendTest { .. } => self.mail_request(request).await,

            Request::ListStarters
            | Request::ListTools
            | Request::InstallTool { .. }
            | Request::PlanProject { .. }
            | Request::CreateProject { .. }
            | Request::SearchPackages { .. }
            | Request::ListTerminals
            | Request::TerminalRead { .. }
            | Request::TerminalWrite { .. }
            | Request::TerminalResize { .. }
            | Request::TerminalKill { .. }
            | Request::TerminalClose { .. } => self.scaffold_request(request).await,

            Request::DeployAnalysis { .. }
            | Request::DeployConfig { .. }
            | Request::DeployWriteConfig { .. } => self.deploy_request(request).await,

            Request::ConfigFiles { .. }
            | Request::ConfigFile { .. }
            | Request::ConfigPreview { .. }
            | Request::ConfigSave { .. }
            | Request::ConfigHistory { .. }
            | Request::ConfigRevert { .. }
            | Request::ConfigHistoryVersion { .. }
            | Request::ConfigCheck { .. } => self.config_request(request).await,

            Request::DbConnections { .. }
            | Request::DbSchema { .. }
            | Request::DbTableInfo { .. }
            | Request::DbRows { .. }
            | Request::DbCheck { .. }
            | Request::DbQuery { .. }
            | Request::DbEditRow { .. }
            | Request::DbCreateDatabase { .. }
            | Request::RedisKeys { .. }
            | Request::RedisGet { .. }
            | Request::RedisEdit { .. } => self.db_request(request).await,

            Request::PhpExtensions { name } => {
                let project = self.project(&name).await?;
                Ok(Response::PhpExtensions(
                    self.php_extensions(&project).await?,
                ))
            }

            Request::PhpModules { executable } => {
                Ok(Response::PhpModules(self.php_modules(&executable).await?))
            }

            Request::RequestMetrics { name } => {
                self.project(&name).await?;
                Ok(Response::RequestMetrics(
                    self.routes.metrics.snapshot(&name),
                ))
            }

            Request::ResetRequestMetrics { name } => {
                self.project(&name).await?;
                self.routes.metrics.reset(&name);
                Ok(Response::Ok)
            }

            Request::TinkerRun { name, code } => {
                let project = self.project(&name).await?;
                Ok(Response::Tinker(self.tinker_run(&project, &code).await?))
            }

            Request::TinkerOpen { name } => {
                let project = self.project(&name).await?;
                Ok(Response::Terminal(self.tinker_open(&project).await?))
            }

            Request::ActionVersions => Ok(self.action_versions()),
            Request::RefreshActionVersions => self.refresh_action_versions().await,
            Request::ActionsAudit { name } => {
                let project = self.project(&name).await?;
                self.actions_audit(&project).await
            }
            Request::ActionsHistory { name, file } => {
                let project = self.project(&name).await?;
                self.actions_history(&project, &file)
            }
            Request::ActionsHistoryVersion { name, file, id } => {
                let project = self.project(&name).await?;
                self.actions_history_version(&project, &file, &id).await
            }
            Request::ActionlintInstall => self.actionlint_install().await,

            Request::SetSubdomains { name, subdomains } => {
                let subdomains =
                    locdev_core::subdomains::validate(&subdomains).map_err(AppError::invalid)?;
                let mut registry = self.registry.lock().await;
                let project = registry.get_mut(&name)?;
                for rule in &subdomains.rules {
                    if let locdev_core::subdomains::SubdomainTarget::Process { id } = &rule.target
                        && !project.processes.contains_key(id)
                    {
                        return Err(AppError::invalid(format!(
                            "{name} has no auxiliary process {id}; turn it on first (`locdev process`)"
                        )));
                    }
                }
                project.subdomains = subdomains;
                let project = project.clone();
                self.commit(&registry)?;
                info!(project = %name, rules = project.subdomains.rules.len(), wildcard = project.subdomains.wildcard, "subdomains changed");
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }
            Request::ExplainHost { host } => {
                Ok(Response::HostExplain(self.explain_host(&host).await))
            }
            Request::SetSecure { name, secure } => {
                let mut registry = self.registry.lock().await;
                let project = registry.get_mut(&name)?;
                project.secure = secure;
                let project = project.clone();
                self.commit(&registry)?;
                info!(project = %name, secure, "https-only changed");
                Ok(Response::Project {
                    project: self.view(&project),
                })
            }
        }
    }

    /// Lets requests start work that outlives them (call once, from `main`).
    pub fn keep_self(self: &Arc<Self>) {
        let _ = self.me.set(Arc::downgrade(self));
    }

    fn me(&self) -> Result<Arc<Daemon>, AppError> {
        self.me
            .get()
            .and_then(std::sync::Weak::upgrade)
            .ok_or_else(|| AppError::internal("the daemon is shutting down"))
    }

    /// Whether a LocDev window is in front right now.
    pub fn app_in_front(&self) -> bool {
        self.app_in_front
            .lock()
            .unwrap()
            .is_some_and(|until| until > std::time::Instant::now())
    }

    async fn status(&self) -> DaemonStatus {
        DaemonStatus {
            version: locdev_core::VERSION.to_string(),
            protocol: PROTOCOL_VERSION,
            pid: std::process::id(),
            started_at: self.started_at,
            projects: self.registry.lock().await.all().len(),
            running: self.supervisor.running(),
            home: self.paths.home.clone(),
            proxy: self.proxy_status(),
            dns: self.dns_status(),
            tls: self.tls_status(),
            mail: self.mailbox_status(),
            unread_notifications: self
                .notifications
                .unread_except(locdev_core::deps::NotificationCategory::Mail),
        }
    }

    async fn runtime_info(&self) -> Vec<RuntimeInfo> {
        let projects = self.registry.lock().await.all().to_vec();
        self.runtimes.info(&projects)
    }

    /// DNS through the OS, the proxy's route and the certificate, separately.
    async fn explain_host(&self, raw: &str) -> locdev_core::subdomains::HostExplain {
        use locdev_core::subdomains::{HostDns, HostExplain, HostRoute, HostTls};
        use locdev_proxy::Destination;
        let host = locdev_core::host::strip_port(raw.trim()).to_ascii_lowercase();

        let mut dns = HostDns::default();
        match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::net::lookup_host((host.as_str(), 80)),
        )
        .await
        {
            Ok(Ok(addrs)) => {
                let mut answers: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                answers.dedup();
                dns.answers = answers;
            }
            Ok(Err(e)) => dns.error = Some(e.to_string()),
            Err(_) => dns.error = Some("the lookup timed out".into()),
        }
        dns.via_locdev = self.dns_server.get().map(|s| s.saw(&host));

        let mut route = HostRoute::default();
        match locdev_core::host::target(&host) {
            locdev_core::host::Target::Site { name, sub, .. } => {
                route.sub = sub.to_string();
                let project = self.registry.lock().await.get(name).ok().cloned();
                match project {
                    None => route.problem = Some(format!("no site is named {name}")),
                    Some(project) => {
                        route.site = Some(project.name.clone());
                        let table = route_of(&project);
                        let (port, target) = match table.destination(sub) {
                            Destination::Site => (project.port, "the site".to_string()),
                            Destination::Sub(rule) => {
                                route.rule = Some(rule.pattern.clone());
                                (rule.port, rule.label.clone())
                            }
                            Destination::NotConfigured => {
                                route.problem =
                                    Some("the wildcard is off and no rule matches".into());
                                (None, "nothing".to_string())
                            }
                        };
                        route.target = target;
                        route.port = port;
                        if route.problem.is_none() && port.is_none() {
                            route.problem = Some("no port yet".into());
                        }
                        if let Some(port) = port {
                            let probe = tokio::net::TcpStream::connect(("127.0.0.1", port));
                            route.reachable = Some(matches!(
                                tokio::time::timeout(std::time::Duration::from_millis(500), probe)
                                    .await,
                                Ok(Ok(_))
                            ));
                        }
                    }
                }
            }
            locdev_core::host::Target::Index => route.target = "LocDev's index page".into(),
            locdev_core::host::Target::Unknown => {
                route.problem = Some(format!("not a .{} or .localhost name", locdev_core::TLD));
            }
        }

        let tls = self.tls_status();
        HostExplain {
            tls: HostTls {
                names: locdev_tls::names_for(&host),
                https_port: tls.port,
                trusted: tls.trusted,
            },
            host,
            dns,
            route,
        }
    }

    async fn project(&self, name: &str) -> Result<Project, AppError> {
        self.registry.lock().await.get(name).cloned()
    }

    /// Stops everything of the site and forgets it (its files stay).
    pub(super) async fn remove_site(&self, name: &str) -> Result<(), AppError> {
        let removed = self.project(name).await?;
        // Stop without holding the registry lock: it can take seconds.
        self.tunnel_stop(name).await;
        self.stop_all_of(name).await;
        let mut registry = self.registry.lock().await;
        registry.remove(name)?;
        self.commit(&registry)?;
        self.supervisor.forget(name);
        self.env_forget(name);
        self.park_forget_site(&removed);
        info!(project = %name, "project removed");
        Ok(())
    }

    async fn add(
        &self,
        path: PathBuf,
        name: Option<String>,
        command: Option<String>,
        port: Option<u16>,
        auto_start: bool,
    ) -> Result<Response, AppError> {
        if !path.is_absolute() {
            return Err(AppError::invalid("project path must be absolute"));
        }
        let path = path
            .canonicalize()
            .map(locdev_core::paths::plain)
            .map_err(|e| AppError::invalid(format!("cannot open {}: {e}", path.display())))?;
        if !path.is_dir() {
            return Err(AppError::invalid(format!(
                "{} is not a folder",
                path.display()
            )));
        }

        let mut registry = self.registry.lock().await;
        let project = self.new_project(&registry, path, name, command, port, auto_start)?;

        registry.insert(project.clone())?;
        self.commit(&registry)?;
        info!(project = %project.name, path = %project.path.display(), "project added");

        Ok(Response::Project {
            project: self.view(&project),
        })
    }

    /// A project for a canonical folder, as `locdev add` makes it: detected
    /// stack and command, and a free port when there is a command.
    fn new_project(
        &self,
        registry: &Registry,
        path: PathBuf,
        name: Option<String>,
        command: Option<String>,
        port: Option<u16>,
        auto_start: bool,
    ) -> Result<Project, AppError> {
        let detection = locdev_frameworks::detect(&self.catalog(), &path);
        let name = project_name(name.as_deref().unwrap_or(&detection.name))?;
        // A detected helper (Vite next to Laravel) is recorded, waiting for
        // the user's decision; nothing runs until then.
        let dev_server = detection.stack.dev_server.clone();
        let command = command
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .or(detection.command);
        let port = match port {
            Some(port) => {
                validate_port(port)?;
                Some(port)
            }
            None if command.is_some() => Some(self.allocate_port(registry)?),
            None => None,
        };
        Ok(Project {
            domain: project_domain(&name)?,
            name,
            path,
            kind: detection.kind,
            command,
            port,
            auto_start,
            dev_server,
            ..Default::default()
        })
    }

    /// Picks the first port in the configured range that no project claims
    /// and that nothing else is currently bound to.
    fn allocate_port(&self, registry: &Registry) -> Result<u16, AppError> {
        let taken: Vec<u16> = registry.ports_in_use().collect();
        self.free_port(&taken)
    }

    fn free_port(&self, taken: &[u16]) -> Result<u16, AppError> {
        let range = self.config.ports;

        (range.start..=range.end)
            .filter(|port| !taken.contains(port))
            .find(|port| TcpListener::bind((Ipv4Addr::LOCALHOST, *port)).is_ok())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::Config,
                    format!("no free port left in {}..{}", range.start, range.end),
                )
            })
    }

    fn proxied_url(&self, project: &Project, scheme: &str, port: Option<u16>) -> Option<String> {
        project.port?;
        let port = port?;
        let default = if scheme == "https" { 443 } else { 80 };
        let port = if port == default {
            String::new()
        } else {
            format!(":{port}")
        };
        Some(format!(
            "{scheme}://{}.{}{port}",
            project.name,
            self.suffix()
        ))
    }

    fn view(&self, project: &Project) -> ProjectView {
        let http = self.proxied_url(project, "http", self.proxy.lock().unwrap().port);
        let tls = self.tls_status();
        let https = self.proxied_url(project, "https", tls.port);
        // Prefer HTTPS once browsers will accept it without warnings.
        let url = if tls.trusted && https.is_some() {
            https.clone()
        } else {
            http
        };

        let dev = project.dev_server.as_ref();
        ProjectView {
            process: self.supervisor.info(&ProcessRole::Main.id(&project.name)),
            dev_process: dev.map(|_| self.supervisor.info(&ProcessRole::Dev.id(&project.name))),
            dev_url: dev
                .and_then(|d| d.port)
                .map(|port| format!("http://127.0.0.1:{port}")),
            stack: self.stack_of(project),
            url,
            https_url: https,
            direct_url: project.port.map(|port| format!("http://127.0.0.1:{port}")),
            path_exists: project.path.is_dir(),
            runtimes: self.runtimes.for_project(project),
            requirements: self.runtimes.requirements_of(project),
            project: project.clone(),
        }
    }
}

fn validate_port(port: u16) -> Result<(), AppError> {
    if port == 0 {
        return Err(AppError::invalid("port must be between 1 and 65535"));
    }
    Ok(())
}

fn sync_routes(routes: &Routes, registry: &Registry) {
    routes.replace(registry.all().iter().map(route_of));
}

/// The proxy's view of a site: its port and each subdomain rule's port.
pub(crate) fn route_of(p: &locdev_core::model::Project) -> Route {
    use locdev_core::subdomains::SubdomainTarget;
    let subdomains = p
        .subdomains
        .rules
        .iter()
        .map(|rule| {
            let (port, label) = match &rule.target {
                SubdomainTarget::Site => (p.port, format!("{}'s dev server", p.name)),
                SubdomainTarget::Port { port } => (Some(*port), format!("port {port}")),
                SubdomainTarget::Process { id } => (
                    p.processes.get(id).and_then(|proc| proc.port),
                    format!("the {id} process"),
                ),
            };
            locdev_proxy::SubRoute {
                pattern: rule.pattern.clone(),
                port,
                label,
            }
        })
        .collect();
    Route {
        name: p.name.clone(),
        port: p.port,
        secure: p.secure,
        subdomains,
        exclusive: !p.subdomains.wildcard,
    }
}

/// `""` clears a pin; anything else must be a version like 22 or 8.3.
pub(super) fn parse_pin(kind: RuntimeKind, raw: &str) -> Result<Option<String>, AppError> {
    let pin = raw.trim().trim_start_matches('v');
    if pin.is_empty() {
        return Ok(None);
    }
    validate_pin(kind, pin)?;
    Ok(Some(pin.to_string()))
}
