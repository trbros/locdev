//! The job executor and scheduler: runs queued jobs (dependency checks and
//! updates, runtime installs) with bounded concurrency and cooperative
//! cancellation, schedules automatic dependency checks, and turns results
//! and process failures into notifications.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use locdev_core::deps::{
    Compatibility, DepStatus, Ecosystem, Job, JobKind, JobProgress, NotificationCategory,
    NotificationLink, Settings, Severity, UpdateKind,
};
use locdev_core::model::{ProcessRole, ProcessState, Project, RuntimeKind, now_ms};
use locdev_deps::analysis;
use locdev_deps::local;
use locdev_deps::registry::{self, FetchError};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use super::Daemon;
use super::deps::Findings;
use crate::fsutil::find_program;
use crate::jobs::{Outcome, Spec};
use crate::notify::New;

/// Automatic checks are spread out instead of starting together.
const STAGGER_MS: u64 = 20_000;
/// A package manager taking longer than this is stopped.
const UPDATE_TIMEOUT: Duration = Duration::from_secs(20 * 60);
/// Lines of a command's output kept in the job log.
const OUTPUT_LINES: usize = 300;

/// Input of a dependency update job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInput {
    pub packages: Vec<String>,
    pub allow_major: bool,
    pub run_scripts: bool,
}

/// Input of a runtime install job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallInput {
    pub kind: RuntimeKind,
    pub version: String,
}

impl Daemon {
    /// Runs forever: takes due jobs up to the concurrency limit, and every
    /// minute schedules the automatic dependency checks that are due.
    pub async fn run_jobs(self: Arc<Self>) {
        let mut schedule = tokio::time::interval(Duration::from_secs(60));
        loop {
            let limit = usize::from(self.settings.get().automation.concurrency);
            while let Some(job) = self.jobs.take_due(now_ms(), limit) {
                let token = CancellationToken::new();
                self.job_tokens
                    .lock()
                    .unwrap()
                    .insert(job.id, token.clone());
                tokio::spawn(self.clone().execute(job, token));
            }
            let wait = self
                .jobs
                .next_due()
                .map(|at| Duration::from_millis(at.saturating_sub(now_ms())))
                .unwrap_or(Duration::from_secs(60))
                .min(Duration::from_secs(60))
                .max(Duration::from_millis(50));
            tokio::select! {
                () = self.job_wake.notified() => {}
                () = tokio::time::sleep(wait) => {}
                _ = schedule.tick() => {
                    self.schedule_audits().await;
                    self.schedule_github().await;
                    self.schedule_definitions();
                    self.schedule_updates();
                    tokio::spawn(self.clone().park_rescan_stale());
                }
            }
        }
    }

