//! Desktop shell. The UI never talks to the system directly: every action is
//! a daemon request, exactly like the CLI.

use std::time::{Duration, Instant};

use locdev_core::TLD;
use locdev_core::paths::Paths;
use locdev_core::protocol::{PROTOCOL_VERSION, Request, Response};
use locdev_core::{AppError, ErrorCode};
use locdev_ipc::{Endpoint, IpcError, launcher};
use locdev_platform::elevate::Mode;
use locdev_platform::{Plan, resolver, service, trust};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::watch;

mod tray;
mod updater;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CommandError {
    /// The daemon is not running (or not reachable).
    DaemonDown {
        message: String,
    },
    Incompatible {
        message: String,
    },
    App {
        code: ErrorCode,
        message: String,
    },
    Other {
        message: String,
    },
}

impl From<IpcError> for CommandError {
    fn from(error: IpcError) -> Self {
        match error {
            IpcError::Io(_) | IpcError::Closed => Self::DaemonDown {
                message: error.to_string(),
            },
            IpcError::IncompatibleProtocol { .. } => Self::Incompatible {
                message: error.to_string(),
            },
            IpcError::App(AppError { code, message }) => Self::App { code, message },
            other => Self::Other {
                message: other.to_string(),
            },
        }
    }
}

impl From<AppError> for CommandError {
    fn from(AppError { code, message }: AppError) -> Self {
        Self::App { code, message }
    }
}

#[derive(Debug, Serialize)]
struct AppInfo {
    /// The app's own version (package.json), not the daemon's.
    version: String,
    /// The daemon/CLI this app ships (and starts).
    bundled_daemon: &'static str,
    arch: &'static str,
    /// Commit (12 hex) and build time (seconds since the epoch), when the
    /// build knew them.
    commit: Option<&'static str>,
    built_at: Option<u64>,
    /// `debug` for `make app`, `release` for installers.
    profile: &'static str,
    /// Where updates come from.
    releases: &'static str,
    protocol: u32,
    os: &'static str,
    endpoint: String,
    config_dir: String,
    log_dir: String,
}

#[tauri::command]
async fn daemon_request(request: Request) -> Result<Response, CommandError> {
    let paths = Paths::resolve()?;
    Ok(locdev_ipc::request(&paths, request).await?)
}

#[tauri::command]
async fn daemon_start() -> Result<bool, CommandError> {
    let paths = Paths::resolve()?;
    Ok(launcher::ensure_running(&paths).await?)
}

/// The splash stays at least this long, so it never just flickers, and at
/// most `SPLASH_MAX`, in case the UI never reports in.
const SPLASH_MIN: Duration = Duration::from_millis(900);
const SPLASH_MAX: Duration = Duration::from_secs(8);
/// Length of the splash's exit animation (see splash.css).
const SPLASH_EXIT: Duration = Duration::from_millis(480);

/// The daemon start begun at launch. It runs while the splash is up and the
/// main window loads hidden, so neither waits for the other.
struct Boot {
    started: Instant,
    daemon: watch::Receiver<Option<Result<bool, String>>>,
}

fn boot(app: &tauri::App) {
    let (done, daemon) = watch::channel(None);
    app.manage(Boot {
        started: Instant::now(),
        daemon,
    });

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let result = match Paths::resolve() {
            Ok(paths) => launcher::ensure_running(&paths)
                .await
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        match &result {
            Ok(started) => {
                let text = if *started {
                    "Daemon started"
                } else {
                    "Daemon running"
                };
                splash(&handle, "stage", &["daemon", "ok", text]);
                // The daemon answers `Status` once its listeners are up, so
                // these are the real states, not a guess.
                if let Ok(status) = daemon_status().await {
                    for (stage, state, text) in boot_stages(&status) {
                        splash(&handle, "stage", &[stage, state, &text]);
                    }
                }
                splash(&handle, "status", &["Loading your sites…"]);
            }
            Err(_) => {
                splash(&handle, "stage", &["daemon", "fail", ""]);
                splash(
                    &handle,
                    "failed",
                    &["Daemon didn't start; opening LocDev anyway"],
                );
            }
        }
        let _ = done.send(Some(result));
        tray::refresh(&handle);
    });

    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SPLASH_MAX).await;
        let hidden = handle
            .get_webview_window("main")
            .is_some_and(|main| !main.is_visible().unwrap_or(true));
        if hidden {
            show_main(&handle);
        }
    });
}

