import { repository } from "./repository.ts";
export type ReleasePlatform = "linux" | "macos" | "windows";
export type ReleaseArchitecture = "x64" | "arm64";
export type ReleaseAsset = {
	id: number;
	name: string;
	url: string;
	size: number;
	sha256: string | null;
	platform: ReleasePlatform | null;
	architecture: ReleaseArchitecture | "universal" | null;
	kind: "desktop" | "cli" | "checksum" | "other";
};
export type AppRelease = {
	id: number;
	tag: string;
	name: string;
	url: string;
	publishedAt: string;
	prerelease: boolean;
	notes: string;
	assets: ReleaseAsset[];
};
export type ReleaseFeed = {
	status: "ok" | "empty" | "unavailable";
	reason?: "not-public" | "rate-limit" | "network";
	releases: AppRelease[];
	checkedAt: string;
};
const repositoryPath = repository.releasePath.toLowerCase();
function record(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null;
}
function releaseUrl(value: unknown, download: boolean): string | null {
	if (typeof value !== "string") return null;
	try {
		const url = new URL(value);
		return url.protocol === "https:" &&
			url.hostname === "github.com" &&
			!url.username &&
			!url.password &&
			url.pathname
				.toLowerCase()
				.startsWith(`${repositoryPath}${download ? "download/" : "tag/"}`)
			? url.href
			: null;
	} catch {
		return null;
	}
}
export function classifyReleaseAsset(
	name: string,
): Pick<ReleaseAsset, "platform" | "architecture" | "kind"> {
	const value = name.toLowerCase();
	if (/^(sha256sums(?:\.txt)?|checksums?(?:\.txt)?|.*\.sha256)$/.test(value))
		return { platform: null, architecture: null, kind: "checksum" };
	const platform = /windows|win32|win64|pc-windows|\.msi$|\.exe$/.test(value)
		? "windows"
		: /darwin|macos|apple|\.dmg$|\.app\.tar\.gz$/.test(value)
			? "macos"
			: /linux|\.deb$|\.rpm$|\.appimage$/.test(value)
				? "linux"
				: null;
	const architecture = /universal/.test(value)
		? "universal"
		: /aarch64|arm64/.test(value)
			? "arm64"
			: /x86_64|amd64|x64/.test(value)
				? "x64"
				: null;
	// `.app.tar.gz` is the macOS updater's bundle (installed apps fetch it
	// through latest.json); people download the .dmg.
	const kind = /\.(deb|rpm|appimage|dmg|msi|exe)$/.test(value)
		? "desktop"
		: /^locdev-v?.*(linux|darwin|windows).*(\.tar\.gz|\.zip)$/.test(value)
			? "cli"
			: "other";
	return { platform, architecture, kind };
}
export function normalizeReleases(value: unknown): AppRelease[] {
	if (!Array.isArray(value)) throw new Error("Invalid releases response");
	return value
		.flatMap((item): AppRelease[] => {
			if (
				!record(item) ||
				item.draft !== false ||
				typeof item.id !== "number" ||
				typeof item.tag_name !== "string" ||
				typeof item.published_at !== "string" ||
				!Number.isFinite(Date.parse(item.published_at))
			)
				return [];
			const url = releaseUrl(item.html_url, false);
			if (!url) return [];
			const assets = (Array.isArray(item.assets) ? item.assets : []).flatMap(
				(asset): ReleaseAsset[] => {
					if (
						!record(asset) ||
						typeof asset.id !== "number" ||
						typeof asset.name !== "string" ||
						asset.state !== "uploaded"
					)
						return [];
					const download = releaseUrl(asset.browser_download_url, true);
					if (!download) return [];
					const digest =
						typeof asset.digest === "string" &&
						/^sha256:[a-f0-9]{64}$/i.test(asset.digest)
							? asset.digest.slice(7).toLowerCase()
							: null;
					return [
						{
							id: asset.id,
							name: asset.name,
							url: download,
							size:
								typeof asset.size === "number" && asset.size >= 0
									? asset.size
									: 0,
							sha256: digest,
							...classifyReleaseAsset(asset.name),
						},
					];
				},
			);
			return [
				{
					id: item.id,
					tag: item.tag_name,
					name:
						typeof item.name === "string" && item.name
							? item.name
							: item.tag_name,
					url,
					publishedAt: item.published_at,
					prerelease: item.prerelease === true,
					notes: typeof item.body === "string" ? item.body : "",
					assets,
				},
			];
		})
		.sort((a, b) => Date.parse(b.publishedAt) - Date.parse(a.publishedAt));
}
/** The visitor's system and processor, from the browser (null when unclear). */
export function detectPlatform(
	userAgent: string,
	platformHint = "",
): {
	platform: ReleasePlatform | null;
	architecture: ReleaseArchitecture | null;
} {
	const ua = `${platformHint} ${userAgent}`.toLowerCase();
	const platform = /windows|win32|win64/.test(ua)
		? "windows"
		: /mac os|macintosh|macos|mac_powerpc/.test(ua) && !/iphone|ipad/.test(ua)
			? "macos"
			: /linux|x11|cros/.test(ua) && !/android/.test(ua)
				? "linux"
				: null;
	const architecture = /arm64|aarch64|armv8/.test(ua)
		? "arm64"
		: /x86_64|x64|win64|wow64|amd64|intel/.test(ua)
			? "x64"
			: null;
	return { platform, architecture };
}

/** What an installer is, for people choosing between several. */
export function assetFormat(name: string): string {
	const value = name.toLowerCase();
	if (value.endsWith(".dmg")) return "DMG";
	if (value.endsWith("-setup.exe") || value.endsWith(".exe"))
		return "Setup (.exe)";
	if (value.endsWith(".msi")) return "MSI";
	if (value.endsWith(".deb")) return "Debian / Ubuntu (.deb)";
	if (value.endsWith(".rpm")) return "Fedora / openSUSE (.rpm)";
	if (value.endsWith(".appimage")) return "AppImage";
	if (value.endsWith(".zip")) return "ZIP";
	if (value.endsWith(".tar.gz")) return "tar.gz";
	return name.split(".").pop() ?? name;
}

export function selectRelease(
	releases: AppRelease[],
	channel: "stable" | "preview",
) {
	return (
		releases.find(
			(release) => release.prerelease === (channel === "preview"),
		) ?? null
	);
}
export function matchingAssets(
	release: AppRelease | null,
	platform: ReleasePlatform,
	architecture: ReleaseArchitecture,
	kind: "desktop" | "cli",
) {
	return (
		release?.assets.filter(
			(asset) =>
				asset.kind === kind &&
				asset.platform === platform &&
				(asset.architecture === architecture ||
					asset.architecture === "universal"),
		) ?? []
	);
}

/** Separates the public HTTP boundary from framework-specific caching. */
export async function loadReleaseFeed(
	request: () => Promise<{
		ok: boolean;
		status: number;
		json: () => Promise<unknown>;
	}>,
	checkedAt = new Date().toISOString(),
): Promise<ReleaseFeed> {
	try {
		const response = await request();
		if (!response.ok)
			return {
				status: "unavailable",
				reason:
					response.status === 404
						? "not-public"
						: [403, 429].includes(response.status)
							? "rate-limit"
							: "network",
				releases: [],
				checkedAt,
			};
		const releases = normalizeReleases(await response.json());
		return { status: releases.length ? "ok" : "empty", releases, checkedAt };
	} catch {
		return {
			status: "unavailable",
			reason: "network",
			releases: [],
			checkedAt,
		};
	}
}