    async fn execute(self: Arc<Self>, job: Job, token: CancellationToken) {
        info!(id = job.id, kind = ?job.kind, project = ?job.project, "job started");
        let outcome = match job.kind {
            JobKind::DepsAudit => self.audit(&job, &token).await,
            JobKind::DepsUpdate => self.update(&job, &token).await,
            JobKind::RuntimeInstall => self.install(&job).await,
            JobKind::GithubRuns => match self.job_project(&job).await {
                Some(project) => match self.github_check_runs(&project).await {
                    Ok(summary) => Outcome::Completed(summary),
                    Err(error) => Outcome::Failed {
                        retry: error.code == locdev_core::ErrorCode::Remote,
                        error: error.message,
                        after_ms: None,
                    },
                },
                None => Outcome::Cancelled,
            },
            JobKind::ServiceInstall => match self.run_service_install(&job.input).await {
                Ok(summary) => Outcome::Completed(summary),
                Err(error) => Outcome::Failed {
                    retry: error.code == locdev_core::ErrorCode::Remote,
                    error: error.message,
                    after_ms: None,
                },
            },
            JobKind::TunnelInstall => match self.run_tunnel_install(&job.input).await {
                Ok(summary) => Outcome::Completed(summary),
                Err(error) => Outcome::Failed {
                    retry: false,
                    error: error.message,
                    after_ms: None,
                },
            },
            JobKind::UpdateCheck => match self.run_update_check().await {
                Ok(summary) => Outcome::Completed(summary),
                Err(error) => Outcome::Failed {
                    retry: error.code == locdev_core::ErrorCode::Remote,
                    error: error.message,
                    after_ms: None,
                },
            },
            JobKind::DefinitionsUpdate => match self.refresh_definitions(false).await {
                Ok(summary) => Outcome::Completed(summary),
                Err(error) => Outcome::Failed {
                    retry: true,
                    error: error.message,
                    after_ms: None,
                },
            },
        };
        let outcome = if token.is_cancelled() {
            Outcome::Cancelled
        } else {
            outcome
        };
        let finished = self.jobs.finish(job.id, outcome);
        self.job_tokens.lock().unwrap().remove(&job.id);
        self.job_wake.notify_one();
        if let Some(finished) = finished {
            info!(id = finished.id, status = ?finished.status, "job ended");
            if finished.status == locdev_core::deps::JobStatus::Failed {
                self.record_job_failure(
                    finished.kind.label(),
                    finished.project.as_deref(),
                    finished.error.as_deref().unwrap_or("failed"),
                )
                .await;
            }
            if !finished.automatic && finished.status.finished() {
                self.notify_job(&finished);
            }
        }
    }

    pub(super) fn cancel_job(&self, id: u64) -> bool {
        let running = self.jobs.cancel_where(|j| j.id == id);
        for id in &running {
            if let Some(token) = self.job_tokens.lock().unwrap().get(id) {
                token.cancel();
            }
        }
        self.job_wake.notify_one();
        !running.is_empty() || self.jobs.get(id).is_some()
    }

    /// Turning automatic checks off (or going offline) stops them now: the
    /// queued ones are cancelled and running ones stop at the next package.
    pub(super) fn settings_changed(&self, before: &Settings, after: &Settings) {
        let audits_off = after.automation.audit.every_ms().is_none() || after.network.offline;
        let were_on = before.automation.audit.every_ms().is_some() && !before.network.offline;
        if audits_off && were_on {
            let running = self
                .jobs
                .cancel_where(|j| j.kind == JobKind::DepsAudit && j.automatic);
            for id in running {
                if let Some(token) = self.job_tokens.lock().unwrap().get(&id) {
                    token.cancel();
                }
            }
            info!("automatic dependency checks stopped");
        }
        self.job_wake.notify_one();
    }

    /// Once a day, a check for newer signed framework definitions, unless
    /// offline or turned off.
    fn schedule_definitions(&self) {
        let settings = self.settings.get();
        if settings.network.offline || !settings.automation.definitions {
            return;
        }
        let last = self.definitions.status().checked_at.unwrap_or(0);
        if now_ms().saturating_sub(last) < 24 * 3_600_000 {
            return;
        }
        self.jobs.enqueue(Spec {
            kind: JobKind::DefinitionsUpdate,
            project: None,
            input: serde_json::Value::Null,
            dedup: "definitions".into(),
            automatic: true,
            max_attempts: 3,
            not_before: None,
        });
        self.job_wake.notify_one();
    }

    /// Every 15 minutes, a workflow check per site with a GitHub remote,
    /// when an account is connected and the user wants it.
    async fn schedule_github(&self) {
        let settings = self.settings.get();
        if settings.network.offline || !settings.integrations.github_watch_runs {
            return;
        }
        if !matches!(self.github_account().await, Ok(Some(_))) {
            return;
        }
        let now = now_ms();
        if now.saturating_sub(*self.github_last_check.lock().unwrap()) < 15 * 60_000 {
            return;
        }
        *self.github_last_check.lock().unwrap() = now;
        let projects = self.registry.lock().await.all().to_vec();
        for project in projects {
            let has_github = self
                .with_repo(&project, |repo| repo.status())
                .await
                .is_ok_and(|s| s.github.is_some());
            if has_github {
                self.jobs.enqueue(Spec {
                    kind: JobKind::GithubRuns,
                    project: Some(project.name.clone()),
                    input: serde_json::Value::Null,
                    dedup: format!("runs:{}", project.name),
                    automatic: true,
                    max_attempts: 2,
                    not_before: None,
                });
            }
        }
        self.job_wake.notify_one();
    }

