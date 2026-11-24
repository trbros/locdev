//! The platforms' public APIs (Vercel REST API, Netlify API). Every call
//! here is in the platform's reference (links in each module); what isn't
//! offered is reported with a link to the dashboard instead. Blocking:
//! callers run these off the async runtime. Tokens are passed in, never
//! stored or logged here.

pub mod netlify;
pub mod vercel;

use locdev_core::remote::{
    RemoteAccount, RemoteDeployment, RemoteEnvVar, RemoteLog, RemotePlatform, RemoteProject,
};
use serde_json::Value;

/// The Git reference a platform builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRef {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub sha: String,
}

/// One platform's client.
pub enum Client {
    Vercel(vercel::Vercel),
    Netlify(netlify::Netlify),
}

pub fn platforms() -> Vec<RemotePlatform> {
    vec![vercel::platform(), netlify::platform()]
}

impl Client {
    /// `base` overrides the API's address (tests); `None` = the platform's.
    pub fn new(
        platform: &str,
        token: String,
        team: Option<String>,
        base: Option<String>,
    ) -> Result<Self, String> {
        match platform {
            vercel::ID => Ok(Self::Vercel(vercel::Vercel::new(token, team, base))),
            netlify::ID => Ok(Self::Netlify(netlify::Netlify::new(token, base))),
            other => Err(format!(
                "LocDev doesn't talk to '{other}' yet (available: {}, {})",
                vercel::ID,
                netlify::ID
            )),
        }
    }

    pub fn account(&self) -> Result<RemoteAccount, String> {
        match self {
            Self::Vercel(c) => c.account(),
            Self::Netlify(c) => c.account(),
        }
    }

    pub fn projects(&self, scope: &str) -> Result<Vec<RemoteProject>, String> {
        match self {
            Self::Vercel(c) => c.projects(scope),
            Self::Netlify(c) => c.projects(),
        }
    }

    pub fn project(&self, id: &str, scope: &str) -> Result<RemoteProject, String> {
        match self {
            Self::Vercel(c) => c.project(id, scope),
            Self::Netlify(c) => c.project(id),
        }
    }

    pub fn deployments(&self, id: &str, scope: &str) -> Result<Vec<RemoteDeployment>, String> {
        match self {
            Self::Vercel(c) => c.deployments(id, scope),
            Self::Netlify(c) => c.deployments(id),
        }
    }

    pub fn domains(&self, id: &str) -> Result<Vec<String>, String> {
        match self {
            Self::Vercel(c) => c.domains(id),
            Self::Netlify(c) => c.domains(id),
        }
    }

    pub fn env(&self, id: &str) -> Result<Vec<RemoteEnvVar>, String> {
        match self {
            Self::Vercel(c) => c.env(id),
            Self::Netlify(c) => c.env(id),
        }
    }

    /// Starts a deployment of `git` (Vercel) or of the branch (Netlify,
    /// which builds the commit its own Git integration sees).
    pub fn deploy(
        &self,
        project: &RemoteProject,
        git: &GitRef,
        production: bool,
        scope: &str,
    ) -> Result<RemoteDeployment, String> {
        match self {
            Self::Vercel(c) => c.deploy(project, git, production, scope),
            Self::Netlify(c) => c.deploy(project, git, production),
        }
    }

    pub fn cancel(&self, deployment: &str) -> Result<(), String> {
        match self {
            Self::Vercel(c) => c.cancel(deployment),
            Self::Netlify(c) => c.cancel(deployment),
        }
    }

    pub fn rollback(&self, project: &str, deployment: &str) -> Result<(), String> {
        match self {
            Self::Vercel(c) => c.rollback(project, deployment),
            Self::Netlify(c) => c.rollback(project, deployment),
        }
    }

    pub fn logs(&self, project: &RemoteProject, deployment: &str) -> Result<RemoteLog, String> {
        match self {
            Self::Vercel(c) => c.logs(deployment),
            Self::Netlify(_) => Ok(netlify::logs_link(project, deployment)),
        }
    }
}

/// A small authenticated JSON client.
pub(crate) struct Http {
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl Http {
    pub(crate) fn new(base: String, token: String) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build()
            .into();
        Self { base, token, agent }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base.trim_end_matches('/'))
    }

    fn explain(&self, path: &str, error: ureq::Error) -> String {
        match error {
            ureq::Error::StatusCode(401) => {
                "the platform refused the token (401): it's wrong, expired or revoked".into()
            }
            ureq::Error::StatusCode(403) => {
                format!("the token isn't allowed to do this (403) at {path}; check its scope")
            }
            ureq::Error::StatusCode(404) => format!("not found (404) at {path}"),
            ureq::Error::StatusCode(429) => {
                "the platform's rate limit was reached (429); try again in a minute".into()
            }
            ureq::Error::StatusCode(code) => format!("the platform answered {code} at {path}"),
            other => format!("could not reach the platform: {other}"),
        }
    }

    fn read(
        &self,
        path: &str,
        response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<Value, String> {
        let mut response = response.map_err(|e| self.explain(path, e))?;
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("reading the answer from {path}: {e}"))?;
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| format!("the answer from {path} isn't JSON: {e}"))
    }

    pub(crate) fn get(&self, path: &str) -> Result<Value, String> {
        let response = self
            .agent
            .get(&self.url(path))
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "LocDev")
            .call();
        self.read(path, response)
    }

    pub(crate) fn post(&self, path: &str, body: Option<&Value>) -> Result<Value, String> {
        let request = self
            .agent
            .post(&self.url(path))
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "LocDev");
        let response = match body {
            Some(body) => request
                .header("Content-Type", "application/json")
                .send(body.to_string()),
            None => request.send_empty(),
        };
        self.read(path, response)
    }

    pub(crate) fn patch(&self, path: &str) -> Result<Value, String> {
        let response = self
            .agent
            .patch(&self.url(path))
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("User-Agent", "LocDev")
            .send_empty();
        self.read(path, response)
    }
}

/// A string field, if present and not empty.
pub(crate) fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `https://` in front of a bare host.
pub(crate) fn https(host: &str) -> String {
    if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("https://{host}")
    }
}

/// RFC 3339 → Unix milliseconds (what Netlify sends).
pub(crate) fn rfc3339_ms(value: &str) -> Option<u64> {
    // 2026-11-04T12:30:45.123Z
    let (date, time) = value.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let time = time.trim_end_matches('Z');
    let time = time.split(['+']).next()?;
    let mut t = time.split(':');
    let (h, min) = (
        t.next()?.parse::<i64>().ok()?,
        t.next()?.parse::<i64>().ok()?,
    );
    let sec: f64 = t.next()?.parse().ok()?;
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let ms = ((days * 86_400 + h * 3600 + min * 60) as f64 + sec) * 1000.0;
    (ms >= 0.0).then_some(ms as u64)
}

#[cfg(test)]
mod tests;
