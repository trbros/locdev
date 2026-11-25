//! Local notifications: kept on disk (`state/notifications.json`), so they
//! survive the app, the daemon and the machine being turned off, shown
//! once (deduplicated by key) and, when the user opted in, also through the
//! OS. Nothing leaves the machine.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use locdev_core::AppError;
use locdev_core::deps::{
    Notification, NotificationCategory, NotificationLink, NotificationSettings, Severity,
};
use locdev_core::model::now_ms;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::fsutil::write_atomic;

const FORMAT: u32 = 1;
/// Kept at most; dismissed and read ones go first.
const CAPACITY: usize = 500;
/// Undelivered notifications older than this are not pushed to the OS on
/// boot (they are still in the app).
const CATCH_UP_MS: u64 = 3 * 24 * 3_600_000;

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    format: u32,
    next_id: u64,
    notifications: Vec<Notification>,
}

pub struct Notifications {
    path: PathBuf,
    store: Mutex<StoreFile>,
}

/// What a caller wants to say; the store fills in the rest.
pub struct New {
    pub category: NotificationCategory,
    pub severity: Severity,
    pub title: String,
    pub body: String,
    pub project: Option<String>,
    pub link: Option<NotificationLink>,
    pub dedup: String,
}

impl Notifications {
    pub fn load(path: &Path) -> Result<Self, AppError> {
        let store = match std::fs::read(path) {
            Ok(raw) => match serde_json::from_slice::<StoreFile>(&raw) {
                Ok(store) if store.format <= FORMAT => store,
                Ok(store) => {
                    return Err(AppError::new(
                        locdev_core::ErrorCode::Config,
                        format!(
                            "{}: written by a newer LocDev (format {})",
                            path.display(),
                            store.format
                        ),
                    ));
                }
                // Never throw the user's data away: keep the broken file aside.
                Err(error) => {
                    warn!(%error, "notifications file is damaged; keeping a copy and starting fresh");
                    let _ = std::fs::rename(path, path.with_extension("json.broken"));
                    StoreFile::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => StoreFile::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path: path.to_path_buf(),
            store: Mutex::new(store),
        })
    }

    fn save(&self, store: &StoreFile) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_vec_pretty(store) {
            Ok(raw) => {
                if let Err(error) = write_atomic(&self.path, &raw) {
                    warn!(%error, "could not save notifications");
                }
            }
            Err(error) => warn!(%error, "could not serialize notifications"),
        }
    }

    /// Adds a notification unless one with the same key exists (dismissed
    /// ones count: a dismissed alert stays dismissed). Returns it if new.
    pub fn push(&self, new: New, settings: &NotificationSettings) -> Option<Notification> {
        if !settings.enabled(new.category) {
            return None;
        }
        let mut store = self.store.lock().unwrap();
        if store.notifications.iter().any(|n| n.dedup == new.dedup) {
            return None;
        }
        store.next_id += 1;
        let notification = Notification {
            id: store.next_id,
            category: new.category,
            severity: new.severity,
            title: new.title,
            body: new.body,
            project: new.project,
            link: new.link,
            created_at: now_ms(),
            read: false,
            dismissed: false,
            delivered: false,
            dedup: new.dedup,
        };
        store.notifications.push(notification.clone());
        if store.notifications.len() > CAPACITY {
            // Oldest dismissed first, then oldest read, then oldest.
            let victim = store
                .notifications
                .iter()
                .position(|n| n.dismissed)
                .or_else(|| store.notifications.iter().position(|n| n.read))
                .unwrap_or(0);
            store.notifications.remove(victim);
        }
        store.format = FORMAT;
        self.save(&store);
        Some(notification)
    }

    pub fn list(&self, include_dismissed: bool) -> Vec<Notification> {
        let store = self.store.lock().unwrap();
        let mut out: Vec<_> = store
            .notifications
            .iter()
            .filter(|n| include_dismissed || !n.dismissed)
            .cloned()
            .collect();
        out.sort_by_key(|j| std::cmp::Reverse(j.id));
        out
    }

