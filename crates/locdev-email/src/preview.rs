//! A static preview: LocDev substitutes sample data and expands the
//! project's components itself, without running anything. Blade logic
//! (`@if`, `@foreach`) isn't evaluated; the framework preview does that.

use std::path::Path;

use locdev_core::email::EmailFormat;
use serde_json::Value;

use crate::blocks::escape;

pub struct Rendered {
    pub html: String,
    pub text: String,
    pub notes: Vec<String>,
}

fn lookup<'a>(data: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut cur = data;
    for key in path {
        cur = match cur {
            Value::Object(map) => map.get(key)?,
            Value::Array(items) => items.get(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn show(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `$user->name`, `$order['id']`, with `?? 'default'`, plus the helpers a
/// template commonly calls. Anything else shows as `‹expr›`.
pub fn evaluate(expr: &str, data: &Value) -> String {
    let expr = expr.trim();
    if let Some((left, right)) = expr.split_once("??") {
        let value = evaluate_opt(left.trim(), data);
        return value
            .unwrap_or_else(|| unquote(right.trim()).unwrap_or_else(|| evaluate(right, data)));
    }
    evaluate_opt(expr, data).unwrap_or_else(|| format!("‹{expr}›"))
}

fn unquote(s: &str) -> Option<String> {
    let q = s.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    s.strip_prefix(q)?.strip_suffix(q).map(str::to_string)
}

fn first_arg(call: &str) -> Option<String> {
    let inner = call.split_once('(')?.1;
    let arg = inner.split([',', ')']).next()?.trim();
    unquote(arg)
}

fn evaluate_opt(expr: &str, data: &Value) -> Option<String> {
    if let Some(s) = unquote(expr) {
        return Some(s);
    }
    if let Some(var) = expr.strip_prefix('$') {
        let mut path = Vec::new();
        let mut rest = var;
        let name_end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        path.push(rest[..name_end].to_string());
        rest = &rest[name_end..];
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix("->") {
                let end = r
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(r.len());
                path.push(r[..end].to_string());
                rest = &r[end..];
            } else {
                let r = rest.strip_prefix('[')?;
                let end = r.find(']')?;
                path.push(unquote(r[..end].trim()).unwrap_or_else(|| r[..end].trim().to_string()));
                rest = &r[end + 1..];
            }
        }
        return lookup(data, &path).map(show);
    }
    let name = expr.split('(').next()?.trim();
    match name {
        "config" => {
            let key = first_arg(expr)?;
            Some(
                data.get("config")
                    .and_then(|c| c.get(&key))
                    .map(show)
                    .unwrap_or_else(|| {
                        if key == "app.name" {
                            "Laravel".into()
                        } else {
                            key
                        }
                    }),
            )
        }
        "__" | "trans" | "trans_choice" => first_arg(expr),
        "route" => first_arg(expr).map(|r| format!("https://example.test/{}", r.replace('.', "/"))),
        "url" | "asset" => {
            first_arg(expr).map(|u| format!("https://example.test/{}", u.trim_start_matches('/')))
        }
        // The year is what templates usually print (`© {{ date('Y') }}`).
        "date" if first_arg(expr).as_deref() == Some("Y") => Some(current_year().to_string()),
        "date" | "now" => Some(format!("‹{expr}›")),
        _ => None,
    }
}

fn current_year() -> i64 {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64 / 86_400);
    // Civil year from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(mp >= 10)
}

/// Removes `{{-- … --}}`.
fn strip_blade_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("--}}") {
            Some(end) => rest = &rest[start + end + 4..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

const DIRECTIVES: &[&str] = &[
    "@if",
    "@elseif",
    "@else",
    "@endif",
    "@foreach",
    "@endforeach",
    "@for",
    "@endfor",
    "@forelse",
    "@endforelse",
    "@empty",
    "@endempty",
    "@unless",
    "@endunless",
    "@isset",
    "@endisset",
    "@auth",
    "@endauth",
    "@guest",
    "@endguest",
    "@switch",
    "@case",
    "@break",
    "@default",
    "@endswitch",
    "@while",
    "@endwhile",
    "@props",
    "@php",
    "@endphp",
    "@include",
    "@extends",
    "@section",
    "@endsection",
    "@yield",
    "@lang",
];

/// Drops lines that are only Blade directives (`@if(...)`, `@endforeach`);
/// returns how many were dropped.
fn strip_directives(text: &str) -> (String, usize) {
    let mut out = String::new();
    let mut dropped = 0;
    let mut in_php = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        if in_php {
            dropped += 1;
            if t.starts_with("@endphp") {
                in_php = false;
            }
            continue;
        }
        if t == "@php" {
            in_php = true;
            dropped += 1;
            continue;
        }
        if DIRECTIVES.iter().any(|d| {
            t.starts_with(d)
                && t[d.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric())
        }) {
            dropped += 1;
            continue;
        }
        out.push_str(line);
    }
    (out, dropped)
}

/// `{{ expr }}` (escaped) and `{!! expr !!}` (raw) with sample data.
fn substitute(text: &str, data: &Value) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let next = [rest.find("{{"), rest.find("{!!")]
            .into_iter()
            .flatten()
            .min();
        let Some(i) = next else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..i]);
        let raw = rest[i..].starts_with("{!!");
        let (open, close) = if raw { (3, "!!}") } else { (2, "}}") };
        match rest[i + open..].find(close) {
            Some(end) => {
                let value = evaluate(&rest[i + open..i + open + end], data);
                out.push_str(&if raw { value } else { escape(&value) });
                rest = &rest[i + open + end + close.len()..];
            }
            None => {
                out.push_str(&rest[i..]);
                break;
            }
        }
    }
    out
}

