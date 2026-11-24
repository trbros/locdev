//! Opening projects and files in the user's code editor. The editors come
//! from `locdev_core::editor` (launchers and their documented arguments);
//! this finds the installed ones and starts them, without a shell.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;

use locdev_core::editor::{self, EDITORS, EXTRA_DIRS, EditorInfo, EditorsView};
use locdev_core::protocol::{Request, Response};
use locdev_core::{AppError, ErrorCode};
use tracing::info;

use super::Daemon;
use crate::fsutil::find_program;

#[derive(Default)]
pub(super) struct EditorsState {
    /// Detection looks at the disk; kept until a refresh is asked for.
    found: Mutex<Option<Vec<EditorInfo>>>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

fn expand(place: &str) -> Option<PathBuf> {
    match place.strip_prefix("~/") {
        Some(rest) => home().map(|h| h.join(rest)),
        None => Some(PathBuf::from(place)),
    }
}

fn applies(system: &str) -> bool {
    match system {
        "unix" => cfg!(unix),
        other => other == std::env::consts::OS,
    }
}

/// The installed editors, in catalog order: each launcher on PATH (plus
/// the folders installers use that a daemon's PATH may lack), else its
/// known install location.
fn detect() -> Vec<EditorInfo> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(
        EXTRA_DIRS
            .iter()
            .filter(|(system, _)| applies(system))
            .filter_map(|(_, dir)| expand(dir)),
    );
    let search = std::env::join_paths(&dirs).unwrap_or_default();
    let search = search.to_string_lossy();
    EDITORS
        .iter()
        .filter_map(|def| {
            let program = def
                .programs
                .iter()
                .find_map(|name| find_program(name, &search))
                .or_else(|| {
                    def.places
                        .iter()
                        .filter(|(system, _)| applies(system))
                        .filter_map(|(_, place)| expand(place))
                        .find(|path| path.is_file())
                })?;
            Some(EditorInfo {
                id: def.id.into(),
                name: def.name.into(),
                program,
            })
        })
        .collect()
}

impl Daemon {
    async fn editors(&self, refresh: bool) -> Vec<EditorInfo> {
        if !refresh && let Some(found) = self.editors.found.lock().unwrap().clone() {
            return found;
        }
        let found = tokio::task::spawn_blocking(detect)
            .await
            .unwrap_or_default();
        *self.editors.found.lock().unwrap() = Some(found.clone());
        found
    }

    async fn editors_view(&self, refresh: bool) -> EditorsView {
        let editors = self.editors(refresh).await;
        let preferred = self.settings.get().editor.preferred;
        let default = preferred
            .clone()
            .filter(|p| editors.iter().any(|e| &e.id == p))
            .or_else(|| editors.first().map(|e| e.id.clone()));
        EditorsView {
            editors,
            preferred,
            default,
        }
    }

    /// Starts the editor on a site's folder, or on a file inside it. The
    /// editor runs on its own; LocDev doesn't wait for it.
    async fn editor_open(
        &self,
        site: &str,
        path: Option<&str>,
        line: Option<u32>,
        column: Option<u32>,
        choice: Option<&str>,
    ) -> Result<(), AppError> {
        let project = self.project(site).await?;
        let view = self.editors_view(false).await;
        let id = choice.map(str::to_string).or(view.default).ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                "no code editor found; install one (VS Code, Cursor, Zed…) or put its command on PATH",
            )
        })?;
        let def =
            editor::def(&id).ok_or_else(|| AppError::invalid(format!("unknown editor: {id}")))?;
        let found = view.editors.iter().find(|e| e.id == id).ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                format!("{} isn't installed here", def.name),
            )
        })?;
        let target = match path.filter(|p| !p.is_empty()) {
            Some(relative) => editor::inside(&project.path, relative).ok_or_else(|| {
                AppError::invalid(format!("{relative} isn't a file inside {}", project.domain))
            })?,
            None => project.path.clone(),
        };
        let args = editor::args(def.style, &target, line, column);
        let mut command = locdev_core::process::command(&found.program);
        command
            .args(&args)
            .current_dir(&project.path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|e| {
            AppError::new(
                ErrorCode::Io,
                format!("couldn't start {}: {e}", found.program.display()),
            )
        })?;
        // Launchers hand off to the editor and exit; reap them so they
        // don't linger as zombies.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        info!(editor = %id, target = %target.display(), "opened in editor");
        Ok(())
    }

    pub(super) async fn editor_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::EditorList { refresh } => {
                Ok(Response::Editors(self.editors_view(refresh).await))
            }
            Request::EditorOpen {
                site,
                path,
                line,
                column,
                editor,
            } => {
                self.editor_open(&site, path.as_deref(), line, column, editor.as_deref())
                    .await?;
                Ok(Response::Ok)
            }
            _ => unreachable!("editor_request only gets editor requests"),
        }
    }
}