/// Calls `window.splash.<method>(args…)` in the splash, queuing the call if
/// its script hasn't loaded yet. Arguments are JSON-encoded, never spliced.
fn splash(app: &AppHandle, method: &str, args: &[&str]) {
    if let Some(window) = app.get_webview_window("splash") {
        let call = serde_json::to_string(
            &(std::iter::once(method)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()),
        )
        .unwrap_or_default();
        let _ = window.eval(format!(
            "(c => window.splash ? window.splash[c[0]](...c.slice(1)) : (window.__splashQueue ??= []).push(c))({call})"
        ));
    }
}

/// Each listener's state for the splash's boot trace: `ok`, `warn` (up, with
/// a caveat) or `fail`.
fn boot_stages(
    status: &locdev_core::model::DaemonStatus,
) -> Vec<(&'static str, &'static str, String)> {
    let proxy = &status.proxy;
    let proxy = match (proxy.enabled, proxy.port, &proxy.error) {
        (false, ..) => ("warn", "Proxy disabled".to_string()),
        (_, Some(port), None) => ("ok", format!("Proxy on :{port}")),
        (_, Some(port), Some(_)) => ("warn", format!("Proxy on fallback :{port}")),
        (_, None, _) => ("fail", "Proxy not listening".to_string()),
    };
    let dns = &status.dns;
    let dns = match (dns.port, dns.routing) {
        (None, _) => ("fail", "DNS server off".to_string()),
        (Some(_), locdev_core::model::Routing::Locdev) => ("ok", "*.test resolves".to_string()),
        (Some(_), _) => ("warn", "*.test not set up; using .localhost".to_string()),
    };
    let tls = &status.tls;
    let tls = match (tls.port, tls.trusted) {
        (None, _) => ("fail", "HTTPS off".to_string()),
        (Some(_), true) => ("ok", "HTTPS trusted".to_string()),
        (Some(_), false) => ("warn", "HTTPS not trusted yet".to_string()),
    };
    vec![
        ("proxy", proxy.0, proxy.1),
        ("dns", dns.0, dns.1),
        ("https", tls.0, tls.1),
    ]
}

/// Brings the main window up (from the splash, the tray or hidden), closing
/// the splash if it's still there.
/// What an OS notification asked the app to open (`--notification=<id>` or
/// `=list`), until the window takes it.
static OPEN_NOTIFICATION: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// The notification a launch argument names: an id, or `list`.
fn notification_arg<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter().find_map(|a| {
        let value = a.as_ref().strip_prefix("--notification=")?;
        (value == "list" || value.parse::<u64>().is_ok()).then(|| value.to_string())
    })
}

/// The window asks once it's up, for the launch that came from a notification.
#[tauri::command]
fn take_notification_to_open() -> Option<String> {
    OPEN_NOTIFICATION.lock().unwrap().take()
}

pub(crate) fn show_main(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
}

/// Waits for the launch-time daemon start; the UI's first step.
#[tauri::command]
async fn daemon_boot(boot: State<'_, Boot>) -> Result<bool, CommandError> {
    let mut daemon = boot.daemon.clone();
    let result = match daemon.wait_for(Option::is_some).await {
        Ok(result) => result.clone().unwrap_or(Ok(false)),
        Err(_) => Err("the daemon start was interrupted".into()),
    };
    result.map_err(|message| CommandError::Other { message })
}

/// The UI has its first data: play the splash's exit, then show the app.
#[tauri::command]
async fn app_ready(app: AppHandle, boot: State<'_, Boot>) -> Result<(), CommandError> {
    tokio::time::sleep(SPLASH_MIN.saturating_sub(boot.started.elapsed())).await;
    if app.get_webview_window("splash").is_some() {
        splash(&app, "done", &[]);
        tokio::time::sleep(SPLASH_EXIT).await;
    }
    show_main(&app);
    Ok(())
}

#[tauri::command]
async fn daemon_stop() -> Result<bool, CommandError> {
    let paths = Paths::resolve()?;
    Ok(launcher::stop(&paths).await?)
}

/// System changes that need administrator rights.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Target {
    /// Route *.test lookups to LocDev's DNS server.
    Dns,
    /// Trust the local CA in the system and browser stores.
    Tls,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Install,
    Uninstall,
}

/// Common name of the LocDev CA (see `locdev_tls::CA_NAME`).
const CA_NAME: &str = "LocDev Local CA";

