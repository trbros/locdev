// Parked folders for the mock backend. Discovery, watching and naming live
// in the daemon (`daemon/park.rs`, `locdev-frameworks::discover`).

import { DaemonError } from "./errors";
import type { ParkedView, ParkRequest, ParkResponse, ProjectView } from "./protocol";

const now = Date.now();

const dirs: ParkedView[] = [
  {
    path: "~/Code",
    enabled: true,
    depth: 1,
    exclude: ["archive-*"],
    ignored: ["old-crm"],
    on_new: "suggest",
    on_missing: "unregister",
    exists: true,
    watching: true,
    last_scan: now - 90_000,
    projects: [
      {
        path: "~/Code/acme-shop",
        rel: "acme-shop",
        name: "acme-shop",
        kind: "laravel",
        framework: "Laravel",
        state: "manual",
      },
      { path: "~/Code/blog", rel: "blog", name: "blog", kind: "next", framework: "Next.js", state: "manual" },
      { path: "~/Code/docs", rel: "docs", name: "docs", kind: "static", framework: "HTML5", state: "manual" },
      {
        path: "~/Code/landing",
        rel: "landing",
        name: "landing",
        kind: "vite",
        framework: "React + Vite",
        state: "manual",
      },
      { path: "~/Code/old-crm", rel: "old-crm", name: "old-crm", kind: "php", framework: "PHP", state: "ignored" },
      {
        path: "~/Code/playground",
        rel: "playground",
        name: "playground",
        kind: "sveltekit",
        framework: "SvelteKit",
        state: "suggested",
      },
    ],
    errors: [],
  },
  {
    path: "~/Sites",
    enabled: true,
    depth: 2,
    on_new: "register",
    on_missing: "unregister",
    exists: true,
    watching: true,
    last_scan: now - 20_000,
    projects: [
      {
        path: "~/Sites/wp-agency",
        rel: "wp-agency",
        name: "wp-agency",
        kind: "wordpress",
        framework: "WordPress",
        state: "registered",
      },
      {
        path: "~/Sites/clients/docs",
        rel: "clients/docs",
        name: "docs-clients",
        kind: "astro",
        framework: "Astro",
        state: "registered",
        note: "'docs' was already taken",
      },
    ],
    errors: [{ path: "~/Sites/private", message: "Permission denied (os error 13)" }],
  },
];

function reply(added: string[] = [], removed: string[] = []): ParkResponse {
  return { type: "parked", dirs: structuredClone(dirs), added, removed };
}

function find(path: string): ParkedView {
  const dir = dirs.find((d) => d.path === path);
  if (!dir) throw new DaemonError("app", `${path} is not a parked folder`, "NOT_FOUND");
  return dir;
}

export function handlePark(
  req: ParkRequest,
  projects: () => ProjectView[],
  removeSite: (name: string) => void,
): ParkResponse {
  // Projects added by hand since the last list show up as sites.
  for (const dir of dirs) {
    for (const p of dir.projects) {
      if (p.state === "suggested" && projects().some((s) => s.path === p.path)) p.state = "manual";
    }
  }
  switch (req.type) {
    case "park_list":
      return reply();
    case "park_set": {
      const existing = dirs.find((d) => d.path === req.path);
      const options = req.options ?? {};
      if (options.depth !== undefined && (options.depth < 1 || options.depth > 3)) {
        throw new DaemonError("app", "depth must be between 1 and 3", "INVALID_INPUT");
      }
      if (!existing) {
        dirs.push({
          path: req.path,
          enabled: true,
          depth: 1,
          on_new: "register",
          on_missing: "unregister",
          exists: true,
          watching: true,
          last_scan: Date.now(),
          projects: [],
          errors: [],
        });
      }
      const dir = find(req.path);
      Object.assign(dir, Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)));
      dir.watching = dir.enabled;
      dir.last_scan = Date.now();
      return reply();
    }
    case "park_remove": {
      const dir = find(req.path);
      const removed: string[] = [];
      if (!req.keep_sites) {
        for (const p of dir.projects.filter((p) => p.state === "registered")) {
          removeSite(p.name);
          removed.push(p.name);
        }
      }
      dirs.splice(dirs.indexOf(dir), 1);
      return reply([], removed);
    }
    case "park_scan": {
      for (const dir of req.path ? [find(req.path)] : dirs.filter((d) => d.enabled)) dir.last_scan = Date.now();
      return reply();
    }
    case "park_ignore": {
      const dir = dirs.find((d) => req.path.startsWith(`${d.path}/`));
      const project = dir?.projects.find((p) => p.path === req.path);
      if (!dir || !project) throw new DaemonError("app", `${req.path} is not inside a parked folder`, "NOT_FOUND");
      const removed: string[] = [];
      if (req.ignore) {
        if (project.state === "registered") {
          removeSite(project.name);
          removed.push(project.name);
        }
        project.state = "ignored";
        dir.ignored = [...(dir.ignored ?? []), project.rel];
      } else {
        project.state = dir.on_new === "register" ? "registered" : "suggested";
        dir.ignored = (dir.ignored ?? []).filter((r) => r !== project.rel);
      }
      return reply([], removed);
    }
  }
}
