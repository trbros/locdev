//! `locdev deploy …`: which platforms fit a site, what it needs, readiness
//! (`analyze`, `config`, local only), and the connected platforms' APIs
//! (`connect`, `link`, `status`, `run`, `rollback`…).

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum DeployCommand {
    /// What the site needs from a host, the platforms that fit and readiness checks.
    #[command(visible_alias = "check")]
    Analyze { site: Option<String> },
    /// Print the config file LocDev would write for a platform; --write creates it.
    Config {
        /// `render` or `netlify` (others set themselves up; the analysis says how).
        provider: String,
        site: Option<String>,
        /// Create the file (never over an existing one).
        #[arg(long)]
        write: bool,
        /// Write without asking.
        #[arg(long, short)]
        yes: bool,
    },
    /// Platforms LocDev talks to (Vercel, Netlify) and which are connected.
    Platforms,
    /// Connect a platform with a token (read from stdin, kept in the system keyring).
    Connect {
        #[arg(value_parser = ["vercel", "netlify"])]
        platform: String,
        /// Vercel team (slug or team_… ID) the token acts for.
        #[arg(long)]
        team: Option<String>,
    },
    /// Forget a platform's token.
    Disconnect {
        #[arg(value_parser = ["vercel", "netlify"])]
        platform: String,
    },
    /// The platform's projects (Vercel) or sites (Netlify).
    Projects {
        #[arg(value_parser = ["vercel", "netlify"])]
        platform: String,
    },
    /// Link a site to a remote project (by ID or name).
    Link {
        #[arg(value_parser = ["vercel", "netlify"])]
        platform: String,
        project: String,
        site: Option<String>,
    },
    /// Remove the site's link (nothing changes on the platform).
    Unlink { site: Option<String> },
    /// The linked project: deployments, domains, variable names, and what a deploy would build.
    Status { site: Option<String> },
    /// Deploy the pushed commit of the site's branch (a preview unless --production).
    Run {
        site: Option<String>,
        #[arg(long)]
        production: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Cancel a deployment that is building.
    Cancel {
        deployment: String,
        site: Option<String>,
    },
    /// Send production traffic back to an earlier deployment.
    Rollback {
        deployment: String,
        site: Option<String>,
        #[arg(long, short)]
        yes: bool,
    },
    /// A deployment's build log (or the dashboard page that has it).
    Logs {
        deployment: String,
        site: Option<String>,
    },
}

pub async fn run(command: DeployCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DeployCommand::Analyze { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::DeployReport(report) =
                daemon_call(paths, Request::DeployAnalysis { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.deploy_report(&report);
        }
        DeployCommand::Config {
            provider,
            site,
            write,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::DeployConfig(config) = daemon_call(
                paths,
                Request::DeployConfig {
                    name: name.clone(),
                    provider: provider.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.deploy_config(&config);
            if !write {
                return Ok(ExitCode::SUCCESS);
            }
            if config.exists {
                bail!(
                    "{} already exists; LocDev doesn't overwrite it",
                    config.file
                );
            }
            if !yes {
                if !std::io::stdin().is_terminal() {
                    bail!("run it on a terminal to confirm, or pass --yes");
                }
                eprint!("Create {}? [y/N] ", config.file);
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if !matches!(answer.trim(), "y" | "Y" | "yes") {
                    eprintln!("Nothing written.");
                    return Ok(ExitCode::SUCCESS);
                }
            }
            let Response::DeployConfigWritten { file } = daemon_call(
                paths,
                Request::DeployWriteConfig {
                    name,
                    provider,
                    hash: config.hash,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            eprintln!("Created {file}. Review it and commit it with the project.");
        }
        DeployCommand::Platforms => {
            let Response::DeployPlatforms { platforms } =
                daemon_call(paths, Request::DeployPlatforms).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.remote_platforms(&platforms);
        }
        DeployCommand::Connect { platform, team } => {
            // From stdin, never from an argument (which shell history keeps).
            if std::io::stdin().is_terminal() {
                let Response::DeployPlatforms { platforms } =
                    daemon_call(paths, Request::DeployPlatforms).await?
                else {
                    bail!("unexpected response from daemon");
                };
                if let Some(p) = platforms.iter().find(|p| p.id == platform) {
                    eprintln!("{}\n{}", p.token_help, p.token_url);
                }
                eprint!("Paste the token (it stays in your system keyring), then Enter: ");
            }
            let token = crate::vcs::read_secret()?;
            let Response::DeployPlatforms { platforms } = daemon_call(
                paths,
                Request::DeployConnect {
                    platform,
                    token,
                    team,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.remote_platforms(&platforms);
        }
        DeployCommand::Disconnect { platform } => {
            daemon_call(
                paths,
                Request::DeployDisconnect {
                    platform: platform.clone(),
                },
            )
            .await?;
            out.message(&format!("{platform} disconnected; its token was removed."));
        }
        DeployCommand::Projects { platform } => {
            let Response::RemoteProjects { projects } =
                daemon_call(paths, Request::DeployRemoteProjects { platform }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.remote_projects(&projects);
        }
        DeployCommand::Link {
            platform,
            project,
            site,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            daemon_call(
                paths,
                Request::DeployLinkSite {
                    site: name.clone(),
                    platform: Some(platform),
                    project: Some(project),
                },
            )
            .await?;
            out.message(&format!("{name} linked; `locdev deploy status {name}`."));
        }
        DeployCommand::Unlink { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            daemon_call(
                paths,
                Request::DeployLinkSite {
                    site: name.clone(),
                    platform: None,
                    project: None,
                },
            )
            .await?;
            out.message(&format!("{name} unlinked."));
        }
        DeployCommand::Status { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::RemoteStatus(status) =
                daemon_call(paths, Request::DeployRemoteStatus { site: name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.remote_status(&status);
        }
        DeployCommand::Run {
            site,
            production,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            if !yes {
                let Response::RemoteStatus(status) =
                    daemon_call(paths, Request::DeployRemoteStatus { site: name.clone() }).await?
                else {
                    bail!("unexpected response from daemon");
                };
                eprintln!(
                    "{} {} on {} ({}): {} at {}",
                    if production {
                        "Production deploy of"
                    } else {
                        "Preview deploy of"
                    },
                    status.project.name,
                    status.platform.name,
                    status.project.repo.as_deref().unwrap_or("?"),
                    status.git.branch.as_deref().unwrap_or("?"),
                    status
                        .git
                        .commit
                        .as_deref()
                        .map(|c| &c[..c.len().min(10)])
                        .unwrap_or("?"),
                );
                if !status.git.problems.is_empty() {
                    bail!("not deploying: {}", status.git.problems.join("; "));
                }
                crate::work::confirm("Deploy?")?;
            }
            let Response::RemoteDeployment { deployment } = daemon_call(
                paths,
                Request::DeployRun {
                    site: name,
                    production,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.remote_deployments(std::slice::from_ref(&deployment));
        }
        DeployCommand::Cancel { deployment, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            daemon_call(
                paths,
                Request::DeployCancel {
                    site: name,
                    deployment,
                },
            )
            .await?;
            out.message("Cancelled.");
        }
        DeployCommand::Rollback {
            deployment,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            if !yes {
                eprintln!("Production traffic of {name} goes back to {deployment}.");
                crate::work::confirm("Roll back?")?;
            }
            daemon_call(
                paths,
                Request::DeployRollback {
                    site: name,
                    deployment,
                },
            )
            .await?;
            out.message("Production now serves that deployment.");
        }
        DeployCommand::Logs { deployment, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::RemoteLog(log) = daemon_call(
                paths,
                Request::DeployLogs {
                    site: name,
                    deployment,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            if out.is_json() {
                out.print_json(&log);
            } else if let Some(link) = log.link {
                println!("This platform's API has no build logs; they're at {link}");
            } else {
                for line in log.lines {
                    println!("{line}");
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
