import assert from "node:assert/strict";
import test from "node:test";
import { loadReleaseFeed } from "../lib/releases.ts";
import { repository } from "../lib/repository.ts";

test("GitHub feed degrades safely for no releases, HTTP failures, malformed data and network errors", async () => {
	for (const [status, reason] of [
		[404, "not-public"],
		[403, "rate-limit"],
		[429, "rate-limit"],
		[500, "network"],
	]) {
		const feed = await loadReleaseFeed(async () => ({
			ok: false,
			status,
			json: async () => [],
		}));
		assert.equal(feed.status, "unavailable");
		assert.equal(feed.reason, reason);
		assert.deepEqual(feed.releases, []);
	}
	const empty = await loadReleaseFeed(async () => ({
		ok: true,
		status: 200,
		json: async () => [],
	}));
	assert.equal(empty.status, "empty");
	const offline = await loadReleaseFeed(async () => {
		throw new Error("offline");
	});
	assert.equal(offline.status, "unavailable");
	const invalid = await loadReleaseFeed(async () => ({
		ok: true,
		status: 200,
		json: async () => ({ message: "invalid" }),
	}));
	assert.equal(invalid.status, "unavailable");
	const published = await loadReleaseFeed(async () => ({
		ok: true,
		status: 200,
		json: async () => [
			{
				id: 3,
				draft: false,
				prerelease: true,
				tag_name: "v0.1.0-beta",
				published_at: "2026-10-01T00:00:00Z",
				html_url: `${repository.releases}/tag/v0.1.0-beta`,
				assets: [],
			},
		],
	}));
	assert.equal(published.status, "ok");
	assert.equal(published.releases[0].prerelease, true);
});
