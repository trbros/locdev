//! Puts LocDev's shim folder (`locdev_runtimes::terminal`) first on the
//! user's PATH, so a new terminal runs the default PHP/Node.js/… versions.
//! Per user, no administrator rights, and only on request:
//!
//! - bash and zsh: a marked block at the end of `~/.bashrc` / `~/.zshrc`
//!   (the files that exist, plus the one of the login shell). Removing it
//!   takes out exactly that block and nothing else.
//! - fish: a file of LocDev's own, `~/.config/fish/conf.d/locdev.fish`.
//! - Windows: the folder at the front of the user's `Path`. The system
//!   `Path` still comes first there, so a PHP/Node.js installed for all
//!   users wins over it.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::MARKER;

const BEGIN: &str = "# >>> Managed by LocDev: runtimes >>>";
const END: &str = "# <<< Managed by LocDev: runtimes <<<";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPath {
    /// Whether new terminals get the folder.
    pub enabled: bool,
    /// Files that carry LocDev's lines (empty on Windows).
    pub files: Vec<PathBuf>,
    /// Whether the folder is already on this process' PATH; otherwise a
    /// new terminal is needed to see it.
    pub active: bool,
}

pub fn status(dir: &Path) -> TerminalPath {
    let active =
        std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d == dir));
    #[cfg(windows)]
    {
        TerminalPath {
            enabled: windows_user_path().is_some_and(|p| p.split(';').any(|d| Path::new(d) == dir)),
            files: Vec::new(),
            active,
        }
    }
    #[cfg(not(windows))]
    {
        let files: Vec<PathBuf> = home()
            .map(|home| {
                candidates(&home)
                    .into_iter()
                    .filter(|f| {
                        std::fs::read_to_string(f).is_ok_and(|s| {
                            s.contains(MARKER) && s.contains(&dir.display().to_string())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        TerminalPath {
            enabled: !files.is_empty(),
            files,
            active,
        }
    }
}

pub fn enable(dir: &Path) -> io::Result<TerminalPath> {
    #[cfg(windows)]
    {
        crate::cli::windows_path(dir, true, true)?;
    }
    #[cfg(not(windows))]
    {
        let home = home().ok_or_else(|| io::Error::other("no home folder"))?;
        let shell = std::env::var("SHELL").unwrap_or_default();
        for file in targets(&home, &shell) {
            if file.extension().is_some_and(|e| e == "fish") {
                if let Some(parent) = file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&file, fish(dir))?;
            } else {
                let current = std::fs::read_to_string(&file).unwrap_or_default();
                std::fs::write(&file, with_block(&current, Some(dir)))?;
            }
        }
    }
    Ok(status(dir))
}

pub fn disable(dir: &Path) -> io::Result<TerminalPath> {
    #[cfg(windows)]
    {
        crate::cli::windows_path(dir, false, false)?;
    }
    #[cfg(not(windows))]
    if let Some(home) = home() {
        for file in candidates(&home) {
            let Ok(current) = std::fs::read_to_string(&file) else {
                continue;
            };
            if !current.contains(MARKER) {
                continue;
            }
            if file.extension().is_some_and(|e| e == "fish") {
                std::fs::remove_file(&file)?;
            } else {
                let without = with_block(&current, None);
                if without != current {
                    std::fs::write(&file, without)?;
                }
            }
        }
    }
    Ok(status(dir))
}

#[cfg(not(windows))]
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Every file LocDev may write to.
#[cfg_attr(windows, allow(dead_code))]
fn candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".bashrc"),
        home.join(".zshrc"),
        home.join(".config/fish/conf.d/locdev.fish"),
    ]
}

/// The files to write: the startup files that exist, plus the login
/// shell's own (created when missing).
#[cfg_attr(windows, allow(dead_code))]
fn targets(home: &Path, shell: &str) -> Vec<PathBuf> {
    let shell = shell.rsplit('/').next().unwrap_or_default();
    let mut out = Vec::new();
    for (file, name) in [(".bashrc", "bash"), (".zshrc", "zsh")] {
        let path = home.join(file);
        if path.is_file() || shell == name {
            out.push(path);
        }
    }
    if home.join(".config/fish").is_dir() || shell == "fish" {
        out.push(home.join(".config/fish/conf.d/locdev.fish"));
    }
    if out.is_empty() {
        out.push(home.join(".bashrc"));
    }
    out
}

fn quoted(dir: &Path) -> String {
    format!("'{}'", dir.display().to_string().replace('\'', r"'\''"))
}

/// `contents` without LocDev's block, then (with `dir`) a fresh block at
/// the end. Everything else stays byte for byte.
#[cfg_attr(windows, allow(dead_code))]
fn with_block(contents: &str, dir: Option<&Path>) -> String {
    let mut out = String::with_capacity(contents.len() + 200);
    let mut inside = false;
    for line in contents.split_inclusive('\n') {
        let trimmed = line.trim_end();
        if trimmed == BEGIN {
            inside = true;
            continue;
        }
        if inside {
            if trimmed == END {
                inside = false;
            }
            continue;
        }
        out.push_str(line);
    }
    if let Some(dir) = dir {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(
            "{BEGIN}\n# The default PHP, Node.js and package manager versions chosen in LocDev.\n\
             # Remove it from LocDev (Runtimes) or with `locdev runtime terminal off`.\n\
             export PATH={}:\"$PATH\"\n{END}\n",
            quoted(dir)
        ));
    }
    out
}

