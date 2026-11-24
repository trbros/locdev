//! Process output → events. One `Assembler` per process reads its lines in
//! order and recognizes, per runtime, what spans several lines (stack
//! traces, panics, build errors) and what's one line (warnings, HTTP
//! errors of PHP's built-in server). Lines nothing recognizes become a log
//! event only at or above the chosen level (the supervisor's own guess).
//!
//! Formats, from the runtimes' own output:
//! - PHP: `PHP Fatal error:  Uncaught …`, `Stack trace:`, `#N file(line): …`
//!   and `  thrown in … on line N`; `PHP Warning:  … in … on line N`; the
//!   built-in server's `[date] 127.0.0.1:port [500]: GET /path`.
//! - Node.js: `Error: …` followed by `    at fn (file:line:col)`; Vite's
//!   `[vite] Internal server error: …`; Next.js' ` ⨯ …`.
//! - Python: `Traceback (most recent call last):`, `  File "…", line N, in f`,
//!   ending with `Type: message`; `logging`'s `ERROR:name:message`.
//! - Rust: `thread 'main' panicked at src/main.rs:10:5:` + message;
//!   `tracing_subscriber::fmt` lines (`<time>  ERROR target: message`);
//!   `error[E0425]: …` + `  --> file:line:col` from cargo.
//! - TypeScript: `file(line,col): error TS1234: …` / `file:line:col - error TS…`.

use std::collections::BTreeMap;

use locdev_core::observe::{Event, EventKind, EventSeverity, EventSource, Frame, RequestInfo};

/// A process output line, with the supervisor's level guess.
pub struct Line<'a> {
    pub text: &'a str,
    pub at: u64,
    pub level: Option<EventSeverity>,
}

/// An event before the pipeline gives it an id, a site and a fingerprint.
pub fn blank(
    kind: EventKind,
    severity: EventSeverity,
    message: impl Into<String>,
    at: u64,
) -> Event {
    Event {
        id: 0,
        at,
        kind,
        severity,
        source: EventSource::Output,
        site: None,
        process: None,
        message: message.into(),
        code: None,
        framework: None,
        runtime: None,
        file: None,
        line: None,
        stack: Vec::new(),
        request: None,
        dump: None,
        raw: String::new(),
        context: BTreeMap::new(),
        fingerprint: String::new(),
        hint: None,
    }
}

/// Removes terminal color codes (`ESC [ … m`) and other CSI sequences.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Dependency code: collapsed in stack traces.
pub fn is_vendor(file: &str) -> bool {
    let f = file.replace('\\', "/");
    [
        "/vendor/",
        "node_modules",
        "site-packages",
        "dist-packages",
        "/.cargo/registry/",
        "/rustc/",
        "/lib/python",
    ]
    .iter()
    .any(|m| f.contains(m))
        || f.starts_with("node:")
        || f.starts_with("internal/")
        || f.starts_with("<frozen")
}

fn frame(
    function: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
) -> Frame {
    let vendor = file.as_deref().is_some_and(is_vendor);
    Frame {
        function,
        file,
        line,
        column,
        vendor,
    }
}

/// `path:line[:col]` → parts (paths may hold colons, as `C:\…`).
fn split_location(s: &str) -> (String, Option<u32>, Option<u32>) {
    let s = s.trim().trim_start_matches("file://");
    let mut parts: Vec<&str> = s.rsplitn(3, ':').collect();
    parts.reverse();
    let num = |p: &str| p.parse::<u32>().ok();
    match parts.as_slice() {
        [path, l, c] if num(l).is_some() && num(c).is_some() => (path.to_string(), num(l), num(c)),
        [a, b, l] if num(l).is_some() => (format!("{a}:{b}"), num(l), None),
        [path, l] if num(l).is_some() => (path.to_string(), num(l), None),
        _ => (s.to_string(), None, None),
    }
}

/// `#0 /app/Foo.php(12): Foo->bar()` (PHP and Laravel's log).
pub fn php_frame(line: &str) -> Option<Frame> {
    let rest = line.trim().strip_prefix('#')?;
    let (_, rest) = rest.split_once(' ')?;
    if rest.starts_with('{') {
        return None;
    }
    if let Some(open) = rest.find('(')
        && let Some(close) = rest[open..].find("): ")
    {
        let file = rest[..open].to_string();
        let line_no = rest[open + 1..open + close].parse().ok();
        let function = rest[open + close + 3..].to_string();
        return Some(frame(Some(function), Some(file), line_no, None));
    }
    // `[internal function]: …`
    Some(frame(Some(rest.to_string()), None, None, None))
}

