// New projects for the mock backend: the real catalog (generated from
// `locdev-scaffold`), a few tools, plans for the main starters and a
// terminal that plays a scripted run. The real planning and the PTY live in
// the daemon (`daemon/scaffold.rs`, `daemon/terminal.rs`).

import starters from "@/generated/starters.json";

import { DaemonError } from "./errors";
import type {
  NewProject,
  PlannedFile,
  PackageHit,
  ProjectPlan,
  Request,
  Response,
  Starter,
  Step,
  TerminalInfo,
  Tool,
} from "./protocol";

const catalog = starters as Starter[];

const tools: Tool[] = [
  {
    id: "php",
    name: "PHP",
    description: "Runs Laravel, Symfony and WordPress.",
    docs: "https://www.php.net/downloads",
    installed: true,
    version: "PHP 8.4.14 (cli)",
    install: { kind: "runtime", runtime: "php" },
    used_by: ["Laravel", "Symfony", "WordPress (Bedrock)"],
  },
  {
    id: "composer",
    name: "Composer",
    description: "PHP's package manager; creates Laravel, Symfony and Bedrock projects.",
    docs: "https://getcomposer.org/download/",
    installed: true,
    version: "Composer version 2.9.2",
    install: { kind: "runtime", runtime: "composer" },
    used_by: ["Laravel", "Symfony", "WordPress (Bedrock)"],
  },
  {
    id: "laravel",
    name: "Laravel installer",
    description: "`laravel new`, with starter kits, testing and database choices.",
    docs: "https://laravel.com/docs/installation#installing-php",
    installed: false,
    install: {
      kind: "command",
      steps: [
        {
          label: "Install Laravel installer",
          program: "composer",
          args: ["global", "require", "laravel/installer"],
          cwd: "~",
        },
      ],
    },
    used_by: ["Laravel"],
  },
  {
    id: "symfony",
    name: "Symfony CLI",
    description: "`symfony new`; Composer works without it.",
    docs: "https://symfony.com/download",
    installed: false,
    used_by: ["Symfony"],
  },
  {
    id: "node",
    name: "Node.js",
    description: "Runs the JavaScript starters and front-end builds.",
    docs: "https://nodejs.org/en/download",
    installed: true,
    version: "v24.11.1",
    install: { kind: "runtime", runtime: "node" },
    used_by: [],
  },
  {
    id: "npm",
    name: "npm",
    description: "Comes with Node.js.",
    docs: "https://docs.npmjs.com/",
    installed: true,
    version: "11.6.2",
    install: { kind: "runtime", runtime: "node" },
    used_by: ["Next.js", "Vite", "Nuxt", "Astro", "SvelteKit", "NestJS"],
  },
  {
    id: "pnpm",
    name: "pnpm",
    description: "Fast, disk-efficient package manager.",
    docs: "https://pnpm.io/installation",
    installed: true,
    version: "10.20.0",
    install: { kind: "runtime", runtime: "pnpm" },
    used_by: ["Next.js", "Vite", "Nuxt", "Astro", "SvelteKit", "NestJS"],
  },
  {
    id: "bun",
    name: "Bun",
    description: "JavaScript runtime and package manager.",
    docs: "https://bun.com/docs/installation",
    installed: false,
    used_by: ["Next.js", "Vite", "Nuxt", "Astro", "SvelteKit", "NestJS"],
  },
  {
    id: "python",
    name: "Python",
    description: "Runs Django, Flask and FastAPI; each project gets its own virtual environment.",
    docs: "https://www.python.org/downloads/",
    installed: true,
    version: "Python 3.12.3",
    used_by: ["Django", "Flask", "FastAPI"],
  },
  {
    id: "uv",
    name: "uv",
    description: "Fast Python package and project manager; optional.",
    docs: "https://docs.astral.sh/uv/getting-started/installation/",
    installed: true,
    version: "uv 0.9.8",
    used_by: [],
  },
  {
    id: "pipx",
    name: "pipx",
    description: "Installs Python command-line tools in their own environments; optional.",
    docs: "https://pipx.pypa.io/stable/installation/",
    installed: false,
    used_by: [],
  },
  {
    id: "poetry",
    name: "Poetry",
    description: "Python dependency management and packaging; optional.",
    docs: "https://python-poetry.org/docs/#installation",
    installed: false,
    used_by: [],
  },
  {
    id: "cargo",
    name: "Cargo",
    description: "Rust's build tool and package manager; creates the Axum and Actix Web starters.",
    docs: "https://www.rust-lang.org/tools/install",
    installed: true,
    version: "cargo 1.91.0",
    used_by: ["Rust web server"],
  },
  {
    id: "rustup",
    name: "rustup",
    description: "Installs and updates Rust toolchains (and Cargo with them).",
    docs: "https://rustup.rs/",
    installed: true,
    version: "rustup 1.28.2",
    used_by: [],
  },
  {
    id: "wp",
    name: "WP-CLI",
    description: "WordPress's command line; downloads WordPress for new projects.",
    docs: "https://wp-cli.org/#installing",
    installed: false,
    used_by: ["WordPress"],
  },
  {
    id: "git",
    name: "Git",
    description: "Version control; starters can initialize a repository.",
    docs: "https://git-scm.com/downloads",
    installed: true,
    version: "git version 2.51.0",
    used_by: [],
  },
  {
    id: "rails",
    name: "Rails",
    description: "`rails new`; needs Ruby.",
    docs: "https://guides.rubyonrails.org/install_ruby_on_rails.html",
    installed: false,
    install: {
      kind: "command",
      steps: [{ label: "Install Rails", program: "gem", args: ["install", "rails"], cwd: "~" }],
    },
    used_by: ["Ruby on Rails"],
  },
  {
    id: "dotnet",
    name: ".NET SDK",
    description: "`dotnet new` for ASP.NET Core.",
    docs: "https://dotnet.microsoft.com/download",
    installed: false,
    used_by: ["ASP.NET Core"],
  },
];

