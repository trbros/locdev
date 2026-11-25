use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::AppError;
use crate::actions::{
    ActionVersion, ExistingWorkflow, SavePlan, TemplateInfo, Validation, Workflow,
};
use crate::db::{
    DbConnection, DbFilter, DbRows, DbTable, DbTableInfo, QueryCheck, RedisEdit, RedisKey,
    RedisValue, RowEdit,
};
use crate::deps::{DepsReport, Job, Notification, Settings, UpdatePlan};
use crate::env::{
    EnvBackup, EnvHistoryEntry, EnvProfile, EnvVar, ProjectEnvReport, ProjectEnvSettings, SyncPlan,
    SyncResult, VaultStatus,
};
use crate::frameworks::{DefinitionInfo, RegistryStatus, StackReport};
use crate::git::{
    Branch, Commit, CommitDetail, Confirmation, DestructivePreview, DeviceCode, GitOp, GitResult,
    GitStatus, GithubAccount, GithubAction, GithubData, GithubSection, RefLogEntry, Remote,
    SecurityStatus, Stash,
};
use crate::model::{
    AuxProcessView, Check, DaemonStatus, DevServerMode, DnsStatus, LogLine, PhpExtensionsReport,
    ProcessRole, ProcessView, ProjectView, Release, RuntimeInfo, RuntimeKind, TerminalRuntimes,
    TlsStatus,
};
use crate::services::{InstanceBackup, InstanceView, ServiceInfo, ServiceKind, ServiceRelease};

/// Bump on any breaking change to `Request`/`Response`.
/// 2: package managers in `RuntimeKind`, new error codes.
pub const PROTOCOL_VERSION: u32 = 2;

/// Every message on the wire carries the protocol version of its sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub protocol: u32,
    pub body: T,
}

impl<T> Envelope<T> {
    pub fn new(body: T) -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Status,
    Shutdown,
    /// The app says whether one of its windows is in front. While it is, the
    /// daemon leaves notifications to the app's toasts instead of the OS.
    /// Sent on focus changes and every few seconds; it expires on its own.
    ClientPresence {
        focused: bool,
    },

