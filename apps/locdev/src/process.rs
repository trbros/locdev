//! `locdev process …`: a site's auxiliary processes (queue workers,
//! schedulers, WebSocket servers) next to its dev server.

use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::model::{DevServerMode, ProcessRole};
use locdev_core::paths::Paths;
use locdev_core::protocol::{ProcessAction, Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum ProcessCommand {
    /// The site's processes: the dev server, its helper and the auxiliaries.
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// Start an auxiliary process (needs a mode other than ask/never).
    Start {
        process: String,
        site: Option<String>,
    },
    Stop {
        process: String,
        site: Option<String>,
    },
    Restart {
        process: String,
        site: Option<String>,
    },
    /// Decide how LocDev runs it: ask, manual, auto (with the site) or never.
    Mode {
        process: String,
        mode: String,
        site: Option<String>,
        /// Run this command instead of the offered one ("" goes back to it).
        #[arg(long)]
        command: Option<String>,
    },
    /// Add a process of your own (a worker, a watcher…).
    Add {
        process: String,
        site: Option<String>,
        /// The command; {port} gets a port of its own.
        #[arg(long)]
        command: String,
        #[arg(long)]
        label: Option<String>,
        /// manual (default) or auto.
        #[arg(long, default_value = "manual")]
        mode: String,
    },
    /// Remove a process you added, or forget the decision about an offered one.
    #[command(visible_alias = "rm")]
    Remove {
        process: String,
        site: Option<String>,
    },
    /// Show a process's output.
    Logs {
        process: String,
        site: Option<String>,
        #[arg(short = 'n', long, default_value_t = 100)]
        lines: usize,
        #[arg(short, long)]
        follow: bool,
    },
}

pub(crate) fn parse_mode(mode: &str) -> anyhow::Result<DevServerMode> {
    Ok(match mode {
        "ask" => DevServerMode::Ask,
        "manual" => DevServerMode::Manual,
        "auto" => DevServerMode::Auto,
        "never" => DevServerMode::Never,
        other => bail!("'{other}': use ask, manual, auto or never"),
    })
}

pub async fn run(command: ProcessCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let name = |site| async { anyhow::Ok(site::resolve(paths, site).await?.project.name) };
    let control = |name: String, process: String, action| Request::ControlProcess {
        name,
        process,
        action,
    };
    let request = match command {
        ProcessCommand::List { site } => {
            let view = site::resolve(paths, site).await?;
            let Response::ProjectProcesses { processes } = daemon_call(
                paths,
                Request::ProjectProcesses {
                    name: view.project.name.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.project_processes(&view, &processes);
            return Ok(ExitCode::SUCCESS);
        }
        ProcessCommand::Logs {
            process,
            site,
            lines,
            follow,
        } => {
            let name = name(site).await?;
            return site::logs(
                paths,
                name,
                ProcessRole::Main,
                Some(process),
                lines,
                follow,
                out,
            )
            .await;
        }
        ProcessCommand::Start { process, site } => {
            control(name(site).await?, process, ProcessAction::Start)
        }
        ProcessCommand::Stop { process, site } => {
            control(name(site).await?, process, ProcessAction::Stop)
        }
        ProcessCommand::Restart { process, site } => {
            control(name(site).await?, process, ProcessAction::Restart)
        }
        ProcessCommand::Mode {
            process,
            mode,
            site,
            command,
        } => Request::SetProcess {
            name: name(site).await?,
            process,
            mode: Some(parse_mode(&mode)?),
            command,
            label: None,
        },
        ProcessCommand::Add {
            process,
            site,
            command,
            label,
            mode,
        } => Request::SetProcess {
            name: name(site).await?,
            process,
            mode: Some(parse_mode(&mode)?),
            command: Some(command),
            label,
        },
        ProcessCommand::Remove { process, site } => Request::RemoveProcess {
            name: name(site).await?,
            process,
        },
    };
    let Response::ProjectProcesses { processes } = daemon_call(paths, request).await? else {
        bail!("unexpected response from daemon");
    };
    out.aux_processes(&processes);
    Ok(ExitCode::SUCCESS)
}
