//! Sharing a site on the internet (`locdev tunnel`, Tools → Tunnels). The
//! provider's own program runs under the supervisor as `@tunnel:<site>`,
//! pointed at the site's own server only (never the proxy, a database or
//! LocDev itself); its local metrics server tells the public address.
//! Tunnels end with the daemon: their address changes on every start, so
//! nothing is restarted on boot.

use std::collections::HashMap;
use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use locdev_core::deps::JobKind;
use locdev_core::model::{ProcessRole, ProcessState};
use locdev_core::protocol::{Request, Response};
use locdev_core::tunnel::{TunnelProgram, TunnelProvider, TunnelState, TunnelView};
use locdev_core::{AppError, ErrorCode};
use locdev_tunnel::cloudflare;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::info;

use super::Daemon;
use crate::jobs::Spec as JobSpec;
use crate::supervisor::Spec;

struct Tunnel {
    provider: String,
    allowed: Vec<String>,
    metrics: u16,
    target: u16,
    url: Option<String>,
}

#[derive(Default)]
pub struct TunnelsState {
    tunnels: Mutex<HashMap<String, Tunnel>>,
    /// `--version` of a program, by path (running it costs a moment).
    versions: Mutex<HashMap<PathBuf, String>>,
}

/// Input of a tunnel provider install job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInstallInput {
    pub provider: String,
}

pub fn process_id(site: &str) -> String {
    format!("@tunnel:{site}")
}

fn free_port() -> Result<u16, AppError> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
        .local_addr()?
        .port())
}

