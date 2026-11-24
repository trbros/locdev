// PHP settings for the mock backend: two lines, edits kept in memory. The
// real validation, probing and restarts live in the daemon
// (`daemon/phpini.rs`, `locdev-runtimes::php_ini`).

import { DaemonError } from "./errors";
import type { PhpConfigView, PhpDirective, PhpIniChange, PhpRequest, PhpResponse } from "./protocol";

const HEADER =
  "; Managed by LocDev: PHP settings for the sites it runs (Runtimes → PHP).\n; Edit here or with `locdev php set`; a php.ini of your system is never changed.\n";

const DEFS: Omit<PhpDirective, "original" | "effective" | "value">[] = [
  [
    "memory_limit",
    "Memory limit",
    "size",
    "The most memory a script may use; -1 for no limit.",
    "ini.core.php#ini.memory-limit",
  ],
  [
    "upload_max_filesize",
    "Largest upload",
    "size",
    "The largest file a form can upload.",
    "ini.core.php#ini.upload-max-filesize",
  ],
  [
    "post_max_size",
    "Largest POST",
    "size",
    "The largest request body; must be at least the largest upload.",
    "ini.core.php#ini.post-max-size",
  ],
  [
    "max_file_uploads",
    "Files per request",
    "int",
    "How many files one request may upload.",
    "ini.core.php#ini.max-file-uploads",
  ],
  [
    "max_execution_time",
    "Execution time limit",
    "int",
    "Seconds a request may run; 0 for no limit. Commands (artisan, composer) always run without one.",
    "info.configuration.php#ini.max-execution-time",
  ],
  [
    "max_input_time",
    "Input time limit",
    "int",
    "Seconds to read a request's input; -1 uses the execution limit.",
    "info.configuration.php#ini.max-input-time",
  ],
  [
    "max_input_vars",
    "Input variables",
    "int",
    "How many GET, POST and cookie variables a request may have.",
    "info.configuration.php#ini.max-input-vars",
  ],
  [
    "display_errors",
    "Show errors",
    "bool",
    "Print errors in the page.",
    "errorfunc.configuration.php#ini.display-errors",
  ],
  [
    "display_startup_errors",
    "Show startup errors",
    "bool",
    "Print errors that happen while PHP starts.",
    "errorfunc.configuration.php#ini.display-startup-errors",
  ],
  [
    "log_errors",
    "Log errors",
    "bool",
    "Write errors to the log (the site's output in LocDev).",
    "errorfunc.configuration.php#ini.log-errors",
  ],
  [
    "error_reporting",
    "Error level",
    "error_level",
    "Which errors are reported, as constants (E_ALL & ~E_DEPRECATED) or a number.",
    "errorfunc.configuration.php#ini.error-reporting",
  ],
  [
    "date.timezone",
    "Timezone",
    "timezone",
    "The default timezone of date functions (America/Sao_Paulo, UTC).",
    "datetime.configuration.php#ini.date.timezone",
  ],
  [
    "default_socket_timeout",
    "Socket timeout",
    "int",
    "Seconds to wait on socket-based streams (HTTP requests from PHP).",
    "filesystem.configuration.php#ini.default-socket-timeout",
  ],
  [
    "realpath_cache_size",
    "Realpath cache",
    "size",
    "Cache of resolved file paths; bigger helps large projects.",
    "ini.core.php#ini.realpath-cache-size",
  ],
  [
    "opcache.enable_cli",
    "OPcache for the built-in server",
    "bool",
    "Cache compiled code in PHP's built-in server and commands (needs OPcache).",
    "opcache.configuration.php#ini.opcache.enable-cli",
  ],
].map(([key, label, kind, description, page]) => ({
  key,
  label,
  kind: kind as PhpDirective["kind"],
  description,
  docs: `https://www.php.net/manual/en/${page}`,
}));

const ORIGINAL: Record<string, string> = {
  memory_limit: "128M",
  upload_max_filesize: "2M",
  post_max_size: "8M",
  max_file_uploads: "20",
  max_execution_time: "30",
  max_input_time: "-1",
  max_input_vars: "1000",
  display_errors: "Off",
  display_startup_errors: "Off",
  log_errors: "On",
  error_reporting: "22527",
  "date.timezone": "UTC",
  default_socket_timeout: "60",
  realpath_cache_size: "4096K",
  "opcache.enable_cli": "Off",
};

const state: Record<string, { text: string; history: { id: string; at: number; text: string }[] }> = {
  "8.4": {
    text: `${HEADER}memory_limit = 512M\nupload_max_filesize = 64M\npost_max_size = 64M\ndate.timezone = "America/Sao_Paulo"\n`,
    history: [{ id: "1793700000000", at: Date.now() - 3 * 86_400_000, text: `${HEADER}memory_limit = 256M\n` }],
  },
  "8.3": { text: "", history: [] },
};

const VERSIONS: Record<string, string> = { "8.4": "8.4.14", "8.3": "8.3.27" };

