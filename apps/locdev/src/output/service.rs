//! Rendering for `locdev service …`.

use locdev_core::model::ProcessState;
use locdev_core::services::{
    InstallSource, InstanceBackup, InstanceView, ServiceInfo, ServiceRelease,
};
use serde_json::json;

use super::{Output, Style, ago, state_label};

impl Output {
    pub fn services(&self, services: &[ServiceInfo], instances: &[InstanceView]) {
        if self.json {
            self.print_json(json!({ "services": services, "instances": instances }));
            return;
        }
        for s in services {
            let installs: Vec<String> = s
                .installs
                .iter()
                .map(|i| match i.source {
                    InstallSource::Managed => format!("{} (LocDev)", i.version),
                    InstallSource::System => format!("{} (system)", i.version),
                })
                .collect();
            println!(
                "{:<11} {}",
                self.paint(Style::Bold, &s.name),
                if installs.is_empty() {
                    self.paint(Style::Dim, "not installed")
                } else {
                    installs.join(", ")
                }
            );
            if let Some(why) = &s.no_download {
                println!("            {}", self.paint(Style::Dim, why));
            }
        }
        println!();
        self.instances(instances);
    }

    pub fn instances(&self, instances: &[InstanceView]) {
        if self.json {
            self.print_json(instances);
            return;
        }
        if instances.is_empty() {
            println!(
                "No instances yet. Create one: locdev service create db --service postgres --start"
            );
            return;
        }
        for v in instances {
            let i = &v.instance;
            let state = match (v.info.state, v.ready) {
                (ProcessState::Running, true) => self.paint(Style::Green, "ready   "),
                (ProcessState::Running, false) => self.paint(Style::Yellow, "starting"),
                (ProcessState::Failed, _) => self.paint(Style::Red, "failed  "),
                (other, _) => self.paint(Style::Dim, &format!("{:<8}", state_label(other))),
            };
            println!(
                "{:<14} {state} {:<10} {:<9} 127.0.0.1:{:<6} {}",
                i.name,
                i.kind.label(),
                i.version,
                i.port,
                if i.autostart {
                    self.paint(Style::Dim, "starts with LocDev")
                } else {
                    String::new()
                }
            );
        }
    }

    pub fn instance(&self, v: &InstanceView) {
        if self.json {
            self.print_json(v);
            return;
        }
        let i = &v.instance;
        println!(
            "{}  {} {} on 127.0.0.1:{}",
            self.paint(Style::Bold, &i.name),
            i.kind.label(),
            i.version,
            i.port
        );
        println!("data     {}", v.data_dir.display());
        println!(
            "state    {}{}",
            state_label(v.info.state),
            if v.ready {
                ", accepting connections"
            } else {
                ""
            }
        );
        if let Some(message) = &v.info.message {
            println!("{}", self.paint(Style::Red, message));
        }
        for (k, val) in &v.connection {
            println!("  {k}={val}");
        }
        println!(
            "{}",
            self.paint(
                Style::Dim,
                "Local only: listens on 127.0.0.1, users have no password."
            )
        );
    }

    pub fn service_releases(&self, releases: &[ServiceRelease]) {
        if self.json {
            self.print_json(releases);
            return;
        }
        for r in releases {
            println!(
                "{:<10} {:<10} {}",
                r.kind.id(),
                r.version,
                self.paint(Style::Dim, &r.publisher)
            );
        }
    }

    pub fn instance_backups(&self, backups: &[InstanceBackup]) {
        if self.json {
            self.print_json(backups);
            return;
        }
        if backups.is_empty() {
            println!("No backups yet.");
        }
        for b in backups {
            println!(
                "{:<15} {:>10} B  {} ago  {}",
                b.id,
                b.size,
                ago(b.at),
                self.paint(Style::Dim, &b.path.display().to_string())
            );
        }
    }
}
