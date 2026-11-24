// Deploy platforms for the mock backend: Vercel connected with a project,
// Netlify not. The real calls, token storage and Git checks are the daemon's.

import { DaemonError } from "./errors";
import type {
  ProjectView,
  RemoteDeployment,
  RemotePlatform,
  RemoteProject,
  RemoteRequest,
  RemoteResponse,
} from "./protocol";

const now = Date.now();

const platforms: RemotePlatform[] = [
  {
    id: "vercel",
    name: "Vercel",
    docs: "https://vercel.com/docs/rest-api",
    token_url: "https://vercel.com/account/settings/tokens",
    token_help:
      "Account Settings → Tokens → Create Token. Scope it to the team that owns the projects (and give its slug or ID here), with an expiration you're comfortable with.",
    capabilities: { deploy: true, cancel: true, rollback: true, logs: true, env: true, domains: true },
    account: { user: "ana", email: "ana@acme.com", team: "acme" },
  },
  {
    id: "netlify",
    name: "Netlify",
    docs: "https://open-api.netlify.com/",
    token_url: "https://app.netlify.com/user/applications#personal-access-tokens",
    token_help:
      "User settings → Applications → Personal access tokens → New access token. It acts as your user on every team you're in.",
    capabilities: { deploy: true, cancel: true, rollback: true, logs: false, env: true, domains: true },
    account: null,
  },
];

const projects: RemoteProject[] = [
  {
    id: "prj_8sKq2",
    name: "acme-shop",
    url: "https://shop.acme.com",
    framework: "laravel",
    repo: "acme/acme-shop",
    branch: "main",
    dashboard: "https://vercel.com/acme/acme-shop",
  },
  {
    id: "prj_3Hf91",
    name: "acme-blog",
    url: "https://blog.acme.com",
    framework: "nextjs",
    repo: "acme/blog",
    branch: "main",
    dashboard: "https://vercel.com/acme/acme-blog",
  },
];

const deployments: RemoteDeployment[] = [
  {
    id: "dpl_7Qm4",
    state: "building",
    url: "https://acme-shop-7qm4.vercel.app",
    created_at: now - 90_000,
    branch: "main",
    commit: "4be1c02d9a",
    message: "Fix the cart total with coupons",
    production: true,
    current: false,
    dashboard: "https://vercel.com/acme/acme-shop/7Qm4",
  },
  {
    id: "dpl_6Lp1",
    state: "ready",
    url: "https://acme-shop-6lp1.vercel.app",
    created_at: now - 26 * 3600_000,
    branch: "main",
    commit: "91d0aa73f2",
    message: "Checkout: Pix payments",
    production: true,
    current: true,
    dashboard: "https://vercel.com/acme/acme-shop/6Lp1",
  },
  {
    id: "dpl_5Xc8",
    state: "error",
    url: "https://acme-shop-5xc8.vercel.app",
    created_at: now - 3 * 86_400_000,
    branch: "feature/search",
    commit: "c3f8e1b0aa",
    message: "Search with Meilisearch",
    production: false,
    current: false,
    error: 'Command "npm run build" exited with 1',
    dashboard: "https://vercel.com/acme/acme-shop/5Xc8",
  },
];

export function handleRemote(req: RemoteRequest, sites: () => ProjectView[]): RemoteResponse {
  const site = (name: string) => {
    const p = sites().find((s) => s.name === name);
    if (!p) throw new DaemonError("app", `no project named '${name}'`, "NOT_FOUND");
    return p;
  };
  switch (req.type) {
    case "deploy_platforms":
      return { type: "deploy_platforms", platforms: structuredClone(platforms) };
    case "deploy_connect": {
      if (req.token.length < 8) throw new DaemonError("app", "the platform refused the token (401)", "UNAUTHORIZED");
      const p = platforms.find((x) => x.id === req.platform)!;
      p.account = { user: "ana", email: "ana@acme.com", team: req.team ?? null };
      return { type: "deploy_platforms", platforms: structuredClone(platforms) };
    }
    case "deploy_disconnect":
      platforms.find((x) => x.id === req.platform)!.account = null;
      return { type: "deploy_platforms", platforms: structuredClone(platforms) };
    case "deploy_remote_projects":
      if (!platforms.find((x) => x.id === req.platform)?.account)
        throw new DaemonError("app", `${req.platform} isn't connected`, "UNAUTHORIZED");
      return { type: "remote_projects", projects: req.platform === "vercel" ? projects : [] };
    case "deploy_remote_status": {
      const link = site(req.site).deploy;
      if (!link) throw new DaemonError("app", `${req.site} isn't linked to a deploy project`, "NOT_RUNNABLE");
      return {
        type: "remote_status",
        link,
        platform: platforms[0],
        project: projects.find((p) => p.id === link.project_id) ?? projects[0],
        deployments: structuredClone(deployments),
        domains: ["shop.acme.com", "acme-shop.vercel.app"],
        env: [
          { key: "APP_KEY", targets: ["production", "preview"] },
          { key: "DATABASE_URL", targets: ["production"] },
          { key: "STRIPE_SECRET", targets: ["production"] },
        ],
        git: { branch: "main", commit: "4be1c02d9a7e", problems: [] },
        notes: [
          "Changing variables isn't done from LocDev yet: their names are listed here, and the dashboard edits them.",
        ],
      };
    }
    case "deploy_run": {
      const d: RemoteDeployment = {
        id: `dpl_${Math.random().toString(36).slice(2, 6)}`,
        state: "queued",
        created_at: Date.now(),
        branch: "main",
        commit: "4be1c02d9a",
        production: req.production,
        current: false,
        dashboard: "https://vercel.com/acme/acme-shop",
      };
      deployments.unshift(d);
      return { type: "remote_deployment", deployment: d };
    }
    case "deploy_cancel":
      deployments.find((d) => d.id === req.deployment)!.state = "canceled";
      return { type: "ok" } as unknown as RemoteResponse;
    case "deploy_rollback":
      for (const d of deployments) d.current = d.id === req.deployment;
      return { type: "ok" } as unknown as RemoteResponse;
    case "deploy_logs":
      return {
        type: "remote_log",
        lines: [
          "Cloning github.com/acme/acme-shop (Branch: main, Commit: 4be1c02)",
          'Running "vercel build"',
          "Installing dependencies…",
          "added 812 packages in 21s",
          'Running "npm run build"',
        ],
      };
    case "deploy_link_site":
      throw new Error("handled by the main mock");
  }
}
