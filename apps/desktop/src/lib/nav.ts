import {
  Activity,
  ArrowDownToLine,
  Bug,
  Keyboard,
  FolderPlus,
  FolderSearch,
  PackageCheck,
  Bell,
  History,
  Inbox,
  MailPlus,
  KeyRound,
  Layers,
  Package,
  Timer,
  Boxes,
  Cog,
  Database,
  FileCog,
  Rocket,
  Gauge,
  GitBranch,
  GitPullRequest,
  Globe,
  HardDrive,
  Info,
  LayoutDashboard,
  Lock,
  type LucideIcon,
  Network,
  Palette,
  Plug,
  Server,
  Share2,
  ShieldCheck,
  SlidersHorizontal,
  SquareTerminal,
  Stethoscope,
  Workflow,
  Wrench,
} from "lucide-react";
import { t } from "@/lib/i18n";

export type ViewId = "dashboard" | "sites" | "runtimes" | "services" | "tools" | "notifications" | "settings";

/** A subpage: a tab or a section inside a page, never a sidebar entry of its own. */
export interface SubDef {
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
}

export interface ViewDef {
  id: ViewId;
  label: string;
  /** Shown as a path-like eyebrow above the title, e.g. `~/sites`. */
  path: string;
  description: string;
  icon: LucideIcon;
  group: "workspace" | "tools" | "system";
  /** Roadmap milestone for views that are not implemented yet. */
  soon?: string;
  /** Subpages. Only `tools` lists them in the sidebar (as a submenu). */
  subs?: SubDef[];
  /** The subpage opened by default. */
  defaultSub?: string;
}

export const TOOLS: SubDef[] = [
  {
    id: "new-project",
    label: t("New project"),
    description: t("Create a project with its framework's installer, options and database"),
    icon: FolderPlus,
  },
  {
    id: "installers",
    label: t("Installers"),
    description: t("The tools new projects need: Laravel installer, Composer, package managers…"),
    icon: PackageCheck,
  },
  {
    id: "processes",
    label: t("Processes & logs"),
    description: t("Every dev server LocDev runs, live output and failures"),
    icon: Activity,
  },
  {
    id: "debug",
    label: t("Debug Center"),
    description: t("Exceptions, dumps, error logs and failures from your sites"),
    icon: Bug,
  },
  {
    id: "database",
    label: t("Database"),
    description: t("Tables, rows and SQL for your sites' databases and LocDev instances"),
    icon: Database,
  },
  {
    id: "mailbox",
    label: t("Mailbox"),
    description: t("E-mail your sites send, captured here instead of delivered"),
    icon: Inbox,
  },
  {
    id: "tunnels",
    label: t("Tunnels"),
    description: t("Share a running site on the internet with a public address"),
    icon: Share2,
  },
  {
    id: "actions",
    label: t("Actions builder"),
    description: t("GitHub Actions workflows from templates, checked before they're saved"),
    icon: Workflow,
  },
  { id: "cli", label: t("Command line"), description: t("Every locdev command, searchable"), icon: SquareTerminal },
  { id: "doctor", label: t("Doctor"), description: t("Find and fix environment problems"), icon: Stethoscope },
];

export const SETTINGS: SubDef[] = [
  { id: "overview", label: t("Overview"), description: t("Everything LocDev runs, at a glance"), icon: Gauge },
  { id: "daemon", label: t("Daemon"), description: t("locdevd: start, stop, login item and IPC"), icon: Server },
  { id: "proxy", label: t("Proxy"), description: t("The reverse proxy in front of every site"), icon: Network },
  { id: "dns", label: t("DNS"), description: t("How this machine resolves *.test"), icon: Globe },
  {
    id: "parked",
    label: t("Parked folders"),
    description: t("Folders whose projects become sites on their own, kept in step as projects come and go"),
    icon: FolderSearch,
  },
  { id: "https", label: t("HTTPS"), description: t("The local certificate authority"), icon: Lock },
  {
    id: "automations",
    label: t("Automations"),
    description: t("Dependency checks, the job queue and offline mode"),
    icon: Timer,
  },
  {
    id: "frameworks",
    label: t("Frameworks"),
    description: t("What LocDev recognizes, how it runs each framework, and updates to the definitions"),
    icon: Layers,
  },
  {
    id: "environment",
    label: t("Environment"),
    description: t("Variables and secrets shared with your sites, without touching their .env files"),
    icon: KeyRound,
  },
  {
    id: "security",
    label: t("Security"),
    description: t("The password for destructive operations, and backups"),
    icon: ShieldCheck,
  },
  {
    id: "integrations",
    label: t("Integrations"),
    description: t("GitHub, your code editor and deploy platforms"),
    icon: Plug,
  },
  {
    id: "updates",
    label: t("Updates"),
    description: t("New versions of the app, the daemon and the CLI, and going back to an earlier one"),
    icon: ArrowDownToLine,
  },
  { id: "appearance", label: t("Appearance"), description: t("Theme, sidebar and motion"), icon: Palette },
  {
    id: "shortcuts",
    label: t("Shortcuts"),
    description: t("Keyboard shortcuts: see them all, change them, find conflicts"),
    icon: Keyboard,
  },
  { id: "about", label: t("About"), description: t("Versions, folders and the command line"), icon: Info },
];

