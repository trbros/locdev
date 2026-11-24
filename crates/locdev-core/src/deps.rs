//! Wire types for project dependencies, jobs, notifications and the
//! settings the app manages.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::RuntimeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecosystem {
    Npm,
    Composer,
}

impl Ecosystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Composer => "Packagist",
        }
    }
}

/// How the project declares a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepKind {
    Prod,
    Dev,
    Peer,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepStatus {
    UpToDate,
    Outdated,
    Vulnerable,
    /// Not checked yet, not installed, or not a registry package.
    Unknown,
}

/// The jump from the current version to the candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateKind {
    None,
    Patch,
    Minor,
    Major,
}

/// What the metadata says about moving to the candidate. Never a guarantee:
/// "compatible" means no conflict was found in the checks that could run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Compatible,
    Possibly,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advisory {
    /// OSV/GHSA id.
    pub id: String,
    pub summary: String,
    /// `CRITICAL`, `HIGH`, `MODERATE`, `LOW`, when the source says.
    pub severity: Option<String>,
    pub url: String,
    /// The first version that fixes it, when known.
    pub fixed: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageLinks {
    /// npmjs.com / packagist.org page.
    pub registry: String,
    pub repository: Option<String>,
    /// The repository's releases page (GitHub/GitLab), when it has one.
    pub releases: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageReport {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub kind: DepKind,
    /// The manifest's constraint (`^1.2.0`).
    pub declared: String,
    /// What the lockfile resolved.
    pub locked: Option<String>,
    /// What is in node_modules / vendor right now.
    pub installed: Option<String>,
    /// Newest stable release the declared constraint allows.
    pub wanted: Option<String>,
    /// Newest stable release.
    pub latest: Option<String>,
    /// From the current version to `latest`.
    pub update: UpdateKind,
    pub status: DepStatus,
    /// Of moving to `latest`.
    pub compatibility: Compatibility,
    pub reasons: Vec<String>,
    pub advisories: Vec<Advisory>,
    pub links: PackageLinks,
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepsReport {
    pub project: String,
    /// The package manager the lockfile belongs to.
    pub manager: Option<RuntimeKind>,
    /// Manifests and lockfiles that were read.
    pub files: Vec<String>,
    pub packages: Vec<PackageReport>,
    /// When the registries were last asked (ms), if ever.
    pub checked_at: Option<u64>,
    /// Problems reading files or asking registries; the rest is still valid.
    pub errors: Vec<String>,
}

/// One package's proposed change, shown before anything runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedChange {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub from: Option<String>,
    pub to: String,
    /// The constraint written to the manifest, when it changes.
    pub constraint: Option<String>,
    pub update: UpdateKind,
    pub compatibility: Compatibility,
    pub reasons: Vec<String>,
    pub links: PackageLinks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePlan {
    pub project: String,
    pub changes: Vec<PlannedChange>,
    /// The exact programs and arguments that will run, in order.
    pub commands: Vec<Vec<String>>,
    /// Files the package manager is expected to change.
    pub files: Vec<String>,
    /// Install scripts (postinstall…) run only when asked.
    pub run_scripts: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    DepsAudit,
    DepsUpdate,
    RuntimeInstall,
    /// Looks for failed workflow runs of a site's GitHub repository.
    GithubRuns,
    /// Checks for newer signed framework definitions.
    DefinitionsUpdate,
    /// Downloads and verifies a database or cache version.
    ServiceInstall,
    /// Downloads and verifies a tunnel provider's program.
    TunnelInstall,
    /// Looks for a newer LocDev release.
    UpdateCheck,
}

impl JobKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::DepsAudit => "Dependency check",
            Self::DepsUpdate => "Dependency update",
            Self::RuntimeInstall => "Runtime install",
            Self::GithubRuns => "Workflow check",
            Self::DefinitionsUpdate => "Framework definitions check",
            Self::ServiceInstall => "Service install",
            Self::TunnelInstall => "Tunnel program install",
            Self::UpdateCheck => "Update check",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

impl JobStatus {
    pub fn finished(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobProgress {
    pub done: u64,
    pub total: Option<u64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    pub project: Option<String>,
    pub status: JobStatus,
    /// Started by a schedule rather than by the user.
    #[serde(default)]
    pub automatic: bool,
    /// All times in ms since the epoch.
    pub created_at: u64,
    /// Not run before this.
    pub scheduled_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub progress: Option<JobProgress>,
    pub result: Option<String>,
    pub error: Option<String>,
    /// Steps and the output of what ran (bounded).
    #[serde(default)]
    pub log: Vec<String>,
    /// Kind-specific input (packages to update…).
    #[serde(default)]
    pub input: serde_json::Value,
    /// Jobs with the same key don't run twice at once.
    pub dedup: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    DependencyUpdate,
    Security,
    ProcessFailure,
    Runtime,
    Certificate,
    Job,
    Workflow,
    Action,
    /// A message captured by the Mailbox.
    Mail,
    /// A newer LocDev release.
    Update,
}

impl NotificationCategory {
    pub const ALL: [Self; 10] = [
        Self::DependencyUpdate,
        Self::Security,
        Self::ProcessFailure,
        Self::Runtime,
        Self::Certificate,
        Self::Job,
        Self::Workflow,
        Self::Action,
        Self::Mail,
        Self::Update,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::DependencyUpdate => "Dependency updates",
            Self::Security => "Security alerts",
            Self::ProcessFailure => "Process failures",
            Self::Runtime => "Runtimes",
            Self::Certificate => "Certificates",
            Self::Job => "Finished jobs",
            Self::Workflow => "Workflow failures",
            Self::Action => "Results of your actions",
            Self::Mail => "Captured emails",
            Self::Update => "LocDev updates",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

/// Where a notification leads in the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationLink {
    pub view: String,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: u64,
    pub category: NotificationCategory,
    pub severity: Severity,
    pub title: String,
    pub body: String,
    pub project: Option<String>,
    pub link: Option<NotificationLink>,
    pub created_at: u64,
    pub read: bool,
    pub dismissed: bool,
    /// Shown by the OS (when native notifications are on).
    pub delivered: bool,
    /// Same key, same notification: never shown twice.
    pub dedup: String,
}

/// How often dependencies are checked against the registries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditInterval {
    /// Never checked, not even on request.
    Off,
    /// Only when asked.
    Manual,
    #[serde(rename = "6h")]
    SixHours,
    #[serde(rename = "12h")]
    TwelveHours,
    #[default]
    Daily,
    Weekly,
}

impl AuditInterval {
    /// Time between automatic checks; `None` when they don't run.
    pub fn every_ms(self) -> Option<u64> {
        const HOUR: u64 = 3_600_000;
        match self {
            Self::Off | Self::Manual => None,
            Self::SixHours => Some(6 * HOUR),
            Self::TwelveHours => Some(12 * HOUR),
            Self::Daily => Some(24 * HOUR),
            Self::Weekly => Some(7 * 24 * HOUR),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomationSettings {
    pub audit: AuditInterval,
    /// Jobs that run at the same time.
    pub concurrency: u8,
    /// Registry requests per second, across all jobs.
    pub requests_per_second: u8,
    /// Check daily for newer signed framework definitions.
    pub definitions: bool,
    /// Check daily for a newer LocDev release (notification only; nothing
    /// is installed without the user).
    pub updates: bool,
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            audit: AuditInterval::Daily,
            concurrency: 2,
            requests_per_second: 8,
            definitions: true,
            updates: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationSettings {
    /// Also show notifications through the OS when no LocDev window is in
    /// front (the app shows its own toasts then).
    pub native: bool,
    /// Categories that produce notifications at all.
    pub categories: BTreeMap<NotificationCategory, bool>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            native: true,
            categories: NotificationCategory::ALL
                .into_iter()
                .map(|c| (c, true))
                .collect(),
        }
    }
}

impl NotificationSettings {
    pub fn enabled(&self, category: NotificationCategory) -> bool {
        self.categories.get(&category).copied().unwrap_or(true)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkSettings {
    /// No background network use (runtime catalogs, support schedules,
    /// dependency checks). Explicit requests still go out.
    pub offline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationSettings {
    /// Client id of a GitHub OAuth App, for the device flow. Empty: connect
    /// with a token or the GitHub CLI instead.
    pub github_client_id: String,
    /// Check connected repositories for failed workflow runs (every 15 min).
    pub github_watch_runs: bool,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self {
            github_client_id: String::new(),
            github_watch_runs: true,
        }
    }
}

/// When a project file already has a shared variable's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvConflictPolicy {
    /// Skip it when a file the dev server reads has the key; files it
    /// doesn't read (`.env.example`, `.env.production`) are only reported.
    #[default]
    ActiveFiles,
    /// Skip it when any `.env*` file has the key.
    AnyFile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvSettings {
    pub conflicts: EnvConflictPolicy,
}

/// Settings the app and CLI change at runtime (`settings.toml`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub automation: AutomationSettings,
    pub notifications: NotificationSettings,
    pub network: NetworkSettings,
    pub integrations: IntegrationSettings,
    pub env: EnvSettings,
    /// The code editor projects and files open in.
    pub editor: crate::editor::EditorSettings,
    /// The Debug Center: retention, disk space, which log lines count.
    pub observe: crate::observe::ObserveSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_interval_wire_names() {
        let json = serde_json::to_string(&[AuditInterval::SixHours, AuditInterval::Daily]).unwrap();
        assert_eq!(json, r#"["6h","daily"]"#);
        assert_eq!(AuditInterval::Manual.every_ms(), None);
        assert_eq!(AuditInterval::Daily.every_ms(), Some(86_400_000));
    }

    #[test]
    fn settings_default_and_partial() {
        let settings: Settings = toml::from_str("[automation]\naudit = \"weekly\"\n").unwrap();
        assert_eq!(settings.automation.audit, AuditInterval::Weekly);
        assert_eq!(settings.automation.concurrency, 2);
        // On by default; the daemon only shows them with no LocDev window in front.
        assert!(settings.notifications.native);
        assert!(
            settings
                .notifications
                .enabled(NotificationCategory::Security)
        );
    }
}
