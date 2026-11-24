// Config Files for the mock backend: an in-memory Vite project. The real
// parsing, version checks and safe saving live in `locdev-config`.

import { DaemonError } from "./errors";
import type {
  ConfigFileView,
  ConfigFinding,
  ConfigHistoryEntry,
  ConfigOption,
  ConfigRequest,
  ConfigResponse,
} from "./protocol";

const files: Record<string, string> = {
  "vite.config.ts": `import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173, // LocDev proxies acme-shop.test here
    host: true,
  },
});
`,
  "tsconfig.json": `{
  // Shared by the app and the tests
  "compilerOptions": {
    "target": "ES2022",
    "strict": true,
    "moduleResolution": "bundler"
  },
  "include": ["src"]
}
`,
  "package.json": `{
  "name": "acme-shop",
  "type": "module",
  "private": true
}
`,
  "src/index.css": `@import "tailwindcss";\n\n@theme {\n  --color-brand: oklch(0.7 0.18 40);\n}\n`,
};
const history: Record<string, (ConfigHistoryEntry & { content: string })[]> = {};

function hash(text: string) {
  let h = 0;
  for (const c of text) h = (h * 31 + c.charCodeAt(0)) | 0;
  return `mock-${(h >>> 0).toString(16)}`;
}

const option = (
  o: Partial<ConfigOption> & Pick<ConfigOption, "key" | "section" | "label" | "kind" | "state">,
): ConfigOption => ({
  description: "",
  choices: [],
  default: null,
  example: null,
  docs: null,
  versions: null,
  editable: o.state.state !== "code",
  blocked: o.state.state === "code" ? "computed in code; edit the text instead" : null,
  ...o,
});

function view(path: string): ConfigFileView {
  const content = files[path];
  if (content === undefined)
    throw new DaemonError("app", `${path} isn't a config file LocDev manages in this project`, "NOT_FOUND");
  const port = Number(/port: (\d+)/.exec(files["vite.config.ts"])?.[1] ?? 5173);
  const strictPort = /strictPort: (true|false)/.exec(files["vite.config.ts"])?.[1];
  const base = { content, hash: hash(content), notes: [] as string[], visual_blocked: null, syntax_error: null };
  if (path === "vite.config.ts")
    return {
      ...base,
      file: {
        path,
        kind: "vite",
        label: "Vite",
        format: "js",
        package: "vite",
        version: "7.1.3",
        docs: "https://vite.dev/config/",
        visual: true,
      },
      notes: ["plugins: react(), tailwindcss()"],
      options: [
        option({
          key: "server.port",
          section: "Server",
          label: "Port",
          kind: "number",
          state: { state: "set", value: port },
          default: 5173,
          description: "Port of the dev server. LocDev proxies the site's domain to it.",
          docs: "https://vite.dev/config/server-options#server-port",
        }),
        option({
          key: "server.host",
          section: "Server",
          label: "Host",
          kind: "string",
          state: { state: "set", value: true },
          default: "localhost",
          description: "Address the dev server listens on. `true` or `0.0.0.0` exposes it to the network.",
        }),
        option({
          key: "server.strictPort",
          section: "Server",
          label: "Strict port",
          kind: "bool",
          state: strictPort ? { state: "set", value: strictPort === "true" } : { state: "missing" },
          default: false,
          description: "Fail instead of trying the next port when the port is taken.",
        }),
        option({
          key: "build.sourcemap",
          section: "Build",
          label: "Source maps",
          kind: "choice",
          choices: [false, true, "inline", "hidden"],
          state: { state: "missing" },
          default: false,
          description: "Source maps for the production build.",
        }),
        option({
          key: "build.outDir",
          section: "Build",
          label: "Output folder",
          kind: "string",
          state: { state: "code", code: 'resolve(__dirname, "dist")' },
          default: "dist",
          description: "Where the production build is written.",
        }),
      ],
    };
  if (path === "tsconfig.json")
    return {
      ...base,
      file: {
        path,
        kind: "tsconfig",
        label: "TypeScript",
        format: "jsonc",
        package: "typescript",
        version: "5.9.2",
        docs: "https://www.typescriptlang.org/tsconfig/",
        visual: true,
      },
      options: [
        option({
          key: "compilerOptions.strict",
          section: "Type checking",
          label: "Strict",
          kind: "bool",
          state: { state: "set", value: true },
          default: false,
          description: "Turns on the whole family of strict checks.",
        }),
        option({
          key: "compilerOptions.erasableSyntaxOnly",
          section: "Modules",
          label: "Erasable syntax only",
          kind: "bool",
          state: { state: "missing" },
          default: false,
          versions: "typescript >= 5.8.0",
          description: "Refuse syntax that type stripping can't remove.",
          editable: false,
          blocked: "needs typescript 5.8.0 or newer (this project has 5.7.3)",
        }),
      ],
    };
  if (path === "package.json")
    return {
      ...base,
      file: {
        path,
        kind: "package",
        label: "package.json",
        format: "json",
        package: null,
        version: null,
        docs: "https://docs.npmjs.com/cli/configuring-npm/package-json",
        visual: true,
      },
      options: [
        option({
          key: "type",
          section: "Package",
          label: "Module type",
          kind: "choice",
          choices: ["module", "commonjs"],
          state: { state: "set", value: "module" },
          default: "commonjs",
        }),
      ],
    };
  return {
    ...base,
    file: {
      path,
      kind: "tailwind-css",
      label: "Tailwind CSS (CSS-first config)",
      format: "css",
      package: "tailwindcss",
      version: "4.1.12",
      docs: "https://tailwindcss.com/docs/theme",
      visual: false,
    },
    options: [],
  };
}

