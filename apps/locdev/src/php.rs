//! `locdev php …`: PHP settings per version line. LocDev's own ini file is
//! added to the sites it runs; a system php.ini is only read.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::Subcommand;
use locdev_core::model::RuntimeKind;
use locdev_core::paths::Paths;
use locdev_core::phpini::{PhpConfigView, PhpIniChange, PhpIniPreview};
use locdev_core::protocol::{Request, Response};

use crate::daemon_call;
use crate::output::Output;
use crate::work::confirm;

#[derive(Subcommand)]
pub(crate) enum PhpCommand {
    /// PHP version lines and whether LocDev sets anything for them.
    #[command(visible_alias = "ls")]
    Lines,
    /// A line's settings: PHP's own value, LocDev's and the one in use.
    Show {
        /// 8.3, 8.4… (default: LocDev's default PHP).
        #[arg(long)]
        line: Option<String>,
    },
    /// Set directives (`memory_limit=512M upload_max_filesize=64M`); running sites on the line restart.
    Set {
        #[arg(required = true, value_name = "KEY=VALUE")]
        values: Vec<String>,
        #[arg(long)]
        line: Option<String>,
        /// Don't ask before applying.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Drop directives from LocDev's file (PHP's own value again).
    Unset {
        #[arg(required = true)]
        keys: Vec<String>,
        #[arg(long)]
        line: Option<String>,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Remove every LocDev setting for the line.
    Reset {
        #[arg(long)]
        line: Option<String>,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Edit LocDev's file for the line in $EDITOR; checked by PHP before it's applied.
    Edit {
        #[arg(long)]
        line: Option<String>,
    },
    /// Print a file PHP loads for the line (its php.ini, its scan folder), read-only.
    File {
        path: PathBuf,
        #[arg(long)]
        line: Option<String>,
    },
    /// Earlier versions of LocDev's file for the line.
    History {
        #[arg(long)]
        line: Option<String>,
    },
    /// Go back to an earlier version (`locdev php history` lists them).
    Restore {
        id: String,
        #[arg(long)]
        line: Option<String>,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

/// The line asked for, else the one of LocDev's default PHP, else the first.
async fn line(paths: &Paths, line: Option<String>) -> anyhow::Result<String> {
    if let Some(line) = line {
        return Ok(line);
    }
    let Response::PhpLines { lines } = daemon_call(paths, Request::PhpConfigLines).await? else {
        bail!("unexpected response from daemon");
    };
    if lines.is_empty() {
        bail!("no PHP installed (`locdev runtime install php 8.4`)");
    }
    if let Response::Runtimes { runtimes } =
        daemon_call(paths, Request::ListRuntimes { refresh: false }).await?
        && let Some(default) = runtimes
            .iter()
            .find(|r| r.kind == RuntimeKind::Php)
            .and_then(|r| r.default.clone())
    {
        let wanted: String = default.split('.').take(2).collect::<Vec<_>>().join(".");
        if lines.iter().any(|l| l.line == wanted) {
            return Ok(wanted);
        }
    }
    Ok(lines[0].line.clone())
}

async fn view(paths: &Paths, line: &str) -> anyhow::Result<PhpConfigView> {
    let Response::PhpConfig { config, .. } = daemon_call(
        paths,
        Request::PhpConfig {
            line: line.to_string(),
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    Ok(config)
}

/// Shows the change, asks, applies it.
async fn change(
    paths: &Paths,
    out: &Output,
    line: String,
    change: PhpIniChange,
    yes: bool,
) -> anyhow::Result<()> {
    let Response::PhpConfig { preview, .. } = daemon_call(
        paths,
        Request::PhpConfigChange {
            line: line.clone(),
            change: change.clone(),
            dry_run: true,
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    let preview = preview.context("no preview from the daemon")?;
    if preview.before == preview.after {
        out.message("Nothing to change.");
        return Ok(());
    }
    if !out.is_json() {
        print_preview(&preview);
    }
    if !yes {
        confirm("Apply?")?;
    }
    let Response::PhpConfig {
        config, restarted, ..
    } = daemon_call(
        paths,
        Request::PhpConfigChange {
            line,
            change,
            dry_run: false,
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    out.php_config(&config);
    if !restarted.is_empty() && !out.is_json() {
        println!("\nRestarted: {}", restarted.join(", "));
    }
    Ok(())
}

fn print_preview(preview: &PhpIniPreview) {
    for diff in similar_lines(&preview.before, &preview.after) {
        println!("{diff}");
    }
    for warning in &preview.warnings {
        println!("! {warning}");
    }
    if preview.restart.is_empty() {
        println!("No site on this line is running.");
    } else {
        println!("Restarts: {}", preview.restart.join(", "));
    }
}

/// A minimal line diff: removed lines with `-`, added with `+`.
fn similar_lines(before: &str, after: &str) -> Vec<String> {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    let mut out: Vec<String> = old
        .iter()
        .filter(|l| !new.contains(l))
        .map(|l| format!("- {l}"))
        .collect();
    out.extend(
        new.iter()
            .filter(|l| !old.contains(l))
            .map(|l| format!("+ {l}")),
    );
    out
}

fn pairs(values: &[String]) -> anyhow::Result<BTreeMap<String, Option<String>>> {
    values
        .iter()
        .map(|raw| {
            let (k, v) = raw
                .split_once('=')
                .with_context(|| format!("'{raw}' is not KEY=VALUE"))?;
            Ok((k.trim().to_string(), Some(v.trim().to_string())))
        })
        .collect()
}

pub async fn run(command: PhpCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        PhpCommand::Lines => {
            let Response::PhpLines { lines } = daemon_call(paths, Request::PhpConfigLines).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.php_lines(&lines);
        }
        PhpCommand::Show { line: l } => {
            let line = line(paths, l).await?;
            out.php_config(&view(paths, &line).await?);
        }
        PhpCommand::Set {
            values,
            line: l,
            yes,
        } => {
            let line = line(paths, l).await?;
            let values = pairs(&values)?;
            change(paths, out, line, PhpIniChange::Set { values }, yes).await?;
        }
        PhpCommand::Unset { keys, line: l, yes } => {
            let line = line(paths, l).await?;
            let values = keys.into_iter().map(|k| (k, None)).collect();
            change(paths, out, line, PhpIniChange::Set { values }, yes).await?;
        }
        PhpCommand::Reset { line: l, yes } => {
            let line = line(paths, l).await?;
            change(paths, out, line, PhpIniChange::Reset, yes).await?;
        }
        PhpCommand::Restore { id, line: l, yes } => {
            let line = line(paths, l).await?;
            change(paths, out, line, PhpIniChange::Restore { id }, yes).await?;
        }
        PhpCommand::History { line: l } => {
            let line = line(paths, l).await?;
            out.php_history(&view(paths, &line).await?);
        }
        PhpCommand::File { path, line: l } => {
            let line = line(paths, l).await?;
            let path = std::path::absolute(&path)?;
            let Response::PhpIniFile { text, .. } =
                daemon_call(paths, Request::PhpIniRead { line, path }).await?
            else {
                bail!("unexpected response from daemon");
            };
            print!("{text}");
        }
        PhpCommand::Edit { line: l } => {
            if !std::io::stdin().is_terminal() {
                bail!("`locdev php edit` needs a terminal; use `locdev php set` in scripts");
            }
            let line = line(paths, l).await?;
            let config = view(paths, &line).await?;
            let scratch = tempfile::Builder::new()
                .prefix("locdev-php-")
                .suffix(".ini")
                .tempfile()?;
            let initial = if config.text.is_empty() {
                format!(
                    "; LocDev's PHP {line} settings. Lines you add are kept; save and quit to apply.\n"
                )
            } else {
                config.text.clone()
            };
            scratch.as_file().write_all(initial.as_bytes())?;
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        "notepad".into()
                    } else {
                        "vi".into()
                    }
                });
            let status = std::process::Command::new(&editor)
                .arg(scratch.path())
                .status()
                .with_context(|| format!("could not run {editor}"))?;
            if !status.success() {
                bail!("{editor} exited with {status}; nothing was changed");
            }
            let text = std::fs::read_to_string(scratch.path())?;
            if text == initial {
                out.message("No changes.");
                return Ok(ExitCode::SUCCESS);
            }
            let change_ = PhpIniChange::Text {
                text,
                hash: config.hash,
            };
            change(paths, out, line, change_, false).await?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