const opt = (p: NewProject, s: Starter, id: string) =>
  p.options[id] ?? s.options.find((o) => o.id === id)?.default ?? "";

function plan(p: NewProject): ProjectPlan {
  const s = catalog.find((x) => x.id === p.starter);
  if (!s) throw new DaemonError("app", `unknown starter '${p.starter}'`);
  if (!/^[a-z0-9][a-z0-9_-]{0,62}$/.test(p.name))
    throw new DaemonError("app", "the name may only have lowercase letters, digits, '-' and '_'");
  const dir = `${p.parent.replace(/\/$/, "")}/${p.name}`;
  const step = (label: string, program: string, args: string[], cwd = p.parent): Step => ({
    label,
    program,
    args,
    cwd,
  });
  const steps: Step[] = [];
  const files: PlannedFile[] = [];
  const pm = p.method === "pnpm" ? "pnpm" : p.method === "bun" ? "bun" : "npm";
  if (s.id === "laravel") {
    // Same branching as `laravel new` (and locdev-scaffold's plan).
    const starterKit = opt(p, s, "starter_kit") === "true";
    const kit = starterKit ? opt(p, s, "kit") : opt(p, s, "stack");
    const workos = starterKit && opt(p, s, "auth") === "workos";
    const classes = starterKit && kit === "livewire" && !workos && opt(p, s, "single_file") === "false";
    const teams = starterKit && !workos && !classes && opt(p, s, "teams") === "true";
    const drivers: Record<string, string> = { postgres: "pgsql", mysql: "mysql", mariadb: "mariadb" };
    const driver = drivers[p.database?.service ?? ""] ?? opt(p, s, "database");
    if (p.method === "installer") {
      const args = ["new", p.name, "--no-interaction"];
      if (kit !== "blade") {
        args.push(`--${kit}`);
        if (!starterKit) args.push("--no-authentication");
      }
      if (workos) args.push("--workos");
      if (classes) args.push("--livewire-class-components");
      if (teams) args.push("--teams");
      const node = opt(p, s, "node");
      args.push(`--${opt(p, s, "testing")}`, `--database=${driver}`, node === "none" ? "--no-node" : `--${node}`);
      args.push(opt(p, s, "boost") === "true" ? "--boost" : "--no-boost");
      if (opt(p, s, "git") === "true") args.push("--git");
      steps.push(step("Create the project", "laravel", args));
    } else {
      const branch = workos ? ":dev-workos" : teams ? ":dev-teams" : classes ? ":dev-components" : "";
      const pkg =
        kit === "blade"
          ? "laravel/laravel"
          : !starterKit
            ? `laravel/blank-${kit}-starter-kit`
            : `laravel/${kit}-starter-kit${branch}`;
      const args = ["create-project", pkg, p.name, "--no-interaction"];
      if (kit !== "blade") args.push("--stability=dev");
      steps.push(step("Create the project", "composer", args));
      if (kit !== "blade" && opt(p, s, "node") !== "none") {
        steps.push(step("Install the front-end dependencies", opt(p, s, "node"), ["install"], dir));
        steps.push(step("Build the front-end", opt(p, s, "node"), ["run", "build"], dir));
      }
    }
  } else if (s.id === "vite") {
    const fw = opt(p, s, "framework");
    const t = opt(p, s, "language") === "ts" ? `${fw}-ts` : fw;
    const head =
      pm === "npm"
        ? ["create", "--yes", "vite@latest", p.name, "--"]
        : pm === "pnpm"
          ? ["dlx", "create-vite@latest", p.name]
          : ["create-vite@latest", p.name];
    steps.push(step("Create the project", pm === "bun" ? "bunx" : pm, [...head, "--template", t, "--no-interactive"]));
    steps.push(step("Install the front-end dependencies", pm, ["install"], dir));
  } else if (s.id === "nextjs") {
    const run = pm === "npm" ? ["npx", "--yes"] : pm === "pnpm" ? ["pnpm", "dlx"] : ["bunx"];
    const flags = [
      opt(p, s, "language") === "js" ? "--js" : "--ts",
      opt(p, s, "tailwind") === "true" ? "--tailwind" : "--no-tailwind",
      ({ biome: "--biome", none: "--no-linter" } as Record<string, string>)[opt(p, s, "linter")] ?? "--eslint",
      opt(p, s, "app") === "true" ? "--app" : "--no-app",
      opt(p, s, "src") === "true" ? "--src-dir" : "--no-src-dir",
      `--use-${pm}`,
    ];
    if (opt(p, s, "git") !== "true") flags.push("--disable-git");
    steps.push(
      step("Create the project", run[0], [...run.slice(1), "create-next-app@latest", p.name, "--yes", ...flags]),
    );
  } else if (["php", "flask", "fastapi", "rust"].includes(s.id)) {
    // LocDev's own starter files, written before the steps (see `templates/`).
    files.push(...(STARTER_FILES[s.id === "rust" ? opt(p, s, "framework") : s.id] ?? []));
    if (s.id === "flask" || s.id === "fastapi") {
      steps.push(step("Create the virtual environment", "python3", ["-m", "venv", ".venv"], dir));
      steps.push(
        step(`Install ${s.name}`, `${dir}/.venv/bin/python`, ["-m", "pip", "install", "-r", "requirements.txt"], dir),
      );
    } else if (s.id === "rust") {
      const git = opt(p, s, "git") === "true" ? "git" : "none";
      steps.push(step("Create the Cargo package", "cargo", ["init", "--name", p.name, "--vcs", git], dir));
      if (opt(p, s, "framework") === "axum") {
        steps.push(step("Add Axum", "cargo", ["add", "axum"], dir));
        steps.push(step("Add Tokio", "cargo", ["add", "tokio", "--features", "full"], dir));
      } else steps.push(step("Add Actix Web", "cargo", ["add", "actix-web"], dir));
    }
    if (s.id !== "rust" && opt(p, s, "git") === "true")
      steps.push(step("Initialize a Git repository", "git", ["init"], dir));
  } else {
    steps.push(step("Create the project", s.methods[0].tool, ["new", p.name]));
  }
  if (p.packages.length > 0)
    steps.push(
      s.packages === "packagist"
        ? step("Add the chosen packages", "composer", ["require", ...p.packages], dir)
        : step("Add the chosen packages", pm, [pm === "npm" ? "install" : "add", ...p.packages], dir),
    );
  const actions: string[] = [];
  const finish: Step[] = [];
  if (p.database) {
    const db = p.name.replaceAll("-", "_");
    actions.push(
      `Create the ${p.database.service} instance '${p.database.instance}' if it doesn't exist, and start it`,
    );
    actions.push(`Create the database '${db}' in it`);
    actions.push(
      "Point DB_CONNECTION, DB_HOST, DB_PORT, DB_DATABASE, DB_USERNAME and DB_PASSWORD in the new project's .env at it",
    );
    finish.push(step("Run the migrations", "php", ["artisan", "migrate", "--no-interaction"], dir));
  }
  if (p.register) actions.push(`Register the folder as the site ${p.name}`);
  const toolOf: Record<string, string> = { npx: "npm", bunx: "bun" };
  const missing = [
    ...new Set(
      [...steps, ...finish]
        .map((st) => tools.find((t) => t.id === (toolOf[st.program] ?? st.program)))
        .filter((t): t is Tool => !!t && !t.installed)
        .map((t) => t.name),
    ),
  ];
  const notes = [
    ...(p.node ? [`Created with Node.js ${p.node}, and the site is pinned to it.`] : []),
    ...(p.php ? [`Created with PHP ${p.php}, and the site is pinned to it.`] : []),
  ];
  return { dir, make_dir: files.length > 0, files, steps, actions, finish, notes, missing };
}

