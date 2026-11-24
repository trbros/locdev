//! The update check: LocDev's published releases on GitHub, compared with
//! this daemon. Read only and anonymous; installing is the app's updater
//! (signed) or the CLI's install script (SHA256SUMS), never the daemon.

use std::sync::Mutex;

use locdev_core::deps::{JobKind, NotificationCategory, NotificationLink, Severity};
use locdev_core::model::now_ms;
use locdev_core::protocol::{Request, Response};
use locdev_core::update::{UpdateStatus, is_newer};
use locdev_core::{AppError, ErrorCode};
use locdev_github::releases;
use tracing::info;

use super::Daemon;
use crate::jobs::Spec as JobSpec;
use crate::notify::New;

const API: &str = "https://api.github.com";
/// Answers are reused this long unless a refresh is asked for.
const FRESH_MS: u64 = 6 * 3_600_000;
/// Between automatic checks.
const DAILY_MS: u64 = 24 * 3_600_000;

#[derive(Default)]
pub(super) struct UpdatesState {
    last: Mutex<Option<UpdateStatus>>,
}

impl Daemon {
    fn updates_api(&self) -> Result<String, AppError> {
        let api = self
            .config
            .updates
            .api
            .clone()
            .unwrap_or_else(|| API.into());
        if !(api.starts_with("https://")
            || api.starts_with("http://127.0.0.1:")
            || api.starts_with("http://localhost:"))
        {
            return Err(AppError::new(
                ErrorCode::Config,
                format!("[updates] api must be https (or http on loopback): {api}"),
            ));
        }
        Ok(api.trim_end_matches('/').to_string())
    }

    fn updates_repo(&self) -> Result<String, AppError> {
        let repo = self.config.updates.repo.trim().to_string();
        let ok = repo.split_once('/').is_some_and(|(owner, name)| {
            // `.` is allowed in names, but never `..` or a leading dot.
            let part = |s: &str| {
                !s.is_empty()
                    && !s.starts_with('.')
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            };
            part(owner) && part(name) && !name.contains('/')
        });
        if !ok {
            return Err(AppError::new(
                ErrorCode::Config,
                format!("[updates] repo must be owner/name: {repo}"),
            ));
        }
        Ok(repo)
    }

    fn update_status_blank(&self, repo: String) -> UpdateStatus {
        UpdateStatus {
            repo,
            daemon: env!("CARGO_PKG_VERSION").into(),
            latest: None,
            releases: Vec::new(),
            daemon_update: false,
            checked_at: None,
            error: None,
        }
    }

    /// Asks GitHub (blocking work off the runtime) and keeps the answer.
    /// A failed check keeps the releases seen last and records why.
    pub(super) async fn update_check(&self, refresh: bool) -> Result<UpdateStatus, AppError> {
        let repo = self.updates_repo()?;
        if let Some(last) = self.updates.last.lock().unwrap().clone()
            && !refresh
            && last.repo == repo
            && last
                .checked_at
                .is_some_and(|at| now_ms().saturating_sub(at) < FRESH_MS)
        {
            return Ok(last);
        }
        let api = self.updates_api()?;
        let target = repo.clone();
        let result = tokio::task::spawn_blocking(move || releases::list(&api, &target))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
        let mut status = self
            .updates
            .last
            .lock()
            .unwrap()
            .clone()
            .filter(|s| s.repo == repo)
            .unwrap_or_else(|| self.update_status_blank(repo));
        status.checked_at = Some(now_ms());
        match result {
            Ok(list) => {
                status.latest = list.iter().find(|r| !r.prerelease).cloned();
                status.daemon_update = status
                    .latest
                    .as_ref()
                    .and_then(|r| r.daemon.as_deref())
                    .is_some_and(|v| is_newer(v, &status.daemon));
                status.releases = list;
                status.error = None;
            }
            Err(error) => status.error = Some(error),
        }
        *self.updates.last.lock().unwrap() = Some(status.clone());
        Ok(status)
    }

    /// The daily job: a notification when a newer release is out (once per
    /// release; nothing is installed).
    pub(super) async fn run_update_check(&self) -> Result<String, AppError> {
        let status = self.update_check(true).await?;
        if let Some(error) = &status.error {
            return Err(AppError::new(ErrorCode::Remote, error.clone()));
        }
        let Some(latest) = &status.latest else {
            return Ok("no published release yet".into());
        };
        // Only the daemon/CLI is compared here: the app checks its own
        // version through its updater and says so itself.
        if status.daemon_update {
            self.notify(New {
                category: NotificationCategory::Update,
                severity: Severity::Info,
                title: format!("LocDev {} is available", latest.tag),
                body: format!(
                    "This daemon and CLI are {}; the release ships {}.",
                    status.daemon,
                    latest.daemon.as_deref().unwrap_or("?")
                ),
                project: None,
                link: Some(NotificationLink {
                    view: "settings".into(),
                    sub: Some("updates".into()),
                    project: None,
                }),
                dedup: format!("update:{}", latest.tag),
            });
            info!(tag = %latest.tag, "newer LocDev release");
            return Ok(format!("{} is available", latest.tag));
        }
        Ok(format!("up to date ({})", latest.tag))
    }

    /// Once a day, unless offline or turned off.
    pub(super) fn schedule_updates(&self) {
        let settings = self.settings.get();
        if settings.network.offline || !settings.automation.updates {
            return;
        }
        let last = self
            .updates
            .last
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.checked_at)
            .unwrap_or(0);
        if now_ms().saturating_sub(last) < DAILY_MS {
            return;
        }
        self.jobs.enqueue(JobSpec {
            kind: JobKind::UpdateCheck,
            project: None,
            input: serde_json::Value::Null,
            dedup: "updates".into(),
            automatic: true,
            max_attempts: 2,
            not_before: None,
        });
        self.job_wake.notify_one();
    }

    pub(super) async fn update_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::UpdateCheck { refresh } => {
                Ok(Response::Updates(self.update_check(refresh).await?))
            }
            _ => unreachable!("update_request only gets update requests"),
        }
    }
}
