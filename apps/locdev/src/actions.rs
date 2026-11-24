//! `locdev actions …`: GitHub Actions workflows for a site.

use std::process::ExitCode;

use anyhow::bail;
use clap::Subcommand;
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};

use crate::output::Output;
use crate::work::confirm;
use crate::{daemon_call, site};

#[derive(Subcommand)]
pub(crate) enum ActionsCommand {
    /// Templates that fit the site, and its workflow files.
    List { site: Option<String> },
    /// A workflow from a template, printed (or saved with --save).
    New {
        template: String,
        site: Option<String>,
        /// File name under .github/workflows/ (the template's by default).
        #[arg(long)]
        file: Option<String>,
        /// Write it, after showing the file and the checks. Never replaces a file.
        #[arg(long)]
        save: bool,
        /// Do not ask before saving.
        #[arg(short, long)]
        yes: bool,
    },
    /// Check the site's workflow files (or one of them, or any YAML file).
    Validate {
        site: Option<String>,
        /// Only this file (e.g. ci.yml).
        #[arg(long)]
        file: Option<String>,
        /// Check a YAML file from disk instead (`-` for stdin); no site needed.
        #[arg(long, value_name = "PATH", conflicts_with = "file")]
        yaml: Option<String>,
    },
    /// Write a workflow you edited as .github/workflows/<file>: the diff and
    /// the checks first; an existing file is replaced only as it was reviewed.
    Save {
        /// File name under .github/workflows/ (e.g. ci.yml).
        file: String,
        /// The YAML to write (`-` for stdin).
        #[arg(long)]
        from: String,
        #[arg(long)]
        site: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    /// The action versions templates use; --refresh asks GitHub.
    Versions {
        #[arg(long)]
        refresh: bool,
    },
    /// Local versions LocDev kept of a workflow file; --show prints one.
    History {
        /// File name under .github/workflows/ (e.g. ci.yml).
        file: String,
        site: Option<String>,
        /// A version id from the list.
        #[arg(long)]
        show: Option<String>,
    },
    /// Download actionlint (verified) so validation uses it.
    InstallActionlint,
}

pub async fn run(command: ActionsCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    let name = |site| async { anyhow::Ok(site::resolve(paths, site).await?.project.name) };
    match command {
        ActionsCommand::List { site } => {
            let name = name(site).await?;
            match daemon_call(paths, Request::ActionsOverview { name }).await? {
                Response::ActionsOverview {
                    templates,
                    existing,
                    actionlint,
                    ..
                } => out.actions_overview(&templates, &existing, actionlint.as_deref()),
                other => bail!("unexpected response from daemon: {other:?}"),
            }
        }
        ActionsCommand::Versions { refresh } => {
            let request = if refresh {
                Request::RefreshActionVersions
            } else {
                Request::ActionVersions
            };
            let Response::ActionVersions {
                actions,
                checked_at,
            } = daemon_call(paths, request).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.action_versions(&actions, checked_at);
        }
        ActionsCommand::Validate {
            yaml: Some(source), ..
        } => {
            let yaml = read_input(&source)?;
            let Response::ActionsValidation(validation) =
                daemon_call(paths, Request::ActionsValidate { yaml }).await?
            else {
                bail!("unexpected response from daemon");
            };
            out.validation(&source, &validation);
            if validation.has_errors() {
                return Ok(ExitCode::FAILURE);
            }
        }
        ActionsCommand::Save {
            file,
            from,
            site,
            yes,
        } => {
            let name = name(site).await?;
            let yaml = read_input(&from)?;
            let Response::ActionsPlan(plan) = daemon_call(
                paths,
                Request::ActionsPlan {
                    name: name.clone(),
                    file: file.clone(),
                    yaml: yaml.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.text(&plan.diff, false);
            out.validation(&file, &plan.validation);
            if plan.validation.has_errors() {
                bail!("fix the errors above first; nothing was written");
            }
            if !yes {
                let verb = if plan.exists { "Replace" } else { "Create" };
                confirm(&format!("{verb} {}?", plan.path))?;
            }
            let Response::ActionsSaved { path } = daemon_call(
                paths,
                Request::ActionsSave {
                    name,
                    file,
                    yaml,
                    replace: plan.current_hash,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            out.message(&format!("saved {path}"));
        }
        ActionsCommand::Validate {
            site, file: None, ..
        } => {
            let name = name(site).await?;
            let Response::ActionsAudit { files, actionlint } =
                daemon_call(paths, Request::ActionsAudit { name }).await?
            else {
                bail!("unexpected response from daemon");
            };
            if files.is_empty() {
                out.note("no workflow files in .github/workflows/");
            }
            if actionlint.is_none() {
                out.note("actionlint isn't installed: `locdev actions install-actionlint` adds its checks");
            }
            let mut failed = false;
            for audit in files {
                let validation = locdev_core::actions::Validation {
                    findings: audit.findings,
                    ..Default::default()
                };
                failed |= validation.has_errors();
                out.validation(&audit.file, &validation);
            }
            if failed {
                return Ok(ExitCode::FAILURE);
            }
        }
        ActionsCommand::History { file, site, show } => {
            let name = name(site).await?;
            match show {
                Some(id) => {
                    let Response::ActionsYaml { yaml, validation } = daemon_call(
                        paths,
                        Request::ActionsHistoryVersion {
                            name,
                            file: file.clone(),
                            id,
                        },
                    )
                    .await?
                    else {
                        bail!("unexpected response from daemon");
                    };
                    out.workflow(&yaml, &file, &validation);
                }
                None => {
                    let Response::ActionsHistory { versions } =
                        daemon_call(paths, Request::ActionsHistory { name, file }).await?
                    else {
                        bail!("unexpected response from daemon");
                    };
                    if out.is_json() {
                        out.print_json(&versions);
                    } else if versions.is_empty() {
                        println!("no versions kept yet");
                    }
                    for v in versions.iter().filter(|_| !out.is_json()) {
                        println!(
                            "{}  {}  {} bytes",
                            v.id,
                            locdev_core::model::civil_date(v.at),
                            v.size
                        );
                    }
                }
            }
        }
        ActionsCommand::InstallActionlint => {
            daemon_call(paths, Request::ActionlintInstall).await?;
            out.message("actionlint installed; validation uses it now");
        }
        ActionsCommand::Validate { site, file, .. } => {
            let name = name(site).await?;
            let files: Vec<String> = file.into_iter().collect();
            if files.is_empty() {
                out.note("no workflow files in .github/workflows/");
            }
            let mut failed = false;
            for file in files {
                let Response::ActionsYaml { validation, .. } = daemon_call(
                    paths,
                    Request::ActionsRead {
                        name: name.clone(),
                        file: file.clone(),
                    },
                )
                .await?
                else {
                    bail!("unexpected response from daemon");
                };
                failed |= validation.has_errors();
                out.validation(&file, &validation);
            }
            if failed {
                return Ok(ExitCode::FAILURE);
            }
        }
        ActionsCommand::New {
            template,
            site,
            file,
            save,
            yes,
        } => {
            let name = name(site).await?;
            let Response::ActionsWorkflow {
                workflow,
                file: suggested,
            } = daemon_call(
                paths,
                Request::ActionsTemplate {
                    name: name.clone(),
                    template,
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            let Response::ActionsYaml { yaml, validation } =
                daemon_call(paths, Request::ActionsRender { workflow }).await?
            else {
                bail!("unexpected response from daemon");
            };
            let file = file.unwrap_or(suggested);
            if !save {
                out.workflow(&yaml, &file, &validation);
                return Ok(ExitCode::SUCCESS);
            }
            let Response::ActionsPlan(plan) = daemon_call(
                paths,
                Request::ActionsPlan {
                    name: name.clone(),
                    file: file.clone(),
                    yaml: yaml.clone(),
                },
            )
            .await?
            else {
                bail!("unexpected response from daemon");
            };
            if plan.exists {
                bail!(
                    "{} already exists; pick another name with --file, or edit it in the app (Tools → Actions builder), which shows the diff",
                    plan.path
                );
            }
            out.workflow(&yaml, &file, &plan.validation);
            if !yes {
                confirm(&format!("Create {}?", plan.path))?;
            }
            if let Response::ActionsSaved { path } = daemon_call(
                paths,
                Request::ActionsSave {
                    name,
                    file,
                    yaml,
                    replace: None,
                },
            )
            .await?
            {
                out.message(&format!("created {path}"));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn read_input(source: &str) -> anyhow::Result<String> {
    if source == "-" {
        let mut text = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(source).map_err(|e| anyhow::anyhow!("could not read {source}: {e}"))
    }
}
