//! Rendering for `locdev config …`.

use locdev_core::config::{
    ConfigFile, ConfigFileView, ConfigFinding, ConfigHistoryEntry, ConfigPreview, FindingKind,
    OptionState,
};

use super::{Output, Style, truncate};

impl Output {
    pub fn config_files(&self, files: &[ConfigFile]) {
        if self.json {
            self.print_json(files);
            return;
        }
        if files.is_empty() {
            println!("No config files LocDev knows in this site.");
        }
        for f in files {
            let version = match (&f.package, &f.version) {
                (Some(p), Some(v)) => format!("{p} {v}"),
                (Some(p), None) => format!("{p} (not installed)"),
                _ => String::new(),
            };
            println!(
                "{:<28} {:<34} {:<24} {}",
                f.path,
                f.label,
                self.paint(Style::Dim, &version),
                if f.visual { "" } else { "text only" }
            );
        }
    }

    pub fn config_view(&self, view: &ConfigFileView) {
        if self.json {
            self.print_json(view);
            return;
        }
        let f = &view.file;
        println!(
            "{}  {}",
            self.paint(Style::Bold, &f.path),
            self.paint(Style::Dim, &f.docs)
        );
        if let Some(error) = &view.syntax_error {
            println!(
                "{}",
                self.paint(Style::Red, &format!("syntax error: {error}"))
            );
        }
        if let Some(why) = &view.visual_blocked {
            println!("{}", self.paint(Style::Yellow, why));
        }
        for note in &view.notes {
            println!("{}", self.paint(Style::Dim, note));
        }
        let mut section = "";
        for o in &view.options {
            if o.section != section {
                section = &o.section;
                println!("\n{}", self.paint(Style::Bold, section));
            }
            let value = match &o.state {
                OptionState::Set { value } => value.to_string(),
                OptionState::Code { code } => format!("code: {}", truncate(code, 40)),
                OptionState::Missing => match &o.default {
                    Some(d) => self.paint(Style::Dim, &format!("(default {d})")),
                    None => self.paint(Style::Dim, "(not set)"),
                },
            };
            println!("  {:<40} {}", o.key, value);
            if let Some(why) = &o.blocked {
                println!("  {:<40} {}", "", self.paint(Style::Yellow, why));
            }
        }
    }

    pub fn config_preview(&self, preview: &ConfigPreview) {
        if self.json {
            self.print_json(preview);
            return;
        }
        if preview.diff.is_empty() {
            eprintln!("No change: the file already says that.");
            return;
        }
        for line in preview.diff.lines() {
            let style = match line.chars().next() {
                Some('+') if !line.starts_with("+++") => Style::Green,
                Some('-') if !line.starts_with("---") => Style::Red,
                Some('@') => Style::Cyan,
                _ => Style::Dim,
            };
            eprintln!("{}", self.paint(style, line));
        }
        for comment in &preview.removed_comments {
            eprintln!(
                "{}",
                self.paint(
                    Style::Yellow,
                    &format!("removes a comment: {}", comment.trim())
                )
            );
        }
    }

    pub fn config_findings(&self, findings: &[ConfigFinding]) {
        if self.json {
            self.print_json(findings);
            return;
        }
        if findings.is_empty() {
            println!(
                "No findings: the config matches the installed versions as far as LocDev's rules go."
            );
        }
        for f in findings {
            let (label, style) = match f.kind {
                FindingKind::Incompatible => ("incompatible", Style::Red),
                FindingKind::Deprecated => ("deprecated", Style::Yellow),
                FindingKind::Security => ("security", Style::Red),
                FindingKind::Performance => ("performance", Style::Yellow),
                FindingKind::Development => ("development", Style::Yellow),
                FindingKind::Available => ("available", Style::Cyan),
            };
            println!(
                "{:<13} {}  {}",
                self.paint(style, label),
                self.paint(Style::Bold, &f.title),
                self.paint(Style::Dim, &f.file)
            );
            println!("{:<13} {}", "", f.detail);
            println!(
                "{:<13} {}  {}",
                "",
                self.paint(Style::Dim, &f.applies),
                self.paint(Style::Blue, &f.source)
            );
            if let Some((key, value)) = &f.fix {
                println!(
                    "{:<13} fix: locdev config set {} {key} '{value}'",
                    "", f.file
                );
            }
        }
    }

    pub fn config_history(&self, entries: &[ConfigHistoryEntry]) {
        if self.json {
            self.print_json(entries);
            return;
        }
        if entries.is_empty() {
            println!("Nothing kept yet: LocDev keeps a copy before each save it makes.");
        }
        for e in entries {
            println!("{:<16} {:>8} B  replaced by: {}", e.id, e.size, e.reason);
        }
    }
}
