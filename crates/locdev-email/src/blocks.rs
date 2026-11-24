//! A template as segments: LocDev blocks between marker comments, and code
//! kept exactly as written. Markers are comments of the template language
//! (`{{-- --}}` in Blade, `<!-- -->` in HTML), so they never reach the sent
//! message. Parsing and emitting without changes gives back the same bytes.
//!
//! Each block records a short hash of the markup it generated. When the
//! markup no longer matches (edited in an IDE), the block is `edited` and
//! kept as written until the user regenerates it from its properties.

use locdev_core::email::{EmailFormat, Segment};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Blade,
    Markdown,
    Html,
}

impl Flavor {
    pub fn of(format: EmailFormat) -> Option<Self> {
        match format {
            EmailFormat::Blade => Some(Self::Blade),
            EmailFormat::Markdown => Some(Self::Markdown),
            EmailFormat::Html => Some(Self::Html),
            _ => None,
        }
    }

    fn comment(self, body: &str) -> String {
        match self {
            Self::Blade | Self::Markdown => format!("{{{{-- {body} --}}}}"),
            Self::Html => format!("<!-- {body} -->"),
        }
    }

    fn open_prefix(self) -> &'static str {
        match self {
            Self::Blade | Self::Markdown => "{{-- locdev:block ",
            Self::Html => "<!-- locdev:block ",
        }
    }

    fn comment_end(self) -> &'static str {
        match self {
            Self::Blade | Self::Markdown => "--}}",
            Self::Html => "-->",
        }
    }

    fn close(self) -> String {
        self.comment("/locdev:block")
    }

    fn theme_prefix(self) -> &'static str {
        match self {
            Self::Blade | Self::Markdown => "{{-- locdev:email ",
            Self::Html => "<!-- locdev:email ",
        }
    }
}

pub fn default_theme() -> Value {
    json!({
        "width": 600,
        "background": "#f4f4f5",
        "card": "#ffffff",
        "text": "#18181b",
        "muted": "#71717a",
        "accent": "#ea580c",
        "font": "Arial, Helvetica, sans-serif",
    })
}

/// JSON that can't end the comment it lives in (`--` never appears raw).
fn marker_json(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_default()
        .replace("--", "-\\u002d")
}

pub fn short_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(4)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// One block as found in the file: its markers and markup, verbatim.
#[derive(Debug, Clone)]
pub struct Found {
    pub id: String,
    pub block: String,
    pub props: Value,
    pub raw: String,
    pub edited: bool,
}

pub struct Parsed {
    pub segments: Vec<Segment>,
    pub found: Vec<Found>,
    pub theme: Value,
}

pub fn parse(content: &str, flavor: Flavor, theme_default: Value) -> Parsed {
    let theme = read_theme(content, flavor).unwrap_or(theme_default);
    let mut segments = Vec::new();
    let mut found = Vec::new();
    let open = flavor.open_prefix();
    let close = flavor.close();
    let mut at = 0;
    let mut code_start = 0;
    while let Some(rel) = content[at..].find(open) {
        let start = at + rel;
        let Some(end_rel) = content[start..].find(flavor.comment_end()) else {
            break;
        };
        let header_end = start + end_rel + flavor.comment_end().len();
        let json_text = content[start + open.len()..start + end_rel].trim();
        let Some(close_rel) = content[header_end..].find(&close) else {
            break;
        };
        let close_end = header_end + close_rel + close.len();
        let meta: Option<Value> = serde_json::from_str(json_text).ok();
        let Some(meta) = meta.filter(|m| m["type"].is_string()) else {
            // Not ours after all; it stays code.
            at = header_end;
            continue;
        };
        if start > code_start {
            segments.push(Segment::Code {
                id: format!("c{}", segments.len()),
                code: content[code_start..start].to_string(),
            });
        }
        let inner = content[header_end..header_end + close_rel]
            .trim_matches(['\n', '\r'])
            .to_string();
        let id = format!("b{}", found.len());
        let block = meta["type"].as_str().unwrap_or("").to_string();
        let props = meta["props"].clone();
        let edited = meta["h"].as_str() != Some(&short_hash(&inner));
        segments.push(Segment::Block {
            id: id.clone(),
            block: block.clone(),
            props: props.clone(),
            edited,
        });
        found.push(Found {
            id,
            block,
            props,
            raw: content[start..close_end].to_string(),
            edited,
        });
        at = close_end;
        code_start = close_end;
    }
    if code_start < content.len() {
        segments.push(Segment::Code {
            id: format!("c{}", segments.len()),
            code: content[code_start..].to_string(),
        });
    }
    Parsed {
        segments,
        found,
        theme,
    }
}

