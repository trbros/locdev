//! The command catalog: every command, argument and flag, read from the clap
//! definitions themselves, plus what clap can't know (category, the version
//! a command arrived in, whether it needs administrator rights, examples).
//! The app's CLI page and the docs render this file, so they can't drift
//! from the real CLI: a test fails when the committed copy is out of date.

use clap::{Arg, ArgAction, CommandFactory};
use serde::Serialize;

use crate::Cli;

/// No version field on purpose: the committed copies would go stale on every
/// version bump. The app shows the version of the CLI it actually bundles.
#[derive(Debug, Serialize, PartialEq)]
pub struct Catalog {
    pub categories: Vec<Category>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Category {
    pub id: &'static str,
    pub label: &'static str,
    pub commands: Vec<Entry>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Entry {
    /// The full invocation: `locdev runtime install`.
    pub usage: String,
    pub name: String,
    pub about: String,
    pub details: Option<String>,
    pub aliases: Vec<String>,
    pub args: Vec<ArgEntry>,
    /// The LocDev version that introduced it.
    pub since: &'static str,
    /// Asks for administrator rights (it shows the plan and confirms first).
    pub privileged: bool,
    pub examples: Vec<&'static str>,
    /// Platform differences worth knowing.
    pub notes: Vec<&'static str>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ArgEntry {
    /// `<name>` for positionals, `--flag`/`-f` for options.
    pub name: String,
    pub positional: bool,
    pub required: bool,
    pub takes_value: bool,
    pub help: String,
    pub default: Option<String>,
}

/// What clap doesn't describe, keyed by the command path after `locdev`.
struct Meta {
    path: &'static str,
    category: &'static str,
    since: &'static str,
    privileged: bool,
    examples: &'static [&'static str],
    notes: &'static [&'static str],
}

const fn meta(path: &'static str, category: &'static str, since: &'static str) -> Meta {
    Meta {
        path,
        category,
        since,
        privileged: false,
        examples: &[],
        notes: &[],
    }
}

const CATEGORIES: &[(&str, &str)] = &[
    ("daemon", "Daemon"),
    ("sites", "Sites"),
    ("servers", "Dev servers and processes"),
    ("https", "HTTPS and DNS"),
    ("runtimes", "Runtimes and package managers"),
    ("deps", "Dependencies, jobs and notifications"),
    ("env", "Environment variables and secrets"),
    ("stack", "Frameworks and stacks"),
    ("services", "Databases and caches"),
    ("db", "Database manager"),
    ("config", "Config files"),
    ("deploy", "Deploy assistant"),
    ("mail", "Mailbox"),
    ("email", "Email Studio"),
    ("git", "Git, GitHub, Actions and security"),
    ("system", "System integration"),
];

