import { t } from "@/lib/i18n";
// Mirror of `locdev_core::protocol` and `locdev_core::model`.
// Keep in sync with crates/locdev-core when the protocol changes.

export const PROTOCOL_VERSION = 2;

export type ProjectKind =
  "laravel" | "wordpress" | "php" | "next" | "nuxt" | "astro" | "sveltekit" | "vite" | "node" | "static" | "unknown";

/** Tools that run a dev server; each takes its port differently. */
export type DevTool = "vite" | "next" | "nuxt" | "astro";
/** The user's decision about a detected helper dev server. */
export type DevServerMode = "ask" | "manual" | "auto" | "never";

export interface DevServer {
  tool: DevTool;
  /** `{port}` is replaced with `port` at start. */
  command: string;
  port?: number;
  mode: DevServerMode;
}

/** A project's makeup, read from its files (nothing is run). */
export interface Stack {
  kind: ProjectKind;
  /** "Laravel + Vite", "React + Vite"… */
  framework: string;
  dev_tool: DevTool | null;
  package_manager: RuntimeKind | null;
  command: string | null;
  /** A helper the project could use (Vite next to Laravel). */
  dev_server: DevServer | null;
  config_files: string[];
  /** The framework adapter that runs it (`laravel`, `django`…). */
  adapter: string | null;
  integration: Integration;
  /** Everything recognized, the primary one first. */
  components: Component[];
  notes: string[];
  /** Extra variables carrying the port (`SERVER_PORT`, `ASPNETCORE_URLS=…`). */
  port_env: string[];
  /** A Python virtual environment it runs with. */
  venv: string | null;
  /** Programs the command needs on PATH. */
  requires: string[];
  /** Monorepo packages, each its own site. */
  workspaces: WorkspaceMember[];
  /** Auxiliary processes the components know how to run. */
  processes: ProcessOffer[];
}

export type ProcessKind = "worker" | "scheduler" | "websocket" | "tool" | "other";

export interface ProcessOffer {
  id: string;
  name: string;
  kind: ProcessKind;
  command: string;
  port: boolean;
  conflicts: string[];
  note: string | null;
  source: string;
}

export interface ProjectProcess {
  mode: DevServerMode;
  command?: string;
  label?: string;
  port?: number;
}

/** One auxiliary process: offered or added, the user's decision, its state. */
export interface AuxProcessView {
  id: string;
  name: string;
  kind: ProcessKind;
  command: string | null;
  offered: string | null;
  custom: boolean;
  source: string | null;
  mode: DevServerMode;
  port: number | null;
  conflicts: string[];
  note: string | null;
  info: ProcessInfo;
}

export type ProcessAction = "start" | "stop" | "restart";

export type ProcsRequest =
  | { type: "project_processes"; name: string }
  | { type: "control_process"; name: string; process: string; action: ProcessAction }
  | {
      type: "set_process";
      name: string;
      process: string;
      mode?: DevServerMode | null;
      command?: string | null;
      label?: string | null;
    }
  | { type: "remove_process"; name: string; process: string };

/** native: LocDev builds the command; script: a package.json script; custom: the user sets it. */
export type Integration = "native" | "script" | "custom" | "none";
export type ComponentCategory = "framework" | "ui" | "language" | "runtime" | "tool";

export interface Component {
  id: string;
  name: string;
  category: ComponentCategory;
  version: string | null;
  version_source: string | null;
  evidence: string[];
  manual: boolean;
  primary: boolean;
}

export interface WorkspaceMember {
  path: string;
  adapter: string | null;
  framework: string;
}

export type ProcessRole = "main" | "dev";

export type ProcessState = "stopped" | "running" | "stopping" | "exited" | "failed";

export interface ProcessInfo {
  state: ProcessState;
  pid: number | null;
  started_at: number | null;
  exit_code: number | null;
  message: string | null;
  /** Automatic restarts since the last manual start. */
  restarts?: number;
}

/** A supervised process, as on the Processes page and `locdev ps`. */
export interface ProcessView {
  project: string;
  role: ProcessRole;
  /** The auxiliary process id, when it is one. */
  aux: string | null;
  label: string;
  command: string | null;
  port: number | null;
  info: ProcessInfo;
  /** Percent of one CPU, over the process tree. */
  cpu: number | null;
  /** Resident bytes, over the process tree. */
  memory: number | null;
  runtimes: string[];
  log_file: string | null;
}

export interface Project {
  name: string;
  domain: string;
  path: string;
  kind: ProjectKind;
  command?: string;
  port?: number;
  auto_start: boolean;
  /** Node.js pin ("22", "20.11"); falls back to the default. */
  node?: string;
  /** PHP pin ("8.3"); falls back to the default. */
  php?: string;
  /** Package manager pins (`{ pnpm: "9" }`). */
  tools?: Partial<Record<RuntimeKind, string>>;
  /** The helper dev server, once detected or chosen. */
  dev_server?: DevServer;
  restart_on_failure?: boolean;
  /** HTTPS only: plain HTTP redirects. */
  secure?: boolean;
  /** The adapter chosen by hand. */
  adapter?: string;
  /** Components added by hand. */
  stacks?: string[];
  /** Detected components the user hid. */
  hidden_stacks?: string[];
  /** Decisions about auxiliary processes, by id. */
  processes?: Record<string, ProjectProcess>;
  /** Absent means the default: wildcard on, no rules. */
  subdomains?: Subdomains;
  /** Absent means off. */
  mail?: { enabled: boolean; override_real: boolean };
  /** The parked folder this site came from. */
  parked?: string;
  /** The deploy platform project this site is linked to. */
  deploy?: DeployLink | null;
  /** dump()/dd() go to the Debug Center. */
  dumps?: boolean;
}

// ── Deploy platforms (`locdev deploy connect/status/run…`) ────────────────────

export interface RemoteCapabilities {
  deploy: boolean;
  cancel: boolean;
  rollback: boolean;
  logs: boolean;
  env: boolean;
  domains: boolean;
}

export interface RemoteAccount {
  user: string;
  email?: string | null;
  team?: string | null;
}

export interface RemotePlatform {
  id: string;
  name: string;
  docs: string;
  token_url: string;
  token_help: string;
  capabilities: RemoteCapabilities;
  account?: RemoteAccount | null;
}

export interface RemoteProject {
  id: string;
  name: string;
  url?: string | null;
  framework?: string | null;
  repo?: string | null;
  branch?: string | null;
  dashboard: string;
}

export interface DeployLink {
  platform: string;
  project_id: string;
  project_name: string;
}

export type RemoteState = "queued" | "building" | "ready" | "error" | "canceled" | "unknown";

export interface RemoteDeployment {
  id: string;
  state: RemoteState;
  url?: string | null;
  created_at?: number | null;
  branch?: string | null;
  commit?: string | null;
  message?: string | null;
  production: boolean;
  current: boolean;
  error?: string | null;
  dashboard: string;
}

export interface RemoteStatus {
  link: DeployLink;
  platform: RemotePlatform;
  project: RemoteProject;
  deployments: RemoteDeployment[];
  domains: string[];
  env: { key: string; targets: string[] }[];
  git: { branch?: string | null; commit?: string | null; problems: string[] };
  notes: string[];
}

export interface RemoteLog {
  lines: string[];
  link?: string | null;
}

export type RemoteRequest =
  | { type: "deploy_platforms" }
  | { type: "deploy_connect"; platform: string; token: string; team?: string | null }
  | { type: "deploy_disconnect"; platform: string }
  | { type: "deploy_remote_projects"; platform: string }
  | { type: "deploy_link_site"; site: string; platform?: string | null; project?: string | null }
  | { type: "deploy_remote_status"; site: string }
  | { type: "deploy_run"; site: string; production: boolean }
  | { type: "deploy_cancel"; site: string; deployment: string }
  | { type: "deploy_rollback"; site: string; deployment: string }
  | { type: "deploy_logs"; site: string; deployment: string };

export type RemoteResponse =
  | { type: "deploy_platforms"; platforms: RemotePlatform[] }
  | { type: "remote_projects"; projects: RemoteProject[] }
  | ({ type: "remote_status" } & RemoteStatus)
  | { type: "remote_deployment"; deployment: RemoteDeployment }
  | ({ type: "remote_log" } & RemoteLog);

// ── Tunnels (`locdev tunnel`) ─────────────────────────────────────────────────

export interface TunnelProgram {
  version: string;
  path: string;
  managed: boolean;
  outdated?: string | null;
}

export interface TunnelProvider {
  id: string;
  name: string;
  description: string;
  docs: string;
  license: string;
  account: boolean;
  limits: string[];
  program?: TunnelProgram | null;
  installable: boolean;
  install_note?: string | null;
}

export type TunnelState = "starting" | "online" | "failed" | "stopped";

export interface TunnelView {
  site: string;
  provider: string;
  state: TunnelState;
  url?: string | null;
  started_at?: number | null;
  allowed: string[];
  target: string;
  error?: string | null;
}

export type TunnelRequest =
  | { type: "tunnel_providers" }
  | { type: "tunnel_install"; provider: string }
  | { type: "tunnel_list" }
  | { type: "tunnel_start"; site: string; provider?: string | null; allowed?: string[] }
  | { type: "tunnel_stop"; site: string }
  | { type: "tunnel_logs"; site: string; after?: number | null; limit?: number | null };

export type TunnelResponse =
  | { type: "tunnel_providers"; providers: TunnelProvider[] }
  | { type: "tunnels"; tunnels: TunnelView[] }
  | { type: "tunnel"; tunnel: TunnelView };

// ── Updates (`locdev update`) ────────────────────────────────────────────────

/** One published LocDev release (drafts never show up). */
export interface ReleaseInfo {
  /** `v0.14.0`: the app's version. */
  tag: string;
  /** From the release's versions.json; null when it has none. */
  app?: string | null;
  daemon?: string | null;
  published_at?: string | null;
  url: string;
  notes: string;
  prerelease: boolean;
  /** Carries the app updater's latest.json (installable, or a rollback target). */
  updater: boolean;
}

