//! Rendering for `locdev park …`.

use locdev_core::park::{OnMissing, OnNew, ParkedState, ParkedView};
use serde_json::json;

use super::{Output, Style, ago};

impl Output {
    pub fn parked(&self, dirs: &[ParkedView], added: &[String], removed: &[String]) {
        if self.json {
            self.print_json(json!({ "dirs": dirs, "added": added, "removed": removed }));
            return;
        }
        if dirs.is_empty() {
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    "No parked folders. `locdev park ~/Development` turns every project in it into a site."
                )
            );
        }
        for (i, view) in dirs.iter().enumerate() {
            if i > 0 {
                println!();
            }
            self.parked_dir(view);
        }
        if !added.is_empty() {
            println!(
                "\n{} {}",
                self.paint(Style::Green, "Added:"),
                added.join(", ")
            );
        }
        if !removed.is_empty() {
            println!(
                "{} {}",
                self.paint(Style::Yellow, "Removed:"),
                removed.join(", ")
            );
        }
    }

    fn parked_dir(&self, view: &ParkedView) {
        let dir = &view.dir;
        let mut facts = vec![if !dir.enabled {
            self.paint(Style::Dim, "off")
        } else if !view.exists {
            self.paint(Style::Red, "missing")
        } else if view.watching {
            self.paint(Style::Green, "watching")
        } else {
            self.paint(Style::Yellow, "rescanned every 5 min")
        }];
        facts.push(format!("depth {}", dir.depth));
        facts.push(match dir.on_new {
            OnNew::Register => "new: become sites".into(),
            OnNew::Suggest => "new: suggested".into(),
        });
        facts.push(match dir.on_missing {
            OnMissing::Unregister => "gone: removed".into(),
            OnMissing::Keep => "gone: kept".into(),
        });
        if let Some(at) = view.last_scan {
            facts.push(format!("scanned {} ago", ago(at)));
        }
        println!(
            "{}  {}",
            self.paint(Style::Bold, &dir.path.display().to_string()),
            facts.join(" · ")
        );
        if !dir.exclude.is_empty() {
            println!(
                "  {}",
                self.paint(Style::Dim, &format!("excluded: {}", dir.exclude.join(", ")))
            );
        }
        if let Some(error) = &view.watch_error {
            println!(
                "  {}",
                self.paint(Style::Yellow, &format!("watching unavailable: {error}"))
            );
        }
        let width = view
            .projects
            .iter()
            .map(|p| p.rel.chars().count())
            .max()
            .unwrap_or(0)
            .max(8);
        for project in &view.projects {
            let (state, style) = match project.state {
                ParkedState::Registered => ("site", Style::Green),
                ParkedState::Manual => ("site (added by hand)", Style::Cyan),
                ParkedState::Suggested => ("suggested", Style::Yellow),
                ParkedState::Ignored => ("ignored", Style::Dim),
                ParkedState::Failed => ("not added", Style::Red),
            };
            let name = match project.state {
                ParkedState::Ignored | ParkedState::Failed => String::new(),
                _ => format!("{}.test", project.name),
            };
            let mut line = format!(
                "  {:<width$}  {:<28}  {:<18}  {}",
                project.rel,
                name,
                project.framework,
                self.paint(style, state)
            );
            if let Some(note) = &project.note {
                line.push_str(&self.paint(Style::Dim, &format!("  ({note})")));
            }
            println!("{line}");
            if project.state == ParkedState::Suggested {
                println!(
                    "  {:<width$}  {}",
                    "",
                    self.paint(
                        Style::Dim,
                        &format!("locdev add {}", project.path.display())
                    )
                );
            }
        }
        if view.projects.is_empty() && dir.enabled && view.exists {
            println!("  {}", self.paint(Style::Dim, "No projects found yet."));
        }
        for error in &view.errors {
            println!(
                "  {} {}: {}",
                self.paint(Style::Red, "!"),
                error.path.display(),
                error.message
            );
        }
    }
}
