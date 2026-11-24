// In-memory stand-in for the daemon, used when the UI runs in a plain browser
// (`pnpm dev` without Tauri). Lets the interface be designed without a backend.

import type { Backend } from "./daemon";
import type { AppUpdateProgress, DebugRequest, EditorRequest } from "./protocol";
import { DaemonError } from "./errors";
import { handleActions } from "./mock-actions";
import { handleConfig } from "./mock-config";
import { handleDb } from "./mock-db";
import { handleDeploy } from "./mock-deploy";
import { handleEmail } from "./mock-email";
import { handleInsights } from "./mock-insights";
import { handleScaffold } from "./mock-scaffold";
import { handleMail } from "./mock-mail";
import { handlePark } from "./mock-park";
import { handlePhp } from "./mock-php";
import { handleTunnels } from "./mock-tunnels";
import { handleDebug } from "./mock-debug";
import { handleEditors } from "./mock-editors";
import { handleUpdates, mockAppInstall, mockAppUpdate } from "./mock-updates";
import { handleRemote } from "./mock-remote";
import { handleEnv } from "./mock-env";
import { handleProcs } from "./mock-procs";
import { handleServices } from "./mock-services";
import { handleStack, mockStackExtras } from "./mock-stack";
import { handleGit } from "./mock-git";
import {
  type ActionsRequest,
  type Check,
  type ConfigRequest,
  type DbRequest,
  type DeployRequest,
  type EmailRequest,
  type MailRequest,
  type ParkRequest,
  type PhpRequest,
  type TunnelRequest,
  type RemoteRequest,
  type EnvRequest,
  type ProcsRequest,
  type ServicesRequest,
  type StackRequest,
  type GitRequest,
  type DepsReport,
  type Job,
  KIND_LABEL,
  type Notification,
  type PackageReport,
  type Settings,
  type UpdatePlan,
  type ProcessRole,
  type ProcessView,
  type DnsStatus,
  type LogLine,
  type ProjectView,
  type Release,
  type Request,
  type ResolvedRuntime,
  type Response,
  type RuntimeInfo,
  type RuntimeKind,
  type RuntimeVersion,
  type SystemAction,
  type SystemPlan,
  type SystemTarget,
  type TlsStatus,
  versionMatches,
} from "./protocol";

const now = Date.now();
let running = true;
let startedAt = now - 2 * 3600_000 - 14 * 60_000;
let seq = 0;
/** Whether new terminals get the default versions (Runtimes → Overview). */
let terminalOn = false;

const logs = new Map<string, LogLine[]>();

