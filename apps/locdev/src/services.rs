//! `locdev service …`: databases and caches LocDev installs and runs.

use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{ProcessAction, Request, Response};
use locdev_core::services::{InstanceView, ServiceKind};
use locdev_ipc::launcher;

use crate::daemon_call;
use crate::output::Output;
use crate::work::confirm;

#[derive(Subcommand)]
pub(crate) enum ServiceCmd {
    /// Services, their installations (system or LocDev) and the instances.
    #[command(visible_alias = "ls")]
    List {
        /// Look for system installations again.
        #[arg(long)]
        refresh: bool,
    },
    /// Instances and whether they accept connections.
    Status,
    /// Versions LocDev can download for a service.
    Available {
        service: ServiceKind,
    },
    /// Versions the system's package manager offers (Homebrew, apt, winget).
    Packages {
        service: ServiceKind,
    },
    /// Install one of `locdev service packages` in this terminal (asks first).
    InstallPackage {
        service: ServiceKind,
        /// The package id from `locdev service packages`.
        id: String,
        #[arg(short, long)]
        yes: bool,
    },
    /// Download and verify a version (newest when omitted). Without a
    /// service, registers the daemon to start at login (old meaning; see `locdev startup`).
    Install {
        service: Option<ServiceKind>,
        version: Option<String>,
    },
    /// Remove a LocDev-downloaded version (never a system one). Without a
    /// service, stops starting the daemon at login (old meaning).
    Uninstall {
        service: Option<ServiceKind>,
        version: Option<String>,
    },
    /// A new instance: its own data folder, port, config and logs.
    Create {
        name: String,
        /// postgres, mariadb, mysql, mongodb, valkey or redis.
        #[arg(long = "service")]
        kind: ServiceKind,
        /// The newest installed one when omitted ("17", "17.4").
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        /// Start it whenever LocDev starts.
        #[arg(long)]
        autostart: bool,
        /// Start it now.
        #[arg(long)]
        start: bool,
    },
    /// Start an instance (or the only instance of a service).
    Start {
        instance: String,
    },
    Stop {
        instance: String,
    },
    Restart {
        instance: String,
    },
    /// Start it with LocDev, or not.
    Autostart {
        instance: String,
        enabled: bool,
    },
    /// Remove an instance. Its data stays unless --delete-data (asks for the name).
    #[command(visible_alias = "rm")]
    Remove {
        instance: String,
        #[arg(long)]
        delete_data: bool,
    },
    /// A cold copy of the instance's data (it stops and starts again).
    Backup {
        instance: String,
    },
    Backups {
        instance: String,
    },
    /// Put a backup back; the current data is moved aside, not deleted.
    Restore {
        instance: String,
        id: String,
        #[arg(short, long)]
        yes: bool,
    },
    /// The instance's output.
    Logs {
        instance: String,
        #[arg(short = 'n', long, default_value_t = 100)]
        lines: usize,
    },
}

