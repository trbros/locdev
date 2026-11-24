//! What a project needs from a host, read from its stack, files and
//! dependencies. Every conclusion carries the evidence it came from;
//! variables are read by name (values only for a few switches like
//! `QUEUE_CONNECTION`), and no value ever leaves this module.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use locdev_core::db::DbEngine;
use locdev_core::deploy::{DeployProfile, EnvNeed, Need, Rendering};
use locdev_core::model::{ComponentCategory, ProcessKind, RuntimeKind, Stack};

pub struct Inputs<'a> {
    pub root: &'a Path,
    pub name: &'a str,
    pub stack: &'a Stack,
    /// Databases the project points at (from its .env), and whether each is remote.
    pub databases: Vec<(DbEngine, bool)>,
    /// Kinds of the auxiliary processes the user turned on in LocDev.
    pub processes: Vec<ProcessKind>,
}

/// Facts read once from the project's files.
pub struct Files {
    pub deps: BTreeSet<String>,
    pub scripts: BTreeMap<String, String>,
    pub engines_node: Option<String>,
    pub php: Option<String>,
    /// `.env` (values kept here only to read switches; never returned).
    pub env: BTreeMap<String, String>,
    /// Names in `.env.example` / `.env.dist`.
    pub example: Option<Vec<String>>,
}

fn read_json(root: &Path, file: &str) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(root.join(file)).ok()?;
    serde_json::from_str(&text).ok()
}

fn dotenv(root: &Path, file: &str) -> Option<Vec<(String, String)>> {
    let bytes = std::fs::read(root.join(file)).ok()?;
    let doc = locdev_env::dotenv::parse(&bytes).ok()?;
    Some(doc.entries.into_iter().map(|e| (e.key, e.value)).collect())
}

impl Files {
    pub fn read(root: &Path) -> Self {
        let mut deps: BTreeSet<String> = locdev_deps::local::read(root)
            .packages
            .into_iter()
            .map(|p| p.name)
            .collect();
        let package = read_json(root, "package.json");
        let scripts = package
            .as_ref()
            .and_then(|p| p["scripts"].as_object())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let engines_node = package
            .as_ref()
            .and_then(|p| p["engines"]["node"].as_str())
            .map(str::to_string);
        let composer = read_json(root, "composer.json");
        let php = composer
            .as_ref()
            .and_then(|c| c["require"]["php"].as_str())
            .map(str::to_string);
        // Python and Ruby manifests aren't read by locdev-deps; names are enough here.
        for (file, prefix) in [("requirements.txt", ""), ("Gemfile", "gem ")] {
            if let Ok(text) = std::fs::read_to_string(root.join(file)) {
                for line in text.lines() {
                    let line = line
                        .trim()
                        .strip_prefix(prefix)
                        .unwrap_or("")
                        .trim_matches(['"', '\'', ' ']);
                    let name: String = line
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        deps.insert(name.to_lowercase());
                    }
                }
            }
        }
        let env = dotenv(root, ".env")
            .unwrap_or_default()
            .into_iter()
            .collect();
        let example = [".env.example", ".env.dist", ".env.sample"]
            .iter()
            .find_map(|f| dotenv(root, f))
            .map(|entries| entries.into_iter().map(|(k, _)| k).collect());
        Self {
            deps,
            scripts,
            engines_node,
            php,
            env,
            example,
        }
    }

    pub fn has(&self, dep: &str) -> bool {
        self.deps.contains(dep)
    }
}

fn ids(stack: &Stack) -> Vec<&str> {
    stack.components.iter().map(|c| c.id.as_str()).collect()
}

