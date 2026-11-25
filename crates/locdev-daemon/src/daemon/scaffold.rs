//! New projects and the tools they need. The plan is computed by
//! `locdev-scaffold` and recomputed here when the user confirms, so what
//! runs is what was shown. Steps run in a terminal (see `terminal.rs`) with
//! LocDev's runtimes first on PATH; between them LocDev creates the database
//! instance and the database the user chose, and registers the site.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use locdev_core::model::{Project, RuntimeKind};
use locdev_core::protocol::{Request, Response};
use locdev_core::scaffold::{NewProject, ProjectPlan, Step, ToolInstall};
use locdev_core::services::ServiceKind;
use locdev_core::{AppError, ErrorCode};
use tracing::{info, warn};

use super::Daemon;

impl Daemon {
    /// PATH and the rest of the environment for scaffold steps: LocDev's
    /// runtimes first (the versions chosen for the new project, else the
    /// defaults, as a site with no pins would get), then the folders
    /// installers land in, then the system PATH.
    pub(super) fn scaffold_env(&self, parent: &Path) -> Vec<(String, String)> {
        self.scaffold_env_with(parent, None, None)
            .unwrap_or_default()
    }

    /// Fails when a chosen version isn't installed.
    fn scaffold_env_with(
        &self,
        parent: &Path,
        node: Option<&str>,
        php: Option<&str>,
    ) -> Result<Vec<(String, String)>, AppError> {
        let probe = Project {
            name: "new".into(),
            path: parent.to_path_buf(),
            node: node.map(str::to_string),
            php: php.map(str::to_string),
            ..Default::default()
        };
        let mut env = if node.is_some() || php.is_some() {
            self.runtimes.env_for(&probe)?
        } else {
            self.runtimes.env_for(&probe).unwrap_or_default()
        };
        let home = Self::home_dir();
        let base = env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
        // Installers' folders go last: they only add what PATH lacks, and
        // never shadow the runtimes LocDev put first (a pnpm or node in
        // ~/.local/bin was taken over the project's own).
        let mut dirs: Vec<PathBuf> = std::env::split_paths(&base).collect();
        for extra in locdev_scaffold::tools::extra_dirs(&home) {
            if extra.is_dir() && !dirs.contains(&extra) {
                dirs.push(extra);
            }
        }
        let path = std::env::join_paths(dirs)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or(base);
        env.retain(|(k, _)| k != "PATH");
        env.push(("PATH".into(), path));
        Ok(env)
    }

