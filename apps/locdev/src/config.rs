//! `locdev config …`: a site's config files, options, analysis and history.

use std::io::IsTerminal;
use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::config::{ConfigChange, ConfigPreview};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum ConfigCommand {
    /// The site's config files (only the ones that exist).
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// A file's options as LocDev reads them, or its text with --text.
    Show {
        /// A path from `locdev config list` (`vite.config.ts`, `src/app.css`).
        file: String,
        site: Option<String>,
        /// Print the file instead of its options.
        #[arg(long)]
        text: bool,
    },
    /// Change one option, keeping comments and formatting. Shows the diff first.
    Set {
        file: String,
        /// Dotted option key (`server.port`, `compilerOptions.strict`).
        key: String,
        /// JSON literal (`true`, `5173`, `"bundler"`); anything else is text.
        value: String,
        site: Option<String>,
        /// Save without asking (the diff is still printed).
        #[arg(long, short)]
        yes: bool,
    },
    /// Check the config against the installed versions; nothing is changed.
    Check { site: Option<String> },
    /// Versions LocDev kept before each save.
    History { file: String, site: Option<String> },
    /// Put a kept version back (the current one is kept too).
    Revert {
        file: String,
        /// An id from `locdev config history`.
        id: String,
        site: Option<String>,
        #[arg(long, short)]
        yes: bool,
    },
}

fn parse_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

async fn preview(
    paths: &Paths,
    name: &str,
    file: &str,
    change: ConfigChange,
) -> anyhow::Result<ConfigPreview> {
    let Response::ConfigPreview(preview) = daemon_call(
        paths,
        Request::ConfigPreview {
            name: name.into(),
            path: file.into(),
            change,
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    Ok(preview)
}

async fn base_hash(paths: &Paths, name: &str, file: &str) -> anyhow::Result<String> {
    let Response::ConfigFile(view) = daemon_call(
        paths,
        Request::ConfigFile {
            name: name.into(),
            path: file.into(),
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    Ok(view.hash)
}

/// Shows the diff and asks, unless `yes`. False when nothing changes or the user declines.
fn approve(out: &Output, preview: &ConfigPreview, yes: bool) -> anyhow::Result<bool> {
    out.config_preview(preview);
    if preview.diff.is_empty() {
        return Ok(false);
    }
    if let Some(error) = &preview.syntax_error {
        bail!("the result wouldn't parse: {error}");
    }
    if yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        bail!("run it on a terminal to confirm, or pass --yes");
    }
    eprint!("Save this change? [y/N] ");
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

pub async fn run(command: ConfigCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        ConfigCommand::List { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::ConfigFiles { files } =
                daemon_call(paths, Request::ConfigFiles { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.config_files(&files);
        }
        ConfigCommand::Show { file, site, text } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::ConfigFile(view) =
                daemon_call(paths, Request::ConfigFile { name, path: file }).await?
            else {
                bail!("unexpected response from daemon");
            };
            if text && !out.is_json() {
                print!("{}", view.content);
            } else {
                out.config_view(&view);
            }
        }
        ConfigCommand::Set {
            file,
            key,
            value,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let hash = base_hash(paths, &name, &file).await?;
            let change = ConfigChange::Set {
                key,
                value: parse_value(&value),
            };
            let shown = preview(paths, &name, &file, change.clone()).await?;
            if !approve(out, &shown, yes)? {
                eprintln!("Nothing saved.");
                return Ok(ExitCode::SUCCESS);
            }
            daemon_call(
                paths,
                Request::ConfigSave {
                    name,
                    path: file.clone(),
                    change,
                    base_hash: hash,
                },
            )
            .await?;
            eprintln!("Saved {file}. `locdev config history {file}` lists what it replaced.");
        }
        ConfigCommand::Check { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::ConfigFindings { findings } =
                daemon_call(paths, Request::ConfigCheck { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.config_findings(&findings);
        }
        ConfigCommand::History { file, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::ConfigHistory { entries } =
                daemon_call(paths, Request::ConfigHistory { name, path: file }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.config_history(&entries);
        }
        ConfigCommand::Revert {
            file,
            id,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let hash = base_hash(paths, &name, &file).await?;
            let Response::ConfigHistoryVersion { content } = daemon_call(
                paths,
                Request::ConfigHistoryVersion {
                    name: name.clone(),
                    path: file.clone(),
                    id: id.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            let shown = preview(paths, &name, &file, ConfigChange::Text { content }).await?;
            if !approve(out, &shown, yes)? {
                eprintln!("Nothing changed.");
                return Ok(ExitCode::SUCCESS);
            }
            daemon_call(
                paths,
                Request::ConfigRevert {
                    name,
                    path: file.clone(),
                    id,
                    base_hash: hash,
                },
            )
            .await?;
            eprintln!("Reverted {file}.");
        }
    }
    Ok(ExitCode::SUCCESS)
}