const STARTER_FILES: Record<string, PlannedFile[]> = {
  php: [
    {
      path: "public/index.php",
      contents:
        "<?php\n\n// Served by PHP's built-in server from public/.\n\ndeclare(strict_types=1);\n\n?>\n<h1>It works</h1>\n<p>PHP <?= htmlspecialchars(PHP_VERSION) ?>, served by LocDev.</p>\n",
    },
  ],
  flask: [
    {
      path: "app.py",
      contents:
        'from flask import Flask\n\napp = Flask(__name__)\n\n\n@app.get("/")\ndef index():\n    return "<h1>It works</h1><p>Flask, served by LocDev.</p>"\n',
    },
    { path: "requirements.txt", contents: "flask\n" },
    { path: ".gitignore", contents: ".venv/\n__pycache__/\n*.pyc\n.env\n" },
  ],
  fastapi: [
    {
      path: "main.py",
      contents:
        'from fastapi import FastAPI\n\napp = FastAPI()\n\n\n@app.get("/")\ndef index():\n    return {"message": "It works: FastAPI, served by LocDev."}\n',
    },
    { path: "requirements.txt", contents: "fastapi[standard]\n" },
    { path: ".gitignore", contents: ".venv/\n__pycache__/\n*.pyc\n.env\n" },
  ],
  axum: [
    {
      path: "src/main.rs",
      contents:
        'use axum::{Router, routing::get};\n\n#[tokio::main]\nasync fn main() {\n    // LocDev gives the site its port in PORT.\n    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());\n    let addr = format!("127.0.0.1:{port}");\n    let app = Router::new().route("/", get(|| async { "It works" }));\n    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();\n    axum::serve(listener, app).await.unwrap();\n}\n',
    },
  ],
  actix: [
    {
      path: "src/main.rs",
      contents:
        'use actix_web::{App, HttpServer, get};\n\n#[get("/")]\nasync fn index() -> &\'static str {\n    "It works"\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);\n    HttpServer::new(|| App::new().service(index)).bind(("127.0.0.1", port))?.run().await\n}\n',
    },
  ],
};

