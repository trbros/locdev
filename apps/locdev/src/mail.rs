//! `locdev mail …`: LocDev's Mailbox (capture-only SMTP on loopback).

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum MailCommand {
    /// The capture server: port and how many messages it holds.
    Status,
    /// Captured messages, newest first.
    #[command(visible_alias = "ls")]
    List {
        /// Only this site's box.
        site: Option<String>,
        /// Text to find in the subject, sender, recipients or preview.
        #[arg(long, short)]
        search: Option<String>,
        #[arg(long)]
        unread: bool,
        #[arg(long)]
        attachments: bool,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// One message: envelope, headers, text, links, attachments and problems.
    Show {
        id: String,
        /// Print the HTML part instead.
        #[arg(long, conflicts_with = "headers")]
        html: bool,
        /// Print every header.
        #[arg(long)]
        headers: bool,
    },
    /// Save the message as received (.eml).
    Raw {
        id: String,
        /// File to write; prints to stdout without it.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Save an attachment.
    Attachment {
        id: String,
        index: usize,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Delete messages.
    #[command(visible_alias = "rm")]
    Delete {
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// Delete every message, or one site's.
    Clear {
        site: Option<String>,
        #[arg(long, short)]
        yes: bool,
    },
    /// How the site sends mail now and what turning the Mailbox on changes.
    Setup {
        site: Option<String>,
    },
    /// Send the site's mail to the Mailbox (in its processes' environment only).
    Enable {
        site: Option<String>,
        /// Replace the project's real SMTP or API transport while it's on.
        #[arg(long)]
        override_real: bool,
    },
    Disable {
        site: Option<String>,
    },
    /// Send a test message to the Mailbox (never anywhere else).
    Test {
        site: Option<String>,
    },
}

fn decode(b64: &str) -> anyhow::Result<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(b64.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0);
    for c in b64.bytes().filter(|c| *c != b'=') {
        let v = TABLE
            .iter()
            .position(|t| *t == c)
            .context("bad base64 from daemon")? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

fn write_out(path: Option<PathBuf>, bytes: &[u8], what: &str) -> anyhow::Result<()> {
    match path {
        Some(path) => {
            // Never over an existing file.
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| format!("{} (LocDev doesn't overwrite files)", path.display()))?;
            std::io::Write::write_all(&mut file, bytes)?;
            eprintln!("Saved {what} to {}", path.display());
        }
        None => std::io::Write::write_all(&mut std::io::stdout(), bytes)?,
    }
    Ok(())
}

pub async fn run(command: MailCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        MailCommand::Status => {
            let Response::MailboxStatus(status) =
                daemon_call(paths, Request::MailboxStatus).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mailbox_status(&status);
        }
        MailCommand::List {
            site,
            search,
            unread,
            attachments,
            limit,
        } => {
            let site = match site {
                Some(s) => Some(site::resolve(paths, Some(s)).await?.project.name),
                None => None,
            };
            let Response::MailList { messages } = daemon_call(
                paths,
                Request::MailList {
                    site,
                    query: search,
                    unread,
                    attachments,
                    limit: Some(limit),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mail_list(&messages);
        }
        MailCommand::Show { id, html, headers } => {
            let Response::MailDetail(detail) = daemon_call(paths, Request::MailGet { id }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mail_detail(&detail, html, headers);
        }
        MailCommand::Raw { id, output } => {
            let Response::MailRaw { base64 } = daemon_call(paths, Request::MailRaw { id }).await?
            else {
                bail!("unexpected response from daemon");
            };
            write_out(output, &decode(&base64)?, "the message")?;
        }
        MailCommand::Attachment { id, index, output } => {
            let Response::MailAttachment { name, base64, .. } =
                daemon_call(paths, Request::MailAttachment { id, index }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let bytes = decode(&base64)?;
            if output.is_none() && std::io::stdout().is_terminal() {
                bail!(
                    "{} is binary; pass --output <file>",
                    name.as_deref().unwrap_or("the attachment")
                );
            }
            write_out(output, &bytes, name.as_deref().unwrap_or("the attachment"))?;
        }
        MailCommand::Delete { ids } => {
            let Response::MailDeleted { count } =
                daemon_call(paths, Request::MailDelete { ids }).await?
            else {
                bail!("unexpected response from daemon");
            };
            eprintln!("Deleted {count} message(s).");
        }
        MailCommand::Clear { site, yes } => {
            let site = match site {
                Some(s) => Some(site::resolve(paths, Some(s)).await?.project.name),
                None => None,
            };
            if !yes {
                if !std::io::stdin().is_terminal() {
                    bail!("run it on a terminal to confirm, or pass --yes");
                }
                eprint!(
                    "Delete every message{}? [y/N] ",
                    site.as_deref()
                        .map(|s| format!(" from {s}"))
                        .unwrap_or_default()
                );
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if !matches!(answer.trim(), "y" | "Y" | "yes") {
                    return Ok(ExitCode::SUCCESS);
                }
            }
            let Response::MailDeleted { count } =
                daemon_call(paths, Request::MailClear { site }).await?
            else {
                bail!("unexpected response from daemon");
            };
            eprintln!("Deleted {count} message(s).");
        }
        MailCommand::Setup { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::MailSetup(setup) =
                daemon_call(paths, Request::MailSetup { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mail_setup(&setup);
        }
        MailCommand::Enable {
            site,
            override_real,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::MailSetup(setup) = daemon_call(
                paths,
                Request::MailEnable {
                    name: name.clone(),
                    enabled: true,
                    override_real,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mail_setup(&setup);
            eprintln!(
                "Restart {name} so its processes get the new variables (`locdev reload {name}`)."
            );
        }
        MailCommand::Disable { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::MailSetup(setup) = daemon_call(
                paths,
                Request::MailEnable {
                    name: name.clone(),
                    enabled: false,
                    override_real: false,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.mail_setup(&setup);
            eprintln!("Restart {name} so its processes stop getting the variables.");
        }
        MailCommand::Test { site } => {
            let name = match site {
                Some(s) => Some(site::resolve(paths, Some(s)).await?.project.name),
                None => None,
            };
            daemon_call(paths, Request::MailSendTest { name, to: None }).await?;
            eprintln!("Sent a test message to the Mailbox (`locdev mail list`).");
        }
    }
    Ok(ExitCode::SUCCESS)
}
