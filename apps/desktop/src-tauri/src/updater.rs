//! The app's own updates, through the Tauri updater: a `latest.json` from
//! LocDev's GitHub releases whose files are signed with the key in
//! tauri.conf.json (the plugin refuses anything else). The UI only names a
//! release tag; the address is built here.

use std::sync::Mutex;

use locdev_core::paths::Paths;
use locdev_core::update::valid_tag;
use locdev_ipc::launcher;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Url};
use tauri_plugin_updater::{Updater, UpdaterExt};
use time::format_description::well_known::Rfc3339;

use crate::CommandError;

/// Must match the endpoint in tauri.conf.json.
pub(crate) const RELEASES: &str = "https://github.com/trbros/locdev/releases";

/// One install at a time.
static INSTALLING: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Serialize)]
pub struct AppUpdate {
    /// This app's version.
    current: String,
    /// The daemon/CLI inside this app (they ship together).
    bundled_daemon: &'static str,
    /// The offered version; `None` = nothing newer (or that release has no
    /// updater files).
    version: Option<String>,
    date: Option<String>,
    notes: Option<String>,
    /// Going back to an earlier release.
    downgrade: bool,
    /// Whether this copy can replace itself; `reason` says why not.
    can_install: bool,
    /// `development_build` or `linux_package` (the UI words it).
    reason: Option<&'static str>,
    /// The release page, to download by hand.
    page: String,
}

#[derive(Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
    done: bool,
}

fn other(message: impl ToString) -> CommandError {
    CommandError::Other {
        message: message.to_string(),
    }
}

/// Why this copy can't update itself, if it can't.
fn blocker() -> Option<&'static str> {
    if cfg!(debug_assertions) {
        return Some("development_build");
    }
    // The updater replaces AppImages only; packages come from the release.
    if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_none() {
        return Some("linux_package");
    }
    None
}

fn blocked(reason: &str) -> CommandError {
    other(match reason {
        "development_build" => "this is a development build; updates are for installed copies",
        _ => {
            "installed from a .deb or .rpm package: download the new package from the release page"
        }
    })
}

/// The latest release, or exactly `tag` (allowed to be older: a rollback).
fn updater(app: &AppHandle, tag: Option<&str>) -> Result<Updater, CommandError> {
    let mut builder = app.updater_builder();
    if let Some(tag) = tag {
        if !valid_tag(tag) {
            return Err(other(format!("not a release tag: {tag}")));
        }
        let url = Url::parse(&format!("{RELEASES}/download/{tag}/latest.json")).map_err(other)?;
        builder = builder
            .endpoints(vec![url])
            .map_err(other)?
            .version_comparator(|current, update| update.version != current);
    }
    builder.build().map_err(other)
}

fn page(tag: Option<&str>) -> String {
    match tag {
        Some(tag) => format!("{RELEASES}/tag/{tag}"),
        None => format!("{RELEASES}/latest"),
    }
}

#[tauri::command]
pub async fn app_update_check(
    app: AppHandle,
    tag: Option<String>,
) -> Result<AppUpdate, CommandError> {
    let current = app.package_info().version.to_string();
    let found = updater(&app, tag.as_deref())?
        .check()
        .await
        .map_err(|e| other(format!("couldn't check for updates: {e}")))?;
    let reason = blocker();
    Ok(match found {
        Some(update) => AppUpdate {
            downgrade: locdev_core::update::is_newer(&current, &update.version),
            current,
            bundled_daemon: env!("CARGO_PKG_VERSION"),
            version: Some(update.version.clone()),
            // RFC 3339, which the UI's `new Date()` reads; the date's
            // Display (`2026-11-24 22:51:12.62 +00:00:00`) showed "Invalid Date".
            date: update.date.and_then(|d| d.format(&Rfc3339).ok()),
            notes: update.body.clone(),
            can_install: reason.is_none(),
            reason,
            page: page(tag.as_deref()),
        },
        None => AppUpdate {
            current,
            bundled_daemon: env!("CARGO_PKG_VERSION"),
            version: None,
            date: None,
            notes: None,
            downgrade: false,
            can_install: reason.is_none(),
            reason,
            page: page(tag.as_deref()),
        },
    })
}

/// Downloads, checks the signature and installs (the same release the
/// check offered: it's fetched again here, never taken from the UI).
/// Progress goes out as `app-update-progress`. The new version runs after
/// `app_restart`; Windows' installer closes the app by itself.
#[tauri::command]
pub async fn app_update_install(
    app: AppHandle,
    tag: Option<String>,
) -> Result<String, CommandError> {
    if let Some(reason) = blocker() {
        return Err(blocked(reason));
    }
    {
        let mut busy = INSTALLING.lock().unwrap();
        if *busy {
            return Err(other("an update is already being installed"));
        }
        *busy = true;
    }
    let result = async {
        let update = updater(&app, tag.as_deref())?
            .check()
            .await
            .map_err(|e| other(format!("couldn't check for updates: {e}")))?
            .ok_or_else(|| other("that release has no update for this system"))?;
        // Windows replaces the bundled locdevd.exe, which can't happen while
        // it runs: stop it first (the new app starts it again).
        if cfg!(windows)
            && let Ok(paths) = Paths::resolve()
        {
            let _ = launcher::stop(&paths).await;
        }
        let mut downloaded = 0u64;
        let progress = app.clone();
        let finished = app.clone();
        update
            .download_and_install(
                move |chunk, total| {
                    downloaded += chunk as u64;
                    let _ = progress.emit(
                        "app-update-progress",
                        Progress {
                            downloaded,
                            total,
                            done: false,
                        },
                    );
                },
                move || {
                    let _ = finished.emit(
                        "app-update-progress",
                        Progress {
                            downloaded: 0,
                            total: None,
                            done: true,
                        },
                    );
                },
            )
            .await
            .map_err(|e| other(format!("the update failed: {e}")))?;
        Ok(update.version.clone())
    }
    .await;
    *INSTALLING.lock().unwrap() = false;
    result
}

/// Restarts into the installed version. The daemon is stopped first so the
/// new app starts the daemon it ships (sites stop for a moment).
#[tauri::command]
pub async fn app_restart(app: AppHandle, restart_daemon: bool) -> Result<(), CommandError> {
    if restart_daemon && let Ok(paths) = Paths::resolve() {
        let _ = launcher::stop(&paths).await;
    }
    app.restart();
}