pub fn runtime(stack: &Stack, root: &Path) -> &'static str {
    let ids = ids(stack);
    let any = |list: &[&str]| list.iter().any(|id| ids.contains(id));
    if any(&["laravel", "wordpress", "php"]) {
        "php"
    } else if any(&["django", "flask", "fastapi", "python"]) {
        "python"
    } else if any(&["rails", "ruby"]) {
        "ruby"
    } else if any(&["go"]) {
        "go"
    } else if any(&["spring", "java"]) {
        "java"
    } else if any(&["dotnet"]) {
        "dotnet"
    } else if any(&["rust"]) {
        "rust"
    } else if any(&["deno"]) {
        "deno"
    } else if root.join("package.json").is_file() {
        "node"
    } else {
        "static"
    }
}

const SERVERS: &[&str] = &[
    "laravel",
    "wordpress",
    "php",
    "django",
    "flask",
    "fastapi",
    "rails",
    "express",
    "fastify",
    "hono",
    "nestjs",
    "elysia",
    "spring",
    "dotnet",
    "go",
    "rust",
    "nodejs",
    "bun",
    "deno",
];
const SSR: &[&str] = &["remix", "solid-start", "qwik", "redwood"];

fn config_value(root: &Path, base: &str, key: &str) -> Option<(String, serde_json::Value)> {
    for ext in ["ts", "mjs", "js", "cjs", "mts"] {
        let file = format!("{base}.{ext}");
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
            continue;
        };
        let read = locdev_config::js::read(&text, &file, &[key]).ok()?;
        return match read.values.into_iter().next()?.1 {
            locdev_core::config::OptionState::Set { value } => Some((file, value)),
            _ => None,
        };
    }
    None
}

pub fn rendering(stack: &Stack, root: &Path, files: &Files) -> (Rendering, String) {
    let ids = ids(stack);
    let has = |id: &str| ids.contains(&id);
    if has("nextjs") {
        return match config_value(root, "next.config", "output") {
            Some((file, v)) if v == "export" => {
                (Rendering::Static, format!("output: 'export' in {file}"))
            }
            _ => (
                Rendering::Ssr,
                "Next.js without output: 'export' renders on a server".into(),
            ),
        };
    }
    if has("astro") {
        let adapter = files.deps.iter().find(|d| {
            d.starts_with("@astrojs/")
                && ["node", "vercel", "netlify", "cloudflare"]
                    .iter()
                    .any(|a| d.ends_with(a))
        });
        return match (config_value(root, "astro.config", "output"), adapter) {
            (Some((file, v)), _) if v == "server" => {
                (Rendering::Ssr, format!("output: 'server' in {file}"))
            }
            (_, Some(a)) => (Rendering::Ssr, format!("{a} adapter installed")),
            _ => (
                Rendering::Static,
                "Astro without an adapter builds static pages".into(),
            ),
        };
    }
    if has("nuxt") {
        return match config_value(root, "nuxt.config", "ssr") {
            Some((file, v)) if v == false => (Rendering::Spa, format!("ssr: false in {file}")),
            _ => (
                Rendering::Ssr,
                "Nuxt renders on the server by default".into(),
            ),
        };
    }
    if has("sveltekit") {
        return if files.has("@sveltejs/adapter-static") {
            (
                Rendering::Static,
                "@sveltejs/adapter-static installed".into(),
            )
        } else {
            (
                Rendering::Ssr,
                "SvelteKit without adapter-static renders on a server".into(),
            )
        };
    }
    if has("angular") {
        return if files.has("@angular/ssr") {
            (Rendering::Ssr, "@angular/ssr installed".into())
        } else {
            (
                Rendering::Spa,
                "Angular without @angular/ssr is a client-side app".into(),
            )
        };
    }
    if let Some(id) = ids.iter().find(|id| SSR.contains(id)) {
        return (Rendering::Ssr, format!("{id} renders on a server"));
    }
    if let Some(id) = ids.iter().find(|id| SERVERS.contains(id)) {
        return (
            Rendering::Server,
            format!("{id} runs as an application server"),
        );
    }
    if has("hugo") || has("gatsby") {
        return (Rendering::Static, "a static site generator".into());
    }
    if has("vite") || has("ember") {
        return (Rendering::Spa, "a Vite (client-side) app".into());
    }
    if root.join("package.json").is_file() && files.scripts.contains_key("start") {
        return (Rendering::Server, "package.json has a start script".into());
    }
    (Rendering::Static, "plain files".into())
}