#[cfg_attr(windows, allow(dead_code))]
fn fish(dir: &Path) -> String {
    format!(
        "# {MARKER}: the default PHP, Node.js and package manager versions chosen in LocDev.\n\
         # Remove it from LocDev (Runtimes) or with `locdev runtime terminal off`.\n\
         fish_add_path --path --prepend {}\n",
        quoted(dir)
    )
}

#[cfg(windows)]
fn windows_user_path() -> Option<String> {
    let output = locdev_core::process::command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','User')",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_added_once_and_removed_exactly() {
        let dir = Path::new("/home/u/.local/share/locdev/bin");
        let original = "alias ll='ls -l'\nexport EDITOR=vim";
        let added = with_block(original, Some(dir));
        assert!(added.starts_with("alias ll='ls -l'\nexport EDITOR=vim\n# >>> Managed by LocDev"));
        assert!(added.contains("export PATH='/home/u/.local/share/locdev/bin':\"$PATH\"\n"));
        // Again: still one block.
        let again = with_block(&added, Some(dir));
        assert_eq!(again.matches(BEGIN).count(), 1);
        assert_eq!(again, added);
        // Off: the user's lines come back (plus the newline the block needed).
        assert_eq!(with_block(&added, None), format!("{original}\n"));
        assert_eq!(with_block("a\nb\n", None), "a\nb\n");
    }

    #[test]
    fn writes_the_existing_files_and_the_login_shell() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        assert_eq!(targets(h, "/bin/zsh"), [h.join(".zshrc")]);
        std::fs::write(h.join(".bashrc"), "").unwrap();
        assert_eq!(
            targets(h, "/bin/zsh"),
            [h.join(".bashrc"), h.join(".zshrc")]
        );
        std::fs::create_dir_all(h.join(".config/fish")).unwrap();
        assert!(targets(h, "/usr/bin/bash").contains(&h.join(".config/fish/conf.d/locdev.fish")));
        let nothing = tempfile::tempdir().unwrap();
        assert_eq!(
            targets(nothing.path(), ""),
            [nothing.path().join(".bashrc")]
        );
    }

    #[test]
    fn paths_are_quoted_for_the_shell() {
        assert_eq!(quoted(Path::new("/a b/it's")), r"'/a b/it'\''s'");
        assert!(fish(Path::new("/x")).contains("fish_add_path --path --prepend '/x'"));
    }
}
