//! The tray icon: the daemon's state at a glance and the everyday actions
//! (sites, daemon, window) without opening the app. Like the UI, it only
//! sends daemon requests.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use locdev_core::model::{ProcessState, ProjectView};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_ipc::launcher;
use tauri::image::Image;
use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Notify;

const ICON_UP: &[u8] = include_bytes!("../icons/tray-on.png");
#[cfg(not(target_os = "macos"))]
const ICON_DOWN: &[u8] = include_bytes!("../icons/tray-off.png");
/// macOS menu bar icons are one-color templates the system tints; the
/// daemon's state shows in the menu and tooltip instead.
#[cfg(target_os = "macos")]
const ICON_TEMPLATE: &[u8] = include_bytes!("../icons/tray-template.png");

/// The dot over the icon: what's waiting to be read. Colors are documented
/// for users (docs: App tour → Tray) and in the tooltip.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Dot {
    #[default]
    None,
    /// Unread notifications: blue.
    Notifications,
    /// Unread captured emails: green.
    Mail,
    /// Both: violet.
    Both,
}

impl Dot {
    fn of(notifications: usize, mail: usize) -> Self {
        match (notifications > 0, mail > 0) {
            (false, false) => Self::None,
            (true, false) => Self::Notifications,
            (false, true) => Self::Mail,
            (true, true) => Self::Both,
        }
    }

    fn color(self) -> Option<[u8; 3]> {
        match self {
            Self::None => None,
            Self::Notifications => Some([0x3B, 0x9E, 0xFF]),
            Self::Mail => Some([0x22, 0xC5, 0x5E]),
            Self::Both => Some([0xB4, 0x6B, 0xFF]),
        }
    }
}

/// The icon for a state, with the dot drawn over the bottom-right corner.
/// On macOS a dot needs color, so it uses the full-color icon instead of
/// the template (which the system would tint).
fn icon_for(up: bool, dot: Dot) -> Option<(Image<'static>, bool)> {
    #[cfg(target_os = "macos")]
    let (bytes, template) = if dot == Dot::None {
        (ICON_TEMPLATE, true)
    } else {
        (ICON_UP, false)
    };
    #[cfg(not(target_os = "macos"))]
    let (bytes, template) = (if up { ICON_UP } else { ICON_DOWN }, false);
    #[cfg(target_os = "macos")]
    let _ = up;
    let base = Image::from_bytes(bytes).ok()?;
    let Some(color) = dot.color() else {
        return Some((base.to_owned(), template));
    };
    let (w, h) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    paint_dot(&mut rgba, w, h, color);
    Some((Image::new_owned(rgba, w, h), template))
}

/// A filled circle with a dark ring, so it reads on light and dark panels.
fn paint_dot(rgba: &mut [u8], w: u32, h: u32, color: [u8; 3]) {
    let size = w.min(h) as f32;
    let r = size * 0.21;
    let ring = (size * 0.045).max(1.0);
    let (cx, cy) = (w as f32 - r - ring, h as f32 - r - ring);
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            // Coverage for a soft edge (one pixel of antialiasing).
            let inner = (r - d + 0.5).clamp(0.0, 1.0);
            let outer = (r + ring - d + 0.5).clamp(0.0, 1.0);
            if outer <= 0.0 {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            let px = &mut rgba[i..i + 4];
            let ring_rgb = [0x13, 0x10, 0x0E];
            for c in 0..3 {
                let fill = color[c] as f32 * inner + ring_rgb[c] as f32 * (1.0 - inner);
                px[c] = (fill * outer + px[c] as f32 * (1.0 - outer)).round() as u8;
            }
            px[3] = (255.0 * outer + px[3] as f32 * (1.0 - outer)).round() as u8;
        }
    }
}

/// The daemon is local, so asking every few seconds costs next to nothing.
const POLL: Duration = Duration::from_secs(3);

/// Wakes the refresh loop right after an action, instead of at the next poll.
struct Refresh(Arc<Notify>);

/// The menu's texts in the app's language: the app sends them, translated
/// from its own catalogs, once it loads (and again after the language
/// changes, which reloads it). English, the keys, until then. The number
/// counts the changes, so a new language rebuilds the menu.
static STRINGS: Mutex<(u64, BTreeMap<String, String>)> = Mutex::new((0, BTreeMap::new()));