export interface UpdateStatus {
  repo: string;
  /** This daemon's (and CLI's) version. */
  daemon: string;
  latest?: ReleaseInfo | null;
  releases: ReleaseInfo[];
  daemon_update: boolean;
  checked_at?: number | null;
  error?: string | null;
}

/** Semver order of `a` over `b` (prerelease below its release); unparsable → false. */
export function isNewer(a: string, b: string): boolean {
  const parse = (v: string) => /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?/.exec(v);
  const x = parse(a);
  const y = parse(b);
  if (!x || !y) return false;
  for (let i = 1; i <= 3; i++) {
    if (Number(x[i]) !== Number(y[i])) return Number(x[i]) > Number(y[i]);
  }
  if (!x[4] || !y[4]) return !x[4] && !!y[4];
  return x[4].localeCompare(y[4], "en", { numeric: true }) > 0;
}

// ── Debug Center (`locdev debug`) ────────────────────────────────────────────

export type EventKind = "exception" | "dump" | "log" | "http" | "process" | "build" | "job";
export type EventSeverity = "debug" | "info" | "warn" | "error" | "fatal";
export type EventSource = "output" | "log_file" | "dumper" | "proxy" | "supervisor" | "jobs";
export const SEVERITY_ORDER: EventSeverity[] = ["debug", "info", "warn", "error", "fatal"];

export interface StackFrame {
  function?: string | null;
  file?: string | null;
  line?: number | null;
  column?: number | null;
  /** Dependency code (vendor, node_modules, site-packages…). */
  vendor: boolean;
}

export interface EventRequest {
  method?: string | null;
  url?: string | null;
  status?: number | null;
  duration_ms?: number | null;
  controller?: string | null;
}

export type DumpKeyKind = "key" | "public" | "protected" | "private" | "meta" | "dynamic";
export interface DumpEntry {
  key: string;
  kind: DumpKeyKind;
  value: DumpNode;
}
export type DumpNode =
  | { t: "null" }
  | { t: "bool"; v: boolean }
  | { t: "int"; v: number }
  | { t: "float"; v: number }
  | { t: "str"; v: string; cut?: number; binary?: boolean }
  | { t: "text"; v: string }
  | { t: "array"; count: number; items?: DumpEntry[]; cut?: number }
  | { t: "object"; class: string; handle?: number | null; items?: DumpEntry[]; cut?: number }
  | { t: "resource"; kind: string; items?: DumpEntry[] }
  | { t: "seen"; handle?: number | null; class?: string | null }
  | { t: "redacted" }
  | { t: "truncated" };

export interface DebugEvent {
  id: number;
  at: number;
  kind: EventKind;
  severity: EventSeverity;
  source: EventSource;
  site?: string | null;
  process?: string | null;
  message: string;
  code?: string | null;
  framework?: string | null;
  runtime?: string | null;
  file?: string | null;
  line?: number | null;
  stack?: StackFrame[];
  request?: EventRequest | null;
  dump?: DumpNode | null;
  raw?: string;
  context?: Record<string, string>;
  fingerprint: string;
  hint?: string | null;
}

export interface EventGroup {
  fingerprint: string;
  kind: EventKind;
  severity: EventSeverity;
  site?: string | null;
  message: string;
  code?: string | null;
  count: number;
  first: number;
  last: number;
  last_id: number;
  /** Per hour, last 24 hours, oldest first. */
  hourly: number[];
}

export interface EventFilter {
  site?: string | null;
  kinds?: EventKind[];
  min_severity?: EventSeverity | null;
  query?: string | null;
  after?: number | null;
  before?: number | null;
  fingerprint?: string | null;
}

export interface ObserveSettings {
  enabled: boolean;
  retention_days: number;
  max_mb: number;
  logs_from: EventSeverity;
}

export interface ObserveStatus {
  settings: ObserveSettings;
  events: number;
  disk_bytes: number;
  dump_server?: string | null;
  dump_sites: string[];
}

export type DebugRequest =
  | { type: "observe_events"; filter?: EventFilter; limit?: number | null }
  | { type: "observe_event"; id: number }
  | { type: "observe_groups"; filter?: EventFilter }
  | { type: "observe_status" }
  | { type: "observe_clear"; site?: string | null }
  | { type: "observe_dumps"; site: string; enabled: boolean };

export type DebugResponse =
  | { type: "observe_events"; events: DebugEvent[]; newest: number }
  | { type: "observe_event"; event: DebugEvent }
  | { type: "observe_groups"; groups: EventGroup[] }
  | ({ type: "observe_status" } & ObserveStatus)
  | { type: "observe_dumps"; site: string; enabled: boolean; restarted: boolean };

// ── Code editors (`locdev editor`) ───────────────────────────────────────────

export interface EditorInfo {
  id: string;
  name: string;
  /** The launcher LocDev runs. */
  program: string;
}

export interface EditorsView {
  editors: EditorInfo[];
  preferred?: string | null;
  /** What opens when none is named: the preferred one if installed, else the first found. */
  default?: string | null;
}

export type EditorRequest =
  | { type: "editor_list"; refresh?: boolean }
  | {
      type: "editor_open";
      site: string;
      /** Relative to the site's folder. */
      path?: string | null;
      line?: number | null;
      column?: number | null;
      editor?: string | null;
    };
export type EditorResponse = { type: "editors" } & EditorsView;

export type UpdateRequest = { type: "update_check"; refresh?: boolean };
export type UpdateResponse = { type: "updates" } & UpdateStatus;

/** What the app's own updater (the Tauri shell) found. */
export interface AppUpdate {
  current: string;
  bundled_daemon: string;
  /** null: nothing newer (or that release has no files for this system). */
  version: string | null;
  date: string | null;
  notes: string | null;
  downgrade: boolean;
  can_install: boolean;
  /** Why it can't: `development_build` or `linux_package`. */
  reason: "development_build" | "linux_package" | null;
  page: string;
}

export interface AppUpdateProgress {
  downloaded: number;
  total: number | null;
  done: boolean;
}

// ── PHP settings (`locdev php`) ──────────────────────────────────────────────

export type DirectiveKind = "size" | "int" | "bool" | "timezone" | "error_level";

export interface PhpDirective {
  key: string;
  label: string;
  kind: DirectiveKind;
  description: string;
  docs: string;
  /** PHP's own value (without LocDev's file). */
  original?: string | null;
  /** The value in use. */
  effective?: string | null;
  /** LocDev's value, when it sets one. */
  value?: string | null;
}

export interface PhpIniBackup {
  id: string;
  at: number;
  lines: number;
}

export interface PhpConfigView {
  line: string;
  version: string;
  executable: string;
  sapi: string;
  loaded_ini?: string | null;
  scanned: string[];
  file: string;
  text: string;
  hash: string;
  directives: PhpDirective[];
  warnings: string[];
  sites: string[];
  history: PhpIniBackup[];
}

export type PhpIniChange =
  | { kind: "set"; values: Record<string, string | null> }
  | { kind: "text"; text: string; hash: string }
  | { kind: "restore"; id: string }
  | { kind: "reset" };

export interface PhpIniPreview {
  before: string;
  after: string;
  restart: string[];
  warnings: string[];
}

export interface PhpLine {
  line: string;
  version: string;
  executable: string;
  configured: boolean;
}

export type PhpRequest =
  | { type: "php_config_lines" }
  | { type: "php_config"; line: string }
  | { type: "php_config_change"; line: string; change: PhpIniChange; dry_run?: boolean }
  | { type: "php_ini_read"; line: string; path: string };

export type PhpResponse =
  | { type: "php_lines"; lines: PhpLine[] }
  | { type: "php_config"; config: PhpConfigView; preview?: PhpIniPreview | null; restarted: string[] }
  | { type: "php_ini_file"; path: string; text: string };

// ── Parked folders (`locdev park`) ──────────────────────────────────────────

export type OnNew = "register" | "suggest";
export type OnMissing = "unregister" | "keep";
export type ParkedState = "registered" | "manual" | "suggested" | "ignored" | "failed";

export interface ParkedDir {
  path: string;
  enabled: boolean;
  /** 1 to 3 levels below the folder. */
  depth: number;
  exclude?: string[];
  /** Relative paths of projects the user ignored. */
  ignored?: string[];
  on_new: OnNew;
  on_missing: OnMissing;
}

export interface ParkedProject {
  path: string;
  rel: string;
  name: string;
  kind: ProjectKind;
  framework: string;
  state: ParkedState;
  note?: string;
}

export interface ParkedView extends ParkedDir {
  exists: boolean;
  watching: boolean;
  watch_error?: string;
  last_scan?: number;
  projects: ParkedProject[];
  errors: { path: string; message: string }[];
}

export interface ParkOptions {
  enabled?: boolean;
  depth?: number;
  exclude?: string[];
  on_new?: OnNew;
  on_missing?: OnMissing;
}

export type ParkRequest =
  | { type: "park_list" }
  | { type: "park_set"; path: string; options?: ParkOptions }
  | { type: "park_remove"; path: string; keep_sites?: boolean }
  | { type: "park_scan"; path?: string | null }
  | { type: "park_ignore"; path: string; ignore: boolean };

export type ParkResponse = { type: "parked"; dirs: ParkedView[]; added: string[]; removed: string[] };

export interface ProjectView extends Project {
  process: ProcessInfo;
  /** Through the proxy: `http://<name>.test[:port]`. */
  url: string | null;
  /** The dev server itself: `http://127.0.0.1:<port>`. */
  direct_url: string | null;
  /** HTTPS through the proxy, when HTTPS is served. */
  https_url: string | null;
  path_exists: boolean;
  /** Runtimes the project runs with (pinned or default). */
  runtimes: ResolvedRuntime[];
  /** What the project's own files ask for (`.nvmrc`, `engines`…). */
  requirements: Requirement[];
  stack: Stack;
  /** The helper's live state, when the project has one. */
  dev_process: ProcessInfo | null;
  dev_url: string | null;
}

export interface Requirement {
  kind: RuntimeKind;
  /** As written: `20`, `lts/*`, `>=18 <23`, `^8.2`. */
  constraint: string;
  /** `.nvmrc`, `package.json (engines)`… */
  file: string;
  /** Version files are followed like pins; ranges only steer the choice. */
  exact: boolean;
}

