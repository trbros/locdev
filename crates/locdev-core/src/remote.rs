//! Deploy platforms LocDev talks to through their public APIs (`locdev
//! deploy connect/status/run…`, the site's Deploy tab). Tokens stay in
//! the system keyring; a call needs the platform to document it.

use serde::{Deserialize, Serialize};

/// What a platform's public API lets LocDev do.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoteCapabilities {
    pub deploy: bool,
    pub cancel: bool,
    pub rollback: bool,
    /// Build logs through the API (else a link to the dashboard).
    pub logs: bool,
    pub env: bool,
    pub domains: bool,
}

/// The connected account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAccount {
    pub user: String,
    #[serde(default)]
    pub email: Option<String>,
    /// The team the token acts for (Vercel), when set.
    #[serde(default)]
    pub team: Option<String>,
}

/// A platform and how to connect it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePlatform {
    pub id: String,
    pub name: String,
    /// The API reference LocDev follows.
    pub docs: String,
    /// Where to create a token.
    pub token_url: String,
    /// How to create it and which access it needs.
    pub token_help: String,
    pub capabilities: RemoteCapabilities,
    #[serde(default)]
    pub account: Option<RemoteAccount>,
}

/// A project (Vercel) or site (Netlify) on the platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub framework: Option<String>,
    /// `owner/repo` of the connected Git repository.
    #[serde(default)]
    pub repo: Option<String>,
    /// The production branch, when the platform says.
    #[serde(default)]
    pub branch: Option<String>,
    /// The project's page on the platform.
    pub dashboard: String,
}

/// A local site linked to a remote project (kept in `projects.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployLink {
    pub platform: String,
    pub project_id: String,
    pub project_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteState {
    Queued,
    Building,
    Ready,
    Error,
    Canceled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeployment {
    pub id: String,
    pub state: RemoteState,
    #[serde(default)]
    pub url: Option<String>,
    /// Unix milliseconds.
    #[serde(default)]
    pub created_at: Option<u64>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    pub production: bool,
    /// The one production traffic goes to now.
    #[serde(default)]
    pub current: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub dashboard: String,
}

/// A variable on the platform: its name and where it applies, never its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEnvVar {
    pub key: String,
    pub targets: Vec<String>,
}

/// What deploying now would send, checked against the local repository.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeployGitCheck {
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    /// Problems that make the deploy not match the local code (unpushed
    /// commits, uncommitted changes, another repository).
    #[serde(default)]
    pub problems: Vec<String>,
}

/// A linked site's view of its remote project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub link: DeployLink,
    pub platform: RemotePlatform,
    pub project: RemoteProject,
    pub deployments: Vec<RemoteDeployment>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub env: Vec<RemoteEnvVar>,
    pub git: DeployGitCheck,
    /// What the API doesn't offer here, and where to do it instead.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A deployment's build log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteLog {
    pub lines: Vec<String>,
    /// When the API has no logs: the dashboard page that does.
    #[serde(default)]
    pub link: Option<String>,
}
