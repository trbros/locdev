import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { buildProjectRecipe, quoteArgument } from "../lib/project-recipe.ts";
import {
	assetFormat,
	classifyReleaseAsset,
	detectPlatform,
	matchingAssets,
	normalizeReleases,
	selectRelease,
} from "../lib/releases.ts";
import { repository } from "../lib/repository.ts";
import { readStackLocation, stackLocationHref } from "../lib/stack-links.ts";
import { stacks } from "../lib/stacks.ts";

const base = repository.releases;
const digest = "a".repeat(64);
const asset = (name, id = 1) => ({
	id,
	name,
	state: "uploaded",
	size: 1234,
	digest: `sha256:${digest}`,
	browser_download_url: `${base}/download/v0.1.0/${name}`,
});
const release = (extra = {}) => ({
	id: 1,
	draft: false,
	prerelease: false,
	tag_name: "v0.1.0",
	name: "Foundation",
	published_at: "2026-10-01T10:00:00Z",
	html_url: `${base}/tag/v0.1.0`,
	body: "## Changes\nReal notes",
	assets: [asset("LocDev_0.1.0_amd64.deb"), asset("SHA256SUMS", 2)],
	...extra,
});

test("release parsing excludes drafts and untrusted downloads and preserves published hashes", () => {
	const results = normalizeReleases([
		release(),
		release({ id: 2, draft: true }),
		release({ id: 3, html_url: "javascript:alert(1)" }),
		release({
			id: 4,
			assets: [
				asset("bad.exe", 3),
				{
					...asset("evil.msi", 4),
					browser_download_url: "https://evil.example/install",
				},
			],
		}),
	]);
	assert.equal(results.length, 2);
	assert.equal(results[0].assets[0].sha256, digest);
	assert.equal(results[1].assets.length, 1);
	assert.equal(normalizeReleases([release({ published_at: null })]).length, 0);
	assert.throws(() => normalizeReleases({ message: "not found" }));
	assert.equal(
		normalizeReleases([
			release({ assets: [{ ...asset("x.deb"), digest: "sha256:invalid" }] }),
		])[0].assets[0].sha256,
		null,
	);
});
test("assets match OS, architecture and package without treating signatures as installers", () => {
	assert.deepEqual(
		classifyReleaseAsset("locdev-v0.1.0-aarch64-apple-darwin.tar.gz"),
		{ platform: "macos", architecture: "arm64", kind: "cli" },
	);
	assert.deepEqual(classifyReleaseAsset("LocDev_0.1.0_x64-setup.exe"), {
		platform: "windows",
		architecture: "x64",
		kind: "desktop",
	});
	assert.equal(classifyReleaseAsset("LocDev_amd64.deb.sig").kind, "other");
	// What tauri-action publishes for each system (Tauri 2).
	for (const [name, platform, architecture] of [
		["LocDev_0.18.0_amd64.AppImage", "linux", "x64"],
		["LocDev_0.18.0_amd64.deb", "linux", "x64"],
		["LocDev-0.18.0-1.x86_64.rpm", "linux", "x64"],
		["LocDev_0.18.0_arm64.deb", "linux", "arm64"],
		["LocDev-0.18.0-1.aarch64.rpm", "linux", "arm64"],
		["LocDev_0.18.0_aarch64.AppImage", "linux", "arm64"],
		["LocDev_0.18.0_aarch64.dmg", "macos", "arm64"],
		["LocDev_0.18.0_x64.dmg", "macos", "x64"],
		["LocDev_0.18.0_x64-setup.exe", "windows", "x64"],
		["LocDev_0.18.0_x64_en-US.msi", "windows", "x64"],
	])
		assert.deepEqual(
			classifyReleaseAsset(name),
			{ platform, architecture, kind: "desktop" },
			name,
		);
	// The updater's files are not installers.
	for (const name of [
		"LocDev_aarch64.app.tar.gz",
		"LocDev_aarch64.app.tar.gz.sig",
		"LocDev_0.18.0_x64-setup.exe.sig",
		"latest.json",
		"versions.json",
	])
		assert.equal(classifyReleaseAsset(name).kind, "other", name);
	assert.equal(classifyReleaseAsset("SHA256SUMS").kind, "checksum");
	const data = normalizeReleases([
		release(),
		release({ id: 2, prerelease: true, published_at: "2026-11-01T00:00:00Z" }),
	]);
	assert.equal(selectRelease(data, "stable").id, 1);
	assert.equal(selectRelease(data, "preview").id, 2);
	assert.equal(
		matchingAssets(selectRelease(data, "stable"), "linux", "x64", "desktop")
			.length,
		1,
	);
	assert.equal(
		matchingAssets(selectRelease(data, "stable"), "linux", "arm64", "desktop")
			.length,
		0,
	);
	assert.equal(selectRelease([], "stable"), null);
});
const input = {
	name: "my-app",
	path: "/home/user/Code/it's app",
	command: "npm run dev -- --port {port}",
	port: "8100",
	autoStart: true,
	shell: "posix",
	stack: stacks.find((s) => s.id === "vue"),
};
test("recipe validates input and generates actual registry kinds and CLI flags", () => {
	const result = buildProjectRecipe(input);
	assert.deepEqual(result.errors, []);
	assert.ok(result.cli.includes("--auto-start"));
	assert.ok(result.cli.includes("{port}"));
	assert.ok(result.toml.includes('kind = "node"'));
	assert.ok(result.toml.includes("auto_start = true"));
	const custom = buildProjectRecipe({
		...input,
		stack: stacks.find((s) => s.id === "rails"),
	});
	assert.ok(custom.toml.includes('kind = "unknown"'));
	for (const [key, value, error] of [
		["name", "../bad", "nameError"],
		["path", "relative/path", "pathError"],
		["command", "node\nother", "commandError"],
		["port", "80", "portError"],
		["port", "65536", "portError"],
	]) {
		const invalid = buildProjectRecipe({ ...input, [key]: value });
		assert.ok(invalid.errors.includes(error));
		assert.equal(invalid.cli, "");
		assert.equal(invalid.toml, "");
	}
	const windows = buildProjectRecipe({
		...input,
		shell: "powershell",
		path: "C:\\Code\\my app",
	});
	assert.deepEqual(windows.errors, []);
	assert.ok(windows.toml.includes("C:\\\\Code\\\\my app"));
	assert.equal(quoteArgument("it's", "powershell"), "'it''s'");
});
test("POSIX quoting preserves literals rather than executing shell syntax", {
	skip: process.platform === "win32" && "needs /bin/sh",
}, () => {
	const value = "path's $(printf unexpected); & quoted";
	const result = spawnSync(
		"/bin/sh",
		["-c", `printf '%s' ${quoteArgument(value, "posix")}`],
		{ encoding: "utf8" },
	);
	assert.equal(result.status, 0);
	assert.equal(result.stdout, value);
});
test("stack URLs preserve selection, search and filters and handle malformed values", () => {
	const params = new URLSearchParams("stack=rails&q=Ruby&filter=custom");
	const read = readStackLocation(params);
	assert.equal(stacks[read.index].id, "rails");
	assert.equal(read.open, true);
	assert.equal(read.query, "Ruby");
	assert.equal(read.filter, "custom");
	const href = stackLocationHref("/ja/stacks", params, { workflow: "0" });
	assert.ok(href.startsWith("/ja/stacks?"));
	assert.equal(
		readStackLocation(new URL(href, "https://locdev.test").searchParams).open,
		false,
	);
	const invalid = readStackLocation(
		new URLSearchParams("stack=bogus&filter=bogus"),
	);
	assert.equal(invalid.open, false);
	assert.equal(invalid.index, 0);
	assert.equal(invalid.filter, "all");
});

test("the download page starts on the visitor's own system", () => {
	const mac = detectPlatform(
		"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
	);
	assert.equal(mac.platform, "macos");
	assert.deepEqual(
		detectPlatform(
			"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
		),
		{ platform: "windows", architecture: "x64" },
	);
	assert.deepEqual(
		detectPlatform("Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36"),
		{ platform: "linux", architecture: "arm64" },
	);
	assert.equal(
		detectPlatform("Mozilla/5.0 (Linux; Android 14; Pixel 8)").platform,
		null,
	);
	assert.equal(
		detectPlatform("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)")
			.platform,
		null,
	);
	assert.equal(
		assetFormat("LocDev_0.18.0_amd64.deb"),
		"Debian / Ubuntu (.deb)",
	);
	assert.equal(assetFormat("LocDev_0.18.0_x64-setup.exe"), "Setup (.exe)");
});
