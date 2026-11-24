//! Rendering for `locdev php …`.

use locdev_core::model::now_ms;
use locdev_core::phpini::PhpConfigView;
use locdev_core::protocol::PhpLine;

use super::{Output, Style, ago};

impl Output {
    pub fn php_lines(&self, lines: &[PhpLine]) {
        if self.json {
            self.print_json(lines);
            return;
        }
        if lines.is_empty() {
            println!("No PHP installed (`locdev runtime install php 8.4`).");
        }
        for l in lines {
            println!(
                "{:<6} {:<10} {}  {}",
                l.line,
                l.version,
                if l.configured {
                    self.paint(Style::Green, "LocDev settings")
                } else {
                    self.paint(Style::Dim, "PHP's own values")
                },
                self.paint(Style::Dim, &l.executable.display().to_string())
            );
        }
    }

    pub fn php_config(&self, c: &PhpConfigView) {
        if self.json {
            self.print_json(c);
            return;
        }
        println!(
            "{}  {}  {}",
            self.paint(Style::Bold, &format!("PHP {}", c.line)),
            c.version,
            self.paint(Style::Dim, &c.executable.display().to_string())
        );
        match &c.loaded_ini {
            Some(ini) => println!("  php.ini   {}", ini.display()),
            None => println!(
                "  php.ini   {}",
                self.paint(Style::Dim, "none (PHP's built-in defaults)")
            ),
        }
        if !c.scanned.is_empty() {
            println!(
                "  scanned   {} files from PHP's scan folder",
                c.scanned.len()
            );
        }
        println!("  LocDev    {}", c.file.display());
        println!();
        println!(
            "  {:<24} {:<22} {:<16} {}",
            self.paint(Style::Dim, "directive"),
            self.paint(Style::Dim, "PHP's own"),
            self.paint(Style::Dim, "LocDev"),
            self.paint(Style::Dim, "in use")
        );
        for d in &c.directives {
            let dash = || "—".to_string();
            let value = d
                .value
                .clone()
                .map_or_else(dash, |v| self.paint(Style::Cyan, &v));
            println!(
                "  {:<24} {:<22} {:<16} {}",
                d.key,
                d.original.clone().unwrap_or_else(dash),
                value,
                d.effective.clone().unwrap_or_else(dash)
            );
        }
        for w in &c.warnings {
            println!("{} {w}", self.paint(Style::Yellow, "!"));
        }
        if !c.sites.is_empty() {
            println!("\nRunning on this line: {}", c.sites.join(", "));
        }
    }

    pub fn php_history(&self, c: &PhpConfigView) {
        if self.json {
            self.print_json(&c.history);
            return;
        }
        if c.history.is_empty() {
            println!("No earlier versions of LocDev's PHP {} file.", c.line);
        }
        for h in &c.history {
            println!(
                "{}  {} ago  {} lines",
                h.id,
                ago(h.at.min(now_ms())),
                h.lines
            );
        }
    }
}
