// The Actions builder for the browser mock (`mock.ts`). A compact version of
// the daemon's renderer and a few of its checks: enough to drive the UI. The
// real rules live in `locdev-actions`.

import { DaemonError } from "./errors";
import type {
  ActionVersion,
  ActionsRequest,
  ActionsResponse,
  Finding,
  Permissions,
  TemplateInfo,
  Validation,
  WorkflowFilter,
  WorkflowModel,
  WorkflowStep,
} from "./protocol";

const VERSIONS: ActionVersion[] = [
  { repo: "actions/checkout", version: "v7", purpose: "Check out the repository" },
  { repo: "actions/setup-node", version: "v7", purpose: "Install Node.js (with dependency caching)" },
  { repo: "actions/cache", version: "v6", purpose: "Cache files between runs" },
  { repo: "actions/upload-artifact", version: "v7", purpose: "Keep build output from a job" },
  { repo: "actions/configure-pages", version: "v6", purpose: "Prepare a GitHub Pages deployment" },
  { repo: "actions/upload-pages-artifact", version: "v5", purpose: "Upload the site for GitHub Pages" },
  { repo: "actions/deploy-pages", version: "v5", purpose: "Publish to GitHub Pages" },
  {
    repo: "actions/dependency-review-action",
    version: "v5",
    purpose: "Block pull requests adding vulnerable dependencies",
  },
  { repo: "github/codeql-action", version: "v4", purpose: "CodeQL code scanning" },
  { repo: "shivammathur/setup-php", version: "v2", purpose: "Install PHP, extensions and Composer" },
  { repo: "pnpm/action-setup", version: "v6", purpose: "Install pnpm" },
  { repo: "softprops/action-gh-release", version: "v3", purpose: "Create a GitHub release" },
];
let checkedAt: number | null = null;

const TEMPLATES: TemplateInfo[] = [
  {
    id: "php-ci",
    title: "PHP / Laravel CI",
    description: "Set up PHP and Composer, build the front-end if there is one, and run the test suite.",
    file: "ci.yml",
    recommended: true,
    reason: "composer.json (Laravel)",
  },
  {
    id: "dependency-review",
    title: "Dependency review",
    description: "Fails pull requests that add dependencies with known vulnerabilities.",
    file: "dependency-review.yml",
    recommended: true,
    reason: "checks npm and Composer changes in pull requests",
  },
  {
    id: "codeql",
    title: "CodeQL code scanning",
    description: "Static analysis for security bugs on pushes, pull requests and weekly.",
    file: "codeql.yml",
    recommended: true,
    reason: "JavaScript/TypeScript code",
  },
  {
    id: "node-ci",
    title: "Node.js CI",
    description:
      "Install with the project's package manager, then lint, type-check, test and build (whichever scripts exist).",
    file: "ci.yml",
    recommended: false,
    reason: "PHP CI covers the front-end build too",
  },
  {
    id: "pages",
    title: "Deploy to GitHub Pages",
    description: "Build the site and publish it to GitHub Pages on every push to the main branch.",
    file: "pages.yml",
    recommended: false,
    reason: "the project needs a server; check that it can export static files",
  },
  {
    id: "release",
    title: "Release on tag",
    description: "Create a GitHub release with generated notes when a v* tag is pushed.",
    file: "release.yml",
    recommended: false,
    reason: "for projects that publish versions",
  },
];

const files: Record<string, Record<string, string>> = { "acme-shop": {} };

const step = (s: Partial<WorkflowStep>): WorkflowStep => ({
  id: null,
  name: null,
  uses: null,
  with: {},
  run: null,
  if: null,
  working_directory: null,
  env: {},
  ...s,
});
const uses = (repo: string, w: Record<string, string> = {}, name: string | null = null) =>
  step({ uses: `${repo}@${VERSIONS.find((v) => v.repo === repo)?.version ?? "main"}`, with: w, name });
const run = (name: string, command: string) => step({ name, run: command });
const scopes = (s: Record<string, string>): Permissions => ({ kind: "scopes", scopes: s });
const branches = (b: string[]): WorkflowFilter => ({ branches: b, tags: [], paths: [] });

