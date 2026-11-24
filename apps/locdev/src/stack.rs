//! `locdev stack …`: framework detection, manual stacks and definitions.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum StackCommand {
    /// Every framework LocDev recognizes, with how it runs each one.
    #[command(visible_alias = "ls")]
    List {
        /// Only these categories: framework, ui, language, runtime, tool.
        #[arg(long)]
        category: Option<String>,
    },
    /// What a folder or a site is made of (the current directory by default).
    Detect {
        /// A folder, or the name of a registered site.
        target: Option<String>,
    },
    /// A site's stack in full: components, evidence, versions, command, prerequisites.
    Inspect { site: Option<String> },
    /// Add a component the files don't prove (or bring back a hidden one).
    Add { stack: String, site: Option<String> },
    /// Hide a detected component, or drop one added by hand.
    #[command(visible_alias = "rm")]
    Remove { stack: String, site: Option<String> },
    /// Choose the adapter that runs the site ("auto" goes back to detection).
    Use {
        adapter: String,
        site: Option<String>,
        /// Also make the adapter's start command the site's command.
        #[arg(long)]
        apply_command: bool,
    },
    /// The definitions in use; --refresh downloads newer signed ones.
    Definitions {
        #[arg(long)]
        refresh: bool,
        /// With --refresh: also take a revision that was rolled back.
        #[arg(long, requires = "refresh")]
        force: bool,
        /// Go back to the previous definitions (or the bundled ones).
        #[arg(long, conflicts_with = "refresh")]
        rollback: bool,
    },
}

pub async fn run(command: StackCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let name = |site| async { anyhow::Ok(site::resolve(paths, site).await?.project.name) };
    let report = |response| match response {
        Response::StackReport(report) => Ok(report),
        other => bail!("unexpected response from daemon: {other:?}"),
    };
    match command {
        StackCommand::List { category } => {
            let Response::StackDefinitions {
                definitions,
                registry,
            } = daemon_call(paths, Request::StackDefinitions).await?
            else {
                bail!("unexpected response from daemon");
            };
            let definitions: Vec<_> = definitions
                .into_iter()
                .filter(|d| {
                    category.as_deref().is_none_or(|c| {
                        serde_json::to_value(d.category)
                            .ok()
                            .and_then(|v| v.as_str().map(|s| s == c))
                            == Some(true)
                    })
                })
                .collect();
            out.stack_list(&definitions, &registry);
        }
        StackCommand::Detect { target } => {
            let request = match target {
                Some(t) if !std::path::Path::new(&t).is_dir() => Request::StackDetect {
                    name: Some(t),
                    path: None,
                },
                Some(t) => Request::StackDetect {
                    name: None,
                    path: Some(PathBuf::from(t)),
                },
                None => Request::StackDetect {
                    name: None,
                    path: Some(std::env::current_dir()?),
                },
            };
            out.stack_report(&report(daemon_call(paths, request).await?)?, false);
        }
        StackCommand::Inspect { site } => {
            let request = Request::StackDetect {
                name: Some(name(site).await?),
                path: None,
            };
            out.stack_report(&report(daemon_call(paths, request).await?)?, true);
        }
        StackCommand::Add { stack, site } => {
            let request = Request::StackSet {
                name: name(site).await?,
                add: vec![stack],
                remove: Vec::new(),
                adapter: None,
                apply_command: false,
            };
            out.stack_report(&report(daemon_call(paths, request).await?)?, false);
        }
        StackCommand::Remove { stack, site } => {
            let request = Request::StackSet {
                name: name(site).await?,
                add: Vec::new(),
                remove: vec![stack],
                adapter: None,
                apply_command: false,
            };
            out.stack_report(&report(daemon_call(paths, request).await?)?, false);
        }
        StackCommand::Use {
            adapter,
            site,
            apply_command,
        } => {
            let adapter = if adapter == "auto" {
                String::new()
            } else {
                adapter
            };
            let request = Request::StackSet {
                name: name(site).await?,
                add: Vec::new(),
                remove: Vec::new(),
                adapter: Some(adapter),
                apply_command,
            };
            out.stack_report(&report(daemon_call(paths, request).await?)?, false);
        }
        StackCommand::Definitions {
            refresh,
            force,
            rollback,
        } => {
            let request = if refresh {
                Request::RefreshDefinitions { force }
            } else if rollback {
                Request::RollbackDefinitions
            } else {
                Request::StackDefinitions
            };
            let Response::StackDefinitions { registry, .. } = daemon_call(paths, request).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.definitions_status(&registry);
        }
    }
    Ok(ExitCode::SUCCESS)
}