    pub fn unread(&self) -> usize {
        self.store
            .lock()
            .unwrap()
            .notifications
            .iter()
            .filter(|n| !n.read && !n.dismissed)
            .count()
    }

    /// Unread ones outside `category` (the tray counts captured emails apart).
    pub fn unread_except(&self, category: NotificationCategory) -> usize {
        self.store
            .lock()
            .unwrap()
            .notifications
            .iter()
            .filter(|n| !n.read && !n.dismissed && n.category != category)
            .count()
    }

    /// Marks read the unread ones of `category` that `keep` rejects (a
    /// captured email that was opened or deleted). Saves only on a change.
    pub fn read_where(&self, category: NotificationCategory, keep: impl Fn(&str) -> bool) {
        let mut store = self.store.lock().unwrap();
        let mut changed = false;
        for n in store
            .notifications
            .iter_mut()
            .filter(|n| n.category == category && !n.read && !keep(&n.dedup))
        {
            n.read = true;
            changed = true;
        }
        if changed {
            self.save(&store);
        }
    }

    /// Marks `ids` (every notification when empty).
    pub fn mark(&self, ids: &[u64], read: Option<bool>, dismissed: Option<bool>) {
        let mut store = self.store.lock().unwrap();
        for n in store
            .notifications
            .iter_mut()
            .filter(|n| ids.is_empty() || ids.contains(&n.id))
        {
            if let Some(read) = read {
                n.read = read;
            }
            if let Some(dismissed) = dismissed {
                n.dismissed = dismissed;
                if dismissed {
                    n.read = true;
                }
            }
        }
        self.save(&store);
    }

    /// Pushes not-yet-shown notifications to the OS: right after they are
    /// created, and on boot for what came in while nothing could show them.
    /// Many at once become one summary. With `app_in_front`, a LocDev window
    /// already shows them as toasts: they count as delivered, so they don't
    /// pop up later as a backlog.
    pub async fn deliver(&self, settings: &NotificationSettings, app_in_front: bool) {
        if !settings.native {
            return;
        }
        if app_in_front {
            let mut store = self.store.lock().unwrap();
            let mut changed = false;
            for n in store.notifications.iter_mut().filter(|n| !n.delivered) {
                n.delivered = true;
                changed = true;
            }
            if changed {
                self.save(&store);
            }
            return;
        }
        let pending: Vec<Notification> = {
            let store = self.store.lock().unwrap();
            let cutoff = now_ms().saturating_sub(CATCH_UP_MS);
            store
                .notifications
                .iter()
                .filter(|n| !n.delivered && !n.read && !n.dismissed && n.created_at >= cutoff)
                .cloned()
                .collect()
        };
        if pending.is_empty() {
            return;
        }
        let (title, body) = if pending.len() > 3 {
            (
                format!("LocDev: {} notifications", pending.len()),
                pending
                    .iter()
                    .take(3)
                    .map(|n| n.title.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n…",
            )
        } else {
            (String::new(), String::new())
        };
        let shown = tokio::task::spawn_blocking(move || {
            if title.is_empty() {
                pending
                    .iter()
                    .all(|n| show(&n.title, &n.body, Opens::One(n.id)))
                    .then_some(pending)
            } else {
                show(&title, &body, Opens::List).then_some(pending)
            }
        })
        .await
        .ok()
        .flatten();
        if let Some(shown) = shown {
            let ids: Vec<u64> = shown.iter().map(|n| n.id).collect();
            let mut store = self.store.lock().unwrap();
            for n in store
                .notifications
                .iter_mut()
                .filter(|n| ids.contains(&n.id))
            {
                n.delivered = true;
            }
            self.save(&store);
        }
    }
}

/// What clicking an OS notification opens in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opens {
    /// That notification: its site, page or message.
    One(u64),
    /// The notification list (a summary of several).
    List,
}

impl Opens {
    /// The argument the app reads (`--notification=<id>` or `=list`).
    fn arg(self) -> String {
        match self {
            Opens::One(id) => format!("--notification={id}"),
            Opens::List => "--notification=list".into(),
        }
    }
}