fn tr(key: &str) -> String {
    STRINGS
        .lock()
        .unwrap()
        .1
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `tr` with `{name}` placeholders.
fn trf(key: &str, vars: &[(&str, usize)]) -> String {
    vars.iter().fold(tr(key), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

/// "1 unread email" / "3 unread emails", each form translated whole.
fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        tr(one)
    } else {
        trf(many, &[("n", n)])
    }
}

pub(crate) fn set_strings(app: &AppHandle, strings: BTreeMap<String, String>) {
    {
        let mut current = STRINGS.lock().unwrap();
        if current.1 == strings {
            return;
        }
        current.0 += 1;
        current.1 = strings;
    }
    refresh(app);
}

/// What the tray shows; the menu is only rebuilt when this changes.
#[derive(Debug, Default, Clone, PartialEq)]
struct Snapshot {
    /// Sites running / registered, from `Status`; `None` when the daemon is down.
    daemon: Option<(usize, usize)>,
    /// Unread notifications and unread captured emails (from `Status`).
    unread: (usize, usize),
    sites: Vec<Site>,
    /// The texts' version (see `STRINGS`).
    strings: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct Site {
    name: String,
    domain: String,
    url: Option<String>,
    state: ProcessState,
    /// Static sites have no process to start or stop.
    runnable: bool,
}

impl From<ProjectView> for Site {
    fn from(view: ProjectView) -> Self {
        Self {
            runnable: view.project.command.is_some(),
            name: view.project.name,
            domain: view.project.domain,
            url: view.https_url.or(view.url),
            state: view.process.state,
        }
    }
}

pub fn init(app: &tauri::App) -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    quiet_appindicator_deprecation();

    let notify = Arc::new(Notify::new());
    app.manage(Refresh(notify.clone()));

    let snapshot = Snapshot::default();
    let tray = TrayIconBuilder::with_id("main")
        .icon(
            icon_for(false, Dot::None)
                .map(|(i, _)| i)
                .ok_or_else(|| tauri::Error::InvalidIcon(std::io::Error::other("tray icon")))?,
        )
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("LocDev")
        .menu(&menu(app.handle(), &snapshot)?)
        // Windows and Linux: left click opens the app, right click the menu.
        // macOS convention is a menu on any click.
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(on_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main(tray.app_handle());
            }
        })
        .build(app)?;

    tauri::async_runtime::spawn(refresh_loop(app.handle().clone(), tray, notify));
    Ok(())
}

/// libayatana-appindicator warns that it's deprecated each time it loads,
/// and the tray crates don't support its successor yet. Drops only that
/// line; anything else from the library still reaches the log.
#[cfg(target_os = "linux")]
fn quiet_appindicator_deprecation() {
    glib::log_set_handler(
        Some("libayatana-appindicator"),
        glib::LogLevels::LEVEL_WARNING,
        false,
        false,
        |domain, level, message| {
            if !message.contains("is deprecated") {
                glib::log_default_handler(domain, level, Some(message));
            }
        },
    );
}

/// Updates the tray now instead of at the next poll (after the launch-time
/// daemon start, or an action).
pub(crate) fn refresh(app: &AppHandle) {
    if let Some(refresh) = app.try_state::<Refresh>() {
        refresh.0.notify_one();
    }
}

async fn refresh_loop(app: AppHandle, tray: TrayIcon, notify: Arc<Notify>) {
    // Matches the menu `init` built.
    let mut shown = Snapshot::default();
    let mut icon = Icon::default();
    loop {
        // Two steps: `Status` answers at once, while other requests wait for
        // the daemon's runtime detection (a second or two after it starts),
        // so the state is right before the site list arrives.
        if let Ok(paths) = Paths::resolve() {
            let counts = daemon_counts(&paths).await;
            let mut next = Snapshot {
                daemon: counts.map(|c| (c.running, c.total)),
                unread: counts
                    .map(|c| (c.notifications, c.mail))
                    .unwrap_or_default(),
                sites: shown.sites.clone(),
                strings: STRINGS.lock().unwrap().0,
            };
            if next.daemon.is_none() {
                next.sites.clear();
            }
            show(&app, &tray, &mut shown, next.clone());
            icon.update(&tray, &shown);
            if next.daemon.is_some()
                && let Some(sites) = list_sites(&paths).await
            {
                next.sites = sites;
                show(&app, &tray, &mut shown, next);
            }
        }
        tokio::select! {
            () = tokio::time::sleep(POLL) => {}
            () = notify.notified() => {}
        }
    }
}

fn show(app: &AppHandle, tray: &TrayIcon, shown: &mut Snapshot, next: Snapshot) {
    if *shown == next {
        return;
    }
    let Ok(menu) = menu(app, &next) else { return };
    let _ = tray.set_menu(Some(menu));
    let _ = tray.set_tooltip(Some(summary(&next)));
    *shown = next;
}

