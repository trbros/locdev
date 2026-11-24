//! `locdev park …`: folders whose projects become sites on their own.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use locdev_core::park::{OnMissing, OnNew, ParkOptions};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::daemon_call;
use crate::output::Output;

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct ParkArgs {
    #[command(subcommand)]
    command: Option<ParkCommand>,
    /// The folder to park, or whose rules to change (default: the current folder).
    path: Option<PathBuf>,
    #[command(flatten)]
    rules: Rules,
}

#[derive(Args, Default)]
pub(crate) struct Rules {
    /// How many levels below the folder to look for projects (1 to 3).
    #[arg(long)]
    depth: Option<u8>,
    /// Skip folders by name or relative path (`archive-*`, `clients/old`); repeatable. Replaces the list.
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,
    /// Clear the exclude list.
    #[arg(long, conflicts_with = "exclude")]
    no_exclude: bool,
    /// New projects are only listed; you add each one yourself.
    #[arg(long, conflicts_with = "register")]
    suggest: bool,
    /// New projects become sites right away (the default).
    #[arg(long)]
    register: bool,
    /// Keep a site whose folder disappeared (marked missing) instead of removing it.
    #[arg(long, conflicts_with = "remove_missing")]
    keep_missing: bool,
    /// Remove a site whose folder disappeared (the default).
    #[arg(long)]
    remove_missing: bool,
}

impl Rules {
    fn options(&self) -> ParkOptions {
        ParkOptions {
            enabled: None,
            depth: self.depth,
            exclude: if self.no_exclude {
                Some(Vec::new())
            } else {
                (!self.exclude.is_empty()).then(|| self.exclude.clone())
            },
            on_new: if self.suggest {
                Some(OnNew::Suggest)
            } else {
                self.register.then_some(OnNew::Register)
            },
            on_missing: if self.keep_missing {
                Some(OnMissing::Keep)
            } else {
                self.remove_missing.then_some(OnMissing::Unregister)
            },
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum ParkCommand {
    /// Parked folders, their rules and the projects found in them.
    #[command(visible_alias = "ls")]
    List,
    /// Stop parking a folder. Its sites are removed (never their files).
    #[command(visible_alias = "forget")]
    Remove {
        /// Default: the current folder.
        path: Option<PathBuf>,
        /// Keep its sites as regular sites.
        #[arg(long)]
        keep_sites: bool,
    },
    /// Look for new and removed projects now.
    Scan {
        /// Default: every parked folder that is on.
        path: Option<PathBuf>,
    },
    /// Don't turn a project of a parked folder into a site (removes its site).
    Ignore { project: PathBuf },
    /// Let an ignored project become a site again.
    Unignore { project: PathBuf },
    /// Resume scanning and watching a parked folder.
    Enable { path: Option<PathBuf> },
    /// Pause a parked folder: no scans, no watching; its sites stay.
    Disable { path: Option<PathBuf> },
}

fn absolute(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = match path {
        Some(path) => path,
        None => std::env::current_dir()?,
    };
    std::path::absolute(&path).with_context(|| format!("invalid path {}", path.display()))
}

async fn call(paths: &Paths, request: Request, out: &Output) -> anyhow::Result<()> {
    let Response::Parked {
        dirs,
        added,
        removed,
    } = daemon_call(paths, request).await?
    else {
        bail!("unexpected response from daemon");
    };
    out.parked(&dirs, &added, &removed);
    Ok(())
}

async fn set(paths: &Paths, path: &Path, options: ParkOptions, out: &Output) -> anyhow::Result<()> {
    let request = Request::ParkSet {
        path: path.to_path_buf(),
        options,
    };
    call(paths, request, out).await
}

/// Turns a parked folder on or off; never parks a new one.
async fn toggle(
    paths: &Paths,
    path: Option<PathBuf>,
    on: bool,
    out: &Output,
) -> anyhow::Result<()> {
    let path = absolute(path)?;
    let path = path.canonicalize().unwrap_or(path);
    let Response::Parked { dirs, .. } = daemon_call(paths, Request::ParkList).await? else {
        bail!("unexpected response from daemon");
    };
    if !dirs.iter().any(|d| d.dir.path == path) {
        bail!(
            "{} is not a parked folder (park it with `locdev park {}`)",
            path.display(),
            path.display()
        );
    }
    let options = ParkOptions {
        enabled: Some(on),
        ..Default::default()
    };
    set(paths, &path, options, out).await
}

pub async fn run(args: ParkArgs, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match args.command {
        None => set(paths, &absolute(args.path)?, args.rules.options(), out).await?,
        Some(ParkCommand::List) => call(paths, Request::ParkList, out).await?,
        Some(ParkCommand::Remove { path, keep_sites }) => {
            let request = Request::ParkRemove {
                path: absolute(path)?,
                keep_sites,
            };
            call(paths, request, out).await?;
        }
        Some(ParkCommand::Scan { path }) => {
            let path = path.map(|p| absolute(Some(p))).transpose()?;
            call(paths, Request::ParkScan { path }, out).await?;
        }
        Some(ParkCommand::Ignore { project }) => {
            let request = Request::ParkIgnore {
                path: absolute(Some(project))?,
                ignore: true,
            };
            call(paths, request, out).await?;
        }
        Some(ParkCommand::Unignore { project }) => {
            let request = Request::ParkIgnore {
                path: absolute(Some(project))?,
                ignore: false,
            };
            call(paths, request, out).await?;
        }
        Some(ParkCommand::Enable { path }) => toggle(paths, path, true, out).await?,
        Some(ParkCommand::Disable { path }) => toggle(paths, path, false, out).await?,
    }
    Ok(ExitCode::SUCCESS)
}
