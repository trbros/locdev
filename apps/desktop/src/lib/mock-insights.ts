// Mock request metrics, Tinker, PHP extensions, workflow audit/history and
// service packages, for the UI without a daemon. English, like all mocks.

import { handleActions } from "./mock-actions";
import { mockTerminal } from "./mock-scaffold";
import type { Request, RequestMetrics, Response, WorkflowVersion } from "./protocol";

const PATHS = [
  ["GET", "/"],
  ["GET", "/products"],
  ["GET", "/cart"],
  ["POST", "/cart/items"],
  ["GET", "/api/products?page=2"],
  ["GET", "/build/assets/app.js"],
] as const;

const reset = new Map<string, number>();

function metrics(site: string): RequestMetrics {
  const since = reset.get(site) ?? Date.now() - 42 * 60_000;
  const minutes = Math.min(42, Math.floor((Date.now() - since) / 60_000));
  const per_minute = Array.from({ length: minutes }, (_, i) => {
    const requests = 8 + ((i * 37) % 29);
    return { minute: since + i * 60_000, requests, errors: i % 11 === 3 ? 2 : 0 };
  });
  const total = per_minute.reduce((n, m) => n + m.requests, 0);
  const errors = per_minute.reduce((n, m) => n + m.errors, 0);
  return {
    site,
    since: minutes ? since : null,
    total,
    status: {
      informational: minutes ? 1 : 0,
      success: Math.max(0, total - errors - Math.floor(total / 9) - Math.floor(total / 20)),
      redirect: Math.floor(total / 9),
      client_error: Math.floor(total / 20),
      server_error: errors,
    },
    upstream_down: errors ? 1 : 0,
    latency: minutes
      ? { samples: Math.min(total, 1000), avg_ms: 46, p50_ms: 31, p95_ms: 148, p99_ms: 410, max_ms: 1220 }
      : { samples: 0, avg_ms: 0, p50_ms: 0, p95_ms: 0, p99_ms: 0, max_ms: 0 },
    per_minute,
    top_paths: minutes
      ? PATHS.map(([method, path], i) => ({
          method,
          path: path.split("?")[0],
          requests: Math.floor(total / (i + 2)),
          errors: path === "/cart/items" ? errors : 0,
          avg_ms: 20 + i * 17,
          max_ms: 90 + i * 140,
        }))
      : [],
    recent: minutes
      ? Array.from({ length: 12 }, (_, i) => {
          const [method, path] = PATHS[i % PATHS.length];
          const status = i === 4 ? 500 : i === 7 ? 404 : i === 2 ? 302 : 200;
          return { at: Date.now() - i * 4_000, method, path, status, ms: 12 + ((i * 53) % 300), host: `${site}.test` };
        })
      : [],
  };
}

const history: Record<string, WorkflowVersion[]> = {};

