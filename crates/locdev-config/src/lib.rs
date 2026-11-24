//! Config Files: the configuration files a project already has, a form for
//! the options LocDev understands, and safe saving.
//!
//! Nothing here executes project code: JavaScript configs are parsed, JSON
//! goes through a concrete syntax tree, and every write is checked against
//! the hash the user reviewed, recorded in the history first and written
//! atomically. Files are never created.

pub mod catalog;
pub mod files;
pub mod history;
pub mod js;
pub mod json;
pub mod rules;

use std::path::Path;

use locdev_core::config::{
    ConfigChange, ConfigFile, ConfigFileView, ConfigFormat, ConfigOption, ConfigPreview,
    OptionState,
};
use locdev_core::semver::Version;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::catalog::{Kind, Opt};
use crate::files::Versions;
use crate::history::History;

pub fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_info(kind: &Kind, rel: &str, format: ConfigFormat, versions: &Versions) -> ConfigFile {
    ConfigFile {
        path: rel.to_string(),
        kind: kind.id.to_string(),
        label: kind.label.to_string(),
        format,
        package: kind.package.map(str::to_string),
        version: kind
            .package
            .and_then(|p| versions.get(p))
            .map(|v| v.to_string()),
        docs: kind.docs.to_string(),
        visual: !kind.options.is_empty()
            && matches!(
                format,
                ConfigFormat::Json | ConfigFormat::Jsonc | ConfigFormat::Js
            ),
    }
}

fn read_text(path: &Path, rel: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{rel}: {e}"))?;
    String::from_utf8(bytes).map_err(|_| format!("{rel} isn't UTF-8 text"))
}

/// The project's config files.
pub fn list(root: &Path) -> Vec<ConfigFile> {
    let versions = Versions::read(root);
    files::discover(root)
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(root.join(&f.rel)).unwrap_or_default();
            file_info(
                f.kind,
                &f.rel,
                files::format(f.kind, &f.rel, &text),
                &versions,
            )
        })
        .collect()
}

/// The syntax error in `text`, if the format has a parser here.
pub fn syntax(format: ConfigFormat, rel: &str, text: &str) -> Option<String> {
    match format {
        ConfigFormat::Json => json::check(text, true),
        ConfigFormat::Jsonc => json::check(text, false),
        ConfigFormat::Js => js::check(text, rel),
        ConfigFormat::Toml => toml::from_str::<toml::Table>(text)
            .err()
            .map(|e| e.to_string()),
        ConfigFormat::Yaml => serde_norway::from_str::<serde_norway::Value>(text)
            .err()
            .map(|e| e.to_string()),
        ConfigFormat::Xml => xml_error(text),
        ConfigFormat::Css => None,
    }
}

fn xml_error(text: &str) -> Option<String> {
    let mut reader = quick_xml::Reader::from_str(text);
    reader.config_mut().check_end_names = true;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Eof) => return None,
            Ok(_) => {}
            Err(e) => return Some(format!("{e} (byte {})", reader.error_position())),
        }
    }
}