    /// Queues a check for every project whose last one is older than the
    /// chosen interval. Nothing when checks are off/manual or offline.
    async fn schedule_audits(&self) {
        let settings = self.settings.get();
        let Some(every) = settings.automation.audit.every_ms() else {
            return;
        };
        if settings.network.offline {
            return;
        }
        let projects = self.registry.lock().await.all().to_vec();
        let now = now_ms();
        let mut queued = 0;
        for project in projects {
            if !has_manifest(&project.path) {
                continue;
            }
            let last = self.findings(&project.name).checked_at.unwrap_or(0);
            if now.saturating_sub(last) < every {
                continue;
            }
            let (_, created) = self.jobs.enqueue(Spec {
                kind: JobKind::DepsAudit,
                project: Some(project.name.clone()),
                input: serde_json::Value::Null,
                dedup: format!("audit:{}", project.name),
                automatic: true,
                max_attempts: 4,
                not_before: Some(now + queued * STAGGER_MS),
            });
            if created {
                queued += 1;
            }
        }
        if queued > 0 {
            info!(queued, "automatic dependency checks queued");
            self.job_wake.notify_one();
        }
    }

    /// Queues a check now (a user request); returns the job.
    pub(super) fn queue_audit(&self, project: &str) -> Job {
        let (job, _) = self.jobs.enqueue(Spec {
            kind: JobKind::DepsAudit,
            project: Some(project.to_string()),
            input: serde_json::Value::Null,
            dedup: format!("audit:{project}"),
            automatic: false,
            max_attempts: 3,
            not_before: None,
        });
        self.job_wake.notify_one();
        job
    }

