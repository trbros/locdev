//! What each platform can run, from its own documentation. Every capability
//! carries the page it comes from; `REVIEWED` is when these were last checked.
//! The order here means nothing: fits are ranked by what the project needs.

/// When the facts below were compiled from the linked pages. The generated
/// file formats (render.yaml, netlify.toml) were checked against their specs
/// that day; Railway's railway.json turned out deprecated, so none is written.
pub const REVIEWED: &str = "2026-11-03";

/// How a platform offers something, and the page that says so.
pub type Offer = Option<(&'static str, &'static str)>;

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub model: &'static str,
    pub docs: &'static str,
    pub summary: &'static str,
    /// Runtimes it builds from source without a Dockerfile.
    pub runtimes: &'static [&'static str],
    /// Runs any app from a Dockerfile.
    pub docker: bool,
    pub static_sites: bool,
    /// Server-rendering frameworks it deploys directly (zero config or an official adapter).
    pub ssr_frameworks: &'static [&'static str],
    /// Backend frameworks it runs as functions (no long-lived process).
    pub function_backends: &'static [&'static str],
    /// Keeps a process running (app servers, workers).
    pub servers: bool,
    pub websockets: Offer,
    pub workers: Offer,
    pub scheduler: Offer,
    pub postgres: Offer,
    pub mysql: Offer,
    pub redis: Offer,
    pub mongo: Offer,
    /// A disk that survives deploys and restarts.
    pub disk: Offer,
    pub strengths: &'static [(&'static str, Option<&'static str>)],
    pub limits: &'static [(&'static str, Option<&'static str>)],
    /// A file LocDev can write for it (see `generate.rs`).
    pub config_file: Option<&'static str>,
    pub setup: Option<&'static str>,
}