export type RuntimeKind = "node" | "php" | "npm" | "pnpm" | "yarn" | "bun" | "composer";
export type RuntimeCategory = "runtime" | "package_manager";
export type RuntimeSource = "managed" | "system";

export type SupportState = "active" | "maintenance" | "security" | "eol";

export interface Support {
  state: SupportState;
  /** End of the current phase, `YYYY-MM-DD`. */
  until: string | null;
}

export interface RuntimeVersion {
  version: string;
  source: RuntimeSource;
  executable: string;
  /** `locdev`, `nvm`, `apt`, `homebrew`, `yerd`, `PATH`, `node 22.11.0`… */
  origin: string;
  support?: Support | null;
}

export interface Release {
  version: string;
  lts: string | null;
  date: string;
}

export type JobState = "downloading" | "verifying" | "extracting" | "done" | "failed";

/** The default versions in the user's terminal (mirrors `TerminalRuntimes`). */
export interface TerminalRuntimes {
  /** LocDev's shim folder. */
  dir: string;
  /** New terminals put it first on PATH. */
  enabled: boolean;
  /** Startup files that carry LocDev's lines (bash, zsh, fish). */
  files: string[];
  /** The daemon already sees it; otherwise open a new terminal. */
  active: boolean;
  /** Commands in it: one per default that resolves. */
  commands: string[];
}

export interface InstallJob {
  kind: RuntimeKind;
  version: string;
  state: JobState;
  downloaded: number;
  total: number | null;
  error: string | null;
  /** Steps taken: sources, checksums, destination. */
  log?: string[];
}

export interface RuntimeInfo {
  kind: RuntimeKind;
  default: string | null;
  /** Newest first; managed wins over system for the same version. */
  versions: RuntimeVersion[];
  job: InstallJob | null;
  /** Whether LocDev can download verified builds of this runtime here. */
  downloads: boolean;
  category: RuntimeCategory;
  /** Newer releases of installed managed lines. */
  updates: RuntimeUpdate[];
  /** Version → sites that resolve to it. */
  usage: Record<string, string[]>;
}

export interface RuntimeUpdate {
  line: string;
  installed: string;
  latest: string;
}

export interface ResolvedRuntime {
  kind: RuntimeKind;
  requested: string;
  /** From the project (its pin or version files) rather than the default. */
  pinned: boolean;
  version: RuntimeVersion | null;
  /** `projects.toml`, `.nvmrc`, `package.json (engines)`… or `default`. */
  from: string;
  /** A declared range the chosen version doesn't satisfy. */
  unmet?: Requirement | null;
}

export const RUNTIME_LABEL: Record<RuntimeKind, string> = {
  node: "Node.js",
  php: "PHP",
  npm: "npm",
  pnpm: "pnpm",
  yarn: "Yarn",
  bun: "Bun",
  composer: "Composer",
};

/** npm ships with each Node.js and follows its version. */
export const pinnable = (kind: RuntimeKind) => kind !== "npm";

/** The runtime a package manager runs on. */
export const RUNS_ON: Partial<Record<RuntimeKind, RuntimeKind>> = {
  npm: "node",
  pnpm: "node",
  yarn: "node",
  composer: "php",
};

/**
 * The release line people pin: Node.js majors ("22"), PHP minors ("8.3",
 * since 8.2 and 8.4 differ as much as Node majors do).
 */
export function releaseLine(kind: RuntimeKind, version: string): string {
  return version
    .split(".")
    .slice(0, kind === "php" ? 2 : 1)
    .join(".");
}

/** Mirrors `RuntimeVersion::matches`: "22" matches 22.11.0. */
export function versionMatches(pin: string, version: string): boolean {
  const parts = (v: string) => {
    const out: number[] = [];
    for (const part of v.trim().replace(/^v/, "").split(/[-+]/)[0].split(".")) {
      if (!/^\d+$/.test(part)) break;
      out.push(Number(part));
    }
    return out;
  };
  const p = parts(pin);
  const v = parts(version);
  return p.length > 0 && p.length <= v.length && p.every((n, i) => n === v[i]);
}

export interface ProxyStatus {
  enabled: boolean;
  port: number | null;
  requested_port: number;
  error: string | null;
  /** `test` when *.test resolves locally, otherwise `localhost`. */
  suffix: string;
}

export type LogStream = "stdout" | "stderr" | "system";

export type LogLevel = "error" | "warn" | "info" | "debug";

export interface LogLine {
  seq: number;
  at: number;
  stream: LogStream;
  text: string;
  /** Only when the line states one itself; plain output has none. */
  level?: LogLevel;
}

export interface DaemonStatus {
  mail?: MailboxStatus;
  /** Unread notifications, captured emails left out (those are `mail.unread`). */
  unread_notifications?: number;
  version: string;
  protocol: number;
  pid: number;
  started_at: number;
  projects: number;
  running: number;
  home: string;
  proxy: ProxyStatus;
  dns: DnsStatus;
  tls: TlsStatus;
}

export interface TlsStatus {
  enabled: boolean;
  port: number | null;
  requested_port: number;
  error: string | null;
  ca_path: string | null;
  /** SHA-256 of the CA, `AB:CD:…`. */
  fingerprint: string | null;
  /** Whether the OS trust store has the CA. */
  trusted: boolean;
  /** Chrome/Firefox stores on Linux; `null` when not applicable. */
  browsers: boolean | null;
}

/** Where *.test lookups made through the OS end up. */
export type Routing = "locdev" | "other" | "none";

export interface DnsStatus {
  enabled: boolean;
  port: number | null;
  /** Loopback address it listens on (LocDev's own on Windows when 127.0.0.1:53 is taken). */
  address?: string | null;
  /** Where LocDev's installed resolver config sends *.test. */
  points_to?: string | null;
  error: string | null;
  /** How this OS is configured: `systemd-resolved`, `/etc/resolver`, `Windows NRPT`, `unsupported`. */
  backend: string;
  installed: boolean | null;
  routing: Routing;
  /** Other tools' config files that also claim *.test. */
  conflicts: string[];
}

export interface ServiceStatus {
  /** `systemd --user`, `launchd`, `Windows Run key`; empty when unsupported. */
  manager: string;
  installed: boolean;
  location: string | null;
}

export interface Script {
  shell: "sh" | "power_shell";
  source: string;
}

/** A privileged change, shown to the user before it runs. */
export interface SystemPlan {
  summary: string;
  /** Runs with administrator rights. */
  script: Script;
  /** Runs afterwards as the current user (browser stores). */
  user_script: Script | null;
}

/** `dns`: route *.test to LocDev. `tls`: trust the local CA. */
export type SystemTarget = "dns" | "tls";
export type SystemAction = "install" | "uninstall";

export type CheckLevel = "ok" | "info" | "warn" | "error";

export interface Check {
  id: string;
  label: string;
  level: CheckLevel;
  detail: string;
  remedy?: string;
}

export type ErrorCode =
  | "INCOMPATIBLE_PROTOCOL"
  | "INVALID_INPUT"
  | "NOT_FOUND"
  | "ALREADY_EXISTS"
  | "NOT_RUNNABLE"
  | "CONFLICT"
  | "UNAUTHORIZED"
  | "REMOTE"
  | "IO"
  | "CONFIG"
  | "INTERNAL";

export type Request =
  | { type: "status" }
  | { type: "shutdown" }
  /** Whether a LocDev window is in front; the daemon then leaves notifications to the app's toasts. */
  | { type: "client_presence"; focused: boolean }
  | { type: "list_starters" }
  | { type: "list_tools" }
  | { type: "install_tool"; id: string }
  | { type: "plan_project"; project: NewProject }
  | { type: "create_project"; project: NewProject }
  | { type: "search_packages"; registry: PackageRegistry; query: string }
  | { type: "list_terminals" }
  | { type: "terminal_read"; id: number; from: number }
  | { type: "terminal_write"; id: number; data: string }
  | { type: "terminal_resize"; id: number; cols: number; rows: number }
  | { type: "terminal_kill"; id: number }
  | { type: "terminal_close"; id: number }
  | { type: "doctor" }
  | { type: "check_dns" }
  | { type: "check_tls" }
  | { type: "list_runtimes"; refresh?: boolean }
  | { type: "runtime_catalog"; kind: RuntimeKind }
  | { type: "install_runtime"; kind: RuntimeKind; version: string }
  | { type: "uninstall_runtime"; kind: RuntimeKind; version: string; force?: boolean }
  | { type: "set_default_runtime"; kind: RuntimeKind; version?: string | null }
  | { type: "terminal_runtimes"; enable?: boolean | null }
  | { type: "list_projects" }
  | {
      type: "add_project";
      path: string;
      name?: string;
      command?: string;
      port?: number;
      auto_start?: boolean;
    }
  | {
      type: "update_project";
      name: string;
      command?: string;
      port?: number;
      auto_start?: boolean;
      /** "" clears the pin. */
      node?: string;
      php?: string;
      /** Package manager pins; "" clears one. */
      tools?: Partial<Record<RuntimeKind, string>>;
      restart_on_failure?: boolean;
    }
  | { type: "remove_project"; name: string }
  | { type: "start_project"; name: string }
  | { type: "stop_project"; name: string }
  | { type: "restart_project"; name: string }
  | {
      type: "project_logs";
      name: string;
      after?: number;
      limit?: number;
      process?: ProcessRole;
      /** An auxiliary process instead (`queue`…). */
      aux?: string | null;
    }
  | { type: "list_processes" }
  | { type: "set_dev_server"; name: string; mode: DevServerMode; command?: string; start?: boolean }
  | { type: "start_dev_server"; name: string }
  | { type: "stop_dev_server"; name: string }
  | { type: "restart_dev_server"; name: string }
  | { type: "check_site_https"; name: string }
  | { type: "set_secure"; name: string; secure: boolean }
  | { type: "set_subdomains"; name: string; subdomains: Subdomains }
  | { type: "explain_host"; host: string }
  | WorkRequest
  | GitRequest
  | ActionsRequest
  | EnvRequest
  | StackRequest
  | ProcsRequest
  | ServicesRequest
  | DbRequest
  | ConfigRequest
  | DeployRequest
  | MailRequest
  | EmailRequest
  | ParkRequest
  | PhpRequest
  | TunnelRequest
  | UpdateRequest
  | EditorRequest
  | DebugRequest
  | RemoteRequest
  | InsightsRequest;

