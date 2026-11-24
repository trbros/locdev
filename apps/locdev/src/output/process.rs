//! Rendering for `locdev process …`.

use locdev_core::model::{
    AuxProcessView, DevServerMode, PhpExtensionsReport, ProcessKind, ProcessState, ProjectView,
};
use serde_json::json;

use super::{Output, Style, state_label};

fn mode_label(mode: DevServerMode) -> &'static str {
    match mode {
        DevServerMode::Ask => "ask",
        DevServerMode::Manual => "manual",
        DevServerMode::Auto => "auto",
        DevServerMode::Never => "never",
    }
}

fn kind_label(kind: ProcessKind) -> &'static str {
    match kind {
        ProcessKind::Worker => "worker",
        ProcessKind::Scheduler => "scheduler",
        ProcessKind::Websocket => "websocket",
        ProcessKind::Tool => "tool",
        ProcessKind::Other => "process",
    }
}

impl Output {
    pub fn php_extensions(&self, r: &PhpExtensionsReport) {
        if self.json {
            self.print_json(r);
            return;
        }
        println!(
            "php      {} {}",
            r.version.as_deref().unwrap_or("?"),
            self.paint(
                Style::Dim,
                &format!(
                    "({}, {})",
                    r.origin.as_deref().unwrap_or("PATH"),
                    r.executable
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                )
            )
        );
        if let Some(error) = &r.error {
            println!("{}", self.paint(Style::Red, error));
            return;
        }
        println!(
            "needs    {} extension(s), {} loaded in this PHP",
            r.required.len(),
            r.loaded.len()
        );
        if r.missing.is_empty() {
            println!("{}", self.paint(Style::Green, "nothing missing"));
        }
        for m in &r.missing {
            println!(
                "  {} {:<14} {}",
                self.paint(Style::Red, "missing"),
                m.name,
                self.paint(Style::Dim, &format!("required by {}", m.from))
            );
        }
        if let Some(remedy) = &r.remedy {
            println!("{remedy}");
        }
        if !r.recommended.is_empty() {
            println!("recommended by the project's packages");
        }
        for m in &r.recommended {
            let loaded = r.loaded.contains(&m.name);
            println!(
                "  {} {:<14} {}",
                if loaded {
                    self.paint(Style::Green, "loaded ")
                } else {
                    self.paint(Style::Yellow, "absent ")
                },
                m.name,
                self.paint(
                    Style::Dim,
                    &format!(
                        "{}{}",
                        m.from,
                        m.reason
                            .as_deref()
                            .map(|r| format!(": {r}"))
                            .unwrap_or_default()
                    )
                )
            );
        }
    }

    pub fn php_modules(&self, m: &locdev_core::model::PhpModules) {
        if self.json {
            self.print_json(m);
            return;
        }
        println!(
            "php {} {}",
            m.version,
            self.paint(
                Style::Dim,
                &format!("({}, {})", m.origin, m.executable.display())
            )
        );
        if let Some(ini) = &m.ini {
            println!("php.ini  {ini}");
        }
        for f in &m.scanned {
            println!("also     {f}");
        }
        println!("loaded ({})", m.extensions.len());
        for e in &m.extensions {
            println!(
                "  {:<24} {}{}",
                e.name,
                e.version.as_deref().unwrap_or(""),
                if e.zend { "  (zend)" } else { "" }
            );
        }
        if !m.available.is_empty() {
            println!(
                "in {} but not loaded: {}",
                m.extension_dir.as_deref().unwrap_or("the extension folder"),
                m.available.join(", ")
            );
        }
    }

    fn process_row(
        &self,
        id: &str,
        name: &str,
        kind: &str,
        state: ProcessState,
        mode: &str,
        command: &str,
    ) {
        let tone = match state {
            ProcessState::Running => Style::Green,
            ProcessState::Failed => Style::Red,
            ProcessState::Stopping => Style::Yellow,
            _ => Style::Dim,
        };
        println!(
            "{:<12} {:<18} {:<10} {} {:<7} {}",
            id,
            name,
            kind,
            self.paint(tone, &format!("{:<8}", state_label(state))),
            mode,
            self.paint(Style::Dim, command)
        );
    }

    /// The dev server, its helper and every auxiliary process.
    pub fn project_processes(&self, view: &ProjectView, aux: &[AuxProcessView]) {
        if self.json {
            self.print_json(
                json!({ "main": view.process, "dev": view.dev_process, "processes": aux }),
            );
            return;
        }
        self.process_row(
            "main",
            "Dev server",
            "server",
            view.process.state,
            if view.project.auto_start {
                "auto"
            } else {
                "manual"
            },
            view.project.command.as_deref().unwrap_or("-"),
        );
        if let (Some(dev), Some(info)) = (&view.project.dev_server, &view.dev_process) {
            self.process_row(
                "dev",
                dev.tool.label(),
                "helper",
                info.state,
                mode_label(dev.mode),
                &dev.command,
            );
        }
        self.aux_processes(aux);
    }

    pub fn aux_processes(&self, aux: &[AuxProcessView]) {
        if self.json {
            self.print_json(aux);
            return;
        }
        for p in aux {
            let name = if p.custom {
                format!("{} (yours)", p.name)
            } else {
                p.name.clone()
            };
            self.process_row(
                &p.id,
                &name,
                kind_label(p.kind),
                p.info.state,
                mode_label(p.mode),
                p.command.as_deref().unwrap_or("-"),
            );
        }
        let undecided: Vec<&str> = aux
            .iter()
            .filter(|p| p.mode == DevServerMode::Ask)
            .map(|p| p.id.as_str())
            .collect();
        if !undecided.is_empty() {
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    &format!(
                        "Offered, waiting for your decision: {} (locdev process mode <process> manual|auto|never)",
                        undecided.join(", ")
                    )
                )
            );
        }
        for p in aux
            .iter()
            .filter(|p| p.note.is_some() && p.mode != DevServerMode::Never)
        {
            println!(
                "{}",
                self.paint(
                    Style::Dim,
                    &format!("· {}: {}", p.id, p.note.as_deref().unwrap_or(""))
                )
            );
        }
    }
}
