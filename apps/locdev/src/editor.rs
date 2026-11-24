//! `locdev editor …`: the code editor sites and files open in.

use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::daemon_call;
use crate::output::Output;

#[derive(Subcommand)]
pub(crate) enum EditorCommand {
    /// Editors installed here and which one opens by default.
    #[command(visible_alias = "ls")]
    List {
        /// Look at the disk again (after installing one).
        #[arg(long)]
        refresh: bool,
    },
    /// Choose the editor sites open in (an id from `locdev editor list`).
    Set {
        /// `none` goes back to the first one found.
        editor: String,
    },
    /// Open a site's folder, or a file inside it (`src/app.ts:12[:4]`).
    Open {
        site: String,
        /// A path inside the site, optionally with `:line[:column]`.
        file: Option<String>,
        /// Another editor than the default one, just this time.
        #[arg(long)]
        editor: Option<String>,
    },
}

/// `src/app.ts:12:4` → (`src/app.ts`, 12, 4). A colon not followed by a
/// number stays part of the path.
fn split_position(raw: &str) -> (String, Option<u32>, Option<u32>) {
    let mut parts: Vec<&str> = raw.rsplitn(3, ':').collect();
    parts.reverse();
    let numbers: Vec<Option<u32>> = parts.iter().map(|p| p.parse().ok()).collect();
    match parts.as_slice() {
        [path, _, _] if numbers[1].is_some() && numbers[2].is_some() => {
            (path.to_string(), numbers[1], numbers[2])
        }
        [_, _, _] if numbers[2].is_some() => {
            let cut = raw.rfind(':').unwrap();
            (raw[..cut].to_string(), numbers[2], None)
        }
        [path, _] if numbers[1].is_some() => (path.to_string(), numbers[1], None),
        _ => (raw.to_string(), None, None),
    }
}

pub async fn run(command: EditorCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        EditorCommand::List { refresh } => {
            let Response::Editors(view) =
                daemon_call(paths, Request::EditorList { refresh }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.editors(&view);
        }
        EditorCommand::Set { editor } => {
            let Response::Editors(view) =
                daemon_call(paths, Request::EditorList { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let preferred = if editor == "none" {
                None
            } else if locdev_core::editor::def(&editor).is_none() {
                bail!("unknown editor {editor}; see `locdev editor list`");
            } else {
                if !view.editors.iter().any(|e| e.id == editor) {
                    eprintln!(
                        "note: {editor} isn't installed here yet; the first one found opens until it is."
                    );
                }
                Some(editor)
            };
            let Response::Settings(mut settings) = daemon_call(paths, Request::GetSettings).await?
            else {
                bail!("unexpected response from daemon");
            };
            settings.editor.preferred = preferred;
            daemon_call(paths, Request::UpdateSettings { settings }).await?;
            let Response::Editors(view) =
                daemon_call(paths, Request::EditorList { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.editors(&view);
        }
        EditorCommand::Open { site, file, editor } => {
            let (path, line, column) = match file.as_deref() {
                Some(raw) => {
                    let (path, line, column) = split_position(raw);
                    (Some(path), line, column)
                }
                None => (None, None, None),
            };
            daemon_call(
                paths,
                Request::EditorOpen {
                    site: site.clone(),
                    path: path.clone(),
                    line,
                    column,
                    editor,
                },
            )
            .await?;
            out.message(&match path {
                Some(path) => format!("Opened {path} from {site}."),
                None => format!("Opened {site}."),
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::split_position;

    #[test]
    fn positions() {
        assert_eq!(split_position("src/a.ts"), ("src/a.ts".into(), None, None));
        assert_eq!(
            split_position("src/a.ts:12"),
            ("src/a.ts".into(), Some(12), None)
        );
        assert_eq!(
            split_position("src/a.ts:12:4"),
            ("src/a.ts".into(), Some(12), Some(4))
        );
        assert_eq!(split_position("a:b.ts:3"), ("a:b.ts".into(), Some(3), None));
        assert_eq!(split_position("a:b.ts"), ("a:b.ts".into(), None, None));
    }
}