export type Response =
  | InsightsResponse
  | { type: "ok" }
  | { type: "starters"; starters: Starter[] }
  | { type: "tools"; tools: Tool[] }
  | ({ type: "project_plan" } & ProjectPlan)
  | ({ type: "terminal" } & TerminalInfo)
  | { type: "terminals"; terminals: TerminalInfo[] }
  | ({ type: "terminal_output" } & TerminalOutput)
  | { type: "packages"; packages: PackageHit[] }
  | ({ type: "status" } & DaemonStatus)
  | { type: "doctor"; checks: Check[] }
  | ({ type: "dns" } & DnsStatus)
  | ({ type: "tls" } & TlsStatus)
  | { type: "runtimes"; runtimes: RuntimeInfo[] }
  | ({ type: "terminal_runtimes" } & TerminalRuntimes)
  | { type: "catalog"; releases: Release[] }
  | { type: "projects"; projects: ProjectView[] }
  | { type: "project"; project: ProjectView }
  | { type: "logs"; lines: LogLine[] }
  | { type: "processes"; processes: ProcessView[] }
  | ParkResponse
  | PhpResponse
  | TunnelResponse
  | UpdateResponse
  | EditorResponse
  | DebugResponse
  | RemoteResponse
  | WorkResponse
  | GitResponse
  | ActionsResponse
  | EnvResponse
  | StackResponse
  | { type: "project_processes"; processes: AuxProcessView[] }
  | ServicesResponse
  | DbResponse
  | ConfigResponse
  | DeployResponse
  | MailResponse
  | EmailResponse
  | ({ type: "host_explain" } & HostExplain)
  | { type: "error"; code: ErrorCode; message: string };

/** The `locdev` command bundled with the app (`locdev_platform::cli::CliStatus`). */
export interface CliStatus {
  bundled: string | null;
  version: string | null;
  /** The `locdev` a new terminal runs (first on PATH). */
  on_path: string | null;
  /** LocDev's own link, when it exists. */
  link: string | null;
  link_path: string | null;
  link_dir_on_path: boolean;
  /** Something that isn't LocDev's sits where the link would go. */
  conflict: boolean;
}

export interface AppInfo {
  version: string;
  /** The daemon/CLI this app ships. */
  bundled_daemon: string;
  arch: string;
  /** Commit (12 hex) and build time (seconds), when the build knew them. */
  commit?: string | null;
  built_at?: number | null;
  profile: "debug" | "release";
  /** Where updates come from (GitHub releases). */
  releases: string;
  protocol: number;
  os: string;
  endpoint: string;
  config_dir: string;
  log_dir: string;
}

export const KIND_LABEL: Record<ProjectKind, string> = {
  laravel: "Laravel",
  wordpress: "WordPress",
  php: "PHP",
  next: "Next.js",
  nuxt: "Nuxt",
  astro: "Astro",
  sveltekit: "SvelteKit",
  vite: "Vite",
  node: "Node.js",
  static: t("Static"),
  unknown: t("Unknown"),
};

export const DEV_TOOL_LABEL: Record<DevTool, string> = { vite: "Vite", next: "Next.js", nuxt: "Nuxt", astro: "Astro" };

// ── Dependencies, jobs, notifications, settings (locdev_core::deps) ──────────

export type Ecosystem = "npm" | "composer";
export type DepKind = "prod" | "dev" | "peer" | "optional";
export type DepStatus = "up_to_date" | "outdated" | "vulnerable" | "unknown";
export type UpdateKind = "none" | "patch" | "minor" | "major";
/** Never a guarantee: "compatible" means no check found a conflict. */
export type Compatibility = "compatible" | "possibly" | "incompatible" | "unknown";

export interface Advisory {
  id: string;
  summary: string;
  severity: string | null;
  url: string;
  fixed: string | null;
}

export interface PackageLinks {
  registry: string;
  repository: string | null;
  releases: string | null;
  homepage: string | null;
}

export interface PackageReport {
  name: string;
  ecosystem: Ecosystem;
  kind: DepKind;
  declared: string;
  locked: string | null;
  installed: string | null;
  wanted: string | null;
  latest: string | null;
  update: UpdateKind;
  status: DepStatus;
  compatibility: Compatibility;
  reasons: string[];
  advisories: Advisory[];
  links: PackageLinks;
  deprecated: string | null;
}

export interface DepsReport {
  project: string;
  manager: RuntimeKind | null;
  files: string[];
  packages: PackageReport[];
  checked_at: number | null;
  errors: string[];
}

export interface PlannedChange {
  name: string;
  ecosystem: Ecosystem;
  from: string | null;
  to: string;
  constraint: string | null;
  update: UpdateKind;
  compatibility: Compatibility;
  reasons: string[];
  links: PackageLinks;
}

export interface UpdatePlan {
  project: string;
  changes: PlannedChange[];
  /** The exact programs and arguments, in order. */
  commands: string[][];
  files: string[];
  run_scripts: boolean;
  warnings: string[];
}

export type JobKind =
  | "deps_audit"
  | "deps_update"
  | "runtime_install"
  | "github_runs"
  | "definitions_update"
  | "service_install"
  | "tunnel_install"
  | "update_check";
export type JobStatus = "pending" | "running" | "completed" | "failed" | "cancelled" | "paused";

export interface Job {
  id: number;
  kind: JobKind;
  project: string | null;
  status: JobStatus;
  automatic: boolean;
  created_at: number;
  scheduled_at: number;
  started_at: number | null;
  finished_at: number | null;
  attempts: number;
  max_attempts: number;
  progress: { done: number; total: number | null; label: string } | null;
  result: string | null;
  error: string | null;
  log: string[];
  dedup: string;
}

export const JOB_LABEL: Record<JobKind, string> = {
  deps_audit: t("Dependency check"),
  deps_update: t("Dependency update"),
  runtime_install: t("Runtime install"),
  github_runs: t("Workflow check"),
  definitions_update: t("Framework definitions check"),
  service_install: t("Service install"),
  tunnel_install: t("Tunnel program install"),
  update_check: t("Update check"),
};

/** A job's state, as shown. */
export const JOB_STATUS_LABEL: Record<JobStatus, string> = {
  pending: t("queued"),
  running: t("running"),
  completed: t("done"),
  failed: t("failed"),
  cancelled: t("cancelled"),
  paused: t("paused"),
};

export const jobFinished = (s: JobStatus) => s === "completed" || s === "failed" || s === "cancelled";

export type NotificationCategory =
  | "dependency_update"
  | "security"
  | "process_failure"
  | "runtime"
  | "certificate"
  | "job"
  | "workflow"
  | "action"
  | "mail"
  | "update";

export const CATEGORY_LABEL: Record<NotificationCategory, string> = {
  dependency_update: t("Dependency updates"),
  security: t("Security alerts"),
  process_failure: t("Process failures"),
  runtime: t("Runtimes"),
  certificate: t("Certificates"),
  job: t("Failed jobs"),
  workflow: t("Workflow failures"),
  action: t("Results of your actions"),
  mail: t("Captured emails"),
  update: t("LocDev updates"),
};

export interface Notification {
  id: number;
  category: NotificationCategory;
  severity: "info" | "warn" | "error";
  title: string;
  body: string;
  project: string | null;
  link: { view: string; sub?: string | null; project?: string | null } | null;
  created_at: number;
  read: boolean;
  dismissed: boolean;
  delivered: boolean;
  dedup: string;
}

export type AuditInterval = "off" | "manual" | "6h" | "12h" | "daily" | "weekly";

export interface Settings {
  automation: {
    audit: AuditInterval;
    concurrency: number;
    requests_per_second: number;
    definitions: boolean;
    updates: boolean;
  };
  notifications: { native: boolean; categories: Partial<Record<NotificationCategory, boolean>> };
  network: { offline: boolean };
  integrations: { github_client_id: string; github_watch_runs: boolean };
  /** An id from `locdev editor list`; absent = the first one found. */
  editor: { preferred?: string | null };
  observe: ObserveSettings;
  env: { conflicts: EnvConflictPolicy };
}

export type JobAction = "cancel" | "pause" | "resume" | "retry";

/** Requests and responses added for dependencies, jobs and notifications. */
export type WorkRequest =
  | { type: "list_deps"; name: string }
  | { type: "check_deps"; name: string }
  | { type: "plan_deps_update"; name: string; packages: string[]; allow_major?: boolean; run_scripts?: boolean }
  | { type: "update_deps"; name: string; packages: string[]; allow_major?: boolean; run_scripts?: boolean }
  | { type: "list_jobs"; project?: string | null }
  | { type: "get_job"; id: number }
  | { type: "control_job"; id: number; action: JobAction }
  | { type: "list_notifications"; all?: boolean }
  | { type: "mark_notifications"; ids?: number[]; read?: boolean; dismissed?: boolean }
  | { type: "test_notification" }
  | { type: "get_settings" }
  | { type: "update_settings"; settings: Settings };

export type WorkResponse =
  | ({ type: "deps" } & DepsReport)
  | ({ type: "deps_plan" } & UpdatePlan)
  | { type: "job"; job: Job }
  | { type: "jobs"; jobs: Job[] }
  | { type: "notifications"; notifications: Notification[]; unread: number }
  | ({ type: "settings" } & Settings);

// ── Git, security and GitHub (locdev_core::git) ─────────────────────────────

export type ChangeKind = "added" | "modified" | "deleted" | "renamed" | "copied" | "type_changed";
export type RepoState = "clean" | "merging" | "rebasing" | "cherry_picking" | "reverting" | "bisecting";

export interface FileChange {
  path: string;
  orig_path: string | null;
  staged: ChangeKind | null;
  unstaged: ChangeKind | null;
  untracked: boolean;
  conflicted: boolean;
  added: number | null;
  removed: number | null;
}

export interface GithubRepo {
  owner: string;
  name: string;
}