fn read_theme(content: &str, flavor: Flavor) -> Option<Value> {
    let start = content.find(flavor.theme_prefix())? + flavor.theme_prefix().len();
    let end = content[start..].find(flavor.comment_end())? + start;
    serde_json::from_str(content[start..end].trim()).ok()
}

/// The file text for `segments`. Blocks whose properties didn't change keep
/// their exact markup (even when edited outside); others are regenerated.
/// Unchanged input gives back the same bytes: line breaks are only added
/// around blocks that are new or regenerated.
pub fn emit(
    segments: &[Segment],
    found: &[Found],
    flavor: Flavor,
    theme: &Value,
    preview: bool,
) -> String {
    let mut out = String::new();
    for (i, segment) in segments.iter().enumerate() {
        match segment {
            Segment::Code { code, .. } => out.push_str(code),
            Segment::Block {
                id,
                block,
                props,
                edited,
            } => {
                let original = found.iter().find(|f| &f.id == id);
                let unchanged = original.filter(|f| &f.block == block && &f.props == props);
                // Markup edited outside stays as written unless the client
                // clears `edited` (the user asked to regenerate it). Unedited
                // blocks keep their bytes, except in previews, where they're
                // regenerated to carry the selection tag.
                let keep = unchanged.is_some_and(|f| if f.edited { *edited } else { !preview });
                match unchanged.filter(|_| keep) {
                    Some(f) => out.push_str(&f.raw),
                    _ => {
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push('\n');
                        }
                        let inner =
                            render(block, props, flavor, theme, preview.then_some(id.as_str()));
                        let meta =
                            json!({ "type": block, "props": props, "h": short_hash(&inner) });
                        out.push_str(
                            &flavor.comment(&format!("locdev:block {}", marker_json(&meta))),
                        );
                        out.push('\n');
                        out.push_str(&inner);
                        out.push('\n');
                        out.push_str(&flavor.close());
                        let next_starts_line = matches!(segments.get(i + 1), Some(Segment::Code { code, .. }) if code.starts_with('\n'));
                        if !next_starts_line {
                            out.push('\n');
                        }
                    }
                }
            }
        }
    }
    set_theme(&out, flavor, theme)
}

/// Replaces the theme marker's JSON (or leaves the text alone without one).
fn set_theme(text: &str, flavor: Flavor, theme: &Value) -> String {
    let Some(start) = text
        .find(flavor.theme_prefix())
        .map(|s| s + flavor.theme_prefix().len())
    else {
        return text.to_string();
    };
    let Some(end) = text[start..].find(flavor.comment_end()).map(|e| e + start) else {
        return text.to_string();
    };
    let current: Option<Value> = serde_json::from_str(text[start..end].trim()).ok();
    if current.as_ref() == Some(theme) {
        return text.to_string();
    }
    format!("{}{} {}", &text[..start], marker_json(theme), &text[end..])
}

fn s<'a>(props: &'a Value, key: &str, default: &'a str) -> &'a str {
    props[key]
        .as_str()
        .filter(|v| !v.is_empty())
        .unwrap_or(default)
}

fn n(props: &Value, key: &str, default: i64) -> i64 {
    props[key]
        .as_i64()
        .or_else(|| props[key].as_str().and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}

/// HTML-escapes text but leaves Blade echoes (`{{ … }}`, `{!! … !!}`) as written.
pub fn text_html(text: &str, flavor: Flavor) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let next = if flavor == Flavor::Html {
            None
        } else {
            [rest.find("{{"), rest.find("{!!")]
                .into_iter()
                .flatten()
                .min()
        };
        let (plain, tail) = match next {
            Some(i) => rest.split_at(i),
            None => (rest, ""),
        };
        out.push_str(&escape(plain).replace('\n', "<br>"));
        if tail.is_empty() {
            break;
        }
        let close = if tail.starts_with("{!!") { "!!}" } else { "}}" };
        match tail.find(close) {
            Some(end) => {
                out.push_str(&tail[..end + close.len()]);
                rest = &tail[end + close.len()..];
            }
            None => {
                out.push_str(&escape(tail));
                break;
            }
        }
    }
    out
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// An attribute value: Blade echoes kept, the rest escaped.
fn attr(value: &str, flavor: Flavor) -> String {
    text_html(value, flavor).replace("<br>", " ")
}

