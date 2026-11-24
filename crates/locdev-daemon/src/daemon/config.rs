//! Config Files: the daemon reads and writes a site's config files through
//! `locdev-config` (parsers only, never the project's code). Saves name the
//! hash the user reviewed; the previous content is kept under
//! `data/state/config-history` before anything is written.

use std::path::PathBuf;

use locdev_config::history::History;
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use tracing::info;

use super::Daemon;

fn error(message: String) -> AppError {
    let code = if message.contains("changed since") {
        ErrorCode::Conflict
    } else if message.contains("isn't a config file") || message.starts_with("no saved version") {
        ErrorCode::NotFound
    } else {
        ErrorCode::InvalidInput
    };
    AppError::new(code, message)
}

/// Parsing and lockfile reads are blocking work; keep them off the runtime.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
        .map_err(error)
}

impl Daemon {
    fn config_history_base(&self) -> PathBuf {
        self.paths.data_dir.join("state").join("config-history")
    }

    pub(super) async fn config_request(&self, request: Request) -> Result<Response, AppError> {
        let base = self.config_history_base();
        match request {
            Request::ConfigFiles { name } => {
                let root = self.project(&name).await?.path;
                let files = blocking(move || Ok(locdev_config::list(&root))).await?;
                Ok(Response::ConfigFiles { files })
            }
            Request::ConfigFile { name, path } => {
                let root = self.project(&name).await?.path;
                Ok(Response::ConfigFile(
                    blocking(move || locdev_config::open(&root, &path)).await?,
                ))
            }
            Request::ConfigPreview { name, path, change } => {
                let root = self.project(&name).await?.path;
                Ok(Response::ConfigPreview(
                    blocking(move || locdev_config::preview(&root, &path, &change)).await?,
                ))
            }
            Request::ConfigSave {
                name,
                path,
                change,
                base_hash,
            } => {
                let root = self.project(&name).await?.path;
                let history = History::new(&base, &name, &path);
                let shown = path.clone();
                let hash = blocking(move || {
                    locdev_config::save(&root, &path, &change, &base_hash, &history)
                })
                .await?;
                info!(site = %name, file = %shown, "config file saved");
                Ok(Response::ConfigSaved { hash })
            }
            Request::ConfigHistory { name, path } => {
                let root = self.project(&name).await?.path;
                let history = History::new(&base, &name, &path);
                let entries = blocking(move || {
                    locdev_config::files::resolve(&root, &path)?;
                    Ok(history.list())
                })
                .await?;
                Ok(Response::ConfigHistory { entries })
            }
            Request::ConfigHistoryVersion { name, path, id } => {
                let root = self.project(&name).await?.path;
                let history = History::new(&base, &name, &path);
                let content = blocking(move || {
                    locdev_config::files::resolve(&root, &path)?;
                    history.load(&id)
                })
                .await?;
                Ok(Response::ConfigHistoryVersion { content })
            }
            Request::ConfigRevert {
                name,
                path,
                id,
                base_hash,
            } => {
                let root = self.project(&name).await?.path;
                let history = History::new(&base, &name, &path);
                let shown = path.clone();
                let hash = blocking(move || {
                    locdev_config::revert(&root, &path, &id, &base_hash, &history)
                })
                .await?;
                info!(site = %name, file = %shown, "config file reverted");
                Ok(Response::ConfigSaved { hash })
            }
            Request::ConfigCheck { name } => {
                let root = self.project(&name).await?.path;
                let findings = blocking(move || Ok(locdev_config::rules::check(&root))).await?;
                Ok(Response::ConfigFindings { findings })
            }
            _ => unreachable!("not a config request"),
        }
    }
}
