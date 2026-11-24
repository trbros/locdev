use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Laravel,
    Wordpress,
    Php,
    Next,
    Nuxt,
    Astro,
    Sveltekit,
    /// A Vite app (React, Vue, Svelte… served by Vite itself).
    Vite,
    Node,
    Static,
    #[default]
    Unknown,
}

impl ProjectKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Laravel => "Laravel",
            Self::Wordpress => "WordPress",
            Self::Php => "PHP",
            Self::Next => "Next.js",
            Self::Nuxt => "Nuxt",
            Self::Astro => "Astro",
            Self::Sveltekit => "SvelteKit",
            Self::Vite => "Vite",
            Self::Node => "Node.js",
            Self::Static => "Static",
            Self::Unknown => "Unknown",
        }
    }
}

/// A project as stored in the registry. Removing it never touches its files.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub domain: String,
    pub path: PathBuf,
    pub kind: ProjectKind,
    /// Shell command that runs the dev server. `{port}` is replaced with `port`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default)]
    pub auto_start: bool,
    /// Node.js version pin ("22", "22.11", "22.11.0"); falls back to the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// PHP version pin ("8.3", "8.3.14"); falls back to the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub php: Option<String>,
    /// Package manager pins (`pnpm = "9"`); Node.js and PHP use the fields above.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<RuntimeKind, String>,
    /// A second dev server next to the main one (Vite beside `artisan serve`),
    /// run only with the user's consent (`mode`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dev_server: Option<DevServer>,
    /// Restart the main process when it fails (with backoff and a limit).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restart_on_failure: bool,
    /// Serve over HTTPS only: plain HTTP requests are redirected.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secure: bool,
    /// LocDev's Mailbox: point the site's mail at the capture server.
    #[serde(default, skip_serializing_if = "MailSettings::is_off")]
    pub mail: MailSettings,
    /// Which subdomains reach the site and where specific ones go.
    #[serde(
        default,
        skip_serializing_if = "crate::subdomains::Subdomains::is_default"
    )]
    pub subdomains: crate::subdomains::Subdomains,
    /// The framework adapter chosen by hand (`locdev stack use`), when
    /// detection picked another one or none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Components added by hand (`locdev stack add`), shown even when their
    /// files don't prove them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stacks: Vec<String>,
    /// Detected components the user said aren't there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_stacks: Vec<String>,
    /// Auxiliary processes (queue worker, scheduler…) and what the user
    /// decided for each, by id. Offered ones run only after a decision.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub processes: BTreeMap<String, ProjectProcess>,
    /// The parked folder this site came from (`locdev park`); it follows
    /// that folder's rules when its own folder disappears.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked: Option<PathBuf>,
    /// The deploy platform project this site is linked to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deploy: Option<crate::remote::DeployLink>,
    /// `dump()`/`dd()` go to LocDev's Debug Center (Symfony VarDumper's
    /// server format, set in the process environment only).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dumps: bool,
}

/// The user's choice about one auxiliary process.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectProcess {
    #[serde(default)]
    pub mode: DevServerMode,
    /// Replaces the offered command; required for a custom process.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// A name for a process the user added (not offered by an adapter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

/// What an auxiliary process does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    /// Consumes a queue (queue:work, Horizon, Sidekiq, Celery).
    Worker,
    /// Runs scheduled tasks (schedule:work, Celery beat).
    Scheduler,
    /// A WebSocket server (Reverb).
    Websocket,
    /// A dev tool with its own server (Storybook).
    Tool,
    #[default]
    Other,
}

/// One auxiliary process of a project: what is offered, what the user
/// chose, and how it is doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuxProcessView {
    pub id: String,
    pub name: String,
    pub kind: ProcessKind,
    /// The command it runs with (the user's, else the offered one).
    #[serde(default)]
    pub command: Option<String>,
    /// The command the adapter offers, when it does.
    #[serde(default)]
    pub offered: Option<String>,
    /// Added by the user rather than offered by an adapter.
    pub custom: bool,
    #[serde(default)]
    pub source: Option<String>,
    pub mode: DevServerMode,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub info: ProcessInfo,
}