const terminals = new Map<
  number,
  { info: TerminalInfo; out: string; script: string[]; started: number; killed?: boolean }
>();
let nextTerminal = 0;

/** A scripted terminal for other mocks (Tinker, package installs). */
export function mockTerminal(title: string, lines: string[]): TerminalInfo {
  const id = ++nextTerminal;
  const info: TerminalInfo = { id, title, running: true, step: 1, steps: 1 };
  terminals.set(id, { info, out: "", script: lines, started: Date.now() });
  return info;
}

function script(p: ProjectPlan, site: string | null): string[] {
  const lines: string[] = [];
  for (const st of [...p.steps, ...p.finish]) {
    lines.push(`\r\n\x1b[1;38;5;208m▸ ${st.label}\x1b[0m\r\n\x1b[2m$ ${[st.program, ...st.args].join(" ")}\x1b[0m\r\n`);
    if (st.program === "laravel") {
      lines.push(
        "\r\n   \x1b[38;5;208m_                               _\x1b[0m\r\n  \x1b[38;5;208m| |                             | |\x1b[0m\r\n  \x1b[38;5;208m| |     __ _ _ __ __ ___   _____| |\x1b[0m\r\n",
      );
      lines.push('\r\n\x1b[32m  Creating a "laravel/react-starter-kit" project at "./' + site + '"\x1b[0m\r\n');
      lines.push("  Installing laravel/framework (v13.2.0)\r\n  - Downloading laravel/framework (v13.2.0)\r\n");
      lines.push(
        "  Generating optimized autoload files\r\n  \x1b[32m80 packages you are using are looking for funding.\x1b[0m\r\n",
      );
      lines.push(
        "\r\n\x1b[42;30m INFO \x1b[0m Application ready in [" +
          site +
          "]. You can start your local development using:\r\n",
      );
    } else if (st.program === "php") {
      lines.push(
        "\r\n   \x1b[44;37m INFO \x1b[0m Running migrations.\r\n\r\n  0001_01_01_000000_create_users_table ........ \x1b[32m22.73ms DONE\x1b[0m\r\n  0001_01_01_000001_create_cache_table ........ \x1b[32m11.02ms DONE\x1b[0m\r\n",
      );
    } else if (st.args[0] === "run") {
      lines.push(
        "\r\n> build\r\n> vite build\r\n\r\n\x1b[32m✓\x1b[0m 1712 modules transformed.\r\n\x1b[32m✓ built in 3.41s\x1b[0m\r\n",
      );
    } else if (st.program === "composer") {
      lines.push(
        "./composer.json has been updated\r\n  - Installing laravel/horizon (v5.40.0): Extracting archive\r\n",
      );
    } else {
      lines.push("added 312 packages, and audited 313 packages in 9s\r\n\x1b[32mfound 0 vulnerabilities\x1b[0m\r\n");
    }
    if (st.label === "Create the project" && p.actions.length > 1) {
      for (const a of p.actions.slice(0, -1)) lines.push(`\r\n\x1b[2m› ${a}…\x1b[0m\r\n`);
    }
  }
  if (site) lines.push(`\r\n\x1b[2m› Registering ${site} as a site…\x1b[0m\r\n`);
  lines.push("\r\n\x1b[2m› Done.\x1b[0m\r\n");
  return lines;
}

