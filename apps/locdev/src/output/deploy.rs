//! Rendering for `locdev deploy …`.

use locdev_core::deploy::{CheckLevel, DeployConfig, DeployReport, Rendering, Verdict};

use super::{Output, Style};

impl Output {
    pub fn deploy_report(&self, r: &DeployReport) {
        if self.json {
            self.print_json(r);
            return;
        }
        let p = &r.profile;
        let rendering = match p.rendering {
            Rendering::Static => "static files",
            Rendering::Spa => "client-side app (static files)",
            Rendering::Ssr => "server rendering",
            Rendering::Server => "application server",
        };
        println!(
            "{}{}  {} {}",
            self.paint(Style::Bold, &p.framework),
            p.version
                .as_deref()
                .map(|v| format!(" {v}"))
                .unwrap_or_default(),
            p.runtime,
            p.runtime_version
                .as_deref()
                .unwrap_or("(version not declared)")
        );
        println!(
            "  {rendering}  {}",
            self.paint(Style::Dim, &format!("({})", p.rendering_evidence))
        );
        if let Some(b) = &p.build_command {
            println!("  build  {b}");
        }
        if let Some(s) = &p.start_command {
            println!("  start  {s}");
        }
        if let Some(o) = &p.output_dir {
            println!("  output {o}");
        }
        for n in &p.needs {
            println!(
                "  needs  {:<30} {}",
                n.label,
                self.paint(Style::Dim, &n.evidence)
            );
        }
        if !p.env.is_empty() {
            let secrets = p.env.iter().filter(|e| e.secret).count();
            let names: Vec<&str> = p.env.iter().take(12).map(|e| e.name.as_str()).collect();
            let more = p.env.len().saturating_sub(names.len());
            println!(
                "  env    {} variables ({secrets} secret): {}{}",
                p.env.len(),
                names.join(", "),
                if more > 0 {
                    format!(" and {more} more (--json lists them)")
                } else {
                    String::new()
                }
            );
        }

        println!("\n{}", self.paint(Style::Bold, "Platforms"));
        for f in &r.providers {
            let (label, style) = match f.verdict {
                Verdict::Fits => ("fits", Style::Green),
                Verdict::NeedsChanges => ("changes", Style::Yellow),
                Verdict::NotSuitable => ("no", Style::Red),
            };
            println!(
                "{:<8} {}  {}",
                self.paint(style, label),
                self.paint(Style::Bold, &f.name),
                self.paint(Style::Dim, &f.model)
            );
            let points = if f.verdict == Verdict::NotSuitable {
                &f.limitations
            } else {
                &f.requirements
            };
            for point in points.iter().take(if f.verdict == Verdict::NotSuitable {
                1
            } else {
                8
            }) {
                println!("{:<8}   - {}", "", point.text);
            }
            if f.verdict != Verdict::NotSuitable {
                if let Some(file) = &f.config_file {
                    println!(
                        "{:<8}   {}",
                        "",
                        self.paint(
                            Style::Cyan,
                            &format!("locdev deploy config {} prints a {file}", f.id)
                        )
                    );
                } else if let Some(setup) = &f.setup {
                    println!("{:<8}   {}", "", self.paint(Style::Dim, setup));
                }
            }
        }

        println!("\n{}", self.paint(Style::Bold, "Readiness"));
        for c in &r.checks {
            let (label, style) = match c.level {
                CheckLevel::Ok => ("ok", Style::Green),
                CheckLevel::Warn => ("warn", Style::Yellow),
                CheckLevel::Fail => ("fail", Style::Red),
            };
            println!(
                "{:<6} {}  {}",
                self.paint(style, label),
                c.title,
                self.paint(Style::Dim, &c.detail)
            );
        }
        println!("\n{}", self.paint(Style::Dim, &format!("Platform facts compiled {} from their docs; `--json` has every source link. Nothing was sent anywhere.", r.reviewed)));
    }

    pub fn deploy_config(&self, c: &DeployConfig) {
        if self.json {
            self.print_json(c);
            return;
        }
        eprintln!(
            "{}{}",
            self.paint(Style::Bold, &c.file),
            if c.exists {
                self.paint(Style::Yellow, "  (already exists: not written)")
            } else {
                String::new()
            }
        );
        print!("{}", c.content);
        for note in &c.notes {
            eprintln!("{}", self.paint(Style::Dim, &format!("note: {note}")));
        }
    }
}
