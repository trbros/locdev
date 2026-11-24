import "server-only";
import { unstable_cache } from "next/cache";
import { cache } from "react";
import { loadReleaseFeed } from "./releases.ts";
import { repository } from "./repository.ts";

/** Cache failures too, so unavailable repositories do not exhaust the public API quota. */
const getCachedReleaseFeed = unstable_cache(
	() =>
		loadReleaseFeed(() =>
			fetch(repository.api, {
				headers: {
					Accept: "application/vnd.github+json",
					"X-GitHub-Api-Version": "2026-03-10",
				},
				next: { revalidate: 900 },
				signal: AbortSignal.timeout(8000),
			}),
		),
	["locdev-public-releases-v2", repository.url],
	{ revalidate: 300 },
);
/** Public, published releases only. No credentials or drafts enter the public site. */
export const getReleaseFeed = cache(getCachedReleaseFeed);
