//! The GitHub Actions builder: templates from the project's stack, a model
//! the visual editor edits, YAML in and out, validation, and saving into
//! `.github/workflows/` without ever overwriting a file by surprise.

pub mod parse;
pub mod templates;
pub mod validate;
pub mod versions;
pub mod yaml;

use std::io::Write;
use std::path::{Path, PathBuf};

use locdev_core::actions::{ExistingWorkflow, SavePlan, Validation};
use sha2::{Digest, Sha256};

pub use templates::Facts;
pub use versions::Versions;

const DIR: &str = ".github/workflows";
/// Workflow files larger than this aren't opened.
const MAX_SIZE: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsError(pub String);

impl std::fmt::Display for ActionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

type Result<T> = std::result::Result<T, ActionsError>;

fn err(message: impl Into<String>) -> ActionsError {
    ActionsError(message.into())
}

/// A bare `name.yml`/`name.yaml`: no folders, no hidden files.
pub fn check_file_name(file: &str) -> Result<()> {
    let stem_ok = file
        .strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))
        .is_some_and(|stem| {
            !stem.is_empty()
                && !stem.starts_with(['.', '-'])
                && stem
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        });
    if stem_ok && file.len() <= 100 {
        Ok(())
    } else {
        Err(err(format!(
            "`{file}`: use a name like ci.yml (letters, digits, - _ .)"
        )))
    }
}

/// `<project>/.github/workflows`, refusing a folder that leads outside the
/// project through a symlink. The nearest part that exists is checked, so
/// a `.github` symlink is caught before anything is created through it.
fn workflows_dir(project: &Path) -> Result<PathBuf> {
    let dir = project.join(DIR);
    let existing = dir.ancestors().find(|p| p.exists()).unwrap_or(project);
    if let (Ok(root), Ok(real)) = (project.canonicalize(), existing.canonicalize())
        && !real.starts_with(&root)
    {
        return Err(err(format!("{DIR} points outside the project")));
    }
    Ok(dir)
}

fn target(project: &Path, file: &str) -> Result<PathBuf> {
    check_file_name(file)?;
    let path = workflows_dir(project)?.join(file);
    if path.is_symlink() {
        return Err(err(format!(
            "{DIR}/{file} is a symlink; LocDev won't write through it"
        )));
    }
    Ok(path)
}

pub fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn existing(project: &Path) -> Vec<ExistingWorkflow> {
    let Ok(dir) = workflows_dir(project) else {
        return vec![];
    };
    let mut list: Vec<ExistingWorkflow> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let file = entry.file_name().to_string_lossy().to_string();
            check_file_name(&file).ok()?;
            let name = read(project, &file)
                .ok()
                .and_then(|(text, _)| serde_norway::from_str::<serde_norway::Value>(&text).ok())
                .and_then(|v| v.get("name")?.as_str().map(str::to_string));
            Some(ExistingWorkflow { file, name })
        })
        .collect();
    list.sort_by(|a, b| a.file.cmp(&b.file));
    list
}

/// A workflow file's text and its hash.
pub fn read(project: &Path, file: &str) -> Result<(String, String)> {
    let path = target(project, file)?;
    let meta = std::fs::metadata(&path).map_err(|e| err(format!("{DIR}/{file}: {e}")))?;
    if meta.len() > MAX_SIZE {
        return Err(err(format!("{DIR}/{file} is too large to open here")));
    }
    let bytes = std::fs::read(&path).map_err(|e| err(format!("{DIR}/{file}: {e}")))?;
    let digest = hash(&bytes);
    let text =
        String::from_utf8(bytes).map_err(|_| err(format!("{DIR}/{file} isn't UTF-8 text")))?;
    Ok((text, digest))
}

/// What saving `text` as `file` would change.
pub fn plan(project: &Path, file: &str, text: &str, validation: Validation) -> Result<SavePlan> {
    let path = target(project, file)?;
    let current = if path.exists() {
        Some(read(project, file)?)
    } else {
        None
    };
    let old = current.as_ref().map(|(t, _)| t.as_str()).unwrap_or("");
    let relative = format!("{DIR}/{file}");
    let diff = similar::TextDiff::from_lines(old, text)
        .unified_diff()
        .context_radius(3)
        .header(
            if current.is_some() {
                &relative
            } else {
                "/dev/null"
            },
            &relative,
        )
        .to_string();
    Ok(SavePlan {
        path: relative,
        exists: current.is_some(),
        current_hash: current.map(|(_, h)| h),
        diff,
        validation,
    })
}

/// Writes `text` to `file`. A new file is created only if nothing is there
/// (checked atomically); an existing one is replaced only when `replace` is
/// the hash of what's there now, i.e. what the user saw in the diff.
pub fn save(project: &Path, file: &str, text: &str, replace: Option<&str>) -> Result<String> {
    let path = target(project, file)?;
    let relative = format!("{DIR}/{file}");
    match replace {
        None => {
            std::fs::create_dir_all(path.parent().unwrap_or(project))
                .map_err(|e| err(format!("{DIR}: {e}")))?;
            // Re-check after creating the folders.
            let path = target(project, file)?;
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::AlreadyExists => err(format!(
                        "{relative} already exists; review the changes and replace it explicitly"
                    )),
                    _ => err(format!("{relative}: {e}")),
                })?;
            out.write_all(text.as_bytes())
                .and_then(|_| out.sync_all())
                .map_err(|e| err(format!("{relative}: {e}")))?;
        }
        Some(expected) => {
            let (_, current) = read(project, file)?;
            if current != expected {
                return Err(err(format!(
                    "{relative} changed since you reviewed it; review the changes again"
                )));
            }
            // Temp file next to it, then rename: never a half-written workflow.
            let tmp = path.with_extension("locdev-tmp");
            std::fs::write(&tmp, text)
                .and_then(|_| std::fs::rename(&tmp, &path))
                .map_err(|e| {
                    let _ = std::fs::remove_file(&tmp);
                    err(format!("{relative}: {e}"))
                })?;
        }
    }
    Ok(relative)
}

#[cfg(test)]
mod tests;