const META: &[Meta] = &[
    Meta {
        examples: &["locdev", "locdev tui --light"],
        notes: &[
            "Needs an interactive terminal; piped or with --json, `locdev` alone prints the help.",
            "Windows: use Windows Terminal (the classic console lacks true color).",
        ],
        ..meta("tui", "daemon", "0.3")
    },
    meta("version", "daemon", "0.1"),
    Meta {
        examples: &["locdev update", "locdev update --refresh --json"],
        notes: &[
            "Reads the published releases on GitHub without a token and installs nothing: the app updates itself (signed), the standalone CLI through its install script (checked against SHA256SUMS).",
            "The daemon also checks once a day and notifies about a newer release; turn it off in Settings → Updates or with `[automation] updates = false` in settings.toml.",
        ],
        ..meta("update", "daemon", "0.3")
    },
    meta("status", "daemon", "0.1"),
    Meta {
        examples: &["locdev doctor", "locdev doctor --json"],
        ..meta("doctor", "daemon", "0.1")
    },
    meta("start", "daemon", "0.1"),
    meta("stop", "daemon", "0.1"),
    meta("restart", "daemon", "0.1"),
    Meta {
        examples: &[
            "locdev add . --start",
            "locdev add ~/Code/shop --name shop --port 8200",
        ],
        ..meta("add", "sites", "0.1")
    },
    meta("list", "sites", "0.1"),
    Meta {
        examples: &[
            "locdev park",
            "locdev park ~/Development",
            "locdev park ~/Clients --depth 2 --exclude 'archive-*'",
            "locdev park ~/Experiments --suggest --keep-missing",
        ],
        notes: &[
            "A subfolder counts as a project only when its files say so (artisan, composer.json, package.json, pyproject.toml, Cargo.toml, index.html…); node_modules, vendor, hidden and build folders are skipped. A taken name gets the parent folder's name (shop-clients) or a number. New sites start stopped.",
            "Run again on a parked folder to change its rules; unset options are kept.",
        ],
        ..meta("park", "sites", "0.3")
    },
    meta("park list", "sites", "0.3"),
    Meta {
        examples: &["locdev tunnel providers"],
        notes: &[
            "LocDev runs the provider's own program; it never relays traffic itself. Only the site's own server is shared (never the proxy, a database or LocDev), with the site's domain as Host.",
        ],
        ..meta("tunnel", "sites", "0.3")
    },
    meta("tunnel providers", "sites", "0.3"),
    Meta {
        notes: &[
            "Events come from the sites' output (exceptions and stack traces of PHP, Node.js, Python and Rust, build errors, warning/error lines), Laravel's storage/logs, the proxy (5xx answers), process failures and failed jobs. Secrets are hidden before anything is stored: .env values, LocDev's secrets, auth headers, cookies, tokens, passwords in URLs and JSON.",
        ],
        ..meta("debug", "servers", "0.3")
    },
    Meta {
        examples: &[
            "locdev debug events -f",
            "locdev debug events --site shop --kind exception --kind http",
            "locdev debug events --level error --search SQLSTATE --json",
        ],
        ..meta("debug events", "servers", "0.3")
    },
    Meta {
        examples: &["locdev debug errors", "locdev debug errors --site shop"],
        ..meta("debug errors", "servers", "0.3")
    },
    Meta {
        examples: &["locdev debug show 42"],
        ..meta("debug show", "servers", "0.3")
    },
    Meta {
        examples: &["locdev debug export --site shop > shop-events.jsonl"],
        ..meta("debug export", "servers", "0.3")
    },
    Meta {
        examples: &["locdev debug clear --site shop", "locdev debug clear -y"],
        ..meta("debug clear", "servers", "0.3")
    },
    Meta {
        examples: &["locdev debug dumps shop on", "locdev debug dumps shop off"],
        notes: &[
            "PHP sites with Symfony VarDumper (Laravel, Symfony): LocDev sets VAR_DUMPER_FORMAT=server and VAR_DUMPER_SERVER in the site's process environment only; no file changes. dump() and dd() then go to LocDev instead of the page (dd() still ends the request). If LocDev's dump server can't be reached, VarDumper prints them as usual. A running site restarts to pick it up.",
        ],
        ..meta("debug dumps", "servers", "0.3")
    },
    meta("debug status", "servers", "0.3"),
    meta("editor", "sites", "0.3"),
    Meta {
        notes: &[
            "Finds VS Code, Cursor, Windsurf, VSCodium, Zed, Sublime Text and JetBrains IDEs (PhpStorm, WebStorm, IntelliJ IDEA, PyCharm, RustRover) by their command-line launchers on PATH, in the folders installers use (Homebrew, JetBrains Toolbox scripts) and in their usual install locations.",
        ],
        examples: &["locdev editor list", "locdev editor list --refresh --json"],
        ..meta("editor list", "sites", "0.3")
    },
    Meta {
        examples: &["locdev editor set zed", "locdev editor set none"],
        ..meta("editor set", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev editor open shop",
            "locdev editor open shop app/Http/Kernel.php:42",
            "locdev editor open shop src/main.ts:12:4 --editor cursor",
        ],
        notes: &[
            "Only the site's folder or a file inside it (no `..`, no links out). The editor starts on its own; LocDev doesn't wait for it.",
        ],
        ..meta("editor open", "sites", "0.3")
    },
    Meta {
        examples: &["locdev tunnel install cloudflare"],
        notes: &[
            "Downloads cloudflared from Cloudflare's GitHub release only when the checksum in the release notes and the one GitHub reports for the file agree, and checks the download against it. A cloudflared already on PATH (Homebrew, apt) is used as is.",
        ],
        ..meta("tunnel install", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev tunnel start shop",
            "locdev tunnel start shop --allow ana@example.com --allow '*@acme.com'",
        ],
        notes: &[
            "The site must be running. The address (https://….trycloudflare.com) changes every time and stops working when the tunnel stops; up to 200 requests at the same time, no Server-Sent Events. With --allow, visitors confirm their e-mail with a one-time PIN.",
        ],
        ..meta("tunnel start", "sites", "0.3")
    },
    meta("tunnel stop", "sites", "0.3"),
    meta("tunnel list", "sites", "0.3"),
    meta("tunnel logs", "sites", "0.3"),
    Meta {
        examples: &[
            "locdev park remove ~/Development",
            "locdev park remove ~/Development --keep-sites",
        ],
        notes: &["Sites are removed from LocDev only; their folders and files stay."],
        ..meta("park remove", "sites", "0.3")
    },
    Meta {
        examples: &["locdev park scan", "locdev park scan ~/Development"],
        notes: &[
            "Changes are also picked up as they happen (file events) and by a rescan every 5 minutes.",
        ],
        ..meta("park scan", "sites", "0.3")
    },
    Meta {
        examples: &["locdev park ignore ~/Development/old-shop"],
        notes: &[
            "Removing a parked site with `locdev remove` ignores it too, so it doesn't come back.",
        ],
        ..meta("park ignore", "sites", "0.3")
    },
    meta("park unignore", "sites", "0.3"),
    meta("park enable", "sites", "0.3"),
    meta("park disable", "sites", "0.3"),
    Meta {
        examples: &[
            "locdev edit blog --node 22",
            "locdev edit shop --php 8.3 --composer 2",
            "locdev edit blog --node \"\"",
        ],
        ..meta("edit", "sites", "0.1")
    },
    meta("remove", "sites", "0.1"),
    Meta {
        examples: &["locdev up", "locdev up blog"],
        ..meta("up", "sites", "0.1")
    },
    meta("down", "sites", "0.1"),
    meta("reload", "sites", "0.1"),
    Meta {
        examples: &["locdev logs -f", "locdev logs shop --dev -n 50"],
        ..meta("logs", "sites", "0.1")
    },
    Meta {
        examples: &["locdev info", "locdev info shop --json"],
        ..meta("info", "sites", "0.2")
    },
    Meta {
        examples: &["locdev open", "locdev open blog --https"],
        notes: &["Uses xdg-open on Linux, open on macOS and the URL handler on Windows."],
        ..meta("open", "sites", "0.2")
    },
    meta("domain", "sites", "0.1"),
    Meta {
        privileged: true,
        examples: &[
            "locdev secure",
            "locdev secure shop --yes",
            "locdev secure shop --off",
        ],
        notes: &[
            "Trusting the CA and routing *.test ask for administrator rights (polkit/sudo, the macOS dialog or UAC).",
            "Firefox and Chrome on Linux keep their own certificate store; it is updated as your user.",
        ],
        ..meta("secure", "https", "0.2")
    },
    meta("dns", "https", "0.1"),
    meta("dns status", "https", "0.1"),
    Meta {
        privileged: true,
        notes: &[
            "Linux: systemd-resolved drop-in. macOS: /etc/resolver/test. Windows: an NRPT rule (needs port 53).",
        ],
        ..meta("dns install", "https", "0.1")
    },
    Meta {
        privileged: true,
        ..meta("dns uninstall", "https", "0.1")
    },
    meta("tls", "https", "0.1"),
    meta("tls status", "https", "0.1"),
    Meta {
        privileged: true,
        notes: &[
            "Linux: update-ca-certificates or update-ca-trust. macOS: the System keychain. Windows: the Root store.",
        ],
        ..meta("tls trust", "https", "0.1")
    },
    Meta {
        privileged: true,
        ..meta("tls untrust", "https", "0.1")
    },
    meta("dev", "servers", "0.2"),
    meta("dev status", "servers", "0.2"),
    meta("dev start", "servers", "0.2"),
    meta("dev stop", "servers", "0.2"),
    meta("dev restart", "servers", "0.2"),
    Meta {
        examples: &[
            "locdev dev mode auto",
            "locdev dev mode manual shop --start",
            "locdev dev mode never",
        ],
        ..meta("dev mode", "servers", "0.2")
    },
    Meta {
        examples: &["locdev dev logs -f"],
        ..meta("dev logs", "servers", "0.2")
    },
    Meta {
        examples: &["locdev ps", "locdev ps --json"],
        ..meta("ps", "servers", "0.2")
    },
    meta("deps", "deps", "0.2"),
    meta("deps list", "deps", "0.2"),
    meta("deps outdated", "deps", "0.2"),
    Meta {
        examples: &["locdev deps check", "locdev deps check shop --no-wait"],
        ..meta("deps check", "deps", "0.2")
    },
    Meta {
        examples: &["locdev deps audit", "locdev deps audit --json"],
        notes: &[
            "Advisories come from osv.dev (GitHub, npm and PHP security advisories). Exits with 1 when a version in use is affected.",
        ],
        ..meta("deps audit", "deps", "0.2")
    },
    Meta {
        examples: &[
            "locdev deps update react vite",
            "locdev deps update laravel/framework --major",
            "locdev deps update vite --yes",
        ],
        notes: &[
            "Shows the exact commands and asks first. Install scripts are skipped unless --scripts.",
        ],
        ..meta("deps update", "deps", "0.2")
    },
    meta("jobs", "deps", "0.2"),
    meta("jobs list", "deps", "0.2"),
    meta("jobs status", "deps", "0.2"),
    meta("jobs cancel", "deps", "0.2"),
    meta("jobs pause", "deps", "0.2"),
    meta("jobs resume", "deps", "0.2"),
    meta("jobs retry", "deps", "0.2"),
    meta("notifications", "deps", "0.2"),
    meta("notifications list", "deps", "0.2"),
    meta("notifications read", "deps", "0.2"),
    meta("notifications dismiss", "deps", "0.2"),
    meta("notifications test", "deps", "0.2"),
    meta("git", "git", "0.2"),
    Meta {
        examples: &["locdev git status", "locdev git status shop --json"],
        ..meta("git status", "git", "0.2")
    },
    Meta {
        examples: &["locdev git log -n 50"],
        ..meta("git log", "git", "0.2")
    },
    meta("git branches", "git", "0.2"),
    meta("git stashes", "git", "0.2"),
    Meta {
        notes: &[
            "Lists what LocDev ran (and refused) in this repository, and the reflog, which is how a reset or a deleted branch is undone.",
        ],
        ..meta("git history", "git", "0.2")
    },
    meta("github", "git", "0.2"),
    meta("github status", "git", "0.2"),
    Meta {
        examples: &[
            "locdev github connect < token.txt",
            "locdev github connect --gh",
        ],
        notes: &[
            "The token goes to the system keyring (Secret Service, Keychain, Windows Credential Manager), never to a file. With no keyring available, connecting fails.",
        ],
        ..meta("github connect", "git", "0.2")
    },
    meta("github disconnect", "git", "0.2"),
    meta("actions", "git", "0.2"),
    Meta {
        examples: &["locdev actions list", "locdev actions list shop --json"],
        ..meta("actions list", "git", "0.2")
    },
    Meta {
        examples: &[
            "locdev actions new node-ci",
            "locdev actions new php-ci --save",
            "locdev actions new pages --file deploy.yml --save",
        ],
        notes: &[
            "Prints the workflow and its checks. --save creates the file after asking, and never replaces one: edit existing files in the app, which shows the diff.",
        ],
        ..meta("actions new", "git", "0.2")
    },
    Meta {
        examples: &[
            "locdev actions validate",
            "locdev actions validate --file ci.yml",
        ],
        notes: &[
            "Structure, script injection, pull_request_target, token permissions, unpinned and outdated actions; actionlint too when it is on PATH. Exits with 1 when there are errors.",
        ],
        ..meta("actions validate", "git", "0.2")
    },
    meta("actions versions", "git", "0.2"),
    Meta {
        examples: &[
            "locdev actions history ci.yml",
            "locdev actions history ci.yml --show 1763900000000-saved",
        ],
        notes: &[
            "LocDev keeps a version when it reads a workflow changed outside it and around every save (50 per file).",
        ],
        ..meta("actions history", "git", "0.4")
    },
    Meta {
        examples: &["locdev actions install-actionlint"],
        notes: &[
            "From rhysd/actionlint's latest GitHub release; installed only when the release's checksums file and GitHub's digest agree on the SHA-256.",
        ],
        ..meta("actions install-actionlint", "git", "0.4")
    },
    Meta {
        examples: &[
            "locdev metrics",
            "locdev metrics shop --json",
            "locdev metrics shop --reset",
        ],
        notes: &[
            "Counted by LocDev's proxy in memory since the daemon started: requests that skip the proxy (127.0.0.1:<port>) aren't seen. Latency is the time to the response headers.",
        ],
        ..meta("metrics", "servers", "0.4")
    },
    Meta {
        examples: &[
            "locdev tinker -e 'App\\Models\\User::count()'",
            "echo 'config(\"app.name\")' | locdev tinker",
            "locdev tinker --site shop",
        ],
        notes: &[
            "Runs `php artisan tinker --execute` with the site's PHP and environment (30 s, 256 KB of output). At a prompt each line runs on its own; for a full REPL use the app's Tinker console.",
        ],
        ..meta("tinker", "servers", "0.4")
    },
    meta("security", "git", "0.2"),
    meta("security status", "git", "0.2"),
    Meta {
        examples: &[
            "locdev security password",
            "locdev security password --remove",
        ],
        notes: &[
            "Only an Argon2id hash is stored. It protects operations started through LocDev (the app, the CLI); Git itself in a terminal is not affected.",
        ],
        ..meta("security password", "git", "0.2")
    },
    meta("process", "servers", "0.3"),
    Meta {
        examples: &["locdev process list", "locdev process list shop --json"],
        ..meta("process list", "servers", "0.3")
    },
    Meta {
        examples: &[
            "locdev process start queue",
            "locdev process start horizon shop",
        ],
        notes: &[
            "Refused while an alternative that consumes the same jobs is running (Horizon and queue:work, Solid Queue and Sidekiq).",
        ],
        ..meta("process start", "servers", "0.3")
    },
    meta("process stop", "servers", "0.3"),
    meta("process restart", "servers", "0.3"),
    Meta {
        examples: &[
            "locdev process mode queue auto",
            "locdev process mode scheduler manual shop",
            "locdev process mode queue manual --command \"php artisan queue:work --queue=high,default\"",
            "locdev process mode reverb never",
        ],
        notes: &[
            "Offered processes start as ask: nothing runs until you decide. auto starts it with the site; choosing one turns its alternatives' auto off.",
        ],
        ..meta("process mode", "servers", "0.3")
    },
    Meta {
        examples: &[
            "locdev process add mailer --command \"php artisan queue:work --queue=mail\" --label \"Mail queue\"",
        ],
        notes: &["{port} in the command gets a free port of its own."],
        ..meta("process add", "servers", "0.3")
    },
    meta("process remove", "servers", "0.3"),
    Meta {
        examples: &["locdev process logs queue -f"],
        ..meta("process logs", "servers", "0.3")
    },
    meta("db", "db", "0.3"),
    Meta {
        examples: &["locdev db list", "locdev db list shop --json"],
        notes: &[
            "Found in each site's .env (DB_*, DATABASE_URL, REDIS_URL/REDIS_*), SQLite files in the usual places, and LocDev instances. Passwords are never shown or sent to clients.",
        ],
        ..meta("db list", "db", "0.3")
    },
    meta("db connect", "db", "0.3"),
    Meta {
        examples: &[
            "locdev db tables",
            "locdev db tables shop --conn instance:db",
        ],
        notes: &["A database on another host (it may be production) needs --allow-remote."],
        ..meta("db tables", "db", "0.3")
    },
    Meta {
        examples: &[
            "locdev db describe users",
            "locdev db describe public.orders",
        ],
        ..meta("db describe", "db", "0.3")
    },
    Meta {
        examples: &[
            "locdev db rows users --where role=admin --order-by created_at --desc --limit 20",
        ],
        notes: &["Values are compared as text and always sent as parameters."],
        ..meta("db rows", "db", "0.3")
    },
    Meta {
        examples: &[
            "locdev db query \"select count(*) from users\"",
            "locdev db query \"delete from sessions\" shop",
        ],
        notes: &[
            "DROP, TRUNCATE, ALTER … DROP, GRANT/REVOKE and UPDATE/DELETE without WHERE (or with one that is always true, like 1=1) ask for the database's name on a terminal; the daemon checks it again. At most 1,000 rows come back; operations time out after 35 s.",
        ],
        ..meta("db query", "db", "0.3")
    },
    Meta {
        examples: &[
            "locdev db create shop",
            "locdev db create shop shop_testing",
        ],
        notes: &["Only when you run it: a missing database is reported, never created on its own."],
        ..meta("db create", "db", "0.3")
    },
    meta("config", "config", "0.3"),
    Meta {
        examples: &["locdev config list", "locdev config list shop --json"],
        notes: &[
            "Only files that exist are listed; LocDev never creates a config file to fill the list.",
        ],
        ..meta("config list", "config", "0.3")
    },
    Meta {
        examples: &[
            "locdev config show vite.config.ts",
            "locdev config show tsconfig.json --text",
        ],
        notes: &[
            "JavaScript and TypeScript configs are parsed, never executed; values computed in code are shown as code and can only be changed in the text.",
        ],
        ..meta("config show", "config", "0.3")
    },
    Meta {
        examples: &[
            "locdev config set vite.config.ts server.port 5180",
            "locdev config set tsconfig.json compilerOptions.moduleResolution '\"bundler\"'",
        ],
        notes: &[
            "Comments and formatting are kept; the diff is shown before saving, options the installed version doesn't have are refused, and the previous content goes to the history.",
        ],
        ..meta("config set", "config", "0.3")
    },
    Meta {
        examples: &["locdev config check", "locdev config check --json"],
        notes: &[
            "Each finding names the versions it applies to and its official source. Nothing is changed; a suggested fix is a `locdev config set` you run yourself.",
        ],
        ..meta("config check", "config", "0.3")
    },
    meta("config history", "config", "0.3"),
    meta("config revert", "config", "0.3"),
    meta("subdomain", "sites", "0.3"),
    Meta {
        examples: &["locdev subdomain list shop", "locdev subdomain list --json"],
        notes: &[
            "With the wildcard on (the default) every <sub>.<site>.test reaches the site; rules send specific ones elsewhere. Exact rules win over wildcard ones, then the longest.",
        ],
        ..meta("subdomain list", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev subdomain add api shop --port 9000",
            "locdev subdomain add '*.tenants' shop",
            "locdev subdomain add ws shop --process reverb",
        ],
        notes: &[
            "The Host header reaches the app unchanged (plus X-Forwarded-Host), so a multi-tenant app reads the tenant from it.",
        ],
        ..meta("subdomain add", "sites", "0.3")
    },
    meta("subdomain remove", "sites", "0.3"),
    Meta {
        examples: &["locdev subdomain wildcard off shop"],
        notes: &["Off: only the site itself and listed subdomains answer; others get a 404 page."],
        ..meta("subdomain wildcard", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev route api.shop.test",
            "locdev route acme.tenants.shop.test --json",
        ],
        notes: &[
            "DNS and routing are separate checks: the name has to resolve to this machine and the proxy has to know where to send it. Exits non-zero when either fails.",
        ],
        ..meta("route", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev new",
            "locdev new laravel shop --php 8.4",
            "locdev new nextjs web --package-manager pnpm",
            "locdev new vite ui --typescript",
            "locdev new django api",
            "locdev new rust edge -o framework=actix",
        ],
        notes: &[
            "`locdev new <starter> <name>` is the same as `locdev new project <starter> <name>` and takes the same options; alone, in a terminal, `locdev new` asks what to create.",
        ],
        ..meta("new", "sites", "0.3")
    },
    Meta {
        examples: &[
            "locdev new email contact",
            "locdev new email order-shipped --markdown --site shop",
            "locdev new email component button",
        ],
        notes: &[
            "Laravel: runs `php artisan make:mail <Name>Mail --view=emails.<name>` (or --markdown) and fills the view with editable blocks; components go to resources/views/components/mail (<x-mail.name>). Other stacks: emails/<name>.html and emails/components/. Existing files are never overwritten.",
        ],
        ..meta("new email", "email", "0.3")
    },
    meta("new email component", "email", "0.3"),
    Meta {
        examples: &[
            "locdev new list",
            "locdev new project laravel shop -o kit=react --database postgres:db",
            "locdev new project nextjs web --method pnpm -o linter=biome -p zod",
            "locdev new project django api --in ~/Code --yes",
            "locdev new project vue front --php 8.4 --node 22",
        ],
        notes: &[
            "Runs the framework's own installer (laravel new, composer create-project, npm create vite, create-vue, ng new, rails new, cargo init…) without a shell, after showing every command. Output streams here and what you type goes to the installer. The folder must not exist. At the end the folder is registered as a site; with --database, the instance and database are created and the new project's .env points at them.",
            "Plain PHP, Flask, FastAPI and the Rust servers have no generator: LocDev writes its own minimal files (shown in the plan) into the new folder first, then pip or Cargo runs. --php/--node pick the version the installer runs with, and the new site is pinned to it.",
        ],
        ..meta("new project", "sites", "0.3")
    },
    Meta {
        examples: &["locdev new list", "locdev new list --json"],
        ..meta("new list", "sites", "0.3")
    },
    meta("tools", "sites", "0.3"),
    Meta {
        examples: &["locdev tools list"],
        ..meta("tools list", "sites", "0.3")
    },
    Meta {
        examples: &["locdev tools install laravel"],
        notes: &[
            "Only tools with an official one-line install (the Laravel installer through Composer, Rails through gem). Runtimes (PHP, Composer, Node.js, pnpm) are installed with `locdev runtime install`; the others link to their installer.",
        ],
        ..meta("tools install", "sites", "0.3")
    },
    meta("email", "email", "0.3"),
    Meta {
        examples: &["locdev email list", "locdev email list shop --json"],
        ..meta("email list", "email", "0.3")
    },
    Meta {
        examples: &[
            "locdev email preview resources/views/emails/welcome.blade.php > welcome.html",
            "locdev email preview resources/views/emails/welcome.blade.php --framework --text",
        ],
        notes: &[
            "The static preview substitutes sample data without running anything (Blade logic isn't evaluated); --framework lets Laravel render it in a separate, time-limited process.",
        ],
        ..meta("email preview", "email", "0.3")
    },
    Meta {
        examples: &["locdev email send-test resources/views/emails/welcome.blade.php --framework"],
        ..meta("email send-test", "email", "0.3")
    },
    meta("mail", "mail", "0.3"),
    meta("mail status", "mail", "0.3"),
    Meta {
        examples: &[
            "locdev mail list",
            "locdev mail list shop --search invoice --unread",
        ],
        ..meta("mail list", "mail", "0.3")
    },
    Meta {
        examples: &[
            "locdev mail show 1793700000000",
            "locdev mail show 1793700000000 --headers",
            "locdev mail show 1793700000000 --html > mail.html",
        ],
        notes: &[
            "The envelope (MAIL FROM, RCPT TO, HELO, AUTH) is what the SMTP session said; Date and the rest come from the message. Problems are checks against RFC 5322 and MIME.",
        ],
        ..meta("mail show", "mail", "0.3")
    },
    Meta {
        examples: &["locdev mail raw 1793700000000 -o message.eml"],
        ..meta("mail raw", "mail", "0.3")
    },
    Meta {
        examples: &["locdev mail attachment 1793700000000 0 -o invoice.pdf"],
        ..meta("mail attachment", "mail", "0.3")
    },
    meta("mail delete", "mail", "0.3"),
    meta("mail clear", "mail", "0.3"),
    Meta {
        examples: &["locdev mail setup shop"],
        notes: &[
            "Laravel, PHP's mail() (WordPress) and stacks whose .env names SMTP_*/MAIL_*/EMAIL_* variables are wired through the process environment; Django, Rails and Node apps that set SMTP in code get the snippet to add.",
        ],
        ..meta("mail setup", "mail", "0.3")
    },
    Meta {
        examples: &[
            "locdev mail enable shop",
            "locdev mail enable shop --override-real",
        ],
        notes: &[
            "Only the site's processes get the variables; the project's files are never edited. A project pointing at a real mail server (another host, SES, Postmark…) is refused without --override-real.",
        ],
        ..meta("mail enable", "mail", "0.3")
    },
    meta("mail disable", "mail", "0.3"),
    Meta {
        examples: &["locdev mail test", "locdev mail test shop"],
        ..meta("mail test", "mail", "0.3")
    },
    meta("deploy", "deploy", "0.3"),
    Meta {
        examples: &["locdev deploy analyze", "locdev deploy check shop --json"],
        notes: &[
            "Platforms are ranked by what the site needs (runtime, server rendering, WebSockets, workers, scheduler, databases, disk), never by popularity; each fact links its source. Variables are listed by name only. Nothing is sent to any platform.",
        ],
        ..meta("deploy analyze", "deploy", "0.3")
    },
    Meta {
        examples: &[
            "locdev deploy config render",
            "locdev deploy config netlify --write",
        ],
        notes: &[
            "render.yaml for Node.js apps and netlify.toml for static builds; secrets are declared for the platform to ask, never copied from .env. An existing file is never replaced.",
        ],
        ..meta("deploy config", "deploy", "0.3")
    },
    meta("deploy platforms", "deploy", "0.3"),
    Meta {
        examples: &[
            "locdev deploy connect vercel --team acme",
            "printf %s \"$NETLIFY_TOKEN\" | locdev deploy connect netlify",
        ],
        notes: &[
            "The token is read from stdin (never an argument), checked against the platform first, and kept in the system keyring; only the account's name is written to disk.",
        ],
        ..meta("deploy connect", "deploy", "0.3")
    },
    meta("deploy disconnect", "deploy", "0.3"),
    meta("deploy projects", "deploy", "0.3"),
    Meta {
        examples: &[
            "locdev deploy link vercel shop",
            "locdev deploy link netlify my-blog blog",
        ],
        ..meta("deploy link", "deploy", "0.3")
    },
    meta("deploy unlink", "deploy", "0.3"),
    Meta {
        examples: &["locdev deploy status", "locdev deploy status shop --json"],
        notes: &[
            "Variables are listed by name only; LocDev never asks the platform for their values.",
        ],
        ..meta("deploy status", "deploy", "0.3")
    },
    Meta {
        examples: &["locdev deploy run", "locdev deploy run shop --production"],
        notes: &[
            "Builds the commit the branch has pushed to GitHub. Uncommitted changes, unpushed commits, another repository, or --production on a branch other than the production one are refused, so what's deployed matches the code here. Vercel builds that exact commit; Netlify builds the branch through its own Git integration.",
        ],
        ..meta("deploy run", "deploy", "0.3")
    },
    meta("deploy cancel", "deploy", "0.3"),
    Meta {
        examples: &["locdev deploy rollback dpl_8Fk2… shop"],
        notes: &[
            "Vercel: points production at that deployment. Netlify: publishes that deploy again.",
        ],
        ..meta("deploy rollback", "deploy", "0.3")
    },
    Meta {
        examples: &["locdev deploy logs dpl_8Fk2…"],
        notes: &[
            "Netlify's public API has no build logs: the command prints the deploy's page instead.",
        ],
        ..meta("deploy logs", "deploy", "0.3")
    },
    meta("stack", "stack", "0.3"),
    Meta {
        examples: &[
            "locdev stack list",
            "locdev stack list --category framework --json",
        ],
        notes: &[
            "native: LocDev builds the dev command with host and port; script: runs one of the project's package.json scripts (the app must read PORT); custom: recognized, but the command has to come from you.",
        ],
        ..meta("stack list", "stack", "0.3")
    },
    Meta {
        examples: &[
            "locdev stack detect",
            "locdev stack detect ~/Code/api",
            "locdev stack detect shop",
        ],
        notes: &[
            "Only reads files (manifests, lockfiles, config); nothing in the project is run. Monorepos list their packages; add each as its own site.",
        ],
        ..meta("stack detect", "stack", "0.3")
    },
    Meta {
        examples: &["locdev stack inspect", "locdev stack inspect shop --json"],
        ..meta("stack inspect", "stack", "0.3")
    },
    Meta {
        examples: &["locdev stack add tailwind", "locdev stack add inertia shop"],
        ..meta("stack add", "stack", "0.3")
    },
    meta("stack remove", "stack", "0.3"),
    Meta {
        examples: &[
            "locdev stack use flask --apply-command",
            "locdev stack use django shop",
            "locdev stack use auto shop",
        ],
        notes: &[
            "--apply-command replaces the site's command with the adapter's; without it only the stack changes.",
        ],
        ..meta("stack use", "stack", "0.3")
    },
    Meta {
        examples: &[
            "locdev stack definitions",
            "locdev stack definitions --refresh",
            "locdev stack definitions --rollback",
        ],
        notes: &[
            "Definitions are data (detection rules, commands, links), signed with minisign; LocDev only uses a set whose signature matches the key it ships, never an older revision than one already seen, and never a format it doesn't understand.",
            "Checked daily unless offline or `locdev settings set automation.definitions off`.",
        ],
        ..meta("stack definitions", "stack", "0.3")
    },
    meta("env", "env", "0.3"),
    Meta {
        examples: &[
            "locdev env list",
            "locdev env list --profile laravel --json",
        ],
        ..meta("env list", "env", "0.3")
    },
    Meta {
        examples: &[
            "locdev env add STRIPE_SECRET_KEY",
            "printf %s \"$KEY\" | locdev env add STRIPE_SECRET_KEY",
            "locdev env add SENTRY_DSN https://… --plain",
            "locdev env add QUEUE_CONNECTION redis --plain --profile laravel",
        ],
        notes: &[
            "Values are secret (encrypted in the vault) by default; NEXT_PUBLIC_, VITE_ and other browser prefixes default to plain and can't be secret.",
            "Without VALUE the value is read from standard input, so it stays out of your shell history.",
            "Running sites get the change after `locdev reload`.",
        ],
        ..meta("env add", "env", "0.3")
    },
    meta("env remove", "env", "0.3"),
    Meta {
        examples: &[
            "locdev env show STRIPE_SECRET_KEY",
            "locdev env show STRIPE_SECRET_KEY --reveal",
        ],
        notes: &[
            "--reveal asks for the LocDev password when `locdev security` requires one, and is recorded in the history.",
        ],
        ..meta("env show", "env", "0.3")
    },
    meta("env profile", "env", "0.3"),
    meta("env profile list", "env", "0.3"),
    Meta {
        examples: &["locdev env profile add laravel --label Laravel"],
        ..meta("env profile add", "env", "0.3")
    },
    Meta {
        notes: &["Only empty profiles can be removed; the global profile can't."],
        ..meta("env profile remove", "env", "0.3")
    },
    Meta {
        examples: &[
            "locdev env use shop --add-profile laravel",
            "locdev env use shop --disable SENTRY_DSN",
            "locdev env use shop --global false",
        ],
        ..meta("env use", "env", "0.3")
    },
    Meta {
        examples: &["locdev env check", "locdev env check shop --json"],
        notes: &[
            "Shows each variable as injected, skipped, overridden, disabled or blocked, the site's .env files with the order its framework reads them, and every conflict. Values of secrets are never shown.",
        ],
        ..meta("env check", "env", "0.3")
    },
    meta("env conflicts", "env", "0.3"),
    Meta {
        examples: &[
            "locdev env sync --dry-run",
            "locdev env sync shop",
            "locdev env sync shop --file .env.local --yes",
            "locdev env sync shop --file .env.example",
        ],
        notes: &[
            "Without a site, every registered site.",
            "Only appends lines; comments, order, encoding and line endings stay as they were. An encrypted backup is taken first (`locdev env backups`).",
            "Secrets never go into .env.production or .env.staging; .env.example gets empty placeholders.",
        ],
        ..meta("env sync", "env", "0.3")
    },
    meta("env backups", "env", "0.3"),
    meta("env restore", "env", "0.3"),
    meta("env history", "env", "0.3"),
    meta("settings", "system", "0.2"),
    meta("settings show", "system", "0.2"),
    Meta {
        examples: &[
            "locdev settings set automation.audit weekly",
            "locdev settings set network.offline on",
            "locdev settings set notifications.native on",
            "locdev settings set env.conflicts any_file",
            "locdev settings set automation.definitions off",
        ],
        ..meta("settings set", "system", "0.2")
    },
    meta("startup", "system", "0.3"),
    meta("startup status", "system", "0.3"),
    Meta {
        notes: &[
            "systemd --user on Linux, a LaunchAgent on macOS, the Run key on Windows. No administrator rights.",
            "Was `locdev service install` until 0.3; that still works without a service name.",
        ],
        ..meta("startup install", "system", "0.3")
    },
    meta("startup uninstall", "system", "0.3"),
    meta("service", "services", "0.3"),
    Meta {
        examples: &[
            "locdev service list",
            "locdev service list --refresh --json",
        ],
        notes: &[
            "System installations (PATH, /usr/lib/postgresql/*, Homebrew, Program Files) are used as they are and never changed or removed.",
        ],
        ..meta("service list", "services", "0.3")
    },
    meta("service status", "services", "0.3"),
    Meta {
        examples: &["locdev service available postgres"],
        notes: &[
            "PostgreSQL: builds of the official source by theseus-rs (all OS). MariaDB: MariaDB Foundation (Linux, Windows). MongoDB: MongoDB, Inc. (Ubuntu 22.04+/Debian 12, macOS, Windows). Valkey: download.valkey.io (Linux).",
            "MySQL (MD5 only) and Redis (no binaries) can't be downloaded: use a system install, or MariaDB/Valkey.",
        ],
        ..meta("service available", "services", "0.3")
    },
    Meta {
        examples: &[
            "locdev service install postgresql",
            "locdev service install valkey 9.1.3",
        ],
        notes: &[
            "Runs as a background job; the download is installed only when its SHA-256 matches the publisher's.",
        ],
        ..meta("service install", "services", "0.3")
    },
    meta("service uninstall", "services", "0.3"),
    Meta {
        examples: &["locdev service packages mysql"],
        notes: &[
            "macOS (and Linux with Homebrew): versioned formulae (mysql@8.4, postgresql@16, mongodb-community@8.0). Debian/Ubuntu: the distribution's package. Windows: winget (MySQL 8.0/8.4).",
        ],
        ..meta("service packages", "services", "0.4")
    },
    Meta {
        examples: &[
            "locdev service install-package mysql mysql@8.4",
            "locdev service install-package mysql oracle-mysql-8.4 --yes",
        ],
        notes: &[
            "Runs the package manager in a terminal as your user (sudo asks there on Linux); the result is a system install LocDev finds and never changes.",
        ],
        ..meta("service install-package", "services", "0.4")
    },
    Meta {
        examples: &[
            "locdev service create db --service postgres --start",
            "locdev service create cache --service valkey --autostart",
            "locdev service create legacy --service postgres --version 16 --port 5433",
        ],
        notes: &[
            "Each instance has its own data folder, port, config and logs; servers listen on 127.0.0.1 only, with passwordless local users (development only).",
        ],
        ..meta("service create", "services", "0.3")
    },
    Meta {
        examples: &["locdev service start db", "locdev service start postgresql"],
        notes: &["A service name works when it has a single instance."],
        ..meta("service start", "services", "0.3")
    },
    meta("service stop", "services", "0.3"),
    meta("service restart", "services", "0.3"),
    meta("service autostart", "services", "0.3"),
    Meta {
        examples: &[
            "locdev service remove db",
            "locdev service remove db --delete-data",
        ],
        notes: &[
            "The data is kept unless --delete-data, which asks for the instance's name; backups are always kept.",
        ],
        ..meta("service remove", "services", "0.3")
    },
    Meta {
        notes: &[
            "A cold copy: the instance stops, its data folder is archived, and it starts again.",
        ],
        ..meta("service backup", "services", "0.3")
    },
    meta("service backups", "services", "0.3"),
    Meta {
        notes: &["The current data is moved to data.before-<time>, not deleted."],
        ..meta("service restore", "services", "0.3")
    },
    meta("service logs", "services", "0.3"),
    meta("runtime", "runtimes", "0.2"),
    Meta {
        examples: &["locdev php show", "locdev php show --line 8.3 --json"],
        notes: &[
            "LocDev's sites run on PHP's built-in server, which reads the same files as the php command; LocDev's own file is added to the sites it runs through PHP_INI_SCAN_DIR, after PHP's scan folder. A php.ini LocDev didn't write is never changed.",
        ],
        ..meta("php", "runtimes", "0.3")
    },
    meta("php lines", "runtimes", "0.3"),
    meta("php show", "runtimes", "0.3"),
    Meta {
        examples: &[
            "locdev php set memory_limit=512M upload_max_filesize=64M post_max_size=64M",
            "locdev php set date.timezone=America/Sao_Paulo --line 8.3 --yes",
        ],
        notes: &[
            "The change is shown first, backed up, checked against what PHP actually loads (and put back if PHP refuses it); only running sites on that PHP line restart.",
        ],
        ..meta("php set", "runtimes", "0.3")
    },
    Meta {
        examples: &["locdev php unset memory_limit"],
        ..meta("php unset", "runtimes", "0.3")
    },
    meta("php reset", "runtimes", "0.3"),
    Meta {
        examples: &["locdev php edit --line 8.4"],
        notes: &[
            "Lines LocDev doesn't offer in the form (xdebug.mode…) are kept; a syntax error is refused before anything changes.",
        ],
        ..meta("php edit", "runtimes", "0.3")
    },
    Meta {
        examples: &["locdev php file /etc/php/8.3/cli/php.ini"],
        notes: &["Only files that PHP line loads; read-only."],
        ..meta("php file", "runtimes", "0.3")
    },
    meta("php history", "runtimes", "0.3"),
    meta("php restore", "runtimes", "0.3"),
    Meta {
        examples: &[
            "locdev runtime extensions",
            "locdev runtime extensions shop --json",
        ],
        notes: &[
            "Required: ext-* in composer.json and composer.lock, the framework's requirements (Laravel), and drivers named in .env (DB_CONNECTION, Redis with phpredis). Checked against `php -m` of the site's own PHP; exits with 1 when something is missing.",
            "Nothing is installed: the output says how for that PHP (apt package, pecl, php.ini). LocDev's PHP builds are static, with a fixed set of extensions.",
        ],
        ..meta("runtime extensions", "runtimes", "0.3")
    },
    meta("runtime list", "runtimes", "0.2"),
    Meta {
        examples: &["locdev runtime modules", "locdev runtime modules 8.3"],
        notes: &[
            "Also lists the extension files its extension folder has but php.ini doesn't load (Windows: php_*.dll).",
        ],
        ..meta("runtime modules", "runtimes", "0.4")
    },
    Meta {
        examples: &[
            "locdev runtime available node",
            "locdev runtime available pnpm --all",
        ],
        ..meta("runtime available", "runtimes", "0.2")
    },
    Meta {
        examples: &[
            "locdev runtime install node lts",
            "locdev runtime install php 8.3",
            "locdev runtime install pnpm 9",
            "locdev runtime install composer 2",
        ],
        ..meta("runtime install", "runtimes", "0.2")
    },
    Meta {
        examples: &["locdev runtime update node 22"],
        ..meta("runtime update", "runtimes", "0.2")
    },
    meta("runtime uninstall", "runtimes", "0.2"),
    Meta {
        examples: &[
            "locdev runtime default node 22",
            "locdev runtime default php --clear",
        ],
        ..meta("runtime default", "runtimes", "0.2")
    },
    Meta {
        examples: &[
            "locdev runtime terminal on",
            "locdev runtime terminal",
            "locdev runtime terminal off",
        ],
        notes: &[
            "Linux/macOS: adds a marked block to ~/.bashrc and ~/.zshrc (and a file in fish's conf.d); off removes exactly that.",
            "Windows: puts the folder first on the user's Path; a PHP or Node.js installed for all users still comes before it.",
        ],
        ..meta("runtime terminal", "runtimes", "0.3")
    },
    // Everything the app does, from the terminal too (0.3).
    Meta {
        examples: &[
            "locdev git diff src/app.ts",
            "locdev git diff composer.json --staged",
        ],
        ..meta("git diff", "git", "0.3")
    },
    meta("git show", "git", "0.3"),
    meta("git remotes", "git", "0.3"),
    Meta {
        examples: &["locdev git stage src/app.ts README.md"],
        ..meta("git stage", "git", "0.3")
    },
    meta("git unstage", "git", "0.3"),
    Meta {
        examples: &[
            "locdev git discard src/app.ts",
            "locdev git discard tmp.txt --confirm \"discard 1 file\"",
        ],
        notes: &[
            "Destructive: shows what is lost, asks for the phrase and the LocDev password (when set); a backup is saved first when backups are on.",
        ],
        ..meta("git discard", "git", "0.3")
    },
    Meta {
        examples: &["locdev git commit -m \"fix: checkout total\""],
        ..meta("git commit", "git", "0.3")
    },
    Meta {
        examples: &["locdev git branch feature/cart --switch"],
        ..meta("git branch", "git", "0.3")
    },
    meta("git switch", "git", "0.3"),
    meta("git delete-branch", "git", "0.3"),
    meta("git merge", "git", "0.3"),
    meta("git merge-abort", "git", "0.3"),
    meta("git fetch", "git", "0.3"),
    meta("git pull", "git", "0.3"),
    Meta {
        examples: &["locdev git push", "locdev git push --force"],
        notes: &["--force uses --force-with-lease and is previewed as destructive."],
        ..meta("git push", "git", "0.3")
    },
    meta("git stash", "git", "0.3"),
    Meta {
        examples: &["locdev git stash push -m \"wip\" --untracked"],
        ..meta("git stash push", "git", "0.3")
    },
    meta("git stash apply", "git", "0.3"),
    Meta {
        examples: &["locdev git worktree list"],
        ..meta("git worktree list", "git", "0.4")
    },
    meta("git worktree", "git", "0.4"),
    Meta {
        examples: &[
            "locdev git worktree add feature/login --create --site-for-it --copy-env",
            "locdev git worktree add hotfix --path /src/shop-hotfix",
        ],
        notes: &[
            "The folder goes next to the repository as <repo>-<branch> unless --path; with --site-for-it it becomes its own site (<repo>-<branch>.test), so two branches run side by side.",
            "--copy-env copies the untracked .env files of the main folder that the new one lacks; it never overwrites.",
        ],
        ..meta("git worktree add", "git", "0.4")
    },
    Meta {
        examples: &["locdev git worktree remove ../shop-feature-login"],
        notes: &[
            "Destructive: the folder is deleted (its branch and commits stay). Uncommitted changes block it unless --force; a snapshot of tracked changes goes under refs/locdev/backups/ first.",
        ],
        ..meta("git worktree remove", "git", "0.4")
    },
    meta("git worktree prune", "git", "0.4"),
    meta("git stash pop", "git", "0.3"),
    meta("git stash drop", "git", "0.3"),
    meta("git remote", "git", "0.3"),
    meta("git remote add", "git", "0.3"),
    meta("git remote remove", "git", "0.3"),
    meta("git remote set-url", "git", "0.3"),
    meta("git revert", "git", "0.3"),
    meta("git restore", "git", "0.3"),
    Meta {
        examples: &[
            "locdev git reset HEAD~1",
            "locdev git reset origin/main --mode hard",
        ],
        ..meta("git reset", "git", "0.3")
    },
    Meta {
        notes: &[
            "Needs an OAuth App client id in LocDev's build; otherwise connect with a token (`locdev github connect`).",
        ],
        ..meta("github login", "git", "0.3")
    },
    Meta {
        examples: &[
            "locdev github show",
            "locdev github show runs",
            "locdev github show pulls shop",
        ],
        ..meta("github show", "git", "0.3")
    },
    meta("github rerun", "git", "0.3"),
    meta("github cancel-run", "git", "0.3"),
    Meta {
        examples: &["locdev github dispatch 1234567 --ref main"],
        ..meta("github dispatch", "git", "0.3")
    },
    Meta {
        examples: &[
            "locdev security options",
            "locdev security options --backups true",
        ],
        ..meta("security options", "git", "0.3")
    },
    Meta {
        examples: &["locdev actions save ci.yml --from ./ci.yml"],
        ..meta("actions save", "git", "0.3")
    },
    Meta {
        examples: &["locdev db insert users --set name=Ana --set email=ana@example.test"],
        ..meta("db insert", "db", "0.3")
    },
    Meta {
        examples: &["locdev db update users --key id=5 --set name=Ana --null deleted_at"],
        ..meta("db update", "db", "0.3")
    },
    meta("db delete", "db", "0.3"),
    Meta {
        examples: &["locdev db keys", "locdev db keys \"session:*\""],
        ..meta("db keys", "db", "0.3")
    },
    meta("db get", "db", "0.3"),
    meta("db set", "db", "0.3"),
    Meta {
        examples: &[
            "locdev db expire cache:home 300",
            "locdev db expire cache:home none",
        ],
        ..meta("db expire", "db", "0.3")
    },
    meta("db del", "db", "0.3"),
    meta("email show", "email", "0.3"),
    Meta {
        examples: &[
            "locdev email block resources/views/mail/welcome.blade.php b2 --set text=\"Welcome!\"",
        ],
        ..meta("email block", "email", "0.3")
    },
    Meta {
        notes: &["Uses $VISUAL, then $EDITOR (vi; Notepad on Windows)."],
        ..meta("email edit", "email", "0.3")
    },
    meta("email save", "email", "0.3"),
    meta("email history", "email", "0.3"),
    meta("email version", "email", "0.3"),
    meta("email restore", "email", "0.3"),
    Meta {
        examples: &["locdev email sample resources/views/mail/welcome.blade.php sample.json"],
        ..meta("email sample", "email", "0.3")
    },
    Meta {
        examples: &[
            "locdev tools search npm tailwind",
            "locdev tools search packagist sanctum",
        ],
        ..meta("tools search", "sites", "0.3")
    },
    meta("tools terminals", "sites", "0.3"),
    meta("tools attach", "sites", "0.3"),
    meta("tools kill", "sites", "0.3"),
    meta("tools close", "sites", "0.3"),
];