    /// Asks the registries about every declared package, then OSV about the
    /// versions in use. Stops between packages when cancelled, or when an
    /// automatic check was turned off meanwhile.
    async fn audit(&self, job: &Job, token: &CancellationToken) -> Outcome {
        let Some(project) = self.job_project(job).await else {
            return Outcome::Failed {
                error: "the site no longer exists".into(),
                retry: false,
                after_ms: None,
            };
        };
        let path = project.path.clone();
        let local = tokio::task::spawn_blocking(move || local::read(&path))
            .await
            .unwrap_or_default();
        let total = local.packages.len() as u64;
        let stop = || {
            let s = self.settings.get();
            token.is_cancelled()
                || (job.automatic && (s.automation.audit.every_ms().is_none() || s.network.offline))
        };

        for (i, dep) in local.packages.iter().enumerate() {
            if stop() {
                return Outcome::Cancelled;
            }
            self.jobs.progress(
                job.id,
                JobProgress {
                    done: i as u64,
                    total: Some(total),
                    label: dep.name.clone(),
                },
            );
            // Git/path/workspace dependencies aren't in a registry.
            let registry_range = match dep.ecosystem {
                Ecosystem::Npm => locdev_core::semver::parse_npm(&dep.constraint).is_ok(),
                Ecosystem::Composer => locdev_core::semver::parse_composer(&dep.constraint).is_ok(),
            };
            if !registry_range {
                continue;
            }
            match self.package_info(dep.ecosystem, &dep.name).await {
                Ok(_) | Err(FetchError::NotFound) | Err(FetchError::Invalid(_)) => {}
                Err(FetchError::RateLimited(after)) => {
                    self.registry_cache.save();
                    return Outcome::Failed {
                        error: "the registry is rate limiting requests".into(),
                        retry: true,
                        after_ms: Some(after.unwrap_or(300) * 1000),
                    };
                }
                Err(error @ FetchError::Network(_)) => {
                    self.registry_cache.save();
                    return Outcome::Failed {
                        error: error.to_string(),
                        retry: true,
                        after_ms: None,
                    };
                }
            }
        }
        if stop() {
            return Outcome::Cancelled;
        }

        // Advisories for the versions actually in use, in one batch.
        let queries: Vec<(Ecosystem, String, String)> = local
            .packages
            .iter()
            .filter_map(|d| {
                Some((
                    d.ecosystem,
                    d.name.clone(),
                    analysis::current(d)?.to_string(),
                ))
            })
            .collect();
        self.limiter
            .wait(self.settings.get().automation.requests_per_second)
            .await;
        let batch = queries.clone();
        let found = match tokio::task::spawn_blocking(move || registry::advisories(&batch)).await {
            Ok(Ok(found)) => found,
            Ok(Err(error)) => {
                self.jobs
                    .log(job.id, format!("advisories unavailable: {error}"));
                self.registry_cache.save();
                return Outcome::Failed {
                    error: format!("osv.dev: {error}"),
                    retry: error.transient(),
                    after_ms: None,
                };
            }
            Err(error) => {
                return Outcome::Failed {
                    error: error.to_string(),
                    retry: false,
                    after_ms: None,
                };
            }
        };
        let mut findings = Findings {
            checked_at: Some(now_ms()),
            ..Default::default()
        };
        for ((eco, name, version), list) in queries.into_iter().zip(found) {
            if !list.is_empty() {
                findings
                    .advisories
                    .insert(format!("{}:{name}", eco.label()), (version, list));
            }
        }
        self.save_findings(&project.name, &findings);
        self.registry_cache.save();

        let report = self.deps_report(&project).await;
        let outdated: Vec<_> = report
            .packages
            .iter()
            .filter(|p| {
                p.status == DepStatus::Outdated && p.compatibility != Compatibility::Incompatible
            })
            .collect();
        let vulnerable: Vec<_> = report
            .packages
            .iter()
            .filter(|p| p.status == DepStatus::Vulnerable)
            .collect();
        self.notify_audit(&project, &outdated, &vulnerable);
        Outcome::Completed(format!(
            "{total} package(s) checked: {} with updates, {} with known vulnerabilities",
            report
                .packages
                .iter()
                .filter(|p| p.update != UpdateKind::None)
                .count(),
            vulnerable.len()
        ))
    }

    fn notify_audit(
        &self,
        project: &Project,
        outdated: &[&locdev_core::deps::PackageReport],
        vulnerable: &[&locdev_core::deps::PackageReport],
    ) {
        let link = Some(NotificationLink {
            view: "sites".into(),
            sub: Some("dependencies".into()),
            project: Some(project.name.clone()),
        });
        if !outdated.is_empty() {
            // One notification per set of new versions: the same updates
            // never notify twice, a newer release does.
            let set: BTreeSet<String> = outdated
                .iter()
                .map(|p| format!("{}@{}", p.name, p.latest.as_deref().unwrap_or("")))
                .collect();
            let lines: Vec<String> = outdated
                .iter()
                .take(4)
                .map(|p| {
                    format!(
                        "{} {} → {}",
                        p.name,
                        p.installed
                            .as_deref()
                            .or(p.locked.as_deref())
                            .unwrap_or("?"),
                        p.latest.as_deref().unwrap_or("?")
                    )
                })
                .collect();
            self.notify(New {
                category: NotificationCategory::DependencyUpdate,
                severity: Severity::Info,
                title: format!(
                    "{} update{} for {}",
                    outdated.len(),
                    if outdated.len() == 1 { "" } else { "s" },
                    project.domain
                ),
                body: lines.join("\n") + if outdated.len() > 4 { "\n…" } else { "" },
                project: Some(project.name.clone()),
                link: link.clone(),
                dedup: format!(
                    "deps:{}:{}",
                    project.name,
                    fnv(&set.into_iter().collect::<Vec<_>>().join(","))
                ),
            });
        }
        for package in vulnerable {
            for advisory in &package.advisories {
                self.notify(New {
                    category: NotificationCategory::Security,
                    severity: Severity::Error,
                    title: format!("{} in {} ({})", advisory.id, package.name, project.domain),
                    body: match &advisory.fixed {
                        Some(fixed) => format!("{}\nFixed in {fixed}.", advisory.summary),
                        None => advisory.summary.clone(),
                    },
                    project: Some(project.name.clone()),
                    link: link.clone(),
                    dedup: format!("sec:{}:{}:{}", project.name, package.name, advisory.id),
                });
            }
        }
    }

