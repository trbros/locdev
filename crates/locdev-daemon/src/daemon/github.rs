//! A site's GitHub repository through the connected account. Local Git and
//! GitHub stay apart: these calls go to api.github.com and never touch the
//! repository on disk. Every change on GitHub is a request the user made.

use std::sync::Mutex;

use locdev_core::deps::{NotificationCategory, NotificationLink, Severity};
use locdev_core::git::{
    DeviceCode, GithubAccount, GithubAction, GithubAuthMethod, GithubData, GithubRepo,
    GithubSection,
};
use locdev_core::model::Project;
use locdev_core::{AppError, ErrorCode};
use locdev_github::{Client, GithubError, Token, device};

use super::Daemon;
use crate::notify::New;

/// What the daemon remembers between calls: the account (not the token,
/// which stays in the keyring) and a device flow in progress.
#[derive(Default)]
pub struct GithubState {
    pub account: Mutex<Option<GithubAccount>>,
    pub device: Mutex<Option<device::Pending>>,
    /// Ids of failed runs already notified, per repository.
    pub seen_runs: Mutex<std::collections::HashMap<String, u64>>,
}

pub fn app_error(error: GithubError) -> AppError {
    let code = match error {
        GithubError::Unauthorized(_) => ErrorCode::Unauthorized,
        GithubError::NotFound(_) => ErrorCode::NotFound,
        _ => ErrorCode::Remote,
    };
    AppError::new(code, error.to_string())
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, GithubError> + Send + 'static,
) -> Result<T, AppError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(app_error)
}

impl Daemon {
    /// The connected account, read once from the keyring and remembered.
    pub(super) async fn github_account(&self) -> Result<Option<GithubAccount>, AppError> {
        if let Some(account) = self.github.account.lock().unwrap().clone() {
            return Ok(Some(account));
        }
        let Some((token, method)) = blocking(locdev_github::load).await? else {
            return Ok(None);
        };
        let account = blocking(move || Client::new(token).account(method)).await?;
        *self.github.account.lock().unwrap() = Some(account.clone());
        Ok(Some(account))
    }

    /// Checks `token` against GitHub, then keeps it in the keyring.
    pub(super) async fn github_connect(
        &self,
        token: Token,
        method: GithubAuthMethod,
    ) -> Result<GithubAccount, AppError> {
        let check = token.clone();
        let account = blocking(move || Client::new(check).account(method)).await?;
        blocking(move || locdev_github::store(&token, method)).await?;
        *self.github.account.lock().unwrap() = Some(account.clone());
        tracing::info!(login = %account.login, ?method, "GitHub account connected");
        Ok(account)
    }

    pub(super) async fn github_disconnect(&self) -> Result<(), AppError> {
        blocking(locdev_github::forget).await?;
        *self.github.account.lock().unwrap() = None;
        *self.github.device.lock().unwrap() = None;
        Ok(())
    }