/// The icon actually on screen. On Linux every `set_icon` deletes the
/// previous PNG and writes a new one; the AppIndicator extension loads it
/// asynchronously, so back-to-back updates left it pointing at a deleted
/// file, drawn as "...". So the icon changes only when what it shows changes
/// (daemon up/down, the dot), and never twice within a few seconds: a change
/// that comes too soon waits for the next poll.
#[derive(Default)]
struct Icon {
    shown: Option<(bool, Dot)>,
    changed_at: Option<std::time::Instant>,
}

impl Icon {
    const SETTLE: Duration = Duration::from_secs(2);

    fn update(&mut self, tray: &TrayIcon, snapshot: &Snapshot) {
        let want = (
            snapshot.daemon.is_some(),
            Dot::of(snapshot.unread.0, snapshot.unread.1),
        );
        // `init` set the dimmed icon without a dot.
        let shown = self.shown.unwrap_or((false, Dot::None));
        if want == shown
            || self
                .changed_at
                .is_some_and(|at| at.elapsed() < Self::SETTLE)
        {
            return;
        }
        if let Some((image, template)) = icon_for(want.0, want.1) {
            let _ = tray.set_icon_with_as_template(Some(image), template);
            self.shown = Some(want);
            self.changed_at = Some(std::time::Instant::now());
        }
    }
}

#[derive(Clone, Copy)]
struct Counts {
    running: usize,
    total: usize,
    notifications: usize,
    mail: usize,
}

async fn daemon_counts(paths: &Paths) -> Option<Counts> {
    match locdev_ipc::request(paths, Request::Status).await {
        Ok(Response::Status(status)) => Some(Counts {
            running: status.running,
            total: status.projects,
            notifications: status.unread_notifications,
            mail: status.mail.unread,
        }),
        _ => None,
    }
}

async fn list_sites(paths: &Paths) -> Option<Vec<Site>> {
    match locdev_ipc::request(paths, Request::ListProjects).await {
        Ok(Response::Projects { projects }) => Some(projects.into_iter().map(Site::from).collect()),
        _ => None,
    }
}

/// The daemon's state: the menu's first line.
fn status_line(snapshot: &Snapshot) -> String {
    match snapshot.daemon {
        Some((running, total)) => trf(
            "LocDev: daemon running · {running}/{total} sites up",
            &[("running", running), ("total", total)],
        ),
        None => tr("LocDev: daemon stopped"),
    }
}

/// The tooltip: state and unread counts on one line.
fn summary(snapshot: &Snapshot) -> String {
    let mut out = status_line(snapshot);
    if snapshot.daemon.is_none() {
        return out;
    }
    if let Some(unread) = unread_line(snapshot) {
        out.push_str(" · ");
        out.push_str(&unread);
    }
    out
}

/// "2 unread notifications (blue dot)", "1 unread email (green dot)", or both (violet).
fn unread_line(snapshot: &Snapshot) -> Option<String> {
    let (n, m) = snapshot.unread;
    let dot = match Dot::of(n, m) {
        Dot::None => return None,
        Dot::Notifications => tr("blue dot"),
        Dot::Mail => tr("green dot"),
        Dot::Both => tr("violet dot"),
    };
    let mut parts = Vec::new();
    if n > 0 {
        parts.push(count(
            n,
            "1 unread notification",
            "{n} unread notifications",
        ));
    }
    if m > 0 {
        parts.push(count(m, "1 unread email", "{n} unread emails"));
    }
    Some(format!("{} ({dot})", parts.join(", ")))
}