function template(id: string): WorkflowModel {
  const base: WorkflowModel = {
    name: "CI",
    on: {
      push: branches(["main"]),
      pull_request: branches([]),
      workflow_dispatch: false,
      schedule: [],
      release: false,
    },
    permissions: scopes({ contents: "read" }),
    concurrency: { group: "${{ github.workflow }}-${{ github.ref }}", cancel_in_progress: true },
    env: {},
    jobs: [],
  };
  const job = (id: string, steps: WorkflowStep[], extra: Partial<WorkflowModel["jobs"][0]> = {}) => ({
    id,
    name: null,
    runs_on: "ubuntu-latest",
    needs: [],
    if: null,
    permissions: null,
    environment: null,
    timeout_minutes: 20,
    matrix: {},
    env: {},
    steps,
    ...extra,
  });
  switch (id) {
    case "php-ci":
      return {
        ...base,
        jobs: [
          job("tests", [
            uses("actions/checkout"),
            uses("shivammathur/setup-php", {
              "php-version": "8.4",
              extensions: "mbstring, intl, pdo_sqlite, bcmath, fileinfo",
              coverage: "none",
              tools: "composer:v2",
            }),
            run("Install Composer dependencies", "composer install --no-interaction --prefer-dist --no-progress"),
            uses("actions/setup-node", { cache: "npm", "node-version": "lts/*" }),
            run("Install dependencies", "npm ci"),
            run("Build front-end", "npm run build"),
            run("Prepare the environment", "cp .env.example .env\nphp artisan key:generate\n"),
            run("Test", "php artisan test"),
          ]),
        ],
      };
    case "dependency-review":
      return {
        ...base,
        name: "Dependency review",
        on: { ...base.on, push: null },
        concurrency: null,
        jobs: [
          job(
            "review",
            [uses("actions/checkout"), uses("actions/dependency-review-action", { "fail-on-severity": "high" })],
            { timeout_minutes: null },
          ),
        ],
      };
    default:
      return {
        ...base,
        jobs: [
          job("ci", [
            uses("actions/checkout"),
            uses("actions/setup-node", { cache: "npm", "node-version": "lts/*" }),
            run("Install dependencies", "npm ci"),
            run("Test", "npm run test"),
          ]),
        ],
      };
  }
}

// ── Rendering (the same layout as `locdev_actions::yaml`) ───────────────────