export interface GitStatus {
  branch: string | null;
  head: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  state: RepoState;
  changes: FileChange[];
  github: GithubRepo | null;
}

export interface Commit {
  sha: string;
  short: string;
  author: string;
  email: string;
  /** Seconds since the epoch. */
  time: number;
  refs: string[];
  subject: string;
  parents: string[];
}

export interface CommitDetail {
  commit: Commit;
  body: string;
  files: { path: string; added: number | null; removed: number | null }[];
  diff: string;
  truncated: boolean;
}

export interface Branch {
  name: string;
  remote: boolean;
  sha: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  current: boolean;
  time: number;
  subject: string;
  merged: boolean;
}

export interface Stash {
  index: number;
  message: string;
  time: number;
}

export interface Remote {
  name: string;
  fetch_url: string;
  push_url: string;
}

export interface RefLogEntry {
  sha: string;
  message: string;
  time: number;
}

export type ResetMode = "soft" | "mixed" | "hard";

export type GitOp =
  | { op: "stage"; paths: string[] }
  | { op: "unstage"; paths: string[] }
  | { op: "discard"; paths: string[] }
  | { op: "commit"; message: string }
  | { op: "create_branch"; name: string; start?: string | null; switch?: boolean }
  | { op: "switch"; name: string }
  | { op: "delete_branch"; name: string; force?: boolean }
  | { op: "merge"; branch: string }
  | { op: "abort_merge" }
  | { op: "fetch"; remote?: string | null }
  | { op: "pull" }
  | { op: "push"; force?: boolean }
  | { op: "stash_push"; message?: string | null; untracked?: boolean }
  | { op: "stash_apply"; index: number }
  | { op: "stash_pop"; index: number }
  | { op: "stash_drop"; index: number }
  | { op: "remote_add"; name: string; url: string }
  | { op: "remote_remove"; name: string }
  | { op: "remote_set_url"; name: string; url: string }
  | { op: "revert"; sha: string }
  | { op: "restore_files"; sha: string; paths: string[] }
  | { op: "reset"; target: string; mode: ResetMode }
  | {
      op: "worktree_add";
      branch: string;
      create?: boolean;
      start?: string | null;
      /** Next to the repository (`<repo>-<branch>`) when omitted. */
      path?: string | null;
      site?: boolean;
      copy_env?: boolean;
    }
  | { op: "worktree_remove"; path: string; force?: boolean }
  | { op: "worktree_prune" };

export interface Worktree {
  path: string;
  branch: string | null;
  head: string;
  main: boolean;
  locked: boolean;
  prunable: boolean;
  /** The LocDev site registered for this folder. */
  site: string | null;
}

export interface DestructivePreview {
  summary: string;
  lost: string[];
  /** Text the user must type to confirm. */
  phrase: string;
  /** A LocDev password is set and required. */
  password: boolean;
  backup: string | null;
  /** Why it can't run right now. */
  blocked: string | null;
}

export interface Confirmation {
  phrase: string;
  password?: string | null;
}

export interface GitResult {
  message: string;
  output: string;
  backup: string | null;
}

export interface SecurityStatus {
  password_set: boolean;
  require_password: boolean;
  backups: boolean;
  locked_until: number | null;
}

export type GithubAuthMethod = "device" | "token" | "gh_cli";

export interface GithubAccount {
  login: string;
  name: string | null;
  method: GithubAuthMethod;
  scopes: string[];
  storage: string;
}