pub fn catalog() -> Catalog {
    let mut entries = Vec::new();
    walk(&Cli::command(), "", &mut entries);

    let mut categories: Vec<Category> = CATEGORIES
        .iter()
        .map(|(id, label)| Category {
            id,
            label,
            commands: Vec::new(),
        })
        .collect();
    for (category, entry) in entries {
        categories
            .iter_mut()
            .find(|c| c.id == category)
            .unwrap_or_else(|| panic!("unknown category {category}"))
            .commands
            .push(entry);
    }
    categories.retain(|c| !c.commands.is_empty());
    Catalog { categories }
}

fn walk(command: &clap::Command, prefix: &str, out: &mut Vec<(&'static str, Entry)>) {
    for sub in command
        .get_subcommands()
        .filter(|c| !c.is_hide_set() && c.get_name() != "help")
    {
        let path = if prefix.is_empty() {
            sub.get_name().to_string()
        } else {
            format!("{prefix} {}", sub.get_name())
        };
        let meta = META
            .iter()
            .find(|m| m.path == path)
            .unwrap_or_else(|| panic!("`locdev {path}` has no entry in catalog::META"));
        out.push((
            meta.category,
            Entry {
                usage: format!("locdev {path}"),
                name: path.clone(),
                about: sub.get_about().map(|s| s.to_string()).unwrap_or_default(),
                details: sub.get_long_about().map(|s| s.to_string()),
                aliases: sub.get_visible_aliases().map(str::to_string).collect(),
                args: sub
                    .get_arguments()
                    .filter(|a| !is_builtin(a))
                    .map(arg)
                    .collect(),
                since: meta.since,
                privileged: meta.privileged,
                examples: meta.examples.to_vec(),
                notes: meta.notes.to_vec(),
            },
        ));
        walk(sub, &path, out);
    }
}

fn is_builtin(arg: &Arg) -> bool {
    matches!(arg.get_id().as_str(), "help" | "version" | "json")
}

fn arg(arg: &Arg) -> ArgEntry {
    let positional = arg.is_positional();
    let takes_value = !matches!(
        arg.get_action(),
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count
    );
    let name = if positional {
        format!("<{}>", arg.get_id())
    } else {
        let long = arg.get_long().map(|l| format!("--{l}"));
        let short = arg.get_short().map(|s| format!("-{s}"));
        [short, long]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ")
    };
    ArgEntry {
        name,
        positional,
        required: arg.is_required_set(),
        takes_value,
        help: arg.get_help().map(|h| h.to_string()).unwrap_or_default(),
        default: arg
            .get_default_values()
            .first()
            .map(|v| v.to_string_lossy().into_owned())
            .filter(|_| takes_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed copies the app and the docs render. Regenerate with
    /// `make cli-catalog` after changing a command.
    const COPIES: [&str; 2] = [
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../desktop/src/generated/cli.json"
        ),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/generated/cli.json"),
    ];

    #[test]
    fn every_command_is_described() {
        // `walk` panics on a command without metadata.
        let catalog = catalog();
        let total: usize = catalog.categories.iter().map(|c| c.commands.len()).sum();
        assert_eq!(
            total,
            META.len(),
            "META has entries for commands that no longer exist"
        );
        let secure = catalog
            .categories
            .iter()
            .flat_map(|c| &c.commands)
            .find(|e| e.name == "secure")
            .unwrap();
        assert!(secure.privileged);
        assert!(
            secure
                .args
                .iter()
                .any(|a| a.name == "-y, --yes" && !a.takes_value)
        );
    }

    #[test]
    fn committed_catalog_is_current() {
        let fresh = serde_json::to_string_pretty(&catalog()).unwrap() + "\n";
        let update = std::env::var_os("LOCDEV_UPDATE_CATALOG").is_some();
        for copy in COPIES {
            if update {
                std::fs::create_dir_all(std::path::Path::new(copy).parent().unwrap()).unwrap();
                std::fs::write(copy, &fresh).unwrap();
            }
            let committed = std::fs::read_to_string(copy).unwrap_or_default();
            assert!(
                committed == fresh,
                "{copy} is out of date; run `make cli-catalog`"
            );
        }
    }
}