/// `key="v"` and `:key="$expr"` of a tag's attributes.
fn tag_attrs(tag: &str, data: &Value) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    let mut rest = tag;
    while let Some(eq) = rest.find("=\"") {
        let key_start = rest[..eq].rfind([' ', '\n', '\t']).map_or(0, |i| i + 1);
        let key = &rest[key_start..eq];
        let Some(end) = rest[eq + 2..].find('"') else {
            break;
        };
        let value = &rest[eq + 2..eq + 2 + end];
        match key.strip_prefix(':') {
            Some(k) => out.insert(k.to_string(), Value::String(evaluate(value, data))),
            None => out.insert(key.to_string(), Value::String(value.to_string())),
        };
        rest = &rest[eq + 2 + end + 1..];
    }
    out
}

/// Inlines `<x-mail.name …>slot</x-mail.name>` with the project's component.
fn expand_components(text: &str, root: &Path, data: &Value, depth: usize) -> String {
    if depth > 3 {
        return text.to_string();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<x-mail.") {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let Some(tag_end) = tail.find('>') else {
            out.push_str(tail);
            return out;
        };
        let name_end = tail[1..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .map_or(tag_end, |e| e + 1);
        let tag_name = &tail[1..name_end];
        let open_tag = &tail[..tag_end + 1];
        let self_closing = open_tag.ends_with("/>");
        let (slot, consumed) = if self_closing {
            (String::new(), tag_end + 1)
        } else {
            let close = format!("</{tag_name}>");
            match tail.find(&close) {
                Some(c) => (tail[tag_end + 1..c].to_string(), c + close.len()),
                None => (String::new(), tag_end + 1),
            }
        };
        let file = root.join("resources/views/components").join(format!(
            "{}.blade.php",
            tag_name.trim_start_matches("x-").replace('.', "/")
        ));
        match std::fs::read_to_string(&file) {
            Ok(component) => {
                let mut scope = serde_json::Map::new();
                for prop in crate::discover::blade_props(&component) {
                    if let Some(d) = prop.default.as_deref().and_then(unquote) {
                        scope.insert(prop.name, Value::String(d));
                    }
                }
                scope.extend(tag_attrs(open_tag, data));
                let slot = expand_components(&substitute(&slot, data), root, data, depth + 1);
                scope.insert("slot".into(), Value::String(slot));
                let (body, _) = strip_directives(&strip_blade_comments(&component));
                // Slot is already HTML: insert it raw.
                let body = body.replace("{{ $slot }}", "{!! $slot !!}").replace("{{$slot}}", "{!! $slot !!}");
                out.push_str(&expand_components(&substitute(&body, &Value::Object(scope)), root, data, depth + 1));
            }
            Err(_) => out.push_str(&format!("<div style=\"border:1px dashed #f59e0b;padding:8px;color:#92400e\">‹{tag_name}: no component file›</div>")),
        }
        rest = &tail[consumed..];
    }
    out.push_str(rest);
    out
}

