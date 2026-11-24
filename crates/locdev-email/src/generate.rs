//! `locdev new email <name>` and `locdev new email component <name>`: files
//! in the framework's own structure, created only where nothing exists.
//! For Laravel, the Mailable class comes from `php artisan make:mail` (the
//! framework's generator); LocDev writes the Blade view with its blocks.

use std::path::Path;

use serde_json::json;

use crate::blocks::{Flavor, default_theme, emit};
use crate::discover::{is_laravel, view_path};
use locdev_core::email::Segment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Template,
    Component,
}

#[derive(Debug)]
pub struct NewPlan {
    /// Files to create (relative path, content), never over an existing one.
    pub files: Vec<(String, String)>,
    /// `php artisan make:mail …` to run first, when the framework has it.
    pub artisan: Option<Vec<String>>,
    /// A file the native command creates and LocDev then fills (its view).
    pub fill_after_artisan: Option<(String, String)>,
    pub notes: Vec<String>,
}

/// `order-shipped` / `order_shipped` → `OrderShipped`.
pub fn studly(name: &str) -> String {
    name.split(['-', '_', ' ', '.'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// `order-shipped` → `Order shipped`.
pub fn title(name: &str) -> String {
    let words = kebab(name).replace('-', " ");
    let mut c = words.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// `OrderShipped` → `order-shipped`.
pub fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 && !out.ends_with('-') {
            out.push('-');
        }
        out.push(if c == '_' || c == ' ' {
            '-'
        } else {
            c.to_ascii_lowercase()
        });
    }
    out
}

pub fn valid_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 60
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{name}` isn't a name LocDev can use: letters, digits, `-` and `_`, starting with a letter"
        ))
    }
}

fn block(id: usize, kind: &str, props: serde_json::Value) -> Segment {
    Segment::Block {
        id: format!("n{id}"),
        block: kind.into(),
        props,
        edited: false,
    }
}

fn code(id: usize, code: &str) -> Segment {
    Segment::Code {
        id: format!("c{id}"),
        code: code.into(),
    }
}

/// A starter template with a layout (code) around editable blocks.
pub fn starter(flavor: Flavor, title: &str) -> String {
    let theme = default_theme();
    let marker = serde_json::to_string(&theme).unwrap_or_default();
    let segments = match flavor {
        Flavor::Markdown => vec![
            code(
                0,
                &format!("{{{{-- locdev:email {marker} --}}}}\n<x-mail::message>\n"),
            ),
            block(1, "heading", json!({ "text": title, "level": 1 })),
            code(2, "\n"),
            block(
                3,
                "text",
                json!({ "text": "Hello {{ $name ?? 'there' }},\n\nWrite your message here." }),
            ),
            code(4, "\n"),
            block(
                5,
                "button",
                json!({ "text": "Open", "url": "{{ $url ?? url('/') }}" }),
            ),
            code(
                6,
                "\nThanks,<br>\n{{ config('app.name') }}\n</x-mail::message>\n",
            ),
        ],
        Flavor::Blade | Flavor::Html => {
            let (app, year) = if flavor == Flavor::Blade {
                ("{{ config('app.name') }}", "{{ date('Y') }}")
            } else {
                ("Your app", "")
            };
            let head = format!(
                "{marker_line}\n<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n</head>\n<body style=\"margin:0;padding:0;background:{bg};\">\n<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"background:{bg};\"><tr><td align=\"center\" style=\"padding:24px 12px;\">\n<table role=\"presentation\" width=\"{width}\" cellpadding=\"0\" cellspacing=\"0\" style=\"width:100%;max-width:{width}px;background:{card};border-radius:8px;font-family:{font};color:{text};\">\n",
                marker_line = match flavor {
                    Flavor::Html => format!("<!-- locdev:email {marker} -->"),
                    _ => format!("{{{{-- locdev:email {marker} --}}}}"),
                },
                bg = theme["background"].as_str().unwrap_or(""),
                card = theme["card"].as_str().unwrap_or(""),
                width = theme["width"],
                font = theme["font"].as_str().unwrap_or(""),
                text = theme["text"].as_str().unwrap_or(""),
            );
            let greeting = if flavor == Flavor::Blade {
                "Hello {{ $name ?? 'there' }},"
            } else {
                "Hello,"
            };
            let url = if flavor == Flavor::Blade {
                "{{ $url ?? url('/') }}"
            } else {
                "https://example.com"
            };
            vec![
                code(0, &head),
                block(1, "header", json!({ "title": app, "align": "center" })),
                block(2, "heading", json!({ "text": title, "level": 1 })),
                block(
                    3,
                    "text",
                    json!({ "text": format!("{greeting}\n\nWrite your message here.") }),
                ),
                block(4, "button", json!({ "text": "Open", "url": url })),
                block(5, "divider", json!({})),
                block(
                    6,
                    "footer",
                    json!({ "text": format!("© {year} {app}").trim().to_string() }),
                ),
                code(7, "</table>\n</td></tr></table>\n</body>\n</html>\n"),
            ]
        }
    };
    emit(&segments, &[], flavor, &theme, false)
}

fn component_starter(name: &str, laravel: bool) -> String {
    if laravel {
        let (props, body) = match name {
            "button" => (
                "['url', 'color' => '#ea580c']",
                "<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" align=\"center\"><tr><td style=\"border-radius:6px;background:{{ $color }};\">\n<a href=\"{{ $url }}\" style=\"display:inline-block;padding:12px 22px;font-size:15px;font-weight:bold;color:#ffffff;text-decoration:none;\">{{ $slot }}</a>\n</td></tr></table>",
            ),
            "header" => (
                "['title' => config('app.name'), 'logo' => null]",
                "<div style=\"text-align:center;padding:8px 0;\">\n@if ($logo)\n<img src=\"{{ $logo }}\" alt=\"\" height=\"32\" style=\"height:32px;border:0;\">\n@endif\n<div style=\"font-size:18px;font-weight:bold;\">{{ $title }}</div>\n</div>",
            ),
            "footer" => (
                "['company' => config('app.name')]",
                "<p style=\"margin:0;font-size:12px;line-height:1.6;color:#71717a;text-align:center;\">\n{{ $slot }}<br>\n&copy; {{ date('Y') }} {{ $company }}\n</p>",
            ),
            _ => (
                "['title' => null]",
                "<div style=\"padding:8px 0;\">\n@if ($title)\n<strong>{{ $title }}</strong><br>\n@endif\n{{ $slot }}\n</div>",
            ),
        };
        format!(
            "{{{{-- locdev:component {{\"description\":\"{}\"}} --}}}}\n@props({props})\n{body}\n",
            format!("Email component `{name}`: use it as <x-mail.{name}>.").replace('"', "'")
        )
    } else {
        let (props, body) = match name {
            "button" => (
                r#"["text","url"]"#,
                "<a href=\"{{url}}\" style=\"display:inline-block;padding:12px 22px;background:#ea580c;color:#ffffff;border-radius:6px;text-decoration:none;font-weight:bold;\">{{text}}</a>",
            ),
            "footer" => (
                r#"["text"]"#,
                "<p style=\"margin:0;font-size:12px;color:#71717a;text-align:center;\">{{text}}</p>",
            ),
            "header" => (
                r#"["title"]"#,
                "<div style=\"text-align:center;font-size:18px;font-weight:bold;\">{{title}}</div>",
            ),
            _ => (
                r#"["text"]"#,
                "<div style=\"padding:8px 0;\">{{text}}</div>",
            ),
        };
        format!(
            "<!-- locdev:component {{\"props\":{props},\"description\":\"Email component `{name}`.\"}} -->\n{body}\n"
        )
    }
}

/// What `locdev new email …` would create. `markdown` picks a Markdown mailable (Laravel).
pub fn plan(
    root: &Path,
    kind: Kind,
    name: &str,
    markdown: bool,
    artisan_ok: bool,
) -> Result<NewPlan, String> {
    valid_name(name)?;
    let slug = kebab(name);
    let laravel = is_laravel(root);
    let mut notes = Vec::new();
    let plan = match (kind, laravel) {
        (Kind::Template, true) => {
            let view = format!("emails.{slug}");
            let rel = view_path(&view);
            let class = {
                let s = studly(name);
                if s.ends_with("Mail") {
                    s
                } else {
                    format!("{s}Mail")
                }
            };
            let class_rel = format!("app/Mail/{class}.php");
            if root.join(&class_rel).exists() {
                return Err(format!(
                    "{class_rel} already exists; LocDev doesn't overwrite it"
                ));
            }
            let flavor = if markdown {
                Flavor::Markdown
            } else {
                Flavor::Blade
            };
            let content = starter(flavor, &title(name));
            if artisan_ok {
                let option = if markdown {
                    format!("--markdown={view}")
                } else {
                    format!("--view={view}")
                };
                notes.push(format!("{class} (app/Mail) comes from Laravel's own `php artisan make:mail`; LocDev fills the view with editable blocks."));
                NewPlan {
                    files: Vec::new(),
                    artisan: Some(vec!["make:mail".into(), class, option]),
                    fill_after_artisan: Some((rel, content)),
                    notes,
                }
            } else {
                notes.push(format!(
                    "Only the view was created: run `php artisan make:mail {class} --{}={view}` for the Mailable class (PHP or vendor/ wasn't available here).",
                    if markdown { "markdown" } else { "view" }
                ));
                NewPlan {
                    files: vec![(rel, content)],
                    artisan: None,
                    fill_after_artisan: None,
                    notes,
                }
            }
        }
        (Kind::Component, true) => {
            let rel = format!("resources/views/components/mail/{slug}.blade.php");
            notes.push(format!("Use it in a template as <x-mail.{slug}>; its props come from @props, so the Studio lists them."));
            NewPlan {
                files: vec![(rel, component_starter(&slug, true))],
                artisan: None,
                fill_after_artisan: None,
                notes,
            }
        }
        (Kind::Template, false) => {
            let rel = format!("emails/{slug}.html");
            notes.push("Plain HTML: send it with your framework's mailer (or render it with your template engine).".into());
            NewPlan {
                files: vec![(rel, starter(Flavor::Html, &title(name)))],
                artisan: None,
                fill_after_artisan: None,
                notes,
            }
        }
        (Kind::Component, false) => {
            let rel = format!("emails/components/{slug}.html");
            notes.push(
                "In the Studio, its {{placeholders}} become the component's properties.".into(),
            );
            NewPlan {
                files: vec![(rel, component_starter(&slug, false))],
                artisan: None,
                fill_after_artisan: None,
                notes,
            }
        }
    };
    for (rel, _) in plan.files.iter().chain(plan.fill_after_artisan.iter()) {
        if root.join(rel).exists() {
            return Err(format!("{rel} already exists; LocDev doesn't overwrite it"));
        }
    }
    Ok(plan)
}

/// Writes a file only if nothing is there.
pub fn create(root: &Path, rel: &str, content: &str) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                format!("{rel} already exists; LocDev doesn't overwrite it")
            }
            _ => format!("{rel}: {e}"),
        })?;
    std::io::Write::write_all(&mut file, content.as_bytes()).map_err(|e| format!("{rel}: {e}"))
}
