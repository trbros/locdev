//! Vercel REST API (reference: https://vercel.com/docs/rest-api, checked
//! 2026-11-04): `GET /v2/user`, `GET /v10/projects`, `GET
//! /v9/projects/{idOrName}`, `GET /v7/deployments`, `POST /v13/deployments`
//! (gitSource github org/repo/ref/sha), `PATCH /v12/deployments/{id}/cancel`,
//! `POST /v1/projects/{projectId}/rollback/{deploymentId}`, `GET
//! /v3/deployments/{idOrUrl}/events`, `GET /v9/projects/{idOrName}/domains`,
//! `GET /v10/projects/{idOrName}/env` (values are never requested).

use locdev_core::remote::{
    RemoteAccount, RemoteCapabilities, RemoteDeployment, RemoteEnvVar, RemoteLog, RemotePlatform,
    RemoteProject, RemoteState,
};
use serde_json::{Value, json};

use super::{GitRef, Http, https, text};

pub const ID: &str = "vercel";
const API: &str = "https://api.vercel.com";

pub fn platform() -> RemotePlatform {
    RemotePlatform {
        id: ID.into(),
        name: "Vercel".into(),
        docs: "https://vercel.com/docs/rest-api".into(),
        token_url: "https://vercel.com/account/settings/tokens".into(),
        token_help: "Account Settings → Tokens → Create Token. Scope it to the team that owns the projects (and give its slug or ID here), with an expiration you're comfortable with.".into(),
        capabilities: RemoteCapabilities {
            deploy: true,
            cancel: true,
            rollback: true,
            logs: true,
            env: true,
            domains: true,
        },
        account: None,
    }
}

pub struct Vercel {
    http: Http,
    team: Option<String>,
}

impl Vercel {
    pub fn new(token: String, team: Option<String>, base: Option<String>) -> Self {
        Self {
            http: Http::new(base.unwrap_or_else(|| API.into()), token),
            team: team.filter(|t| !t.is_empty()),
        }
    }

    /// `teamId=`/`slug=` for the team the calls act for.
    fn scoped(&self, path: &str) -> String {
        let Some(team) = &self.team else {
            return path.to_string();
        };
        let sep = if path.contains('?') { '&' } else { '?' };
        let key = if team.starts_with("team_") {
            "teamId"
        } else {
            "slug"
        };
        format!("{path}{sep}{key}={team}")
    }

    pub fn account(&self) -> Result<RemoteAccount, String> {
        let user = self.http.get(&self.scoped("/v2/user"))?;
        let user = user.get("user").unwrap_or(&Value::Null);
        Ok(RemoteAccount {
            user: text(user, "username").ok_or("Vercel sent no username")?,
            email: text(user, "email"),
            team: self.team.clone(),
        })
    }