fn need(id: &str, label: &str, evidence: String) -> Need {
    Need {
        id: id.into(),
        label: label.into(),
        evidence,
    }
}

const WS_DEPS: &[&str] = &[
    "socket.io",
    "ws",
    "@nestjs/websockets",
    "@fastify/websocket",
    "laravel/reverb",
    "beyondcode/laravel-websockets",
    "channels",
    "uwebsockets.js",
];
const WORKER_DEPS: &[&str] = &[
    "bullmq",
    "bull",
    "bee-queue",
    "pg-boss",
    "graphile-worker",
    "laravel/horizon",
    "celery",
    "dramatiq",
    "rq",
    "sidekiq",
    "solid_queue",
    "good_job",
];
const CRON_DEPS: &[&str] = &[
    "node-cron",
    "cron",
    "croner",
    "agenda",
    "@nestjs/schedule",
    "django-celery-beat",
    "whenever",
];
const MONGO_DEPS: &[&str] = &[
    "mongoose",
    "mongodb",
    "mongodb/laravel-mongodb",
    "pymongo",
    "motor",
    "mongoid",
];
const REDIS_DEPS: &[&str] = &["ioredis", "redis", "predis/predis", "@upstash/redis"];

pub fn needs(inputs: &Inputs, files: &Files) -> Vec<Need> {
    let stack = inputs.stack;
    let laravel = ids(stack).contains(&"laravel");
    let mut out = Vec::new();
    let found = |list: &[&str]| list.iter().find(|d| files.has(d)).map(|d| d.to_string());

    if let Some(dep) = found(WS_DEPS) {
        out.push(need(
            "websockets",
            "WebSocket server",
            format!("{dep} is installed"),
        ));
    } else if inputs.processes.contains(&ProcessKind::Websocket) {
        out.push(need(
            "websockets",
            "WebSocket server",
            "a WebSocket process is on in LocDev".into(),
        ));
    }

    let queue = files
        .env
        .get("QUEUE_CONNECTION")
        .map(|v| v.trim().to_lowercase());
    if let Some(dep) = found(WORKER_DEPS) {
        out.push(need(
            "workers",
            "Queue workers",
            format!("{dep} is installed"),
        ));
    } else if laravel && queue.as_deref().is_some_and(|q| q != "sync" && q != "null") {
        out.push(need(
            "workers",
            "Queue workers",
            format!("QUEUE_CONNECTION={} in .env", queue.unwrap_or_default()),
        ));
    } else if inputs.processes.contains(&ProcessKind::Worker) {
        out.push(need(
            "workers",
            "Queue workers",
            "a worker process is on in LocDev".into(),
        ));
    }

    let schedules = laravel
        && ["routes/console.php", "app/Console/Kernel.php"]
            .iter()
            .any(|f| {
                std::fs::read_to_string(inputs.root.join(f))
                    .is_ok_and(|t| t.contains("Schedule::") || t.contains("$schedule->"))
            });
    if schedules {
        out.push(need(
            "scheduler",
            "Scheduled tasks",
            "the Laravel scheduler has tasks (routes/console.php)".into(),
        ));
    } else if let Some(dep) = found(CRON_DEPS) {
        out.push(need(
            "scheduler",
            "Scheduled tasks",
            format!("{dep} is installed"),
        ));
    } else if inputs.processes.contains(&ProcessKind::Scheduler) {
        out.push(need(
            "scheduler",
            "Scheduled tasks",
            "a scheduler process is on in LocDev".into(),
        ));
    }

    let mut seen = BTreeSet::new();
    for (engine, _) in &inputs.databases {
        let (id, label) = match engine {
            DbEngine::Postgres => ("postgres", "PostgreSQL"),
            DbEngine::Mysql => ("mysql", "MySQL/MariaDB"),
            DbEngine::Sqlite => ("sqlite", "SQLite file"),
            DbEngine::Redis => ("redis", "Redis-compatible cache/queue"),
        };
        if seen.insert(id) {
            out.push(need(id, label, "a connection in the project's .env".into()));
        }
    }
    if !seen.contains("redis")
        && let Some(dep) = found(REDIS_DEPS)
    {
        out.push(need(
            "redis",
            "Redis-compatible cache/queue",
            format!("{dep} is installed"),
        ));
    }
    if let Some(dep) = found(MONGO_DEPS) {
        out.push(need("mongo", "MongoDB", format!("{dep} is installed")));
    }

    if laravel {
        let disk = files
            .env
            .get("FILESYSTEM_DISK")
            .map(|v| v.trim().to_lowercase())
            .unwrap_or_else(|| "local".into());
        let stored = std::fs::read_dir(inputs.root.join("storage/app/public"))
            .map(|d| d.flatten().any(|e| e.file_name() != ".gitignore"))
            .unwrap_or(false);
        if (disk == "local" || disk == "public") && stored {
            out.push(need(
                "storage",
                "Files on local disk",
                format!("FILESYSTEM_DISK={disk} with files in storage/app/public"),
            ));
        }
    }
    out
}

