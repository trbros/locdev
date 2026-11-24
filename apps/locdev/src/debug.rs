//! `locdev debug …`: the Debug Center from a terminal — exceptions, dumps,
//! log errors, HTTP 5xx and process failures of the sites LocDev runs.

use std::process::ExitCode;
use std::time::Duration;

use anyhow::bail;
use clap::{Subcommand, ValueEnum};
use locdev_core::observe::{EventFilter, EventKind, EventSeverity};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::daemon_call;
use crate::output::Output;
use crate::work::confirm;

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Kind {
    Exception,
    Dump,
    Log,
    Http,
    Process,
    Build,
    Job,
}

impl From<Kind> for EventKind {
    fn from(k: Kind) -> Self {
        match k {
            Kind::Exception => EventKind::Exception,
            Kind::Dump => EventKind::Dump,
            Kind::Log => EventKind::Log,
            Kind::Http => EventKind::Http,
            Kind::Process => EventKind::Process,
            Kind::Build => EventKind::Build,
            Kind::Job => EventKind::Job,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum Level {
    Info,
    Warn,
    Error,
}

#[derive(clap::Args, Clone)]
pub(crate) struct FilterArgs {
    /// Only this site.
    #[arg(long)]
    site: Option<String>,
    /// Only these kinds (repeatable).
    #[arg(long, value_enum)]
    kind: Vec<Kind>,
    /// This level and above.
    #[arg(long, value_enum)]
    level: Option<Level>,
    /// Text in the message, error type, file or URL.
    #[arg(long)]
    search: Option<String>,
}

impl FilterArgs {
    fn filter(&self) -> EventFilter {
        EventFilter {
            site: self.site.clone(),
            kinds: self.kind.iter().map(|k| (*k).into()).collect(),
            min_severity: self.level.map(|l| match l {
                Level::Info => EventSeverity::Info,
                Level::Warn => EventSeverity::Warn,
                Level::Error => EventSeverity::Error,
            }),
            query: self.search.clone(),
            ..Default::default()
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum DebugCommand {
    /// Recent events, newest last; `--follow` keeps printing new ones.
    Events {
        #[command(flatten)]
        filter: FilterArgs,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, short = 'f')]
        follow: bool,
    },
    /// Problems grouped by repeat, with counts.
    Errors {
        #[command(flatten)]
        filter: FilterArgs,
    },
    /// One event in full: stack trace, request, dumped value, likely cause.
    Show { id: u64 },
    /// Events as JSON lines (for a file or another tool).
    Export {
        #[command(flatten)]
        filter: FilterArgs,
        #[arg(long, default_value_t = 2000)]
        limit: usize,
    },
    /// Forget a site's events, or every event.
    Clear {
        #[arg(long)]
        site: Option<String>,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Send a PHP site's dump()/dd() to LocDev (on) or back to the page (off).
    Dumps {
        site: String,
        #[arg(value_parser = ["on", "off"])]
        state: String,
    },
    /// Retention, disk use, the dump server and which sites send dumps.
    Status,
}

async fn events(
    paths: &Paths,
    filter: EventFilter,
    limit: usize,
) -> anyhow::Result<(Vec<locdev_core::observe::Event>, u64)> {
    let Response::ObserveEvents { events, newest } = daemon_call(
        paths,
        Request::ObserveEvents {
            filter,
            limit: Some(limit),
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    Ok((events, newest))
}

pub async fn run(command: DebugCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DebugCommand::Events {
            filter,
            limit,
            follow,
        } => {
            let base = filter.filter();
            let (mut list, mut newest) = events(paths, base.clone(), limit).await?;
            list.reverse();
            out.debug_events(&list);
            if follow {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let (mut fresh, latest) = events(
                        paths,
                        EventFilter {
                            after: Some(newest),
                            ..base.clone()
                        },
                        500,
                    )
                    .await?;
                    newest = latest;
                    fresh.reverse();
                    out.debug_events(&fresh);
                }
            }
        }
        DebugCommand::Errors { filter } => {
            let Response::ObserveGroups { groups } = daemon_call(
                paths,
                Request::ObserveGroups {
                    filter: filter.filter(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.debug_groups(&groups);
        }
        DebugCommand::Show { id } => {
            let Response::ObserveEvent { event } =
                daemon_call(paths, Request::ObserveEvent { id }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.debug_event(&event);
        }
        DebugCommand::Export { filter, limit } => {
            let (mut list, _) = events(paths, filter.filter(), limit).await?;
            list.reverse();
            for e in list {
                println!("{}", serde_json::to_string(&e)?);
            }
        }
        DebugCommand::Clear { site, yes } => {
            if !yes {
                confirm(&match &site {
                    Some(s) => format!("Forget every Debug Center event of {s}?"),
                    None => "Forget every Debug Center event of every site?".into(),
                })?;
            }
            daemon_call(paths, Request::ObserveClear { site: site.clone() }).await?;
            out.message("Cleared.");
        }
        DebugCommand::Dumps { site, state } => {
            let enabled = state == "on";
            let Response::ObserveDumps { restarted, .. } = daemon_call(
                paths,
                Request::ObserveDumps {
                    site: site.clone(),
                    enabled,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.message(&match (enabled, restarted) {
                (true, true) => format!("{site}'s dumps now go to LocDev (restarted to pick it up). `locdev debug events --kind dump -f` shows them."),
                (true, false) => format!("{site}'s dumps will go to LocDev once it starts."),
                (false, true) => format!("{site}'s dumps go back to the page (restarted)."),
                (false, false) => format!("{site}'s dumps go back to the page."),
            });
        }
        DebugCommand::Status => {
            let Response::ObserveStatus(status) =
                daemon_call(paths, Request::ObserveStatus).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.debug_status(&status);
        }
    }
    Ok(ExitCode::SUCCESS)
}
