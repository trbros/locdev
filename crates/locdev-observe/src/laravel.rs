//! Laravel's log file (`storage/logs/laravel.log`, Monolog's LineFormatter):
//!
//! ```text
//! [2026-11-05 10:00:00] local.ERROR: Boom {"exception":"[object] (RuntimeException(code: 0): Boom at /app/routes/web.php:7)
//! [stacktrace]
//! #0 /app/vendor/…/Route.php(205): …
//! #1 {main}
//! "}
//! ```
//!
//! Entries start with `[date] channel.LEVEL: `; everything until the next
//! one belongs to it. Only read, never written.

use locdev_core::observe::{Event, EventKind, EventSeverity, EventSource};

use crate::parse::{blank, php_frame};

/// Splits appended text into entries; the last one waits for the next
/// entry (or `flush`) since more lines may follow.
#[derive(Default)]
pub struct LaravelLog {
    partial: String,
    entry: Vec<String>,
}

const MAX_ENTRY_LINES: usize = 500;

fn is_start(line: &str) -> bool {
    let b = line.as_bytes();
    b.len() > 22
        && b[0] == b'['
        && b[5] == b'-'
        && b[8] == b'-'
        && b[11] == b' '
        && line[1..].contains("] ")
}

impl LaravelLog {
    /// Text appended to the file since the last read.
    pub fn push(&mut self, chunk: &str, at: u64, from: EventSeverity) -> Vec<Event> {
        self.partial.push_str(chunk);
        let mut out = Vec::new();
        while let Some(nl) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=nl).collect();
            let line = line.trim_end_matches(['\n', '\r']).to_string();
            if is_start(&line) && !self.entry.is_empty() {
                out.extend(entry(&std::mem::take(&mut self.entry), at, from));
            }
            if (is_start(&line) || !self.entry.is_empty()) && self.entry.len() < MAX_ENTRY_LINES {
                self.entry.push(line);
            }
        }
        out
    }

    /// The entry in progress (after a pause in writes).
    pub fn flush(&mut self, at: u64, from: EventSeverity) -> Vec<Event> {
        if self.entry.is_empty() {
            return Vec::new();
        }
        entry(&std::mem::take(&mut self.entry), at, from)
            .into_iter()
            .collect()
    }
}

fn level(name: &str) -> Option<EventSeverity> {
    Some(match name {
        "DEBUG" => EventSeverity::Debug,
        "INFO" | "NOTICE" => EventSeverity::Info,
        "WARNING" => EventSeverity::Warn,
        "ERROR" => EventSeverity::Error,
        "CRITICAL" | "ALERT" | "EMERGENCY" => EventSeverity::Fatal,
        _ => return None,
    })
}

fn entry(lines: &[String], at: u64, from: EventSeverity) -> Option<Event> {
    let head = lines.first()?;
    let (stamp, rest) = head[1..].split_once("] ")?;
    let (channel_level, message) = rest.split_once(": ")?;
    let (channel, level_name) = channel_level.rsplit_once('.')?;
    let severity = level(level_name)?;
    if severity < from {
        return None;
    }
    // The message ends where Monolog's JSON context starts.
    let (message, context) = match message.find(" {\"") {
        Some(i) => (&message[..i], &message[i + 1..]),
        None => (
            message.trim_end_matches(" [] []").trim_end_matches(" []"),
            "",
        ),
    };
    let mut e = blank(EventKind::Log, severity, message.trim(), at);
    e.source = EventSource::LogFile;
    e.runtime = Some("php".into());
    e.framework = Some("laravel".into());
    e.context.insert("channel".into(), channel.to_string());
    e.context.insert("logged_at".into(), stamp.to_string());
    // `[object] (Class(code: 0): message at /file.php:12)`
    if let Some(obj) = context.split_once("[object] (").map(|(_, r)| r)
        && let Some((class, rest)) = obj.split_once("(code: ")
    {
        e.kind = EventKind::Exception;
        e.code = Some(class.to_string());
        if let Some((code, rest)) = rest.split_once("): ") {
            if code != "0" {
                e.context.insert("code".into(), code.to_string());
            }
            if let Some((msg, loc)) = rest.rsplit_once(" at ") {
                if e.message.is_empty() {
                    e.message = msg.to_string();
                }
                let loc = loc.trim_end_matches(')');
                if let Some((file, line)) = loc.rsplit_once(':') {
                    e.file = Some(file.to_string());
                    e.line = line.parse().ok();
                }
            }
        }
    }
    for line in &lines[1..] {
        if line.starts_with('#')
            && let Some(f) = php_frame(line)
        {
            e.stack.push(f);
        }
    }
    e.raw = lines.join("\n");
    Some(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = r#"[2026-11-05 10:00:00] local.INFO: User logged in {"id":3}
[2026-11-05 10:00:01] local.ERROR: Boom {"userId":3,"exception":"[object] (RuntimeException(code: 0): Boom at /app/routes/web.php:7)
[stacktrace]
#0 /app/vendor/laravel/framework/src/Illuminate/Routing/Route.php(205): Illuminate\\Routing\\RouteFileRegistrar->{closure}()
#1 /app/public/index.php(17): Illuminate\\Foundation\\Application->handleRequest()
#2 {main}
"}
[2026-11-05 10:00:02] local.WARNING: Slow query [] []
"#;

    #[test]
    fn entries_with_exceptions() {
        let mut log = LaravelLog::default();
        let mut out = log.push(LOG, 1, EventSeverity::Warn);
        out.extend(log.flush(1, EventSeverity::Warn));
        assert_eq!(out.len(), 2, "{out:#?}");
        let e = &out[0];
        assert_eq!(e.kind, EventKind::Exception);
        assert_eq!(e.code.as_deref(), Some("RuntimeException"));
        assert_eq!(e.message, "Boom");
        assert_eq!(
            (e.file.as_deref(), e.line),
            (Some("/app/routes/web.php"), Some(7))
        );
        assert_eq!(e.stack.len(), 2);
        assert_eq!(out[1].severity, EventSeverity::Warn);
        assert_eq!(out[1].message, "Slow query");
    }

    #[test]
    fn writes_split_across_reads() {
        let mut log = LaravelLog::default();
        let (a, b) = LOG.split_at(150);
        let mut out = log.push(a, 1, EventSeverity::Error);
        out.extend(log.push(b, 1, EventSeverity::Error));
        out.extend(log.flush(1, EventSeverity::Error));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].stack.len(), 2);
    }
}
