//! Finding a project's email templates, the mailables that use them and the
//! components it defines. Files are read, never executed.
//!
//! Conventions LocDev uses (and `locdev new email` writes):
//! - Laravel: templates in `resources/views/emails/`, components in
//!   `resources/views/components/mail/` (used as `<x-mail.name>`, props from
//!   Blade's own `@props([...])`).
//! - Other stacks: templates in `emails/`, components in `emails/components/`
//!   (`<!-- locdev:component {"props":["text","url"]} -->` at the top).

use std::path::{Path, PathBuf};

use locdev_core::email::{ComponentProp, EmailComponent, EmailFormat, EmailTemplate, MailableInfo};

const MAX_FILE: u64 = 512 * 1024;
const MAX_FILES: usize = 2000;
const SKIP: &[&str] = &[
    "node_modules",
    "vendor",
    ".git",
    "storage",
    "dist",
    "build",
    ".next",
    "public",
];

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 8 || out.len() > MAX_FILES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if meta.is_dir() {
            if !SKIP.contains(&name.as_str()) && !name.starts_with('.') {
                walk(&path, depth + 1, out);
            }
        } else if meta.is_file() && meta.len() <= MAX_FILE {
            out.push(path);
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn is_laravel(root: &Path) -> bool {
    root.join("artisan").is_file() && root.join("resources/views").is_dir()
}

/// `resources/views/emails/welcome.blade.php` → `emails.welcome`.
pub fn view_name(rel: &str) -> Option<String> {
    let inner = rel
        .strip_prefix("resources/views/")?
        .strip_suffix(".blade.php")?;
    Some(inner.replace('/', "."))
}

pub fn view_path(name: &str) -> String {
    format!("resources/views/{}.blade.php", name.replace('.', "/"))
}

fn blade_format(text: &str, used_as_text: bool) -> EmailFormat {
    if text.contains("x-mail::message")
        || text.contains("@component('mail::message')")
        || text.contains("@component(\"mail::message\")")
    {
        EmailFormat::Markdown
    } else if used_as_text && !text.contains('<') {
        EmailFormat::Text
    } else {
        EmailFormat::Blade
    }
}

/// The string literal right after `key` (`view: 'emails.x'`, `->view("x")`).
fn literal_after<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = text[at..].find(key) {
        let start = at + i + key.len();
        let rest = text[start..].trim_start();
        if let Some(q) = rest.chars().next().filter(|c| *c == '\'' || *c == '"')
            && let Some(end) = rest[1..].find(q)
        {
            out.push(&rest[1..1 + end]);
        }
        at = start;
    }
    out
}

/// What each `app/Mail/*.php` renders and with which variables.
pub fn mailables(root: &Path) -> Vec<(MailableInfo, Vec<(String, bool)>)> {
    let mut files = Vec::new();
    walk(&root.join("app/Mail"), 0, &mut files);
    let mut out = Vec::new();
    for file in files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "php"))
    {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let Some(class) = text
            .split("class ")
            .nth(1)
            .and_then(|c| c.split_whitespace().next())
        else {
            continue;
        };
        let namespace = text
            .lines()
            .find_map(|l| {
                l.trim()
                    .strip_prefix("namespace ")
                    .map(|n| n.trim_end_matches(';').trim().to_string())
            })
            .unwrap_or_else(|| "App\\Mail".into());
        let mut views: Vec<(String, bool)> = Vec::new();
        for key in ["view:", "html:", "markdown:", "->view(", "->markdown("] {
            views.extend(
                literal_after(&text, key)
                    .into_iter()
                    .map(|v| (v.to_string(), false)),
            );
        }
        for key in ["text:", "->text("] {
            views.extend(
                literal_after(&text, key)
                    .into_iter()
                    .map(|v| (v.to_string(), true)),
            );
        }
        // Public properties are available in the view; `with:` keys too.
        let mut variables: Vec<String> = Vec::new();
        for part in text.split("public ").skip(1) {
            let decl = part.split([',', ';', ')', '=']).next().unwrap_or("");
            if let Some(var) = decl.split_whitespace().find(|w| w.starts_with('$')) {
                let name = var.trim_start_matches('$').to_string();
                if !name.is_empty() && !variables.contains(&name) {
                    variables.push(name);
                }
            }
        }
        if let Some(with) = text.find("with:").or_else(|| text.find("->with(")) {
            let body = &text[with..text[with..].find(']').map_or(text.len(), |e| with + e)];
            for line in body.split(',') {
                if let Some(arrow) = line.find("=>") {
                    let key = line[..arrow]
                        .trim()
                        .trim_start_matches('[')
                        .trim()
                        .trim_matches(['\'', '"']);
                    if !key.is_empty()
                        && key.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && !variables.contains(&key.to_string())
                    {
                        variables.push(key.to_string());
                    }
                }
            }
        }
        let subject = literal_after(&text, "subject:")
            .first()
            .map(|s| s.to_string());
        out.push((
            MailableInfo {
                class: format!("{namespace}\\{class}"),
                path: rel(root, file),
                variables,
                subject,
            },
            views,
        ));
    }
    out
}

