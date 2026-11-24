//! Databases and caches: installations, instances supervised like sites
//! (`@svc:<name>`, so they never clash with a site), backups. Data is never
//! deleted without the instance's name typed back.

use std::path::PathBuf;
use std::sync::Mutex;

use locdev_core::deps::JobKind;
use locdev_core::protocol::{ProcessAction, Request, Response};
use locdev_core::services::{
    InstallSource, Instance, InstanceView, ServiceInfo, ServiceInstall, ServiceKind,
};
use locdev_core::{AppError, ErrorCode};
use locdev_services::{Instances, Layout, releases, spec};
use tracing::info;

use super::Daemon;
use crate::jobs::Spec as JobSpec;
use crate::supervisor::Spec;

pub struct ServicesState {
    pub layout: Layout,
    file: PathBuf,
    instances: Mutex<Instances>,
    installs: Mutex<Option<Vec<ServiceInstall>>>,
}

impl ServicesState {
    pub fn load(root: PathBuf, file: PathBuf) -> Result<Self, AppError> {
        let instances = Instances::load(&file).map_err(|e| AppError::new(ErrorCode::Config, e))?;
        Ok(Self {
            layout: Layout::new(root),
            file,
            instances: Mutex::new(instances),
            installs: Mutex::new(None),
        })
    }

    /// Every instance (for the database manager's connection list).
    pub fn list(&self) -> Vec<Instance> {
        self.instances.lock().unwrap().list.clone()
    }

    fn save(&self, instances: &Instances) -> Result<(), AppError> {
        instances
            .save(&self.file)
            .map_err(|e| AppError::new(ErrorCode::Io, e))
    }
}

fn svc_id(name: &str) -> String {
    format!("@svc:{name}")
}

/// Waits (up to 15 s) until nothing listens on `port`: a database may still
/// be flushing to disk after the supervisor saw its shell exit.
async fn port_released(port: u16) {
    for _ in 0..150 {
        if !locdev_services::ready(port) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn confirm_name(name: &str, confirm: Option<&str>, what: &str) -> Result<(), AppError> {
    if confirm == Some(name) {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorCode::Unauthorized,
            format!("{what}: type the instance's name ({name}) to confirm"),
        ))
    }
}

impl Daemon {
    async fn service_installs(&self, refresh: bool) -> Vec<ServiceInstall> {
        if !refresh && let Some(cached) = self.services.installs.lock().unwrap().clone() {
            return cached;
        }
        let layout = self.services.layout.clone();
        let found = tokio::task::spawn_blocking(move || locdev_services::discover(&layout))
            .await
            .unwrap_or_default();
        *self.services.installs.lock().unwrap() = Some(found.clone());
        found
    }

    fn instance_view(&self, instance: &Instance) -> InstanceView {
        let id = svc_id(&instance.name);
        let info = self.supervisor.info(&id);
        let running = info.state == locdev_core::model::ProcessState::Running;
        InstanceView {
            ready: running && locdev_services::ready(instance.port),
            data_dir: self.services.layout.dirs(&instance.name).data,
            connection: spec::connection(instance.kind, instance.port),
            log_file: self.supervisor.log_path(&id).filter(|p| p.is_file()),
            info,
            instance: instance.clone(),
        }
    }