/// `    at fn (file:line:col)` / `    at file:line:col`.
fn node_frame(line: &str) -> Option<Frame> {
    let rest = line.trim_start().strip_prefix("at ")?;
    if let Some(open) = rest.rfind(" (")
        && rest.ends_with(')')
    {
        let (file, l, c) = split_location(&rest[open + 2..rest.len() - 1]);
        return Some(frame(Some(rest[..open].to_string()), Some(file), l, c));
    }
    let (file, l, c) = split_location(rest);
    Some(frame(None, Some(file), l, c))
}

/// `  File "/app/x.py", line 12, in handler`.
fn python_frame(line: &str) -> Option<Frame> {
    let rest = line.trim_start().strip_prefix("File \"")?;
    let (file, rest) = rest.split_once('"')?;
    let rest = rest.trim_start_matches(", line ");
    let (num, rest) = rest.split_once(", in ").unwrap_or((rest, ""));
    Some(frame(
        (!rest.is_empty()).then(|| rest.trim().to_string()),
        Some(file.to_string()),
        num.trim().parse().ok(),
        None,
    ))
}

/// `Type: message` closing a Python traceback.
fn python_exception(line: &str) -> Option<(String, String)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (ty, msg) = match line.split_once(": ") {
        Some((t, m)) => (t, m),
        None => (line.trim(), ""),
    };
    let last = ty.rsplit('.').next().unwrap_or(ty);
    let ok = !ty.is_empty()
        && ty
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && last.chars().next().is_some_and(|c| c.is_ascii_uppercase());
    ok.then(|| (ty.to_string(), msg.to_string()))
}