export const RUNTIME_TABS: SubDef[] = [
  { id: "overview", label: t("Overview"), description: t("Defaults, warnings and updates"), icon: Gauge },
  { id: "node", label: t("Node.js"), description: t("Node.js versions"), icon: Boxes },
  { id: "php", label: t("PHP"), description: t("PHP versions"), icon: Boxes },
  {
    id: "package-managers",
    label: t("Package managers"),
    description: t("npm, pnpm, Yarn, Bun and Composer"),
    icon: HardDrive,
  },
];

export type SiteTab =
  | "overview"
  | "server"
  | "dependencies"
  | "environment"
  | "config"
  | "email"
  | "metrics"
  | "tinker"
  | "deploy"
  | "git"
  | "github"
  | "settings";

export const SITE_TABS: SubDef[] = [
  { id: "overview", label: t("Overview"), description: t("Status, addresses and runtimes"), icon: Gauge },
  { id: "server", label: t("Server"), description: t("Process and output"), icon: Server },
  {
    id: "dependencies",
    label: t("Dependencies"),
    description: t("Packages, updates, compatibility and vulnerabilities"),
    icon: Package,
  },
  {
    id: "environment",
    label: t("Environment"),
    description: t("Shared variables this site gets, its .env files and conflicts"),
    icon: KeyRound,
  },
  {
    id: "config",
    label: t("Config"),
    description: t("The site's config files: options, analysis and history"),
    icon: FileCog,
  },
  {
    id: "email",
    label: t("Email Studio"),
    description: t("The site's email templates and components, edited visually with Blade kept as written"),
    icon: MailPlus,
  },
  {
    id: "deploy",
    label: t("Deploy"),
    description: t("Platforms that fit this site, what they need and readiness checks"),
    icon: Rocket,
  },
  {
    id: "metrics",
    label: t("Metrics"),
    description: t("Requests through LocDev's proxy: status codes, latency and busiest paths"),
    icon: Activity,
  },
  {
    id: "tinker",
    label: t("Tinker"),
    description: t("Run PHP in the Laravel app with the site's environment"),
    icon: SquareTerminal,
  },
  {
    id: "git",
    label: t("Git"),
    description: t("Changes, commits, branches, worktrees, stashes and remotes"),
    icon: GitBranch,
  },
  { id: "github", label: t("GitHub"), description: t("Issues, pull requests and workflow runs"), icon: GitPullRequest },
  { id: "settings", label: t("Settings"), description: t("Command, port, runtimes and startup"), icon: Cog },
];

export const NOTIFICATION_TABS: SubDef[] = [
  { id: "pending", label: t("Pending"), description: t("Not read yet"), icon: Inbox },
  { id: "history", label: t("History"), description: t("Everything, dismissed included"), icon: History },
  { id: "preferences", label: t("Preferences"), description: t("Categories and desktop notifications"), icon: Cog },
];

export const VIEWS: ViewDef[] = [
  {
    id: "dashboard",
    label: t("Dashboard"),
    path: "~/",
    description: t("What's running and what needs attention"),
    icon: LayoutDashboard,
    group: "workspace",
  },
  {
    id: "sites",
    label: t("Sites"),
    path: "~/sites",
    description: t("Projects served by LocDev"),
    icon: Globe,
    group: "workspace",
  },
  {
    id: "runtimes",
    label: t("Runtimes"),
    path: "~/runtimes",
    description: t("Node.js, PHP and package managers per site"),
    icon: Boxes,
    group: "workspace",
    subs: RUNTIME_TABS,
    defaultSub: "overview",
  },
  {
    id: "services",
    label: t("Services"),
    path: "~/services",
    description: t("Databases and caches with their own data, port and logs"),
    icon: Database,
    group: "workspace",
  },
  {
    id: "tools",
    label: t("Tools"),
    path: "~/tools",
    description: t("Diagnostics and utilities"),
    icon: Wrench,
    group: "tools",
    subs: TOOLS,
    defaultSub: "processes",
  },
  {
    id: "notifications",
    label: t("Notifications"),
    path: "~/notifications",
    description: t("Updates, failures and results, kept until you dismiss them"),
    icon: Bell,
    group: "system",
    subs: NOTIFICATION_TABS,
    defaultSub: "pending",
  },
  {
    id: "settings",
    label: t("Settings"),
    path: "~/settings",
    description: t("Daemon, proxy, DNS, HTTPS and app preferences"),
    icon: SlidersHorizontal,
    group: "system",
    subs: SETTINGS,
    defaultSub: "overview",
  },
];

export const viewById = (id: ViewId) => VIEWS.find((v) => v.id === id)!;

export const subById = (view: ViewDef, sub: string | undefined) =>
  view.subs?.find((s) => s.id === (sub ?? view.defaultSub));

export const MOD = typeof navigator !== "undefined" && navigator.userAgent.includes("Mac") ? "⌘" : "Ctrl";