/// An auxiliary process an adapter knows for this project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessOffer {
    pub id: String,
    pub name: String,
    pub kind: ProcessKind,
    /// `{port}` is replaced when `port` is set.
    pub command: String,
    #[serde(default)]
    pub port: bool,
    /// Alternatives that must not run together (Horizon and queue:work).
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// The component that offers it (`laravel`, `rails`…).
    pub source: String,
}

/// Tools that run a project's dev server; each takes its port differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevTool {
    Vite,
    Next,
    Nuxt,
    Astro,
}

impl DevTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vite => "Vite",
            Self::Next => "Next.js",
            Self::Nuxt => "Nuxt",
            Self::Astro => "Astro",
        }
    }
}

/// What the user decided about running a detected dev server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevServerMode {
    /// Not decided yet: LocDev asks and runs nothing.
    #[default]
    Ask,
    /// Started and stopped by hand only.
    Manual,
    /// Started together with the site.
    Auto,
    /// Never managed; no more questions.
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevServer {
    pub tool: DevTool,
    /// `{port}` is replaced with `port` at start.
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default)]
    pub mode: DevServerMode,
}

/// How far LocDev knows how to run a framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Integration {
    /// Detected, and LocDev builds the dev command with host and port.
    Native,
    /// Detected; runs one of the project's package.json scripts.
    Script,
    /// Detected, but the dev command has to come from the user.
    Custom,
    /// Nothing recognized.
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentCategory {
    #[default]
    Framework,
    /// React, Vue, Svelte… (a UI library, not a server).
    Ui,
    Language,
    Runtime,
    /// Vite, Tailwind CSS, Inertia…
    Tool,
}

/// One piece of a project's stack, with the evidence that proves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    /// Definition id (`laravel`, `react`, `typescript`).
    pub id: String,
    pub name: String,
    pub category: ComponentCategory,
    /// Installed version when known, else the declared one.
    #[serde(default)]
    pub version: Option<String>,
    /// Where the version came from (`node_modules`, `composer.lock`, `package.json (range)`…).
    #[serde(default)]
    pub version_source: Option<String>,
    /// The files or entries that matched (`artisan`, `composer.json: laravel/framework`).
    #[serde(default)]
    pub evidence: Vec<String>,
    /// Added by the user, not detected.
    #[serde(default)]
    pub manual: bool,
    /// The one that runs the dev server.
    #[serde(default)]
    pub primary: bool,
}

/// A package of a monorepo, as found in the workspace configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMember {
    /// Relative to the project (`apps/web`).
    pub path: String,
    #[serde(default)]
    pub adapter: Option<String>,
    /// "Next.js", "NestJS"… or the folder name.
    pub framework: String,
}

/// A project's makeup as read from its files.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Stack {
    pub kind: ProjectKind,
    /// Human label: "Laravel + Vite", "React + Vite", "Next.js".
    pub framework: String,
    /// The tool behind the main dev server, when it is a known one.
    #[serde(default)]
    pub dev_tool: Option<DevTool>,
    /// From the lockfile (npm when there is none).
    #[serde(default)]
    pub package_manager: Option<RuntimeKind>,
    /// The main command LocDev would use for this project today.
    #[serde(default)]
    pub command: Option<String>,
    /// A helper dev server the project could use (Vite next to Laravel).
    #[serde(default)]
    pub dev_server: Option<DevServer>,
    #[serde(default)]
    pub config_files: Vec<String>,
    /// The framework adapter that runs the project (`laravel`, `django`…).
    #[serde(default)]
    pub adapter: Option<String>,
    #[serde(default)]
    pub integration: Integration,
    /// Everything recognized, the primary one first.
    #[serde(default)]
    pub components: Vec<Component>,
    /// What to know about the command (the app must read PORT…).
    #[serde(default)]
    pub notes: Vec<String>,
    /// Extra variables that carry the port (`SERVER_PORT` for Spring Boot).
    #[serde(default)]
    pub port_env: Vec<String>,
    /// A Python virtual environment to run with (`.venv`).
    #[serde(default)]
    pub venv: Option<String>,
    /// Programs the command needs on PATH (`python3`, `bundle`…).
    #[serde(default)]
    pub requires: Vec<String>,
    /// Packages of a monorepo, each one its own site when added.
    #[serde(default)]
    pub workspaces: Vec<WorkspaceMember>,
    /// Auxiliary processes the project's components know how to run.
    #[serde(default)]
    pub processes: Vec<ProcessOffer>,
}

