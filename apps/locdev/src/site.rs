//! Site commands that work from inside a project folder: `info`, `open`,
//! `secure`, `dev …` and `ps`. Without a site name they use the registered
//! project that contains the current directory.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, bail};
use locdev_core::TLD;
use locdev_core::model::{CheckLevel, DevServerMode, ProcessRole, ProjectView, Routing};
use locdev_core::paths::Paths;
use locdev_core::protocol::{Request, Response};
use locdev_platform::{resolver, trust};

use crate::output::Output;
use crate::{DevCommand, daemon_call, dns, expect_project};

/// Common name of the LocDev CA (see `locdev_tls::CA_NAME`).
const CA_NAME: &str = "LocDev Local CA";

/// The site named, or the one whose folder contains the current directory
/// (the innermost, when projects are nested).
pub async fn resolve(paths: &Paths, name: Option<String>) -> anyhow::Result<ProjectView> {
    let Response::Projects { projects } = daemon_call(paths, Request::ListProjects).await? else {
        bail!("unexpected response from daemon");
    };
    if let Some(name) = name {
        return projects
            .into_iter()
            .find(|p| p.project.name == name || p.project.domain == name)
            .with_context(|| format!("no site named '{name}' (see `locdev list`)"));
    }
    let cwd = std::env::current_dir()?;
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    projects
        .into_iter()
        .filter(|p| contains(&p.project.path, &cwd))
        .max_by_key(|p| p.project.path.components().count())
        .with_context(|| {
            format!(
                "{} is not inside a registered site; name one, or register this folder with `locdev add`",
                cwd.display()
            )
        })
}

fn contains(root: &Path, path: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(root)
}

pub async fn info(paths: &Paths, name: Option<String>, out: &Output) -> anyhow::Result<ExitCode> {
    out.project_info(&resolve(paths, name).await?);
    Ok(ExitCode::SUCCESS)
}

pub async fn open(
    paths: &Paths,
    name: Option<String>,
    https: bool,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let site = resolve(paths, name).await?;
    let url = if https {
        site.https_url.clone()
    } else {
        site.url.clone()
    }
    .context("the site isn't routed (it has no port, or the proxy is off)")?;
    if site.process.state != locdev_core::model::ProcessState::Running {
        out.note(&format!(
            "{} is not running; start it with `locdev up {}`",
            site.project.domain, site.project.name
        ));
    }
    open_url(&url)?;
    out.message(&url);
    Ok(ExitCode::SUCCESS)
}

/// Hands a LocDev URL to the desktop's browser.
pub(crate) fn open_url(url: &str) -> anyhow::Result<()> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        bail!("not a web address: {url}");
    }
    let mut command = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    let status = command
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("could not start the browser")?;
    if !status.success() {
        bail!("the browser could not be opened; visit {url}");
    }
    Ok(())
}

/// `locdev secure`: checks what HTTPS needs for the site, fixes what needs
/// administrator rights only after showing the plan and asking, then makes
/// the site HTTPS-only. Built on the same CA and resolver as `locdev tls`
/// and `locdev dns`; nothing is installed twice.
pub async fn secure(
    paths: &Paths,
    name: Option<String>,
    off: bool,
    yes: bool,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let site = resolve(paths, name).await?;
    let name = site.project.name.clone();

    if off {
        let request = Request::SetSecure {
            name,
            secure: false,
        };
        let site = expect_project(daemon_call(paths, request).await?)?;
        out.message(&format!(
            "{} is served over HTTP and HTTPS again",
            site.project.domain
        ));
        return Ok(ExitCode::SUCCESS);
    }

    let checks = https_checks(paths, &name).await?;
    out.checks(&checks);
    let status = match daemon_call(paths, Request::Status).await? {
        Response::Status(status) => status,
        other => bail!("unexpected response from daemon: {other:?}"),
    };

    // Problems only the user (or LocDev's dev setup) can fix stop here.
    if let Some(blocking) = checks.iter().find(|c| {
        c.level == CheckLevel::Error
            && matches!(c.id.as_str(), "https-server" | "certificate" | "port")
    }) {
        bail!("{}: {}", blocking.label, blocking.detail);
    }

    let mut changed = false;
    if !status.tls.trusted || status.tls.browsers == Some(false) {
        let ca = status
            .tls
            .ca_path
            .clone()
            .context("the local CA is not available; see `locdev doctor`")?;
        out.note("The LocDev CA isn't trusted yet, so browsers would warn. Trusting it:");
        let plan = trust::install(&ca, &ca.with_file_name("ca.cer"), CA_NAME)
            .map_err(anyhow::Error::msg)?;
        dns::apply(&plan, yes, out)?;
        changed = true;
    }
    if status.dns.routing == Routing::None && resolver::installed(TLD) != Some(true) {
        match status.dns.listen() {
            Some(addr) => match resolver::install(TLD, addr) {
                Ok(plan) => {
                    out.note(&format!(
                        "*.{TLD} doesn't resolve yet. Routing it to LocDev:"
                    ));
                    dns::apply(&plan, yes, out)?;
                    changed = true;
                }
                Err(reason) => out.note(&format!(
                    "*.{TLD} can't be set up automatically here: {reason}"
                )),
            },
            None => out.note("the LocDev DNS server is off; see `locdev doctor`"),
        }
    }

    let site = expect_project(
        daemon_call(
            paths,
            Request::SetSecure {
                name: name.clone(),
                secure: true,
            },
        )
        .await?,
    )?;
    if changed {
        out.checks(&https_checks(paths, &name).await?);
    }
    let url = site.https_url.clone().context("HTTPS isn't being served")?;
    out.message(&format!("{} is HTTPS-only: {url}", site.project.domain));
    if site.process.state != locdev_core::model::ProcessState::Running {
        out.note(&format!("start it with `locdev up {name}`"));
    }
    Ok(ExitCode::SUCCESS)
}