fn row(id: Option<&str>, padding: &str, body: &str) -> String {
    let data = id
        .map(|i| format!(" data-locdev-block=\"{i}\""))
        .unwrap_or_default();
    format!("<tr{data}><td style=\"padding:{padding};\">{body}</td></tr>")
}

/// Markup for one block. Email-safe: tables, inline styles, no scripts.
pub fn render(
    block: &str,
    props: &Value,
    flavor: Flavor,
    theme: &Value,
    preview_id: Option<&str>,
) -> String {
    if flavor == Flavor::Markdown {
        return render_markdown(block, props);
    }
    let text_color = theme["text"].as_str().unwrap_or("#18181b");
    let muted = theme["muted"].as_str().unwrap_or("#71717a");
    let accent = theme["accent"].as_str().unwrap_or("#ea580c");
    let align = s(props, "align", "left");
    match block {
        "heading" => {
            let level = n(props, "level", 1).clamp(1, 3);
            let size = [0, 24, 20, 17][level as usize];
            row(
                preview_id,
                "24px 32px 8px",
                &format!(
                    "<h{level} style=\"margin:0;font-size:{size}px;line-height:1.3;color:{};text-align:{align};\">{}</h{level}>",
                    s(props, "color", text_color),
                    text_html(s(props, "text", "Heading"), flavor)
                ),
            )
        }
        "text" => row(
            preview_id,
            "8px 32px",
            &format!(
                "<p style=\"margin:0;font-size:{}px;line-height:1.6;color:{};text-align:{align};\">{}</p>",
                n(props, "size", 15),
                s(props, "color", text_color),
                text_html(s(props, "text", ""), flavor)
            ),
        ),
        "button" => {
            let bg = s(props, "background", accent);
            let fg = s(props, "color", "#ffffff");
            let radius = n(props, "radius", 6);
            row(
                preview_id,
                "16px 32px",
                &format!(
                    "<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" align=\"{align}\"><tr><td style=\"border-radius:{radius}px;background:{bg};\"><a href=\"{}\" style=\"display:inline-block;padding:12px 22px;font-size:15px;font-weight:bold;color:{fg};text-decoration:none;border-radius:{radius}px;\">{}</a></td></tr></table>",
                    attr(s(props, "url", "#"), flavor),
                    text_html(s(props, "text", "Button"), flavor)
                ),
            )
        }
        "image" => {
            let width = n(props, "width", 536);
            let img = format!(
                "<img src=\"{}\" alt=\"{}\" width=\"{width}\" style=\"display:block;max-width:100%;height:auto;border:0;\">",
                attr(s(props, "src", ""), flavor),
                attr(s(props, "alt", ""), flavor)
            );
            let body = match props["url"].as_str().filter(|u| !u.is_empty()) {
                Some(url) => format!("<a href=\"{}\">{img}</a>", attr(url, flavor)),
                None => img,
            };
            row(
                preview_id,
                "8px 32px",
                &format!("<div style=\"text-align:{align};\">{body}</div>"),
            )
        }
        "divider" => row(
            preview_id,
            "16px 32px",
            &format!(
                "<hr style=\"border:0;border-top:1px solid {};margin:0;\">",
                s(props, "color", "#e4e4e7")
            ),
        ),
        "spacer" => {
            let height = n(props, "height", 24).clamp(4, 200);
            let data = preview_id
                .map(|i| format!(" data-locdev-block=\"{i}\""))
                .unwrap_or_default();
            format!(
                "<tr{data}><td style=\"height:{height}px;line-height:{height}px;font-size:0;\">&nbsp;</td></tr>"
            )
        }
        "columns" => {
            let cells: Vec<String> = props["cells"]
                .as_array()
                .map(|c| {
                    c.iter()
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_else(|| vec![String::new(), String::new()]);
            let width = 100 / cells.len().max(1);
            let tds: String = cells
                .iter()
                .map(|c| {
                    format!(
                        "<td width=\"{width}%\" valign=\"top\" style=\"padding:0 8px;font-size:14px;line-height:1.6;color:{text_color};\">{}</td>",
                        text_html(c, flavor)
                    )
                })
                .collect();
            row(
                preview_id,
                "8px 24px",
                &format!(
                    "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\"><tr>{tds}</tr></table>"
                ),
            )
        }
        "header" => {
            let logo = props["logo"].as_str().filter(|l| !l.is_empty()).map(|l| {
                format!("<img src=\"{}\" alt=\"\" height=\"32\" style=\"display:inline-block;border:0;height:32px;\">", attr(l, flavor))
            });
            let title = props["title"]
                .as_str()
                .filter(|t| !t.is_empty())
                .map(|t| format!("<div style=\"font-size:18px;font-weight:bold;color:{text_color};margin-top:6px;\">{}</div>", text_html(t, flavor)));
            row(
                preview_id,
                "24px 32px 8px",
                &format!(
                    "<div style=\"text-align:{}\">{}{}</div>",
                    s(props, "align", "center"),
                    logo.unwrap_or_default(),
                    title.unwrap_or_default()
                ),
            )
        }
        "footer" => row(
            preview_id,
            "24px 32px",
            &format!(
                "<p style=\"margin:0;font-size:12px;line-height:1.6;color:{muted};text-align:{};\">{}</p>",
                s(props, "align", "center"),
                text_html(s(props, "text", ""), flavor)
            ),
        ),
        "component" => {
            let tag = s(props, "tag", "x-mail.component");
            row(
                preview_id,
                "8px 32px",
                &component_tag(tag, &props["props"], s(props, "slot", ""), flavor),
            )
        }
        other => row(
            preview_id,
            "8px 32px",
            &format!("<!-- unknown block {} -->", escape(other)),
        ),
    }
}

/// `<x-mail.button url="…" :color="$c">slot</x-mail.button>`.
pub fn component_tag(tag: &str, props: &Value, slot: &str, flavor: Flavor) -> String {
    let mut attrs = String::new();
    if let Some(map) = props.as_object() {
        for (key, value) in map {
            let value = value.as_str().unwrap_or("").trim();
            if value.is_empty() {
                continue;
            }
            // `$url` or `{{ $url }}` binds a variable; anything else is literal.
            let bound = value
                .strip_prefix("{{")
                .and_then(|v| v.strip_suffix("}}"))
                .map(str::trim)
                .unwrap_or(value);
            if bound.starts_with('$') {
                attrs.push_str(&format!(" :{key}=\"{}\"", bound.replace('"', "'")));
            } else {
                attrs.push_str(&format!(" {key}=\"{}\"", escape(value)));
            }
        }
    }
    if slot.trim().is_empty() {
        format!("<{tag}{attrs} />")
    } else {
        format!("<{tag}{attrs}>{}</{tag}>", text_html(slot, flavor))
    }
}

/// Laravel Markdown mailables: Markdown text and Laravel's mail components.
/// Lines start at column 0 (indentation would make a code block).
fn render_markdown(block: &str, props: &Value) -> String {
    let text = |key: &str| s(props, key, "").replace("\r\n", "\n");
    match block {
        "heading" => {
            let level = n(props, "level", 1).clamp(1, 3) as usize;
            format!("{} {}", "#".repeat(level), text("text").replace('\n', " "))
        }
        "text" | "footer" => text("text"),
        "button" => {
            let color = s(props, "style", "primary");
            component_tag(
                "x-mail::button",
                &json!({ "url": s(props, "url", "#"), "color": color }),
                s(props, "text", "Button"),
                Flavor::Markdown,
            )
        }
        "panel" => format!("<x-mail::panel>\n{}\n</x-mail::panel>", text("text")),
        "divider" => "---".into(),
        "spacer" => String::new(),
        "image" => format!("![{}]({})", s(props, "alt", ""), s(props, "src", "")),
        "component" => component_tag(
            s(props, "tag", "x-mail::panel"),
            &props["props"],
            s(props, "slot", ""),
            Flavor::Markdown,
        ),
        other => format!("{{{{-- unknown block {other} --}}}}"),
    }
}

/// The blocks each flavor offers.
pub fn palette(flavor: Flavor) -> &'static [&'static str] {
    match flavor {
        Flavor::Markdown => &[
            "heading",
            "text",
            "button",
            "panel",
            "image",
            "divider",
            "component",
        ],
        _ => &[
            "header",
            "heading",
            "text",
            "button",
            "image",
            "columns",
            "divider",
            "spacer",
            "footer",
            "component",
        ],
    }
}