pub const PROVIDERS: &[Provider] = &[
    Provider {
        id: "vercel",
        name: "Vercel",
        model: "serverless",
        docs: "https://vercel.com/docs",
        summary: "Functions and a CDN; frameworks are detected and built without configuration.",
        runtimes: &["node", "python", "go", "ruby", "static"],
        docker: false,
        static_sites: true,
        ssr_frameworks: &[
            "nextjs",
            "nuxt",
            "astro",
            "sveltekit",
            "remix",
            "solid-start",
            "qwik",
            "gatsby",
            "angular",
        ],
        function_backends: &[
            "express", "hono", "fastify", "nestjs", "elysia", "fastapi", "flask",
        ],
        servers: false,
        websockets: Some((
            "Functions serve WebSockets with Fluid compute, within the function duration limit; rooms and presence go in an external store",
            "https://vercel.com/docs/functions/websockets",
        )),
        workers: None,
        scheduler: Some((
            "Cron Jobs call a route on a schedule",
            "https://vercel.com/docs/cron-jobs",
        )),
        postgres: Some((
            "through Marketplace integrations (e.g. Neon)",
            "https://vercel.com/marketplace",
        )),
        mysql: None,
        redis: Some((
            "through Marketplace integrations (e.g. Upstash)",
            "https://vercel.com/marketplace",
        )),
        mongo: None,
        disk: None,
        strengths: &[
            (
                "Next.js is built and served by its own authors' platform, with no configuration",
                Some("https://vercel.com/docs/frameworks/full-stack/nextjs"),
            ),
            (
                "Express, Hono, Fastify, NestJS, Elysia, FastAPI and Flask deploy as functions with zero configuration",
                Some("https://vercel.com/docs/frameworks/backend"),
            ),
            (
                "Preview deployment for every branch and pull request",
                Some(
                    "https://vercel.com/docs/deployments/environments#preview-environment-pre-production",
                ),
            ),
        ],
        limits: &[
            (
                "No server runs in the background between requests; WebSocket connections end at the function's maximum duration",
                Some("https://vercel.com/docs/frameworks/backend"),
            ),
            (
                "Long-running queue workers don't fit (Python's Celery/Dramatiq run as queue-triggered functions)",
                Some("https://vercel.com/docs/frameworks/backend"),
            ),
            (
                "No PHP runtime of its own (community runtimes only)",
                Some("https://vercel.com/docs/functions/runtimes"),
            ),
        ],
        config_file: None,
        setup: Some(
            "Import the Git repository in the dashboard, or run `vercel` in the project; the framework is detected.",
        ),
    },
    Provider {
        id: "netlify",
        name: "Netlify",
        model: "serverless",
        docs: "https://docs.netlify.com/",
        summary: "Static sites with Functions; framework adapters for server rendering.",
        runtimes: &["node", "go", "static"],
        docker: false,
        static_sites: true,
        ssr_frameworks: &[
            "nextjs",
            "nuxt",
            "astro",
            "sveltekit",
            "remix",
            "solid-start",
            "qwik",
            "gatsby",
            "angular",
        ],
        function_backends: &[],
        servers: false,
        websockets: None,
        workers: Some((
            "Background Functions run up to 15 minutes",
            "https://docs.netlify.com/build/functions/background-functions/",
        )),
        scheduler: Some((
            "Scheduled Functions",
            "https://docs.netlify.com/build/functions/scheduled-functions/",
        )),
        postgres: Some((
            "Netlify Database (managed Postgres, credit-based plans)",
            "https://docs.netlify.com/build/data-and-storage/netlify-database/",
        )),
        mysql: None,
        redis: None,
        mongo: None,
        disk: None,
        strengths: &[
            (
                "Next.js runs through Netlify's OpenNext-based adapter, set up automatically",
                Some("https://docs.netlify.com/frameworks/next-js/overview/"),
            ),
            (
                "Deploy previews per pull request",
                Some("https://docs.netlify.com/site-deploys/deploy-previews/"),
            ),
        ],
        limits: &[(
            "Functions run per event in an ephemeral runtime; there is no long-running process for WebSockets or queue consumers",
            Some("https://docs.netlify.com/build/functions/overview/"),
        )],
        config_file: Some("netlify.toml"),
        setup: None,
    },
    Provider {
        id: "cloudflare",
        name: "Cloudflare Workers",
        model: "edge",
        docs: "https://developers.cloudflare.com/workers/",
        summary: "JavaScript at the edge (V8 isolates) with static assets, D1, KV, R2, Queues and Durable Objects.",
        runtimes: &["node", "static"],
        docker: false,
        static_sites: true,
        ssr_frameworks: &[
            "nextjs",
            "nuxt",
            "astro",
            "sveltekit",
            "remix",
            "solid-start",
            "qwik",
            "hono",
            "angular",
        ],
        function_backends: &["hono"],
        servers: false,
        websockets: Some((
            "with Durable Objects",
            "https://developers.cloudflare.com/durable-objects/best-practices/websockets/",
        )),
        workers: Some((
            "Cloudflare Queues (code written for it)",
            "https://developers.cloudflare.com/queues/",
        )),
        scheduler: Some((
            "Cron Triggers",
            "https://developers.cloudflare.com/workers/configuration/cron-triggers/",
        )),
        postgres: Some((
            "an external database through Hyperdrive",
            "https://developers.cloudflare.com/hyperdrive/",
        )),
        mysql: Some((
            "an external database through Hyperdrive",
            "https://developers.cloudflare.com/hyperdrive/",
        )),
        redis: None,
        mongo: None,
        disk: None,
        strengths: &[
            (
                "Next.js through the OpenNext adapter (@opennextjs/cloudflare)",
                Some("https://developers.cloudflare.com/workers/framework-guides/web-apps/nextjs/"),
            ),
            (
                "D1 is a managed SQLite database",
                Some("https://developers.cloudflare.com/d1/"),
            ),
        ],
        limits: &[
            (
                "Not a full Node.js: APIs come through the nodejs_compat flag, and some packages don't run",
                Some("https://developers.cloudflare.com/workers/runtime-apis/nodejs/"),
            ),
            (
                "No local disk; files go to R2",
                Some("https://developers.cloudflare.com/r2/"),
            ),
            (
                "No PHP runtime",
                Some("https://developers.cloudflare.com/workers/languages/"),
            ),
        ],
        config_file: None,
        setup: Some(
            "`npm create cloudflare@latest -- --existing-script` or, for Next.js, `npx @opennextjs/cloudflare migrate`; both write wrangler config.",
        ),
    },
    Provider {
        id: "railway",
        name: "Railway",
        model: "containers",
        docs: "https://docs.railway.com/",
        summary: "Long-running services built from source (Railpack) or a Dockerfile, with databases next to them.",
        runtimes: &[
            "node", "php", "python", "ruby", "go", "java", "rust", "deno", "static",
        ],
        docker: true,
        static_sites: true,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: true,
        websockets: Some((
            "services are long-running, so they hold connections",
            "https://docs.railway.com/reference/public-networking",
        )),
        workers: Some((
            "a second service from the same repo with the worker command",
            "https://docs.railway.com/guides/services",
        )),
        scheduler: Some((
            "a service with a cron schedule",
            "https://docs.railway.com/reference/cron-jobs",
        )),
        postgres: Some((
            "PostgreSQL service",
            "https://docs.railway.com/guides/postgresql",
        )),
        mysql: Some(("MySQL service", "https://docs.railway.com/guides/mysql")),
        redis: Some(("Redis service", "https://docs.railway.com/guides/redis")),
        mongo: Some(("MongoDB service", "https://docs.railway.com/guides/mongodb")),
        disk: Some(("volumes", "https://docs.railway.com/reference/volumes")),
        strengths: &[
            (
                "App, workers and databases in one project, private network between them",
                Some("https://docs.railway.com/reference/private-networking"),
            ),
            (
                "Infrastructure as code in .railway/railway.ts, previewed with `railway config plan`",
                Some("https://docs.railway.com/infrastructure-as-code"),
            ),
        ],
        limits: &[
            (
                "Usage-based pricing; services keep running (and billing) unless configured to sleep",
                Some("https://docs.railway.com/reference/pricing"),
            ),
            (
                "railway.json (config as code) is deprecated and stops working for legacy services on 2026-12-01",
                Some("https://docs.railway.com/reference/config-as-code"),
            ),
        ],
        config_file: None,
        setup: Some(
            "Deploy from the GitHub repository in the dashboard; for infrastructure as code, `npm install railway`, write .railway/railway.ts, then `railway config plan` and `railway config apply`.",
        ),
    },
    Provider {
        id: "render",
        name: "Render",
        model: "containers",
        docs: "https://render.com/docs",
        summary: "Web services, background workers, cron jobs and static sites, described in a render.yaml Blueprint.",
        runtimes: &["node", "python", "ruby", "go", "rust", "static"],
        docker: true,
        static_sites: true,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: true,
        websockets: Some((
            "web services accept WebSocket connections",
            "https://render.com/docs/websocket",
        )),
        workers: Some((
            "background worker services",
            "https://render.com/docs/background-workers",
        )),
        scheduler: Some(("cron job services", "https://render.com/docs/cronjobs")),
        postgres: Some(("Render Postgres", "https://render.com/docs/postgresql")),
        mysql: None,
        redis: Some((
            "Render Key Value (Redis-compatible)",
            "https://render.com/docs/key-value",
        )),
        mongo: None,
        disk: Some((
            "persistent disks (paid instances)",
            "https://render.com/docs/disks",
        )),
        strengths: &[(
            "The whole setup (app, worker, cron, database) in one Blueprint file",
            Some("https://render.com/docs/blueprint-spec"),
        )],
        limits: &[
            (
                "PHP runs only through a Dockerfile",
                Some("https://render.com/docs/language-support"),
            ),
            (
                "Free web services spin down when idle",
                Some("https://render.com/docs/free"),
            ),
        ],
        config_file: Some("render.yaml"),
        setup: None,
    },
    Provider {
        id: "fly",
        name: "Fly.io",
        model: "vm",
        docs: "https://fly.io/docs/",
        summary: "Docker images as Machines (small VMs) in regions you choose; `fly launch` writes the Dockerfile and fly.toml.",
        runtimes: &[],
        docker: true,
        static_sites: false,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: true,
        websockets: Some(("Machines are full VMs", "https://fly.io/docs/machines/")),
        workers: Some((
            "process groups in fly.toml",
            "https://fly.io/docs/apps/processes/",
        )),
        scheduler: Some((
            "a process running a cron daemon such as supercronic",
            "https://fly.io/docs/blueprints/supercronic/",
        )),
        postgres: Some(("Fly Managed Postgres", "https://fly.io/docs/mpg/")),
        mysql: None,
        redis: Some((
            "Upstash Redis through Fly",
            "https://fly.io/docs/upstash/redis/",
        )),
        mongo: None,
        disk: Some((
            "volumes, attached to one Machine each",
            "https://fly.io/docs/volumes/",
        )),
        strengths: &[(
            "`fly launch` detects Laravel, Rails, Django, Node and more and writes a working Dockerfile",
            Some("https://fly.io/docs/launch/"),
        )],
        limits: &[(
            "A volume belongs to one Machine; scaling out needs a database, not files",
            Some("https://fly.io/docs/volumes/overview/"),
        )],
        config_file: None,
        setup: Some(
            "`fly launch` in the project writes fly.toml and a Dockerfile for the detected framework.",
        ),
    },
    Provider {
        id: "forge",
        name: "Laravel Forge",
        model: "managed servers",
        docs: "https://forge.laravel.com/docs",
        summary: "Provisions and manages PHP servers on your cloud account (DigitalOcean, AWS, Hetzner…).",
        runtimes: &["php"],
        docker: false,
        static_sites: true,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: true,
        websockets: Some(("Reverb on the server", "https://forge.laravel.com/docs")),
        workers: Some((
            "queue workers (daemons) managed by Forge",
            "https://forge.laravel.com/docs",
        )),
        scheduler: Some((
            "the Laravel scheduler, enabled per site",
            "https://forge.laravel.com/docs",
        )),
        postgres: Some(("installed on the server", "https://forge.laravel.com/docs")),
        mysql: Some(("installed on the server", "https://forge.laravel.com/docs")),
        redis: Some(("installed on the server", "https://forge.laravel.com/docs")),
        mongo: None,
        disk: Some(("the server's own disk", "https://forge.laravel.com/docs")),
        strengths: &[(
            "Made for Laravel: deploy script, queues, scheduler and SSL handled for you",
            Some("https://forge.laravel.com/docs"),
        )],
        limits: &[(
            "You pay for and own the servers; Forge manages them",
            Some("https://forge.laravel.com/docs"),
        )],
        config_file: None,
        setup: Some(
            "Connect a cloud provider and the Git repository in Forge; the deploy script runs on the server.",
        ),
    },
    Provider {
        id: "vps",
        name: "Your own server (VPS)",
        model: "your server",
        docs: "",
        summary: "Anything runs; you install and keep up the web server, runtime, process manager, TLS and backups.",
        runtimes: &[
            "node", "php", "python", "ruby", "go", "java", "dotnet", "rust", "deno", "static",
        ],
        docker: true,
        static_sites: true,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: true,
        websockets: Some((
            "behind a reverse proxy that passes upgrades",
            "https://nginx.org/en/docs/http/websocket.html",
        )),
        workers: Some((
            "a process manager (systemd, Supervisor)",
            "https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html",
        )),
        scheduler: Some((
            "cron or systemd timers",
            "https://www.freedesktop.org/software/systemd/man/latest/systemd.timer.html",
        )),
        postgres: Some(("installed by you", "https://www.postgresql.org/download/")),
        mysql: Some(("installed by you", "https://dev.mysql.com/doc/")),
        redis: Some((
            "installed by you",
            "https://redis.io/docs/latest/operate/oss_and_stack/install/",
        )),
        mongo: Some((
            "installed by you",
            "https://www.mongodb.com/docs/manual/installation/",
        )),
        disk: Some((
            "the server's own disk",
            "https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html",
        )),
        strengths: &[("No platform limits, fixed price", None)],
        limits: &[(
            "Security updates, TLS renewal, backups and monitoring are yours",
            None,
        )],
        config_file: None,
        setup: None,
    },
    Provider {
        id: "github-pages",
        name: "GitHub Pages",
        model: "static",
        docs: "https://docs.github.com/en/pages",
        summary: "Static files from a repository or an Actions workflow.",
        runtimes: &["static"],
        docker: false,
        static_sites: true,
        ssr_frameworks: &[],
        function_backends: &[],
        servers: false,
        websockets: None,
        workers: None,
        scheduler: None,
        postgres: None,
        mysql: None,
        redis: None,
        mongo: None,
        disk: None,
        strengths: &[(
            "Free for public repositories, deployed by a GitHub Actions workflow",
            Some(
                "https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages",
            ),
        )],
        limits: &[(
            "Static files only: no server code",
            Some(
                "https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages",
            ),
        )],
        config_file: None,
        setup: Some(
            "Settings → Pages → Source: GitHub Actions, with a workflow that builds and uploads the site (the Actions builder has templates).",
        ),
    },
];