/// Laravel's own Markdown mail components, approximated.
fn expand_laravel_mail(text: &str) -> String {
    let mut out = text.to_string();
    // <x-mail::button url="…" color="…">Text</x-mail::button>
    while let Some(start) = out.find("<x-mail::button") {
        let Some(tag_end) = out[start..].find('>').map(|e| start + e) else {
            break;
        };
        let Some(close) = out[tag_end..]
            .find("</x-mail::button>")
            .map(|e| tag_end + e)
        else {
            break;
        };
        let attrs = tag_attrs(&out[start..tag_end], &Value::Null);
        let url = attrs
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("#")
            .to_string();
        let label = out[tag_end + 1..close].trim().to_string();
        let html = format!(
            "<table role=\"presentation\" align=\"center\" style=\"margin:24px auto\"><tr><td style=\"background:#2d3748;border-radius:4px\"><a href=\"{}\" style=\"display:inline-block;padding:8px 18px;color:#fff;text-decoration:none\">{label}</a></td></tr></table>",
            escape(&url)
        );
        out.replace_range(start..close + "</x-mail::button>".len(), &html);
    }
    for (tag, open, close) in [
        (
            "panel",
            "<div style=\"border-left:4px solid #2d3748;background:#edf2f7;padding:12px 16px;margin:16px 0\">",
            "</div>",
        ),
        (
            "subcopy",
            "<div style=\"border-top:1px solid #e8e5ef;margin-top:24px;padding-top:16px;font-size:12px;color:#718096\">",
            "</div>",
        ),
    ] {
        out = out
            .replace(&format!("<x-mail::{tag}>"), open)
            .replace(&format!("</x-mail::{tag}>"), close);
    }
    out
}