impl Project {
    /// The project's own pin for `kind`, if any.
    pub fn pin(&self, kind: RuntimeKind) -> Option<&str> {
        match kind {
            RuntimeKind::Node => self.node.as_deref(),
            RuntimeKind::Php => self.php.as_deref(),
            other => self.tools.get(&other).map(String::as_str),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    #[default]
    Stopped,
    Running,
    Stopping,
    Exited,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub state: ProcessState,
    pub pid: Option<u32>,
    /// Unix timestamp in milliseconds.
    pub started_at: Option<u64>,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    /// Automatic restarts since the last manual start.
    #[serde(default)]
    pub restarts: u32,
}

/// Which of a project's processes: its main dev server or the helper.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRole {
    #[default]
    Main,
    Dev,
}

impl ProcessRole {
    /// The supervisor's key for a project's process.
    pub fn id(self, project: &str) -> String {
        match self {
            Self::Main => project.to_string(),
            Self::Dev => format!("{project}:dev"),
        }
    }
}

/// A supervised process, as listed on the Processes page and by `locdev ps`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessView {
    pub project: String,
    pub role: ProcessRole,
    /// The auxiliary process id (`queue`…); `role` is then `main`.
    #[serde(default)]
    pub aux: Option<String>,
    /// "Vite", "artisan", the command's program…
    pub label: String,
    pub command: Option<String>,
    pub port: Option<u16>,
    pub info: ProcessInfo,
    /// Percent of one CPU, summed over the process tree.
    pub cpu: Option<f32>,
    /// Resident memory in bytes, summed over the process tree.
    pub memory: Option<u64>,
    /// Runtime versions it runs with ("node 22.11.0").
    pub runtimes: Vec<String>,
    /// Where its output is kept on disk.
    pub log_file: Option<PathBuf>,
}

/// A project together with its live runtime state, as returned to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectView {
    #[serde(flatten)]
    pub project: Project,
    pub process: ProcessInfo,
    /// Address through the proxy (`http://<name>.test[:port]`), if routable.
    pub url: Option<String>,
    /// The dev server itself (`http://127.0.0.1:<port>`).
    #[serde(default)]
    pub direct_url: Option<String>,
    /// HTTPS address through the proxy, when HTTPS is being served.
    #[serde(default)]
    pub https_url: Option<String>,
    pub path_exists: bool,
    /// Runtimes the project will run with (pinned or default).
    #[serde(default)]
    pub runtimes: Vec<ResolvedRuntime>,
    /// Versions the project's own files ask for (`.nvmrc`, `engines`…).
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    /// What the project is made of (re-read from its files).
    #[serde(default)]
    pub stack: Stack,
    /// The helper dev server's live state, when the project has one.
    #[serde(default)]
    pub dev_process: Option<ProcessInfo>,
    /// The helper's own address (`http://127.0.0.1:<port>`).
    #[serde(default)]
    pub dev_url: Option<String>,
}

