//! `locdev tunnel …`: share a running site on the internet through a
//! tunnel provider (Cloudflare Quick Tunnels).

use std::process::ExitCode;
use std::time::Duration;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::deps::JobStatus;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_core::tunnel::TunnelState;

use crate::daemon_call;
use crate::output::Output;
use crate::work::{confirm, wait};

#[derive(Subcommand)]
pub(crate) enum TunnelCommand {
    /// Providers, their limits and whether their program is installed.
    Providers,
    /// Download and verify a provider's program (cloudflare).
    Install {
        #[arg(default_value = "cloudflare")]
        provider: String,
    },
    /// Share a running site; prints the public address.
    Start {
        site: String,
        #[arg(long, default_value = "cloudflare")]
        provider: String,
        /// Only these e-mail addresses (or *@domain) may open it; repeatable or comma-separated.
        #[arg(long = "allow", value_name = "EMAIL")]
        allowed: Vec<String>,
        /// Don't ask before exposing the site.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Stop sharing a site.
    Stop { site: String },
    /// Shared sites and their addresses.
    #[command(visible_alias = "ls")]
    List,
    /// The tunnel program's output for a site.
    Logs {
        site: String,
        #[arg(long, default_value_t = 200)]
        lines: usize,
    },
}

pub async fn run(command: TunnelCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        TunnelCommand::Providers => {
            let Response::TunnelProviders { providers } =
                daemon_call(paths, Request::TunnelProviders).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.tunnel_providers(&providers);
        }
        TunnelCommand::Install { provider } => {
            let Response::Job { job } =
                daemon_call(paths, Request::TunnelInstall { provider }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let job = wait(paths, job.id, out).await?;
            match job.status {
                JobStatus::Completed => out.message(job.result.as_deref().unwrap_or("Installed.")),
                _ => bail!(
                    "{}",
                    job.error
                        .unwrap_or_else(|| "the install didn't finish".into())
                ),
            }
        }
        TunnelCommand::Start {
            site,
            provider,
            allowed,
            yes,
        } => {
            if !yes {
                let Response::TunnelProviders { providers } =
                    daemon_call(paths, Request::TunnelProviders).await?
                else {
                    bail!("unexpected response from daemon");
                };
                if let Some(p) = providers.iter().find(|p| p.id == provider) {
                    eprintln!(
                        "{site} will be reachable from the internet through {}:",
                        p.name
                    );
                    for limit in &p.limits {
                        eprintln!("  · {limit}");
                    }
                }
                if allowed.is_empty() {
                    eprintln!(
                        "  · Anyone with the address can open it (use --allow to require an e-mail login)."
                    );
                }
                confirm("Share it?")?;
            }
            let Response::Tunnel { mut tunnel } = daemon_call(
                paths,
                Request::TunnelStart {
                    site: site.clone(),
                    provider: Some(provider),
                    allowed,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            // The address usually comes within seconds; keep asking a while.
            for _ in 0..60 {
                if tunnel.state != TunnelState::Starting {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                let Response::Tunnels { tunnels } = daemon_call(paths, Request::TunnelList).await?
                else {
                    bail!("unexpected response from daemon");
                };
                match tunnels.into_iter().find(|t| t.site == site) {
                    Some(t) => tunnel = t,
                    None => bail!("the tunnel stopped; see `locdev tunnel logs {site}`"),
                }
            }
            out.tunnels(std::slice::from_ref(&tunnel));
            if tunnel.state == TunnelState::Failed {
                return Ok(ExitCode::FAILURE);
            }
        }
        TunnelCommand::Stop { site } => {
            daemon_call(paths, Request::TunnelStop { site: site.clone() }).await?;
            out.message(&format!("{site} is no longer shared."));
        }
        TunnelCommand::List => {
            let Response::Tunnels { tunnels } = daemon_call(paths, Request::TunnelList).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.tunnels(&tunnels);
        }
        TunnelCommand::Logs { site, lines } => {
            let Response::Logs { lines } = daemon_call(
                paths,
                Request::TunnelLogs {
                    site,
                    after: None,
                    limit: Some(lines),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            if out.is_json() {
                out.print_json(&lines);
            } else {
                for line in lines {
                    println!("{}", line.text);
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
