//! Netlify API (OpenAPI: https://open-api.netlify.com/, checked
//! 2026-11-04): `GET /user`, `GET /sites`, `GET /sites/{site_id}`, `GET
//! /sites/{site_id}/deploys`, `POST /sites/{site_id}/builds?branch=`,
//! `POST /deploys/{deploy_id}/cancel`, `POST
//! /sites/{site_id}/deploys/{deploy_id}/restore`, `GET
//! /accounts/{account_id}/env?site_id=` (keys and contexts only). Build
//! logs aren't in the public REST API: LocDev links to the deploy's page.

use locdev_core::remote::{
    RemoteAccount, RemoteCapabilities, RemoteDeployment, RemoteEnvVar, RemoteLog, RemotePlatform,
    RemoteProject, RemoteState,
};
use serde_json::Value;

use super::{GitRef, Http, https, rfc3339_ms, text};

pub const ID: &str = "netlify";
const API: &str = "https://api.netlify.com/api/v1";

pub fn platform() -> RemotePlatform {
    RemotePlatform {
        id: ID.into(),
        name: "Netlify".into(),
        docs: "https://open-api.netlify.com/".into(),
        token_url: "https://app.netlify.com/user/applications#personal-access-tokens".into(),
        token_help: "User settings → Applications → Personal access tokens → New access token. It acts as your user on every team you're in.".into(),
        capabilities: RemoteCapabilities {
            deploy: true,
            cancel: true,
            rollback: true,
            logs: false,
            env: true,
            domains: true,
        },
        account: None,
    }
}

pub struct Netlify {
    http: Http,
}

impl Netlify {
    pub fn new(token: String, base: Option<String>) -> Self {
        Self {
            http: Http::new(base.unwrap_or_else(|| API.into()), token),
        }
    }

    pub fn account(&self) -> Result<RemoteAccount, String> {
        let user = self.http.get("/user")?;
        Ok(RemoteAccount {
            user: text(&user, "full_name")
                .or_else(|| text(&user, "slug"))
                .or_else(|| text(&user, "email"))
                .ok_or("Netlify sent no user")?,
            email: text(&user, "email"),
            team: None,
        })
    }

    pub fn projects(&self) -> Result<Vec<RemoteProject>, String> {
        let body = self.http.get("/sites?per_page=100")?;
        Ok(body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(site_from)
            .collect())
    }

    pub fn project(&self, id: &str) -> Result<RemoteProject, String> {
        site_from(&self.http.get(&format!("/sites/{id}"))?)
            .ok_or_else(|| "Netlify sent no site".into())
    }

    pub fn deployments(&self, site: &str) -> Result<Vec<RemoteDeployment>, String> {
        let current = self.http.get(&format!("/sites/{site}")).ok().and_then(|s| {
            s.pointer("/published_deploy/id")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
        let body = self
            .http
            .get(&format!("/sites/{site}/deploys?per_page=20"))?;
        Ok(body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| deploy_from(d, current.as_deref()))
            .collect())
    }

    pub fn domains(&self, site: &str) -> Result<Vec<String>, String> {
        let s = self.http.get(&format!("/sites/{site}"))?;
        let mut out: Vec<String> = text(&s, "custom_domain").into_iter().collect();
        out.extend(
            s.get("domain_aliases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
        if let Some(default) = text(&s, "url").or_else(|| text(&s, "ssl_url")) {
            let host = default
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .to_string();
            if !out.contains(&host) {
                out.push(host);
            }
        }
        Ok(out)
    }

    pub fn env(&self, site: &str) -> Result<Vec<RemoteEnvVar>, String> {
        let s = self.http.get(&format!("/sites/{site}"))?;
        let account = text(&s, "account_id").ok_or("Netlify sent no account for the site")?;
        let body = self
            .http
            .get(&format!("/accounts/{account}/env?site_id={site}"))?;
        Ok(body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let mut targets: Vec<String> = e
                    .get("values")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|v| text(v, "context"))
                    .collect();
                targets.dedup();
                Some(RemoteEnvVar {
                    key: text(e, "key")?,
                    targets,
                })
            })
            .collect())
    }