fn install(manager: Option<RuntimeKind>) -> &'static str {
    match manager {
        Some(RuntimeKind::Pnpm) => "pnpm install --frozen-lockfile",
        Some(RuntimeKind::Yarn) => "yarn install --frozen-lockfile",
        Some(RuntimeKind::Bun) => "bun install --frozen-lockfile",
        _ => "npm ci",
    }
}

fn run(manager: Option<RuntimeKind>, script: &str) -> String {
    match manager {
        Some(RuntimeKind::Pnpm) => format!("pnpm run {script}"),
        Some(RuntimeKind::Yarn) => format!("yarn {script}"),
        Some(RuntimeKind::Bun) => format!("bun run {script}"),
        _ => format!("npm run {script}"),
    }
}

pub fn commands(
    stack: &Stack,
    root: &Path,
    files: &Files,
    runtime: &str,
    rendering: Rendering,
) -> (Option<String>, Option<String>, Option<String>) {
    let ids = ids(stack);
    let pm = stack.package_manager;
    let node_build = files
        .scripts
        .contains_key("build")
        .then(|| format!("{} && {}", install(pm), run(pm, "build")));
    match runtime {
        "node" => {
            let start = files
                .scripts
                .contains_key("start")
                .then(|| run(pm, "start"));
            let out = match rendering {
                Rendering::Static | Rendering::Spa => Some(
                    if ids.contains(&"nextjs") {
                        "out"
                    } else if ids.contains(&"gatsby") {
                        "public"
                    } else if ids.contains(&"sveltekit") {
                        "build"
                    } else if ids.contains(&"nuxt") {
                        ".output/public"
                    } else {
                        "dist"
                    }
                    .to_string(),
                ),
                _ => None,
            };
            let out = out.map(
                |o| match config_value(root, "vite.config", "build.outDir") {
                    Some((_, serde_json::Value::String(dir))) if o == "dist" => dir,
                    _ => o,
                },
            );
            (
                node_build,
                start.filter(|_| matches!(rendering, Rendering::Ssr | Rendering::Server)),
                out,
            )
        }
        "php" => {
            let composer = "composer install --no-dev --optimize-autoloader".to_string();
            let build = match node_build {
                Some(node) => format!("{composer} && {node}"),
                None => composer,
            };
            (Some(build), None, None)
        }
        "static" if ids.contains(&"hugo") => {
            (Some("hugo --minify".into()), None, Some("public".into()))
        }
        _ => (None, None, None),
    }
}