/// A small HTTP GET to the program's own metrics server on loopback.
async fn metrics_get(port: u16, path: &str) -> Option<String> {
    let request = async {
        let mut s = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .ok()?;
        s.write_all(format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
            .await
            .ok()?;
        let mut out = String::new();
        s.read_to_string(&mut out).await.ok()?;
        out.split_once("\r\n\r\n").map(|(_, body)| body.to_string())
    };
    tokio::time::timeout(Duration::from_secs(2), request)
        .await
        .ok()?
}

impl Daemon {
    fn tunnel_tools_dir(&self) -> PathBuf {
        self.paths.data_dir.join("tools").join("cloudflared")
    }

    /// LocDev's own download (newest), else one on PATH.
    async fn cloudflared(&self) -> Option<TunnelProgram> {
        let managed = std::fs::read_dir(self.tunnel_tools_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path().join(cloudflare::executable_name()))
            .filter(|p| p.is_file())
            .max_by(|a, b| {
                let v = |p: &PathBuf| {
                    p.parent()
                        .and_then(|d| d.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                };
                if locdev_tunnel::version_at_least(&v(a), &v(b)) {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            });
        let (path, managed) = match managed {
            Some(path) => (path, true),
            None => {
                let path_var = std::env::var("PATH").unwrap_or_default();
                (
                    locdev_scaffold::tools::find("cloudflared", &path_var)?,
                    false,
                )
            }
        };
        let cached = self.tunnels.versions.lock().unwrap().get(&path).cloned();
        let version = match cached {
            Some(v) => v,
            None => {
                let exe = path.clone();
                let out = tokio::task::spawn_blocking(move || {
                    locdev_core::process::command(&exe)
                        .arg("--version")
                        .output()
                })
                .await
                .ok()?
                .ok()?;
                let text = String::from_utf8_lossy(&out.stdout).into_owned()
                    + &String::from_utf8_lossy(&out.stderr);
                let v = cloudflare::parse_version(&text)?;
                self.tunnels
                    .versions
                    .lock()
                    .unwrap()
                    .insert(path.clone(), v.clone());
                v
            }
        };
        let outdated =
            (!locdev_tunnel::version_at_least(&version, cloudflare::MIN_VERSION)).then(|| {
                format!(
                    "version {version}: e-mail protection needs {} or newer",
                    cloudflare::MIN_VERSION
                )
            });
        Some(TunnelProgram {
            version,
            path,
            managed,
            outdated,
        })
    }

    async fn tunnel_providers(&self) -> Vec<TunnelProvider> {
        let mut cf = cloudflare::provider();
        cf.program = self.cloudflared().await;
        vec![cf]
    }

    fn tunnel_provider(id: &str) -> Result<(), AppError> {
        if id == cloudflare::ID {
            Ok(())
        } else {
            Err(AppError::not_found(format!(
                "no tunnel provider '{id}' (available: {})",
                cloudflare::ID
            )))
        }
    }

    /// What a tunnel looks like now: its process, and the address once the
    /// program's metrics server knows it.
    async fn tunnel_view(&self, site: &str) -> Option<TunnelView> {
        let (provider, allowed, metrics, target, url) = {
            let tunnels = self.tunnels.tunnels.lock().unwrap();
            let t = tunnels.get(site)?;
            (
                t.provider.clone(),
                t.allowed.clone(),
                t.metrics,
                t.target,
                t.url.clone(),
            )
        };
        let info = self.supervisor.info(&process_id(site));
        let mut url = url;
        if url.is_none() && info.state == ProcessState::Running {
            url = metrics_get(metrics, "/quicktunnel")
                .await
                .and_then(|body| cloudflare::hostname_from_metrics(&body));
            if let Some(found) = &url {
                info!(site, url = %found, "tunnel online");
                if let Some(t) = self.tunnels.tunnels.lock().unwrap().get_mut(site) {
                    t.url = Some(found.clone());
                }
            }
        }
        let state = match info.state {
            ProcessState::Running if url.is_some() => TunnelState::Online,
            ProcessState::Running | ProcessState::Stopping => TunnelState::Starting,
            ProcessState::Failed | ProcessState::Exited => TunnelState::Failed,
            ProcessState::Stopped => TunnelState::Stopped,
        };
        Some(TunnelView {
            site: site.to_string(),
            provider,
            state,
            url: if state == TunnelState::Online {
                url
            } else {
                None
            },
            started_at: info.started_at,
            allowed,
            target: format!("http://127.0.0.1:{target}"),
            error: (state == TunnelState::Failed).then(|| {
                info.message
                    .clone()
                    .unwrap_or_else(|| "the tunnel program stopped; see its log".into())
            }),
        })
    }

    async fn tunnel_list(&self) -> Vec<TunnelView> {
        let mut sites: Vec<String> = self
            .tunnels
            .tunnels
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        sites.sort();
        let mut out = Vec::new();
        for site in sites {
            if let Some(view) = self.tunnel_view(&site).await {
                out.push(view);
            }
        }
        out
    }

    async fn tunnel_start(
        &self,
        site: &str,
        provider: &str,
        allowed: Vec<String>,
    ) -> Result<TunnelView, AppError> {
        Self::tunnel_provider(provider)?;
        let mut allowed: Vec<String> = allowed
            .iter()
            .flat_map(|a| a.split(','))
            .map(|a| a.trim().to_ascii_lowercase())
            .filter(|a| !a.is_empty())
            .collect();
        allowed.dedup();
        if let Some(bad) = allowed.iter().find(|a| !locdev_tunnel::valid_email(a)) {
            return Err(AppError::invalid(format!(
                "'{bad}' is not an e-mail address or *@domain"
            )));
        }
        let project = self.project(site).await?;
        let Some(port) = project.port else {
            return Err(AppError::invalid(format!(
                "{site} has no port to share (it isn't served by a dev server)"
            )));
        };
        if !self.supervisor.is_active(&ProcessRole::Main.id(site)) {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                format!("{site} isn't running; start it first (`locdev up {site}`)"),
            ));
        }
        let program = self.cloudflared().await.ok_or_else(|| {
            AppError::new(
                ErrorCode::NotRunnable,
                "cloudflared isn't installed; install it with `locdev tunnel install cloudflare`",
            )
        })?;
        if !allowed.is_empty()
            && let Some(why) = &program.outdated
        {
            return Err(AppError::new(
                ErrorCode::NotRunnable,
                format!("cloudflared {why}; update it with `locdev tunnel install cloudflare`"),
            ));
        }

        let id = process_id(site);
        self.supervisor.stop(&id).await;
        let metrics = free_port()?;
        let args = cloudflare::args(port, &project.domain, metrics, &allowed);
        let command = std::iter::once(program.path.display().to_string())
            .chain(args)
            .map(|a| locdev_services::spec::quote(&a))
            .collect::<Vec<_>>()
            .join(" ");
        self.tunnels.tunnels.lock().unwrap().insert(
            site.to_string(),
            Tunnel {
                provider: provider.to_string(),
                allowed: allowed.clone(),
                metrics,
                target: port,
                url: None,
            },
        );
        self.supervisor.start(Spec {
            id,
            command,
            cwd: self.paths.data_dir.clone(),
            port: None,
            env: Vec::new(),
            restart_on_failure: false,
            notes: vec![format!(
                "Sharing {} (127.0.0.1:{port}) through a Cloudflare Quick Tunnel{}.",
                project.domain,
                if allowed.is_empty() {
                    String::new()
                } else {
                    format!(", only for {}", allowed.join(", "))
                }
            )],
        })?;
        info!(site, provider, "tunnel started");
        // The address usually arrives within a few seconds.
        for _ in 0..30 {
            if let Some(view) = self.tunnel_view(site).await
                && view.state != TunnelState::Starting
            {
                return Ok(view);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        self.tunnel_view(site)
            .await
            .ok_or_else(|| AppError::internal("the tunnel disappeared"))
    }

    pub(super) async fn tunnel_stop(&self, site: &str) -> bool {
        let known = self.tunnels.tunnels.lock().unwrap().remove(site).is_some();
        self.supervisor.stop(&process_id(site)).await;
        if known {
            info!(site, "tunnel stopped");
        }
        known
    }

    /// The install job: the latest release, checked, under data/tools.
    pub(super) async fn run_tunnel_install(
        &self,
        input: &serde_json::Value,
    ) -> Result<String, AppError> {
        let input: TunnelInstallInput =
            serde_json::from_value(input.clone()).map_err(|e| AppError::internal(e.to_string()))?;
        Self::tunnel_provider(&input.provider)?;
        let dir = self.tunnel_tools_dir();
        let (version, path) = tokio::task::spawn_blocking(move || {
            let release = cloudflare::latest()?;
            let path = cloudflare::install(&release, &dir)?;
            Ok::<_, String>((release.version, path))
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
        .map_err(|e| AppError::new(ErrorCode::Remote, e))?;
        self.tunnels.versions.lock().unwrap().remove(&path);
        Ok(format!("cloudflared {version} installed and verified"))
    }

    pub(super) async fn tunnel_request(&self, request: Request) -> Result<Response, AppError> {
        match request {
            Request::TunnelProviders => Ok(Response::TunnelProviders {
                providers: self.tunnel_providers().await,
            }),
            Request::TunnelInstall { provider } => {
                Self::tunnel_provider(&provider)?;
                if self.settings.get().network.offline {
                    return Err(AppError::new(ErrorCode::Remote, "offline mode is on"));
                }
                let (job, _) = self.jobs.enqueue(JobSpec {
                    kind: JobKind::TunnelInstall,
                    project: None,
                    input: serde_json::to_value(TunnelInstallInput {
                        provider: provider.clone(),
                    })
                    .unwrap_or_default(),
                    dedup: format!("tunnel-install:{provider}"),
                    automatic: false,
                    max_attempts: 1,
                    not_before: None,
                });
                self.job_wake.notify_one();
                Ok(Response::Job { job })
            }
            Request::TunnelList => Ok(Response::Tunnels {
                tunnels: self.tunnel_list().await,
            }),
            Request::TunnelStart {
                site,
                provider,
                allowed,
            } => Ok(Response::Tunnel {
                tunnel: self
                    .tunnel_start(
                        &site,
                        provider.as_deref().unwrap_or(cloudflare::ID),
                        allowed,
                    )
                    .await?,
            }),
            Request::TunnelStop { site } => {
                if !self.tunnel_stop(&site).await {
                    return Err(AppError::not_found(format!("{site} isn't shared")));
                }
                Ok(Response::Tunnels {
                    tunnels: self.tunnel_list().await,
                })
            }
            Request::TunnelLogs { site, after, limit } => Ok(Response::Logs {
                lines: self.supervisor.logs(
                    &process_id(&site),
                    after,
                    limit.unwrap_or(500).min(5000),
                ),
            }),
            _ => unreachable!("not a tunnel request"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_never_clash_with_sites() {
        // Site names can't start with '@'.
        assert_eq!(process_id("shop"), "@tunnel:shop");
        assert!(locdev_core::project_name("@tunnel:shop").is_err());
    }
}
