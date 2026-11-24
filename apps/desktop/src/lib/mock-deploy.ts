// The deploy assistant for the mock backend: a fixed report for a Laravel
// site. The real analysis, platform facts and generators live in `locdev-deploy`.

import { DaemonError } from "./errors";
import type { DeployConfig, DeployReport, DeployRequest, DeployResponse } from "./protocol";

const report: DeployReport = {
  reviewed: "2026-11-03",
  profile: {
    framework: "Laravel",
    version: "12.38.0",
    runtime: "php",
    runtime_version: "^8.3",
    rendering: "server",
    rendering_evidence: "laravel runs as an application server",
    needs: [
      { id: "workers", label: "Queue workers", evidence: "QUEUE_CONNECTION=redis in .env" },
      { id: "scheduler", label: "Scheduled tasks", evidence: "the Laravel scheduler has tasks (routes/console.php)" },
      { id: "postgres", label: "PostgreSQL", evidence: "a connection in the project's .env" },
      { id: "redis", label: "Redis-compatible cache/queue", evidence: "a connection in the project's .env" },
    ],
    build_command: "composer install --no-dev --optimize-autoloader && npm ci && npm run build",
    start_command: null,
    output_dir: null,
    env: [
      { name: "APP_KEY", secret: true, note: "a new key for production (`php artisan key:generate --show`)" },
      { name: "APP_DEBUG", secret: false, note: "false in production" },
      { name: "APP_URL", secret: false, note: "the production URL" },
      { name: "DB_PASSWORD", secret: true, note: "from the production database" },
      { name: "STRIPE_SECRET", secret: true, note: "set it on the platform" },
    ],
  },
  providers: [
    {
      id: "railway",
      name: "Railway",
      model: "containers",
      verdict: "fits",
      summary: "Long-running services built from source (Railpack) or a Dockerfile, with databases next to them.",
      strengths: [
        {
          text: "App, workers and databases in one project, private network between them",
          source: "https://docs.railway.com/reference/private-networking",
        },
      ],
      limitations: [
        {
          text: "Usage-based pricing; services keep running (and billing) unless configured to sleep",
          source: "https://docs.railway.com/reference/pricing",
        },
      ],
      requirements: [
        { text: "Runs as a long-running php service", source: null },
        {
          text: "Queue workers: a second service from the same repo with the worker command",
          source: "https://docs.railway.com/guides/services",
        },
        { text: "Scheduler: a service with a cron schedule", source: "https://docs.railway.com/reference/cron-jobs" },
        { text: "PostgreSQL: PostgreSQL service", source: "https://docs.railway.com/guides/postgresql" },
        { text: "Set 5 environment variables (3 secret) in its settings", source: null },
      ],
      docs: "https://docs.railway.com/",
      config_file: null,
      setup: "Deploy from the GitHub repository in the dashboard.",
    },
    {
      id: "render",
      name: "Render",
      model: "containers",
      verdict: "needs_changes",
      summary: "Web services, background workers, cron jobs and static sites, described in a render.yaml Blueprint.",
      strengths: [
        {
          text: "The whole setup (app, worker, cron, database) in one Blueprint file",
          source: "https://render.com/docs/blueprint-spec",
        },
      ],
      limitations: [{ text: "PHP runs only through a Dockerfile", source: "https://render.com/docs/language-support" }],
      requirements: [
        { text: "Build it from a Dockerfile (php isn't built from source here)", source: "https://render.com/docs" },
        { text: "Queue workers: background worker services", source: "https://render.com/docs/background-workers" },
      ],
      docs: "https://render.com/docs",
      config_file: null,
      setup: null,
    },
    {
      id: "vercel",
      name: "Vercel",
      model: "serverless",
      verdict: "not_suitable",
      summary: "Functions and a CDN; frameworks are detected and built without configuration.",
      strengths: [],
      limitations: [
        { text: "It doesn't run php application servers", source: "https://vercel.com/docs" },
        {
          text: "No PHP runtime of its own (community runtimes only)",
          source: "https://vercel.com/docs/functions/runtimes",
        },
      ],
      requirements: [],
      docs: "https://vercel.com/docs",
      config_file: null,
      setup: null,
    },
    {
      id: "netlify",
      name: "Netlify",
      model: "serverless",
      verdict: "not_suitable",
      summary: "Static sites with Functions; framework adapters for server rendering.",
      strengths: [],
      limitations: [{ text: "It doesn't run php application servers", source: "https://docs.netlify.com/" }],
      requirements: [],
      docs: "https://docs.netlify.com/",
      config_file: null,
      setup: null,
    },
  ],
  checks: [
    {
      id: "git",
      level: "ok",
      title: "Working tree clean",
      detail: "What's committed is what a Git-based deploy builds.",
    },
    {
      id: "env-ignored",
      level: "ok",
      title: ".env stays out of Git",
      detail: "It's ignored; set its values on the platform instead.",
    },
    {
      id: "app-debug",
      level: "warn",
      title: "APP_DEBUG is true locally",
      detail: "Fine here; make sure production sets APP_DEBUG=false.",
    },
  ],
};

let written = false;
const netlify: DeployConfig = {
  provider: "netlify",
  file: "netlify.toml",
  content:
    '# Written by LocDev; review before committing.\n\n[build]\n  command = "npm run build"\n  publish = "dist"\n',
  hash: "mock-netlify",
  exists: false,
  notes: ["Environment variables go in the Netlify UI, not in this file."],
};

export function handleDeploy(req: DeployRequest): DeployResponse {
  switch (req.type) {
    case "deploy_analysis":
      return { type: "deploy_report", ...report };
    case "deploy_config":
      if (req.provider !== "netlify")
        throw new DaemonError("app", `LocDev doesn't write a config file for ${req.provider}`, "INVALID_INPUT");
      return { type: "deploy_config", ...netlify, exists: written };
    case "deploy_write_config":
      if (written) throw new DaemonError("app", "netlify.toml already exists; LocDev doesn't overwrite it", "CONFLICT");
      written = true;
      return { type: "deploy_config_written", file: "netlify.toml" };
  }
}