fn parse_literal(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn version_range(opt: &Opt) -> Option<String> {
    match (opt.since, opt.until) {
        (Some(s), Some(u)) => Some(format!(">= {s}, < {u}")),
        (Some(s), None) => Some(format!(">= {s}")),
        (None, Some(u)) => Some(format!("< {u}")),
        (None, None) => None,
    }
}

/// `None` when the option exists in the installed version (or it's unknown).
fn incompatible(opt: &Opt, package: Option<&str>, installed: Option<&Version>) -> Option<String> {
    let (package, installed) = (package?, installed?);
    if let Some(since) = opt.since.and_then(Version::parse)
        && installed < &since
    {
        return Some(format!(
            "needs {package} {since} or newer (this project has {installed})"
        ));
    }
    if let Some(until) = opt.until.and_then(Version::parse)
        && installed >= &until
    {
        return Some(format!(
            "removed in {package} {until} (this project has {installed})"
        ));
    }
    None
}

/// Reads the file and the form's options.
pub fn open(root: &Path, rel: &str) -> Result<ConfigFileView, String> {
    let (path, kind) = files::resolve(root, rel)?;
    let content = read_text(&path, rel)?;
    let versions = Versions::read(root);
    let format = files::format(kind, rel, &content);
    let file = file_info(kind, rel, format, &versions);
    let syntax_error = syntax(format, rel, &content);
    let installed = kind.package.and_then(|p| versions.get(p));

    let keys: Vec<&str> = kind.options.iter().map(|o| o.key).collect();
    let (states, notes, visual_blocked) = match (format, &syntax_error) {
        (_, Some(error)) => (
            None,
            Vec::new(),
            Some(format!("fix the syntax error first: {error}")),
        ),
        (ConfigFormat::Js, None) => match js::read(&content, rel, &keys) {
            Ok(read) => (
                Some(read.values.into_iter().map(|(_, s)| s).collect::<Vec<_>>()),
                read.notes,
                None,
            ),
            Err(e) => (None, Vec::new(), Some(e)),
        },
        (ConfigFormat::Json | ConfigFormat::Jsonc, None) => {
            let strict = format == ConfigFormat::Json;
            let states: Result<Vec<_>, _> = keys
                .iter()
                .map(|k| json::read(&content, strict, k))
                .collect();
            match states {
                Ok(s) => (Some(s), Vec::new(), None),
                Err(e) => (None, Vec::new(), Some(e)),
            }
        }
        _ if kind.options.is_empty() => (None, Vec::new(), None),
        _ => (
            None,
            Vec::new(),
            Some("this format is edited as text".into()),
        ),
    };

    let options = match &states {
        Some(states) => kind
            .options
            .iter()
            .zip(states)
            .map(|(opt, state)| {
                let blocked = incompatible(opt, kind.package, installed).or_else(|| match state {
                    OptionState::Code { .. } => {
                        Some("computed in code; edit the text instead".into())
                    }
                    _ => None,
                });
                ConfigOption {
                    key: opt.key.into(),
                    section: opt.section.into(),
                    label: opt.label.into(),
                    description: opt.description.into(),
                    kind: opt.kind,
                    choices: opt.choices.iter().map(|c| parse_literal(c)).collect(),
                    default: opt.default.map(parse_literal),
                    example: opt.example.map(str::to_string),
                    docs: opt.docs.map(str::to_string),
                    versions: version_range(opt)
                        .map(|r| format!("{} {r}", kind.package.unwrap_or(""))),
                    state: state.clone(),
                    editable: blocked.is_none(),
                    blocked,
                }
            })
            .collect(),
        None => Vec::new(),
    };

    Ok(ConfigFileView {
        file,
        hash: hash(content.as_bytes()),
        content,
        options,
        notes,
        visual_blocked,
        syntax_error,
    })
}

/// The text after `change`. Option changes go through the parser and are
/// refused when the option doesn't apply to the installed version.
pub fn apply(
    root: &Path,
    rel: &str,
    current: &str,
    change: &ConfigChange,
) -> Result<String, String> {
    let (_, kind) = files::resolve(root, rel)?;
    let format = files::format(kind, rel, current);
    match change {
        ConfigChange::Text { content } => Ok(content.clone()),
        ConfigChange::Set { key, value } => {
            let opt = kind.options.iter().find(|o| o.key == key).ok_or_else(|| {
                format!("`{key}` isn't an option LocDev edits in {rel}; edit the text instead")
            })?;
            let versions = Versions::read(root);
            if let Some(reason) = incompatible(
                opt,
                kind.package,
                kind.package.and_then(|p| versions.get(p)),
            ) {
                return Err(format!("`{key}` {reason}"));
            }
            check_value(opt, value)?;
            match format {
                ConfigFormat::Js => js::set(current, rel, key, value),
                ConfigFormat::Json => json::set(current, true, key, value),
                ConfigFormat::Jsonc => json::set(current, false, key, value),
                _ => Err(format!("{rel} is edited as text")),
            }
        }
    }
}

fn check_value(opt: &Opt, value: &Value) -> Result<(), String> {
    use locdev_core::config::OptionKind::*;
    let ok = match opt.kind {
        Bool => value.is_boolean(),
        Number => value.is_number(),
        String => value.is_string(),
        Choice => opt.choices.iter().any(|c| &parse_literal(c) == value),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("`{value}` isn't a valid value for `{}`", opt.key))
    }
}