export interface DeviceCode {
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface GithubOverview {
  repo: GithubRepo;
  description: string | null;
  private: boolean;
  default_branch: string;
  html_url: string;
  open_issues: number;
  stars: number;
  rate_remaining: number | null;
}

export interface GithubItem {
  number: number;
  title: string;
  state: string;
  author: string;
  url: string;
  created_at: string;
  updated_at: string;
  labels: string[];
  branch: string | null;
  draft: boolean;
}

export interface WorkflowRun {
  id: number;
  name: string;
  branch: string;
  event: string;
  status: string;
  conclusion: string | null;
  sha: string;
  url: string;
  created_at: string;
  workflow_id: number;
}

export interface Workflow {
  id: number;
  name: string;
  path: string;
  state: string;
  url: string;
}

export interface GithubRelease {
  tag: string;
  name: string;
  draft: boolean;
  prerelease: boolean;
  published_at: string | null;
  url: string;
}

export interface CheckRun {
  name: string;
  status: string;
  conclusion: string | null;
  url: string;
}

export type GithubSection =
  "overview" | "issues" | "pulls" | "workflows" | "runs" | "releases" | "branches" | "commits" | "checks";

export type GithubData =
  | ({ section: "overview" } & GithubOverview)
  | { section: "issues"; items: GithubItem[] }
  | { section: "pulls"; items: GithubItem[] }
  | { section: "workflows"; workflows: Workflow[] }
  | { section: "runs"; runs: WorkflowRun[] }
  | { section: "releases"; releases: GithubRelease[] }
  | { section: "branches"; names: string[] }
  | { section: "commits"; commits: Commit[] }
  | { section: "checks"; sha: string; checks: CheckRun[] };

export type GithubAction =
  | { action: "rerun_failed"; run_id: number }
  | { action: "cancel_run"; run_id: number }
  | { action: "dispatch"; workflow_id: number; git_ref: string };

export type GitRequest =
  | { type: "git_status"; name: string }
  | { type: "git_diff"; name: string; path: string; staged?: boolean }
  | { type: "git_log"; name: string; rev?: string | null; skip?: number; limit?: number | null }
  | { type: "git_show"; name: string; sha: string }
  | { type: "git_branches"; name: string }
  | { type: "git_stashes"; name: string }
  | { type: "git_remotes"; name: string }
  | { type: "git_worktrees"; name: string }
  | { type: "git_history"; name: string }
  | { type: "git_preview"; name: string; op: GitOp }
  | { type: "git"; name: string; op: GitOp; confirm?: Confirmation | null }
  | { type: "get_security" }
  | { type: "set_password"; current?: string | null; new?: string | null }
  | { type: "set_security_options"; password?: string | null; require_password: boolean; backups: boolean }
  | { type: "github_account" }
  | { type: "github_connect_token"; token: string }
  | { type: "github_connect_gh_cli" }
  | { type: "github_device_start" }
  | { type: "github_device_poll" }
  | { type: "github_disconnect" }
  | { type: "github"; name: string; section: GithubSection }
  | { type: "github_action"; name: string; action: GithubAction };

export type GitResponse =
  | ({ type: "git_status" } & GitStatus)
  | { type: "text"; text: string; truncated: boolean }
  | { type: "commits"; commits: Commit[] }
  | ({ type: "commit" } & CommitDetail)
  | { type: "branches"; branches: Branch[] }
  | { type: "stashes"; stashes: Stash[] }
  | { type: "worktrees"; worktrees: Worktree[] }
  | { type: "remotes"; remotes: Remote[] }
  | { type: "git_history"; reflog: RefLogEntry[]; operations: string[] }
  | { type: "git_preview"; preview: DestructivePreview | null }
  | ({ type: "git_result" } & GitResult)
  | ({ type: "security" } & SecurityStatus)
  | { type: "github"; account: GithubAccount | null }
  | ({ type: "device_code" } & DeviceCode)
  | ({ type: "github_data" } & GithubData);

// ── GitHub Actions builder (locdev_core::actions) ───────────────────────────

export interface WorkflowFilter {
  branches: string[];
  tags: string[];
  paths: string[];
}

export type Permissions =
  { kind: "read_all" } | { kind: "write_all" } | { kind: "scopes"; scopes: Record<string, string> };

export interface WorkflowStep {
  id: string | null;
  name: string | null;
  uses: string | null;
  with: Record<string, string>;
  run: string | null;
  if: string | null;
  working_directory: string | null;
  env: Record<string, string>;
}

export interface WorkflowJob {
  id: string;
  name: string | null;
  runs_on: string;
  needs: string[];
  if: string | null;
  permissions: Permissions | null;
  environment: string | null;
  timeout_minutes: number | null;
  matrix: Record<string, string[]>;
  env: Record<string, string>;
  steps: WorkflowStep[];
}

export interface WorkflowModel {
  name: string;
  on: {
    push: WorkflowFilter | null;
    pull_request: WorkflowFilter | null;
    workflow_dispatch: boolean;
    schedule: string[];
    release: boolean;
  };
  permissions: Permissions | null;
  concurrency: { group: string; cancel_in_progress: boolean } | null;
  env: Record<string, string>;
  jobs: WorkflowJob[];
}

export interface TemplateInfo {
  id: string;
  title: string;
  description: string;
  file: string;
  recommended: boolean;
  reason: string;
}

export interface ActionVersion {
  repo: string;
  version: string;
  purpose: string;
}

export type FindingLevel = "info" | "warning" | "error";

export interface Finding {
  level: FindingLevel;
  rule: string;
  message: string;
  line: number | null;
  source: "locdev" | "actionlint";
  /** YAML that applies the suggestion. */
  fix?: string | null;
}

export interface WorkflowAudit {
  file: string;
  name: string | null;
  findings: Finding[];
}

export interface WorkflowVersion {
  id: string;
  at: number;
  /** `read` (changed outside LocDev), `before-save`, `saved`. */
  reason: string;
  size: number;
}

export interface Validation {
  findings: Finding[];
  actionlint: string | null;
  workflow: WorkflowModel | null;
  unsupported: string | null;
}

export interface SavePlan {
  path: string;
  exists: boolean;
  current_hash: string | null;
  diff: string;
  validation: Validation;
}

export type ActionsRequest =
  | { type: "actions_overview"; name: string }
  | { type: "actions_template"; name: string; template: string }
  | { type: "actions_render"; workflow: WorkflowModel }
  | { type: "actions_validate"; yaml: string }
  | { type: "actions_read"; name: string; file: string }
  | { type: "actions_plan"; name: string; file: string; yaml: string }
  | { type: "actions_save"; name: string; file: string; yaml: string; replace?: string | null }
  | { type: "action_versions" }
  | { type: "refresh_action_versions" }
  | { type: "actions_audit"; name: string }
  | { type: "actions_history"; name: string; file: string }
  | { type: "actions_history_version"; name: string; file: string; id: string }
  | { type: "actionlint_install" };

export type ActionsResponse =
  | {
      type: "actions_overview";
      templates: TemplateInfo[];
      existing: { file: string; name: string | null }[];
      actions: ActionVersion[];
      versions_checked_at: number | null;
      actionlint: string | null;
    }
  | { type: "actions_workflow"; workflow: WorkflowModel; file: string }
  | { type: "actions_yaml"; yaml: string; validation: Validation }
  | ({ type: "actions_validation" } & Validation)
  | ({ type: "actions_plan" } & SavePlan)
  | { type: "actions_saved"; path: string }
  | { type: "action_versions"; actions: ActionVersion[]; checked_at: number | null }
  | { type: "actions_audit"; files: WorkflowAudit[]; actionlint: string | null }
  | { type: "actions_history"; versions: WorkflowVersion[] };

export const emptyStep = (): WorkflowStep => ({
  id: null,
  name: null,
  uses: null,
  with: {},
  run: "",
  if: null,
  working_directory: null,
  env: {},
});

// ── Shared environment variables (locdev_core::env) ─────────────────────────

export const GLOBAL_PROFILE = "global";

/** Prefixes frameworks inline into browser bundles; a secret can't use one. */
export const PUBLIC_PREFIXES = [
  "NEXT_PUBLIC_",
  "VITE_",
  "NUXT_PUBLIC_",
  "PUBLIC_",
  "REACT_APP_",
  "GATSBY_",
  "EXPO_PUBLIC_",
  "VUE_APP_",
];

export const publicPrefix = (key: string) => PUBLIC_PREFIXES.find((p) => key.startsWith(p)) ?? null;

export type EnvConflictPolicy = "active_files" | "any_file";

export interface EnvProfile {
  id: string;
  label: string;
  description: string;
}

/** `value` is null for secrets unless one was revealed. */
export interface EnvVar {
  key: string;
  profile: string;
  secret: boolean;
  value: string | null;
  description: string;
  updated_at: number;
}

export interface ProjectEnvSettings {
  inherit_global: boolean;
  profiles: string[];
  disabled: string[];
}

export interface VaultStatus {
  available: boolean;
  backend: string;
  error: string | null;
  secrets: number;
}

export interface EnvHistoryEntry {
  at: number;
  action: string;
  key: string | null;
  profile: string | null;
  project: string | null;
  detail: string | null;
}

export type EntryStatus = "injected" | "skipped" | "overridden" | "disabled" | "blocked";

export interface EnvEntry {
  key: string;
  profile: string;
  secret: boolean;
  status: EntryStatus;
  reason: string | null;
  value: string | null;
}

export type EnvFileRole = "base" | "local" | "mode" | "mode_local" | "template";

export interface EnvFileInfo {
  file: string;
  role: EnvFileRole;
  mode: string | null;
  active: boolean;
  rank: number | null;
  note: string;
  keys: number;
  duplicates: string[];
  error: string | null;
}

export type ConflictSource =
  | { kind: "file"; file: string; active: boolean }
  | { kind: "system" }
  | { kind: "locdev" }
  | { kind: "profile"; profile: string };

export interface EnvConflict {
  key: string;
  profile: string;
  source: ConflictSource;
  injected: boolean;
  effect: string;
}

export interface ProjectEnvReport {
  project: string;
  framework: string;
  settings: ProjectEnvSettings;
  entries: EnvEntry[];
  files: EnvFileInfo[];
  conflicts: EnvConflict[];
  notes: string[];
  restart_needed: boolean;
}

export interface SyncPlan {
  project: string;
  file: string;
  exists: boolean;
  add: { key: string; profile: string; secret: boolean; placeholder: boolean }[];
  skip: { key: string; reason: string }[];
  preview: string;
  token: string;
  refused: string | null;
}

export interface SyncResult {
  project: string;
  file: string;
  added: string[];
  backup: string | null;
}

export interface EnvBackup {
  id: string;
  project: string;
  file: string;
  at: number;
  size: number;
}

export type EnvRequest =
  | { type: "env_overview" }
  | {
      type: "env_set";
      key: string;
      profile: string;
      value?: string | null;
      secret?: boolean | null;
      description?: string | null;
    }
  | { type: "env_remove"; key: string; profile: string }
  | { type: "env_reveal"; key: string; profile: string; password?: string | null }
  | { type: "env_save_profile"; id: string; label: string; description: string }
  | { type: "env_remove_profile"; id: string }
  | { type: "env_project"; name: string }
  | { type: "env_set_project"; name: string; settings: ProjectEnvSettings }
  | { type: "env_sync_plan"; name?: string | null; file?: string | null }
  | { type: "env_sync"; name: string; file?: string | null; token: string }
  | { type: "env_backups"; name: string }
  | { type: "env_restore"; name: string; id: string };

export type EnvResponse =
  | {
      type: "env_overview";
      profiles: EnvProfile[];
      vars: EnvVar[];
      vault: VaultStatus;
      history: EnvHistoryEntry[];
    }
  | { type: "env_var"; var: EnvVar }
  | { type: "env_value"; value: string }
  | { type: "env_profile"; profile: EnvProfile }
  | ({ type: "env_project" } & ProjectEnvReport)
  | { type: "env_sync_plans"; plans: SyncPlan[] }
  | ({ type: "env_synced" } & SyncResult)
  | { type: "env_backups"; backups: EnvBackup[] };

// ── Framework adapters (locdev_core::frameworks) ────────────────────────────

export interface DefinitionInfo {
  id: string;
  name: string;
  category: ComponentCategory;
  integration: Integration;
  docs: string;
  runtimes: string[];
  requires: string[];
  /** Dev command templates; `<pm>` is the project's package manager. */
  dev: string[];
  install: string | null;
  build: string | null;
  test: string | null;
  supported: string | null;
  config_files: string[];
  services: string[];
  notes: string[];
}

export interface RegistryStatus {
  origin: "bundled" | "downloaded";
  revision: number;
  bundled_revision: number;
  format: number;
  count: number;
  url: string;
  checked_at: number | null;
  error: string | null;
  previous: number | null;
  skipped: number | null;
}

export interface StackReport {
  target: string;
  path: string;
  stack: Stack;
  command: string | null;
  prerequisites: { program: string; found: string | null }[];
  definition: DefinitionInfo | null;
  install: string | null;
  build: string | null;
  test: string | null;
  adapter_override: string | null;
  added: string[];
  hidden: string[];
}

export type StackRequest =
  | { type: "stack_definitions" }
  | { type: "stack_detect"; name?: string | null; path?: string | null }
  | {
      type: "stack_set";
      name: string;
      add?: string[];
      remove?: string[];
      adapter?: string | null;
      apply_command?: boolean;
    }
  | { type: "refresh_definitions"; force?: boolean }
  | { type: "rollback_definitions" };

export type StackResponse =
  | { type: "stack_definitions"; definitions: DefinitionInfo[]; registry: RegistryStatus }
  | ({ type: "stack_report" } & StackReport);

export const INTEGRATION_LABEL: Record<Integration, string> = {
  native: t("native"),
  script: t("package.json script"),
  custom: t("custom command"),
  none: t("not recognized"),
};

// ── Services (locdev_core::services) ─────────────────────────────────────────

export type ServiceKind = "postgres" | "mariadb" | "mysql" | "mongodb" | "valkey" | "redis" | "sqlite";

export const SERVICE_LABEL: Record<ServiceKind, string> = {
  postgres: "PostgreSQL",
  mariadb: "MariaDB",
  mysql: "MySQL",
  mongodb: "MongoDB",
  valkey: "Valkey",
  redis: "Redis",
  sqlite: "SQLite",
};

export interface ServiceInstall {
  kind: ServiceKind;
  version: string;
  source: "managed" | "system";
  bin_dir: string;
}

export interface ServiceInfo {
  kind: ServiceKind;
  name: string;
  default_port: number | null;
  installs: ServiceInstall[];
  no_download: string | null;
  notes: string[];
  /** Versions the system's package manager offers (installed in a terminal). */
  packages?: ServicePackage[];
}

export interface ServicePackage {
  id: string;
  version: string;
  manager: string;
  commands: string[][];
}

export interface ServiceRelease {
  kind: ServiceKind;
  version: string;
  publisher: string;
  url: string;
}

export interface InstanceView {
  name: string;
  kind: ServiceKind;
  version: string;
  source: "managed" | "system";
  port: number;
  autostart: boolean;
  created_at: number;
  info: ProcessInfo;
  data_dir: string;
  ready: boolean;
  connection: [string, string][];
  log_file: string | null;
}

export interface InstanceBackup {
  id: string;
  instance: string;
  at: number;
  size: number;
  path: string;
}

export type ServicesRequest =
  | { type: "list_services"; refresh?: boolean }
  | { type: "service_releases"; kind: ServiceKind }
  | { type: "install_service"; kind: ServiceKind; version: string }
  | { type: "install_service_package"; kind: ServiceKind; id: string }
  | { type: "uninstall_service"; kind: ServiceKind; version: string }
  | {
      type: "create_instance";
      name: string;
      kind: ServiceKind;
      version?: string | null;
      port?: number | null;
      autostart?: boolean;
    }
  | { type: "control_instance"; name: string; action: ProcessAction }
  | { type: "set_instance"; name: string; autostart: boolean }
  | { type: "remove_instance"; name: string; delete_data?: boolean; confirm?: string | null }
  | { type: "backup_instance"; name: string }
  | { type: "instance_backups"; name: string }
  | { type: "restore_instance"; name: string; id: string; confirm?: string | null }
  | { type: "instance_logs"; name: string; after?: number; limit?: number };

export type ServicesResponse =
  | { type: "services"; services: ServiceInfo[]; instances: InstanceView[] }
  | { type: "service_releases"; releases: ServiceRelease[] }
  | ({ type: "instance" } & InstanceView)
  | ({ type: "instance_backup" } & InstanceBackup)
  | { type: "instance_backups"; backups: InstanceBackup[] };

// ── Database manager (locdev_core::db) ──────────────────────────────────────

export type DbEngine = "postgres" | "mysql" | "sqlite" | "redis";

/** Never holds a secret: the daemon resolves credentials from the id. */
export interface DbConnection {
  id: string;
  label: string;
  engine: DbEngine;
  host: string | null;
  port: number | null;
  database: string | null;
  username: string | null;
  has_password: boolean;
  source: string;
  project: string | null;
  remote: boolean;
}

export interface DbTable {
  name: string;
  schema: string;
  kind: string;
  rows_estimate: number | null;
}

export interface DbColumn {
  name: string;
  data_type: string;
  nullable: boolean;
  default: string | null;
  primary_key: boolean;
}

export interface DbTableInfo {
  table: DbTable;
  columns: DbColumn[];
  indexes: { name: string; columns: string[]; unique: boolean }[];
  foreign_keys: { name: string; columns: string[]; references_table: string; references_columns: string[] }[];
}

export interface DbFilter {
  column: string;
  op: "=" | "!=" | "<" | ">" | "<=" | ">=" | "like" | "is_null" | "not_null";
  value?: string | null;
}

export interface DbRows {
  columns: string[];
  rows: (string | null)[][];
  total: number | null;
  affected: number | null;
  truncated: boolean;
  elapsed_ms: number;
}

export interface QueryCheck {
  destructive: string[];
  writes: boolean;
  confirm: string | null;
}

export interface RedisKey {
  key: string;
  kind: string;
  ttl: number;
}

export interface RedisValue {
  key: string;
  kind: string;
  ttl: number;
  items: [string, string | null][];
  truncated: boolean;
}

export type RowEdit =
  | { op: "update"; key: [string, string | null][]; changes: [string, string | null][] }
  | { op: "insert"; values: [string, string | null][] }
  | { op: "delete"; key: [string, string | null][] };

export type RedisEdit =
  | { op: "set"; key: string; value: string }
  | { op: "expire"; key: string; seconds: number | null }
  | { op: "delete"; key: string };

export type DbRequest =
  | { type: "db_connections"; name?: string | null }
  | { type: "db_schema"; connection: string; allow_remote?: boolean }
  | { type: "db_table_info"; connection: string; table: string; schema?: string; allow_remote?: boolean }
  | {
      type: "db_rows";
      connection: string;
      table: string;
      schema?: string;
      page?: number;
      page_size?: number | null;
      order_by?: string | null;
      desc?: boolean;
      filters?: DbFilter[];
      allow_remote?: boolean;
    }
  | { type: "db_check"; connection: string; sql: string }
  | { type: "db_query"; connection: string; sql: string; confirm?: string | null; allow_remote?: boolean }
  | { type: "db_edit_row"; connection: string; table: string; schema?: string; edit: RowEdit; allow_remote?: boolean }
  | { type: "db_create_database"; connection: string; database?: string | null }
  | { type: "redis_keys"; connection: string; cursor?: string | null; pattern?: string | null; allow_remote?: boolean }
  | { type: "redis_get"; connection: string; key: string; allow_remote?: boolean }
  | { type: "redis_edit"; connection: string; edit: RedisEdit; allow_remote?: boolean };

export type DbResponse =
  | { type: "db_connections"; connections: DbConnection[] }
  | { type: "db_schema"; databases: string[]; tables: DbTable[] }
  | ({ type: "db_table_info" } & DbTableInfo)
  | ({ type: "db_rows" } & DbRows)
  | ({ type: "db_check" } & QueryCheck)
  | { type: "db_affected"; rows: number }
  | { type: "redis_keys"; cursor: string; keys: RedisKey[] }
  | ({ type: "redis_value" } & RedisValue);

// Config Files (crates/locdev-core/src/config.rs). JavaScript configs are
// parsed by the daemon, never executed; saves name the hash that was reviewed.

export type ConfigFormat = "json" | "jsonc" | "js" | "xml" | "toml" | "yaml" | "css";
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export interface ConfigFile {
  path: string;
  kind: string;
  label: string;
  format: ConfigFormat;
  package: string | null;
  version: string | null;
  docs: string;
  visual: boolean;
}

export type OptionKind = "bool" | "number" | "string" | "choice";
export type OptionState = { state: "set"; value: JsonValue } | { state: "code"; code: string } | { state: "missing" };

export interface ConfigOption {
  key: string;
  section: string;
  label: string;
  description: string;
  kind: OptionKind;
  choices: JsonValue[];
  default: JsonValue | null;
  example: string | null;
  docs: string | null;
  versions: string | null;
  state: OptionState;
  editable: boolean;
  blocked: string | null;
}

export interface ConfigFileView {
  file: ConfigFile;
  content: string;
  hash: string;
  options: ConfigOption[];
  notes: string[];
  visual_blocked: string | null;
  syntax_error: string | null;
}

export type ConfigChange = { type: "set"; key: string; value: JsonValue } | { type: "text"; content: string };

export interface ConfigPreview {
  content: string;
  diff: string;
  syntax_error: string | null;
  removed_comments: string[];
}

export interface ConfigHistoryEntry {
  id: string;
  at: number;
  size: number;
  reason: string;
}

export type FindingKind = "incompatible" | "deprecated" | "security" | "performance" | "development" | "available";

export interface ConfigFinding {
  rule: string;
  kind: FindingKind;
  file: string;
  key: string | null;
  title: string;
  detail: string;
  applies: string;
  source: string;
  fix: [string, JsonValue] | null;
}

export type ConfigRequest =
  | { type: "config_files"; name: string }
  | { type: "config_file"; name: string; path: string }
  | { type: "config_preview"; name: string; path: string; change: ConfigChange }
  | { type: "config_save"; name: string; path: string; change: ConfigChange; base_hash: string }
  | { type: "config_history"; name: string; path: string }
  | { type: "config_revert"; name: string; path: string; id: string; base_hash: string }
  | { type: "config_history_version"; name: string; path: string; id: string }
  | { type: "config_check"; name: string };

export type ConfigResponse =
  | { type: "config_files"; files: ConfigFile[] }
  | ({ type: "config_file" } & ConfigFileView)
  | ({ type: "config_preview" } & ConfigPreview)
  | { type: "config_saved"; hash: string }
  | { type: "config_history"; entries: ConfigHistoryEntry[] }
  | { type: "config_history_version"; content: string }
  | { type: "config_findings"; findings: ConfigFinding[] };

// Deploy assistant (crates/locdev-core/src/deploy.rs). Recommendations and
// preparation only: nothing is sent to any platform; variables by name only.

export type Rendering = "static" | "spa" | "ssr" | "server";
export type Verdict = "fits" | "needs_changes" | "not_suitable";
export type ReadinessLevel = "ok" | "warn" | "fail";

export interface DeployNeed {
  id: string;
  label: string;
  evidence: string;
}

export interface EnvNeed {
  name: string;
  secret: boolean;
  note: string;
}

export interface DeployProfile {
  framework: string;
  version: string | null;
  runtime: string;
  runtime_version: string | null;
  rendering: Rendering;
  rendering_evidence: string;
  needs: DeployNeed[];
  build_command: string | null;
  start_command: string | null;
  output_dir: string | null;
  env: EnvNeed[];
}

export interface DeployPoint {
  text: string;
  source: string | null;
}

export interface ProviderFit {
  id: string;
  name: string;
  model: string;
  verdict: Verdict;
  summary: string;
  strengths: DeployPoint[];
  limitations: DeployPoint[];
  requirements: DeployPoint[];
  docs: string;
  config_file: string | null;
  setup: string | null;
}

export interface ReadinessCheck {
  id: string;
  level: ReadinessLevel;
  title: string;
  detail: string;
}

export interface DeployReport {
  profile: DeployProfile;
  providers: ProviderFit[];
  checks: ReadinessCheck[];
  reviewed: string;
}

export interface DeployConfig {
  provider: string;
  file: string;
  content: string;
  hash: string;
  exists: boolean;
  notes: string[];
}

export type DeployRequest =
  | { type: "deploy_analysis"; name: string }
  | { type: "deploy_config"; name: string; provider: string }
  | { type: "deploy_write_config"; name: string; provider: string; hash: string };

export type DeployResponse =
  | ({ type: "deploy_report" } & DeployReport)
  | ({ type: "deploy_config" } & DeployConfig)
  | { type: "deploy_config_written"; file: string };

// Subdomains (crates/locdev-core/src/subdomains.rs). A pattern is the part
// before the site's domain; a leading `*` is one or more labels.

export type SubdomainTarget = { type: "site" } | { type: "port"; port: number } | { type: "process"; id: string };

export interface SubdomainRule {
  pattern: string;
  target: SubdomainTarget;
}

export interface Subdomains {
  wildcard: boolean;
  rules: SubdomainRule[];
}

export interface HostExplain {
  host: string;
  dns: { answers: string[]; via_locdev: boolean | null; error: string | null };
  route: {
    site: string | null;
    sub: string;
    rule: string | null;
    target: string;
    port: number | null;
    reachable: boolean | null;
    problem: string | null;
  };
  tls: { names: string[]; https_port: number | null; trusted: boolean };
}

// Mailbox (crates/locdev-core/src/mail.rs). Capture-only SMTP on loopback.

export interface MailboxStatus {
  enabled: boolean;
  port: number | null;
  error: string | null;
  messages: number;
  unread: number;
}

export interface MailEnvelope {
  from: string;
  to: string[];
  helo: string | null;
  auth_user: string | null;
  protocol: string;
  peer: string;
  received_at: number;
}

export interface MailSummary {
  id: string;
  site: string | null;
  from: string;
  to: string[];
  subject: string;
  date: number | null;
  received_at: number;
  size: number;
  attachments: number;
  has_html: boolean;
  preview: string;
  read: boolean;
}

export interface MailHeader {
  name: string;
  value: string;
}

export interface MailAttachment {
  index: number;
  name: string | null;
  content_type: string;
  size: number;
  content_id: string | null;
  inline: boolean;
}

export interface MailDetail {
  summary: MailSummary;
  envelope: MailEnvelope;
  headers: MailHeader[];
  cc: string[];
  reply_to: string[];
  html: string | null;
  text: string | null;
  text_generated: boolean;
  attachments: MailAttachment[];
  links: { url: string; text: string | null }[];
  problems: string[];
  framework: string | null;
}

export interface MailSetup {
  site: string;
  framework: string;
  enabled: boolean;
  wiring: "automatic" | "manual";
  current: string[];
  real_smtp: boolean;
  injects: MailHeader[];
  snippet: string | null;
  notes: string[];
}

export type MailRequest =
  | { type: "mailbox_status" }
  | {
      type: "mail_list";
      site?: string | null;
      query?: string | null;
      unread?: boolean;
      attachments?: boolean;
      limit?: number | null;
    }
  | { type: "mail_get"; id: string }
  | { type: "mail_raw"; id: string }
  | { type: "mail_attachment"; id: string; index: number }
  | { type: "mail_delete"; ids: string[] }
  | { type: "mail_clear"; site?: string | null }
  | { type: "mail_setup"; name: string }
  | { type: "mail_enable"; name: string; enabled: boolean; override_real?: boolean }
  | { type: "mail_send_test"; name?: string | null; to?: string | null };

export type MailResponse =
  | ({ type: "mailbox_status" } & MailboxStatus)
  | { type: "mail_list"; messages: MailSummary[] }
  | ({ type: "mail_detail" } & MailDetail)
  | { type: "mail_raw"; base64: string }
  | { type: "mail_attachment"; name: string | null; content_type: string; base64: string }
  | { type: "mail_deleted"; count: number }
  | ({ type: "mail_setup" } & MailSetup);

// Email Studio (crates/locdev-core/src/email.rs). LocDev blocks live between
// comment markers; everything else is code, kept exactly as written.

export type EmailFormat = "blade" | "markdown" | "text" | "html" | "mjml" | "react" | "erb" | "django";

export interface MailableInfo {
  class: string;
  path: string;
  variables: string[];
  subject: string | null;
}

export interface EmailTemplate {
  path: string;
  name: string;
  format: EmailFormat;
  mailables: MailableInfo[];
  has_blocks: boolean;
}

export interface EmailComponent {
  name: string;
  path: string;
  usage: string;
  props: { name: string; default: string | null }[];
  slot: boolean;
  description: string | null;
  builtin: boolean;
}

export interface EmailCatalog {
  framework: string;
  templates: EmailTemplate[];
  components: EmailComponent[];
  conventions: string[];
  native: string[];
  framework_preview: boolean;
}

export type Segment =
  | { kind: "block"; id: string; block: string; props: Record<string, JsonValue>; edited: boolean }
  | { kind: "code"; id: string; code: string };

export interface EmailDoc {
  path: string;
  format: EmailFormat;
  content: string;
  hash: string;
  segments: Segment[];
  theme: Record<string, JsonValue>;
  variables: string[];
  sample: JsonValue;
  notes: string[];
}

export type PreviewMode = "static" | "framework";

export interface EmailPreview {
  html: string;
  text: string | null;
  content: string;
  notes: string[];
  error: string | null;
}

export interface EmailCreated {
  files: string[];
  ran: string | null;
  notes: string[];
}

export type EmailRequest =
  | { type: "email_catalog"; name: string }
  | { type: "email_open"; name: string; path: string }
  | { type: "email_stat"; name: string; path: string }
  | {
      type: "email_preview";
      name: string;
      path: string;
      segments?: Segment[] | null;
      theme?: JsonValue;
      data: JsonValue;
      mode: PreviewMode;
    }
  | { type: "email_build"; name: string; path: string; segments: Segment[]; theme: JsonValue }
  | { type: "email_save"; name: string; path: string; content: string; base_hash: string }
  | { type: "email_sample"; name: string; path: string; data: JsonValue }
  | { type: "email_new"; name: string; component: boolean; item: string; markdown: boolean }
  | {
      type: "email_send_test";
      name: string;
      path: string;
      segments?: Segment[] | null;
      theme?: JsonValue;
      data: JsonValue;
      mode: PreviewMode;
      to?: string | null;
    }
  | { type: "email_history"; name: string; path: string }
  | { type: "email_history_version"; name: string; path: string; id: string };

export type EmailResponse =
  | ({ type: "email_catalog" } & EmailCatalog)
  | ({ type: "email_doc" } & EmailDoc)
  | { type: "email_stat"; hash: string }
  | ({ type: "email_preview" } & EmailPreview)
  | { type: "email_built"; content: string; diff: string }
  | { type: "email_saved"; hash: string }
  | ({ type: "email_created" } & EmailCreated);

// ── New projects (locdev-scaffold) ──────────────────────────────────────────

export type PackageRegistry = "npm" | "packagist";

export interface StarterMethod {
  id: string;
  label: string;
  /** The tool that must be installed (see Tool). */
  tool: string;
  note?: string | null;
}

export interface StarterOption {
  id: string;
  label: string;
  help?: string | null;
  /** A choice when there are values; a switch ("true"/"false") otherwise. */
  choices: { value: string; label: string }[];
  default: string;
  /** Only offered with these methods (all when empty). */
  methods: string[];
  /** Only asked when each option (chosen value or default) is one of `values`. */
  when?: { option: string; values: string[] }[];
}

/** Mirrors `Starter::asks` in Rust: the installer's own branching. */
export function starterAsks(
  starter: Starter,
  option: StarterOption,
  method: string,
  chosen: Record<string, string>,
): boolean {
  const value = (id: string) => chosen[id] ?? starter.options.find((o) => o.id === id)?.default ?? "";
  return (
    (option.methods.length === 0 || option.methods.includes(method)) &&
    (option.when ?? []).every((w) => w.values.includes(value(w.option)))
  );
}

export interface Starter {
  id: string;
  name: string;
  family: string;
  framework: string;
  description: string;
  docs: string;
  methods: StarterMethod[];
  options: StarterOption[];
  packages?: PackageRegistry | null;
  database: boolean;
}

export interface Step {
  label: string;
  program: string;
  args: string[];
  cwd: string;
  /** Variables set for this command only. */
  env?: [string, string][];
}

export type ToolInstall = { kind: "command"; steps: Step[] } | { kind: "runtime"; runtime: string };

export interface Tool {
  id: string;
  name: string;
  description: string;
  docs: string;
  installed: boolean;
  version?: string | null;
  path?: string | null;
  install?: ToolInstall | null;
  used_by: string[];
}

export interface DatabaseChoice {
  service: "postgres" | "mysql" | "mariadb";
  instance: string;
}

export interface NewProject {
  starter: string;
  method: string;
  name: string;
  parent: string;
  options: Record<string, string>;
  packages: string[];
  database?: DatabaseChoice | null;
  register: boolean;
  /** Node.js version to create it with and pin the site to. */
  node?: string | null;
  /** PHP version to create it with and pin the site to. */
  php?: string | null;
}

/** A starter file LocDev writes into the new folder before the steps. */
export interface PlannedFile {
  path: string;
  contents: string;
}

export interface ProjectPlan {
  dir: string;
  /** LocDev makes the folder itself; the steps run inside it. */
  make_dir?: boolean;
  files?: PlannedFile[];
  steps: Step[];
  actions: string[];
  finish: Step[];
  notes: string[];
  missing: string[];
}

export interface TerminalInfo {
  id: number;
  title: string;
  running: boolean;
  code?: number | null;
  step: number;
  steps: number;
  site?: string | null;
}

export interface TerminalOutput {
  info: TerminalInfo;
  data: string;
  next: number;
  truncated: boolean;
}

export interface PackageHit {
  name: string;
  description?: string | null;
  version?: string | null;
  downloads?: number | null;
  url?: string | null;
}

/** As `Step::display` in Rust: for showing only. */
export function stepCommand(step: Step): string {
  const quote = (a: string) => (a === "" || /[\s"'$&|;<>*?(){}]/.test(a) ? `'${a.replaceAll("'", "'\\''")}'` : a);
  return [...(step.env ?? []).map(([k, v]) => `${k}=${quote(v)}`), ...[step.program, ...step.args].map(quote)].join(
    " ",
  );
}

// ── Request metrics, Tinker and PHP extensions (v0.4) ───────────────────────

export interface RequestMetrics {
  site: string;
  since: number | null;
  total: number;
  status: {
    informational: number;
    success: number;
    redirect: number;
    client_error: number;
    server_error: number;
  };
  upstream_down: number;
  latency: { samples: number; avg_ms: number; p50_ms: number; p95_ms: number; p99_ms: number; max_ms: number };
  /** Last 60 minutes, oldest first; minutes without traffic left out. */
  per_minute: { minute: number; requests: number; errors: number }[];
  top_paths: { method: string; path: string; requests: number; errors: number; avg_ms: number; max_ms: number }[];
  /** Newest first. */
  recent: { at: number; method: string; path: string; status: number; ms: number; host: string }[];
}

export interface TinkerOutput {
  output: string;
  code: number | null;
  duration_ms: number;
  timed_out: boolean;
  truncated: boolean;
}

export interface ExtRequirement {
  name: string;
  from: string;
  reason?: string | null;
}

export interface PhpExtensionsReport {
  version: string | null;
  executable: string | null;
  origin: string | null;
  loaded: string[];
  required: ExtRequirement[];
  missing: ExtRequirement[];
  recommended: ExtRequirement[];
  remedy: string | null;
  error: string | null;
}

export interface PhpModules {
  executable: string;
  version: string;
  origin: string;
  extensions: { name: string; version: string | null; zend: boolean }[];
  available: string[];
  extension_dir: string | null;
  ini: string | null;
  scanned: string[];
}

export type InsightsRequest =
  | { type: "request_metrics"; name: string }
  | { type: "reset_request_metrics"; name: string }
  | { type: "tinker_run"; name: string; code: string }
  | { type: "tinker_open"; name: string }
  | { type: "php_extensions"; name: string }
  | { type: "php_modules"; executable: string };

export type InsightsResponse =
  | ({ type: "request_metrics" } & RequestMetrics)
  | ({ type: "tinker" } & TinkerOutput)
  | ({ type: "php_extensions" } & PhpExtensionsReport)
  | ({ type: "php_modules" } & PhpModules);
