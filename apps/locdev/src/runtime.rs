//! `locdev runtime …`: Node.js and PHP versions.

use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::bail;
use locdev_core::model::{JobState, Release, RuntimeInfo, RuntimeKind};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{RuntimeCommand, daemon_call};

pub async fn run(command: RuntimeCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        RuntimeCommand::List { refresh } => out.runtimes(&list(paths, refresh).await?),

        RuntimeCommand::Available { kind, all } => {
            let request = Request::RuntimeCatalog { kind };
            let Response::Catalog { releases } = daemon_call(paths, request).await? else {
                bail!("unexpected response from daemon");
            };
            out.catalog(
                kind,
                &if all {
                    releases
                } else {
                    newest_per_line(kind, releases)
                },
            );
        }

        RuntimeCommand::Install { kind, version } => {
            let request = Request::InstallRuntime { kind, version };
            let Response::Runtimes { runtimes } = daemon_call(paths, request).await? else {
                bail!("unexpected response from daemon");
            };
            let exact = job_of(&runtimes, kind)
                .map(|j| j.version.clone())
                .unwrap_or_default();
            return wait_for_install(paths, kind, &exact, out).await;
        }

        RuntimeCommand::Update { kind, line } => {
            // Make sure the catalog is fresh, then install what it says is newer.
            let request = Request::RuntimeCatalog { kind };
            daemon_call(paths, request).await?;
            let runtimes = list(paths, false).await?;
            let updates: Vec<_> = runtimes
                .iter()
                .find(|r| r.kind == kind)
                .map(|r| r.updates.clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|u| {
                    line.as_deref()
                        .is_none_or(|l| l.trim_start_matches('v') == u.line)
                })
                .collect();
            if updates.is_empty() {
                out.message(&format!("{} is up to date", kind.label()));
                return Ok(ExitCode::SUCCESS);
            }
            for update in updates {
                let request = Request::InstallRuntime {
                    kind,
                    version: update.latest.clone(),
                };
                daemon_call(paths, request).await?;
                let code = wait_for_install(paths, kind, &update.latest, out).await?;
                if code != ExitCode::SUCCESS {
                    return Ok(code);
                }
                out.message(&format!(
                    "{} {} stays installed; remove it with `locdev runtime uninstall {} {}`",
                    kind.label(),
                    update.installed,
                    kind.executable(),
                    update.installed
                ));
            }
        }

        RuntimeCommand::Uninstall {
            kind,
            version,
            force,
        } => {
            daemon_call(
                paths,
                Request::UninstallRuntime {
                    kind,
                    version: version.clone(),
                    force,
                },
            )
            .await?;
            out.message(&format!("removed {} {version}", kind.label()));
        }

        RuntimeCommand::Default {
            kind,
            version,
            clear,
        } => {
            let version = if clear {
                None
            } else {
                version.filter(|v| !v.trim().is_empty())
            };
            let Response::Runtimes { runtimes } =
                daemon_call(paths, Request::SetDefaultRuntime { kind, version }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.runtimes(&runtimes);
        }

        RuntimeCommand::Terminal { state } => {
            let enable = state.as_deref().map(|s| s == "on");
            let Response::TerminalRuntimes(terminal) =
                daemon_call(paths, Request::TerminalRuntimes { enable }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.terminal_runtimes(&terminal);
        }

        RuntimeCommand::Modules { version } => {
            let Response::Runtimes { runtimes } =
                daemon_call(paths, Request::ListRuntimes { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let Some(php) = runtimes.iter().find(|r| r.kind == RuntimeKind::Php) else {
                bail!("no PHP found");
            };
            let wanted = version.or_else(|| php.default.clone());
            let found = php
                .versions
                .iter()
                .find(|v| match &wanted {
                    Some(w) => v.version == *w || v.version.starts_with(&format!("{w}.")),
                    None => true,
                })
                .ok_or_else(|| anyhow::anyhow!("no detected PHP matches {wanted:?}"))?;
            let Response::PhpModules(modules) = daemon_call(
                paths,
                Request::PhpModules {
                    executable: found.executable.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.php_modules(&modules);
        }
        RuntimeCommand::Extensions { site } => {
            let name = crate::site::resolve(paths, site).await?.project.name;
            let Response::PhpExtensions(report) =
                daemon_call(paths, Request::PhpExtensions { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.php_extensions(&report);
            if !report.missing.is_empty() {
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn list(paths: &Paths, refresh: bool) -> anyhow::Result<Vec<RuntimeInfo>> {
    match daemon_call(paths, Request::ListRuntimes { refresh }).await? {
        Response::Runtimes { runtimes } => Ok(runtimes),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

fn job_of(runtimes: &[RuntimeInfo], kind: RuntimeKind) -> Option<&locdev_core::model::InstallJob> {
    runtimes.iter().find(|r| r.kind == kind)?.job.as_ref()
}

/// Polls the daemon's install job, drawing a progress line on a terminal.
async fn wait_for_install(
    paths: &Paths,
    kind: RuntimeKind,
    version: &str,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let tty = std::io::stderr().is_terminal() && !out.is_json();
    loop {
        let runtimes = list(paths, false).await?;
        let Some(job) = job_of(&runtimes, kind) else {
            bail!("the install disappeared");
        };

        if tty {
            let detail = match (job.state, job.total) {
                (JobState::Downloading, Some(total)) if total > 0 => format!(
                    "downloading {:>5.1} / {:.1} MB  {:>3}%",
                    job.downloaded as f64 / 1e6,
                    total as f64 / 1e6,
                    job.downloaded * 100 / total
                ),
                (JobState::Downloading, _) => {
                    format!("downloading {:.1} MB", job.downloaded as f64 / 1e6)
                }
                (JobState::Verifying, _) => "verifying sha256…".into(),
                (JobState::Extracting, _) => "extracting…".into(),
                (JobState::Done | JobState::Failed, _) => String::new(),
            };
            eprint!("\r\x1b[2K{} {}  {detail}", kind.label(), job.version);
            std::io::stderr().flush()?;
        }

        match job.state {
            JobState::Done => {
                if tty {
                    eprintln!("\r\x1b[2K");
                }
                out.message(&format!("installed {} {version} (verified)", kind.label()));
                return Ok(ExitCode::SUCCESS);
            }
            JobState::Failed => {
                if tty {
                    eprintln!();
                }
                bail!(
                    "{}",
                    job.error.clone().unwrap_or_else(|| "install failed".into())
                );
            }
            _ => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
}

/// The newest release of each major version, keeping the catalog readable.
/// The newest release of each line: the major for Node and the package
/// managers, major.minor for PHP (8.3 and 8.4 are separate lines).
fn newest_per_line(kind: RuntimeKind, releases: Vec<Release>) -> Vec<Release> {
    let parts = if kind == RuntimeKind::Php { 2 } else { 1 };
    let mut seen = std::collections::HashSet::new();
    releases
        .into_iter()
        .filter(|r| {
            let line: Vec<&str> = r.version.split('.').take(parts).collect();
            seen.insert(line.join("."))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releases(versions: &[&str]) -> Vec<Release> {
        versions
            .iter()
            .map(|v| Release {
                version: v.to_string(),
                lts: None,
                date: String::new(),
            })
            .collect()
    }

    #[test]
    fn php_lines_are_major_minor() {
        let all = releases(&["8.5.13", "8.5.12", "8.4.28", "8.3.37", "7.4.33"]);
        let php: Vec<String> = newest_per_line(RuntimeKind::Php, all.clone())
            .into_iter()
            .map(|r| r.version)
            .collect();
        assert_eq!(php, ["8.5.13", "8.4.28", "8.3.37", "7.4.33"]);
        let node: Vec<String> = newest_per_line(RuntimeKind::Node, all)
            .into_iter()
            .map(|r| r.version)
            .collect();
        assert_eq!(node, ["8.5.13", "7.4.33"]);
    }
}