/// `TypeError: …`, `Error [ERR_X]: …`, `Uncaught ReferenceError: …`.
fn js_error_header(line: &str) -> Option<(String, String)> {
    let line = line.trim_start().trim_start_matches("Uncaught ");
    let (head, msg) = line.split_once(": ")?;
    let name = head.split(' ').next()?;
    (name.ends_with("Error") || name.ends_with("Exception"))
        .then(|| name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .filter(|ok| *ok)
        .map(|_| (name.to_string(), msg.to_string()))
}

/// `Exception: msg in /path/file.php:12` (PHP's uncaught summary).
fn php_uncaught(rest: &str) -> (Option<String>, String, Option<String>, Option<u32>) {
    let rest = rest.trim().trim_start_matches("Uncaught ");
    let (class, msg) = match rest.split_once(": ") {
        Some((c, m)) if !c.contains(' ') => (Some(c.to_string()), m),
        _ => (None, rest),
    };
    match msg.rsplit_once(" in ") {
        Some((m, loc)) => {
            let (file, line, _) = split_location(loc.trim_end_matches('.'));
            (class, m.to_string(), Some(file), line)
        }
        None => (class, msg.to_string(), None, None),
    }
}

#[derive(Debug)]
enum Block {
    Python {
        lines: Vec<String>,
        frames: Vec<Frame>,
    },
    Php {
        event: Event,
    },
    Node {
        event: Event,
        lines: Vec<String>,
    },
    Panic {
        event: Event,
        lines: Vec<String>,
    },
    Build {
        event: Event,
        lines: Vec<String>,
    },
}

/// Reads one process's output.
pub struct Assembler {
    runtime: Option<String>,
    framework: Option<String>,
    logs_from: EventSeverity,
    pending: Option<Block>,
}

const MAX_BLOCK_LINES: usize = 400;

impl Assembler {
    pub fn new(
        runtime: Option<String>,
        framework: Option<String>,
        logs_from: EventSeverity,
    ) -> Self {
        Self {
            runtime,
            framework,
            logs_from,
            pending: None,
        }
    }

    fn tag(&self, mut e: Event, runtime: &str) -> Event {
        e.runtime = Some(runtime.to_string()).or(self.runtime.clone());
        e.framework = self.framework.clone();
        e
    }

    /// Anything still being read (call after a pause in the output).
    pub fn flush(&mut self) -> Vec<Event> {
        match self.pending.take() {
            Some(block) => vec![self.close(block)],
            None => Vec::new(),
        }
    }

    fn close(&self, block: Block) -> Event {
        match block {
            Block::Python { lines, frames } => {
                // No closing line came: keep what was read.
                let mut e = blank(
                    EventKind::Exception,
                    EventSeverity::Error,
                    "Python traceback",
                    0,
                );
                e.stack = frames;
                e.raw = lines.join("\n");
                self.tag(e, "python")
            }
            Block::Php { event } => event,
            Block::Node { mut event, lines } => {
                if event.stack.is_empty() && event.kind == EventKind::Exception {
                    event.kind = EventKind::Log;
                }
                event.raw = lines.join("\n");
                event
            }
            Block::Panic { mut event, lines } | Block::Build { mut event, lines } => {
                if event.raw.is_empty() {
                    event.raw = lines.join("\n");
                }
                event
            }
        }
    }

    pub fn push(&mut self, line: Line<'_>) -> Vec<Event> {
        let text = strip_ansi(line.text);
        let text = text.trim_end_matches(['\r', '\n']);
        let mut out = Vec::new();

        if let Some(block) = self.pending.take() {
            match self.continue_block(block, text, line.at) {
                Step::More(block) => {
                    self.pending = Some(block);
                    return out;
                }
                Step::Done(event) => {
                    out.push(event);
                    return out;
                }
                Step::DoneAndRetry(event) => out.push(event),
            }
        }
        out.extend(self.start(text, line.at, line.level));
        out
    }

    fn continue_block(&self, block: Block, text: &str, at: u64) -> Step {
        match block {
            Block::Python {
                mut lines,
                mut frames,
            } => {
                if text.starts_with(' ') || text.is_empty() {
                    if let Some(f) = python_frame(text) {
                        frames.push(f);
                    }
                    lines.push(text.to_string());
                    return self.more(Block::Python { lines, frames });
                }
                if let Some((ty, msg)) = python_exception(text) {
                    lines.push(text.to_string());
                    let mut e = blank(
                        EventKind::Exception,
                        EventSeverity::Error,
                        if msg.is_empty() { ty.clone() } else { msg },
                        at,
                    );
                    e.code = Some(ty);
                    // Python lists the innermost call last.
                    frames.reverse();
                    if let Some(top) = frames.iter().find(|f| !f.vendor).or(frames.first()) {
                        e.file = top.file.clone();
                        e.line = top.line;
                    }
                    e.stack = frames;
                    e.raw = lines.join("\n");
                    return Step::Done(self.tag(e, "python"));
                }
                // Chained tracebacks: keep reading.
                lines.push(text.to_string());
                self.more(Block::Python { lines, frames })
            }
            Block::Php { mut event } => {
                let trimmed = text.trim();
                if trimmed == "Stack trace:" {
                    event.raw.push('\n');
                    event.raw.push_str(text);
                    return self.more(Block::Php { event });
                }
                if trimmed.starts_with('#') {
                    if let Some(f) = php_frame(trimmed) {
                        event.stack.push(f);
                    }
                    event.raw.push('\n');
                    event.raw.push_str(text);
                    return self.more(Block::Php { event });
                }
                if trimmed.starts_with("thrown in ") {
                    event.raw.push('\n');
                    event.raw.push_str(text);
                    return Step::Done(event);
                }
                Step::DoneAndRetry(event)
            }
            Block::Node {
                mut event,
                mut lines,
            } => {
                if let Some(f) = node_frame(text) {
                    if event.file.is_none() && !f.vendor {
                        event.file = f.file.clone();
                        event.line = f.line;
                    }
                    event.stack.push(f);
                    lines.push(text.to_string());
                    return self.more(Block::Node { event, lines });
                }
                if event.stack.is_empty() {
                    // A lone `Error: …` line: an error log, not an exception.
                    event.kind = EventKind::Log;
                }
                event.raw = lines.join("\n");
                Step::DoneAndRetry(event)
            }
            Block::Panic {
                mut event,
                mut lines,
            } => {
                let trimmed = text.trim_start();
                if trimmed.starts_with("note:")
                    || trimmed.starts_with("stack backtrace:")
                    || text.is_empty()
                {
                    lines.push(text.to_string());
                    event.raw = lines.join("\n");
                    return Step::Done(event);
                }
                if event.message.is_empty() || event.message == "panic" {
                    event.message = text.to_string();
                } else if !text.starts_with(' ') {
                    event.raw = lines.join("\n");
                    return Step::DoneAndRetry(event);
                }
                lines.push(text.to_string());
                self.more(Block::Panic { event, lines })
            }
            Block::Build {
                mut event,
                mut lines,
            } => {
                let trimmed = text.trim_start();
                let indented =
                    text.starts_with(' ') || text.starts_with('\t') || text.starts_with('|');
                if indented || trimmed.starts_with("-->") || text.is_empty() && lines.len() < 2 {
                    if let Some(loc) = trimmed
                        .strip_prefix("--> ")
                        .or_else(|| trimmed.strip_prefix("File: "))
                    {
                        let (file, line, col) = split_location(loc);
                        event.file.get_or_insert(file.clone());
                        event.line = event.line.or(line);
                        event.stack.push(frame(None, Some(file), line, col));
                    }
                    lines.push(text.to_string());
                    return self.more(Block::Build { event, lines });
                }
                event.raw = lines.join("\n");
                Step::DoneAndRetry(event)
            }
        }
    }

    fn more(&self, block: Block) -> Step {
        let size = match &block {
            Block::Python { lines, .. }
            | Block::Node { lines, .. }
            | Block::Panic { lines, .. }
            | Block::Build { lines, .. } => lines.len(),
            Block::Php { event } => event.stack.len(),
        };
        if size >= MAX_BLOCK_LINES {
            return Step::Done(self.close(block));
        }
        Step::More(block)
    }

    fn start(&mut self, text: &str, at: u64, level: Option<EventSeverity>) -> Vec<Event> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        // Python
        if trimmed.starts_with("Traceback (most recent call last):") {
            self.pending = Some(Block::Python {
                lines: vec![text.to_string()],
                frames: Vec::new(),
            });
            return Vec::new();
        }

        // PHP fatal / uncaught
        for (prefix, severity) in [
            ("PHP Fatal error:", EventSeverity::Fatal),
            ("PHP Parse error:", EventSeverity::Fatal),
            ("Fatal error:", EventSeverity::Fatal),
            ("Parse error:", EventSeverity::Fatal),
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let (class, message, file, line) = php_uncaught(rest);
                let kind = if prefix.contains("Parse") {
                    EventKind::Build
                } else {
                    EventKind::Exception
                };
                let mut e = blank(kind, severity, message, at);
                e.code = class.or_else(|| {
                    Some(
                        prefix
                            .trim_end_matches(':')
                            .trim_start_matches("PHP ")
                            .to_string(),
                    )
                });
                e.file = file;
                e.line = line;
                e.raw = text.to_string();
                self.pending = Some(Block::Php {
                    event: self.tag(e, "php"),
                });
                return Vec::new();
            }
        }
        // PHP warnings and notices (one line)
        for (prefix, severity) in [
            ("PHP Warning:", EventSeverity::Warn),
            ("PHP Deprecated:", EventSeverity::Warn),
            ("PHP Notice:", EventSeverity::Info),
            ("Warning:", EventSeverity::Warn),
            ("Deprecated:", EventSeverity::Warn),
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix)
                && rest.contains(" on line ")
                && severity >= self.logs_from
            {
                let rest = rest.trim();
                let (msg, loc) = rest.rsplit_once(" in ").unwrap_or((rest, ""));
                let (file, line) = match loc.rsplit_once(" on line ") {
                    Some((f, l)) => (Some(f.to_string()), l.trim().parse().ok()),
                    None => (None, None),
                };
                let mut e = blank(EventKind::Log, severity, msg, at);
                e.code = Some(
                    prefix
                        .trim_end_matches(':')
                        .trim_start_matches("PHP ")
                        .to_string(),
                );
                e.file = file;
                e.line = line;
                e.raw = text.to_string();
                return vec![self.tag(e, "php")];
            }
        }
        // PHP's built-in server: `[Thu Nov  5 10:00:00 2026] 127.0.0.1:54321 [500]: GET /path`
        if let Some(e) = php_server_line(trimmed, at) {
            let e = self.tag(e, "php");
            // ` - Uncaught …` is followed by `Stack trace:` lines.
            if e.kind == EventKind::Exception {
                self.pending = Some(Block::Php { event: e });
                return Vec::new();
            }
            return vec![e];
        }

        // Rust panics
        if let Some(rest) = trimmed.strip_prefix("thread '")
            && let Some((thread, rest)) = rest.split_once("' panicked at ")
        {
            let mut e = blank(EventKind::Exception, EventSeverity::Fatal, "panic", at);
            e.code = Some("panic".into());
            e.context.insert("thread".into(), thread.to_string());
            if let Some(old) = rest.strip_prefix('\'') {
                // Before Rust 1.73: `panicked at 'message', src/main.rs:10:5`
                if let Some((msg, loc)) = old.rsplit_once("', ") {
                    e.message = msg.to_string();
                    let (file, line, col) = split_location(loc);
                    e.stack.push(frame(None, Some(file.clone()), line, col));
                    e.file = Some(file);
                    e.line = line;
                }
            } else {
                let (file, line, col) = split_location(rest.trim_end_matches(':'));
                e.stack.push(frame(None, Some(file.clone()), line, col));
                e.file = Some(file);
                e.line = line;
            }
            let e = self.tag(e, "rust");
            self.pending = Some(Block::Panic {
                event: e,
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }
        // cargo/rustc errors
        // `error[E0425]: …` always; a bare `error: …` only from Rust builds.
        if let Some(rest) = trimmed.strip_prefix("error[").or_else(|| {
            trimmed
                .strip_prefix("error: ")
                .filter(|_| self.runtime.as_deref() == Some("rust"))
        }) && !rest.starts_with("could not compile")
        {
            let (code, msg) = match trimmed.strip_prefix("error[") {
                Some(r) => match r.split_once("]: ") {
                    Some((c, m)) => (Some(c.to_string()), m.to_string()),
                    None => (None, r.to_string()),
                },
                None => (None, rest.to_string()),
            };
            let mut e = blank(EventKind::Build, EventSeverity::Error, msg, at);
            e.code = code;
            self.pending = Some(Block::Build {
                event: self.tag(e, "rust"),
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }

        // Vite / Next.js / TypeScript
        if let Some(msg) = trimmed.strip_prefix("[vite] Internal server error: ") {
            let mut e = blank(EventKind::Build, EventSeverity::Error, msg, at);
            e.code = Some("vite".into());
            self.pending = Some(Block::Build {
                event: self.tag(e, "node"),
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }
        if let Some(msg) = trimmed.strip_prefix("⨯ ") {
            let mut e = blank(EventKind::Exception, EventSeverity::Error, msg, at);
            if let Some((name, m)) = js_error_header(msg) {
                e.code = Some(name);
                e.message = m;
            } else if msg.contains("Module not found") || msg.starts_with("./") {
                e.kind = EventKind::Build;
            }
            self.pending = Some(Block::Node {
                event: self.tag(e, "node"),
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }
        if let Some(e) = typescript_line(trimmed, at) {
            return vec![self.tag(e, "node")];
        }
        if trimmed == "Failed to compile." || trimmed.starts_with("Failed to compile") {
            let e = blank(EventKind::Build, EventSeverity::Error, trimmed, at);
            self.pending = Some(Block::Build {
                event: self.tag(e, "node"),
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }
        if let Some((name, msg)) = js_error_header(trimmed)
            && self.runtime.as_deref() != Some("python")
        {
            let mut e = blank(EventKind::Exception, EventSeverity::Error, msg, at);
            e.code = Some(name);
            self.pending = Some(Block::Node {
                event: self.tag(e, "node"),
                lines: vec![text.to_string()],
            });
            return Vec::new();
        }

        // Rust `tracing` and Python `logging` default formats
        if let Some(e) = tracing_line(trimmed, at) {
            return (e.severity >= self.logs_from)
                .then(|| self.tag(e, "rust"))
                .into_iter()
                .collect();
        }
        if let Some(e) = python_logging_line(trimmed, at) {
            return (e.severity >= self.logs_from)
                .then(|| self.tag(e, "python"))
                .into_iter()
                .collect();
        }

        // Anything else the supervisor marked as a warning or an error.
        match level {
            Some(severity) if severity >= self.logs_from => {
                let mut e = blank(EventKind::Log, severity, trimmed, at);
                e.raw = text.to_string();
                let runtime = self.runtime.clone().unwrap_or_default();
                vec![self.tag(e, &runtime)]
            }
            _ => Vec::new(),
        }
    }
}

enum Step {
    More(Block),
    Done(Event),
    /// The block ended before this line; read the line on its own.
    DoneAndRetry(Event),
}

fn php_server_line(line: &str, at: u64) -> Option<Event> {
    let rest = line.strip_prefix('[')?;
    let (_, rest) = rest.split_once("] ")?;
    let (_, rest) = rest.split_once(" [")?;
    let (status, rest) = rest.split_once("]: ")?;
    let status: u16 = status.parse().ok()?;
    let (method, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let (path, error) = match rest.split_once(" - ") {
        Some((p, e)) => (p.to_string(), Some(e)),
        None => (rest.to_string(), None),
    };
    let request = RequestInfo {
        method: Some(method.to_string()),
        url: Some(path.clone()),
        status: Some(status),
        ..Default::default()
    };
    // The request ended in an error PHP reported (it may still be a 200
    // with display_errors on).
    if let Some(error) = error.filter(|e| {
        e.starts_with("Uncaught ") || e.contains("Fatal error") || e.contains("Parse error")
    }) {
        let error = error
            .trim_start_matches("PHP ")
            .trim_start_matches("Fatal error: ")
            .trim_start_matches("Parse error: ");
        let (class, message, file, line_no) = php_uncaught(error);
        let mut e = blank(EventKind::Exception, EventSeverity::Fatal, message, at);
        e.code = class;
        e.file = file;
        e.line = line_no;
        e.request = Some(request);
        e.raw = line.to_string();
        return Some(e);
    }
    if status < 500 {
        return None;
    }
    let mut e = blank(
        EventKind::Http,
        EventSeverity::Error,
        format!("{status} {method} {path}"),
        at,
    );
    e.code = Some(status.to_string());
    e.request = Some(request);
    e.raw = line.to_string();
    Some(e)
}

/// `src/a.ts(3,7): error TS2322: …` or `src/a.ts:3:7 - error TS2322: …`.
fn typescript_line(line: &str, at: u64) -> Option<Event> {
    let (loc, rest) = line
        .split_once(": error TS")
        .or_else(|| line.split_once(" - error TS"))?;
    let (code, msg) = rest.split_once(": ")?;
    if !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (file, l, c) = match loc.rsplit_once('(') {
        Some((f, pos)) => {
            let pos = pos.trim_end_matches(')');
            let (l, c) = pos.split_once(',').unwrap_or((pos, ""));
            (f.to_string(), l.parse().ok(), c.parse().ok())
        }
        None => split_location(loc),
    };
    let mut e = blank(EventKind::Build, EventSeverity::Error, msg, at);
    e.code = Some(format!("TS{code}"));
    e.stack.push(frame(None, Some(file.clone()), l, c));
    e.file = Some(file);
    e.line = l;
    e.raw = line.to_string();
    Some(e)
}

fn severity_word(w: &str) -> Option<EventSeverity> {
    Some(match w {
        "ERROR" | "error" => EventSeverity::Error,
        "WARN" | "WARNING" | "warn" | "warning" => EventSeverity::Warn,
        "INFO" | "info" => EventSeverity::Info,
        "DEBUG" | "TRACE" | "debug" | "trace" => EventSeverity::Debug,
        "CRITICAL" | "FATAL" | "critical" | "fatal" => EventSeverity::Fatal,
        _ => return None,
    })
}

/// `2026-11-05T10:00:00.123456Z ERROR my_app::server: message key=value`.
fn tracing_line(line: &str, at: u64) -> Option<Event> {
    let mut parts = line.split_whitespace();
    let time = parts.next()?;
    if !(time.len() >= 19 && time.as_bytes()[4] == b'-' && time.contains('T')) {
        return None;
    }
    let severity = severity_word(parts.next()?)?;
    let rest = line
        .splitn(3, char::is_whitespace)
        .filter(|p| !p.is_empty())
        .nth(2)
        .unwrap_or("")
        .trim_start();
    let rest = rest
        .trim_start_matches(|c: char| c.is_ascii_uppercase())
        .trim_start();
    let (target, msg) = rest.split_once(": ").unwrap_or(("", rest));
    let mut e = blank(EventKind::Log, severity, msg, at);
    if !target.is_empty() {
        e.context.insert("target".into(), target.to_string());
    }
    e.raw = line.to_string();
    Some(e)
}

/// `ERROR:app.views:message` (Python's `logging.basicConfig` default).
fn python_logging_line(line: &str, at: u64) -> Option<Event> {
    let (level, rest) = line.split_once(':')?;
    let severity =
        severity_word(level).filter(|_| level.chars().all(|c| c.is_ascii_uppercase()))?;
    let (logger, msg) = rest.split_once(':')?;
    if logger.contains(' ') || logger.is_empty() {
        return None;
    }
    let mut e = blank(EventKind::Log, severity, msg, at);
    e.context.insert("logger".into(), logger.to_string());
    e.raw = line.to_string();
    Some(e)
}

#[cfg(test)]
mod tests;