async fn daemon_status() -> Result<locdev_core::model::DaemonStatus, CommandError> {
    let paths = Paths::resolve()?;
    match locdev_ipc::request(&paths, Request::Status).await? {
        Response::Status(status) => Ok(status),
        other => Err(CommandError::Other {
            message: format!("unexpected response: {other:?}"),
        }),
    }
}

/// Builds the plan from the daemon's real state. Used both to show the plan
/// and to apply it, so what runs is exactly what was shown.
async fn plan_for(target: Target, action: Action) -> Result<Plan, CommandError> {
    let other = |message: String| CommandError::Other { message };
    match (target, action) {
        (Target::Dns, Action::Install) => {
            let addr =
                daemon_status().await?.dns.listen().ok_or_else(|| {
                    other("the LocDev DNS server is not running; see Doctor".into())
                })?;
            resolver::install(TLD, addr).map_err(other)
        }
        (Target::Dns, Action::Uninstall) => resolver::uninstall(TLD).map_err(other),
        (Target::Tls, Action::Install) => {
            let ca = daemon_status()
                .await?
                .tls
                .ca_path
                .ok_or_else(|| other("the local CA is not available; see Doctor".into()))?;
            trust::install(&ca, &ca.with_file_name("ca.cer"), CA_NAME).map_err(other)
        }
        (Target::Tls, Action::Uninstall) => trust::uninstall(CA_NAME).map_err(other),
    }
}

#[tauri::command]
async fn system_plan(target: Target, action: Action) -> Result<Plan, CommandError> {
    plan_for(target, action).await
}

/// Applies the plan through the OS authorization prompt (polkit, macOS
/// admin dialog or UAC). The UI only calls this after showing the plan.
#[tauri::command]
async fn system_apply(target: Target, action: Action) -> Result<(), CommandError> {
    let plan = plan_for(target, action).await?;
    tauri::async_runtime::spawn_blocking(move || plan.apply(Mode::Gui))
        .await
        .map_err(|e| CommandError::Other {
            message: e.to_string(),
        })?
        .map_err(|e| CommandError::Other {
            message: e.to_string(),
        })
}

/// The tray menu's texts in the app's language (see `tray::set_strings`).
#[tauri::command]
fn tray_language(app: AppHandle, strings: std::collections::BTreeMap<String, String>) {
    tray::set_strings(&app, strings);
}

#[tauri::command]
fn service_status() -> service::ServiceStatus {
    service::status()
}

/// Turns "start LocDev at login" on or off. Per user; no elevation needed.
#[tauri::command]
fn service_set(enabled: bool) -> Result<service::ServiceStatus, CommandError> {
    let other = |e: std::io::Error| CommandError::Other {
        message: e.to_string(),
    };
    if enabled {
        let daemon = launcher::daemon_executable().ok_or_else(|| CommandError::Other {
            message: "could not find locdevd next to the app".into(),
        })?;
        let home = std::env::var_os(locdev_core::paths::HOME_ENV).map(std::path::PathBuf::from);
        service::install(&daemon, home.as_deref()).map_err(other)?;
    } else {
        service::uninstall().map_err(other)?;
    }
    Ok(service::status())
}

/// The `locdev` command bundled with the app: its version and whether a
/// terminal finds it.
#[tauri::command]
async fn cli_status() -> locdev_platform::cli::CliStatus {
    // Runs `locdev --version`: off the main thread.
    tauri::async_runtime::spawn_blocking(|| {
        locdev_platform::cli::status(locdev_platform::cli::bundled().as_deref())
    })
    .await
    .unwrap_or_else(|_| locdev_platform::cli::status(None))
}

/// Adds or removes the per-user link that puts the bundled `locdev` on PATH.
#[tauri::command]
async fn cli_link(enabled: bool) -> Result<locdev_platform::cli::CliStatus, CommandError> {
    let other = |e: std::io::Error| CommandError::Other {
        message: e.to_string(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let bundled = locdev_platform::cli::bundled();
        if enabled {
            let cli = bundled.as_deref().ok_or_else(|| CommandError::Other {
                message: "this build has no locdev next to the app".into(),
            })?;
            locdev_platform::cli::install(cli).map_err(other)?;
        } else {
            locdev_platform::cli::uninstall().map_err(other)?;
        }
        Ok(locdev_platform::cli::status(bundled.as_deref()))
    })
    .await
    .map_err(|e| CommandError::Other {
        message: e.to_string(),
    })?
}

