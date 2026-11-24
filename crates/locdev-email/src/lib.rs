//! Email Studio: a project's email templates and components, LocDev's
//! visual blocks inside them (Blade and other code kept as written), static
//! and framework previews, and generators in the framework's own structure.

pub mod blocks;
pub mod discover;
pub mod framework;
pub mod generate;
pub mod preview;

use std::path::{Path, PathBuf};

use locdev_core::email::{EmailCatalog, EmailDoc, EmailFormat, Segment};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::blocks::Flavor;

pub fn hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn catalog(root: &Path, framework: &str, artisan: bool) -> EmailCatalog {
    let laravel = discover::is_laravel(root);
    let mut native = Vec::new();
    if laravel && artisan {
        native.push(
            "php artisan make:mail <Name>Mail --view=emails.<name> (used by `locdev new email`)"
                .into(),
        );
    }
    EmailCatalog {
        framework: framework.to_string(),
        templates: discover::templates(root),
        components: discover::components(root),
        conventions: if laravel {
            vec![
                "Templates: resources/views/emails/<name>.blade.php, with a Mailable in app/Mail".into(),
                "Components: resources/views/components/mail/<name>.blade.php, used as <x-mail.name>, props from @props([...])".into(),
            ]
        } else {
            vec![
                "Templates: emails/<name>.html".into(),
                "Components: emails/components/<name>.html with <!-- locdev:component {\"props\":[...]} --> and {{prop}} placeholders".into(),
            ]
        },
        native,
        framework_preview: laravel && framework::laravel_ready(root),
    }
}

/// The template at `rel`, if it's one the catalog lists, inside the project.
pub fn resolve(root: &Path, rel: &str) -> Result<(PathBuf, EmailFormat), String> {
    let templates = discover::templates(root);
    let found = templates
        .iter()
        .find(|t| t.path == rel)
        .ok_or_else(|| format!("{rel} isn't an email template LocDev found in this project"))?;
    let path = root.join(&found.path);
    let real_root = root.canonicalize().map_err(|e| e.to_string())?;
    let real = path.canonicalize().map_err(|e| format!("{rel}: {e}"))?;
    if !real.starts_with(&real_root) {
        return Err(format!("{rel} is outside the project"));
    }
    Ok((path, found.format))
}

fn read(path: &Path, rel: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{rel}: {e}"))?;
    String::from_utf8(bytes).map_err(|_| format!("{rel} isn't UTF-8 text"))
}

/// The file as segments. Formats without blocks are one code segment.
pub fn open(root: &Path, rel: &str, sample: Value) -> Result<EmailDoc, String> {
    let (path, format) = resolve(root, rel)?;
    let content = read(&path, rel)?;
    let mut notes = Vec::new();
    let (segments, theme) = match Flavor::of(format) {
        Some(flavor) => {
            let parsed = blocks::parse(&content, flavor, blocks::default_theme());
            if parsed.found.iter().any(|f| f.edited) {
                notes.push("Some blocks were changed outside LocDev: they're kept as written and marked; regenerate one from its properties to edit it visually.".into());
            }
            (parsed.segments, parsed.theme)
        }
        None => {
            notes.push(format!("{format:?} templates are edited as code here; LocDev's visual blocks work in Blade, Markdown mailables and HTML."));
            (
                vec![Segment::Code {
                    id: "c0".into(),
                    code: content.clone(),
                }],
                blocks::default_theme(),
            )
        }
    };
    Ok(EmailDoc {
        path: rel.to_string(),
        format,
        hash: hash(content.as_bytes()),
        variables: discover::variables(&content),
        content,
        segments,
        theme,
        sample,
        notes,
    })
}

/// The file text the editor's segments produce, against the file as it is.
pub fn build(
    root: &Path,
    rel: &str,
    segments: &[Segment],
    theme: &Value,
    preview: bool,
) -> Result<(String, EmailFormat), String> {
    let (path, format) = resolve(root, rel)?;
    let current = read(&path, rel)?;
    let Some(flavor) = Flavor::of(format) else {
        // Code-only formats: the single code segment is the file.
        let code = segments.iter().filter_map(|s| match s {
            Segment::Code { code, .. } => Some(code.as_str()),
            _ => None,
        });
        return Ok((code.collect(), format));
    };
    let parsed = blocks::parse(&current, flavor, blocks::default_theme());
    Ok((
        blocks::emit(segments, &parsed.found, flavor, theme, preview),
        format,
    ))
}

pub fn diff(rel: &str, before: &str, after: &str) -> String {
    similar::TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(rel, rel)
        .to_string()
}

/// Saves `content` if the file still has `base_hash`; returns the new hash.
/// The previous content goes to `history` first; the write is atomic.
pub fn save(
    root: &Path,
    rel: &str,
    content: &str,
    base_hash: &str,
    history: &locdev_config::history::History,
    reason: &str,
) -> Result<String, String> {
    let (path, _) = resolve(root, rel)?;
    let current = read(&path, rel)?;
    if hash(current.as_bytes()) != base_hash {
        return Err(format!(
            "{rel} changed since you opened it (another editor?); review it again"
        ));
    }
    if content == current {
        return Ok(base_hash.to_string());
    }
    history
        .record(&current, reason)
        .map_err(|e| format!("couldn't keep the previous version: {e}"))?;
    let tmp = path.with_file_name(format!(
        ".{}.locdev-tmp",
        path.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
    ));
    let permissions = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .permissions();
    std::fs::write(&tmp, content)
        .and_then(|_| std::fs::set_permissions(&tmp, permissions))
        .and_then(|_| std::fs::rename(&tmp, &path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{rel}: {e}")
        })?;
    Ok(hash(content.as_bytes()))
}

/// Sample data for a template: what was saved, else a guess from its variables.
pub fn default_sample(variables: &[String]) -> Value {
    let mut map = serde_json::Map::new();
    for name in variables {
        let value = match name.as_str() {
            "url" | "link" | "actionUrl" | "action_url" => {
                Value::String("https://example.test/action".into())
            }
            "name" | "username" => Value::String("Ana".into()),
            "email" => Value::String("ana@example.com".into()),
            "user" | "customer" | "notifiable" => {
                serde_json::json!({ "name": "Ana", "email": "ana@example.com" })
            }
            "token" | "code" => Value::String("6F1C-9A2B".into()),
            "organization" | "company" | "team" => Value::String("Acme".into()),
            _ => Value::String(format!("{name}…")),
        };
        map.insert(name.clone(), value);
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests;
