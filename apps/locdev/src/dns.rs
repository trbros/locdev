//! `locdev dns …`: inspect and change how the OS resolves *.test.

use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{Context, bail};
use locdev_core::TLD;
use locdev_core::model::{DnsStatus, Routing};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_platform::elevate::Mode;
use locdev_platform::{Plan, resolver};

use crate::output::Output;
use crate::{DnsCommand, daemon_call};

pub async fn run(command: DnsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DnsCommand::Status => {
            let status = check(paths).await?;
            out.dns(&status);
            Ok(ExitCode::SUCCESS)
        }

        DnsCommand::Install { yes } => {
            let addr = current(paths)
                .await?
                .listen()
                .context("the LocDev DNS server is not running; see `locdev doctor`")?;
            let plan = resolver::install(TLD, addr).map_err(anyhow::Error::msg)?;
            apply(&plan, yes, out)?;

            let status = check(paths).await?;
            out.dns(&status);
            Ok(match status.routing {
                Routing::Locdev => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            })
        }

        DnsCommand::Uninstall { yes } => {
            let plan = resolver::uninstall(TLD).map_err(anyhow::Error::msg)?;
            apply(&plan, yes, out)?;
            out.dns(&check(paths).await?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn current(paths: &Paths) -> anyhow::Result<DnsStatus> {
    match daemon_call(paths, Request::Status).await? {
        Response::Status(status) => Ok(status.dns),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

/// Asks the daemon to probe routing again (OS changes are not pushed to it).
async fn check(paths: &Paths) -> anyhow::Result<DnsStatus> {
    match daemon_call(paths, Request::CheckDns).await? {
        Response::Dns(status) => Ok(status),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

/// Shows exactly what will run as administrator and asks before running it.
pub fn apply(plan: &Plan, yes: bool, out: &Output) -> anyhow::Result<()> {
    out.plan(plan);

    if !yes {
        if !std::io::stdin().is_terminal() {
            bail!("refusing to change system settings without confirmation; re-run with --yes");
        }
        eprint!("Run this with administrator rights? [y/N] ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            bail!("cancelled; nothing was changed");
        }
    }

    plan.apply(Mode::Terminal)
        .context("could not apply the change")
}
