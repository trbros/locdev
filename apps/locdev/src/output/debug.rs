//! Rendering for `locdev debug …`.

use locdev_core::observe::{DumpNode, Event, EventGroup, EventSeverity, KeyKind, ObserveStatus};

use super::{Output, Style};

fn clock(ms: u64) -> String {
    let s = (ms / 1000) % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

fn kind(e: &Event) -> String {
    serde_json::to_value(e.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

impl Output {
    fn severity(&self, s: EventSeverity) -> String {
        match s {
            EventSeverity::Fatal => self.paint(Style::Red, "FATAL"),
            EventSeverity::Error => self.paint(Style::Red, "ERROR"),
            EventSeverity::Warn => self.paint(Style::Yellow, "WARN "),
            EventSeverity::Info => self.paint(Style::Dim, "INFO "),
            EventSeverity::Debug => self.paint(Style::Dim, "DEBUG"),
        }
    }

    pub fn debug_events(&self, events: &[Event]) {
        if self.json {
            self.print_json(events);
            return;
        }
        for e in events {
            let place = match (&e.file, e.line) {
                (Some(f), Some(l)) => format!("  {}:{l}", f.rsplit('/').next().unwrap_or(f)),
                (Some(f), None) => format!("  {}", f.rsplit('/').next().unwrap_or(f)),
                _ => String::new(),
            };
            let code = e
                .code
                .as_deref()
                .map(|c| format!("{c}: "))
                .unwrap_or_default();
            println!(
                "{} {} {:<9} {:<14} {}{}{}  {}",
                self.paint(Style::Dim, &clock(e.at)),
                self.severity(e.severity),
                kind(e),
                e.site.as_deref().unwrap_or("-"),
                code,
                e.message.lines().next().unwrap_or(""),
                self.paint(Style::Dim, &place),
                self.paint(Style::Dim, &format!("#{}", e.id)),
            );
        }
    }

    pub fn debug_groups(&self, groups: &[EventGroup]) {
        if self.json {
            self.print_json(groups);
            return;
        }
        if groups.is_empty() {
            println!("Nothing yet.");
        }
        for g in groups {
            let bars: String = g
                .hourly
                .iter()
                .map(|n| match n {
                    0 => '·',
                    1 => '▁',
                    2..=3 => '▃',
                    4..=9 => '▅',
                    _ => '█',
                })
                .collect();
            println!(
                "{:>5}×  {}  {:<14} {}{}",
                g.count,
                self.severity(g.severity),
                g.site.as_deref().unwrap_or("-"),
                g.code
                    .as_deref()
                    .map(|c| format!("{c}: "))
                    .unwrap_or_default(),
                g.message.lines().next().unwrap_or("")
            );
            println!(
                "        {}  last {} · {}",
                self.paint(Style::Dim, &bars),
                clock(g.last),
                self.paint(Style::Dim, &format!("locdev debug show {}", g.last_id))
            );
        }
    }

    pub fn debug_event(&self, e: &Event) {
        if self.json {
            self.print_json(e);
            return;
        }
        println!(
            "{} {}",
            self.severity(e.severity),
            self.paint(Style::Bold, &e.message)
        );
        let field = |label: &str, value: Option<String>| {
            if let Some(v) = value.filter(|v| !v.is_empty()) {
                println!("  {:<10} {v}", label);
            }
        };
        field("kind", Some(kind(e)));
        field("code", e.code.clone());
        field("site", e.site.clone());
        field("process", e.process.clone());
        field("framework", e.framework.clone());
        field("runtime", e.runtime.clone());
        field(
            "where",
            e.file.as_ref().map(|f| match e.line {
                Some(l) => format!("{f}:{l}"),
                None => f.clone(),
            }),
        );
        if let Some(r) = &e.request {
            field(
                "request",
                Some(
                    [
                        r.method.clone(),
                        r.url.clone(),
                        r.status.map(|s| s.to_string()),
                        r.duration_ms.map(|d| format!("{d} ms")),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                ),
            );
        }
        for (k, v) in &e.context {
            field(k, Some(v.clone()));
        }
        if let Some(hint) = &e.hint {
            println!(
                "  {}",
                self.paint(Style::Yellow, &format!("likely cause: {hint}"))
            );
        }
        if !e.stack.is_empty() {
            println!("\n  stack trace");
            for f in &e.stack {
                let place = match (&f.file, f.line) {
                    (Some(file), Some(l)) => format!("{file}:{l}"),
                    (Some(file), None) => file.clone(),
                    _ => String::new(),
                };
                let text = format!("    {} {}", f.function.as_deref().unwrap_or(""), place);
                println!(
                    "{}",
                    if f.vendor {
                        self.paint(Style::Dim, &text)
                    } else {
                        text
                    }
                );
            }
        }
        if let Some(d) = &e.dump {
            println!();
            let mut lines = Vec::new();
            dump_lines(d, 1, &mut lines);
            for l in lines {
                println!("{l}");
            }
        }
        if !e.raw.is_empty() && e.stack.is_empty() && e.dump.is_none() {
            println!("\n{}", self.paint(Style::Dim, &e.raw));
        }
    }

    pub fn debug_status(&self, s: &ObserveStatus) {
        if self.json {
            self.print_json(s);
            return;
        }
        println!(
            "collecting     {}",
            if s.settings.enabled { "on" } else { "off" }
        );
        println!(
            "kept           {} days, up to {} MB ({:.1} MB used)",
            s.settings.retention_days,
            s.settings.max_mb,
            s.disk_bytes as f64 / 1_048_576.0
        );
        println!("in memory      {} events", s.events);
        println!(
            "dump server    {}",
            s.dump_server.as_deref().unwrap_or("not listening")
        );
        println!(
            "dumps from     {}",
            if s.dump_sites.is_empty() {
                "no site (`locdev debug dumps <site> on`)".into()
            } else {
                s.dump_sites.join(", ")
            }
        );
    }
}

/// A dumped value as indented text (VarDumper's CLI look, roughly).
fn dump_lines(node: &DumpNode, depth: usize, out: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    let scalar = |n: &DumpNode| locdev_observe_summary(n);
    match node {
        DumpNode::Array { items, count, .. } => {
            out.push(format!("{pad}array:{count} ["));
            entries(items, depth + 1, out);
            out.push(format!("{pad}]"));
        }
        DumpNode::Object {
            class,
            handle,
            items,
            ..
        } => {
            out.push(format!("{pad}{class} {{#{}", handle.unwrap_or(0)));
            entries(items, depth + 1, out);
            out.push(format!("{pad}}}"));
        }
        other => out.push(format!("{pad}{}", scalar(other))),
    }
}

fn entries(items: &[locdev_core::observe::DumpEntry], depth: usize, out: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    for e in items {
        let key = match e.kind {
            KeyKind::Key => format!("\"{}\" =>", e.key),
            KeyKind::Public => format!("+{}:", e.key),
            KeyKind::Protected => format!("#{}:", e.key),
            KeyKind::Private => format!("-{}:", e.key),
            KeyKind::Meta => format!("{}:", e.key),
            KeyKind::Dynamic => format!("+\"{}\":", e.key),
        };
        match &e.value {
            DumpNode::Array { .. } | DumpNode::Object { .. } => {
                let mut inner = Vec::new();
                dump_lines(&e.value, depth, &mut inner);
                if let Some(first) = inner.first_mut() {
                    *first = format!("{pad}{key} {}", first.trim_start());
                }
                out.extend(inner);
            }
            v => out.push(format!("{pad}{key} {}", locdev_observe_summary(v))),
        }
    }
}

fn locdev_observe_summary(n: &DumpNode) -> String {
    match n {
        DumpNode::Str { v, .. } => format!("\"{v}\""),
        DumpNode::Text { v } => v.lines().next().unwrap_or("").to_string(),
        DumpNode::Null => "null".into(),
        DumpNode::Bool { v } => v.to_string(),
        DumpNode::Int { v } => v.to_string(),
        DumpNode::Float { v } => v.to_string(),
        DumpNode::Resource { kind, .. } => format!("{kind} resource"),
        DumpNode::Seen { class, handle } => format!(
            "{} {{#{}}} (above)",
            class.as_deref().unwrap_or("object"),
            handle.unwrap_or(0)
        ),
        DumpNode::Redacted => "[redacted]".into(),
        DumpNode::Truncated => "…".into(),
        DumpNode::Array { count, .. } => format!("array:{count}"),
        DumpNode::Object { class, .. } => class.clone(),
    }
}
