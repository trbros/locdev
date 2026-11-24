/** Roadmap read from trbros/locdev/apps/docs/src/content/roadmap.mdx on 2026-11-01.
 * This is editorial content, not an automated release feed.
 */
export const milestones = [
	{ version: "0.1", key: "foundation", status: "active" },
	{ version: "0.2", key: "runtimes", status: "active" },
	{ version: "0.3", key: "services", status: "planned" },
	{ version: "0.4", key: "tools", status: "planned" },
	{ version: "0.5", key: "workspaces", status: "planned" },
	{ version: "0.6", key: "ecosystem", status: "planned" },
	{ version: "1.0", key: "stable", status: "planned" },
] as const;

export { githubReleases, githubRepo } from "./repository.ts";

/** Where voluntary financial support goes (also in .github/FUNDING.yml). */
export const sponsors = {
	github: "https://github.com/sponsors/bnacci",
	coffee: "https://buymeacoffee.com/bnacci",
} as const;
