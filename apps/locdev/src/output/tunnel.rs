//! Rendering for `locdev tunnel …`.

use locdev_core::tunnel::{TunnelProvider, TunnelState, TunnelView};

use super::{Output, Style, uptime};

impl Output {
    pub fn tunnel_providers(&self, providers: &[TunnelProvider]) {
        if self.json {
            self.print_json(providers);
            return;
        }
        for p in providers {
            println!("{}  {}", self.paint(Style::Bold, &p.id), p.name);
            println!("  {}", p.description);
            match &p.program {
                Some(prog) => println!(
                    "  {} {} {}{}",
                    self.paint(Style::Green, "installed"),
                    prog.version,
                    self.paint(Style::Dim, &prog.path.display().to_string()),
                    prog.outdated
                        .as_ref()
                        .map(|o| format!("  ({o})"))
                        .unwrap_or_default()
                ),
                None if p.installable => println!(
                    "  {} `locdev tunnel install {}`",
                    self.paint(Style::Yellow, "not installed:"),
                    p.id
                ),
                None => println!(
                    "  {} {}",
                    self.paint(Style::Red, "not available:"),
                    p.install_note.as_deref().unwrap_or("")
                ),
            }
            for limit in &p.limits {
                println!("  · {limit}");
            }
            println!("  {}", self.paint(Style::Dim, &p.docs));
        }
    }

    pub fn tunnels(&self, tunnels: &[TunnelView]) {
        if self.json {
            self.print_json(tunnels);
            return;
        }
        if tunnels.is_empty() {
            println!("No site is shared. `locdev tunnel start <site>` shares one.");
        }
        for t in tunnels {
            let state = match t.state {
                TunnelState::Online => self.paint(Style::Green, "online"),
                TunnelState::Starting => self.paint(Style::Yellow, "starting"),
                TunnelState::Failed => self.paint(Style::Red, "failed"),
                TunnelState::Stopped => self.paint(Style::Dim, "stopped"),
            };
            let up = t
                .started_at
                .filter(|_| t.state == TunnelState::Online)
                .map(|at| format!(" · up {}", uptime(at)))
                .unwrap_or_default();
            println!("{:<16} {state}{up}  → {}", t.site, t.target);
            if let Some(url) = &t.url {
                println!("  {}", self.paint(Style::Bold, url));
            }
            if t.allowed.is_empty() {
                println!(
                    "  {}",
                    self.paint(Style::Yellow, "anyone with the address can open it")
                );
            } else {
                println!("  only {}", t.allowed.join(", "));
            }
            if let Some(error) = &t.error {
                println!("  {} {error}", self.paint(Style::Red, "!"));
            }
        }
    }
}