    pub(super) async fn github_device_start(&self) -> Result<DeviceCode, AppError> {
        let client_id = self.settings.get().integrations.github_client_id;
        if client_id.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                "the device flow needs a GitHub OAuth App client id (Settings → Integrations); \
                 connect with a token or the GitHub CLI instead",
            ));
        }
        let pending = blocking(move || device::start(client_id.trim())).await?;
        let code = pending.code.clone();
        *self.github.device.lock().unwrap() = Some(pending);
        Ok(code)
    }

    /// `Ok(None)` while the user hasn't approved yet.
    pub(super) async fn github_device_poll(&self) -> Result<Option<GithubAccount>, AppError> {
        let Some(pending) = self.github.device.lock().unwrap().take() else {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "no device authorization in progress",
            ));
        };
        let (pending, result) = tokio::task::spawn_blocking(move || {
            let result = device::poll(&pending);
            (pending, result)
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
        match result.map_err(app_error)? {
            device::Poll::Done(token) => Ok(Some(
                self.github_connect(token, GithubAuthMethod::Device).await?,
            )),
            device::Poll::Pending | device::Poll::SlowDown => {
                *self.github.device.lock().unwrap() = Some(pending);
                Ok(None)
            }
        }
    }

    async fn github_client(&self) -> Result<Token, AppError> {
        match blocking(locdev_github::load).await? {
            Some((token, _)) => Ok(token),
            None => Err(AppError::new(
                ErrorCode::Unauthorized,
                "no GitHub account connected (Settings → Integrations)",
            )),
        }
    }

    pub(super) async fn github_repo(&self, project: &Project) -> Result<GithubRepo, AppError> {
        let status = self.with_repo(project, |repo| repo.status()).await?;
        status
            .github
            .ok_or_else(|| AppError::not_found(format!("{} has no GitHub remote", project.name)))
    }

    pub(super) async fn github_data(
        &self,
        project: &Project,
        section: GithubSection,
    ) -> Result<GithubData, AppError> {
        let repo = self.github_repo(project).await?;
        let token = self.github_client().await?;
        let result = blocking(move || {
            let client = Client::new(token);
            Ok(match section {
                GithubSection::Overview => GithubData::Overview(client.overview(&repo)?),
                GithubSection::Issues => GithubData::Issues {
                    items: client.issues(&repo)?,
                },
                GithubSection::Pulls => GithubData::Pulls {
                    items: client.pulls(&repo)?,
                },
                GithubSection::Workflows => GithubData::Workflows {
                    workflows: client.workflows(&repo)?,
                },
                GithubSection::Runs => GithubData::Runs {
                    runs: client.runs(&repo, 30)?,
                },
                GithubSection::Releases => GithubData::Releases {
                    releases: client.releases(&repo)?,
                },
                GithubSection::Branches => GithubData::Branches {
                    names: client.branches(&repo)?,
                },
                GithubSection::Commits => {
                    let branch = client.overview(&repo)?.default_branch;
                    GithubData::Commits {
                        commits: client.commits(&repo, &branch)?,
                    }
                }
                GithubSection::Checks => {
                    let branch = client.overview(&repo)?.default_branch;
                    let sha = client
                        .commits(&repo, &branch)?
                        .first()
                        .map(|c| c.sha.clone())
                        .unwrap_or_default();
                    GithubData::Checks {
                        checks: client.checks(&repo, &sha)?,
                        sha,
                    }
                }
            })
        })
        .await;
        if matches!(&result, Err(e) if e.code == ErrorCode::Unauthorized) {
            // An expired or revoked token: forget the cached account.
            *self.github.account.lock().unwrap() = None;
        }
        result
    }

    pub(super) async fn github_action(
        &self,
        project: &Project,
        action: GithubAction,
    ) -> Result<String, AppError> {
        let repo = self.github_repo(project).await?;
        let token = self.github_client().await?;
        let message = match &action {
            GithubAction::RerunFailed { run_id } => {
                format!("re-running the failed jobs of run {run_id}")
            }
            GithubAction::CancelRun { run_id } => format!("cancelling run {run_id}"),
            GithubAction::Dispatch { git_ref, .. } => format!("workflow started on {git_ref}"),
        };
        tracing::info!(project = %project.name, ?action, "GitHub action");
        blocking(move || {
            let client = Client::new(token);
            match action {
                GithubAction::RerunFailed { run_id } => client.rerun_failed(&repo, run_id),
                GithubAction::CancelRun { run_id } => client.cancel_run(&repo, run_id),
                GithubAction::Dispatch {
                    workflow_id,
                    git_ref,
                } => {
                    if git_ref.is_empty()
                        || git_ref.starts_with('-')
                        || git_ref.contains(char::is_whitespace)
                    {
                        return Err(GithubError::Other(format!(
                            "'{git_ref}' is not a branch or tag"
                        )));
                    }
                    client.dispatch(&repo, workflow_id, &git_ref)
                }
            }
        })
        .await?;
        Ok(message)
    }

    /// Notifies about workflow runs that failed since the last look.
    pub(super) async fn github_check_runs(&self, project: &Project) -> Result<String, AppError> {
        let repo = self.github_repo(project).await?;
        let token = self.github_client().await?;
        let slug = repo.slug();
        let runs = blocking(move || Client::new(token).runs(&repo, 20)).await?;
        let failed: Vec<_> = runs
            .iter()
            .filter(|r| {
                r.status == "completed"
                    && matches!(r.conclusion.as_deref(), Some("failure" | "timed_out"))
            })
            .collect();
        let seen_path = self.paths.data_dir.join("state").join("github-runs.json");
        if self.github.seen_runs.lock().unwrap().is_empty() {
            // Survives restarts, so failures while the machine was off still notify.
            if let Some(saved) = std::fs::read(&seen_path)
                .ok()
                .and_then(|raw| serde_json::from_slice(&raw).ok())
            {
                *self.github.seen_runs.lock().unwrap() = saved;
            }
        }
        let last_seen = self.github.seen_runs.lock().unwrap().get(&slug).copied();
        let newest = runs.iter().map(|r| r.id).max().unwrap_or(0);
        // The first look only sets the baseline: old failures aren't news.
        if let Some(last_seen) = last_seen {
            for run in failed.iter().filter(|r| r.id > last_seen) {
                self.notify(New {
                    category: NotificationCategory::Workflow,
                    severity: Severity::Error,
                    title: format!("{} failed on {} ({slug})", run.name, run.branch),
                    body: format!("{} · {}", run.event, &run.sha[..run.sha.len().min(7)]),
                    project: Some(project.name.clone()),
                    link: Some(NotificationLink {
                        view: "sites".into(),
                        sub: Some("github".into()),
                        project: Some(project.name.clone()),
                    }),
                    dedup: format!("wf:{slug}:{}", run.id),
                });
            }
        }
        let snapshot = {
            let mut seen = self.github.seen_runs.lock().unwrap();
            seen.insert(slug, newest.max(last_seen.unwrap_or(0)));
            seen.clone()
        };
        if let Ok(raw) = serde_json::to_vec(&snapshot) {
            let _ = crate::fsutil::write_atomic(&seen_path, &raw);
        }
        Ok(format!("{} run(s), {} failed", runs.len(), failed.len()))
    }
}