    /// The project's runtime choices, checked (format and installed).
    fn scaffold_pins(&self, project: &mut NewProject) -> Result<Vec<String>, AppError> {
        project.node = match project.node.as_deref() {
            Some(raw) => super::parse_pin(RuntimeKind::Node, raw)?,
            None => None,
        };
        project.php = match project.php.as_deref() {
            Some(raw) => super::parse_pin(RuntimeKind::Php, raw)?,
            None => None,
        };
        // One at a time, so the error names the version that's missing.
        for (kind, pin) in [
            (RuntimeKind::Node, &project.node),
            (RuntimeKind::Php, &project.php),
        ] {
            let Some(pin) = pin else { continue };
            let (node, php) = match kind {
                RuntimeKind::Node => (Some(pin.as_str()), None),
                _ => (None, Some(pin.as_str())),
            };
            if self.scaffold_env_with(&project.parent, node, php).is_err() {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "{} {pin} is not installed; install it with `locdev runtime install {} {pin}` or pick another version",
                        kind.label(),
                        kind.executable()
                    ),
                ));
            }
        }
        let mut notes = Vec::new();
        for (label, pin) in [("Node.js", &project.node), ("PHP", &project.php)] {
            if let Some(pin) = pin {
                notes.push(format!(
                    "Created with {label} {pin}, and the site is pinned to it."
                ));
            }
        }
        Ok(notes)
    }

    pub(super) fn home_dir() -> PathBuf {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Where the app creates new projects by default: `~/LocDev/Sites`
    /// (LocDev's own folder, clear of the `~/Sites` macOS already has).
    pub(super) fn sites_dir() -> PathBuf {
        Self::home_dir().join("LocDev").join("Sites")
    }

    async fn scaffold_tools(&self) -> Vec<locdev_core::scaffold::Tool> {
        let path = self
            .scaffold_env(&Self::home_dir())
            .into_iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v)
            .unwrap_or_default();
        let home = Self::home_dir();
        tokio::task::spawn_blocking(move || {
            locdev_scaffold::tools(&path, &home, &locdev_scaffold::starters())
        })
        .await
        .unwrap_or_default()
    }

    /// The plan, with the tools it needs that aren't installed.
    async fn scaffold_plan(&self, project: &mut NewProject) -> Result<ProjectPlan, AppError> {
        // The default folder is made on first use; any other must exist.
        if project.parent == Self::sites_dir() && !project.parent.is_dir() {
            std::fs::create_dir_all(&project.parent).map_err(|e| {
                AppError::new(
                    ErrorCode::Io,
                    format!("could not create {}: {e}", project.parent.display()),
                )
            })?;
        }
        let mut plan = locdev_scaffold::plan(project).map_err(AppError::invalid)?;
        plan.notes.extend(self.scaffold_pins(project)?);
        let tools = self.scaffold_tools().await;
        let mut missing = Vec::new();
        for step in plan.steps.iter().chain(&plan.finish) {
            if let Some(id) = locdev_scaffold::plan::tool_of(&step.program)
                && let Some(tool) = tools.iter().find(|t| t.id == id)
                && !tool.installed
                && !missing.contains(&tool.name)
            {
                missing.push(tool.name.clone());
            }
        }
        plan.missing = missing;
        Ok(plan)
    }

    pub(super) async fn scaffold_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::ListStarters => Ok(Response::Starters {
                starters: locdev_scaffold::starters(),
                sites_dir: Some(Self::sites_dir()),
            }),
            Request::ListTools => Ok(Response::Tools {
                tools: self.scaffold_tools().await,
            }),
            Request::PlanProject { mut project } => Ok(Response::ProjectPlan(
                self.scaffold_plan(&mut project).await?,
            )),
            Request::CreateProject { mut project } => {
                let plan = self.scaffold_plan(&mut project).await?;
                if !plan.missing.is_empty() {
                    return Err(AppError::new(
                        ErrorCode::NotRunnable,
                        format!("install {} first", plan.missing.join(", ")),
                    ));
                }
                let me = self.me()?;
                let steps = plan.steps.len() + plan.finish.len();
                let session = self.terminals.open(
                    format!("New {} project: {}", project.starter, project.name),
                    steps,
                )?;
                let info = session.info();
                info!(starter = %project.starter, name = %project.name, terminal = info.id, "creating a project");
                tokio::spawn(async move { me.create_project(session, project, plan).await });
                Ok(Response::Terminal(info))
            }
            Request::InstallTool { id } => {
                let tools = self.scaffold_tools().await;
                let tool = tools
                    .iter()
                    .find(|t| t.id == id)
                    .ok_or_else(|| AppError::not_found(format!("no tool {id}")))?;
                let steps = match &tool.install {
                    Some(ToolInstall::Command { .. }) => {
                        locdev_scaffold::tools::install_steps(&id, &Self::home_dir())
                            .unwrap_or_default()
                    }
                    Some(ToolInstall::Runtime { runtime }) => {
                        return Err(AppError::invalid(format!(
                            "{} is installed from Runtimes (`locdev runtime install {runtime} …`)",
                            tool.name
                        )));
                    }
                    None => {
                        return Err(AppError::invalid(format!(
                            "LocDev can't install {}; see {}",
                            tool.name, tool.docs
                        )));
                    }
                };
                let me = self.me()?;
                let session = self
                    .terminals
                    .open(format!("Install {}", tool.name), steps.len())?;
                let info = session.info();
                let env = self.scaffold_env(&Self::home_dir());
                tokio::spawn(async move {
                    let code = me.run_steps(&session, &steps, 0, &env).await;
                    session.finish(code);
                });
                Ok(Response::Terminal(info))
            }
            Request::SearchPackages { registry, query } => {
                if self.settings.get().network.offline {
                    return Err(AppError::new(ErrorCode::Remote, "offline mode is on"));
                }
                self.limiter
                    .wait(self.settings.get().automation.requests_per_second)
                    .await;
                let packages = tokio::task::spawn_blocking(move || {
                    locdev_scaffold::search::search(registry, &query)
                })
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
                .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
                Ok(Response::Packages { packages })
            }
            Request::ListTerminals => Ok(Response::Terminals {
                terminals: self.terminals.list(),
            }),
            Request::TerminalRead { id, from } => {
                Ok(Response::TerminalOutput(self.terminals.read(id, from)?))
            }
            Request::TerminalWrite { id, data } => {
                self.terminals.write(id, &data)?;
                Ok(Response::Ok)
            }
            Request::TerminalResize { id, cols, rows } => {
                self.terminals.resize(id, cols, rows)?;
                Ok(Response::Ok)
            }
            Request::TerminalKill { id } => {
                self.terminals.kill(id)?;
                Ok(Response::Ok)
            }
            Request::TerminalClose { id } => {
                self.terminals.close(id)?;
                Ok(Response::Ok)
            }
            other => Err(AppError::internal(format!(
                "not a scaffold request: {other:?}"
            ))),
        }
    }

    /// Runs `steps` in order; the exit code of the first that fails, or 0.
    /// `None` when stopped or a program couldn't start.
    pub(super) async fn run_steps(
        &self,
        session: &Arc<super::terminal::Session>,
        steps: &[Step],
        offset: usize,
        env: &[(String, String)],
    ) -> Option<i32> {
        for (i, step) in steps.iter().enumerate() {
            match session.run(offset + i + 1, step, env).await {
                Ok(0) => {}
                Ok(code) => return Some(code as i32),
                Err(error) => {
                    if !session.killed() {
                        session.note(&error.message);
                    }
                    return None;
                }
            }
            if session.killed() {
                return None;
            }
        }
        Some(0)
    }

    async fn create_project(
        self: Arc<Self>,
        session: Arc<super::terminal::Session>,
        project: NewProject,
        plan: ProjectPlan,
    ) {
        let env = match self.scaffold_env_with(
            &project.parent,
            project.node.as_deref(),
            project.php.as_deref(),
        ) {
            Ok(env) => env,
            Err(error) => {
                session.note(&error.message);
                session.finish(Some(1));
                return;
            }
        };
        if plan.make_dir || !plan.files.is_empty() {
            match write_starter(&plan) {
                Ok(written) => {
                    for path in written {
                        session.note(&format!("Wrote {path}"));
                    }
                }
                Err(error) => {
                    session.note(&format!("Could not write the starter files: {error}"));
                    session.finish(Some(1));
                    return;
                }
            }
        }
        let code = self.run_steps(&session, &plan.steps, 0, &env).await;
        if code != Some(0) {
            session.finish(code);
            return;
        }
        match locdev_scaffold::plan::after_steps(&project.starter, &project.name, &plan.dir) {
            Ok(done) => {
                for line in done {
                    session.note(&line);
                }
            }
            Err(error) => {
                session.note(&format!("Could not finish the project's settings: {error}"));
                session.finish(Some(1));
                return;
            }
        }
        if let Some(db) = &project.database {
            if let Err(error) = self
                .scaffold_database(&session, &project, db, &plan.dir)
                .await
            {
                session.note(&format!("Database: {}", error.message));
                session.finish(Some(1));
                return;
            }
            let code = self
                .run_steps(&session, &plan.finish, plan.steps.len(), &env)
                .await;
            if code != Some(0) {
                session.finish(code);
                return;
            }
        }
        if project.register {
            session.note(&format!("Registering {} as a site…", project.name));
            match self
                .add(
                    plan.dir.clone(),
                    Some(project.name.clone()),
                    None,
                    None,
                    false,
                )
                .await
            {
                Ok(_) => {
                    if project.node.is_some() || project.php.is_some() {
                        self.scaffold_pin_site(&project).await;
                    }
                    session.set_site(project.name.clone());
                }
                Err(error) => {
                    warn!(%error, "could not register the new project");
                    session.note(&format!("Could not register the site: {}", error.message));
                }
            }
        }
        session.finish(Some(0));
    }

    /// Pins the new site to the versions it was created with.
    async fn scaffold_pin_site(&self, project: &NewProject) {
        let mut registry = self.registry.lock().await;
        let Ok(site) = registry.get_mut(&project.name) else {
            return;
        };
        site.node.clone_from(&project.node);
        site.php.clone_from(&project.php);
        if let Err(error) = self.commit(&registry) {
            warn!(%error, "could not pin the new site's runtimes");
        }
    }

    /// The instance (created and started if needed), the database, and the
    /// project's `.env` pointed at them.
    async fn scaffold_database(
        &self,
        session: &super::terminal::Session,
        project: &NewProject,
        db: &locdev_core::scaffold::DatabaseChoice,
        dir: &Path,
    ) -> Result<(), AppError> {
        let kind = match db.service.as_str() {
            "postgres" => ServiceKind::Postgres,
            "mysql" => ServiceKind::Mysql,
            "mariadb" => ServiceKind::Mariadb,
            other => {
                return Err(AppError::invalid(format!(
                    "'{other}' is not a database LocDev runs"
                )));
            }
        };
        let exists = self.instance(&db.instance).is_ok();
        if !exists {
            session.note(&format!(
                "Creating the {} instance {}…",
                kind.label(),
                db.instance
            ));
            self.services_request(Request::CreateInstance {
                name: db.instance.clone(),
                kind,
                version: None,
                port: None,
                autostart: false,
            })
            .await?;
        }
        let instance = self.instance(&db.instance)?;
        if instance.kind != kind {
            return Err(AppError::invalid(format!(
                "{} is a {} instance, not {}",
                db.instance,
                instance.kind.label(),
                kind.label()
            )));
        }
        if !locdev_services::ready(instance.port) {
            session.note(&format!("Starting {}…", db.instance));
            self.start_instance(&db.instance).await?;
        }
        // Accepting connections, not just a PID (first start initializes the data folder).
        let mut waited = 0;
        while !locdev_services::ready(instance.port) {
            if waited >= 120 {
                return Err(AppError::new(
                    ErrorCode::NotRunnable,
                    format!(
                        "{} didn't start accepting connections; see `locdev service logs {}`",
                        db.instance, db.instance
                    ),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            waited += 1;
        }
        let database = project.name.replace('-', "_");
        session.note(&format!("Creating the database {database}…"));
        // The port answers a moment before the server takes queries; an
        // existing database is fine (a second try after a failed step).
        let mut attempt = 0;
        loop {
            let created = self
                .db_request(Request::DbCreateDatabase {
                    connection: format!("instance:{}", db.instance),
                    database: Some(database.clone()),
                })
                .await;
            match created {
                Ok(_) => break,
                Err(error) if error.message.contains("exists") => break,
                Err(_) if attempt < 10 => {
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                Err(error) => return Err(error),
            }
        }
        let mut pairs: Vec<(String, String)> =
            locdev_services::spec::connection(kind, instance.port)
                .into_iter()
                .filter(|(k, _)| k.starts_with("DB_"))
                .collect();
        pairs.push(("DB_DATABASE".into(), database));
        let env_file = dir.join(".env");
        let content = std::fs::read_to_string(&env_file).unwrap_or_default();
        crate::fsutil::write_atomic(
            &env_file,
            locdev_scaffold::plan::set_env(&content, &pairs).as_bytes(),
        )?;
        session.note(&format!(
            "Set {} in .env.",
            pairs
                .iter()
                .map(|(k, _)| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        Ok(())
    }
}

/// Makes the new folder and writes the starter's files into it: never over
/// an existing folder or file, never outside the folder.
fn write_starter(plan: &ProjectPlan) -> std::io::Result<Vec<String>> {
    use std::io::Write;
    use std::path::Component;

    std::fs::create_dir(&plan.dir)?;
    let mut written = Vec::new();
    for file in &plan.files {
        let rel = Path::new(&file.path);
        if rel.as_os_str().is_empty()
            || !rel.components().all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(std::io::Error::other(format!(
                "{} is not a path inside the project",
                file.path
            )));
        }
        let path = plan.dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(file.contents.as_bytes())?;
        written.push(file.path.clone());
    }
    Ok(written)
}