const packages: Record<"npm" | "packagist", PackageHit[]> = {
  packagist: [
    {
      name: "laravel/horizon",
      description: "Dashboard and code-driven configuration for Laravel queues.",
      downloads: 98_000_000,
      url: "https://packagist.org/packages/laravel/horizon",
    },
    {
      name: "spatie/laravel-permission",
      description: "Permission handling for Laravel 12 and up",
      downloads: 82_000_000,
      url: "https://packagist.org/packages/spatie/laravel-permission",
    },
    {
      name: "laravel/telescope",
      description: "An elegant debug assistant for the Laravel framework.",
      downloads: 61_000_000,
      url: "https://packagist.org/packages/laravel/telescope",
    },
    {
      name: "barryvdh/laravel-debugbar",
      description: "PHP Debugbar integration for Laravel",
      downloads: 140_000_000,
      url: "https://packagist.org/packages/barryvdh/laravel-debugbar",
    },
  ],
  npm: [
    {
      name: "zod",
      description: "TypeScript-first schema declaration and validation library",
      version: "4.1.12",
      downloads: 140_000_000,
      url: "https://www.npmjs.com/package/zod",
    },
    {
      name: "@tanstack/react-query",
      description: "Hooks for managing, caching and syncing asynchronous and remote data in React",
      version: "5.90.5",
      downloads: 38_000_000,
      url: "https://www.npmjs.com/package/@tanstack/react-query",
    },
    {
      name: "date-fns",
      description: "Modern JavaScript date utility library",
      version: "4.1.0",
      downloads: 120_000_000,
      url: "https://www.npmjs.com/package/date-fns",
    },
  ],
};

