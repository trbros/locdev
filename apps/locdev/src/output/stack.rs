//! Rendering for `locdev stack …`.

use locdev_core::frameworks::{DefinitionInfo, DefinitionsOrigin, RegistryStatus, StackReport};
use locdev_core::model::{ComponentCategory, Integration};
use serde_json::json;

use super::{Output, Style, ago};

fn integration_label(i: Integration) -> &'static str {
    match i {
        Integration::Native => "native",
        Integration::Script => "script",
        Integration::Custom => "custom",
        Integration::None => "none",
    }
}

fn category_label(c: ComponentCategory) -> &'static str {
    match c {
        ComponentCategory::Framework => "framework",
        ComponentCategory::Ui => "ui",
        ComponentCategory::Language => "language",
        ComponentCategory::Runtime => "runtime",
        ComponentCategory::Tool => "tool",
    }
}

impl Output {
    pub fn stack_list(&self, definitions: &[DefinitionInfo], registry: &RegistryStatus) {
        if self.json {
            self.print_json(json!({ "definitions": definitions, "registry": registry }));
            return;
        }
        for d in definitions {
            let tone = match d.integration {
                Integration::Native => Style::Green,
                Integration::Script => Style::Cyan,
                _ => Style::Dim,
            };
            println!(
                "{:<13} {:<22} {:<10} {}  {}",
                d.id,
                d.name,
                category_label(d.category),
                self.paint(
                    tone,
                    &format!(
                        "{:<9}",
                        match d.integration {
                            Integration::None => "component",
                            other => integration_label(other),
                        }
                    )
                ),
                self.paint(Style::Dim, d.dev.first().map(String::as_str).unwrap_or(""))
            );
        }
        println!();
        self.definitions_status(registry);
    }

    pub fn definitions_status(&self, r: &RegistryStatus) {
        if self.json {
            self.print_json(r);
            return;
        }
        let origin = match r.origin {
            DefinitionsOrigin::Bundled => "bundled with LocDev",
            DefinitionsOrigin::Downloaded => "downloaded, signature verified",
        };
        println!(
            "definitions   revision {} ({origin}), {} frameworks, format {}",
            r.revision, r.count, r.format
        );
        println!("bundled       revision {}", r.bundled_revision);
        if let Some(previous) = r.previous {
            println!("previous      revision {previous} (locdev stack definitions --rollback)");
        }
        if let Some(skipped) = r.skipped {
            println!(
                "skipped       revision {skipped} (rolled back; --refresh --force takes it again)"
            );
        }
        println!(
            "last check    {}",
            r.checked_at
                .map(|at| format!("{} ago", ago(at)))
                .unwrap_or_else(|| "never".into())
        );
        println!("source        {}", self.paint(Style::Dim, &r.url));
        if let Some(error) = &r.error {
            println!(
                "{}",
                self.paint(Style::Yellow, &format!("last update failed: {error}"))
            );
        }
    }

    pub fn stack_report(&self, r: &StackReport, full: bool) {
        if self.json {
            self.print_json(r);
            return;
        }
        let s = &r.stack;
        println!(
            "{}  {}",
            self.paint(Style::Bold, &r.target),
            self.paint(
                Style::Dim,
                &format!(
                    "{} · {}{}",
                    s.framework,
                    integration_label(s.integration),
                    r.adapter_override
                        .as_ref()
                        .map(|a| format!(" · adapter chosen by hand: {a}"))
                        .unwrap_or_default()
                )
            )
        );
        for c in &s.components {
            let version = match (&c.version, &c.version_source) {
                (Some(v), Some(src)) => format!("{v} ({src})"),
                (Some(v), None) => v.clone(),
                _ => String::new(),
            };
            let mark = if c.primary {
                self.paint(Style::Green, "●")
            } else {
                " ".into()
            };
            let manual = if c.manual {
                self.paint(Style::Yellow, " [added]")
            } else {
                String::new()
            };
            println!(
                "  {mark} {:<22} {:<10} {:<28}{manual}",
                c.name,
                category_label(c.category),
                version
            );
            if full {
                println!("      {}", self.paint(Style::Dim, &c.evidence.join(", ")));
            }
        }
        if !r.hidden.is_empty() {
            println!(
                "  {}",
                self.paint(Style::Dim, &format!("hidden: {}", r.hidden.join(", ")))
            );
        }
        match (&r.command, &s.command) {
            (Some(current), Some(suggested)) if current != suggested => {
                println!("command       {current}");
                println!(
                    "              {}",
                    self.paint(
                        Style::Dim,
                        &format!(
                            "adapter suggests: {suggested} (locdev stack use {} --apply-command)",
                            s.adapter.as_deref().unwrap_or("auto")
                        )
                    )
                );
            }
            (Some(current), _) => println!("command       {current}"),
            (None, Some(suggested)) => println!("command       {suggested}"),
            (None, None) => println!(
                "command       {}",
                self.paint(
                    Style::Yellow,
                    "none; set one with `locdev edit <site> --command`"
                )
            ),
        }
        for var in &s.port_env {
            println!("port env      {var}");
        }
        if let Some(venv) = &s.venv {
            println!("virtualenv    {venv}");
        }
        if let Some(pm) = s.package_manager {
            println!("packages      {}", pm.executable());
        }
        for p in &r.prerequisites {
            let state = match &p.found {
                Some(path) => self.paint(Style::Green, &path.display().to_string()),
                None => self.paint(Style::Red, "not found on PATH"),
            };
            println!("requires      {:<10} {state}", p.program);
        }
        if full {
            for (label, value) in [
                ("install", &r.install),
                ("build", &r.build),
                ("test", &r.test),
            ] {
                if let Some(value) = value {
                    println!("{label:<13} {}", self.paint(Style::Dim, value));
                }
            }
            if !s.config_files.is_empty() {
                println!("config        {}", s.config_files.join(", "));
            }
            if let Some(def) = &r.definition {
                println!("docs          {}", def.docs);
                if !def.services.is_empty() {
                    println!(
                        "services      {} {}",
                        def.services.join(", "),
                        self.paint(Style::Dim, "(commonly used; not managed by LocDev yet)")
                    );
                }
            }
        }
        if !s.workspaces.is_empty() {
            println!("{}", self.paint(Style::Bold, "Workspace packages"));
            for m in &s.workspaces {
                println!("  {:<28} {}", m.path, m.framework);
            }
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    "Add each one as its own site: locdev add <path>"
                )
            );
        }
        for note in &s.notes {
            println!("{}", self.paint(Style::Dim, &format!("· {note}")));
        }
    }
}