    /// Netlify builds through its own Git integration; LocDev asks for a
    /// build of the branch (production when it's the production branch).
    pub fn deploy(
        &self,
        project: &RemoteProject,
        git: &GitRef,
        production: bool,
    ) -> Result<RemoteDeployment, String> {
        let path = if production {
            format!("/sites/{}/builds", project.id)
        } else {
            format!("/sites/{}/builds?branch={}", project.id, git.branch)
        };
        let build = self.http.post(&path, None)?;
        let deploy_id = text(&build, "deploy_id").or_else(|| text(&build, "id"));
        Ok(RemoteDeployment {
            id: deploy_id.clone().unwrap_or_default(),
            state: RemoteState::Queued,
            url: None,
            created_at: text(&build, "created_at").and_then(|t| rfc3339_ms(&t)),
            branch: Some(git.branch.clone()),
            commit: Some(git.sha.clone()),
            message: None,
            production,
            current: false,
            error: None,
            dashboard: match deploy_id {
                Some(id) => format!("{}/deploys/{id}", project.dashboard),
                None => format!("{}/deploys", project.dashboard),
            },
        })
    }

    pub fn cancel(&self, deployment: &str) -> Result<(), String> {
        self.http
            .post(&format!("/deploys/{deployment}/cancel"), None)?;
        Ok(())
    }

    pub fn rollback(&self, site: &str, deployment: &str) -> Result<(), String> {
        self.http
            .post(&format!("/sites/{site}/deploys/{deployment}/restore"), None)?;
        Ok(())
    }
}

/// Netlify has no build-log endpoint in its public REST API.
pub fn logs_link(project: &RemoteProject, deployment: &str) -> RemoteLog {
    RemoteLog {
        lines: Vec::new(),
        link: Some(format!("{}/deploys/{deployment}", project.dashboard)),
    }
}

fn site_from(s: &Value) -> Option<RemoteProject> {
    let build = s.get("build_settings").unwrap_or(&Value::Null);
    let repo = text(build, "repo_path").or_else(|| {
        text(build, "repo_url").map(|u| {
            u.trim_end_matches(".git")
                .rsplitn(3, '/')
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/")
        })
    });
    Some(RemoteProject {
        id: text(s, "id").or_else(|| text(s, "site_id"))?,
        name: text(s, "name")?,
        url: text(s, "ssl_url")
            .or_else(|| text(s, "url"))
            .map(|u| https(&u)),
        framework: s
            .pointer("/published_deploy/framework")
            .and_then(Value::as_str)
            .map(str::to_string),
        repo,
        branch: text(build, "repo_branch"),
        dashboard: text(s, "admin_url").unwrap_or_else(|| "https://app.netlify.com/".into()),
    })
}

fn state(raw: &str) -> RemoteState {
    match raw {
        "new" | "pending_review" | "enqueued" | "accepted" | "retrying" => RemoteState::Queued,
        "building" | "uploading" | "uploaded" | "preparing" | "prepared" | "processing"
        | "processed" => RemoteState::Building,
        "ready" => RemoteState::Ready,
        "error" | "rejected" => RemoteState::Error,
        "canceled" | "cancelled" => RemoteState::Canceled,
        _ => RemoteState::Unknown,
    }
}

fn deploy_from(d: &Value, current: Option<&str>) -> Option<RemoteDeployment> {
    let id = text(d, "id")?;
    let site_admin = text(d, "admin_url").unwrap_or_else(|| "https://app.netlify.com".into());
    Some(RemoteDeployment {
        state: state(&text(d, "state").unwrap_or_default()),
        url: text(d, "deploy_ssl_url")
            .or_else(|| text(d, "deploy_url"))
            .map(|u| https(&u)),
        created_at: text(d, "created_at").and_then(|t| rfc3339_ms(&t)),
        branch: text(d, "branch"),
        commit: text(d, "commit_ref"),
        message: text(d, "title"),
        production: text(d, "context").as_deref() == Some("production"),
        current: current == Some(id.as_str()),
        error: text(d, "error_message"),
        dashboard: format!("{site_admin}/deploys/{id}"),
        id,
    })
}
