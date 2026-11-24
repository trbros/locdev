//! `locdev new email …` and `locdev email …`: the Email Studio from the terminal.

use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use locdev_core::email::{EmailDoc, PreviewMode, Segment};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum NewCommand {
    /// A new email template (Laravel: a Mailable via `make:mail` plus its view), or a component.
    Email(NewEmail),
    /// A new project, created with the framework's own installer (the plan is shown first).
    Project(crate::scaffold::NewProjectArgs),
    /// The starters `locdev new project` can create, with their methods and options.
    #[command(visible_alias = "ls")]
    List,
    /// `locdev new <starter> <name> [options]`: the same as `locdev new project`.
    #[command(external_subcommand)]
    Starter(Vec<String>),
}

#[derive(Args)]
pub(crate) struct NewArgs {
    /// Without one, `locdev new` asks what to create (in a terminal).
    #[command(subcommand)]
    command: Option<NewCommand>,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub(crate) struct NewEmail {
    #[command(subcommand)]
    component: Option<NewEmailComponent>,
    /// The template's name (`contact`, `order-shipped`).
    #[arg(required = true)]
    name: Option<String>,
    /// The site; defaults to the one in the current folder.
    #[arg(long)]
    site: Option<String>,
    /// A Laravel Markdown mailable instead of a Blade view.
    #[arg(long)]
    markdown: bool,
}

#[derive(Subcommand)]
pub(crate) enum NewEmailComponent {
    /// A reusable email component (Laravel: <x-mail.name>, props from @props).
    Component {
        name: String,
        #[arg(long)]
        site: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum EmailCommand {
    /// The site's email templates, mailables and components.
    #[command(visible_alias = "ls")]
    List { site: Option<String> },
    /// Render a template with its sample data (static, or by the framework).
    Preview {
        /// A path from `locdev email list`.
        path: String,
        site: Option<String>,
        /// Let the framework render it (runs the project's code in a separate process).
        #[arg(long)]
        framework: bool,
        /// Print the text version instead of the HTML.
        #[arg(long)]
        text: bool,
    },
    /// Render a template and send it to the Mailbox (never anywhere else).
    SendTest {
        path: String,
        site: Option<String>,
        #[arg(long)]
        framework: bool,
    },
    /// A template's blocks (LocDev's) and code (yours), variables and sample data.
    Show { path: String, site: Option<String> },
    /// Change a block's properties, like the visual editor (the diff is shown first).
    Block {
        path: String,
        /// The block's id (`locdev email show`).
        id: String,
        /// `prop=value`; JSON values (`true`, `12`, `"…"`) are parsed as JSON.
        #[arg(long = "set", value_name = "PROP=VALUE")]
        props: Vec<String>,
        /// Regenerate a block edited outside LocDev from its properties.
        #[arg(long)]
        regenerate: bool,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Edit the template in $VISUAL/$EDITOR; saved with history, only if
    /// nothing else changed it meanwhile.
    Edit { path: String, site: Option<String> },
    /// Replace the template with a file's content (history kept).
    Save {
        path: String,
        /// The new content (`-` for stdin).
        #[arg(long)]
        from: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Versions LocDev kept before each save.
    History { path: String, site: Option<String> },
    /// Print a kept version.
    Version {
        path: String,
        id: String,
        #[arg(long)]
        site: Option<String>,
    },
    /// Put a kept version back (the current one is kept too).
    Restore {
        path: String,
        id: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// Set the sample data previews use (a JSON object; kept by LocDev, not in the project).
    Sample {
        path: String,
        /// A JSON file (`-` for stdin).
        data: String,
        #[arg(long)]
        site: Option<String>,
    },
}

pub async fn new(args: NewArgs, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let args = match args.command {
        None => return crate::scaffold::wizard(paths, out).await,
        Some(NewCommand::Email(args)) => args,
        Some(NewCommand::Project(args)) => {
            return crate::scaffold::new_project(args, paths, out).await;
        }
        Some(NewCommand::List) => return crate::scaffold::list(paths, out).await,
        Some(NewCommand::Starter(argv)) => {
            return crate::scaffold::shortcut(argv, paths, out).await;
        }
    };
    let (component, item, site, markdown) = match args.component {
        Some(NewEmailComponent::Component { name, site }) => (true, name, site, false),
        None => (
            false,
            args.name.unwrap_or_default(),
            args.site,
            args.markdown,
        ),
    };
    let name = site::resolve(paths, site).await?.project.name;
    let Response::EmailCreated(created) = daemon_call(
        paths,
        Request::EmailNew {
            name,
            component,
            item,
            markdown,
        },
    )
    .await?
    else {
        bail!("unexpected response from daemon");
    };
    out.email_created(&created);
    Ok(ExitCode::SUCCESS)
}

pub async fn run(command: EmailCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        EmailCommand::List { site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::EmailCatalog(catalog) =
                daemon_call(paths, Request::EmailCatalog { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.email_catalog(&catalog);
        }
        EmailCommand::Preview {
            path,
            site,
            framework,
            text,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::EmailDoc(doc) = daemon_call(
                paths,
                Request::EmailOpen {
                    name: name.clone(),
                    path: path.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            let mode = if framework {
                PreviewMode::Framework
            } else {
                PreviewMode::Static
            };
            let Response::EmailPreview(preview) = daemon_call(
                paths,
                Request::EmailPreview {
                    name,
                    path,
                    segments: None,
                    theme: None,
                    data: doc.sample,
                    mode,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            if let Some(error) = preview.error {
                bail!(error);
            }
            for note in &preview.notes {
                eprintln!("note: {note}");
            }
            if out.is_json() {
                out.print_json(&preview);
            } else if text {
                println!("{}", preview.text.unwrap_or_default());
            } else {
                print!("{}", preview.html);
            }
        }
        EmailCommand::SendTest {
            path,
            site,
            framework,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::EmailDoc(doc) = daemon_call(
                paths,
                Request::EmailOpen {
                    name: name.clone(),
                    path: path.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            let mode = if framework {
                PreviewMode::Framework
            } else {
                PreviewMode::Static
            };
            daemon_call(
                paths,
                Request::EmailSendTest {
                    name,
                    path,
                    segments: None,
                    theme: None,
                    data: doc.sample,
                    mode,
                    to: None,
                },
            )
            .await?;
            eprintln!("Sent to the Mailbox (`locdev mail list`).");
        }
        EmailCommand::Show { path, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            out.email_doc(&open(paths, &name, &path).await?);
        }
        EmailCommand::Block {
            path,
            id,
            props,
            regenerate,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let mut doc = open(paths, &name, &path).await?;
            let Some(Segment::Block {
                props: values,
                edited,
                ..
            }) = doc
                .segments
                .iter_mut()
                .find(|s| matches!(s, Segment::Block { id: b, .. } if *b == id))
            else {
                bail!("{path} has no block {id} (see `locdev email show {path}`)");
            };
            for prop in &props {
                let (key, value) = prop
                    .split_once('=')
                    .with_context(|| format!("`{prop}`: use prop=value"))?;
                let value = serde_json::from_str(value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
                match values {
                    serde_json::Value::Object(map) => {
                        map.insert(key.to_string(), value);
                    }
                    _ => bail!("block {id} has no properties to set"),
                }
            }
            if regenerate {
                *edited = false;
            } else if *edited && !props.is_empty() {
                bail!(
                    "block {id} was edited outside LocDev and is kept as written; add --regenerate to rebuild it from its properties"
                );
            }
            let Response::EmailBuilt { content, diff } = daemon_call(
                paths,
                Request::EmailBuild {
                    name: name.clone(),
                    path: path.clone(),
                    segments: doc.segments,
                    theme: doc.theme,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            save(paths, out, &name, &path, content, &diff, doc.hash, yes).await?;
        }
        EmailCommand::Edit { path, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let doc = open(paths, &name, &path).await?;
            let edited = edit_in_editor(&path, &doc.content)?;
            if edited == doc.content {
                eprintln!("No changes.");
                return Ok(ExitCode::SUCCESS);
            }
            // Already reviewed in the editor: save without asking again.
            save(paths, out, &name, &path, edited, "", doc.hash, true).await?;
        }
        EmailCommand::Save {
            path,
            from,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let doc = open(paths, &name, &path).await?;
            let content = read_input(&from)?;
            if content == doc.content {
                eprintln!("No changes.");
                return Ok(ExitCode::SUCCESS);
            }
            save(paths, out, &name, &path, content, "", doc.hash, yes).await?;
        }
        EmailCommand::History { path, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let Response::ConfigHistory { entries } =
                daemon_call(paths, Request::EmailHistory { name, path }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.config_history(&entries);
        }
        EmailCommand::Version { path, id, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let content = version(paths, &name, &path, &id).await?;
            if out.is_json() {
                out.print_json(serde_json::json!({ "content": content }));
            } else {
                print!("{content}");
            }
        }
        EmailCommand::Restore {
            path,
            id,
            site,
            yes,
        } => {
            let name = site::resolve(paths, site).await?.project.name;
            let doc = open(paths, &name, &path).await?;
            let content = version(paths, &name, &path, &id).await?;
            save(paths, out, &name, &path, content, "", doc.hash, yes).await?;
        }
        EmailCommand::Sample { path, data, site } => {
            let name = site::resolve(paths, site).await?.project.name;
            let data: serde_json::Value =
                serde_json::from_str(&read_input(&data)?).context("the sample data isn't JSON")?;
            if !data.is_object() {
                bail!("the sample data must be a JSON object ({{\"user\": …}})");
            }
            daemon_call(paths, Request::EmailSample { name, path, data }).await?;
            out.message("sample data saved");
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn open(paths: &Paths, name: &str, path: &str) -> anyhow::Result<EmailDoc> {
    match daemon_call(
        paths,
        Request::EmailOpen {
            name: name.to_string(),
            path: path.to_string(),
        },
    )
    .await?
    {
        Response::EmailDoc(doc) => Ok(*doc),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

async fn version(paths: &Paths, name: &str, path: &str, id: &str) -> anyhow::Result<String> {
    match daemon_call(
        paths,
        Request::EmailHistoryVersion {
            name: name.to_string(),
            path: path.to_string(),
            id: id.to_string(),
        },
    )
    .await?
    {
        Response::ConfigHistoryVersion { content } => Ok(content),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

/// Shows the diff (when there is one), asks, and saves against `base_hash`:
/// the daemon refuses when the file changed since it was read.
#[allow(clippy::too_many_arguments)]
async fn save(
    paths: &Paths,
    out: &Output,
    name: &str,
    path: &str,
    content: String,
    diff: &str,
    base_hash: String,
    yes: bool,
) -> anyhow::Result<()> {
    if !diff.is_empty() {
        out.text(diff, false);
    }
    if !yes {
        crate::work::confirm(&format!("Save {path}?"))?;
    }
    daemon_call(
        paths,
        Request::EmailSave {
            name: name.to_string(),
            path: path.to_string(),
            content,
            base_hash,
        },
    )
    .await?;
    out.message(&format!(
        "saved {path} (the previous version is in `locdev email history {path}`)"
    ));
    Ok(())
}

fn read_input(source: &str) -> anyhow::Result<String> {
    if source == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(source).with_context(|| format!("could not read {source}"))
    }
}

/// Opens a private temporary copy in the user's editor and returns what it
/// holds when the editor exits. `$VISUAL`/`$EDITOR` may carry arguments
/// (`code --wait`); they are split without a shell.
fn edit_in_editor(path: &str, content: &str) -> anyhow::Result<String> {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "notepad".into()
            } else {
                "vi".into()
            }
        });
    let mut words = crate::tui::split_args(&editor).map_err(anyhow::Error::msg)?;
    if words.is_empty() {
        bail!("$EDITOR is empty");
    }
    let program = words.remove(0);
    let file_name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "template.blade.php".into());
    let dir = tempfile::Builder::new().prefix("locdev-email-").tempdir()?;
    let file = dir.path().join(file_name);
    std::fs::write(&file, content)?;
    // The CLI's own console: a terminal editor needs it (no CREATE_NO_WINDOW).
    let status = std::process::Command::new(&program)
        .args(words)
        .arg(&file)
        .status()
        .with_context(|| format!("could not start the editor `{program}` (set $EDITOR)"))?;
    if !status.success() {
        bail!("the editor exited with {status}; nothing was saved");
    }
    Ok(std::fs::read_to_string(&file)?)
}