/// A version the project's files ask for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    pub kind: RuntimeKind,
    /// As written: `20`, `lts/*`, `>=18 <23`, `^8.2`, `9.12.0`.
    pub constraint: String,
    /// Where it was found: `.nvmrc`, `package.json (engines)`…
    pub file: String,
    /// A version file (`.nvmrc`, `.php-version`…) names one version line and
    /// is followed like a pin; ranges (`engines`, `require.php`) only steer
    /// the choice among installed versions.
    pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Monotonic per-process sequence number; use it to poll for new lines.
    pub seq: u64,
    /// Unix timestamp in milliseconds.
    pub at: u64,
    pub stream: LogStream,
    pub text: String,
    /// Only set when the line says so itself (`ERROR`, `npm warn`…); plain
    /// output has no level, and stderr is not an error by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<LogLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub version: String,
    pub protocol: u32,
    pub pid: u32,
    /// Unix timestamp in milliseconds.
    pub started_at: u64,
    pub projects: usize,
    pub running: usize,
    pub home: PathBuf,
    #[serde(default)]
    pub proxy: ProxyStatus,
    #[serde(default)]
    pub dns: DnsStatus,
    #[serde(default)]
    pub tls: TlsStatus,
    #[serde(default)]
    pub mail: crate::mail::MailboxStatus,
    /// Unread, not dismissed notifications, captured emails left out (those
    /// are `mail.unread`). The tray's dot reads both.
    #[serde(default)]
    pub unread_notifications: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TlsStatus {
    pub enabled: bool,
    /// HTTPS port of the proxy; `None` when HTTPS is not being served.
    pub port: Option<u16>,
    pub requested_port: u16,
    pub error: Option<String>,
    /// The local CA certificate (PEM).
    pub ca_path: Option<PathBuf>,
    /// SHA-256 fingerprint of the CA, `AB:CD:…`.
    pub fingerprint: Option<String>,
    /// Whether the OS trust store contains the CA.
    pub trusted: bool,
    /// Whether browsers with their own store (Chrome/Firefox on Linux) trust
    /// it; `None` when there is no such store or it cannot be checked.
    pub browsers: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DnsStatus {
    pub enabled: bool,
    /// Port the DNS server listens on; `None` if it could not bind.
    pub port: Option<u16>,
    /// Loopback address it listens on (`127.0.0.1`, or LocDev's own on Windows).
    #[serde(default)]
    pub address: Option<String>,
    /// Where LocDev's installed resolver configuration sends the TLD.
    #[serde(default)]
    pub points_to: Option<String>,
    pub error: Option<String>,
    /// How this OS gets configured (`systemd-resolved`, `/etc/resolver`, ...).
    pub backend: String,
    /// Whether LocDev's resolver configuration is present; `None` if unknown.
    pub installed: Option<bool>,
    /// Observed by resolving a unique name through the OS.
    pub routing: Routing,
    /// Other tools' configuration that also claims the TLD.
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MailSettings {
    pub enabled: bool,
    /// The user agreed to replace a real SMTP/API transport while enabled.
    #[serde(default)]
    pub override_real: bool,
}

impl MailSettings {
    pub fn is_off(&self) -> bool {
        !self.enabled && !self.override_real
    }
}

impl DnsStatus {
    /// Where the DNS server listens, for the resolver configuration.
    pub fn listen(&self) -> Option<std::net::SocketAddr> {
        let ip = self
            .address
            .as_deref()
            .unwrap_or("127.0.0.1")
            .parse()
            .ok()?;
        Some(std::net::SocketAddr::new(ip, self.port?))
    }
}

/// Where `*.test` lookups made through the OS resolver end up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Routing {
    /// They reach LocDev's DNS server.
    Locdev,
    /// They resolve to this machine, but through another tool.
    Other,
    /// They do not resolve.
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub enabled: bool,
    /// Port the proxy actually listens on; `None` when it could not bind.
    pub port: Option<u16>,
    /// Port from the configuration; differs from `port` after a fallback.
    pub requested_port: u16,
    /// Why the requested port could not be used.
    pub error: Option<String>,
    /// Suffix used in site URLs: `test` when `*.test` resolves locally,
    /// otherwise `localhost`, which browsers resolve on their own.
    pub suffix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckLevel {
    Ok,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub label: String,
    pub level: CheckLevel,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

impl Check {
    pub fn new(id: &str, label: &str, level: CheckLevel, detail: impl Into<String>) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            level,
            detail: detail.into(),
            remedy: None,
        }
    }

    pub fn with_remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }
}

/// Language runtimes and the package managers that run on them. Both are
/// versioned, detected and put on a project's PATH the same way; the
/// category keeps them apart wherever they are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    Node,
    Php,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Composer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCategory {
    #[default]
    Runtime,
    PackageManager,
}