/** Mirrors the daemon: a level only when the line states one. */
function levelOf(text: string): LogLine["level"] {
  const t = text.trimStart().toUpperCase();
  if (/^(\[?ERROR\b|NPM ERR!|PHP FATAL ERROR)/.test(t) || t.includes(".ERROR:")) return "error";
  if (/^(\[?WARN(ING)?\b|NPM WARN|PHP WARNING)/.test(t)) return "warn";
  if (/^\[?INFO\b/.test(t)) return "info";
  return undefined;
}

function log(name: string, text: string, stream: LogLine["stream"] = "stdout") {
  const lines = logs.get(name) ?? [];
  lines.push({ seq: ++seq, at: Date.now(), stream, text, level: stream === "system" ? undefined : levelOf(text) });
  logs.set(name, lines.slice(-2000));
}

function project(
  name: string,
  kind: ProjectView["kind"],
  path: string,
  command: string | undefined,
  port: number | undefined,
  state: ProjectView["process"]["state"],
): ProjectView {
  return {
    name,
    domain: `${name}.test`,
    path,
    kind,
    command,
    port,
    auto_start: state === "running",
    process: {
      state,
      pid: state === "running" ? 40000 + Math.floor(Math.random() * 9999) : null,
      started_at: state === "running" ? now - 40 * 60_000 : null,
      exit_code: state === "failed" ? 1 : null,
      message: state === "failed" ? "process exited with exit status: 1" : null,
    },
    url: port ? `http://${name}.localhost` : null,
    direct_url: port ? `http://127.0.0.1:${port}` : null,
    https_url: port ? `https://${name}.localhost` : null,
    path_exists: true,
    runtimes: [],
    stack: {
      kind,
      framework: STACK_LABEL[kind] ?? KIND_LABEL[kind],
      dev_tool: kind === "next" ? "next" : kind === "node" ? "vite" : null,
      package_manager: kind === "next" || kind === "node" ? "pnpm" : null,
      command: command ?? null,
      dev_server:
        kind === "laravel"
          ? { tool: "vite", command: "npm run dev -- --port {port} --strictPort --host 127.0.0.1", mode: "ask" }
          : null,
      config_files: kind === "laravel" || kind === "node" ? ["vite.config.js"] : [],
      ...mockStackExtras(kind),
    },
    dev_process: null,
    dev_url: null,
    requirements:
      kind === "next"
        ? [{ kind: "node", constraint: ">=20", file: "package.json (engines)", exact: false }]
        : kind === "laravel"
          ? [{ kind: "php", constraint: "^8.2", file: "composer.json (require)", exact: false }]
          : [],
  };
}

const STACK_LABEL: Partial<Record<ProjectView["kind"], string>> = {
  laravel: "Laravel + Vue + Vite",
  node: "React + Vite",
};

const projects: ProjectView[] = [
  project(
    "acme-shop",
    "laravel",
    "~/Code/acme-shop",
    "php artisan serve --host=127.0.0.1 --port={port} --no-reload",
    8100,
    "running",
  ),
  project("blog", "next", "~/Code/blog", "pnpm run dev --hostname 127.0.0.1 --port {port}", 8101, "running"),
  project("landing", "node", "~/Code/landing", "npm run dev", 8102, "stopped"),
  {
    ...project("wp-agency", "wordpress", "~/Sites/wp-agency", "php -S 127.0.0.1:{port} -t .", 8103, "failed"),
    parked: "~/Sites",
  },
  project("docs", "static", "~/Code/docs", undefined, undefined, "stopped"),
];
// acme-shop is linked to a Vercel project (Site → Deploy).
projects[0].deploy = { platform: "vercel", project_id: "prj_8sKq2", project_name: "acme-shop" };

// The Laravel site has Vite detected and recorded, waiting for a decision.
projects[0].dev_server = structuredClone(projects[0].stack.dev_server!);

queueMicrotask(() => refreshUrls());
for (const p of projects) {
  if (p.command) log(p.name, `$ ${p.command.replace("{port}", String(p.port))}`, "system");
}
log("acme-shop", "   INFO  Server running on [http://127.0.0.1:8100].");
log("acme-shop", "  Press Ctrl+C to stop the server");
log("acme-shop", "  2026-10-31 12:04:11 /api/products ........................ ~ 0.21ms");
log("blog", "  ▲ Next.js 15.4.2");
log("blog", "  - Local:        http://127.0.0.1:8101");
log("blog", " ✓ Ready in 1612ms");
log("acme-shop", "[2026-11-02 12:04:12] local.ERROR: SQLSTATE[HY000] [2002] Connection refused", "stderr");
log("acme-shop", "  WARN  The [public/storage] link does not exist.");
log("wp-agency", "PHP Fatal error:  Uncaught Error: Call to undefined function mysqli_connect()", "stderr");
log("wp-agency", "process ended with code 1", "system");

const checks: Check[] = [
  { id: "config", label: "Configuration folder", level: "ok", detail: "~/.config/locdev" },
  { id: "projects", label: "Project folders", level: "ok", detail: "5 registered, all folders present" },
  { id: "proxy", label: "HTTP proxy", level: "ok", detail: "listening on 127.0.0.1:80" },
  { id: "port-80", label: "HTTP port 80", level: "ok", detail: "used by the LocDev proxy" },
  {
    id: "port-443",
    label: "HTTPS port 443",
    level: "warn",
    detail: "in use by another program; the LocDev proxy will need it",
    remedy: "sudo lsof -iTCP:443 -sTCP:LISTEN",
  },
  {
    id: "resolver",
    label: ".test resolver",
    level: "ok",
    detail: "*.test resolves to this machine",
  },
  { id: "tool-php", label: "PHP", level: "ok", detail: "/usr/bin/php" },
  { id: "tool-composer", label: "Composer", level: "ok", detail: "/usr/local/bin/composer" },
  { id: "tool-node", label: "Node.js", level: "ok", detail: "/usr/bin/node" },
  { id: "tool-npm", label: "npm", level: "ok", detail: "/usr/bin/npm" },
];

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

const RT = "~/.local/share/locdev/runtimes";
const active = (until: string) => ({ state: "active" as const, until });

function info(kind: RuntimeKind, extra: Partial<RuntimeInfo> & Pick<RuntimeInfo, "versions">): RuntimeInfo {
  return {
    kind,
    default: null,
    downloads: true,
    job: null,
    category: kind === "node" || kind === "php" ? "runtime" : "package_manager",
    updates: [],
    usage: {},
    ...extra,
  };
}

const runtimes: RuntimeInfo[] = [
  info("node", {
    default: "22",
    updates: [{ line: "22", installed: "22.23.4", latest: "22.23.6" }],
    versions: [
      {
        version: "24.16.0",
        source: "system",
        executable: "~/.nvm/versions/node/v24.16.0/bin/node",
        origin: "nvm",
        support: { state: "maintenance", until: "2028-04-30" },
      },
      {
        version: "22.23.4",
        source: "managed",
        executable: `${RT}/node/22.23.4/bin/node`,
        origin: "locdev",
        support: { state: "maintenance", until: "2027-04-30" },
      },
      {
        version: "18.20.8",
        source: "managed",
        executable: `${RT}/node/18.20.8/bin/node`,
        origin: "locdev",
        support: { state: "eol", until: "2025-04-30" },
      },
    ],
  }),
  info("php", {
    default: "8.3",
    downloads: true,
    versions: [
      {
        version: "8.4.14",
        source: "system",
        executable: "/usr/bin/php8.4",
        origin: "system",
        support: active("2026-12-31"),
      },
      {
        version: "8.3.27",
        source: "system",
        executable: "/usr/bin/php8.3",
        origin: "system",
        support: { state: "security", until: "2027-12-31" },
      },
      {
        version: "8.1.33",
        source: "system",
        executable: "/opt/homebrew/opt/php@8.1/bin/php",
        origin: "homebrew",
        support: { state: "eol", until: "2025-12-31" },
      },
    ],
  }),
  info("npm", {
    downloads: false,
    versions: [
      { version: "10.9.4", source: "managed", executable: `${RT}/node/22.23.4/bin/npm`, origin: "node 22.23.4" },
      { version: "10.8.2", source: "managed", executable: `${RT}/node/18.20.8/bin/npm`, origin: "node 18.20.8" },
    ],
  }),
  info("pnpm", {
    default: "10",
    versions: [
      { version: "10.18.3", source: "managed", executable: `${RT}/pnpm/10.18.3/locdev-bin/pnpm`, origin: "locdev" },
    ],
  }),
  info("yarn", {
    versions: [{ version: "1.22.22", source: "system", executable: "~/.yarn/bin/yarn", origin: "yarn" }],
  }),
  info("bun", { downloads: false, versions: [] }),
  info("composer", {
    versions: [
      {
        version: "2.11.1",
        source: "managed",
        executable: `${RT}/composer/2.11.1/locdev-bin/composer`,
        origin: "locdev",
      },
    ],
  }),
];

const catalog: Release[] = [
  { version: "26.13.0", lts: "Lithium", date: "2026-10-28" },
  { version: "25.9.0", lts: null, date: "2026-03-31" },
  { version: "24.22.2", lts: "Krypton", date: "2026-10-27" },
  { version: "23.11.1", lts: null, date: "2025-05-14" },
  { version: "22.23.4", lts: "Jod", date: "2026-10-20" },
  { version: "20.20.2", lts: "Iron", date: "2026-03-24" },
  { version: "18.20.8", lts: "Hydrogen", date: "2025-03-27" },
];

/** Like the real StaticPHP catalog (2026-11-22): every line, older patches with a checksum. */
const phpCatalog: Release[] = [
  { version: "8.5.13", lts: null, date: "2026-11-22" },
  { version: "8.5.12", lts: null, date: "2026-11-16" },
  { version: "8.4.28", lts: null, date: "2026-11-22" },
  { version: "8.4.27", lts: null, date: "2026-11-16" },
  { version: "8.3.37", lts: null, date: "2026-11-22" },
  { version: "8.2.36", lts: null, date: "2026-11-22" },
];

const byVersion = (a: RuntimeVersion, b: RuntimeVersion) =>
  b.version.localeCompare(a.version, undefined, { numeric: true });

/** Mirrors the daemon (simplified): the project's own pin, else the default. */
function resolveRuntimes(p: ProjectView): ResolvedRuntime[] {
  const wanted: RuntimeKind[] = ["laravel", "wordpress", "php"].includes(p.kind)
    ? ["php", "composer"]
    : ["next", "node"].includes(p.kind)
      ? ["node", "pnpm"]
      : [];
  return wanted.flatMap((kind) => {
    const info = runtimes.find((r) => r.kind === kind)!;
    const own = kind === "node" ? p.node : kind === "php" ? p.php : p.tools?.[kind];
    const requested = own ?? info.default;
    if (!requested) return [];
    const version = info.versions.find((v) => versionMatches(requested, v.version)) ?? null;
    return [{ kind, requested, pinned: own !== undefined, version, from: own ? "projects.toml" : "default" }];
  });
}

/** Version → sites, like the daemon's `usage`. */
function withUsage(): RuntimeInfo[] {
  const all = structuredClone(runtimes);
  for (const r of all) r.usage = {};
  for (const p of projects) {
    for (const resolved of resolveRuntimes(p)) {
      if (!resolved.version) continue;
      const target = all.find((r) => r.kind === resolved.kind)!;
      (target.usage[resolved.version.version] ??= []).push(p.name);
    }
  }
  return all;
}

function simulateInstall(kind: RuntimeKind, version: string) {
  const target = runtimes.find((r) => r.kind === kind)!;
  const total = 44_337_064;
  target.job = {
    kind,
    version,
    state: "downloading",
    downloaded: 0,
    total,
    error: null,
    log: [
      `installing ${kind} ${version}`,
      `source https://nodejs.org/dist/v${version}/node-v${version}-linux-x64.tar.gz`,
    ],
  };
  const timer = setInterval(() => {
    const job = target.job!;
    if (job.state === "downloading") {
      job.downloaded = Math.min(total, job.downloaded + 4_800_000);
      if (job.downloaded >= total) job.state = "verifying";
    } else if (job.state === "verifying") {
      job.state = "extracting";
    } else {
      job.state = "done";
      target.versions.push({
        version,
        source: "managed",
        executable: `${RT}/${kind}/${version}/bin/${kind}`,
        origin: "locdev",
      });
      job.log?.push("sha256 matches", `installed in ${RT}/${kind}/${version}`);
      target.versions.sort(byVersion);
      clearInterval(timer);
    }
  }, 300);
}

/** The `locdev` bundled with the app (an AppImage: not on PATH until linked). */
const cli = {
  bundled: "/opt/LocDev/locdev",
  version: __DAEMON_VERSION__ as string | null,
  on_path: null as string | null,
  link: null as string | null,
  link_path: "~/.local/bin/locdev" as string | null,
  link_dir_on_path: true,
  conflict: false,
};

const service = {
  manager: "systemd --user",
  installed: false,
  location: "~/.config/systemd/user/locdevd.service",
};

const dns: DnsStatus = {
  enabled: true,
  port: 10053,
  error: null,
  backend: "systemd-resolved",
  installed: false,
  routing: "none",
  conflicts: ["/etc/systemd/resolved.conf.d/other-tool-test.conf"],
};

const tls: TlsStatus = {
  enabled: true,
  port: 443,
  requested_port: 443,
  error: null,
  ca_path: "~/.local/share/locdev/tls/ca.pem",
  fingerprint: "C9:66:87:1C:87:BB:C0:6B:C1:F9:CD:34:C0:59:60:5A:9A:F1:75:AA:48:9F:F9:8B:69:DC:FE:FF:59:F0:1D:13",
  trusted: false,
  browsers: false,
};

/** Mirrors how the daemon picks each site's URLs. */
function refreshUrls() {
  const suffix = "test";
  for (const p of projects) {
    if (!p.port) continue;
    p.https_url = `https://${p.name}.${suffix}`;
    p.url = tls.trusted ? p.https_url : `http://${p.name}.${suffix}`;
  }
}

function plan(target: SystemTarget, action: SystemAction): SystemPlan {
  if (target === "tls") {
    const nss = ["~/.pki/nssdb", "~/snap/firefox/common/.mozilla/firefox/abc.default"];
    return action === "install"
      ? {
          summary:
            "Trust the LocDev CA system-wide (update-ca-certificates), then update 2 browser certificate store(s) as you.",
          script: {
            shell: "sh",
            source: `set -e\ninstall -m 0644 '${tls.ca_path}' /usr/local/share/ca-certificates/locdev-ca.crt\nupdate-ca-certificates\n`,
          },
          user_script: {
            shell: "sh",
            source: nss
              .map((db) => `certutil -d "sql:${db}" -A -t 'C,,' -n 'LocDev Local CA' -i '${tls.ca_path}'\n`)
              .join(""),
          },
        }
      : {
          summary: "Stop trusting the LocDev CA system-wide, then update 2 browser certificate store(s) as you.",
          script: {
            shell: "sh",
            source: "set -e\nrm -f /usr/local/share/ca-certificates/locdev-ca.crt\nupdate-ca-certificates\n",
          },
          user_script: {
            shell: "sh",
            source: nss
              .map((db) => `certutil -d "sql:${db}" -D -n 'LocDev Local CA' >/dev/null 2>&1 || true\n`)
              .join(""),
          },
        };
  }

  const path = "/etc/systemd/resolved.conf.d/locdev-test.conf";
  return action === "install"
    ? {
        summary: `Route *.test to LocDev (127.0.0.1:${dns.port}) by adding ${path} and restarting systemd-resolved.`,
        script: {
          shell: "sh",
          source: `set -e\nmkdir -p /etc/systemd/resolved.conf.d\ncat > ${path} <<'LOCDEV'\n# Managed by LocDev: send *.test lookups to the LocDev daemon.\n# Remove with \`locdev dns uninstall\`.\n[Resolve]\nDNS=127.0.0.1:${dns.port}\nDomains=~test\nLOCDEV\nsystemctl restart systemd-resolved\n`,
        },
        user_script: null,
      }
    : {
        summary: `Stop routing *.test to LocDev: remove ${path} and restart systemd-resolved.`,
        script: { shell: "sh", source: `set -e\nrm -f ${path}\nsystemctl restart systemd-resolved\n` },
        user_script: null,
      };
}

function find(name: string): ProjectView {
  const p = projects.find((p) => p.name === name);
  if (!p) throw new DaemonError("app", `no project named '${name}'`, "NOT_FOUND");
  return p;
}

function start(p: ProjectView) {
  if (!p.command) {
    throw new DaemonError("app", `project '${p.name}' has no command configured`, "NOT_RUNNABLE");
  }
  p.process = {
    state: "running",
    pid: 40000 + Math.floor(Math.random() * 9999),
    started_at: Date.now(),
    exit_code: null,
    message: null,
  };
  log(p.name, `$ ${p.command.replace("{port}", String(p.port))}`, "system");
  log(p.name, `listening on http://127.0.0.1:${p.port}`);
}

function stop(p: ProjectView) {
  if (p.process.state !== "running") return;
  log(p.name, "stopping…", "system");
  p.process = { ...p.process, state: "stopped", pid: null };
  log(p.name, "process ended with code 0", "system");
}

async function handle(req: Request): Promise<Response> {
  await delay(120);
  if (!running) throw new DaemonError("daemon_down", "connection refused");

  switch (req.type) {
    case "status":
      return {
        type: "status",
        version: __DAEMON_VERSION__,
        protocol: 2,
        pid: 31337,
        started_at: startedAt,
        projects: projects.length,
        running: projects.filter((p) => p.process.state === "running").length,
        home: "~/.config/locdev",
        proxy: {
          enabled: true,
          port: 80,
          requested_port: 80,
          error: null,
          suffix: "test",
        },
        dns: structuredClone(dns),
        tls: structuredClone(tls),
        unread_notifications: notifications.filter((n) => !n.read && !n.dismissed && n.category !== "mail").length,
      };
    case "client_presence":
      return { type: "ok" };
    case "shutdown":
      running = false;
      return { type: "ok" };
    case "list_runtimes":
      if (req.refresh) await delay(500);
      return { type: "runtimes", runtimes: withUsage() };
    case "runtime_catalog":
      await delay(250);
      if (req.kind === "php") return { type: "catalog", releases: structuredClone(phpCatalog) };
      if (req.kind !== "node") throw new DaemonError("app", "LocDev can't download this one", "NOT_RUNNABLE");
      return { type: "catalog", releases: structuredClone(catalog) };
    case "install_runtime": {
      const releases = req.kind === "php" ? phpCatalog : catalog;
      const release =
        req.version === "lts"
          ? releases.find((r) => r.lts)
          : releases.find((r) => versionMatches(req.version, r.version));
      if (!release) throw new DaemonError("app", `no release matches '${req.version}'`, "NOT_FOUND");
      simulateInstall(req.kind, release.version);
      return { type: "runtimes", runtimes: withUsage() };
    }
    case "uninstall_runtime": {
      const users = withUsage().find((r) => r.kind === req.kind)!.usage[req.version] ?? [];
      if (users.length && !req.force) {
        throw new DaemonError(
          "app",
          `${req.kind} ${req.version} is used by ${users.join(", ")}; pass --force to remove it anyway`,
          "CONFLICT",
        );
      }
      const target = runtimes.find((r) => r.kind === req.kind)!;
      target.versions = target.versions.filter((v) => !(v.version === req.version && v.source === "managed"));
      return { type: "runtimes", runtimes: withUsage() };
    }
    case "set_default_runtime": {
      runtimes.find((r) => r.kind === req.kind)!.default = req.version || null;
      return { type: "runtimes", runtimes: withUsage() };
    }
    case "terminal_runtimes": {
      if (req.enable != null) terminalOn = req.enable;
      const commands = runtimes
        .filter((r) => r.default)
        .flatMap((r) => (r.kind === "node" ? ["corepack", "node", "npm", "npx"] : [r.kind]))
        .sort();
      return {
        type: "terminal_runtimes",
        dir: "/home/dev/.local/share/locdev/bin",
        enabled: terminalOn,
        files: terminalOn ? ["/home/dev/.bashrc", "/home/dev/.zshrc"] : [],
        active: false,
        commands,
      };
    }
    case "check_tls":
      await delay(300);
      return { type: "tls", ...structuredClone(tls) };
    case "check_dns":
      await delay(300);
      return { type: "dns", ...structuredClone(dns) };
    case "doctor":
      await delay(400);
      return { type: "doctor", checks };
    case "list_projects":
      return {
        type: "projects",
        projects: structuredClone(projects).map((p) => ({ ...p, runtimes: resolveRuntimes(p) })),
      };
    case "add_project": {
      const name = (req.name || req.path.split(/[\\/]/).filter(Boolean).pop() || "site")
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
      if (projects.some((p) => p.name === name)) {
        throw new DaemonError("app", `a project named '${name}' already exists`, "ALREADY_EXISTS");
      }
      const port = req.port ?? 8100 + projects.length;
      const p = project(name, "node", req.path, req.command || "npm run dev", port, "stopped");
      p.auto_start = req.auto_start ?? false;
      projects.push(p);
      projects.sort((a, b) => a.name.localeCompare(b.name));
      return { type: "project", project: structuredClone(p) };
    }
    case "update_project": {
      const p = find(req.name);
      if (req.command !== undefined) p.command = req.command.trim() || undefined;
      if (req.port !== undefined) {
        p.port = req.port;
        refreshUrls();
        p.direct_url = `http://127.0.0.1:${req.port}`;
      }
      if (req.auto_start !== undefined) p.auto_start = req.auto_start;
      if (req.node !== undefined) p.node = req.node || undefined;
      if (req.php !== undefined) p.php = req.php || undefined;
      for (const [kind, pin] of Object.entries(req.tools ?? {})) {
        p.tools = { ...p.tools, [kind]: pin || undefined };
      }
      return { type: "project", project: structuredClone(p) };
    }
    case "remove_project": {
      const index = projects.findIndex((p) => p.name === req.name);
      if (index < 0) throw new DaemonError("app", `no project named '${req.name}'`, "NOT_FOUND");
      projects.splice(index, 1);
      return { type: "ok" };
    }
    case "start_project": {
      const p = find(req.name);
      if (p.process.state !== "running") start(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "stop_project": {
      const p = find(req.name);
      await delay(300);
      stop(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "restart_project": {
      const p = find(req.name);
      stop(p);
      start(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "project_logs": {
      find(req.name);
      const key = req.aux ? `${req.name}:${req.aux}` : req.process === "dev" ? `${req.name}:dev` : req.name;
      const lines = (logs.get(key) ?? []).filter((l) => l.seq > (req.after ?? 0));
      return { type: "logs", lines: lines.slice(-(req.limit ?? 500)) };
    }
    case "list_processes":
      return { type: "processes", processes: processViews() };
    case "set_dev_server": {
      const p = find(req.name);
      const dev = p.dev_server ?? p.stack.dev_server;
      if (!dev) throw new DaemonError("app", `no helper dev server detected for ${req.name}`, "NOT_FOUND");
      p.dev_server = { ...dev, mode: req.mode, command: req.command || dev.command, port: dev.port ?? 5173 };
      if (req.start) startDev(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "start_dev_server": {
      const p = find(req.name);
      if (!p.dev_server || p.dev_server.mode === "ask") {
        throw new DaemonError("app", "LocDev doesn't run it until you decide", "NOT_RUNNABLE");
      }
      startDev(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "stop_dev_server":
    case "restart_dev_server": {
      const p = find(req.name);
      if (p.dev_process) p.dev_process = { ...p.dev_process, state: "stopped", pid: null };
      log(`${p.name}:dev`, "process ended with code 0", "system");
      if (req.type === "restart_dev_server") startDev(p);
      return { type: "project", project: structuredClone(p) };
    }
    case "check_site_https": {
      const p = find(req.name);
      return {
        type: "doctor",
        checks: [
          { id: "https-server", label: "HTTPS listener", level: "ok", detail: "127.0.0.1:443" },
          {
            id: "dns",
            label: "*.test resolution",
            level: dns.routing === "locdev" ? "ok" : "warn",
            detail:
              dns.routing === "locdev"
                ? `${p.domain} → LocDev`
                : `${p.domain} doesn't resolve yet; https://${p.name}.localhost works meanwhile`,
            remedy: dns.routing === "locdev" ? undefined : "locdev dns install",
          },
          {
            id: "certificate",
            label: "Certificate",
            level: "ok",
            detail: `issued on demand for ${p.domain} and *.${p.domain}`,
          },
          {
            id: "trust",
            label: "System trust",
            level: tls.trusted ? "ok" : "warn",
            detail: tls.trusted ? "the LocDev CA is trusted" : "the LocDev CA is not trusted yet; browsers will warn",
            remedy: tls.trusted ? undefined : "locdev tls trust",
          },
          { id: "port", label: "Dev server port", level: "ok", detail: `proxied to 127.0.0.1:${p.port}` },
          {
            id: "secure",
            label: "HTTPS only",
            level: "info",
            detail: p.secure ? "plain HTTP requests are redirected to HTTPS" : "served over both HTTP and HTTPS",
          },
        ],
      };
    }
    case "set_subdomains": {
      const p = find(req.name);
      const seen = new Set<string>();
      const rules = req.subdomains.rules.map((r) => {
        const pattern = r.pattern.trim().toLowerCase();
        if (!/^(\*\.)?[a-z0-9-]+(\.[a-z0-9-]+)*$/.test(pattern) || pattern === "*")
          throw new DaemonError("app", `\`${pattern}\` isn't a subdomain`, "INVALID_INPUT");
        if (seen.has(pattern)) throw new DaemonError("app", `\`${pattern}\` is listed twice`, "INVALID_INPUT");
        seen.add(pattern);
        return { ...r, pattern };
      });
      p.subdomains = { wildcard: req.subdomains.wildcard, rules };
      return { type: "project", project: structuredClone(p) };
    }
    case "explain_host": {
      const host = req.host.trim().toLowerCase().replace(/:\d+$/, "");
      const labels = host.split(".");
      const p = labels.length >= 2 ? projects.find((x) => x.name === labels[labels.length - 2]) : undefined;
      const sub = labels.slice(0, -2).join(".");
      const subs = p?.subdomains ?? { wildcard: true, rules: [] };
      const rule = subs.rules.find(
        (r) => r.pattern === sub || (r.pattern.startsWith("*.") && sub.endsWith(r.pattern.slice(1))),
      );
      const problem = !p
        ? `no site is named ${labels[labels.length - 2]}`
        : !sub || rule || subs.wildcard
          ? null
          : "the wildcard is off and no rule matches";
      const port = rule?.target.type === "port" ? rule.target.port : (p?.port ?? null);
      return {
        type: "host_explain",
        host,
        dns: { answers: ["127.0.0.1"], via_locdev: true, error: null },
        route: {
          site: p?.name ?? null,
          sub,
          rule: rule?.pattern ?? null,
          target: problem ? "nothing" : rule ? `port ${port}` : "the site",
          port: problem ? null : port,
          reachable: problem ? null : true,
          problem,
        },
        tls: { names: p ? [`${p.name}.test`, `*.${p.name}.test`] : ["localhost"], https_port: 443, trusted: false },
      };
    }
    case "set_secure": {
      const p = find(req.name);
      p.secure = req.secure;
      return { type: "project", project: structuredClone(p) };
    }
    case "list_deps": {
      find(req.name);
      const report = depsReports[req.name] ?? {
        project: req.name,
        manager: null,
        files: [],
        packages: [],
        checked_at: null,
        errors: [],
      };
      return { type: "deps", ...structuredClone(report) };
    }
    case "check_deps": {
      find(req.name);
      const job = newJob("deps_audit", req.name);
      const total = depsReports[req.name]?.packages.length ?? 0;
      let done = 0;
      const timer = setInterval(() => {
        if (done >= total) {
          clearInterval(timer);
          if (depsReports[req.name]) depsReports[req.name].checked_at = Date.now();
          finish(job, `${total} package(s) checked`);
          return;
        }
        job.progress = { done, total, label: depsReports[req.name].packages[done].name };
        done++;
      }, 350);
      return { type: "job", job: structuredClone(job) };
    }
    case "plan_deps_update":
      await delay(400);
      return { type: "deps_plan", ...planFor(req.name, req.packages, !!req.allow_major, !!req.run_scripts) };
    case "update_deps": {
      const plan = planFor(req.name, req.packages, !!req.allow_major, !!req.run_scripts);
      const job = newJob("deps_update", req.name);
      job.log.push(...plan.commands.map((c) => `$ ${c.join(" ")}`));
      setTimeout(
        () => job.log.push("", "changed 3 packages, and audited 214 packages in 3s", "", "found 0 vulnerabilities"),
        600,
      );
      setTimeout(() => {
        for (const change of plan.changes) {
          const p = depsReports[req.name].packages.find((x) => x.name === change.name)!;
          Object.assign(p, {
            installed: change.to,
            locked: change.to,
            update: "none",
            status: "up_to_date",
            compatibility: "compatible",
            reasons: [],
            advisories: [],
          });
        }
        finish(
          job,
          `updated ${plan.changes.map((c) => `${c.name} ${c.to}`).join(", ")}; changed ${plan.files.join(", ")}`,
        );
      }, 1600);
      return { type: "job", job: structuredClone(job) };
    }
    case "list_jobs":
      return { type: "jobs", jobs: structuredClone(jobs.filter((j) => !req.project || j.project === req.project)) };
    case "get_job": {
      const job = jobs.find((j) => j.id === req.id);
      if (!job) throw new DaemonError("app", `no job ${req.id}`, "NOT_FOUND");
      return { type: "job", job: structuredClone(job) };
    }
    case "control_job": {
      const job = jobs.find((j) => j.id === req.id);
      if (!job) throw new DaemonError("app", `no job ${req.id}`, "NOT_FOUND");
      job.status = req.action === "cancel" ? "cancelled" : req.action === "pause" ? "paused" : "pending";
      return { type: "job", job: structuredClone(job) };
    }
    case "list_notifications": {
      const list = notifications.filter((n) => req.all || !n.dismissed);
      return {
        type: "notifications",
        notifications: structuredClone(list),
        unread: notifications.filter((n) => !n.read && !n.dismissed).length,
      };
    }
    case "mark_notifications": {
      for (const n of notifications.filter((n) => !req.ids?.length || req.ids.includes(n.id))) {
        if (req.read !== undefined) n.read = req.read;
        if (req.dismissed !== undefined) n.dismissed = n.read = req.dismissed || n.read;
      }
      return { type: "ok" };
    }
    case "test_notification":
      notify({
        category: "action",
        severity: "info",
        title: "Notifications work",
        body: "This is how LocDev will tell you about updates and failures.",
        project: null,
        link: null,
        dedup: `test:${Date.now()}`,
      });
      return { type: "ok" };
    case "get_settings":
      return { type: "settings", ...structuredClone(settings) };
    case "update_settings":
      settings = structuredClone(req.settings);
      return { type: "settings", ...structuredClone(settings) };
    default:
      {
        const insights = handleInsights(req);
        if (insights) return insights;
      }
      if (
        [
          "list_services",
          "service_releases",
          "install_service",
          "uninstall_service",
          "create_instance",
          "control_instance",
          "set_instance",
          "remove_instance",
          "backup_instance",
          "instance_backups",
          "restore_instance",
        ].includes(req.type)
      ) {
        return handleServices(req as ServicesRequest) as Response;
      }
      if (req.type === "instance_logs") return { type: "logs", lines: [] };
      if (req.type.startsWith("db_") || req.type.startsWith("redis_")) return handleDb(req as DbRequest) as Response;
      if (req.type.startsWith("config_")) return handleConfig(req as ConfigRequest) as Response;
      if (req.type === "deploy_link_site") {
        const p = find(req.site);
        p.deploy =
          req.platform && req.project
            ? {
                platform: req.platform,
                project_id: req.project,
                project_name: req.project === "prj_3Hf91" ? "acme-blog" : "acme-shop",
              }
            : null;
        return { type: "project", project: structuredClone(p) };
      }
      if (
        req.type === "deploy_platforms" ||
        req.type.startsWith("deploy_connect") ||
        req.type.startsWith("deploy_disconnect") ||
        req.type.startsWith("deploy_remote") ||
        ["deploy_run", "deploy_cancel", "deploy_rollback", "deploy_logs"].includes(req.type)
      )
        return handleRemote(req as RemoteRequest, () => projects) as Response;
      if (req.type.startsWith("deploy_")) return handleDeploy(req as DeployRequest) as Response;
      if (req.type.startsWith("mail")) return handleMail(req as MailRequest) as Response;
      if (req.type.startsWith("email_")) return handleEmail(req as EmailRequest) as Response;
      if (req.type.startsWith("php_")) return handlePhp(req as PhpRequest) as Response;
      if (req.type === "tunnel_logs") {
        const now = Date.now();
        return {
          type: "logs",
          lines: [
            "Sharing acme-shop.test (127.0.0.1:8100) through a Cloudflare Quick Tunnel.",
            "INF Requesting new quick Tunnel on trycloudflare.com...",
            "INF Registered tunnel connection connIndex=0 location=gru01 protocol=quic",
          ].map((text, i) => ({ seq: i + 1, at: now - (3 - i) * 1000, stream: "stdout" as const, text })),
        } as Response;
      }
      if (req.type.startsWith("tunnel_")) return handleTunnels(req as TunnelRequest, () => projects) as Response;
      if (req.type === "update_check") return handleUpdates(req) as Response;
      if (req.type.startsWith("editor_")) return handleEditors(req as EditorRequest, () => settings.editor.preferred);
      if (req.type.startsWith("observe_"))
        return handleDebug(
          req as DebugRequest,
          () => projects,
          () => settings.observe,
        );
      if (req.type.startsWith("park_")) {
        return handlePark(
          req as ParkRequest,
          () => projects,
          (name) => {
            const index = projects.findIndex((p) => p.name === name);
            if (index >= 0) projects.splice(index, 1);
          },
        );
      }
      {
        const scaffold = handleScaffold(req);
        if (scaffold) return scaffold;
      }
      if (["project_processes", "control_process", "set_process", "remove_process"].includes(req.type)) {
        return handleProcs(req as ProcsRequest, find, log);
      }
      if (req.type.startsWith("stack_") || req.type.endsWith("_definitions")) {
        return handleStack(req as StackRequest, () => projects);
      }
      if (req.type.startsWith("env_")) {
        return handleEnv(req as EnvRequest, () =>
          projects.map((p) => ({
            name: p.name,
            kind: p.kind,
            framework: p.stack.framework,
            running: p.process.state === "running",
          })),
        );
      }
      return req.type.startsWith("actions") || req.type.endsWith("action_versions")
        ? handleActions(req as ActionsRequest)
        : handleGit(req as GitRequest);
  }
}

// ── Dependencies, jobs, notifications, settings ─────────────────────────────

const links = (name: string, repo?: string): PackageReport["links"] => ({
  registry:
    name.includes("/") && !name.startsWith("@")
      ? `https://packagist.org/packages/${name}`
      : `https://www.npmjs.com/package/${name}`,
  repository: repo ?? null,
  releases: repo ? `${repo}/releases` : null,
  homepage: null,
});

function pkg(
  name: string,
  kind: PackageReport["kind"],
  declared: string,
  installed: string,
  latest: string,
  extra: Partial<PackageReport> = {},
): PackageReport {
  const [a, b] = [installed, latest].map((v) => v.split(".").map(Number));
  const update: PackageReport["update"] =
    installed === latest ? "none" : a[0] !== b[0] ? "major" : a[1] !== b[1] ? "minor" : "patch";
  return {
    name,
    ecosystem: name.includes("/") && !name.startsWith("@") ? "composer" : "npm",
    kind,
    declared,
    locked: installed,
    installed,
    wanted: update === "major" ? installed : latest,
    latest,
    update,
    status: update === "none" ? "up_to_date" : "outdated",
    compatibility: update === "none" ? "compatible" : update === "major" ? "possibly" : "compatible",
    reasons:
      update === "major"
        ? [
            `outside your constraint ${declared}; the manifest would change`,
            "major release: breaking changes are possible; read the release notes",
          ]
        : update === "none"
          ? []
          : [`within your constraint ${declared}`],
    advisories: [],
    links: links(name, extra.links?.repository ?? undefined),
    deprecated: null,
    ...extra,
  };
}

const depsReports: Record<string, DepsReport> = {
  "acme-shop": {
    project: "acme-shop",
    manager: "npm",
    files: ["package.json", "package-lock.json", "composer.json", "composer.lock"],
    checked_at: now - 3 * 3600_000,
    errors: [],
    packages: [
      pkg("laravel/framework", "prod", "^11.0", "11.9.2", "11.31.0", {
        links: links("laravel/framework", "https://github.com/laravel/framework"),
      }),
      pkg("guzzlehttp/guzzle", "prod", "^7.8", "7.9.2", "7.9.2"),
      pkg("phpunit/phpunit", "dev", "^10.5", "10.5.20", "11.4.3", {
        links: links("phpunit/phpunit", "https://github.com/sebastianbergmann/phpunit"),
      }),
      pkg("axios", "dev", "^1.6.0", "1.6.8", "1.7.7", {
        status: "vulnerable",
        advisories: [
          {
            id: "GHSA-8hc4-vh64-cxmj",
            summary: "Server-Side Request Forgery in axios",
            severity: "HIGH",
            url: "https://osv.dev/vulnerability/GHSA-8hc4-vh64-cxmj",
            fixed: "1.7.4",
          },
        ],
        reasons: ["within your constraint ^1.6.0", "GHSA-8hc4-vh64-cxmj is fixed in 1.7.4"],
      }),
      pkg("laravel-vite-plugin", "dev", "^1.0.0", "1.3.0", "3.2.0", {
        compatibility: "incompatible",
        reasons: [
          "outside your constraint ^1.0.0; the manifest would change",
          "needs vite ^8.0.0; the project has 5.4.2",
        ],
        links: links("laravel-vite-plugin", "https://github.com/laravel/vite-plugin"),
      }),
      pkg("vite", "dev", "^5.0.0", "5.4.2", "8.3.5", { links: links("vite", "https://github.com/vitejs/vite") }),
    ],
  },
  blog: {
    project: "blog",
    manager: "pnpm",
    files: ["package.json", "pnpm-lock.yaml"],
    checked_at: now - 26 * 3600_000,
    errors: [],
    packages: [
      pkg("next", "prod", "^15.4.0", "15.4.2", "15.5.4"),
      pkg("react", "prod", "^19.0.0", "19.1.0", "19.3.1"),
      pkg("typescript", "dev", "^5.6.0", "5.9.2", "5.9.2"),
    ],
  },
};

let nextJob = 7;
const jobs: Job[] = [
  {
    id: 6,
    kind: "deps_audit",
    project: "blog",
    status: "pending",
    automatic: true,
    created_at: now - 60_000,
    scheduled_at: now + 20 * 60_000,
    started_at: null,
    finished_at: null,
    attempts: 0,
    max_attempts: 4,
    progress: null,
    result: null,
    error: null,
    log: [],
    dedup: "audit:blog",
  },
  {
    id: 5,
    kind: "deps_audit",
    project: "acme-shop",
    status: "completed",
    automatic: true,
    created_at: now - 3 * 3600_000,
    scheduled_at: now - 3 * 3600_000,
    started_at: now - 3 * 3600_000,
    finished_at: now - 3 * 3600_000 + 9000,
    attempts: 1,
    max_attempts: 4,
    progress: null,
    result: "6 package(s) checked: 5 with updates, 1 with known vulnerabilities",
    error: null,
    log: [],
    dedup: "audit:acme-shop",
  },
];

let nextNotification = 4;
const notifications: Notification[] = [
  {
    id: 3,
    category: "security",
    severity: "error",
    title: "GHSA-8hc4-vh64-cxmj in axios (acme-shop.test)",
    body: "Server-Side Request Forgery in axios\nFixed in 1.7.4.",
    project: "acme-shop",
    link: { view: "sites", sub: "dependencies", project: "acme-shop" },
    created_at: now - 3 * 3600_000,
    read: false,
    dismissed: false,
    delivered: false,
    dedup: "sec:acme-shop:axios:GHSA-8hc4-vh64-cxmj",
  },
  {
    id: 2,
    category: "dependency_update",
    severity: "info",
    title: "5 updates for acme-shop.test",
    body: "laravel/framework 11.9.2 → 11.31.0\nphpunit/phpunit 10.5.20 → 11.4.3\naxios 1.6.8 → 1.7.7\nvite 5.4.2 → 8.3.5\n…",
    project: "acme-shop",
    link: { view: "sites", sub: "dependencies", project: "acme-shop" },
    created_at: now - 3 * 3600_000,
    read: false,
    dismissed: false,
    delivered: false,
    dedup: "deps:acme-shop:x",
  },
  {
    id: 1,
    category: "process_failure",
    severity: "error",
    title: "wp-agency.test stopped with an error",
    body: "process exited with exit status: 1",
    project: "wp-agency",
    link: { view: "sites", sub: "server", project: "wp-agency" },
    created_at: now - 5 * 3600_000,
    read: true,
    dismissed: false,
    delivered: false,
    dedup: "fail:wp-agency:1",
  },
];

let settings: Settings = {
  automation: { audit: "daily", concurrency: 2, requests_per_second: 8, definitions: true, updates: true },
  notifications: { native: true, categories: {} },
  network: { offline: false },
  integrations: { github_client_id: "", github_watch_runs: true },
  env: { conflicts: "active_files" },
  editor: { preferred: "vscode" },
  observe: { enabled: true, retention_days: 7, max_mb: 100, logs_from: "warn" },
};

function newJob(kind: Job["kind"], project: string | null, automatic = false): Job {
  const job: Job = {
    id: nextJob++,
    kind,
    project,
    status: "running",
    automatic,
    created_at: Date.now(),
    scheduled_at: Date.now(),
    started_at: Date.now(),
    finished_at: null,
    attempts: 1,
    max_attempts: 3,
    progress: null,
    result: null,
    error: null,
    log: [],
    dedup: `${kind}:${project}`,
  };
  jobs.unshift(job);
  return job;
}

function notify(n: Omit<Notification, "id" | "created_at" | "read" | "dismissed" | "delivered">) {
  notifications.unshift({
    ...n,
    id: nextNotification++,
    created_at: Date.now(),
    read: false,
    dismissed: false,
    delivered: false,
  });
}

function finish(job: Job, result: string) {
  job.status = "completed";
  job.finished_at = Date.now();
  job.progress = null;
  job.result = result;
  notify({
    category: "action",
    severity: "info",
    title: `${job.kind === "deps_update" ? "Dependency update" : "Dependency check"} for ${job.project} finished`,
    body: result,
    project: job.project,
    link: { view: "sites", sub: "dependencies", project: job.project },
    dedup: `job:${job.id}`,
  });
}

function planFor(name: string, packages: string[], allowMajor: boolean, runScripts: boolean): UpdatePlan {
  const report = depsReports[name];
  const changes = packages.flatMap((n) => {
    const p = report?.packages.find((x) => x.name === n);
    if (!p || p.update === "none") return [];
    const major = p.update === "major";
    if (major && !allowMajor) return [];
    return [
      {
        name: p.name,
        ecosystem: p.ecosystem,
        from: p.installed,
        to: p.latest!,
        constraint: major ? `^${p.latest}` : null,
        update: p.update,
        compatibility: p.compatibility,
        reasons: p.reasons,
        links: p.links,
      },
    ];
  });
  const npm = changes.filter((c) => c.ecosystem === "npm");
  const composer = changes.filter((c) => c.ecosystem === "composer");
  const scripts = runScripts ? [] : ["--ignore-scripts"];
  const commands: string[][] = [];
  if (npm.some((c) => !c.constraint))
    commands.push(["npm", "update", ...npm.filter((c) => !c.constraint).map((c) => c.name), ...scripts]);
  if (npm.some((c) => c.constraint))
    commands.push([
      "npm",
      "install",
      "--save-dev",
      ...npm.filter((c) => c.constraint).map((c) => `${c.name}@${c.constraint}`),
      ...scripts,
    ]);
  if (composer.length)
    commands.push([
      "composer",
      "update",
      ...composer.map((c) => c.name),
      "--with-dependencies",
      "--no-interaction",
      ...(runScripts ? [] : ["--no-scripts"]),
    ]);
  const skipped = packages.filter((n) => !changes.some((c) => c.name === n));
  return {
    project: name,
    changes,
    commands,
    files: [
      ...(npm.length ? ["package.json", "package-lock.json"] : []),
      ...(composer.length ? ["composer.json", "composer.lock"] : []),
    ],
    run_scripts: runScripts,
    warnings: skipped.map((n) => `${n} is already at the newest ${allowMajor ? "stable" : "allowed"} version`),
  };
}

function startDev(p: ProjectView) {
  const key = `${p.name}:dev`;
  p.dev_process = {
    state: "running",
    pid: 50000 + Math.floor(Math.random() * 999),
    started_at: Date.now(),
    exit_code: null,
    message: null,
  };
  p.dev_url = `http://127.0.0.1:${p.dev_server?.port ?? 5173}`;
  log(key, `$ ${p.dev_server!.command.replace("{port}", String(p.dev_server!.port ?? 5173))}`, "system");
  log(key, `  VITE v8.3.5  ready in 214 ms`);
  log(key, `  ➜  Local:   http://127.0.0.1:${p.dev_server?.port ?? 5173}/`);
  log(key, `  LARAVEL   plugin v3.2.0`);
}

function processViews(): ProcessView[] {
  return projects.flatMap((p) => {
    const roles: ProcessRole[] = p.dev_server ? ["main", "dev"] : ["main"];
    return roles.map((role) => {
      const info =
        role === "main"
          ? p.process
          : (p.dev_process ?? { state: "stopped", pid: null, started_at: null, exit_code: null, message: null });
      const live = info.state === "running";
      return {
        project: p.name,
        role,
        aux: null,
        label: role === "main" ? (p.command?.split(" ")[0] ?? "server") : "Vite",
        command: role === "main" ? (p.command ?? null) : (p.dev_server?.command ?? null),
        port: role === "main" ? (p.port ?? null) : (p.dev_server?.port ?? null),
        info,
        cpu: live ? Math.round(Math.random() * 40) / 10 : null,
        memory: live ? (role === "main" ? 84_000_000 : 196_000_000) + Math.round(Math.random() * 4e6) : null,
        runtimes: resolveRuntimes(p).flatMap((r) => (r.version ? [`${r.kind} ${r.version.version}`] : [])),
        log_file: `~/.local/share/locdev/logs/sites/${role === "dev" ? `${p.name}-dev` : p.name}.log`,
      };
    });
  });
}

// Keep running projects chatty so the log view has something to show.
setInterval(() => {
  for (const p of projects) {
    if (p.process.state === "running" && Math.random() < 0.3) {
      const ms = (Math.random() * 40).toFixed(2);
      log(p.name, `  ${new Date().toISOString().slice(0, 19).replace("T", " ")} GET / ........ ~ ${ms}ms`);
    }
  }
}, 1500);

const progressListeners = new Set<(p: AppUpdateProgress) => void>();

export const mockBackend: Backend = {
  request: handle,
  // No shell, no splash: the mock daemon is already up.
  boot: async () => false,
  ready: async () => {},
  async start() {
    await delay(600);
    if (running) return false;
    running = true;
    startedAt = Date.now();
    return true;
  },
  async stop() {
    await delay(400);
    if (!running) return false;
    projects.forEach(stop);
    running = false;
    return true;
  },
  async systemPlan(target, action) {
    await delay(150);
    return plan(target, action);
  },
  async systemApply(target, action) {
    await delay(900);
    if (target === "dns") {
      dns.installed = action === "install";
      dns.routing = action === "install" ? "locdev" : "none";
    } else {
      tls.trusted = tls.browsers = action === "install";
    }
    refreshUrls();
  },
  async saveMailFile(_id, _attachment, fileName) {
    return `~/Downloads/${fileName}`;
  },
  async serviceStatus() {
    return { ...service };
  },
  async serviceSet(enabled) {
    await delay(300);
    service.installed = enabled;
    return { ...service };
  },
  async cliStatus() {
    return { ...cli };
  },
  async cliLink(enabled) {
    await delay(300);
    cli.link = enabled ? cli.link_path : null;
    cli.on_path = enabled ? cli.link_path : null;
    return { ...cli };
  },
  async appUpdateCheck(tag) {
    await delay(500);
    return mockAppUpdate(tag);
  },
  async appUpdateInstall(tag) {
    return mockAppInstall(tag, (p) => progressListeners.forEach((l) => l(p)));
  },
  async appRestart() {
    await delay(300);
    window.location.reload();
  },
  async onAppUpdateProgress(listener) {
    progressListeners.add(listener);
    return () => progressListeners.delete(listener);
  },
  async saveTextFile(fileName, text) {
    const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
    const a = Object.assign(document.createElement("a"), { href: url, download: fileName });
    a.click();
    URL.revokeObjectURL(url);
    return `~/Downloads/${fileName}`;
  },
  async info() {
    return {
      version: __APP_VERSION__,
      bundled_daemon: __DAEMON_VERSION__,
      arch: "x86_64",
      commit: "27a5508c41f0",
      built_at: 1_793_836_800,
      profile: "release" as const,
      releases: "https://github.com/trbros/locdev/releases",
      protocol: 2,
      os: "linux",
      endpoint: "/run/user/1000/locdev/locdevd.sock",
      config_dir: "~/.config/locdev",
      log_dir: "~/.local/share/locdev/logs",
    };
  },
};