/// Saves a captured message (`.eml`) or one of its attachments where the
/// user picks in the native dialog. The bytes come from the daemon here, so
/// the webview never handles the file. `None` when the dialog is cancelled.
#[tauri::command]
async fn save_mail_file(
    app: AppHandle,
    id: String,
    attachment: Option<usize>,
    file_name: String,
) -> Result<Option<String>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let other = |message: String| CommandError::Other { message };
    let request = match attachment {
        Some(index) => Request::MailAttachment { id, index },
        None => Request::MailRaw { id },
    };
    let base64 = match daemon_request(request).await? {
        Response::MailRaw { base64 } | Response::MailAttachment { base64, .. } => base64,
        _ => return Err(other("unexpected response from the daemon".into())),
    };
    let bytes = locdev_mail::parse::decode_base64(&base64)
        .ok_or_else(|| other("bad data from the daemon".into()))?;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(file_name)
            .blocking_save_file()
    })
    .await
    .map_err(|e| other(e.to_string()))?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    std::fs::write(&path, bytes).map_err(|e| other(format!("{}: {e}", path.display())))?;
    Ok(Some(path.display().to_string()))
}

/// Saves text the user exported (Debug Center events) where they choose,
/// through the native save dialog. `None` when cancelled.
#[tauri::command]
async fn save_text_file(
    app: AppHandle,
    file_name: String,
    text: String,
) -> Result<Option<String>, CommandError> {
    use tauri_plugin_dialog::DialogExt;
    let other = |message: String| CommandError::Other { message };
    // Only a plain name: the folder is the user's choice.
    let file_name = std::path::Path::new(&file_name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "export.txt".into());
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(file_name)
            .blocking_save_file()
    })
    .await
    .map_err(|e| other(e.to_string()))?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    std::fs::write(&path, text).map_err(|e| other(format!("{}: {e}", path.display())))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn app_info(app: AppHandle) -> Result<AppInfo, CommandError> {
    let paths = Paths::resolve()?;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        bundled_daemon: env!("CARGO_PKG_VERSION"),
        arch: std::env::consts::ARCH,
        commit: option_env!("LOCDEV_COMMIT"),
        built_at: option_env!("LOCDEV_BUILD_TIME").and_then(|s| s.parse().ok()),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        releases: updater::RELEASES,
        protocol: PROTOCOL_VERSION,
        os: std::env::consts::OS,
        endpoint: Endpoint::for_paths(&paths).address(),
        config_dir: paths.config_dir.display().to_string(),
        log_dir: paths.log_dir.display().to_string(),
    })
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    *OPEN_NOTIFICATION.lock().unwrap() = notification_arg(&args);
    tauri::Builder::default()
        // First, so a second launch only brings this one forward (also the way
        // back on desktops without a tray).
        .plugin(tauri_plugin_single_instance::init(|app, argv, _| {
            // A click on an OS notification launches the app with what to open.
            if let Some(target) = notification_arg(&argv) {
                *OPEN_NOTIFICATION.lock().unwrap() = Some(target.clone());
                let _ = app.emit("open-notification", target);
            }
            show_main(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // The app draws its own title bar (decorations are off in
            // tauri.conf.json); macOS keeps its traffic lights over it.
            #[cfg(target_os = "macos")]
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.set_decorations(true);
                let _ = main.set_title_bar_style(tauri::TitleBarStyle::Overlay);
                let _ = main.set_title("");
            }
            boot(app);
            tray::init(app)?;
            Ok(())
        })
        // Closing the window keeps LocDev in the tray; "Quit" there exits.
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            daemon_request,
            daemon_start,
            daemon_boot,
            app_ready,
            daemon_stop,
            system_plan,
            system_apply,
            service_status,
            service_set,
            save_mail_file,
            save_text_file,
            tray_language,
            app_info,
            take_notification_to_open,
            cli_status,
            cli_link,
            updater::app_update_check,
            updater::app_update_install,
            updater::app_restart
        ])
        .run(tauri::generate_context!())
        .expect("error while running LocDev");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_notification_a_click_launched_with() {
        assert_eq!(
            notification_arg(&["locdev-app", "--notification=42"]),
            Some("42".into())
        );
        assert_eq!(
            notification_arg(&["locdev-app", "--notification=list"]),
            Some("list".into())
        );
        assert_eq!(
            notification_arg(&["locdev-app", "--notification=../x"]),
            None
        );
        assert_eq!(notification_arg(&["locdev-app"]), None);
    }
}
