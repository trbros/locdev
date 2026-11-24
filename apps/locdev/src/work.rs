//! `locdev deps …`, `locdev jobs …`, `locdev notifications …` and
//! `locdev settings …`: dependency checks and updates, the job queue they
//! run on, notifications and the settings that drive them.

use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use clap::Subcommand;
use locdev_core::deps::{
    AuditInterval, DepStatus, EnvConflictPolicy, Job, JobStatus, NotificationCategory, Settings,
    UpdateKind,
};
use locdev_core::paths::Paths;
use locdev_core::protocol::{JobAction, Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum DepsCommand {
    /// Every declared package: declared, locked, installed, newest.
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// Only packages with a newer release.
    Outdated { site: Option<String> },
    /// Ask the registries now (queues a job and waits for it).
    Check {
        site: Option<String>,
        /// Return right away instead of waiting.
        #[arg(long)]
        no_wait: bool,
    },
    /// Check, then show packages with known vulnerabilities.
    Audit { site: Option<String> },
    /// Update packages with the site's package manager, after showing the plan.
    Update {
        /// Package names.
        #[arg(required = true)]
        packages: Vec<String>,
        #[arg(long)]
        site: Option<String>,
        /// Allow a new major version (rewrites the constraint).
        #[arg(long)]
        major: bool,
        /// Let the package manager run install scripts (off by default).
        #[arg(long)]
        scripts: bool,
        /// Don't ask for confirmation.
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum JobsCommand {
    /// Recent jobs, newest first.
    #[command(visible_alias = "ls")]
    List {
        #[arg(long)]
        site: Option<String>,
    },
    /// One job with its log.
    Status { id: u64 },
    /// Stop a queued or running job.
    Cancel { id: u64 },
    /// Hold a queued job.
    Pause { id: u64 },
    /// Release a held job.
    Resume { id: u64 },
    /// Queue a failed or cancelled job again.
    Retry { id: u64 },
}

#[derive(Subcommand)]
pub(crate) enum NotificationsCommand {
    /// Unread and read notifications (`--all` adds dismissed ones).
    #[command(visible_alias = "ls")]
    List {
        #[arg(long)]
        all: bool,
    },
    /// Mark notifications read (all when no ids are given).
    Read { ids: Vec<u64> },
    /// Dismiss notifications (all when no ids are given).
    Dismiss { ids: Vec<u64> },
    /// Show a test notification.
    Test,
}

#[derive(Subcommand)]
pub(crate) enum SettingsCommand {
    /// Automation, notification and network settings.
    Show,
    /// Change one setting, e.g. `automation.audit weekly` or `network.offline true`.
    Set { key: String, value: String },
}

pub async fn deps(command: DepsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DepsCommand::List { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            out.deps(&list(paths, &name).await?, false);
        }
        DepsCommand::Outdated { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            out.deps(&list(paths, &name).await?, true);
        }
        DepsCommand::Check { site, no_wait } => {
            let name = site::resolve(paths, site).await?.project.name;
            let job = queue_check(paths, &name).await?;
            if no_wait {
                out.job(&job);
                return Ok(ExitCode::SUCCESS);
            }
            let job = wait(paths, job.id, out).await?;
            if job.status != JobStatus::Completed {
                bail!(
                    "{}",
                    job.error
                        .unwrap_or_else(|| "the check did not finish".into())
                );
            }
            out.deps(&list(paths, &name).await?, true);
        }
        DepsCommand::Audit { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let job = wait(paths, queue_check(paths, &name).await?.id, out).await?;
            if job.status != JobStatus::Completed {
                bail!(
                    "{}",
                    job.error
                        .unwrap_or_else(|| "the check did not finish".into())
                );
            }
            let report = list(paths, &name).await?;
            out.advisories(&report);
            if report
                .packages
                .iter()
                .any(|p| p.status == DepStatus::Vulnerable)
            {
                return Ok(ExitCode::FAILURE);
            }
        }
        DepsCommand::Update {
            packages,
            site,
            major,
            scripts,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let plan = match daemon_call(
                paths,
                Request::PlanDepsUpdate {
                    name: name.clone(),
                    packages: packages.clone(),
                    allow_major: major,
                    run_scripts: scripts,
                },
            )
            .await?
            {
                Response::DepsPlan(plan) => plan,
                other => bail!("unexpected response from daemon: {other:?}"),
            };
            out.update_plan(&plan);
            if plan.commands.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            if !yes {
                confirm("Run these commands?")?;
            }
            let request = Request::UpdateDeps {
                name,
                packages,
                allow_major: major,
                run_scripts: scripts,
            };
            let Response::Job { job } = daemon_call(paths, request).await? else {
                bail!("unexpected response from daemon");
            };
            let job = wait(paths, job.id, out).await?;
            out.job(&job);
            if job.status != JobStatus::Completed {
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn list(paths: &Paths, name: &str) -> anyhow::Result<locdev_core::deps::DepsReport> {
    match daemon_call(
        paths,
        Request::ListDeps {
            name: name.to_string(),
        },
    )
    .await?
    {
        Response::Deps(report) => Ok(report),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

async fn queue_check(paths: &Paths, name: &str) -> anyhow::Result<Job> {
    match daemon_call(
        paths,
        Request::CheckDeps {
            name: name.to_string(),
        },
    )
    .await?
    {
        Response::Job { job } => Ok(job),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

/// Polls a job until it finishes, drawing its progress on a terminal.
pub(crate) async fn wait(paths: &Paths, id: u64, out: &Output) -> anyhow::Result<Job> {
    let tty = std::io::stderr().is_terminal() && !out.is_json();
    let mut printed = 0;
    loop {
        let Response::Job { job } = daemon_call(paths, Request::GetJob { id }).await? else {
            bail!("unexpected response from daemon");
        };
        if tty {
            // New log lines (command output) as they come, then a status line.
            for line in job.log.iter().skip(printed) {
                eprintln!("\r\x1b[2K  {line}");
            }
            printed = job.log.len();
            let progress = job
                .progress
                .as_ref()
                .map(|p| match p.total {
                    Some(total) => format!("{}/{total} {}", p.done, p.label),
                    None => p.label.clone(),
                })
                .unwrap_or_else(|| format!("{:?}", job.status).to_lowercase());
            eprint!("\r\x1b[2K{} · {progress}", job.kind.label());
            std::io::stderr().flush()?;
        }
        if job.status.finished() {
            if tty {
                eprintln!("\r\x1b[2K");
            }
            return Ok(job);
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}

pub(crate) fn confirm(question: &str) -> anyhow::Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!("refusing to continue without confirmation; re-run with --yes");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    if !matches!(answer.trim(), "y" | "Y" | "yes") {
        bail!("cancelled; nothing was changed");
    }
    Ok(())
}

pub async fn jobs(command: JobsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let control = |id, action| Request::ControlJob { id, action };
    let request = match command {
        JobsCommand::List { site } => {
            let project = match site {
                Some(name) => Some(site::resolve(paths, Some(name)).await?.project.name),
                None => None,
            };
            match daemon_call(paths, Request::ListJobs { project }).await? {
                Response::Jobs { jobs } => out.jobs(&jobs),
                other => bail!("unexpected response from daemon: {other:?}"),
            }
            return Ok(ExitCode::SUCCESS);
        }
        JobsCommand::Status { id } => Request::GetJob { id },
        JobsCommand::Cancel { id } => control(id, JobAction::Cancel),
        JobsCommand::Pause { id } => control(id, JobAction::Pause),
        JobsCommand::Resume { id } => control(id, JobAction::Resume),
        JobsCommand::Retry { id } => control(id, JobAction::Retry),
    };
    match daemon_call(paths, request).await? {
        Response::Job { job } => out.job(&job),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

pub async fn notifications(
    command: NotificationsCommand,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    match command {
        NotificationsCommand::List { all } => {
            match daemon_call(paths, Request::ListNotifications { all }).await? {
                Response::Notifications {
                    notifications,
                    unread,
                } => out.notifications(&notifications, unread),
                other => bail!("unexpected response from daemon: {other:?}"),
            }
        }
        NotificationsCommand::Read { ids } => {
            daemon_call(
                paths,
                Request::MarkNotifications {
                    ids,
                    read: Some(true),
                    dismissed: None,
                },
            )
            .await?;
            out.message("marked as read");
        }
        NotificationsCommand::Dismiss { ids } => {
            daemon_call(
                paths,
                Request::MarkNotifications {
                    ids,
                    read: None,
                    dismissed: Some(true),
                },
            )
            .await?;
            out.message("dismissed");
        }
        NotificationsCommand::Test => {
            daemon_call(paths, Request::TestNotification).await?;
            out.message("sent; see `locdev notifications` (and your desktop, if native notifications are on)");
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub async fn settings(
    command: SettingsCommand,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let Response::Settings(mut settings) = daemon_call(paths, Request::GetSettings).await? else {
        bail!("unexpected response from daemon");
    };
    if let SettingsCommand::Set { key, value } = command {
        apply(&mut settings, &key, &value)?;
        let Response::Settings(saved) =
            daemon_call(paths, Request::UpdateSettings { settings }).await?
        else {
            bail!("unexpected response from daemon");
        };
        settings = saved;
    }
    out.settings(&settings);
    Ok(ExitCode::SUCCESS)
}

/// Sets one dotted key from text.
fn apply(settings: &mut Settings, key: &str, value: &str) -> anyhow::Result<()> {
    let flag = || -> anyhow::Result<bool> {
        match value {
            "true" | "on" | "yes" | "1" => Ok(true),
            "false" | "off" | "no" | "0" => Ok(false),
            other => bail!("'{other}' is not on/off"),
        }
    };
    match key {
        "automation.audit" => {
            settings.automation.audit = match value {
                "off" => AuditInterval::Off,
                "manual" => AuditInterval::Manual,
                "6h" => AuditInterval::SixHours,
                "12h" => AuditInterval::TwelveHours,
                "daily" => AuditInterval::Daily,
                "weekly" => AuditInterval::Weekly,
                other => bail!("'{other}': use off, manual, 6h, 12h, daily or weekly"),
            }
        }
        "automation.concurrency" => {
            settings.automation.concurrency = value.parse().context("a number from 1 to 8")?
        }
        "automation.requests_per_second" => {
            settings.automation.requests_per_second =
                value.parse().context("a number from 1 to 50")?
        }
        "notifications.native" => settings.notifications.native = flag()?,
        "automation.definitions" => settings.automation.definitions = flag()?,
        "network.offline" => settings.network.offline = flag()?,
        "automation.updates" => settings.automation.updates = flag()?,
        "observe.enabled" => settings.observe.enabled = flag()?,
        "observe.retention_days" => {
            settings.observe.retention_days = value.parse().context("a number of days")?
        }
        "observe.max_mb" => settings.observe.max_mb = value.parse().context("a size in MB")?,
        "observe.logs_from" => {
            settings.observe.logs_from = match value {
                "warn" | "warning" => locdev_core::observe::EventSeverity::Warn,
                "error" => locdev_core::observe::EventSeverity::Error,
                "info" => locdev_core::observe::EventSeverity::Info,
                other => bail!("'{other}': use info, warn or error"),
            }
        }
        "env.conflicts" => {
            settings.env.conflicts = match value {
                "active_files" => EnvConflictPolicy::ActiveFiles,
                "any_file" => EnvConflictPolicy::AnyFile,
                other => bail!("'{other}': use active_files or any_file"),
            }
        }
        other => {
            let Some(category) = other.strip_prefix("notifications.").and_then(|c| {
                NotificationCategory::ALL.into_iter().find(|cat| {
                    serde_json::to_value(cat)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s == c))
                        == Some(true)
                })
            }) else {
                bail!("unknown setting '{other}' (see `locdev settings show`)");
            };
            settings.notifications.categories.insert(category, flag()?);
        }
    }
    Ok(())
}

/// For `locdev deps outdated`: the jump, as shown.
pub fn update_label(kind: UpdateKind) -> &'static str {
    match kind {
        UpdateKind::None => "",
        UpdateKind::Patch => "patch",
        UpdateKind::Minor => "minor",
        UpdateKind::Major => "major",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_dotted_keys() {
        let mut settings = Settings::default();
        apply(&mut settings, "automation.audit", "weekly").unwrap();
        apply(&mut settings, "network.offline", "on").unwrap();
        apply(&mut settings, "notifications.dependency_update", "off").unwrap();
        assert_eq!(settings.automation.audit, AuditInterval::Weekly);
        assert!(settings.network.offline);
        assert!(
            !settings
                .notifications
                .enabled(NotificationCategory::DependencyUpdate)
        );
        assert!(apply(&mut settings, "automation.audit", "hourly").is_err());
        assert!(apply(&mut settings, "nope", "1").is_err());
    }
}
