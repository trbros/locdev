import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { fileURLToPath } from "node:url";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const changelog = read("../../../CHANGELOG.md");

const suffixes = { en: "", "pt-BR": ".pt-BR", es: ".es", ja: ".ja" };

test("site and desktop changelogs match the repository in all four languages", () => {
	for (const suffix of Object.values(suffixes)) {
		const source = read(`../../../CHANGELOG${suffix}.md`);
		assert.equal(read(`../generated/changelog${suffix}.md`), source);
		assert.equal(
			read(`../../desktop/src/generated/changelog${suffix}.md`),
			source,
		);
		assert.deepEqual(
			[...source.matchAll(/^## \[.+$/gm)].map((match) => match[0]),
			[...changelog.matchAll(/^## \[.+$/gm)].map((match) => match[0]),
			"translations must preserve release versions, dates and status markers",
		);
		assert.equal(
			(source.match(/^- /gm) ?? []).length,
			(changelog.match(/^- /gm) ?? []).length,
		);
	}
});

test("changelog loader returns the requested language and safely falls back to English", () => {
	const output = execFileSync(
		process.execPath,
		[
			"--conditions=react-server",
			"--input-type=module",
			"-e",
			`import {getChangelog} from './lib/changelog.ts';
		const result = {};
		for (const locale of ['en', 'pt-BR', 'es', 'ja', '../../LICENSE']) result[locale] = await getChangelog(locale);
		process.stdout.write(JSON.stringify(result));`,
		],
		{ cwd: fileURLToPath(new URL("../", import.meta.url)), encoding: "utf8" },
	);
	const entries = JSON.parse(output);
	for (const [locale, suffix] of Object.entries(suffixes)) {
		const source = read(`../../../CHANGELOG${suffix}.md`);
		assert.equal(
			entries[locale][0].body,
			source.split(/^## /m)[1].split("\n").slice(1).join("\n").trim(),
		);
	}
	assert.deepEqual(entries["../../LICENSE"], entries.en);
});

test("the current app version has an entry naming the daemon's", () => {
	const daemon = read("../../../Cargo.toml").match(
		/\[workspace\.package\][^[]*?version = "([^"]+)"/,
	)?.[1];
	const app = JSON.parse(read("../../desktop/package.json")).version;
	assert.ok(daemon && app, "version not found");
	const entry = changelog
		.split(/^## /m)
		.find((chunk) => chunk.startsWith(`[${app}]`));
	assert.ok(entry, `no entry for app ${app}`);
	assert.ok(entry.includes(`daemon and CLI ${daemon}`), entry.slice(0, 120));
});

test("the changelog starts at the initial release", () => {
	// Everything before it was internal testing, folded into one entry.
	const versions = [...changelog.matchAll(/^## \[(\d[^\]]*)\]/gm)].map(
		(m) => m[1],
	);
	assert.equal(versions.at(-1), "0.19.0");
	assert.match(changelog, /^Initial release\./m);
});
