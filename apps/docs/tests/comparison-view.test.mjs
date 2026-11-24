import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { groups, tools } from "../lib/comparison-matrix.ts";
import {
	brandImages,
	competitors,
	isLocdevAdvantage,
	matchesComparisonView,
	readComparisonLocation,
} from "../lib/comparison-view.ts";

test("comparison starts with Herd and all features, invalid links recover safely", () => {
	for (const search of [
		"",
		"app=locdev&view=bad&category=bad",
		"app=unrecognized",
	]) {
		const state = readComparisonLocation(new URLSearchParams(search));
		assert.equal(state.competitor.id, "herd");
		assert.equal(state.view, "all");
		assert.equal(state.category, "all");
	}
});

test("a shared URL restores the selected app, category, view and search", () => {
	const state = readComparisonLocation(
		new URLSearchParams("app=ddev&category=advanced&view=advantages&q=GitHub"),
	);
	assert.equal(state.competitor.id, "ddev");
	assert.equal(state.category, "advanced");
	assert.equal(state.view, "advantages");
	assert.equal(state.query, "GitHub");
	assert.equal(
		readComparisonLocation(new URLSearchParams({ q: "a".repeat(200) })).query
			.length,
		160,
	);
});

test("unverified competition is never counted as a LocDev advantage or a confirmed difference", () => {
	const github = groups
		.flatMap((g) => g.features)
		.find((f) => f.id === "github");
	assert.equal(github.cells.herd.status, "unknown");
	assert.equal(isLocdevAdvantage(github, "herd"), false);
	assert.equal(matchesComparisonView(github, "herd", "differences"), false);
	assert.equal(matchesComparisonView(github, "herd", "all"), true);
});

test("verified differences include both advantages and limitations", () => {
	const all = groups.flatMap((g) => g.features);
	const native = all.find((f) => f.id === "native");
	assert.equal(isLocdevAdvantage(native, "ddev"), true);
	assert.equal(matchesComparisonView(native, "ddev", "differences"), true);
	// A row where a competitor has more than LocDev is a difference but not
	// an advantage.
	const lagging = {
		...native,
		cells: {
			...native.cells,
			locdev: { status: "partial" },
			herd: { status: "yes" },
		},
	};
	assert.equal(matchesComparisonView(lagging, "herd", "differences"), true);
	assert.equal(matchesComparisonView(lagging, "herd", "advantages"), false);
});

test("every alternative is selectable and every tool has a locally stored brand image", () => {
	assert.equal(competitors.length, tools.length - 1);
	for (const tool of competitors)
		assert.equal(
			readComparisonLocation(new URLSearchParams({ app: tool.id })).competitor
				.id,
			tool.id,
		);
	for (const tool of tools)
		assert.ok(
			existsSync(new URL(`../public${brandImages[tool.id]}`, import.meta.url)),
			tool.id,
		);
});

test("every new comparator control and tool description is translated", () => {
	const catalogs = ["br", "en", "es", "ja"].map(
		(locale) =>
			JSON.parse(
				readFileSync(
					new URL(`../messages/${locale}.json`, import.meta.url),
					"utf8",
				),
			).ComparisonMatrix.experience,
	);
	function check(reference, translated) {
		assert.deepEqual(
			Object.keys(translated).sort(),
			Object.keys(reference).sort(),
		);
		for (const [key, value] of Object.entries(reference)) {
			if (typeof value === "object") check(value, translated[key]);
			else {
				assert.ok(translated[key]?.trim());
				const params = (text) =>
					[...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
				assert.deepEqual(params(translated[key]), params(value), key);
			}
		}
	}
	for (const catalog of catalogs) check(catalogs[1], catalog);
});