function applyChange(path: string, change: Extract<ConfigRequest, { type: "config_save" }>["change"]) {
  if (change.type === "text") return change.content;
  const text = files[path];
  const literal = typeof change.value === "string" ? `"${change.value}"` : JSON.stringify(change.value);
  if (path === "vite.config.ts" && change.key === "server.port") return text.replace(/port: \d+/, `port: ${literal}`);
  if (path === "vite.config.ts" && change.key === "server.strictPort")
    return /strictPort:/.test(text)
      ? text.replace(/strictPort: \w+/, `strictPort: ${literal}`)
      : text.replace("    host: true,\n", `    host: true,\n    strictPort: ${literal},\n`);
  if (path === "vite.config.ts" && change.key === "server.host")
    return text.replace(/host: [^,]+,/, `host: ${literal},`);
  if (path === "tsconfig.json" && change.key === "compilerOptions.strict")
    return text.replace(/"strict": \w+/, `"strict": ${literal}`);
  return text;
}

function diff(before: string, after: string, path: string) {
  if (before === after) return "";
  const a = before.split("\n");
  const b = after.split("\n");
  const out = [`--- ${path}`, `+++ ${path}`, `@@ -1,${a.length} +1,${b.length} @@`];
  const max = Math.max(a.length, b.length);
  for (let i = 0; i < max; i++) {
    if (a[i] === b[i]) out.push(` ${a[i] ?? ""}`);
    else {
      if (a[i] !== undefined) out.push(`-${a[i]}`);
      if (b[i] !== undefined) out.push(`+${b[i]}`);
    }
  }
  return `${out.join("\n")}\n`;
}

const findings = (): ConfigFinding[] =>
  /host: true/.test(files["vite.config.ts"])
    ? [
        {
          rule: "vite.host-exposed",
          kind: "security",
          file: "vite.config.ts",
          key: "server.host",
          title: "The dev server listens on every network interface",
          detail:
            "`server.host` set to `true`/`0.0.0.0` makes the dev server reachable from other machines on the network. LocDev reaches it on 127.0.0.1, so exposing it isn't needed.",
          applies: "vite (any version)",
          source: "https://vite.dev/config/server-options#server-host",
          fix: ["server.host", "localhost"],
        },
      ]
    : [];

export function handleConfig(req: ConfigRequest): ConfigResponse {
  switch (req.type) {
    case "config_files":
      return { type: "config_files", files: Object.keys(files).map((p) => view(p).file) };
    case "config_file":
      return { type: "config_file", ...view(req.path) };
    case "config_preview": {
      const before = view(req.path).content;
      const after = applyChange(req.path, req.change);
      const broken =
        req.path.endsWith(".json") &&
        (() => {
          try {
            JSON.parse(after.replace(/\/\/.*$/gm, ""));
            return false;
          } catch {
            return true;
          }
        })();
      return {
        type: "config_preview",
        content: after,
        diff: diff(before, after, req.path),
        syntax_error: broken ? "invalid JSON" : null,
        // A changed line that keeps its comment isn't a loss (same rule as the daemon).
        removed_comments: before
          .split("\n")
          .filter((l) => l.includes("//") && !after.includes(l.slice(l.indexOf("//")))),
      };
    }
    case "config_save":
    case "config_revert": {
      const current = view(req.path);
      if (current.hash !== req.base_hash)
        throw new DaemonError("app", `${req.path} changed since you opened it; review it again`, "CONFLICT");
      const next =
        req.type === "config_save"
          ? applyChange(req.path, req.change)
          : (history[req.path] ?? []).find((h) => h.id === req.id)?.content;
      if (next === undefined)
        throw new DaemonError("app", `no saved version ${"id" in req ? req.id : ""}`, "NOT_FOUND");
      const reason =
        req.type === "config_revert"
          ? `revert to ${req.id}`
          : req.change.type === "set"
            ? `set ${req.change.key} = ${JSON.stringify(req.change.value)}`
            : "text edit";
      history[req.path] = [
        { id: String(Date.now()), at: Date.now(), size: current.content.length, reason, content: current.content },
        ...(history[req.path] ?? []),
      ];
      files[req.path] = next;
      return { type: "config_saved", hash: hash(next) };
    }
    case "config_history":
      return { type: "config_history", entries: (history[req.path] ?? []).map(({ content: _, ...e }) => e) };
    case "config_history_version": {
      const entry = (history[req.path] ?? []).find((h) => h.id === req.id);
      if (!entry) throw new DaemonError("app", `no saved version ${req.id}`, "NOT_FOUND");
      return { type: "config_history_version", content: entry.content };
    }
    case "config_check":
      return { type: "config_findings", findings: findings() };
  }
}
