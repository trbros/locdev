import assert from "node:assert/strict";
import test from "node:test";
import { normalizeReleases } from "../lib/releases.ts";
import { parseGitHubRepository, repository } from "../lib/repository.ts";

test("repository configuration derives every endpoint from a different owner/name", () => {
	const config = parseGitHubRepository(
		" https://github.com/New-Org/new-app.git/ ",
	);
	assert.equal(config.url, "https://github.com/New-Org/new-app");
	assert.equal(config.slug, "New-Org/new-app");
	assert.equal(config.clone, `${config.url}.git`);
	assert.equal(config.issues, `${config.url}/issues/new`);
	assert.equal(config.pulls, `${config.url}/pulls`);
	assert.equal(config.releases, `${config.url}/releases`);
	assert.equal(
		config.api,
		"https://api.github.com/repos/New-Org/new-app/releases?per_page=30",
	);
});
test("repository configuration rejects credentials, non-GitHub hosts and nested destinations", () => {
	for (const value of [
		undefined,
		"",
		"not a URL",
		"http://github.com/org/repo",
		"https://github.com.evil.test/org/repo",
		"https://user:token@github.com/org/repo",
		"https://github.com/org/repo/issues",
		"https://github.com/org/repo?token=secret",
		"https://github.com/org/repo#readme",
		"https://github.com/org/repo%20name",
		"https://github.com/org//repo",
	]) {
		assert.throws(() => parseGitHubRepository(value), String(value));
	}
});
test("published releases trust only the configured repository and its asset paths", () => {
	const release = (url) => ({
		id: 1,
		tag_name: "v1",
		draft: false,
		published_at: "2026-11-01T00:00:00Z",
		html_url: url,
		assets: [],
	});
	assert.equal(
		normalizeReleases([release(`${repository.releases}/tag/v1`)]).length,
		1,
	);
	assert.equal(
		normalizeReleases([
			release(
				"https://github.com/unrelated-org/unrelated-repo/releases/tag/v1",
			),
		]).length,
		0,
	);
});
