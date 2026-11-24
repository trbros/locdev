/** Public GitHub repository configuration shared by the website and its scripts. */
export function parseGitHubRepository(value: string | undefined) {
	if (!value?.trim())
		throw new Error(
			"Set NEXT_PUBLIC_GITHUB_REPOSITORY_URL in .env before starting or building the site.",
		);
	const url = new URL(value.trim());
	const path = url.pathname.replace(/\/+$/, "").replace(/\.git$/, "");
	const parts = path.split("/").filter(Boolean);
	if (
		url.protocol !== "https:" ||
		url.hostname !== "github.com" ||
		url.port ||
		url.username ||
		url.password ||
		url.search ||
		url.hash ||
		parts.length !== 2 ||
		parts.some(
			(part) =>
				!/^[A-Za-z0-9_.-]+$/.test(part) || part === "." || part === "..",
		) ||
		path !== `/${parts.join("/")}`
	) {
		throw new Error(
			"NEXT_PUBLIC_GITHUB_REPOSITORY_URL must be an HTTPS GitHub repository URL, without credentials, query parameters or a fragment.",
		);
	}
	const [owner, name] = parts;
	const repo = `https://github.com/${owner}/${name}`;
	return {
		url: repo,
		slug: `${owner}/${name}`,
		name,
		issues: `${repo}/issues/new`,
		pulls: `${repo}/pulls`,
		releases: `${repo}/releases`,
		clone: `${repo}.git`,
		api: `https://api.github.com/repos/${owner}/${name}/releases?per_page=30`,
		releasePath: `/${owner}/${name}/releases/`,
	};
}
// Static property access is required for Next.js to inline the public setting.
export const repository = parseGitHubRepository(
	process.env.NEXT_PUBLIC_GITHUB_REPOSITORY_URL,
);
export const githubRepo = repository.url;
export const githubReleases = repository.releases;