function parse(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line || line.startsWith(";") || line.startsWith("[")) continue;
    const at = line.indexOf("=");
    if (at < 0) continue;
    out[line.slice(0, at).trim()] = line
      .slice(at + 1)
      .trim()
      .replace(/^"(.*)"$/, "$1");
  }
  return out;
}

function apply(text: string, values: Record<string, string | null>): string {
  const lines = (text.trim() ? text : HEADER).split("\n").filter((l, i, all) => i < all.length - 1 || l !== "");
  const done = new Set<string>();
  const out: string[] = [];
  for (const line of lines) {
    const key = line.includes("=") && !line.trim().startsWith(";") ? line.split("=")[0].trim() : null;
    if (key && key in values) {
      const v = values[key];
      if (v !== null && !done.has(key)) out.push(`${key} = ${key === "date.timezone" ? `"${v}"` : v}`);
      done.add(key);
    } else out.push(line);
  }
  for (const [key, v] of Object.entries(values)) {
    if (v !== null && !done.has(key)) out.push(`${key} = ${key === "date.timezone" ? `"${v}"` : v}`);
  }
  return `${out.join("\n")}\n`;
}

function view(line: string): PhpConfigView {
  const s = state[line];
  if (!s) throw new DaemonError("app", `no PHP ${line} installed`, "NOT_FOUND");
  const ours = parse(s.text);
  const directives = DEFS.map((d) => ({
    ...d,
    original: ORIGINAL[d.key],
    value: ours[d.key] ?? null,
    effective: ours[d.key] ?? ORIGINAL[d.key],
  }));
  const size = (k: string) => {
    const v = (ours[k] ?? ORIGINAL[k]).toUpperCase();
    const n = Number.parseInt(v, 10);
    return v.endsWith("G") ? n * 1024 : v.endsWith("K") ? n / 1024 : n;
  };
  const warnings =
    size("upload_max_filesize") > size("post_max_size")
      ? ["upload_max_filesize is larger than post_max_size: uploads above post_max_size are dropped whole."]
      : [];
  return {
    line,
    version: VERSIONS[line],
    executable: `~/.local/share/locdev/runtimes/php/${VERSIONS[line]}/bin/php`,
    sapi: "cli",
    loaded_ini: `/etc/php/${line}/cli/php.ini`,
    scanned: [`/etc/php/${line}/cli/conf.d/10-opcache.ini`, `/etc/php/${line}/cli/conf.d/20-intl.ini`],
    file: `~/.config/locdev/php/${line}/90-locdev.ini`,
    text: s.text,
    hash: String(s.text.length),
    directives,
    warnings,
    sites: line === "8.4" ? ["acme-shop"] : [],
    history: s.history.map(({ id, at, text }) => ({ id, at, lines: text.split("\n").length - 1 })),
  };
}

function next(line: string, change: PhpIniChange): string {
  const s = state[line];
  switch (change.kind) {
    case "set":
      for (const [key, v] of Object.entries(change.values)) {
        if (v !== null && key.endsWith("_size") && !/^(-1|\d+[KMG]?)$/i.test(v))
          throw new DaemonError("app", `${key}: '${v}' is not a size like 128M, 2G or -1`, "INVALID_INPUT");
      }
      return apply(s.text, change.values);
    case "text":
      if (change.hash !== String(s.text.length))
        throw new DaemonError("app", "LocDev's file changed since it was opened; reload it and try again", "CONFLICT");
      if (/==/.test(change.text))
        throw new DaemonError("app", "syntax error, unexpected '=' in 90-locdev.ini on line 1", "INVALID_INPUT");
      return change.text;
    case "restore":
      return s.history.find((h) => h.id === change.id)?.text ?? s.text;
    case "reset":
      return "";
  }
}

export function handlePhp(req: PhpRequest): PhpResponse {
  switch (req.type) {
    case "php_config_lines":
      return {
        type: "php_lines",
        lines: Object.keys(state).map((line) => ({
          line,
          version: VERSIONS[line],
          executable: view(line).executable,
          configured: Object.keys(parse(state[line].text)).length > 0,
        })),
      };
    case "php_config":
      return { type: "php_config", config: view(req.line), restarted: [] };
    case "php_ini_read":
      return {
        type: "php_ini_file",
        path: req.path,
        text: `[PHP]\n; The system's php.ini (read-only in LocDev)\nmemory_limit = 128M\nupload_max_filesize = 2M\npost_max_size = 8M\ndisplay_errors = Off\n`,
      };
    case "php_config_change": {
      const before = state[req.line]?.text ?? "";
      const after = next(req.line, req.change);
      const sites = view(req.line).sites;
      if (req.dry_run) {
        return {
          type: "php_config",
          config: view(req.line),
          preview: { before, after, restart: sites, warnings: [] },
          restarted: [],
        };
      }
      if (before) state[req.line].history.unshift({ id: String(Date.now()), at: Date.now(), text: before });
      state[req.line].text = after;
      return { type: "php_config", config: view(req.line), restarted: sites };
    }
  }
}