    /// Runs the planned package manager commands. The plan is recomputed
    /// from the same inputs the user confirmed.
    async fn update(&self, job: &Job, token: &CancellationToken) -> Outcome {
        let fail = |error: String| Outcome::Failed {
            error,
            retry: false,
            after_ms: None,
        };
        let Some(project) = self.job_project(job).await else {
            return fail("the site no longer exists".into());
        };
        let Ok(input) = serde_json::from_value::<UpdateInput>(job.input.clone()) else {
            return fail("invalid job input".into());
        };
        let plan = match self
            .plan_update(
                &project,
                &input.packages,
                input.allow_major,
                input.run_scripts,
            )
            .await
        {
            Ok(plan) => plan,
            Err(error) => return fail(error.message),
        };
        if plan.commands.is_empty() {
            return Outcome::Completed("nothing to update".into());
        }
        let env = match self.runtimes.env_for(&project) {
            Ok(env) => env,
            Err(error) => return fail(error.message),
        };
        let path_var = env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        let before = snapshot(&project.path, &plan.files);
        let path = project.path.clone();
        let local_before = tokio::task::spawn_blocking(move || local::read(&path))
            .await
            .unwrap_or_default();
        let berry = local_before.manager == Some(RuntimeKind::Yarn)
            && locdev_deps::update::yarn_berry(&project.path);
        let manager = local_before.manager.unwrap_or(RuntimeKind::Npm);
        let picks: Vec<_> = plan
            .changes
            .iter()
            .filter_map(|c| {
                let dep = local_before.packages.iter().find(|d| d.name == c.name)?;
                Some(locdev_deps::update::Pick {
                    name: c.name.clone(),
                    ecosystem: c.ecosystem,
                    kind: dep.kind,
                    declared: dep.constraint.clone(),
                    target: c.to.clone(),
                    within_range: c.constraint.is_none(),
                })
            })
            .collect();
        let commands = locdev_deps::update::commands(manager, berry, &picks, input.run_scripts);

        for (i, command) in commands.iter().enumerate() {
            self.jobs.progress(
                job.id,
                JobProgress {
                    done: i as u64,
                    total: Some(commands.len() as u64),
                    label: command.argv.join(" "),
                },
            );
            self.jobs
                .log(job.id, format!("$ {}", command.argv.join(" ")));
            let result = run_command(
                &command.argv,
                &command.env,
                &env,
                &path_var,
                &project.path,
                token,
                |line| self.jobs.log(job.id, line),
            )
            .await;
            if let Err(error) = result {
                return if token.is_cancelled() {
                    Outcome::Cancelled
                } else {
                    fail(error)
                };
            }
        }

        let after = snapshot(&project.path, &plan.files);
        let changed: Vec<&String> = plan
            .files
            .iter()
            .zip(before.iter().zip(after.iter()))
            .filter(|(_, (b, a))| b != a)
            .map(|(f, _)| f)
            .collect();
        Outcome::Completed(format!(
            "updated {}; changed {}",
            plan.changes
                .iter()
                .map(|c| format!("{} {}", c.name, c.to))
                .collect::<Vec<_>>()
                .join(", "),
            if changed.is_empty() {
                "no files".to_string()
            } else {
                changed
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ))
    }

    async fn install(&self, job: &Job) -> Outcome {
        let Ok(input) = serde_json::from_value::<InstallInput>(job.input.clone()) else {
            return Outcome::Failed {
                error: "invalid job input".into(),
                retry: false,
                after_ms: None,
            };
        };
        let runtimes = self.runtimes.clone();
        let (kind, version) = (input.kind, input.version.clone());
        let result =
            tokio::task::spawn_blocking(move || runtimes.run_install(kind, &version)).await;
        for line in self.runtimes.install_log(input.kind) {
            self.jobs.log(job.id, line);
        }
        match result {
            Ok(Ok(())) => Outcome::Completed(format!(
                "installed {} {} (verified)",
                input.kind.label(),
                input.version
            )),
            Ok(Err(error)) => Outcome::Failed {
                error,
                retry: false,
                after_ms: None,
            },
            Err(error) => Outcome::Failed {
                error: error.to_string(),
                retry: false,
                after_ms: None,
            },
        }
    }

    async fn job_project(&self, job: &Job) -> Option<Project> {
        let name = job.project.as_deref()?;
        self.registry.lock().await.get(name).ok().cloned()
    }

    /// Adds a notification and, when native ones are on, shows it.
    pub(super) fn notify(&self, new: New) {
        let settings = self.settings.get().notifications;
        if self.notifications.push(new, &settings).is_some() && settings.native {
            let notifications = self.notifications.clone();
            let front = self.app_in_front();
            tokio::spawn(async move { notifications.deliver(&settings, front).await });
        }
    }

    fn notify_job(&self, job: &Job) {
        let ok = job.status == locdev_core::deps::JobStatus::Completed;
        let what = job.kind.label();
        let site = job
            .project
            .as_deref()
            .map(|p| format!(" for {p}"))
            .unwrap_or_default();
        self.notify(New {
            category: if ok {
                NotificationCategory::Action
            } else {
                NotificationCategory::Job
            },
            severity: if ok {
                Severity::Info
            } else if job.status == locdev_core::deps::JobStatus::Cancelled {
                Severity::Warn
            } else {
                Severity::Error
            },
            title: format!(
                "{what}{site} {}",
                match job.status {
                    locdev_core::deps::JobStatus::Completed => "finished",
                    locdev_core::deps::JobStatus::Cancelled => "was cancelled",
                    _ => "failed",
                }
            ),
            body: job.result.clone().or(job.error.clone()).unwrap_or_default(),
            project: job.project.clone(),
            link: Some(NotificationLink {
                view: if job.kind == JobKind::RuntimeInstall {
                    "runtimes"
                } else {
                    "sites"
                }
                .into(),
                sub: (job.kind != JobKind::RuntimeInstall).then(|| "dependencies".into()),
                project: job.project.clone(),
            }),
            dedup: format!("job:{}", job.id),
        });
    }

    /// Notifies when a dev server stops with an error.
    pub async fn watch_processes(self: Arc<Self>) {
        let mut events = self.supervisor.subscribe();
        loop {
            let (id, info) = match events.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            };
            if info.state != ProcessState::Failed {
                continue;
            }
            let project = id.split(':').next().unwrap_or(&id).to_string();
            let helper = id.ends_with(":dev");
            let domain = self
                .registry
                .lock()
                .await
                .get(&project)
                .map(|p| p.domain.clone())
                .unwrap_or_else(|_| project.clone());
            self.notify(New {
                category: NotificationCategory::ProcessFailure,
                severity: Severity::Error,
                title: format!(
                    "{}{} stopped with an error",
                    domain,
                    if helper { " (helper)" } else { "" }
                ),
                body: info
                    .message
                    .clone()
                    .unwrap_or_else(|| "see its output".into()),
                project: Some(project.clone()),
                link: Some(NotificationLink {
                    view: "sites".into(),
                    sub: Some("server".into()),
                    project: Some(project.clone()),
                }),
                dedup: format!(
                    "fail:{}:{}",
                    if helper {
                        ProcessRole::Dev.id(&project)
                    } else {
                        project.clone()
                    },
                    info.started_at.unwrap_or_default()
                ),
            });
        }
    }
}