fn menu(app: &AppHandle, snapshot: &Snapshot) -> tauri::Result<Menu<tauri::Wry>> {
    // The menu is as wide as its longest line, and native menus take no
    // width: the unread counts get a line of their own so the header
    // doesn't stretch the whole menu when they appear.
    let header = MenuItemBuilder::with_id("header", status_line(snapshot))
        .enabled(false)
        .build(app)?;
    let mut menu = MenuBuilder::new(app).item(&header);
    if snapshot.daemon.is_some()
        && let Some(unread) = unread_line(snapshot)
    {
        let line = MenuItemBuilder::with_id("header-unread", unread)
            .enabled(false)
            .build(app)?;
        menu = menu.item(&line);
    }
    let mut menu = menu.separator();

    if let Some((_, total)) = snapshot.daemon {
        if snapshot.sites.is_empty() {
            let text = tr(if total == 0 {
                "No sites yet"
            } else {
                "Loading sites…"
            });
            let none = MenuItemBuilder::with_id("no-sites", text)
                .enabled(false)
                .build(app)?;
            menu = menu.item(&none);
        }
        for site in &snapshot.sites {
            menu = menu.item(&site_menu(app, site)?);
        }
        menu = menu.separator();
    }

    let (unread, mail) = snapshot.unread;
    if unread > 0 {
        let label = count(unread, "1 unread notification", "{n} unread notifications");
        menu = menu.text("goto:notifications", label);
    }
    if mail > 0 {
        let label = count(mail, "1 unread email", "{n} unread emails");
        menu = menu.text("goto:mailbox", label);
    }
    menu = menu.text("open", tr("Open LocDev")).separator();
    menu = if snapshot.daemon.is_some() {
        menu.text("daemon:restart", tr("Restart daemon"))
            .text("daemon:stop", tr("Stop daemon"))
    } else {
        menu.text("daemon:start", tr("Start daemon"))
    };
    // Quitting closes the app only; the daemon keeps serving sites, as it
    // does for the CLI.
    menu.separator().text("quit", tr("Quit LocDev")).build()
}

fn site_menu(app: &AppHandle, site: &Site) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    // Menus can't color text, so the state is a glyph, like the app's LEDs.
    let mark = match site.state {
        ProcessState::Running => "●",
        ProcessState::Failed | ProcessState::Exited => "✕",
        _ => "○",
    };
    let id = |action: &str| format!("site:{action}:{}", site.name);

    let mut menu = SubmenuBuilder::new(app, format!("{mark}  {}", site.domain));
    if site.url.is_some() {
        menu = menu.text(id("open"), tr("Open in browser"));
    }
    if site.runnable {
        menu = if site.state == ProcessState::Running {
            menu.text(id("restart"), tr("Restart"))
                .text(id("stop"), tr("Stop"))
        } else {
            menu.text(id("start"), tr("Start"))
        };
    }
    menu.build()
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_owned();
    match id.as_str() {
        "open" => crate::show_main(app),
        // The app moves to the page itself (it listens for `navigate`).
        "goto:notifications" => {
            crate::show_main(app);
            let _ = app.emit("navigate", ("notifications", None::<&str>));
        }
        "goto:mailbox" => {
            crate::show_main(app);
            let _ = app.emit("navigate", ("tools", Some("mailbox")));
        }
        "quit" => app.exit(0),
        _ => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(message) = run_action(&app, &id).await {
                    app.dialog()
                        .message(message)
                        .title("LocDev")
                        .kind(MessageDialogKind::Error)
                        .show(|_| {});
                }
                refresh(&app);
            });
        }
    }
}

async fn run_action(app: &AppHandle, id: &str) -> Result<(), String> {
    let paths = Paths::resolve().map_err(|e| e.to_string())?;
    let ipc = |e: locdev_ipc::IpcError| e.to_string();

    match id {
        "daemon:start" => launcher::ensure_running(&paths)
            .await
            .map(drop)
            .map_err(ipc),
        "daemon:stop" => launcher::stop(&paths).await.map(drop).map_err(ipc),
        "daemon:restart" => {
            launcher::stop(&paths).await.map_err(ipc)?;
            launcher::ensure_running(&paths)
                .await
                .map(drop)
                .map_err(ipc)
        }
        _ => {
            let Some((action, name)) = id
                .strip_prefix("site:")
                .and_then(|rest| rest.split_once(':'))
            else {
                return Ok(());
            };
            let name = name.to_owned();
            let request = match action {
                "open" => return open_site(app, &paths, name).await,
                "start" => Request::StartProject { name },
                "stop" => Request::StopProject { name },
                "restart" => Request::RestartProject { name },
                _ => return Ok(()),
            };
            locdev_ipc::request(&paths, request)
                .await
                .map(drop)
                .map_err(ipc)
        }
    }
}

/// Opens the site's current address (HTTPS when served), asked fresh so a
/// changed port or TLS setting is honored.
async fn open_site(app: &AppHandle, paths: &Paths, name: String) -> Result<(), String> {
    let site = match locdev_ipc::request(paths, Request::ListProjects).await {
        Ok(Response::Projects { projects }) => projects
            .into_iter()
            .map(Site::from)
            .find(|s| s.name == name),
        Ok(_) => None,
        Err(e) => return Err(e.to_string()),
    };
    let url = site
        .and_then(|s| s.url)
        .ok_or_else(|| format!("{name} has no address yet"))?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