export function handleInsights(req: Request): Response | null {
  switch (req.type) {
    case "request_metrics":
      return { type: "request_metrics", ...metrics(req.name) };
    case "reset_request_metrics":
      reset.set(req.name, Date.now());
      return { type: "ok" };
    case "tinker_run": {
      const code = req.code.trim();
      const output = code.includes("User::count")
        ? "= 42\n"
        : code.includes("config(")
          ? '= "Acme Shop"\n'
          : code.includes("throw")
            ? "\n   Exception  Something went wrong.\n\n"
            : `= ${JSON.stringify(code.length)}\n`;
      return {
        type: "tinker",
        output,
        code: code.includes("throw") ? 1 : 0,
        duration_ms: 380,
        timed_out: false,
        truncated: false,
      };
    }
    case "tinker_open":
      return {
        type: "terminal",
        ...mockTerminal(`Tinker: ${req.name}`, [
          "\r\n\x1b[1;38;5;208m▸ Tinker\x1b[0m\r\n\x1b[2m$ php artisan tinker\x1b[0m\r\n",
          "Psy Shell v0.12.9 (PHP 8.4.12 — cli) by Justin Hileman\r\n",
          "> ",
        ]),
      };
    case "php_extensions":
      return {
        type: "php_extensions",
        version: "8.4.12",
        executable: "/home/dev/.local/share/locdev/runtimes/php/8.4.12/bin/php",
        origin: "locdev",
        loaded: [
          "ctype",
          "curl",
          "dom",
          "fileinfo",
          "intl",
          "mbstring",
          "openssl",
          "pdo",
          "pdo_sqlite",
          "tokenizer",
          "xml",
        ],
        required: [
          { name: "mbstring", from: "Laravel" },
          { name: "pdo_pgsql", from: ".env (DB_CONNECTION=pgsql)" },
          { name: "intl", from: "composer.json" },
        ],
        missing: [{ name: "pdo_pgsql", from: ".env (DB_CONNECTION=pgsql)" }],
        recommended: [
          { name: "gd", from: "intervention/image", reason: "to use GD library based image processing." },
          { name: "imagick", from: "intervention/image", reason: "to use Imagick based image processing." },
          {
            name: "pcntl",
            from: "laravel/framework",
            reason: "Required to use all features of the queue worker and console signal trapping.",
          },
        ],
        remedy:
          "LocDev's PHP (StaticPHP) has a fixed set of extensions and none of its sets has pdo_pgsql; use a system PHP that has it (`locdev edit acme-shop --php <version>`).",
        error: null,
      };
    case "php_modules":
      return {
        type: "php_modules",
        executable: req.executable,
        version: "8.4.12",
        origin: "locdev",
        extensions: [
          "Core",
          "ctype",
          "curl",
          "date",
          "dom",
          "intl",
          "mbstring",
          "openssl",
          "pdo_sqlite",
          "sodium",
          "Zend OPcache",
        ].map((name) => ({ name, version: name === "curl" ? null : "8.4.12", zend: name.startsWith("Zend") })),
        available: ["gd", "pdo_pgsql", "xdebug"],
        extension_dir: "/home/dev/.local/share/locdev/runtimes/php/8.4.12/lib/php/extensions",
        ini: "/home/dev/.local/share/locdev/runtimes/php/8.4.12/bin/php.ini",
        scanned: [],
      };
    case "actions_audit":
      return {
        type: "actions_audit",
        actionlint: null,
        files: [
          {
            file: "ci.yml",
            name: "CI",
            findings: [
              {
                level: "warning",
                rule: "missing-setup",
                message: "`pnpm` runs in `test` but isn't on GitHub's runners: add pnpm/action-setup before it.",
                line: 18,
                source: "locdev",
                fix: "- uses: pnpm/action-setup@v4",
              },
              {
                level: "info",
                rule: "practice-timeout",
                message: "Job `test` has no `timeout-minutes`: a hung step runs for GitHub's default of 6 hours.",
                line: 9,
                source: "locdev",
                fix: "timeout-minutes: 15",
              },
            ],
          },
        ],
      };
    case "actions_history": {
      const key = `${req.name}/${req.file}`;
      history[key] ??= [
        { id: `${Date.now() - 3_600_000}-read`, at: Date.now() - 3_600_000, reason: "read", size: 812 },
        { id: `${Date.now() - 86_400_000}-saved`, at: Date.now() - 86_400_000, reason: "saved", size: 790 },
      ];
      return { type: "actions_history", versions: history[key] };
    }
    case "actions_history_version":
      // The current file stands in for the kept version.
      return handleActions({ type: "actions_read", name: req.name, file: req.file });
    case "actionlint_install":
      return { type: "ok" };
    case "install_service_package":
      return {
        type: "terminal",
        ...mockTerminal(`Install ${req.kind} ${req.id}`, [
          `\r\n\x1b[1;38;5;208m▸ ${req.id}\x1b[0m\r\n`,
          "==> Fetching downloads for: mysql@8.4\r\n==> Pouring mysql@8.4--8.4.13.arm64_sequoia.bottle.tar.gz\r\n",
          "\r\n\x1b[2m› Done.\x1b[0m\r\n",
        ]),
      };
    default:
      return null;
  }
}