    pub(super) fn instance(&self, name: &str) -> Result<Instance, AppError> {
        self.services
            .instances
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("no service instance named {name}")))
    }

    async fn install_for(&self, instance: &Instance) -> Result<ServiceInstall, AppError> {
        self.service_installs(false)
            .await
            .into_iter()
            .find(|i| {
                i.kind == instance.kind
                    && i.version == instance.version
                    && i.source == instance.source
            })
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "{} {} is no longer installed; install it again or create a new instance",
                        instance.kind.label(),
                        instance.version
                    ),
                )
            })
    }

    /// Stops an instance and waits until its port is free again.
    async fn stop_instance(&self, name: &str) -> Result<(), AppError> {
        let instance = self.instance(name)?;
        self.supervisor.stop(&svc_id(name)).await;
        port_released(instance.port).await;
        Ok(())
    }

    pub(super) async fn start_instance(&self, name: &str) -> Result<(), AppError> {
        let instance = self.instance(name)?;
        let install = self.install_for(&instance).await?;
        let dirs = self.services.layout.dirs(name);
        let command = spec::start_command(instance.kind, &install.bin_dir, &dirs, instance.port)
            .map_err(|e| AppError::new(ErrorCode::NotRunnable, e))?;
        self.supervisor.start(Spec {
            id: svc_id(name),
            command,
            cwd: dirs.root.clone(),
            port: Some(instance.port),
            env: Vec::new(),
            restart_on_failure: true,
            notes: vec![format!(
                "[LocDev] {} {} on 127.0.0.1:{} (data in {})",
                instance.kind.label(),
                instance.version,
                instance.port,
                dirs.data.display()
            )],
        })?;
        info!(instance = name, "service instance started");
        Ok(())
    }

    /// Instances set to start with LocDev.
    pub async fn autostart_instances(&self) {
        let names: Vec<String> = self
            .services
            .instances
            .lock()
            .unwrap()
            .list
            .iter()
            .filter(|i| i.autostart)
            .map(|i| i.name.clone())
            .collect();
        for name in names {
            if let Err(error) = self.start_instance(&name).await {
                tracing::warn!(instance = %name, %error, "service instance did not start");
            }
        }
    }

    async fn services_response(&self, refresh: bool) -> Response {
        let installs = self.service_installs(refresh).await;
        let services = ServiceKind::ALL
            .into_iter()
            .map(|kind| {
                let mut notes = Vec::new();
                if kind.is_cache() {
                    notes.push("Redis and Valkey are alternatives sharing port 6379 by default; each instance gets its own port.".into());
                }
                if kind == ServiceKind::Sqlite {
                    notes.push("A file inside the project (database.sqlite…); nothing to install, start or stop. Back it up by copying the file while the app isn't writing.".into());
                }
                ServiceInfo {
                    kind,
                    name: kind.label().into(),
                    default_port: kind.default_port(),
                    installs: installs.iter().filter(|i| i.kind == kind).cloned().collect(),
                    no_download: releases::unavailable(kind),
                    notes,
                    packages: locdev_services::packages::packages(kind),
                }
            })
            .collect();
        let instances = self
            .services
            .instances
            .lock()
            .unwrap()
            .list
            .iter()
            .map(|i| self.instance_view(i))
            .collect();
        Response::Services {
            services,
            instances,
        }
    }

    /// A port for a new instance: the requested one, else the service's
    /// default when free, else the next free one after it.
    fn instance_port(&self, kind: ServiceKind, wanted: Option<u16>) -> Result<u16, AppError> {
        let taken: Vec<u16> = self
            .services
            .instances
            .lock()
            .unwrap()
            .list
            .iter()
            .map(|i| i.port)
            .collect();
        let free =
            |p: u16| !taken.contains(&p) && std::net::TcpListener::bind(("127.0.0.1", p)).is_ok();
        if let Some(port) = wanted {
            return if free(port) {
                Ok(port)
            } else {
                Err(AppError::new(
                    ErrorCode::Conflict,
                    format!("port {port} is in use"),
                ))
            };
        }
        let start = kind.default_port().unwrap_or(20000);
        (start..start.saturating_add(200))
            .find(|p| free(*p))
            .ok_or_else(|| {
                AppError::new(ErrorCode::Config, "no free port near the service's default")
            })
    }

    /// The download job (`JobKind::ServiceInstall`). Blocking work on a worker.
    pub(super) async fn run_service_install(
        &self,
        input: &serde_json::Value,
    ) -> Result<String, AppError> {
        let kind: ServiceKind = serde_json::from_value(input["kind"].clone())
            .map_err(|_| AppError::invalid("bad service kind"))?;
        let version = input["version"].as_str().unwrap_or_default().to_string();
        let layout = self.services.layout.clone();
        let install = tokio::task::spawn_blocking(move || -> Result<ServiceInstall, String> {
            let release = releases::list(kind)?
                .into_iter()
                .find(|r| r.version == version)
                .ok_or_else(|| format!("{} {version} is not offered for download", kind.label()))?;
            let build = releases::resolve(&release)?;
            locdev_services::install(&layout, &build, &|_, _| {})
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
        self.service_installs(true).await;
        Ok(format!(
            "installed {} {} (SHA-256 verified)",
            install.kind.label(),
            install.version
        ))
    }

    pub(super) async fn services_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::ListServices { refresh } => Ok(self.services_response(refresh).await),

            Request::ServiceReleases { kind } => {
                if self.settings.get().network.offline {
                    return Err(AppError::new(
                        ErrorCode::Remote,
                        "LocDev is in offline mode",
                    ));
                }
                let releases = tokio::task::spawn_blocking(move || releases::list(kind))
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?
                    .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
                Ok(Response::ServiceReleases { releases })
            }

            Request::InstallServicePackage { kind, id } => {
                let package = locdev_services::packages::packages(kind)
                    .into_iter()
                    .find(|p| p.id == id)
                    .ok_or_else(|| {
                        AppError::not_found(format!("no package {id} for {}", kind.label()))
                    })?;
                let home = Self::home_dir();
                let steps: Vec<locdev_core::scaffold::Step> = package
                    .commands
                    .iter()
                    .map(|c| locdev_core::scaffold::Step {
                        label: format!(
                            "{} {} ({})",
                            kind.label(),
                            package.version,
                            package.manager
                        ),
                        program: c[0].clone(),
                        args: c[1..].to_vec(),
                        cwd: home.clone(),
                        env: Vec::new(),
                    })
                    .collect();
                let me = self.me()?;
                let session = self.terminals.open(
                    format!("Install {} {}", kind.label(), package.version),
                    steps.len(),
                )?;
                let info = session.info();
                let env = self.scaffold_env(&home);
                tokio::spawn(async move {
                    let code = me.run_steps(&session, &steps, 0, &env).await;
                    session.finish(code);
                    // The new install shows up on the next refresh.
                    me.services_response(true).await;
                });
                Ok(Response::Terminal(info))
            }

            Request::InstallService { kind, version } => {
                if let Some(why) = releases::unavailable(kind) {
                    return Err(AppError::new(ErrorCode::NotRunnable, why));
                }
                let (job, _) = self.jobs.enqueue(JobSpec {
                    kind: JobKind::ServiceInstall,
                    project: None,
                    input: serde_json::json!({ "kind": kind, "version": version }),
                    dedup: format!("service-install:{}:{version}", kind.id()),
                    automatic: false,
                    max_attempts: 2,
                    not_before: None,
                });
                self.job_wake.notify_one();
                Ok(Response::Job { job })
            }

            Request::UninstallService { kind, version } => {
                let install = self
                    .service_installs(false)
                    .await
                    .into_iter()
                    .find(|i| i.kind == kind && i.version == version && i.source == InstallSource::Managed)
                    .ok_or_else(|| {
                        AppError::not_found(format!(
                            "{} {version} wasn't installed by LocDev (system installs are never removed)",
                            kind.label()
                        ))
                    })?;
                let users: Vec<String> = self
                    .services
                    .instances
                    .lock()
                    .unwrap()
                    .list
                    .iter()
                    .filter(|i| {
                        i.kind == kind && i.version == version && i.source == InstallSource::Managed
                    })
                    .map(|i| i.name.clone())
                    .collect();
                if !users.is_empty() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        format!("used by {}; remove those instances first", users.join(", ")),
                    ));
                }
                let dir = install
                    .bin_dir
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or(install.bin_dir);
                std::fs::remove_dir_all(&dir)?;
                self.service_installs(true).await;
                Ok(self.services_response(false).await)
            }

            Request::CreateInstance {
                name,
                kind,
                version,
                port,
                autostart,
            } => {
                locdev_services::check_instance_name(&name).map_err(AppError::invalid)?;
                if kind == ServiceKind::Sqlite {
                    return Err(AppError::invalid(
                        "SQLite is a file inside the project; there is no instance to create",
                    ));
                }
                if self.services.instances.lock().unwrap().get(&name).is_some() {
                    return Err(AppError::new(
                        ErrorCode::AlreadyExists,
                        format!("an instance named {name} exists"),
                    ));
                }
                let mut candidates: Vec<ServiceInstall> = self
                    .service_installs(false)
                    .await
                    .into_iter()
                    .filter(|i| {
                        i.kind == kind
                            && version.as_deref().is_none_or(|v| {
                                i.version == v || i.version.starts_with(&format!("{v}."))
                            })
                    })
                    .collect();
                candidates.sort_by_key(|i| {
                    std::cmp::Reverse(
                        i.version
                            .split('.')
                            .map(|p| p.parse::<u64>().unwrap_or(0))
                            .collect::<Vec<_>>(),
                    )
                });
                let install = candidates.into_iter().next().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::NotRunnable,
                        format!("no {} installed; `locdev service install {}` or install it on the system", kind.label(), kind.id()),
                    )
                })?;
                let port = self.instance_port(kind, port)?;
                let dirs = self.services.layout.dirs(&name);
                let bin = install.bin_dir.clone();
                tokio::task::spawn_blocking(move || locdev_services::init(kind, &bin, &dirs, port))
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))?
                    .map_err(|e| AppError::new(ErrorCode::NotRunnable, e))?;
                let instance = Instance {
                    name: name.clone(),
                    kind,
                    version: install.version.clone(),
                    source: install.source,
                    port,
                    autostart,
                    created_at: locdev_core::model::now_ms(),
                };
                {
                    let mut instances = self.services.instances.lock().unwrap();
                    instances.list.push(instance.clone());
                    self.services.save(&instances)?;
                }
                info!(instance = %name, kind = kind.id(), port, "service instance created");
                if autostart {
                    self.start_instance(&name).await?;
                }
                Ok(Response::Instance(self.instance_view(&instance)))
            }

            Request::ControlInstance { name, action } => {
                let instance = self.instance(&name)?;
                match action {
                    ProcessAction::Start => self.start_instance(&name).await?,
                    ProcessAction::Stop => self.stop_instance(&name).await?,
                    ProcessAction::Restart => {
                        self.stop_instance(&name).await?;
                        self.start_instance(&name).await?;
                    }
                }
                Ok(Response::Instance(self.instance_view(&instance)))
            }

            Request::SetInstance { name, autostart } => {
                let mut instances = self.services.instances.lock().unwrap();
                let instance = instances
                    .list
                    .iter_mut()
                    .find(|i| i.name == name)
                    .ok_or_else(|| {
                        AppError::not_found(format!("no service instance named {name}"))
                    })?;
                instance.autostart = autostart;
                let instance = instance.clone();
                self.services.save(&instances)?;
                drop(instances);
                Ok(Response::Instance(self.instance_view(&instance)))
            }

            Request::RemoveInstance {
                name,
                delete_data,
                confirm,
            } => {
                self.instance(&name)?;
                if delete_data {
                    confirm_name(
                        &name,
                        confirm.as_deref(),
                        "deleting the data can't be undone",
                    )?;
                }
                self.supervisor.stop(&svc_id(&name)).await;
                {
                    let mut instances = self.services.instances.lock().unwrap();
                    instances.list.retain(|i| i.name != name);
                    self.services.save(&instances)?;
                }
                if delete_data {
                    std::fs::remove_dir_all(self.services.layout.dirs(&name).root)?;
                    info!(instance = %name, "service instance removed with its data");
                } else {
                    info!(instance = %name, "service instance removed; data kept");
                }
                Ok(self.services_response(false).await)
            }

            Request::BackupInstance { name } => {
                self.instance(&name)?;
                let id = svc_id(&name);
                let was_running = self.supervisor.is_active(&id);
                if was_running {
                    self.stop_instance(&name).await?;
                }
                let layout = self.services.layout.clone();
                let target = name.clone();
                let result =
                    tokio::task::spawn_blocking(move || locdev_services::backup(&layout, &target))
                        .await
                        .map_err(|e| AppError::internal(e.to_string()))?;
                if was_running {
                    self.start_instance(&name).await?;
                }
                let backup = result.map_err(|e| AppError::new(ErrorCode::Io, e))?;
                Ok(Response::InstanceBackup(backup))
            }

            Request::InstanceBackups { name } => {
                self.instance(&name)?;
                Ok(Response::InstanceBackups {
                    backups: locdev_services::backups(&self.services.layout, &name),
                })
            }

            Request::RestoreInstance { name, id, confirm } => {
                let instance = self.instance(&name)?;
                confirm_name(
                    &name,
                    confirm.as_deref(),
                    "restoring replaces the current data",
                )?;
                let sid = svc_id(&name);
                let was_running = self.supervisor.is_active(&sid);
                self.stop_instance(&name).await?;
                let layout = self.services.layout.clone();
                let target = name.clone();
                let aside = tokio::task::spawn_blocking(move || {
                    locdev_services::restore(&layout, &target, &id)
                })
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
                .map_err(|e| AppError::new(ErrorCode::Io, e))?;
                info!(instance = %name, aside = %aside.display(), "service instance restored");
                if was_running {
                    self.start_instance(&name).await?;
                }
                Ok(Response::Instance(self.instance_view(&instance)))
            }

            Request::InstanceLogs { name, after, limit } => {
                self.instance(&name)?;
                Ok(Response::Logs {
                    lines: self
                        .supervisor
                        .logs(&svc_id(&name), after, limit.unwrap_or(500)),
                })
            }

            other => Err(AppError::internal(format!(
                "not a services request: {other:?}"
            ))),
        }
    }
}
