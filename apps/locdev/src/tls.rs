//! `locdev tls …`: HTTPS status and trusting the local CA.

use std::process::ExitCode;

use anyhow::{Context, bail};
use locdev_core::model::TlsStatus;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_platform::trust;

use crate::output::Output;
use crate::{TlsCommand, daemon_call, dns};

/// Common name of the LocDev CA (see `locdev_tls::CA_NAME`).
const CA_NAME: &str = "LocDev Local CA";

pub async fn run(command: TlsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        TlsCommand::Status => {
            out.tls(&check(paths).await?);
            Ok(ExitCode::SUCCESS)
        }

        TlsCommand::Trust { yes } => {
            let ca = check(paths)
                .await?
                .ca_path
                .context("the local CA is not available; see `locdev doctor`")?;
            let plan = trust::install(&ca, &ca.with_file_name("ca.cer"), CA_NAME)
                .map_err(anyhow::Error::msg)?;
            dns::apply(&plan, yes, out)?;

            let status = check(paths).await?;
            out.tls(&status);
            Ok(if status.trusted {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }

        TlsCommand::Untrust { yes } => {
            let plan = trust::uninstall(CA_NAME).map_err(anyhow::Error::msg)?;
            dns::apply(&plan, yes, out)?;
            out.tls(&check(paths).await?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Asks the daemon to re-read the trust stores.
async fn check(paths: &Paths) -> anyhow::Result<TlsStatus> {
    match daemon_call(paths, Request::CheckTls).await? {
        Response::Tls(status) => Ok(status),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}
