//! GitHub for LocDev: connecting an account (OAuth device flow, a personal
//! access token, or the token of a logged-in GitHub CLI), keeping the token
//! in the system keyring only, and the REST calls the app needs.
//!
//! The token never reaches a log, a config file or an error message: it
//! lives in [`Token`], whose `Debug` is redacted.

pub mod releases;

use std::process::Stdio;
use std::time::Duration;

use locdev_core::git::{
    CheckRun, Commit, DeviceCode, GithubAccount, GithubAuthMethod, GithubItem, GithubOverview,
    GithubRepo, Release, Workflow, WorkflowRun,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API: &str = "https://api.github.com";
const KEYRING_SERVICE: &str = "dev.locdev.locdev";
const KEYRING_USER: &str = "github";
/// Device flow scopes: private repositories and re-running workflows.
const DEVICE_SCOPES: &str = "repo workflow";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubError {
    /// No token, or GitHub rejected it (expired, revoked).
    Unauthorized(String),
    /// Out of requests until the given time (seconds since the epoch).
    RateLimited(Option<u64>),
    /// Not found, or not visible with this token's permissions.
    NotFound(String),
    Network(String),
    /// The system keyring is missing or refused.
    Keyring(String),
    Other(String),
}

impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized(m) => write!(
                f,
                "GitHub refused the token: {m}; connect the account again"
            ),
            Self::RateLimited(Some(at)) => write!(
                f,
                "GitHub rate limit reached; it resets at {at} (unix time)"
            ),
            Self::RateLimited(None) => write!(f, "GitHub rate limit reached; try again later"),
            Self::NotFound(m) => write!(f, "{m} (not found, or this token has no access to it)"),
            Self::Network(m) => write!(f, "can't reach GitHub: {m}"),
            Self::Keyring(m) => write!(f, "system keyring: {m}"),
            Self::Other(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for GithubError {}

type Result<T> = std::result::Result<T, GithubError>;

/// A GitHub token. Never printed.
#[derive(Clone, Serialize, Deserialize)]
pub struct Token(String);

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

impl Token {
    pub fn new(raw: &str) -> Result<Self> {
        let raw = raw.trim();
        // ghp_, gho_, github_pat_…: printable, no whitespace, sane length.
        if raw.len() < 20 || raw.len() > 512 || !raw.chars().all(|c| c.is_ascii_graphic()) {
            return Err(GithubError::Other(
                "that doesn't look like a GitHub token".into(),
            ));
        }
        Ok(Self(raw.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stored {
    token: Token,
    method: GithubAuthMethod,
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| GithubError::Keyring(e.to_string()))
}

/// The stored token and how it was obtained, if any.
pub fn load() -> Result<Option<(Token, GithubAuthMethod)>> {
    match entry()?.get_password() {
        Ok(raw) => {
            let stored: Stored = serde_json::from_str(&raw)
                .map_err(|_| GithubError::Keyring("unreadable entry".into()))?;
            Ok(Some((stored.token, stored.method)))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(GithubError::Keyring(e.to_string())),
    }
}

/// Keeps the token in the system keyring. There is no file fallback: if
/// the keyring is unavailable, the account isn't connected.
pub fn store(token: &Token, method: GithubAuthMethod) -> Result<()> {
    let raw = serde_json::to_string(&Stored {
        token: token.clone(),
        method,
    })
    .map_err(|e| GithubError::Other(e.to_string()))?;
    entry()?
        .set_password(&raw)
        .map_err(|e| GithubError::Keyring(e.to_string()))
}

pub fn forget() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(GithubError::Keyring(e.to_string())),
    }
}

/// The token of a logged-in GitHub CLI (`gh auth token`), if one is there.
pub fn gh_cli_token() -> Result<Token> {
    let out = locdev_core::process::command("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| GithubError::Other("the GitHub CLI (gh) is not installed".into()))?;
    if !out.status.success() {
        return Err(GithubError::Unauthorized(
            "the GitHub CLI isn't logged in (gh auth login)".into(),
        ));
    }
    Token::new(&String::from_utf8_lossy(&out.stdout))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(concat!("locdev/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

pub struct Client {
    /// `None`: anonymous, for public data (60 requests an hour).
    token: Option<Token>,
    /// Requests left in the window, from the last response.
    pub remaining: std::cell::Cell<Option<u64>>,
}

impl Client {
    pub fn new(token: Token) -> Self {
        Self {
            token: Some(token),
            remaining: std::cell::Cell::new(None),
        }
    }

    pub fn anonymous() -> Self {
        Self {
            token: None,
            remaining: std::cell::Cell::new(None),
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<(Value, Option<String>, Option<String>)> {
        let url = if path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{API}{path}")
        };
        // Only GitHub's API ever gets the token.
        if !url.starts_with(API) {
            return Err(GithubError::Other(format!(
                "refusing to send the token to {url}"
            )));
        }
        let agent = agent();
        let auth = self.token.as_ref().map(|t| format!("Bearer {}", t.0));
        let result = match method {
            "POST" => {
                let mut request = agent
                    .post(&url)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28");
                if let Some(auth) = &auth {
                    request = request.header("Authorization", auth);
                }
                request.send(
                    body.unwrap_or(Value::Object(Default::default()))
                        .to_string(),
                )
            }
            _ => {
                let mut request = agent
                    .get(&url)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28");
                if let Some(auth) = &auth {
                    request = request.header("Authorization", auth);
                }
                request.call()
            }
        };
        let mut response = result.map_err(|e| GithubError::Network(e.to_string()))?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let remaining = header("x-ratelimit-remaining").and_then(|v| v.parse().ok());
        let reset = header("x-ratelimit-reset").and_then(|v| v.parse().ok());
        let link = header("link");
        let scopes = header("x-oauth-scopes");
        self.remaining.set(remaining);
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let message = json["message"].as_str().unwrap_or("").to_string();
        match status {
            200..=299 => Ok((json, link, scopes)),
            401 => Err(GithubError::Unauthorized(message)),
            403 | 429 if remaining == Some(0) || message.contains("rate limit") => {
                Err(GithubError::RateLimited(reset))
            }
            403 => Err(GithubError::Other(format!("GitHub denied it: {message}"))),
            404 => Err(GithubError::NotFound(path.to_string())),
            other => Err(GithubError::Other(format!(
                "GitHub answered {other}: {message}"
            ))),
        }
    }

    fn get(&self, path: &str) -> Result<Value> {
        Ok(self.request("GET", path, None)?.0)
    }

    /// Follows `Link: rel="next"` up to `pages` pages.
    fn get_pages(&self, path: &str, pages: usize) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let mut next = Some(path.to_string());
        for _ in 0..pages {
            let Some(url) = next.take() else {
                break;
            };
            let (json, link, _) = self.request("GET", &url, None)?;
            match json {
                Value::Array(items) => out.extend(items),
                other => out.push(other),
            }
            next = link.and_then(|l| next_link(&l));
        }
        Ok(out)
    }

    pub fn account(&self, method: GithubAuthMethod) -> Result<GithubAccount> {
        let (user, _, scopes) = self.request("GET", "/user", None)?;
        Ok(GithubAccount {
            login: user["login"].as_str().unwrap_or_default().to_string(),
            name: user["name"].as_str().map(str::to_string),
            method,
            scopes: scopes
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            storage: "system keyring".into(),
        })
    }

    pub fn overview(&self, repo: &GithubRepo) -> Result<GithubOverview> {
        let r = self.get(&format!("/repos/{}", repo.slug()))?;
        Ok(GithubOverview {
            repo: repo.clone(),
            description: r["description"].as_str().map(str::to_string),
            private: r["private"].as_bool().unwrap_or(false),
            default_branch: r["default_branch"].as_str().unwrap_or("main").to_string(),
            html_url: r["html_url"].as_str().unwrap_or_default().to_string(),
            open_issues: r["open_issues_count"].as_u64().unwrap_or(0),
            stars: r["stargazers_count"].as_u64().unwrap_or(0),
            rate_remaining: self.remaining.get(),
        })
    }

    pub fn issues(&self, repo: &GithubRepo) -> Result<Vec<GithubItem>> {
        Ok(self
            .get_pages(
                &format!("/repos/{}/issues?state=open&per_page=50", repo.slug()),
                2,
            )?
            .iter()
            // The issues endpoint also lists pull requests.
            .filter(|i| i.get("pull_request").is_none())
            .map(item)
            .collect())
    }

    pub fn pulls(&self, repo: &GithubRepo) -> Result<Vec<GithubItem>> {
        Ok(self
            .get_pages(
                &format!("/repos/{}/pulls?state=open&per_page=50", repo.slug()),
                2,
            )?
            .iter()
            .map(item)
            .collect())
    }

    pub fn workflows(&self, repo: &GithubRepo) -> Result<Vec<Workflow>> {
        let json = self.get(&format!(
            "/repos/{}/actions/workflows?per_page=100",
            repo.slug()
        ))?;
        Ok(json["workflows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|w| Workflow {
                id: w["id"].as_u64().unwrap_or(0),
                name: w["name"].as_str().unwrap_or_default().to_string(),
                path: w["path"].as_str().unwrap_or_default().to_string(),
                state: w["state"].as_str().unwrap_or_default().to_string(),
                url: w["html_url"].as_str().unwrap_or_default().to_string(),
            })
            .collect())
    }

    pub fn runs(&self, repo: &GithubRepo, per_page: u8) -> Result<Vec<WorkflowRun>> {
        let json = self.get(&format!(
            "/repos/{}/actions/runs?per_page={}",
            repo.slug(),
            per_page.clamp(1, 100)
        ))?;
        Ok(json["workflow_runs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(run)
            .collect())
    }

    /// The tag of `owner/repo`'s latest release (`v7.0.1`).
    pub fn latest_tag(&self, slug: &str) -> Result<String> {
        let json = self.get(&format!("/repos/{slug}/releases/latest"))?;
        json["tag_name"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| GithubError::Other(format!("{slug} has no releases")))
    }

    pub fn releases(&self, repo: &GithubRepo) -> Result<Vec<Release>> {
        let json = self.get(&format!("/repos/{}/releases?per_page=20", repo.slug()))?;
        Ok(json
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| Release {
                tag: r["tag_name"].as_str().unwrap_or_default().to_string(),
                name: r["name"].as_str().unwrap_or_default().to_string(),
                draft: r["draft"].as_bool().unwrap_or(false),
                prerelease: r["prerelease"].as_bool().unwrap_or(false),
                published_at: r["published_at"].as_str().map(str::to_string),
                url: r["html_url"].as_str().unwrap_or_default().to_string(),
            })
            .collect())
    }

    pub fn branches(&self, repo: &GithubRepo) -> Result<Vec<String>> {
        Ok(self
            .get_pages(&format!("/repos/{}/branches?per_page=100", repo.slug()), 3)?
            .iter()
            .filter_map(|b| b["name"].as_str().map(str::to_string))
            .collect())
    }

    pub fn commits(&self, repo: &GithubRepo, branch: &str) -> Result<Vec<Commit>> {
        let json = self.get(&format!(
            "/repos/{}/commits?per_page=30&sha={}",
            repo.slug(),
            encode(branch)
        ))?;
        Ok(json
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| {
                let sha = c["sha"].as_str().unwrap_or_default().to_string();
                let message = c["commit"]["message"].as_str().unwrap_or_default();
                Commit {
                    short: sha.chars().take(7).collect(),
                    sha,
                    author: c["commit"]["author"]["name"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    email: String::new(),
                    time: 0,
                    refs: Vec::new(),
                    subject: message.lines().next().unwrap_or_default().to_string(),
                    parents: Vec::new(),
                }
            })
            .collect())
    }

    pub fn checks(&self, repo: &GithubRepo, sha: &str) -> Result<Vec<CheckRun>> {
        let json = self.get(&format!(
            "/repos/{}/commits/{}/check-runs?per_page=100",
            repo.slug(),
            encode(sha)
        ))?;
        Ok(json["check_runs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| CheckRun {
                name: c["name"].as_str().unwrap_or_default().to_string(),
                status: c["status"].as_str().unwrap_or_default().to_string(),
                conclusion: c["conclusion"].as_str().map(str::to_string),
                url: c["html_url"].as_str().unwrap_or_default().to_string(),
            })
            .collect())
    }

    pub fn rerun_failed(&self, repo: &GithubRepo, run: u64) -> Result<()> {
        self.request(
            "POST",
            &format!(
                "/repos/{}/actions/runs/{run}/rerun-failed-jobs",
                repo.slug()
            ),
            None,
        )
        .map(|_| ())
    }

    pub fn cancel_run(&self, repo: &GithubRepo, run: u64) -> Result<()> {
        self.request(
            "POST",
            &format!("/repos/{}/actions/runs/{run}/cancel", repo.slug()),
            None,
        )
        .map(|_| ())
    }

    pub fn dispatch(&self, repo: &GithubRepo, workflow: u64, git_ref: &str) -> Result<()> {
        self.request(
            "POST",
            &format!(
                "/repos/{}/actions/workflows/{workflow}/dispatches",
                repo.slug()
            ),
            Some(serde_json::json!({ "ref": git_ref })),
        )
        .map(|_| ())
    }
}

fn item(i: &Value) -> GithubItem {
    GithubItem {
        number: i["number"].as_u64().unwrap_or(0),
        title: i["title"].as_str().unwrap_or_default().to_string(),
        state: i["state"].as_str().unwrap_or_default().to_string(),
        author: i["user"]["login"].as_str().unwrap_or_default().to_string(),
        url: i["html_url"].as_str().unwrap_or_default().to_string(),
        created_at: i["created_at"].as_str().unwrap_or_default().to_string(),
        updated_at: i["updated_at"].as_str().unwrap_or_default().to_string(),
        labels: i["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l["name"].as_str().map(str::to_string))
            .collect(),
        branch: i["head"]["ref"]
            .as_str()
            .map(|head| format!("{head} → {}", i["base"]["ref"].as_str().unwrap_or("?"))),
        draft: i["draft"].as_bool().unwrap_or(false),
    }
}

fn run(r: &Value) -> WorkflowRun {
    WorkflowRun {
        id: r["id"].as_u64().unwrap_or(0),
        name: r["name"].as_str().unwrap_or_default().to_string(),
        branch: r["head_branch"].as_str().unwrap_or_default().to_string(),
        event: r["event"].as_str().unwrap_or_default().to_string(),
        status: r["status"].as_str().unwrap_or_default().to_string(),
        conclusion: r["conclusion"].as_str().map(str::to_string),
        sha: r["head_sha"].as_str().unwrap_or_default().to_string(),
        url: r["html_url"].as_str().unwrap_or_default().to_string(),
        created_at: r["created_at"].as_str().unwrap_or_default().to_string(),
        workflow_id: r["workflow_id"].as_u64().unwrap_or(0),
    }
}

/// The `rel="next"` URL of a `Link` header.
pub fn next_link(header: &str) -> Option<String> {
    header.split(',').find_map(|part| {
        let (url, rel) = part.split_once(';')?;
        rel.contains("rel=\"next\"").then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string()
        })
    })
}

fn encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '.' | '_' | '~' | '/' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

/// OAuth device flow (RFC 8628) with a registered OAuth App's client id.
pub mod device {
    use super::*;

    pub struct Pending {
        pub client_id: String,
        pub device_code: String,
        pub code: DeviceCode,
    }

    pub fn start(client_id: &str) -> Result<Pending> {
        let mut response = agent()
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .send_form([("client_id", client_id), ("scope", DEVICE_SCOPES)])
            .map_err(|e| GithubError::Network(e.to_string()))?;
        let json: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| GithubError::Other(e.to_string()))?;
        let field = |k: &str| json[k].as_str().map(str::to_string);
        Ok(Pending {
            client_id: client_id.to_string(),
            device_code: field("device_code").ok_or_else(|| {
                GithubError::Other(format!("GitHub refused the client id: {json}"))
            })?,
            code: DeviceCode {
                user_code: field("user_code").unwrap_or_default(),
                verification_uri: field("verification_uri")
                    .unwrap_or_else(|| "https://github.com/login/device".into()),
                expires_in: json["expires_in"].as_u64().unwrap_or(900),
                interval: json["interval"].as_u64().unwrap_or(5),
            },
        })
    }

    pub enum Poll {
        Pending,
        SlowDown,
        Done(Token),
    }

    pub fn poll(pending: &Pending) -> Result<Poll> {
        let mut response = agent()
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .send_form([
                ("client_id", pending.client_id.as_str()),
                ("device_code", pending.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .map_err(|e| GithubError::Network(e.to_string()))?;
        let json: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| GithubError::Other(e.to_string()))?;
        if let Some(token) = json["access_token"].as_str() {
            return Ok(Poll::Done(Token::new(token)?));
        }
        match json["error"].as_str() {
            Some("authorization_pending") => Ok(Poll::Pending),
            Some("slow_down") => Ok(Poll::SlowDown),
            Some("expired_token") => Err(GithubError::Unauthorized(
                "the code expired; start again".into(),
            )),
            Some("access_denied") => Err(GithubError::Unauthorized(
                "you declined the authorization".into(),
            )),
            other => Err(GithubError::Other(format!(
                "device flow: {}",
                other.unwrap_or("unexpected answer")
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_never_print() {
        let token = Token::new("ghp_0123456789abcdefghijklmnopqrstuv").unwrap();
        assert_eq!(format!("{token:?}"), "Token(<redacted>)");
        assert!(Token::new("short").is_err());
        assert!(Token::new("has spaces in the middle of it which is not a token").is_err());
    }

    #[test]
    fn only_the_api_gets_the_token() {
        let client = Client::new(Token::new("ghp_0123456789abcdefghijklmnopqrstuv").unwrap());
        let error = client
            .request("GET", "https://evil.example/steal", None)
            .unwrap_err();
        assert!(matches!(error, GithubError::Other(m) if m.contains("refusing")));
    }

    #[test]
    fn follows_pagination_links() {
        let header = r#"<https://api.github.com/repos/a/b/issues?page=2>; rel="next", <https://api.github.com/repos/a/b/issues?page=5>; rel="last""#;
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://api.github.com/repos/a/b/issues?page=2")
        );
        assert_eq!(next_link(r#"<https://x>; rel="prev""#), None);
    }

    #[test]
    fn reads_items_and_runs() {
        let pr = serde_json::json!({
            "number": 7, "title": "Add X", "state": "open", "user": { "login": "ana" },
            "html_url": "https://github.com/a/b/pull/7", "labels": [{ "name": "feat" }],
            "head": { "ref": "feature" }, "base": { "ref": "main" }, "draft": true,
            "created_at": "2026-11-01T00:00:00Z", "updated_at": "2026-11-02T00:00:00Z"
        });
        let item = item(&pr);
        assert_eq!(item.branch.as_deref(), Some("feature → main"));
        assert!(item.draft);
        let r = run(
            &serde_json::json!({ "id": 1, "name": "CI", "status": "completed", "conclusion": "failure" }),
        );
        assert_eq!(r.conclusion.as_deref(), Some("failure"));
    }
}