export function handleScaffold(req: Request): Response | null {
  switch (req.type) {
    case "list_starters":
      return { type: "starters", starters: catalog, sites_dir: "/home/you/LocDev/Sites" };
    case "list_tools":
      return { type: "tools", tools };
    case "plan_project":
      return { type: "project_plan", ...plan(req.project) };
    case "create_project": {
      const p = plan(req.project);
      if (p.missing.length) throw new DaemonError("app", `install ${p.missing.join(", ")} first`);
      const id = ++nextTerminal;
      const info: TerminalInfo = {
        id,
        title: `New ${req.project.starter} project: ${req.project.name}`,
        running: true,
        step: 1,
        steps: p.steps.length + p.finish.length,
      };
      terminals.set(id, {
        info,
        out: "",
        script: script(p, req.project.register ? req.project.name : null),
        started: Date.now(),
      });
      return { type: "terminal", ...info };
    }
    case "install_tool": {
      const tool = tools.find((t) => t.id === req.id);
      if (!tool || tool.install?.kind !== "command") throw new DaemonError("app", "LocDev can't install this one");
      const id = ++nextTerminal;
      const info: TerminalInfo = { id, title: `Install ${tool.name}`, running: true, step: 1, steps: 1 };
      const st = tool.install.steps[0];
      terminals.set(id, {
        info,
        out: "",
        script: [
          `\r\n\x1b[1;38;5;208m▸ ${st.label}\x1b[0m\r\n\x1b[2m$ ${[st.program, ...st.args].join(" ")}\x1b[0m\r\n`,
          "Changed current directory to /home/you/.config/composer\r\n./composer.json has been updated\r\n",
          "Package operations: 9 installs, 0 updates, 0 removals\r\n  - Installing laravel/installer (v5.31.0)\r\n",
          "\r\n\x1b[2m› Done.\x1b[0m\r\n",
        ],
        started: Date.now(),
      });
      tool.installed = true;
      tool.version = "Laravel Installer 5.31.0";
      return { type: "terminal", ...info };
    }
    case "search_packages":
      return {
        type: "packages",
        packages: packages[req.registry].filter((p) =>
          `${p.name} ${p.description}`.toLowerCase().includes(req.query.toLowerCase()),
        ),
      };
    case "list_terminals":
      return { type: "terminals", terminals: [...terminals.values()].map((t) => t.info) };
    case "terminal_read": {
      const t = terminals.get(req.id);
      if (!t) throw new DaemonError("app", `no terminal ${req.id}`);
      // One scripted chunk every 350 ms.
      const due = Math.min(t.script.length, Math.floor((Date.now() - t.started) / 350));
      t.out = t.script.slice(0, due).join("");
      t.info.step = Math.min(t.info.steps, 1 + Math.floor((due / t.script.length) * t.info.steps));
      if (due === t.script.length && t.info.running) {
        t.info.running = false;
        t.info.code = t.killed ? null : 0;
        const created = t.info.title.split(": ")[1];
        if (!t.killed && created && !t.info.title.startsWith("Install")) t.info.site = created;
      }
      return {
        type: "terminal_output",
        info: { ...t.info },
        data: t.out.slice(req.from),
        next: t.out.length,
        truncated: false,
      };
    }
    case "terminal_write": {
      const t = terminals.get(req.id);
      if (t) t.script.splice(Math.floor((Date.now() - t.started) / 350), 0, req.data.replace(/\r/g, "\r\n"));
      return { type: "ok" };
    }
    case "terminal_resize":
    case "terminal_close":
      return { type: "ok" };
    case "terminal_kill": {
      const t = terminals.get(req.id);
      if (t) {
        t.killed = true;
        t.script = t.script.slice(0, Math.floor((Date.now() - t.started) / 350));
        t.script.push("\r\n\x1b[2m› Stopped.\x1b[0m\r\n");
      }
      return { type: "ok" };
    }
    default:
      return null;
  }
}