    // New projects (`locdev-scaffold`): the catalog, the tools it needs,
    // the plan shown before running, and the terminal that runs it.
    ListStarters,
    /// The tools starters need, found on PATH (runs each one's `--version`).
    ListTools,
    /// Installs a tool by its documented command, in a terminal.
    InstallTool {
        id: String,
    },
    PlanProject {
        project: crate::scaffold::NewProject,
    },
    /// Recomputes the plan and runs it in a terminal; the folder must not exist.
    CreateProject {
        project: crate::scaffold::NewProject,
    },
    SearchPackages {
        registry: crate::scaffold::PackageRegistry,
        query: String,
    },
    ListTerminals,
    /// Output from byte offset `from` (0 for everything kept).
    TerminalRead {
        id: u64,
        #[serde(default)]
        from: u64,
    },
    /// Keystrokes for the running program (answers to its prompts).
    TerminalWrite {
        id: u64,
        data: String,
    },
    TerminalResize {
        id: u64,
        cols: u16,
        rows: u16,
    },
    /// Stops the run (the running program and the steps after it).
    TerminalKill {
        id: u64,
    },
    /// Forgets a finished terminal.
    TerminalClose {
        id: u64,
    },
    Doctor,
    /// Re-checks how the OS resolves the TLD (after installing the resolver).
    CheckDns,
    /// Re-checks whether the OS and browsers trust the local CA.
    CheckTls,
    /// Installed versions, defaults and install progress of every runtime.
    ListRuntimes {
        /// Re-scan the machine for system runtimes (slower: runs each one).
        #[serde(default)]
        refresh: bool,
    },
    /// Versions available for download.
    RuntimeCatalog {
        kind: RuntimeKind,
    },
    /// Starts installing a version in the background; poll `ListRuntimes`.
    /// `version` may be partial ("22") or "lts".
    InstallRuntime {
        kind: RuntimeKind,
        version: String,
    },
    /// Removes a LocDev-managed version (system ones are never touched).
    /// Refused while a running site uses it; `force` overrides the check
    /// for sites that are stopped and for the default.
    UninstallRuntime {
        kind: RuntimeKind,
        version: String,
        #[serde(default)]
        force: bool,
    },
    /// Sets (or clears, with `None`) the global default pin.
    SetDefaultRuntime {
        kind: RuntimeKind,
        #[serde(default)]
        version: Option<String>,
    },
    /// The default versions in the user's terminal; `enable` puts the shim
    /// folder first on PATH (`true`) or takes it out (`false`).
    TerminalRuntimes {
        #[serde(default)]
        enable: Option<bool>,
    },
    ListProjects,
    AddProject {
        path: PathBuf,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        command: Option<String>,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        auto_start: bool,
    },
    UpdateProject {
        name: String,
        /// `Some("")` clears the command.
        #[serde(default)]
        command: Option<String>,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        auto_start: Option<bool>,
        /// Node.js pin; `Some("")` clears it.
        #[serde(default)]
        node: Option<String>,
        /// PHP pin; `Some("")` clears it.
        #[serde(default)]
        php: Option<String>,
        /// Package manager pins; an empty value clears that pin.
        #[serde(default)]
        tools: BTreeMap<RuntimeKind, String>,
        #[serde(default)]
        restart_on_failure: Option<bool>,
    },
    RemoveProject {
        name: String,
        /// Also move the project's folder to the trash (refused for a folder
        /// that holds other sites, the home folder or a drive's root).
        #[serde(default)]
        delete_files: bool,
    },
    /// Deploy platforms LocDev talks to, and which are connected.
    DeployPlatforms,
    /// Checks a token against the platform, then keeps it in the keyring.
    DeployConnect {
        platform: String,
        token: String,
        /// The Vercel team the token acts for (slug or `team_…` ID).
        #[serde(default)]
        team: Option<String>,
    },
    DeployDisconnect {
        platform: String,
    },
    /// The platform's projects (Vercel) or sites (Netlify).
    DeployRemoteProjects {
        platform: String,
    },
    /// Links a site to a remote project; both `None` unlinks it.
    DeployLinkSite {
        site: String,
        #[serde(default)]
        platform: Option<String>,
        #[serde(default)]
        project: Option<String>,
    },
    /// The linked project: deployments, domains, variable names, and what a
    /// deploy from here would build.
    DeployRemoteStatus {
        site: String,
    },
    /// Builds the commit the site's repository has pushed (refused when it
    /// doesn't match the local code).
    DeployRun {
        site: String,
        #[serde(default)]
        production: bool,
    },
    DeployCancel {
        site: String,
        deployment: String,
    },
    /// Production traffic back to an earlier deployment.
    DeployRollback {
        site: String,
        deployment: String,
    },
    DeployLogs {
        site: String,
        deployment: String,
    },
    /// Debug Center events, newest first (`filter.after` for live updates).
    ObserveEvents {
        #[serde(default)]
        filter: crate::observe::EventFilter,
        #[serde(default)]
        limit: Option<usize>,
    },
    ObserveEvent {
        id: u64,
    },
    /// Repeated problems grouped, with counts per hour.
    ObserveGroups {
        #[serde(default)]
        filter: crate::observe::EventFilter,
    },
    ObserveStatus,
    /// Forgets a site's events, or all of them.
    ObserveClear {
        #[serde(default)]
        site: Option<String>,
    },
    /// Sends a PHP site's `dump()`/`dd()` to the Debug Center (through
    /// VarDumper's server format, in the process environment only); a
    /// running site restarts to pick it up.
    ObserveDumps {
        site: String,
        enabled: bool,
    },
    /// Code editors installed here and the preferred one; `refresh` looks
    /// at the disk again.
    EditorList {
        #[serde(default)]
        refresh: bool,
    },
    /// Opens a site's folder, or a file inside it (relative path, optional
    /// line and column), in `editor` or the preferred one.
    EditorOpen {
        site: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        line: Option<u32>,
        #[serde(default)]
        column: Option<u32>,
        #[serde(default)]
        editor: Option<String>,
    },
    /// Published LocDev releases and whether this daemon/CLI is behind;
    /// `refresh` asks GitHub now instead of using the last answer.
    UpdateCheck {
        #[serde(default)]
        refresh: bool,
    },
    /// Tunnel providers and whether their program is here.
    TunnelProviders,
    /// Downloads and verifies a provider's program (a job).
    TunnelInstall {
        provider: String,
    },
    /// Shared sites.
    TunnelList,
    /// Shares a running site on the internet; `allowed` limits it to these
    /// e-mail addresses (or `*@domain`).
    TunnelStart {
        site: String,
        #[serde(default)]
        provider: Option<String>,
        #[serde(default)]
        allowed: Vec<String>,
    },
    TunnelStop {
        site: String,
    },
    TunnelLogs {
        site: String,
        #[serde(default)]
        after: Option<u64>,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// The PHP version lines LocDev can configure.
    PhpConfigLines,
    /// One line's settings: PHP's own values, LocDev's file and its effect.
    PhpConfig {
        line: String,
    },
    /// Changes LocDev's file for a line: with `dry_run`, only what would
    /// change; else backed up, applied, checked against what PHP loads
    /// (rolled back if PHP refuses it) and the line's running sites
    /// restarted.
    PhpConfigChange {
        line: String,
        change: crate::phpini::PhpIniChange,
        #[serde(default)]
        dry_run: bool,
    },
    /// A file PHP loads for a line (its php.ini, its scan folder), read-only.
    PhpIniRead {
        line: String,
        path: PathBuf,
    },
    /// Parked folders and what their last scans found.
    ParkList,
    /// Parks a folder, or changes the rules of one already parked; scans it.
    ParkSet {
        path: PathBuf,
        #[serde(default)]
        options: crate::park::ParkOptions,
    },
    /// Stops parking a folder. Its sites are removed (never their files)
    /// unless `keep_sites`, which turns them into regular sites.
    ParkRemove {
        path: PathBuf,
        #[serde(default)]
        keep_sites: bool,
    },
    /// Scans one parked folder now, or every enabled one.
    ParkScan {
        #[serde(default)]
        path: Option<PathBuf>,
    },
    /// Ignores a project found in a parked folder (its site is removed), or
    /// stops ignoring it.
    ParkIgnore {
        path: PathBuf,
        ignore: bool,
    },
    StartProject {
        name: String,
    },
    StopProject {
        name: String,
    },
    RestartProject {
        name: String,
    },
    ProjectLogs {
        name: String,
        /// Only return lines with `seq` greater than this.
        #[serde(default)]
        after: Option<u64>,
        #[serde(default)]
        limit: Option<usize>,
        /// The main process (default) or the helper dev server.
        #[serde(default)]
        process: ProcessRole,
        /// An auxiliary process instead (`queue`…).
        #[serde(default)]
        aux: Option<String>,
    },
    /// Every supervised process with CPU and memory.
    ListProcesses,
    /// Records the user's decision about the detected helper dev server and
    /// optionally starts it right away. `command` overrides the detected one.
    SetDevServer {
        name: String,
        mode: DevServerMode,
        #[serde(default)]
        command: Option<String>,
        #[serde(default)]
        start: bool,
    },
    StartDevServer {
        name: String,
    },
    StopDevServer {
        name: String,
    },
    RestartDevServer {
        name: String,
    },
    /// Diagnoses HTTPS for one site (DNS, proxy, certificate, trust).
    CheckSiteHttps {
        name: String,
    },
    /// Serves the site over HTTPS only (HTTP redirects), or stops doing so.
    SetSecure {
        name: String,
        secure: bool,
    },
    /// Replaces the site's subdomain settings (wildcard and rules).
    SetSubdomains {
        name: String,
        subdomains: crate::subdomains::Subdomains,
    },
    /// How a hostname is resolved, routed and covered by a certificate.
    ExplainHost {
        host: String,
    },
    /// The site's dependencies from disk, judged with the last check's
    /// registry data. Never goes to the network.
    ListDeps {
        name: String,
    },
    /// Queues a check of the site's dependencies against the registries.
    CheckDeps {
        name: String,
    },
    /// What updating `packages` would do; nothing runs.
    PlanDepsUpdate {
        name: String,
        packages: Vec<String>,
        /// Allow moving past the declared constraint (a new major).
        #[serde(default)]
        allow_major: bool,
        /// Let the package manager run install scripts.
        #[serde(default)]
        run_scripts: bool,
    },
    /// Queues the update `PlanDepsUpdate` showed, with the same inputs.
    UpdateDeps {
        name: String,
        packages: Vec<String>,
        #[serde(default)]
        allow_major: bool,
        #[serde(default)]
        run_scripts: bool,
    },
    ListJobs {
        #[serde(default)]
        project: Option<String>,
    },
    GetJob {
        id: u64,
    },
    ControlJob {
        id: u64,
        action: JobAction,
    },
    ListNotifications {
        /// Include dismissed ones (the history).
        #[serde(default)]
        all: bool,
    },
    /// Marks notifications read/unread or dismissed; every one when `ids`
    /// is empty.
    MarkNotifications {
        #[serde(default)]
        ids: Vec<u64>,
        #[serde(default)]
        read: Option<bool>,
        #[serde(default)]
        dismissed: Option<bool>,
    },
    /// Shows a notification now, to check how they look and arrive.
    TestNotification,
    GetSettings,
    UpdateSettings {
        settings: Settings,
    },
    /// The site's repository: branch, upstream, changes.
    GitStatus {
        name: String,
    },
    GitDiff {
        name: String,
        path: String,
        #[serde(default)]
        staged: bool,
    },
    GitLog {
        name: String,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        skip: usize,
        #[serde(default)]
        limit: Option<usize>,
    },
    GitShow {
        name: String,
        sha: String,
    },
    GitBranches {
        name: String,
    },
    /// The repository's working trees (`GitOp::Worktree*` changes them).
    GitWorktrees {
        name: String,
    },
    GitStashes {
        name: String,
    },
    GitRemotes {
        name: String,
    },
    /// HEAD's reflog and the operations LocDev ran.
    GitHistory {
        name: String,
    },
    /// What `op` would lose; `None` when it isn't destructive.
    GitPreview {
        name: String,
        op: GitOp,
    },
    /// Runs `op`. Destructive ones need `confirm` (the phrase from the
    /// preview, and the password when one is set).
    Git {
        name: String,
        op: GitOp,
        #[serde(default)]
        confirm: Option<Confirmation>,
    },
    GetSecurity,
    /// Sets, changes or (with `new: None`) removes the LocDev password.
    SetPassword {
        #[serde(default)]
        current: Option<String>,
        #[serde(default)]
        new: Option<String>,
    },
    SetSecurityOptions {
        #[serde(default)]
        password: Option<String>,
        require_password: bool,
        backups: bool,
    },
    /// The connected GitHub account, if any.
    GithubAccount,
    /// Connects with a personal access token (kept in the system keyring).
    GithubConnectToken {
        token: String,
    },
    /// Copies the token of a logged-in GitHub CLI (`gh auth token`), once.
    GithubConnectGhCli,
    /// Starts the OAuth device flow; poll `GithubDevicePoll`.
    GithubDeviceStart,
    GithubDevicePoll,
    GithubDisconnect,
    /// One section of a site's GitHub repository.
    Github {
        name: String,
        section: GithubSection,
    },
    /// A change on GitHub, confirmed by the user.
    GithubAction {
        name: String,
        action: GithubAction,
    },
    /// Templates for the site, its workflow files and the action versions.
    ActionsOverview {
        name: String,
    },
    /// A template filled in from the site's stack.
    ActionsTemplate {
        name: String,
        template: String,
    },
    /// A workflow from the visual editor, as YAML, validated.
    ActionsRender {
        workflow: Workflow,
    },
    ActionsValidate {
        yaml: String,
    },
    /// One of the site's workflow files.
    ActionsRead {
        name: String,
        file: String,
    },
    /// What saving `yaml` as `file` would change.
    ActionsPlan {
        name: String,
        file: String,
        yaml: String,
    },
    /// Writes a new file, or replaces one when `replace` is its current
    /// hash (from `ActionsPlan`).
    ActionsSave {
        name: String,
        file: String,
        yaml: String,
        #[serde(default)]
        replace: Option<String>,
    },
    /// Every workflow file of the site, each validated (errors, security,
    /// outdated actions and community practices with their fixes).
    ActionsAudit {
        name: String,
    },
    /// Local versions LocDev kept of a workflow file, newest first.
    ActionsHistory {
        name: String,
        file: String,
    },
    ActionsHistoryVersion {
        name: String,
        file: String,
        id: String,
    },
    /// Downloads actionlint from its GitHub release (SHA-256 from the
    /// release's checksums file, matching GitHub's digest) for validation.
    ActionlintInstall,
    /// The action versions templates use (no network).
    ActionVersions,
    /// Asks GitHub for each known action's latest release.
    RefreshActionVersions,
    /// Shared variables (secret values left out), profiles, the vault and
    /// recent changes.
    EnvOverview,
    /// Creates or changes a shared variable; `None` keeps that field.
    EnvSet {
        key: String,
        #[serde(default = "global_profile")]
        profile: String,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        secret: Option<bool>,
        #[serde(default)]
        description: Option<String>,
    },
    EnvRemove {
        key: String,
        #[serde(default = "global_profile")]
        profile: String,
    },
    /// One value in clear. Needs the LocDev password when one is required.
    EnvReveal {
        key: String,
        #[serde(default = "global_profile")]
        profile: String,
        #[serde(default)]
        password: Option<String>,
    },
    /// Creates or renames a profile.
    EnvSaveProfile {
        id: String,
        #[serde(default)]
        label: String,
        #[serde(default)]
        description: String,
    },
    /// Removes an empty profile.
    EnvRemoveProfile {
        id: String,
    },
    /// What a project's processes get, with every conflict.
    EnvProject {
        name: String,
    },
    EnvSetProject {
        name: String,
        settings: ProjectEnvSettings,
    },
    /// What `env sync` would add to `file` (`.env` by default) of one
    /// project, or of every project. Nothing is written.
    EnvSyncPlan {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        file: Option<String>,
    },
    /// Applies one project's plan; `token` must be the one it showed.
    EnvSync {
        name: String,
        #[serde(default)]
        file: Option<String>,
        token: String,
    },
    /// Encrypted copies of the project's `.env*` files taken before writes.
    EnvBackups {
        name: String,
    },
    /// Puts a backup back (backing up the current file first).
    EnvRestore {
        name: String,
        id: String,
    },
    /// Every framework definition in use, and where they come from.
    StackDefinitions,
    /// A site's stack (`name`) or any folder's (`path`), explained.
    StackDetect {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        path: Option<PathBuf>,
    },
    /// Adds or removes components by hand, or picks the adapter
    /// (`Some("")` goes back to detection). `apply_command` also makes the
    /// adapter's command the site's command.
    StackSet {
        name: String,
        #[serde(default)]
        add: Vec<String>,
        #[serde(default)]
        remove: Vec<String>,
        #[serde(default)]
        adapter: Option<String>,
        #[serde(default)]
        apply_command: bool,
    },
    /// Downloads newer signed definitions now (`force` takes a revision
    /// that was rolled back).
    RefreshDefinitions {
        #[serde(default)]
        force: bool,
    },
    /// Goes back to the previous definitions (or the bundled ones).
    RollbackDefinitions,
    /// A site's auxiliary processes (queue worker, scheduler…), offered or added.
    ProjectProcesses {
        name: String,
    },
    /// Starts, stops or restarts one of them.
    ControlProcess {
        name: String,
        process: String,
        action: ProcessAction,
    },
    /// Records the user's decision about a process, or adds a custom one
    /// (`label` + `command`). `command: Some("")` goes back to the offered one.
    SetProcess {
        name: String,
        process: String,
        #[serde(default)]
        mode: Option<DevServerMode>,
        #[serde(default)]
        command: Option<String>,
        #[serde(default)]
        label: Option<String>,
    },
    /// Removes a custom process, or forgets the decision about an offered one.
    RemoveProcess {
        name: String,
        process: String,
    },
    /// Every service, its installations and the instances.
    ListServices {
        /// Look for system installations again.
        #[serde(default)]
        refresh: bool,
    },
    /// Versions LocDev can download (network).
    ServiceReleases {
        kind: ServiceKind,
    },
    /// Queues a verified download of a version; follow the job.
    InstallService {
        kind: ServiceKind,
        version: String,
    },
    /// Installs a version through the system's package manager (one of
    /// `ServiceInfo.packages`) in a terminal; the daemon looks it up again.
    InstallServicePackage {
        kind: ServiceKind,
        id: String,
    },
    /// Removes a LocDev-downloaded version no instance uses.
    UninstallService {
        kind: ServiceKind,
        version: String,
    },
    /// A new instance with its own data, port, config and logs.
    CreateInstance {
        name: String,
        kind: ServiceKind,
        /// The newest installed version when omitted.
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        autostart: bool,
    },
    ControlInstance {
        name: String,
        action: ProcessAction,
    },
    SetInstance {
        name: String,
        autostart: bool,
    },
    /// Removes an instance. Its data stays unless `delete_data`, which
    /// needs `confirm` to be the instance's name.
    RemoveInstance {
        name: String,
        #[serde(default)]
        delete_data: bool,
        #[serde(default)]
        confirm: Option<String>,
    },
    /// A cold copy of the data (the instance is stopped and started again).
    BackupInstance {
        name: String,
    },
    InstanceBackups {
        name: String,
    },
    /// Puts a backup back; the current data is moved aside, not deleted.
    /// `confirm` must be the instance's name.
    RestoreInstance {
        name: String,
        id: String,
        #[serde(default)]
        confirm: Option<String>,
    },
    InstanceLogs {
        name: String,
        #[serde(default)]
        after: Option<u64>,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// The PHP extensions a site needs against what its PHP loads.
    PhpExtensions {
        name: String,
    },
    /// Every extension a PHP on this machine loads, and the ones its
    /// extension folder has but doesn't load. `executable` must be one of
    /// the detected PHP versions.
    PhpModules {
        executable: PathBuf,
    },
    /// What the proxy saw for a site since the daemon started.
    RequestMetrics {
        name: String,
    },
    ResetRequestMetrics {
        name: String,
    },
    /// Runs PHP code in the Laravel app (`php artisan tinker --execute`),
    /// with the site's PHP and environment; output and time are limited.
    TinkerRun {
        name: String,
        code: String,
    },
    /// An interactive `php artisan tinker` in a terminal.
    TinkerOpen {
        name: String,
    },
    /// The site's email templates, mailables and components.
    EmailCatalog {
        name: String,
    },
    /// A template as segments (blocks and code), with sample data.
    EmailOpen {
        name: String,
        path: String,
    },
    /// The file's current hash, to notice edits made in another editor.
    EmailStat {
        name: String,
        path: String,
    },
    /// Renders the editor's segments (or the file) with sample data.
    EmailPreview {
        name: String,
        path: String,
        #[serde(default)]
        segments: Option<Vec<crate::email::Segment>>,
        #[serde(default)]
        theme: Option<serde_json::Value>,
        #[serde(default)]
        data: serde_json::Value,
        mode: crate::email::PreviewMode,
    },
    /// The file text the segments produce, and its diff. Nothing is written.
    EmailBuild {
        name: String,
        path: String,
        segments: Vec<crate::email::Segment>,
        theme: serde_json::Value,
    },
    /// Writes the reviewed content if the file still has `base_hash`.
    EmailSave {
        name: String,
        path: String,
        content: String,
        base_hash: String,
    },
    /// Keeps sample data for a template's previews (LocDev's state, not the project).
    EmailSample {
        name: String,
        path: String,
        data: serde_json::Value,
    },
    /// `locdev new email [component] <item>`.
    EmailNew {
        name: String,
        #[serde(default)]
        component: bool,
        item: String,
        #[serde(default)]
        markdown: bool,
    },
    /// Renders and sends the result to the Mailbox (never anywhere else).
    EmailSendTest {
        name: String,
        path: String,
        #[serde(default)]
        segments: Option<Vec<crate::email::Segment>>,
        #[serde(default)]
        theme: Option<serde_json::Value>,
        #[serde(default)]
        data: serde_json::Value,
        mode: crate::email::PreviewMode,
        #[serde(default)]
        to: Option<String>,
    },
    EmailHistory {
        name: String,
        path: String,
    },
    EmailHistoryVersion {
        name: String,
        path: String,
        id: String,
    },
    /// The capture server and how many messages it holds.
    MailboxStatus,
    /// Captured messages, newest first.
    MailList {
        #[serde(default)]
        site: Option<String>,
        #[serde(default)]
        query: Option<String>,
        #[serde(default)]
        unread: bool,
        #[serde(default)]
        attachments: bool,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// One message, parsed; marks it read.
    MailGet {
        id: String,
    },
    /// The message exactly as received (`.eml`), base64.
    MailRaw {
        id: String,
    },
    MailAttachment {
        id: String,
        index: usize,
    },
    MailDelete {
        ids: Vec<String>,
    },
    /// Every message, or one site's.
    MailClear {
        #[serde(default)]
        site: Option<String>,
    },
    /// How the site sends mail now and what the Mailbox would change.
    MailSetup {
        name: String,
    },
    /// Turns the site's Mailbox on or off. `override_real` is the consent to
    /// replace a real SMTP/API transport while it's on.
    MailEnable {
        name: String,
        enabled: bool,
        #[serde(default)]
        override_real: bool,
    },
    /// Sends a test message to the Mailbox (never anywhere else).
    MailSendTest {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        to: Option<String>,
    },
    /// What the site needs from a host, which platforms fit and readiness checks.
    DeployAnalysis {
        name: String,
    },
    /// The config file LocDev would write for a platform (nothing is written).
    DeployConfig {
        name: String,
        provider: String,
    },
    /// Writes that file if it still has `hash` (what was reviewed); never overwrites.
    DeployWriteConfig {
        name: String,
        provider: String,
        hash: String,
    },
    /// The site's config files (only the ones that exist).
    ConfigFiles {
        name: String,
    },
    /// A config file: its text, hash and the options the form can show.
    ConfigFile {
        name: String,
        path: String,
    },
    /// What a change would do, as a diff. Nothing is written.
    ConfigPreview {
        name: String,
        path: String,
        change: crate::config::ConfigChange,
    },
    /// Saves a change if the file still has `base_hash` (what was reviewed).
    ConfigSave {
        name: String,
        path: String,
        change: crate::config::ConfigChange,
        base_hash: String,
    },
    /// Versions LocDev kept before each save, newest first.
    ConfigHistory {
        name: String,
        path: String,
    },
    /// Puts a kept version back (the current one is kept too).
    ConfigRevert {
        name: String,
        path: String,
        id: String,
        base_hash: String,
    },
    /// Content of a kept version, to compare.
    ConfigHistoryVersion {
        name: String,
        path: String,
        id: String,
    },
    /// Analysis of the site's config against its installed versions.
    ConfigCheck {
        name: String,
    },
    /// Databases the sites point at, and LocDev's instances. No secrets.
    DbConnections {
        /// Only this site's (plus the instances).
        #[serde(default)]
        name: Option<String>,
    },
    /// Databases on the server and the current database's tables.
    DbSchema {
        connection: String,
        #[serde(default)]
        allow_remote: bool,
    },
    DbTableInfo {
        connection: String,
        table: String,
        #[serde(default)]
        schema: String,
        #[serde(default)]
        allow_remote: bool,
    },
    /// A page of a table, filtered and ordered by its own columns.
    DbRows {
        connection: String,
        table: String,
        #[serde(default)]
        schema: String,
        #[serde(default)]
        page: u64,
        #[serde(default)]
        page_size: Option<u64>,
        #[serde(default)]
        order_by: Option<String>,
        #[serde(default)]
        desc: bool,
        #[serde(default)]
        filters: Vec<DbFilter>,
        #[serde(default)]
        allow_remote: bool,
    },
    /// What a statement would do, before running it.
    DbCheck {
        connection: String,
        sql: String,
    },
    /// Runs SQL; destructive statements need `confirm` (the database's name).
    DbQuery {
        connection: String,
        sql: String,
        #[serde(default)]
        confirm: Option<String>,
        #[serde(default)]
        allow_remote: bool,
    },
    DbEditRow {
        connection: String,
        table: String,
        #[serde(default)]
        schema: String,
        edit: RowEdit,
        #[serde(default)]
        allow_remote: bool,
    },
    /// Creates a database on the connection's server: `database`, or the
    /// one the connection names (it said it was missing).
    DbCreateDatabase {
        connection: String,
        #[serde(default)]
        database: Option<String>,
    },
    RedisKeys {
        connection: String,
        #[serde(default)]
        cursor: Option<String>,
        #[serde(default)]
        pattern: Option<String>,
        #[serde(default)]
        allow_remote: bool,
    },
    RedisGet {
        connection: String,
        key: String,
        #[serde(default)]
        allow_remote: bool,
    },
    RedisEdit {
        connection: String,
        edit: RedisEdit,
        #[serde(default)]
        allow_remote: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessAction {
    Start,
    Stop,
    Restart,
}

fn global_profile() -> String {
    crate::env::GLOBAL_PROFILE.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobAction {
    Cancel,
    Pause,
    Resume,
    Retry,
}

// A response is built once per request and serialized right away, so the
// size gap between variants costs nothing worth boxing for.
#[allow(clippy::large_enum_variant)]
/// A PHP version line LocDev can configure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhpLine {
    pub line: String,
    /// The installation sites resolve to in that line.
    pub version: String,
    pub executable: PathBuf,
    /// LocDev's file for it has settings.
    pub configured: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    Starters {
        starters: Vec<crate::scaffold::Starter>,
        /// Where the app creates new projects by default (`~/LocDev/Sites`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sites_dir: Option<std::path::PathBuf>,
    },
    Tools {
        tools: Vec<crate::scaffold::Tool>,
    },
    ProjectPlan(crate::scaffold::ProjectPlan),
    Terminal(crate::scaffold::TerminalInfo),
    Terminals {
        terminals: Vec<crate::scaffold::TerminalInfo>,
    },
    TerminalOutput(crate::scaffold::TerminalOutput),
    Packages {
        packages: Vec<crate::scaffold::PackageHit>,
    },
    Status(DaemonStatus),
    Doctor {
        checks: Vec<Check>,
    },
    Dns(DnsStatus),
    Tls(TlsStatus),
    Runtimes {
        runtimes: Vec<RuntimeInfo>,
    },
    TerminalRuntimes(TerminalRuntimes),
    Catalog {
        releases: Vec<Release>,
    },
    Projects {
        projects: Vec<ProjectView>,
    },
    Project {
        project: ProjectView,
    },
    DeployPlatforms {
        platforms: Vec<crate::remote::RemotePlatform>,
    },
    RemoteProjects {
        projects: Vec<crate::remote::RemoteProject>,
    },
    RemoteStatus(crate::remote::RemoteStatus),
    RemoteDeployment {
        deployment: crate::remote::RemoteDeployment,
    },
    RemoteLog(crate::remote::RemoteLog),
    TunnelProviders {
        providers: Vec<crate::tunnel::TunnelProvider>,
    },
    Updates(crate::update::UpdateStatus),
    Editors(crate::editor::EditorsView),
    ObserveEvents {
        events: Vec<crate::observe::Event>,
        /// The newest id so far (poll with `after`).
        newest: u64,
    },
    ObserveEvent {
        event: crate::observe::Event,
    },
    ObserveGroups {
        groups: Vec<crate::observe::EventGroup>,
    },
    ObserveStatus(crate::observe::ObserveStatus),
    ObserveDumps {
        site: String,
        enabled: bool,
        restarted: bool,
    },
    Tunnels {
        tunnels: Vec<crate::tunnel::TunnelView>,
    },
    Tunnel {
        tunnel: crate::tunnel::TunnelView,
    },
    PhpLines {
        lines: Vec<PhpLine>,
    },
    PhpConfig {
        config: crate::phpini::PhpConfigView,
        /// With `dry_run`: what the change would do.
        #[serde(default)]
        preview: Option<crate::phpini::PhpIniPreview>,
        /// Sites restarted to pick the change up.
        #[serde(default)]
        restarted: Vec<String>,
    },
    PhpIniFile {
        path: PathBuf,
        text: String,
    },
    Parked {
        dirs: Vec<crate::park::ParkedView>,
        /// Sites the request registered.
        #[serde(default)]
        added: Vec<String>,
        /// Sites the request removed.
        #[serde(default)]
        removed: Vec<String>,
    },
    Logs {
        lines: Vec<LogLine>,
    },
    Processes {
        processes: Vec<ProcessView>,
    },
    Deps(DepsReport),
    DepsPlan(UpdatePlan),
    Job {
        job: Job,
    },
    Jobs {
        jobs: Vec<Job>,
    },
    Notifications {
        notifications: Vec<Notification>,
        unread: usize,
    },
    Settings(Settings),
    GitStatus(GitStatus),
    Text {
        text: String,
        truncated: bool,
    },
    Commits {
        commits: Vec<Commit>,
    },
    Commit(CommitDetail),
    Branches {
        branches: Vec<Branch>,
    },
    Worktrees {
        worktrees: Vec<crate::git::Worktree>,
    },
    Stashes {
        stashes: Vec<Stash>,
    },
    Remotes {
        remotes: Vec<Remote>,
    },
    GitHistory {
        reflog: Vec<RefLogEntry>,
        operations: Vec<String>,
    },
    GitPreview {
        preview: Option<DestructivePreview>,
    },
    GitResult(GitResult),
    Security(SecurityStatus),
    Github {
        account: Option<GithubAccount>,
    },
    DeviceCode(DeviceCode),
    GithubData(GithubData),
    ActionsOverview {
        templates: Vec<TemplateInfo>,
        existing: Vec<ExistingWorkflow>,
        actions: Vec<ActionVersion>,
        /// When the versions were last checked against GitHub (ms).
        versions_checked_at: Option<u64>,
        /// actionlint's version, when installed.
        actionlint: Option<String>,
    },
    ActionsWorkflow {
        workflow: Workflow,
        file: String,
    },
    ActionsYaml {
        yaml: String,
        validation: Validation,
    },
    ActionsValidation(Validation),
    ActionsPlan(SavePlan),
    ActionsSaved {
        path: String,
    },
    ActionVersions {
        actions: Vec<ActionVersion>,
        checked_at: Option<u64>,
    },
    EnvOverview {
        profiles: Vec<EnvProfile>,
        vars: Vec<EnvVar>,
        vault: VaultStatus,
        history: Vec<EnvHistoryEntry>,
    },
    EnvVar {
        var: EnvVar,
    },
    EnvValue {
        value: String,
    },
    EnvProfile {
        profile: EnvProfile,
    },
    EnvProject(ProjectEnvReport),
    EnvSyncPlans {
        plans: Vec<SyncPlan>,
    },
    EnvSynced(SyncResult),
    EnvBackups {
        backups: Vec<EnvBackup>,
    },
    StackDefinitions {
        definitions: Vec<DefinitionInfo>,
        registry: RegistryStatus,
    },
    StackReport(StackReport),
    ProjectProcesses {
        processes: Vec<AuxProcessView>,
    },
    Services {
        services: Vec<ServiceInfo>,
        instances: Vec<InstanceView>,
    },
    ServiceReleases {
        releases: Vec<ServiceRelease>,
    },
    Instance(InstanceView),
    InstanceBackup(InstanceBackup),
    InstanceBackups {
        backups: Vec<InstanceBackup>,
    },
    PhpExtensions(PhpExtensionsReport),
    PhpModules(crate::model::PhpModules),
    ActionsAudit {
        files: Vec<crate::actions::WorkflowAudit>,
        #[serde(default)]
        actionlint: Option<String>,
    },
    ActionsHistory {
        versions: Vec<crate::actions::WorkflowVersion>,
    },
    RequestMetrics(crate::metrics::RequestMetrics),
    Tinker(crate::model::TinkerOutput),
    HostExplain(crate::subdomains::HostExplain),
    EmailCatalog(crate::email::EmailCatalog),
    EmailDoc(Box<crate::email::EmailDoc>),
    EmailStat {
        hash: String,
    },
    EmailPreview(crate::email::EmailPreview),
    EmailBuilt {
        content: String,
        diff: String,
    },
    EmailSaved {
        hash: String,
    },
    EmailCreated(crate::email::EmailCreated),
    MailboxStatus(crate::mail::MailboxStatus),
    MailList {
        messages: Vec<crate::mail::MailSummary>,
    },
    MailDetail(Box<crate::mail::MailDetail>),
    MailRaw {
        base64: String,
    },
    MailAttachment {
        #[serde(default)]
        name: Option<String>,
        content_type: String,
        base64: String,
    },
    MailDeleted {
        count: usize,
    },
    MailSetup(crate::mail::MailSetup),
    DeployReport(crate::deploy::DeployReport),
    DeployConfig(crate::deploy::DeployConfig),
    DeployConfigWritten {
        file: String,
    },
    ConfigFiles {
        files: Vec<crate::config::ConfigFile>,
    },
    ConfigFile(crate::config::ConfigFileView),
    ConfigPreview(crate::config::ConfigPreview),
    ConfigSaved {
        hash: String,
    },
    ConfigHistory {
        entries: Vec<crate::config::ConfigHistoryEntry>,
    },
    ConfigHistoryVersion {
        content: String,
    },
    ConfigFindings {
        findings: Vec<crate::config::ConfigFinding>,
    },
    DbConnections {
        connections: Vec<DbConnection>,
    },
    DbSchema {
        databases: Vec<String>,
        tables: Vec<DbTable>,
    },
    DbTableInfo(DbTableInfo),
    DbRows(DbRows),
    DbCheck(QueryCheck),
    DbAffected {
        rows: u64,
    },
    RedisKeys {
        cursor: String,
        keys: Vec<RedisKey>,
    },
    RedisValue(RedisValue),
    Error(AppError),
}

impl From<AppError> for Response {
    fn from(error: AppError) -> Self {
        Self::Error(error)
    }
}

impl Response {
    pub fn into_result(self) -> Result<Self, AppError> {
        match self {
            Self::Error(error) => Err(error),
            other => Ok(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorCode;

    #[test]
    fn request_wire_format() {
        let json = serde_json::to_value(Envelope::new(Request::StartProject {
            name: "blog".into(),
        }))
        .unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "protocol": PROTOCOL_VERSION,
                "body": { "type": "start_project", "name": "blog" }
            })
        );
    }

    #[test]
    fn error_wire_format() {
        let response = Response::Error(AppError::new(ErrorCode::NotFound, "no such project"));
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(
            json,
            serde_json::json!({ "type": "error", "code": "NOT_FOUND", "message": "no such project" })
        );
        assert_eq!(serde_json::from_value::<Response>(json).unwrap(), response);
    }

    #[test]
    fn optional_fields_default() {
        let request: Request =
            serde_json::from_str(r#"{ "type": "add_project", "path": "/srv/blog" }"#).unwrap();

        assert!(matches!(
            request,
            Request::AddProject {
                name: None,
                auto_start: false,
                ..
            }
        ));
    }

    /// The app mirrors these flat shapes (`GitResponse` in protocol.ts).
    #[test]
    fn nested_tags_flatten() {
        use crate::git::{GitOp, GithubData, ResetMode};

        let response = Response::GithubData(GithubData::Branches {
            names: vec!["main".into()],
        });
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "github_data", "section": "branches", "names": ["main"] })
        );
        assert_eq!(serde_json::from_value::<Response>(json).unwrap(), response);

        let request: Request = serde_json::from_value(serde_json::json!({
            "type": "git", "name": "blog",
            "op": { "op": "reset", "target": "HEAD~1", "mode": "hard" },
            "confirm": { "phrase": "abc1234" }
        }))
        .unwrap();
        assert!(matches!(
            request,
            Request::Git {
                op: GitOp::Reset {
                    mode: ResetMode::Hard,
                    ..
                },
                confirm: Some(_),
                ..
            }
        ));
    }

    /// The app sends every field, with `null` for unset ones (protocol.ts).
    #[test]
    fn workflow_from_the_app() {
        use crate::actions::Permissions;

        let request: Request = serde_json::from_value(serde_json::json!({
            "type": "actions_render",
            "workflow": {
                "name": "CI",
                "on": { "push": { "branches": ["main"], "tags": [], "paths": [] }, "pull_request": null,
                        "workflow_dispatch": true, "schedule": [], "release": false },
                "permissions": { "kind": "scopes", "scopes": { "contents": "read" } },
                "concurrency": null,
                "env": {},
                "jobs": [{
                    "id": "ci", "name": null, "runs_on": "ubuntu-latest", "needs": [], "if": null,
                    "permissions": { "kind": "read_all" }, "environment": null, "timeout_minutes": 15,
                    "matrix": {}, "env": {},
                    "steps": [{ "id": null, "name": null, "uses": "actions/checkout@v7", "with": {},
                                "run": null, "if": null, "working_directory": null, "env": {} }]
                }]
            }
        }))
        .unwrap();
        let Request::ActionsRender { workflow } = request else {
            panic!();
        };
        assert!(workflow.on.workflow_dispatch && workflow.on.pull_request.is_none());
        assert_eq!(workflow.jobs[0].permissions, Some(Permissions::ReadAll));
        assert_eq!(
            workflow.jobs[0].steps[0].uses.as_deref(),
            Some("actions/checkout@v7")
        );
    }
}