pub fn templates(root: &Path) -> Vec<EmailTemplate> {
    if is_laravel(root) {
        laravel_templates(root)
    } else {
        other_templates(root)
    }
}

fn laravel_templates(root: &Path) -> Vec<EmailTemplate> {
    let mailables = mailables(root);
    let mut files = Vec::new();
    walk(&root.join("resources/views"), 0, &mut files);
    let mut out = Vec::new();
    for file in &files {
        let r = rel(root, file);
        let Some(name) = view_name(&r) else { continue };
        if r.starts_with("resources/views/components/") || r.starts_with("resources/views/vendor/")
        {
            continue;
        }
        let in_mail_dir = ["mail", "mails", "email", "emails"]
            .iter()
            .any(|d| name.split('.').any(|part| part == *d));
        let users: Vec<&(MailableInfo, Vec<(String, bool)>)> = mailables
            .iter()
            .filter(|(_, v)| v.iter().any(|(n, _)| *n == name))
            .collect();
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let marked = text.contains("locdev:email") || text.contains("x-mail::message");
        if !in_mail_dir && users.is_empty() && !marked {
            continue;
        }
        let as_text = users
            .iter()
            .any(|(_, v)| v.iter().any(|(n, t)| *n == name && *t));
        out.push(EmailTemplate {
            path: r,
            name,
            format: blade_format(&text, as_text),
            mailables: users.iter().map(|(m, _)| m.clone()).collect(),
            has_blocks: text.contains("locdev:block"),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn other_templates(root: &Path) -> Vec<EmailTemplate> {
    let mut out = Vec::new();
    let dirs = [
        "emails",
        "email",
        "templates/emails",
        "templates/email",
        "mail",
        "src/emails",
        "app/views",
    ];
    for dir in dirs {
        let mut files = Vec::new();
        walk(&root.join(dir), 0, &mut files);
        for file in files {
            let r = rel(root, &file);
            if r.contains("/components/") {
                continue;
            }
            let format = match file.extension().and_then(|e| e.to_str()) {
                Some("html") | Some("htm") if r.starts_with("templates/") => EmailFormat::Django,
                Some("html") | Some("htm") => EmailFormat::Html,
                Some("mjml") => EmailFormat::Mjml,
                Some("tsx") | Some("jsx") => EmailFormat::React,
                Some("erb") if r.contains("_mailer/") => EmailFormat::Erb,
                _ => continue,
            };
            if dir == "app/views" && format != EmailFormat::Erb {
                continue;
            }
            let has_blocks =
                std::fs::read_to_string(&file).is_ok_and(|t| t.contains("locdev:block"));
            if !out.iter().any(|t: &EmailTemplate| t.path == r) {
                out.push(EmailTemplate {
                    name: r.clone(),
                    path: r,
                    format,
                    mailables: Vec::new(),
                    has_blocks,
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Keys of a Blade `@props([...])`: `'url'` or `'color' => 'primary'`.
pub fn blade_props(text: &str) -> Vec<ComponentProp> {
    let Some(start) = text.find("@props(") else {
        return Vec::new();
    };
    let body = &text[start + 7..];
    let Some(end) = body.find("])") else {
        return Vec::new();
    };
    let body = body[..end].trim().trim_start_matches('[');
    let mut out = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in body.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    out.push(current);
    out.into_iter()
        .filter_map(|entry| {
            let entry = entry.trim();
            let (key, default) = match entry.split_once("=>") {
                Some((k, d)) => (k.trim(), Some(d.trim().to_string())),
                None => (entry, None),
            };
            let key = key.trim_matches(['\'', '"']);
            (!key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
            .then(|| ComponentProp {
                name: key.to_string(),
                default,
            })
        })
        .collect()
}

fn component_description(text: &str) -> Option<String> {
    let start = text.find("locdev:component ")? + "locdev:component ".len();
    let end = text[start..]
        .find("--}}")
        .or_else(|| text[start..].find("-->"))?
        + start;
    let meta: serde_json::Value = serde_json::from_str(text[start..end].trim()).ok()?;
    meta["description"].as_str().map(str::to_string)
}

pub fn components(root: &Path) -> Vec<EmailComponent> {
    let mut out = Vec::new();
    if is_laravel(root) {
        let base = root.join("resources/views/components/mail");
        let mut files = Vec::new();
        walk(&base, 0, &mut files);
        for file in files {
            let r = rel(root, &file);
            let Some(inner) = r
                .strip_prefix("resources/views/components/")
                .and_then(|p| p.strip_suffix(".blade.php"))
            else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let dotted = inner.replace('/', ".");
            out.push(EmailComponent {
                name: dotted.rsplit('.').next().unwrap_or(&dotted).to_string(),
                usage: format!("x-{dotted}"),
                path: r,
                props: blade_props(&text),
                slot: text.contains("$slot"),
                description: component_description(&text),
                builtin: false,
            });
        }
        // Laravel's own Markdown mail components.
        for (name, props) in [
            ("button", vec!["url", "color"]),
            ("panel", vec![]),
            ("subcopy", vec![]),
            ("table", vec![]),
        ] {
            out.push(EmailComponent {
                name: name.into(),
                usage: format!("x-mail::{name}"),
                path: String::new(),
                props: props
                    .into_iter()
                    .map(|p| ComponentProp {
                        name: p.into(),
                        default: None,
                    })
                    .collect(),
                slot: true,
                description: Some("Laravel Markdown mail component".into()),
                builtin: true,
            });
        }
    } else {
        let mut files = Vec::new();
        walk(&root.join("emails/components"), 0, &mut files);
        for file in files
            .into_iter()
            .filter(|f| f.extension().is_some_and(|e| e == "html"))
        {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let props = text
                .find("locdev:component ")
                .and_then(|s| {
                    let start = s + "locdev:component ".len();
                    let end = text[start..].find("-->")? + start;
                    serde_json::from_str::<serde_json::Value>(text[start..end].trim()).ok()
                })
                .and_then(|m| m["props"].as_array().cloned())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|p| {
                    p.as_str().map(|n| ComponentProp {
                        name: n.to_string(),
                        default: None,
                    })
                })
                .collect();
            let name = file
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            out.push(EmailComponent {
                usage: format!("emails/components/{name}.html"),
                name,
                path: rel(root, &file),
                props,
                slot: text.contains("{{slot}}"),
                description: component_description(&text),
                builtin: false,
            });
        }
    }
    out
}

/// Variables a template uses: `$user` from `{{ $user->name }}`, `@if($x)`…
pub fn variables(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            let name = &text[start..j];
            if !name.is_empty()
                && !name.chars().next().is_some_and(|c| c.is_ascii_digit())
                && ![
                    "this",
                    "slot",
                    "attributes",
                    "loop",
                    "errors",
                    "message",
                    "component",
                    "__env",
                    "__data",
                ]
                .contains(&name)
                && !out.iter().any(|n| n == name)
            {
                out.push(name.to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}