fn has_manifest(path: &Path) -> bool {
    path.join("package.json").is_file() || path.join("composer.json").is_file()
}

/// A short stable hash for notification keys.
fn fnv(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Contents of `files`, to tell which ones a command changed.
fn snapshot(root: &Path, files: &[String]) -> Vec<Option<Vec<u8>>> {
    files
        .iter()
        .map(|f| std::fs::read(root.join(f)).ok())
        .collect()
}

/// Runs one command without a shell, streaming its output to `log`.
async fn run_command(
    argv: &[String],
    extra_env: &[(String, String)],
    env: &[(String, String)],
    path_var: &str,
    cwd: &Path,
    token: &CancellationToken,
    log: impl Fn(String),
) -> Result<(), String> {
    let (program, args) = argv.split_first().ok_or("empty command")?;
    let executable = find_program(program, path_var)
        .ok_or_else(|| format!("{program} was not found on the site's PATH"))?;
    let mut child = tokio::process::Command::from(locdev_core::process::command(executable))
        .args(args)
        .current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .envs(extra_env.iter().map(|(k, v)| (k, v)))
        .env("CI", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not run {program}: {e}"))?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(line);
            }
        });
    }
    drop(tx);

    let mut kept = 0;
    let wait = async {
        loop {
            tokio::select! {
                line = rx.recv() => match line {
                    Some(line) => {
                        if kept < OUTPUT_LINES {
                            log(line);
                            kept += 1;
                        }
                    }
                    None => break child.wait().await,
                },
            }
        }
    };
    let status = tokio::select! {
        status = tokio::time::timeout(UPDATE_TIMEOUT, wait) => match status {
            Ok(status) => status.map_err(|e| e.to_string())?,
            Err(_) => return Err(format!("{program} took longer than {} minutes", UPDATE_TIMEOUT.as_secs() / 60)),
        },
        () = token.cancelled() => return Err("cancelled".into()),
    };
    if status.success() {
        Ok(())
    } else {
        warn!(%program, ?status, "update command failed");
        Err(format!("{} exited with {status}", argv.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_keys_are_stable() {
        assert_eq!(fnv("react@19.0.0"), fnv("react@19.0.0"));
        assert_ne!(fnv("react@19.0.0"), fnv("react@19.0.1"));
    }

    #[cfg(unix)]
    #[test]
    fn finds_programs_on_a_given_path() {
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join("npm");
        std::fs::write(&tool, "#!/bin/sh\n").unwrap();
        let path = std::env::join_paths([dir.path()])
            .unwrap()
            .into_string()
            .unwrap();
        assert_eq!(find_program("npm", &path), Some(tool));
        assert_eq!(find_program("pnpm", &path), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runs_without_a_shell_and_can_be_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::env::var("PATH").unwrap();
        let lines = std::sync::Mutex::new(Vec::new());
        let ok = run_command(
            &["echo".into(), "a; rm -rf /".into()],
            &[],
            &[],
            &path,
            dir.path(),
            &CancellationToken::new(),
            |l| lines.lock().unwrap().push(l),
        )
        .await;
        assert!(ok.is_ok());
        // The argument reached echo as one literal string: no shell parsed it.
        assert_eq!(lines.lock().unwrap().as_slice(), ["a; rm -rf /"]);

        let token = CancellationToken::new();
        token.cancel();
        let cancelled = run_command(
            &["sleep".into(), "30".into()],
            &[],
            &[],
            &path,
            dir.path(),
            &token,
            |_| {},
        )
        .await;
        assert_eq!(cancelled.unwrap_err(), "cancelled");
    }
}
