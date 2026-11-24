//! `locdev subdomain …` and `locdev route <host>`: which subdomains reach a
//! site, and how one hostname is resolved, routed and certified.

use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_core::subdomains::{SubdomainRule, SubdomainTarget, Subdomains};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum SubdomainCommand {
    /// The site's subdomain rules and whether unlisted ones reach it.
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// Route a subdomain (`api`, `admin.eu`, `*.tenants`) somewhere specific.
    Add {
        /// The part before the site's domain; a leading `*` is one or more labels.
        pattern: String,
        site: Option<String>,
        /// Send it to another server on this machine.
        #[arg(long, conflicts_with = "process")]
        port: Option<u16>,
        /// Send it to one of the site's auxiliary processes (its port).
        #[arg(long)]
        process: Option<String>,
    },
    /// Remove a rule.
    #[command(visible_alias = "rm")]
    Remove {
        pattern: String,
        site: Option<String>,
    },
    /// Whether unlisted subdomains (`*.site.test`) reach the site.
    Wildcard {
        #[arg(value_parser = ["on", "off"])]
        state: String,
        site: Option<String>,
    },
}

async fn save(
    paths: &Paths,
    name: String,
    subdomains: Subdomains,
    out: &Output,
) -> anyhow::Result<()> {
    let Response::Project { project } =
        daemon_call(paths, Request::SetSubdomains { name, subdomains }).await?
    else {
        bail!("unexpected response from daemon");
    };
    out.subdomains(&project);
    Ok(())
}

pub async fn run(
    command: SubdomainCommand,
    paths: &Paths,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    match command {
        SubdomainCommand::List { site } => {
            let view = site::resolve(paths, site).await?;
            out.subdomains(&view);
        }
        SubdomainCommand::Add {
            pattern,
            site,
            port,
            process,
        } => {
            let view = site::resolve(paths, site).await?;
            let target = match (port, process) {
                (Some(port), _) => SubdomainTarget::Port { port },
                (None, Some(id)) => SubdomainTarget::Process { id },
                (None, None) => SubdomainTarget::Site,
            };
            let mut subdomains = view.project.subdomains.clone();
            let normalized =
                locdev_core::subdomains::normalize(&pattern).map_err(anyhow::Error::msg)?;
            subdomains.rules.retain(|r| r.pattern != normalized);
            subdomains.rules.push(SubdomainRule {
                pattern: normalized,
                target,
            });
            save(paths, view.project.name, subdomains, out).await?;
        }
        SubdomainCommand::Remove { pattern, site } => {
            let view = site::resolve(paths, site).await?;
            let normalized =
                locdev_core::subdomains::normalize(&pattern).map_err(anyhow::Error::msg)?;
            let mut subdomains = view.project.subdomains.clone();
            let before = subdomains.rules.len();
            subdomains.rules.retain(|r| r.pattern != normalized);
            if subdomains.rules.len() == before {
                bail!("{} has no rule for {normalized}", view.project.name);
            }
            save(paths, view.project.name, subdomains, out).await?;
        }
        SubdomainCommand::Wildcard { state, site } => {
            let view = site::resolve(paths, site).await?;
            let mut subdomains = view.project.subdomains.clone();
            subdomains.wildcard = state == "on";
            save(paths, view.project.name, subdomains, out).await?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub async fn route(paths: &Paths, host: String, out: &Output) -> anyhow::Result<ExitCode> {
    let Response::HostExplain(explain) = daemon_call(paths, Request::ExplainHost { host }).await?
    else {
        bail!("unexpected response from daemon");
    };
    out.host_explain(&explain);
    let ok = explain.route.problem.is_none()
        && explain
            .dns
            .answers
            .iter()
            .any(|a| a == "127.0.0.1" || a == "::1");
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