impl RuntimeKind {
    pub const ALL: [Self; 7] = [
        Self::Node,
        Self::Php,
        Self::Npm,
        Self::Pnpm,
        Self::Yarn,
        Self::Bun,
        Self::Composer,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Node => "Node.js",
            Self::Php => "PHP",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "Yarn",
            Self::Bun => "Bun",
            Self::Composer => "Composer",
        }
    }

    /// The executable a project expects to find on its PATH.
    pub fn executable(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Php => "php",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Composer => "composer",
        }
    }

    /// Bun is both a runtime and a package manager; it is listed with the
    /// package managers, next to the tools it replaces.
    pub fn category(self) -> RuntimeCategory {
        match self {
            Self::Node | Self::Php => RuntimeCategory::Runtime,
            _ => RuntimeCategory::PackageManager,
        }
    }

    /// The runtime a package manager runs on.
    pub fn requires(self) -> Option<Self> {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn => Some(Self::Node),
            Self::Composer => Some(Self::Php),
            _ => None,
        }
    }

    /// npm ships inside every Node.js release and follows its version.
    pub fn pinnable(self) -> bool {
        self != Self::Npm
    }
}

impl std::str::FromStr for RuntimeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "node" | "nodejs" | "node.js" => Ok(Self::Node),
            "php" => Ok(Self::Php),
            "npm" => Ok(Self::Npm),
            "pnpm" => Ok(Self::Pnpm),
            "yarn" => Ok(Self::Yarn),
            "bun" => Ok(Self::Bun),
            "composer" => Ok(Self::Composer),
            other => Err(format!(
                "unknown runtime '{other}' (expected node, php, npm, pnpm, yarn, bun or composer)"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSource {
    /// Downloaded and verified by LocDev.
    Managed,
    /// Already on the machine (package manager, nvm, other tools).
    System,
}

/// One usable version of a runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeVersion {
    pub version: String,
    pub source: RuntimeSource,
    /// The executable itself, e.g. `/usr/bin/php8.3`.
    pub executable: PathBuf,
    /// Where it came from: `locdev`, `nvm`, `apt`, `homebrew`, `PATH`…
    pub origin: String,
    /// Upstream support for this release line, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<Support>,
}

/// Where a release line stands in its publisher's support policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Support {
    pub state: SupportState,
    /// When the current phase ends (`YYYY-MM-DD`).
    pub until: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportState {
    /// Bug fixes and security fixes (Node.js "current" and active LTS).
    Active,
    /// Node.js maintenance LTS: critical fixes only.
    Maintenance,
    /// PHP: security fixes only.
    Security,
    /// No more fixes of any kind.
    Eol,
}

impl RuntimeVersion {
    /// Whether `version` satisfies a partial pin ("22" matches 22.11.0).
    pub fn matches(pin: &str, version: &str) -> bool {
        let parts = |v: &str| -> Vec<u64> {
            let core = v
                .trim()
                .trim_start_matches('v')
                .split(['-', '+'])
                .next()
                .unwrap_or_default();
            core.split('.').map_while(|p| p.parse().ok()).collect()
        };
        let (pin, version) = (parts(pin), parts(version));
        !pin.is_empty()
            && pin.len() <= version.len()
            && pin.iter().zip(&version).all(|(a, b)| a == b)
    }
}

/// A version available for download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    /// LTS codename (Node.js), if any.
    pub lts: Option<String>,
    pub date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Downloading,
    Verifying,
    Extracting,
    Done,
    Failed,
}

/// The default versions in the user's terminal: LocDev's shim folder and
/// whether new terminals have it first on PATH.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalRuntimes {
    /// The shim folder (`<data>/bin`).
    pub dir: PathBuf,
    /// New terminals put it first on PATH.
    pub enabled: bool,
    /// Startup files that carry LocDev's lines (bash, zsh, fish).
    #[serde(default)]
    pub files: Vec<PathBuf>,
    /// The daemon's own PATH already has it; otherwise open a new terminal.
    #[serde(default)]
    pub active: bool,
    /// Commands in it (`php`, `node`, `npm`…): one per default that resolves.
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallJob {
    pub kind: RuntimeKind,
    pub version: String,
    pub state: JobState,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<String>,
    /// What was done, step by step (sources, checksums, destination).
    #[serde(default)]
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub kind: RuntimeKind,
    /// Global default pin, used by projects without their own.
    pub default: Option<String>,
    /// Usable versions, newest first.
    pub versions: Vec<RuntimeVersion>,
    /// The current or last install of this runtime.
    pub job: Option<InstallJob>,
    /// Whether LocDev can download verified builds of this runtime here.
    pub downloads: bool,
    #[serde(default)]
    pub category: RuntimeCategory,
    /// Newer releases of installed managed lines (from the cached catalog).
    #[serde(default)]
    pub updates: Vec<RuntimeUpdate>,
    /// Which projects resolve to each version.
    #[serde(default)]
    pub usage: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeUpdate {
    /// The release line: "22" (Node.js), "8.3" (PHP).
    pub line: String,
    pub installed: String,
    pub latest: String,
}

/// The runtime a project resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRuntime {
    pub kind: RuntimeKind,
    /// The pin or range that applied.
    pub requested: String,
    /// Whether the version came from the project (its pin or its version
    /// files) rather than the global default.
    pub pinned: bool,
    /// The matching version, if one is installed.
    pub version: Option<RuntimeVersion>,
    /// `projects.toml`, the file that asked for it (`.nvmrc`…) or `default`.
    #[serde(default)]
    pub from: String,
    /// A range the project declares that no installed version satisfies,
    /// or that the chosen one does not (`engines.node: >=22`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmet: Option<Requirement>,
}