/// A small Markdown subset: headings, paragraphs, links, bold, lists, rules.
fn markdown(text: &str) -> String {
    let inline = |s: &str| {
        let mut s = s.to_string();
        while let (Some(a), Some(b)) = (
            s.find("**"),
            s.get(s.find("**").unwrap_or(0) + 2..)
                .and_then(|r| r.find("**")),
        ) {
            let b = a + 2 + b;
            s = format!(
                "{}<strong>{}</strong>{}",
                &s[..a],
                &s[a + 2..b],
                &s[b + 2..]
            );
        }
        while let Some(open) = s.find('[') {
            let Some(mid) = s[open..].find("](").map(|m| open + m) else {
                break;
            };
            let Some(end) = s[mid..].find(')').map(|e| mid + e) else {
                break;
            };
            let link = format!(
                "<a href=\"{}\" style=\"color:#3869d4\">{}</a>",
                &s[mid + 2..end],
                &s[open + 1..mid]
            );
            s.replace_range(open..end + 1, &link);
        }
        s
    };
    let mut html = String::new();
    let mut paragraph: Vec<String> = Vec::new();
    let flush = |html: &mut String, p: &mut Vec<String>| {
        if !p.is_empty() {
            html.push_str(&format!(
                "<p style=\"margin:0 0 16px;line-height:1.6\">{}</p>",
                p.join(" ")
            ));
            p.clear();
        }
    };
    for line in text.lines() {
        let t = line.trim_end();
        if t.trim().is_empty() {
            flush(&mut html, &mut paragraph);
        } else if t.starts_with('<') {
            flush(&mut html, &mut paragraph);
            html.push_str(t);
        } else if let Some(h) = t
            .strip_prefix("### ")
            .or_else(|| t.strip_prefix("## "))
            .or_else(|| t.strip_prefix("# "))
        {
            flush(&mut html, &mut paragraph);
            let level = t.len() - t.trim_start_matches('#').len();
            html.push_str(&format!(
                "<h{level} style=\"margin:0 0 12px;color:#3d4852\">{}</h{level}>",
                inline(h)
            ));
        } else if t == "---" || t == "***" {
            flush(&mut html, &mut paragraph);
            html.push_str("<hr style=\"border:0;border-top:1px solid #e8e5ef;margin:16px 0\">");
        } else if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            flush(&mut html, &mut paragraph);
            html.push_str(&format!(
                "<ul style=\"margin:0 0 8px\"><li>{}</li></ul>",
                inline(item)
            ));
        } else {
            paragraph.push(inline(t.trim()));
        }
    }
    flush(&mut html, &mut paragraph);
    html
}

pub fn text_of(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut in_style = false;
    let lower = html.to_ascii_lowercase();
    // `to_ascii_lowercase` keeps byte offsets, so indices from `html` work in `lower`.
    for (i, c) in html.char_indices() {
        let ahead = &lower[i..];
        if c == '<' {
            if ahead.starts_with("<style") {
                in_style = true;
            } else if ahead.starts_with("</style") {
                in_style = false;
            }
            in_tag = true;
            if ["<br", "</p", "</tr", "</h", "</div", "</li"]
                .iter()
                .any(|t| ahead.starts_with(t))
            {
                out.push('\n');
            }
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag && !in_style {
            out.push(c);
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&");
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render(content: &str, format: EmailFormat, data: &Value, root: &Path) -> Rendered {
    let mut notes = Vec::new();
    let text = match format {
        EmailFormat::Html => content.to_string(),
        _ => strip_blade_comments(content),
    };
    let (text, dropped) = strip_directives(&text);
    if dropped > 0 {
        notes.push(format!(
            "Static preview: {dropped} Blade directive line(s) (@if, @foreach…) aren't evaluated, so every branch shows. The framework preview runs them."
        ));
    }
    if text.contains("@include") || text.contains("@extends") {
        notes.push("@include/@extends aren't expanded here; the framework preview does it.".into());
    }
    let expanded = expand_components(&substitute(&text, data), root, data, 0);
    let html = match format {
        EmailFormat::Markdown => {
            let body = expand_laravel_mail(&expanded);
            let inner = body
                .replace("<x-mail::message>", "")
                .replace("</x-mail::message>", "");
            format!(
                "<!doctype html><html><body style=\"margin:0;background:#edf2f7;font-family:-apple-system,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#718096\"><table role=\"presentation\" width=\"100%\" style=\"padding:32px 12px\"><tr><td align=\"center\"><table role=\"presentation\" width=\"570\" style=\"max-width:570px;background:#fff;border-radius:2px;padding:32px\"><tr><td>{}</td></tr></table></td></tr></table></body></html>",
                markdown(&inner)
            )
        }
        EmailFormat::Text => format!(
            "<pre style=\"font-family:ui-monospace,monospace;white-space:pre-wrap;padding:16px\">{}</pre>",
            escape(&expanded)
        ),
        _ => expanded.clone(),
    };
    let text = if format == EmailFormat::Text {
        // Values were HTML-escaped for the page; the text version shows them plain.
        expanded
            .trim()
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
    } else {
        text_of(&html)
    };
    Rendered { html, text, notes }
}