/// One OS notification; `false` when the desktop can't show it (no session
/// bus, no notification service). Blocking. On Linux a click opens the app
/// on what it's about; macOS and Windows keep their default click.
fn show(title: &str, body: &str, opens: Opens) -> bool {
    let mut notification = notify_rust::Notification::new();
    notification.appname("LocDev").summary(title).body(body);
    #[cfg(all(unix, not(target_os = "macos")))]
    notification.action("default", "Open");
    match notification.show() {
        Ok(handle) => {
            #[cfg(all(unix, not(target_os = "macos")))]
            std::thread::spawn(move || {
                handle.wait_for_action(|action| {
                    if action == "default" {
                        open_app(opens);
                    }
                });
            });
            #[cfg(not(all(unix, not(target_os = "macos"))))]
            let _ = (handle, opens);
            true
        }
        Err(error) => {
            debug!(%error, "native notification not shown");
            false
        }
    }
}

/// Opens the desktop app (next to this daemon in every package, and in a
/// development build) on `opens`; a running app gets the argument through its
/// single-instance handoff.
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
fn open_app(opens: Opens) {
    // `locdev-app` in the packages, `locdev-desktop` in a development build.
    let exe = std::env::current_exe().ok();
    let Some(app) = ["locdev-app", "locdev-desktop"]
        .iter()
        .filter_map(|name| {
            let file = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                (*name).to_string()
            };
            exe.as_ref().map(|exe| exe.with_file_name(file))
        })
        .find(|app| app.is_file())
    else {
        debug!("no desktop app next to the daemon; the notification can't open it");
        return;
    };
    if let Err(error) = locdev_core::process::command(&app)
        .arg(opens.arg())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        debug!(%error, app = %app.display(), "could not open the app from a notification");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_opens_the_notification_or_the_list() {
        assert_eq!(Opens::One(42).arg(), "--notification=42");
        assert_eq!(Opens::List.arg(), "--notification=list");
    }

    fn new(dedup: &str) -> New {
        New {
            category: NotificationCategory::DependencyUpdate,
            severity: Severity::Info,
            title: format!("title {dedup}"),
            body: String::new(),
            project: Some("blog".into()),
            link: None,
            dedup: dedup.into(),
        }
    }

    #[test]
    fn persists_and_never_repeats() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state/notifications.json");
        let settings = NotificationSettings::default();
        let store = Notifications::load(&path).unwrap();

        assert!(store.push(new("a"), &settings).is_some());
        assert!(
            store.push(new("a"), &settings).is_none(),
            "same key, no second notification"
        );
        let b = store.push(new("b"), &settings).unwrap();
        store.mark(&[b.id], None, Some(true));
        // Dismissed stays dismissed: the same alert doesn't come back.
        assert!(store.push(new("b"), &settings).is_none());

        // Survives a restart (the machine turned off and on).
        let again = Notifications::load(&path).unwrap();
        assert_eq!(again.unread(), 1);
        assert_eq!(again.list(false).len(), 1);
        assert_eq!(again.list(true).len(), 2);
        let c = again.push(new("c"), &settings).unwrap();
        assert!(c.id > b.id, "ids keep increasing across restarts");
    }

    #[test]
    fn respects_disabled_categories() {
        let dir = tempfile::tempdir().unwrap();
        let store = Notifications::load(&dir.path().join("n.json")).unwrap();
        let mut settings = NotificationSettings::default();
        settings
            .categories
            .insert(NotificationCategory::DependencyUpdate, false);
        assert!(store.push(new("a"), &settings).is_none());
    }

    #[test]
    fn keeps_a_damaged_file_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("n.json");
        std::fs::write(&path, "{ broken").unwrap();
        let store = Notifications::load(&path).unwrap();
        assert_eq!(store.list(true).len(), 0);
        assert!(dir.path().join("n.json.broken").is_file());
    }

    #[test]
    fn mark_all_read() {
        let dir = tempfile::tempdir().unwrap();
        let store = Notifications::load(&dir.path().join("n.json")).unwrap();
        let settings = NotificationSettings::default();
        store.push(new("a"), &settings);
        store.push(new("b"), &settings);
        store.mark(&[], Some(true), None);
        assert_eq!(store.unread(), 0);
    }
}