/// An instance by name, or the single instance of a service ("postgres").
async fn instance(paths: &Paths, wanted: &str) -> anyhow::Result<String> {
    let Response::Services { instances, .. } =
        daemon_call(paths, Request::ListServices { refresh: false }).await?
    else {
        bail!("unexpected response from daemon");
    };
    if instances.iter().any(|i| i.instance.name == wanted) {
        return Ok(wanted.to_string());
    }
    if let Ok(kind) = wanted.parse::<ServiceKind>() {
        let of_kind: Vec<&InstanceView> = instances
            .iter()
            .filter(|i| i.instance.kind == kind)
            .collect();
        match of_kind.as_slice() {
            [one] => return Ok(one.instance.name.clone()),
            [] => bail!(
                "no {} instance yet; create one: locdev service create {} --service {}",
                kind.label(),
                kind.id(),
                kind.id()
            ),
            many => bail!(
                "several {} instances ({}); name one",
                kind.label(),
                many.iter()
                    .map(|i| i.instance.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
    bail!("no service instance named {wanted} (see `locdev service status`)")
}

fn login_item(install: bool, out: &Output) -> anyhow::Result<ExitCode> {
    use locdev_platform::service;
    out.note("note: `locdev service install|uninstall` without a service now lives at `locdev startup install|uninstall`");
    if install {
        let daemon = launcher::daemon_executable().context("could not find locdevd to register")?;
        let home = std::env::var_os(locdev_core::paths::HOME_ENV).map(std::path::PathBuf::from);
        service::install(&daemon, home.as_deref()).context("could not register the login item")?;
    } else {
        service::uninstall().context("could not remove the login item")?;
    }
    out.service(&service::status());
    Ok(ExitCode::SUCCESS)
}

pub async fn run(command: ServiceCmd, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let control = |name, action| Request::ControlInstance { name, action };
    match command {
        ServiceCmd::List { refresh } => {
            let Response::Services {
                services,
                instances,
            } = daemon_call(paths, Request::ListServices { refresh }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.services(&services, &instances);
        }
        ServiceCmd::Status => {
            let Response::Services { instances, .. } =
                daemon_call(paths, Request::ListServices { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.instances(&instances);
            if !out.is_json() {
                out.service(&locdev_platform::service::status());
            }
        }
        ServiceCmd::Available { service } => {
            let Response::ServiceReleases { releases } =
                daemon_call(paths, Request::ServiceReleases { kind: service }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.service_releases(&releases);
        }
        ServiceCmd::Packages { service } => {
            let Response::Services { services, .. } =
                daemon_call(paths, Request::ListServices { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let packages: Vec<_> = services
                .into_iter()
                .filter(|s| s.kind == service)
                .flat_map(|s| s.packages)
                .collect();
            if out.is_json() {
                out.print_json(&packages);
            } else if packages.is_empty() {
                println!("no package manager here offers {}", service.label());
            }
            for p in packages.iter().filter(|_| !out.is_json()) {
                let commands: Vec<String> = p.commands.iter().map(|c| c.join(" ")).collect();
                println!(
                    "{:<22} {:<28} {}",
                    p.id,
                    format!("{} ({})", p.version, p.manager),
                    commands.join(" && ")
                );
            }
        }
        ServiceCmd::InstallPackage { service, id, yes } => {
            let Response::Services { services, .. } =
                daemon_call(paths, Request::ListServices { refresh: false }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let package = services
                .into_iter()
                .filter(|s| s.kind == service)
                .flat_map(|s| s.packages)
                .find(|p| p.id == id)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no package {id}; see `locdev service packages {}`",
                        service.id()
                    )
                })?;
            for c in &package.commands {
                println!("$ {}", c.join(" "));
            }
            if !yes {
                crate::work::confirm("Run this as your user?")?;
            }
            let Response::Terminal(info) =
                daemon_call(paths, Request::InstallServicePackage { kind: service, id }).await?
            else {
                bail!("unexpected response from daemon");
            };
            return crate::scaffold::follow(paths, info.id).await;
        }
        ServiceCmd::Install { service: None, .. } => return login_item(true, out),
        ServiceCmd::Uninstall { service: None, .. } => return login_item(false, out),
        ServiceCmd::Install {
            service: Some(kind),
            version,
        } => {
            let version = match version {
                Some(v) => v,
                None => {
                    let Response::ServiceReleases { releases } =
                        daemon_call(paths, Request::ServiceReleases { kind }).await?
                    else {
                        bail!("unexpected response from daemon");
                    };
                    releases
                        .first()
                        .map(|r| r.version.clone())
                        .context("no versions offered")?
                }
            };
            let Response::Job { job } = daemon_call(
                paths,
                Request::InstallService {
                    kind,
                    version: version.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.message(&format!(
                "downloading {} {version} (job {}); follow it with `locdev jobs show {}`",
                kind.label(),
                job.id,
                job.id
            ));
        }
        ServiceCmd::Uninstall {
            service: Some(kind),
            version,
        } => {
            let version = version.context("which version? see `locdev service list`")?;
            daemon_call(
                paths,
                Request::UninstallService {
                    kind,
                    version: version.clone(),
                },
            )
            .await?;
            out.message(&format!("removed {} {version}", kind.label()));
        }
        ServiceCmd::Create {
            name,
            kind,
            version,
            port,
            autostart,
            start,
        } => {
            let Response::Instance(view) = daemon_call(
                paths,
                Request::CreateInstance {
                    name: name.clone(),
                    kind,
                    version,
                    port,
                    autostart,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            if start && !autostart {
                daemon_call(paths, control(name, ProcessAction::Start)).await?;
            }
            out.instance(&view);
        }
        ServiceCmd::Start { instance: wanted } => {
            let name = instance(paths, &wanted).await?;
            let Response::Instance(view) =
                daemon_call(paths, control(name, ProcessAction::Start)).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.instance(&view);
        }
        ServiceCmd::Stop { instance: wanted } => {
            let name = instance(paths, &wanted).await?;
            daemon_call(paths, control(name.clone(), ProcessAction::Stop)).await?;
            out.message(&format!("stopped {name}"));
        }
        ServiceCmd::Restart { instance: wanted } => {
            let name = instance(paths, &wanted).await?;
            let Response::Instance(view) =
                daemon_call(paths, control(name, ProcessAction::Restart)).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.instance(&view);
        }
        ServiceCmd::Autostart {
            instance: wanted,
            enabled,
        } => {
            let name = instance(paths, &wanted).await?;
            daemon_call(
                paths,
                Request::SetInstance {
                    name: name.clone(),
                    autostart: enabled,
                },
            )
            .await?;
            out.message(&format!(
                "{name} {} with LocDev",
                if enabled {
                    "starts"
                } else {
                    "no longer starts"
                }
            ));
        }
        ServiceCmd::Remove {
            instance: wanted,
            delete_data,
        } => {
            let name = instance(paths, &wanted).await?;
            let confirm = if delete_data {
                if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    bail!("deleting data needs the instance's name typed on a terminal");
                }
                eprint!(
                    "This deletes {name}'s data for good (backups are kept). Type the instance's name: "
                );
                Some(crate::vcs::read_secret()?)
            } else {
                None
            };
            daemon_call(
                paths,
                Request::RemoveInstance {
                    name: name.clone(),
                    delete_data,
                    confirm,
                },
            )
            .await?;
            out.message(&if delete_data {
                format!("removed {name} and its data")
            } else {
                format!("removed {name}; its data is kept in LocDev's services folder")
            });
        }
        ServiceCmd::Backup { instance: wanted } => {
            let name = instance(paths, &wanted).await?;
            let Response::InstanceBackup(backup) =
                daemon_call(paths, Request::BackupInstance { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.instance_backups(&[backup]);
        }
        ServiceCmd::Backups { instance: wanted } => {
            let name = instance(paths, &wanted).await?;
            let Response::InstanceBackups { backups } =
                daemon_call(paths, Request::InstanceBackups { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.instance_backups(&backups);
        }
        ServiceCmd::Restore {
            instance: wanted,
            id,
            yes,
        } => {
            let name = instance(paths, &wanted).await?;
            if !yes {
                confirm(&format!(
                    "Replace {name}'s data with backup {id}? The current data is moved aside."
                ))?;
            }
            daemon_call(
                paths,
                Request::RestoreInstance {
                    name: name.clone(),
                    id,
                    confirm: Some(name.clone()),
                },
            )
            .await?;
            out.message(&format!("restored {name}"));
        }
        ServiceCmd::Logs {
            instance: wanted,
            lines,
        } => {
            let name = instance(paths, &wanted).await?;
            let Response::Logs { lines } = daemon_call(
                paths,
                Request::InstanceLogs {
                    name,
                    after: None,
                    limit: Some(lines),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.logs(&lines);
        }
    }
    Ok(ExitCode::SUCCESS)
}