/// The comment on a line (from `//`, `/*`, `#` or `<!--`), if any.
fn comment_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with('*') || t.starts_with('#') {
        return Some(t.trim_end());
    }
    ["//", "/*", "<!--"]
        .iter()
        .filter_map(|m| line.find(m))
        .min()
        .map(|i| line[i..].trim_end())
}

/// What saving `change` would do, against the file as it is now.
pub fn preview(root: &Path, rel: &str, change: &ConfigChange) -> Result<ConfigPreview, String> {
    let (path, kind) = files::resolve(root, rel)?;
    let current = read_text(&path, rel)?;
    let content = apply(root, rel, &current, change)?;
    let diff = similar::TextDiff::from_lines(&current, &content);
    let removed_comments = diff
        .iter_all_changes()
        .filter(|c| c.tag() == similar::ChangeTag::Delete)
        // A changed line that keeps its comment (`port: 5180, // why`) isn't a loss.
        .filter(|c| comment_of(c.value()).is_some_and(|comment| !content.contains(comment)))
        .map(|c| c.value().trim_end().to_string())
        .collect();
    let format = files::format(kind, rel, &content);
    Ok(ConfigPreview {
        diff: diff
            .unified_diff()
            .context_radius(3)
            .header(rel, rel)
            .to_string(),
        syntax_error: syntax(format, rel, &content),
        removed_comments,
        content,
    })
}

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_file_name(format!(
        ".{}.locdev-tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    let permissions = std::fs::metadata(path)?.permissions();
    std::fs::write(&tmp, content)?;
    let result =
        std::fs::set_permissions(&tmp, permissions).and_then(|_| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Saves `change` if the file still has `base_hash` (what the user reviewed).
/// The current content goes to `history` first. Returns the new hash.
pub fn save(
    root: &Path,
    rel: &str,
    change: &ConfigChange,
    base_hash: &str,
    history: &History,
) -> Result<String, String> {
    let reason = match change {
        ConfigChange::Set { key, value } => format!("set {key} = {value}"),
        ConfigChange::Text { .. } => "text edit".into(),
    };
    save_as(root, rel, change, base_hash, history, &reason)
}

fn save_as(
    root: &Path,
    rel: &str,
    change: &ConfigChange,
    base_hash: &str,
    history: &History,
    reason: &str,
) -> Result<String, String> {
    let (path, kind) = files::resolve(root, rel)?;
    let current = read_text(&path, rel)?;
    if hash(current.as_bytes()) != base_hash {
        return Err(format!(
            "{rel} changed since you opened it; review it again"
        ));
    }
    let content = apply(root, rel, &current, change)?;
    if content == current {
        return Ok(base_hash.to_string());
    }
    let format = files::format(kind, rel, &content);
    if let Some(error) = syntax(format, rel, &content) {
        return Err(format!("not saved: {error}"));
    }
    history
        .record(&current, reason)
        .map_err(|e| format!("couldn't keep the previous version: {e}"))?;
    write_atomic(&path, &content).map_err(|e| format!("{rel}: {e}"))?;
    Ok(hash(content.as_bytes()))
}

/// Puts a saved version back (the current one is kept in the history too).
pub fn revert(
    root: &Path,
    rel: &str,
    id: &str,
    base_hash: &str,
    history: &History,
) -> Result<String, String> {
    let content = history.load(id)?;
    save_as(
        root,
        rel,
        &ConfigChange::Text { content },
        base_hash,
        history,
        &format!("revert to {id}"),
    )
}

#[cfg(test)]
mod tests;