    pub fn projects(&self, scope: &str) -> Result<Vec<RemoteProject>, String> {
        let body = self.http.get(&self.scoped("/v10/projects?limit=100"))?;
        Ok(body
            .get("projects")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| project_from(p, scope))
            .collect())
    }

    pub fn project(&self, id: &str, scope: &str) -> Result<RemoteProject, String> {
        let body = self.http.get(&self.scoped(&format!("/v9/projects/{id}")))?;
        project_from(&body, scope).ok_or_else(|| "Vercel sent no project".into())
    }

    pub fn deployments(&self, project: &str, scope: &str) -> Result<Vec<RemoteDeployment>, String> {
        let current = self
            .http
            .get(&self.scoped(&format!("/v9/projects/{project}")))
            .ok()
            .and_then(|p| {
                p.pointer("/targets/production/id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        let body = self
            .http
            .get(&self.scoped(&format!("/v7/deployments?projectId={project}&limit=20")))?;
        Ok(body
            .get("deployments")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|d| deployment_from(d, current.as_deref(), scope))
            .collect())
    }

    pub fn domains(&self, project: &str) -> Result<Vec<String>, String> {
        let body = self
            .http
            .get(&self.scoped(&format!("/v9/projects/{project}/domains")))?;
        Ok(body
            .get("domains")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|d| text(d, "name"))
            .collect())
    }

    pub fn env(&self, project: &str) -> Result<Vec<RemoteEnvVar>, String> {
        let body = self
            .http
            .get(&self.scoped(&format!("/v10/projects/{project}/env")))?;
        Ok(body
            .get("envs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                Some(RemoteEnvVar {
                    key: text(e, "key")?,
                    targets: match e.get("target") {
                        Some(Value::Array(t)) => t
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect(),
                        Some(Value::String(t)) => vec![t.clone()],
                        _ => Vec::new(),
                    },
                })
            })
            .collect())
    }

    pub fn deploy(
        &self,
        project: &RemoteProject,
        git: &GitRef,
        production: bool,
        scope: &str,
    ) -> Result<RemoteDeployment, String> {
        let mut body = json!({
            "name": project.name,
            "project": project.id,
            "gitSource": {
                "type": "github",
                "org": git.owner,
                "repo": git.repo,
                "ref": git.branch,
                "sha": git.sha,
            },
        });
        if production {
            body["target"] = json!("production");
        }
        let created = self.http.post(
            &self.scoped("/v13/deployments?skipAutoDetectionConfirmation=1"),
            Some(&body),
        )?;
        deployment_from(&created, None, scope).ok_or_else(|| "Vercel sent no deployment".into())
    }

    pub fn cancel(&self, deployment: &str) -> Result<(), String> {
        self.http
            .patch(&self.scoped(&format!("/v12/deployments/{deployment}/cancel")))?;
        Ok(())
    }

    pub fn rollback(&self, project: &str, deployment: &str) -> Result<(), String> {
        self.http.post(
            &self.scoped(&format!("/v1/projects/{project}/rollback/{deployment}")),
            None,
        )?;
        Ok(())
    }

    pub fn logs(&self, deployment: &str) -> Result<RemoteLog, String> {
        let body = self
            .http
            .get(&self.scoped(&format!("/v3/deployments/{deployment}/events")))?;
        let lines = body
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                e.pointer("/payload/text")
                    .or_else(|| e.get("text"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect();
        Ok(RemoteLog { lines, link: None })
    }
}

fn project_from(p: &Value, scope: &str) -> Option<RemoteProject> {
    let name = text(p, "name")?;
    let link = p.get("link");
    let repo = link.and_then(|l| Some(format!("{}/{}", text(l, "org")?, text(l, "repo")?)));
    Some(RemoteProject {
        id: text(p, "id")?,
        url: p
            .pointer("/targets/production/alias/0")
            .and_then(Value::as_str)
            .map(https),
        framework: text(p, "framework"),
        repo,
        branch: link.and_then(|l| text(l, "productionBranch")),
        dashboard: format!("https://vercel.com/{scope}/{name}"),
        name,
    })
}

fn state(raw: &str) -> RemoteState {
    match raw {
        "QUEUED" | "INITIALIZING" => RemoteState::Queued,
        "BUILDING" => RemoteState::Building,
        "READY" => RemoteState::Ready,
        "ERROR" => RemoteState::Error,
        "CANCELED" => RemoteState::Canceled,
        _ => RemoteState::Unknown,
    }
}

fn deployment_from(d: &Value, current: Option<&str>, scope: &str) -> Option<RemoteDeployment> {
    let id = text(d, "uid").or_else(|| text(d, "id"))?;
    let raw_state = text(d, "readyState")
        .or_else(|| text(d, "state"))
        .unwrap_or_default();
    let meta = d.get("meta").unwrap_or(&Value::Null);
    Some(RemoteDeployment {
        state: state(&raw_state),
        url: text(d, "url").map(|u| https(&u)),
        created_at: d
            .get("created")
            .or_else(|| d.get("createdAt"))
            .and_then(Value::as_u64),
        branch: text(meta, "githubCommitRef"),
        commit: text(meta, "githubCommitSha"),
        message: text(meta, "githubCommitMessage"),
        production: text(d, "target").as_deref() == Some("production"),
        current: current == Some(id.as_str()),
        error: text(d, "errorMessage"),
        dashboard: text(d, "inspectorUrl").unwrap_or_else(|| {
            format!(
                "https://vercel.com/{scope}/{}/{}",
                text(d, "name").unwrap_or_default(),
                id.trim_start_matches("dpl_")
            )
        }),
        id,
    })
}
