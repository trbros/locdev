//! Deploy assistant: the daemon gathers what it already knows about a site
//! (stack, database connections, the auxiliary processes turned on) and
//! `locdev-deploy` turns it into recommendations. Nothing is sent to any
//! platform; a config file is written only as reviewed, never over another.

use locdev_core::model::{DevServerMode, ProcessKind, Project};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use tracing::info;

use super::Daemon;

struct Owned {
    project: Project,
    stack: locdev_core::model::Stack,
    databases: Vec<(locdev_core::db::DbEngine, bool)>,
    processes: Vec<ProcessKind>,
}

impl Owned {
    fn inputs(&self) -> locdev_deploy::Inputs<'_> {
        locdev_deploy::Inputs {
            root: &self.project.path,
            name: &self.project.name,
            stack: &self.stack,
            databases: self.databases.clone(),
            processes: self.processes.clone(),
        }
    }
}

fn error(message: String) -> AppError {
    let code = if message.contains("already exists") || message.contains("review it again") {
        ErrorCode::Conflict
    } else if message.starts_with("no platform") {
        ErrorCode::NotFound
    } else {
        ErrorCode::InvalidInput
    };
    AppError::new(code, message)
}

impl Daemon {
    async fn deploy_inputs(&self, name: &str) -> Result<Owned, AppError> {
        let project = self.project(name).await?;
        let stack = self.stack_of(&project);
        let databases = self
            .site_connections(&project)
            .into_iter()
            .map(|r| (r.public.engine, r.public.remote))
            .collect();
        let processes = stack
            .processes
            .iter()
            .filter(|offer| {
                project
                    .processes
                    .get(&offer.id)
                    .is_some_and(|p| matches!(p.mode, DevServerMode::Auto | DevServerMode::Manual))
            })
            .map(|offer| offer.kind)
            .collect();
        Ok(Owned {
            project,
            stack,
            databases,
            processes,
        })
    }

    pub(super) async fn deploy_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::DeployAnalysis { name } => {
                let owned = self.deploy_inputs(&name).await?;
                let report =
                    tokio::task::spawn_blocking(move || locdev_deploy::analyze(&owned.inputs()))
                        .await
                        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?;
                Ok(Response::DeployReport(report))
            }
            Request::DeployConfig { name, provider } => {
                let owned = self.deploy_inputs(&name).await?;
                let config = tokio::task::spawn_blocking(move || {
                    locdev_deploy::config(&owned.inputs(), &provider)
                })
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
                .map_err(error)?;
                Ok(Response::DeployConfig(config))
            }
            Request::DeployWriteConfig {
                name,
                provider,
                hash,
            } => {
                let owned = self.deploy_inputs(&name).await?;
                let site = name.clone();
                let file = tokio::task::spawn_blocking(move || {
                    locdev_deploy::write(&owned.inputs(), &provider, &hash)
                })
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
                .map_err(error)?;
                info!(site = %site, %file, "deploy config written");
                Ok(Response::DeployConfigWritten { file })
            }
            _ => unreachable!("not a deploy request"),
        }
    }
}