const DB_VARS: &[&str] = &[
    "DATABASE_URL",
    "DB_HOST",
    "DB_PORT",
    "DB_DATABASE",
    "DB_USERNAME",
    "DB_PASSWORD",
    "DB_URL",
    "POSTGRES_URL",
    "MYSQL_URL",
    "MONGODB_URI",
    "MONGO_URL",
];
const CACHE_VARS: &[&str] = &[
    "REDIS_URL",
    "REDIS_HOST",
    "REDIS_PORT",
    "REDIS_PASSWORD",
    "REDIS_USERNAME",
    "KV_URL",
];
const PUBLIC_PREFIXES: &[&str] = &[
    "NEXT_PUBLIC_",
    "VITE_",
    "PUBLIC_",
    "NUXT_PUBLIC_",
    "EXPO_PUBLIC_",
    "REACT_APP_",
];

pub fn secret_name(name: &str) -> bool {
    if PUBLIC_PREFIXES.iter().any(|p| name.starts_with(p)) {
        return false;
    }
    [
        "KEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASS",
        "PRIVATE",
        "CREDENTIAL",
        "DSN",
        "AUTH",
    ]
    .iter()
    .any(|w| name.split('_').any(|part| part == *w) || name.ends_with(w))
}

pub fn env(files: &Files) -> Vec<EnvNeed> {
    let mut names: Vec<String> = files.example.clone().unwrap_or_default();
    let in_example: BTreeSet<String> = names.iter().cloned().collect();
    for key in files.env.keys() {
        if !in_example.contains(key) {
            names.push(key.clone());
        }
    }
    let mut seen = BTreeSet::new();
    names
        .into_iter()
        .filter(|n| seen.insert(n.clone()))
        .map(|name| {
            let note = if DB_VARS.contains(&name.as_str()) {
                "from the production database".to_string()
            } else if CACHE_VARS.contains(&name.as_str()) {
                "from the production cache".to_string()
            } else if name == "APP_KEY" {
                "a new key for production (`php artisan key:generate --show`)".to_string()
            } else if name == "APP_DEBUG" {
                "false in production".to_string()
            } else if name == "APP_ENV" || name == "NODE_ENV" {
                "production".to_string()
            } else if name.ends_with("_URL")
                && [
                    "APP_",
                    "SITE_",
                    "BASE_",
                    "PUBLIC_",
                    "NEXT_PUBLIC_",
                    "NEXTAUTH_",
                ]
                .iter()
                .any(|p| name.starts_with(p))
            {
                "the production URL".to_string()
            } else if !in_example.is_empty() && !in_example.contains(&name) {
                "only in .env: add it to .env.example if production needs it".to_string()
            } else {
                "set it on the platform".to_string()
            };
            EnvNeed {
                secret: secret_name(&name),
                name,
                note,
            }
        })
        .collect()
}

pub fn profile(inputs: &Inputs, files: &Files) -> DeployProfile {
    let stack = inputs.stack;
    let runtime = runtime(stack, inputs.root);
    let (rendering, rendering_evidence) = rendering(stack, inputs.root, files);
    let primary = stack.components.iter().find(|c| c.primary).or_else(|| {
        stack
            .components
            .iter()
            .find(|c| c.category == ComponentCategory::Framework)
    });
    let (build_command, start_command, output_dir) =
        commands(stack, inputs.root, files, runtime, rendering);
    let runtime_version = match runtime {
        "node" => files.engines_node.clone().or_else(|| {
            [".nvmrc", ".node-version"].iter().find_map(|f| {
                std::fs::read_to_string(inputs.root.join(f))
                    .ok()
                    .map(|t| t.trim().to_string())
            })
        }),
        "php" => files.php.clone(),
        _ => None,
    };
    DeployProfile {
        framework: primary.map_or_else(|| stack.framework.clone(), |c| c.name.clone()),
        version: primary.and_then(|c| c.version.clone()),
        runtime: runtime.into(),
        runtime_version,
        rendering,
        rendering_evidence,
        needs: needs(inputs, files),
        build_command,
        start_command,
        output_dir,
        env: env(files),
    }
}
