//! Rendering for `locdev deploy platforms|projects|status|run`.

use locdev_core::remote::{
    RemoteDeployment, RemotePlatform, RemoteProject, RemoteState, RemoteStatus,
};

use super::{Output, Style, ago};

impl Output {
    pub fn remote_platforms(&self, platforms: &[RemotePlatform]) {
        if self.json {
            self.print_json(platforms);
            return;
        }
        for p in platforms {
            let state = match &p.account {
                Some(a) => self.paint(
                    Style::Green,
                    &format!(
                        "connected as {}{}",
                        a.user,
                        a.team
                            .as_ref()
                            .map(|t| format!(" (team {t})"))
                            .unwrap_or_default()
                    ),
                ),
                None => self.paint(
                    Style::Dim,
                    &format!("not connected · `locdev deploy connect {}`", p.id),
                ),
            };
            println!("{:<9} {state}", p.id);
            let c = &p.capabilities;
            let mut can = Vec::new();
            for (on, what) in [
                (c.deploy, "deploy"),
                (c.cancel, "cancel"),
                (c.rollback, "rollback"),
                (c.logs, "build logs"),
                (c.env, "variable names"),
                (c.domains, "domains"),
            ] {
                if on {
                    can.push(what);
                }
            }
            println!("          {}", self.paint(Style::Dim, &can.join(" · ")));
        }
    }

    pub fn remote_projects(&self, projects: &[RemoteProject]) {
        if self.json {
            self.print_json(projects);
            return;
        }
        for p in projects {
            println!(
                "{:<28} {:<22} {} {}",
                p.name,
                self.paint(Style::Dim, &p.id),
                p.repo.as_deref().unwrap_or("—"),
                p.branch
                    .as_deref()
                    .map(|b| format!("({b})"))
                    .unwrap_or_default()
            );
        }
        if projects.is_empty() {
            println!("No projects on this account.");
        }
    }

    pub fn remote_deployments(&self, deployments: &[RemoteDeployment]) {
        if self.json {
            self.print_json(deployments);
            return;
        }
        for d in deployments {
            let (label, style) = match d.state {
                RemoteState::Ready => ("ready", Style::Green),
                RemoteState::Building => ("building", Style::Yellow),
                RemoteState::Queued => ("queued", Style::Yellow),
                RemoteState::Error => ("error", Style::Red),
                RemoteState::Canceled => ("canceled", Style::Dim),
                RemoteState::Unknown => ("?", Style::Dim),
            };
            let mut tags = Vec::new();
            if d.current {
                tags.push("● live".to_string());
            } else if d.production {
                tags.push("production".to_string());
            }
            if let Some(at) = d.created_at {
                tags.push(format!("{} ago", ago(at)));
            }
            println!(
                "{:<30} {:<9} {} {}",
                d.id,
                self.paint(style, label),
                d.branch.as_deref().unwrap_or(""),
                self.paint(Style::Dim, &tags.join(" · "))
            );
            if let Some(m) = &d.message {
                println!("  {m}");
            }
            if let Some(e) = &d.error {
                println!("  {} {e}", self.paint(Style::Red, "!"));
            }
            println!(
                "  {}",
                self.paint(Style::Dim, d.url.as_deref().unwrap_or(&d.dashboard))
            );
        }
    }

    pub fn remote_status(&self, s: &RemoteStatus) {
        if self.json {
            self.print_json(s);
            return;
        }
        println!(
            "{} on {}  {}",
            self.paint(Style::Bold, &s.project.name),
            s.platform.name,
            self.paint(Style::Dim, &s.project.dashboard)
        );
        if let Some(repo) = &s.project.repo {
            println!(
                "  repository {repo}{}",
                s.project
                    .branch
                    .as_deref()
                    .map(|b| format!(", production branch {b}"))
                    .unwrap_or_default()
            );
        }
        if !s.domains.is_empty() {
            println!("  domains    {}", s.domains.join(", "));
        }
        if !s.env.is_empty() {
            let names: Vec<&str> = s.env.iter().map(|e| e.key.as_str()).collect();
            println!("  variables  {} (names only)", names.join(", "));
        }
        println!(
            "  from here  {} at {}",
            s.git.branch.as_deref().unwrap_or("?"),
            s.git
                .commit
                .as_deref()
                .map(|c| &c[..c.len().min(10)])
                .unwrap_or("?")
        );
        for p in &s.git.problems {
            println!("  {} {p}", self.paint(Style::Yellow, "!"));
        }
        println!();
        self.remote_deployments(&s.deployments);
        for note in &s.notes {
            println!("{}", self.paint(Style::Dim, note));
        }
    }
}