const RESERVED = ["true", "false", "yes", "no", "on", "off", "y", "n", "null", "~", ""];
function scalar(v: string): string {
  const plain =
    !RESERVED.includes(v.toLowerCase()) &&
    !/^[?:,[\]{}#&*!|>'"%@`\s]/.test(v) &&
    !v.startsWith("- ") &&
    !/(: | #|:$|\s$)/.test(v) &&
    Number.isNaN(Number(v));
  return plain ? v : `'${v.replaceAll("'", "''")}'`;
}
const flow = (items: string[]) => `[${items.map((i) => (/[,[\]{}]/.test(i) ? `'${i}'` : scalar(i))).join(", ")}]`;

function permissions(p: Permissions, pad: string): string {
  if (p.kind === "read_all") return `${pad}permissions: read-all\n`;
  if (p.kind === "write_all") return `${pad}permissions: write-all\n`;
  const entries = Object.entries(p.scopes);
  if (!entries.length) return `${pad}permissions: {}\n`;
  return `${pad}permissions:\n${entries.map(([k, v]) => `${pad}  ${k}: ${v}\n`).join("")}`;
}

function render(w: WorkflowModel): string {
  let out = w.name ? `name: ${scalar(w.name)}\n\n` : "";
  out += "on:\n";
  const filter = (event: string, f: WorkflowFilter) => {
    out += `  ${event}:\n`;
    if (f.branches.length) out += `    branches: ${flow(f.branches)}\n`;
    if (f.tags.length) out += `    tags: ${flow(f.tags)}\n`;
    if (f.paths.length) out += `    paths: ${flow(f.paths)}\n`;
  };
  if (w.on.push) filter("push", w.on.push);
  if (w.on.pull_request) filter("pull_request", w.on.pull_request);
  if (w.on.workflow_dispatch) out += "  workflow_dispatch:\n";
  if (w.on.schedule.length) out += `  schedule:\n${w.on.schedule.map((c) => `    - cron: ${scalar(c)}\n`).join("")}`;
  if (w.on.release) out += "  release:\n    types: [published]\n";
  if (w.permissions) out += "\n" + permissions(w.permissions, "");
  if (w.concurrency) {
    out += `\nconcurrency:\n  group: ${scalar(w.concurrency.group)}\n  cancel-in-progress: ${w.concurrency.cancel_in_progress}\n`;
  }
  out += "\njobs:\n";
  w.jobs.forEach((j, i) => {
    if (i) out += "\n";
    out += `  ${j.id}:\n`;
    if (j.name) out += `    name: ${scalar(j.name)}\n`;
    out += `    runs-on: ${scalar(j.runs_on)}\n`;
    if (j.needs.length) out += `    needs: ${flow(j.needs)}\n`;
    if (j.if) out += `    if: ${scalar(j.if)}\n`;
    if (j.permissions) out += permissions(j.permissions, "    ");
    if (j.environment) out += `    environment: ${scalar(j.environment)}\n`;
    if (j.timeout_minutes != null) out += `    timeout-minutes: ${j.timeout_minutes}\n`;
    out += "    steps:\n";
    for (const s of j.steps) {
      const lines: string[] = [];
      if (s.name) lines.push(`name: ${scalar(s.name)}`);
      if (s.if) lines.push(`if: ${scalar(s.if)}`);
      if (s.uses) lines.push(`uses: ${scalar(s.uses)}`);
      if (s.run != null) {
        if (s.run.includes("\n")) {
          lines.push(`run: ${s.run.endsWith("\n") ? "|" : "|-"}`);
          for (const l of s.run.replace(/\n+$/, "").split("\n")) lines.push(`  ${l}`);
        } else lines.push(`run: ${scalar(s.run)}`);
      }
      const w = Object.entries(s.with).sort(([a], [b]) => a.localeCompare(b));
      if (w.length) lines.push("with:", ...w.map(([k, v]) => `  ${k}: ${scalar(v)}`));
      out += lines.map((l, i) => (i ? `        ${l}` : `      - ${l}`)).join("\n") + "\n";
    }
  });
  return out;
}

const UNTRUSTED = [
  "github.event.issue.title",
  "github.event.pull_request.title",
  "github.event.pull_request.body",
  "github.event.comment.body",
  "github.head_ref",
  "github.event.head_commit.message",
];

function validate(text: string, workflow: WorkflowModel | null): Validation {
  const findings: Finding[] = [];
  const lines = text.split("\n");
  const lineOf = (needle: string) => {
    const i = lines.findIndex((l) => l.includes(needle));
    return i >= 0 ? i + 1 : null;
  };
  const add = (level: Finding["level"], rule: string, message: string, near?: string) =>
    findings.push({ level, rule, message, line: near ? lineOf(near) : null, source: "locdev" });
  if (!/^on:/m.test(text)) add("error", "structure", "No `on:`: the workflow never runs.");
  if (!/^permissions:/m.test(text) && !/^    permissions:/m.test(text)) {
    add(
      "warning",
      "permissions",
      "No `permissions:`: the token gets the repository's default, which can be write access to everything. Start from `contents: read` and add what jobs need.",
    );
  }
  for (const u of UNTRUSTED) {
    const m = lines.find((l) => l.includes(u) && /run:|^\s{10}/.test(l));
    if (m)
      add(
        "error",
        "injection",
        `\`${u}\` is pasted into a shell script: anyone who controls it can run commands. Pass it through \`env:\` and use "$VAR" in the script.`,
        u,
      );
  }
  for (const m of text.matchAll(/uses: ([\w./-]+)(@[\w.-]+)?/g)) {
    if (!m[2]) add("error", "unpinned-action", `\`${m[1]}\` has no version: add @v… (or a commit SHA).`, m[0]);
    else if (["@main", "@master"].includes(m[2]))
      add(
        "warning",
        "unpinned-action",
        `\`${m[1]}${m[2]}\` follows a branch. Use a release tag or a commit SHA.`,
        m[0],
      );
    else if (!/^(actions|github)\//.test(m[1]))
      add(
        "info",
        "third-party-action",
        `\`${m[1]}\` is a third-party action pinned to a tag, which its owner can move. Pinning the commit SHA is safer.`,
        m[0],
      );
  }
  findings.sort(
    (a, b) => ["error", "warning", "info"].indexOf(a.level) - ["error", "warning", "info"].indexOf(b.level),
  );
  return {
    findings,
    actionlint: null,
    workflow,
    unsupported: workflow ? null : "the mock backend can't read edited YAML back into the visual editor",
  };
}

/** YAML the mock rendered, to switch back to the visual editor. */
const rendered = new Map<string, WorkflowModel>();

const hash = (text: string) => `${text.length}-${[...text].reduce((h, c) => (h * 31 + c.charCodeAt(0)) | 0, 7)}`;

function diff(old: string, next: string, path: string, exists: boolean): string {
  const a = old ? old.replace(/\n$/, "").split("\n") : [];
  const b = next.replace(/\n$/, "").split("\n");
  let out = `--- ${exists ? path : "/dev/null"}\n+++ ${path}\n@@ -${a.length ? 1 : 0},${a.length} +1,${b.length} @@\n`;
  const same = new Set(a.filter((l) => b.includes(l)));
  for (const l of a) if (!same.has(l)) out += `-${l}\n`;
  for (const l of b) out += same.has(l) ? ` ${l}\n` : `+${l}\n`;
  return out;
}

export function handleActions(req: ActionsRequest): ActionsResponse {
  switch (req.type) {
    case "actions_overview":
      return {
        type: "actions_overview",
        templates: TEMPLATES,
        existing: Object.keys(files[req.name] ?? {}).map((file) => ({
          file,
          name: /^name: (.*)$/m.exec(files[req.name][file])?.[1] ?? null,
        })),
        actions: VERSIONS,
        versions_checked_at: checkedAt,
        actionlint: null,
      };
    case "actions_template": {
      const t = TEMPLATES.find((t) => t.id === req.template);
      if (!t) throw new DaemonError("app", `no template named '${req.template}'`, "NOT_FOUND");
      return { type: "actions_workflow", workflow: template(t.id), file: t.file };
    }
    case "actions_render": {
      const yaml = render(req.workflow);
      rendered.set(yaml, structuredClone(req.workflow));
      return { type: "actions_yaml", yaml, validation: validate(yaml, req.workflow) };
    }
    case "actions_validate":
      return { type: "actions_validation", ...validate(req.yaml, rendered.get(req.yaml) ?? null) };
    case "actions_read": {
      const yaml = files[req.name]?.[req.file];
      if (yaml == null) throw new DaemonError("app", `.github/workflows/${req.file}: not found`, "CONFLICT");
      return { type: "actions_yaml", yaml, validation: validate(yaml, rendered.get(yaml) ?? null) };
    }
    case "actions_plan": {
      const current = files[req.name]?.[req.file];
      const path = `.github/workflows/${req.file}`;
      return {
        type: "actions_plan",
        path,
        exists: current != null,
        current_hash: current != null ? hash(current) : null,
        diff: diff(current ?? "", req.yaml, path, current != null),
        validation: validate(req.yaml, rendered.get(req.yaml) ?? null),
      };
    }
    case "actions_save": {
      const site = (files[req.name] ??= {});
      const current = site[req.file];
      const path = `.github/workflows/${req.file}`;
      if (validate(req.yaml, null).findings.some((f) => f.level === "error")) {
        throw new DaemonError("app", "fix the errors first", "INVALID_INPUT");
      }
      if (current != null && req.replace == null) {
        throw new DaemonError(
          "app",
          `${path} already exists; review the changes and replace it explicitly`,
          "CONFLICT",
        );
      }
      if (current != null && req.replace !== hash(current)) {
        throw new DaemonError("app", `${path} changed since you reviewed it; review the changes again`, "CONFLICT");
      }
      site[req.file] = req.yaml;
      return { type: "actions_saved", path };
    }
    case "action_versions":
      return { type: "action_versions", actions: VERSIONS, checked_at: checkedAt };
    case "refresh_action_versions":
      checkedAt = Date.now();
      return { type: "action_versions", actions: VERSIONS, checked_at: checkedAt };
    default:
      // Audit, history and actionlint live in mock-insights.
      throw new Error(`mock-actions: ${req.type}`);
  }
}