/// Today's date in UTC as `YYYY-MM-DD`.
pub fn today() -> String {
    civil_date(now_ms())
}

/// `YYYY-MM-DD` for a Unix timestamp in milliseconds (UTC).
pub fn civil_date(ms: u64) -> String {
    // Howard Hinnant's days-from-civil, inverted.
    let days = (ms / 86_400_000) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dates() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(951_782_400_000), "2000-02-29");
        assert_eq!(civil_date(1_793_577_600_000), "2026-11-02");
    }

    #[test]
    fn package_manager_pins_live_in_tools() {
        let mut project = Project {
            node: Some("22".into()),
            ..Default::default()
        };
        project.tools.insert(RuntimeKind::Pnpm, "9".into());
        assert_eq!(project.pin(RuntimeKind::Node), Some("22"));
        assert_eq!(project.pin(RuntimeKind::Pnpm), Some("9"));
        assert_eq!(project.pin(RuntimeKind::Php), None);

        let raw = toml::to_string(&project).unwrap();
        assert!(raw.contains("[tools]"), "{raw}");
        assert_eq!(toml::from_str::<Project>(&raw).unwrap(), project);
    }
}

/// A PHP extension a project needs, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtRequirement {
    pub name: String,
    /// `composer.json`, a package from composer.lock, `Laravel`, `.env (DB_CONNECTION=pgsql)`…
    pub from: String,
    /// Why it's recommended (the package's own `suggest` text).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The extensions of one PHP on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PhpModules {
    pub executable: PathBuf,
    pub version: String,
    pub origin: String,
    pub extensions: Vec<PhpModule>,
    /// In the extension folder but not loaded (enable them in php.ini).
    #[serde(default)]
    pub available: Vec<String>,
    #[serde(default)]
    pub extension_dir: Option<String>,
    /// The php.ini it read, and the extra .ini files it scanned.
    #[serde(default)]
    pub ini: Option<String>,
    #[serde(default)]
    pub scanned: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PhpModule {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    /// A Zend extension (OPcache, Xdebug).
    #[serde(default)]
    pub zend: bool,
}

/// The result of a Tinker snippet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TinkerOutput {
    pub output: String,
    #[serde(default)]
    pub code: Option<i32>,
    pub duration_ms: u64,
    #[serde(default)]
    pub timed_out: bool,
    /// Cut at the size limit.
    #[serde(default)]
    pub truncated: bool,
}

/// The extensions a project needs against what its PHP loads.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PhpExtensionsReport {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub executable: Option<PathBuf>,
    /// Where that PHP came from (`locdev`, `apt`, `homebrew`, `PATH`…).
    #[serde(default)]
    pub origin: Option<String>,
    pub loaded: Vec<String>,
    pub required: Vec<ExtRequirement>,
    pub missing: Vec<ExtRequirement>,
    /// Suggested by the installed packages (`suggest` in composer.lock) or
    /// the framework, not required; `loaded` says which it has.
    #[serde(default)]
    pub recommended: Vec<ExtRequirement>,
    /// How to get the missing ones for this PHP, when LocDev knows.
    #[serde(default)]
    pub remedy: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}