async fn https_checks(paths: &Paths, name: &str) -> anyhow::Result<Vec<locdev_core::model::Check>> {
    match daemon_call(
        paths,
        Request::CheckSiteHttps {
            name: name.to_string(),
        },
    )
    .await?
    {
        Response::Doctor { checks } => Ok(checks),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
}

pub async fn dev(command: DevCommand, paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match command {
        DevCommand::Status { site } => {
            let site = resolve(paths, site).await?;
            out.dev_server(&site);
        }
        DevCommand::Start { site } => {
            let name = resolve(paths, site).await?.project.name;
            out.dev_server(&expect_project(
                daemon_call(paths, Request::StartDevServer { name }).await?,
            )?);
        }
        DevCommand::Stop { site } => {
            let name = resolve(paths, site).await?.project.name;
            out.dev_server(&expect_project(
                daemon_call(paths, Request::StopDevServer { name }).await?,
            )?);
        }
        DevCommand::Restart { site } => {
            let name = resolve(paths, site).await?.project.name;
            out.dev_server(&expect_project(
                daemon_call(paths, Request::RestartDevServer { name }).await?,
            )?);
        }
        DevCommand::Mode {
            mode,
            site,
            command,
            start,
        } => {
            let name = resolve(paths, site).await?.project.name;
            let request = Request::SetDevServer {
                name,
                mode: match mode.as_str() {
                    "ask" => DevServerMode::Ask,
                    "manual" => DevServerMode::Manual,
                    "auto" => DevServerMode::Auto,
                    "never" => DevServerMode::Never,
                    other => bail!("unknown mode '{other}' (ask, manual, auto or never)"),
                },
                command,
                start,
            };
            out.dev_server(&expect_project(daemon_call(paths, request).await?)?);
        }
        DevCommand::Logs {
            site,
            lines,
            follow,
        } => {
            let name = resolve(paths, site).await?.project.name;
            return logs(paths, name, ProcessRole::Dev, None, lines, follow, out).await;
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints a process's output, then keeps polling with `follow`.
pub async fn logs(
    paths: &Paths,
    name: String,
    process: ProcessRole,
    aux: Option<String>,
    lines: usize,
    follow: bool,
    out: &Output,
) -> anyhow::Result<ExitCode> {
    let mut cursor = None;
    let mut limit = lines;
    loop {
        let request = Request::ProjectLogs {
            name: name.clone(),
            after: cursor,
            limit: Some(limit),
            process,
            aux: aux.clone(),
        };
        let Response::Logs { lines } = daemon_call(paths, request).await? else {
            bail!("unexpected response from daemon");
        };
        if let Some(last) = lines.last() {
            cursor = Some(last.seq);
        }
        out.logs(&lines);

        if !follow {
            return Ok(ExitCode::SUCCESS);
        }
        limit = 1000;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

pub async fn ps(paths: &Paths, out: &Output) -> anyhow::Result<ExitCode> {
    match daemon_call(paths, Request::ListProcesses).await? {
        Response::Processes { processes } => out.processes(&processes),
        other => bail!("unexpected response from daemon: {other:?}"),
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_contain_their_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("src/app");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(contains(dir.path(), &inner.canonicalize().unwrap()));
        assert!(!contains(&inner, &dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn opens_only_web_addresses() {
        assert!(open_url("file:///etc/passwd").is_err());
        assert!(open_url("javascript:alert(1)").is_err());
    }
}
