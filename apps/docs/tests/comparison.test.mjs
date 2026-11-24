import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import {
	groups,
	reviewedOn,
	statusLabels,
	tools,
} from "../lib/comparison-matrix.ts";
import { verifyComparisonEvidence } from "../scripts/check-comparison.mjs";

const locales = ["br", "en", "es", "ja"];
const messages = Object.fromEntries(
	locales.map((locale) => [
		locale,
		JSON.parse(
			readFileSync(
				new URL(`../messages/${locale}.json`, import.meta.url),
				"utf8",
			),
		).ComparisonMatrix,
	]),
);

test("comparison rows cover every tool with a known state and a source", () => {
	assert.deepEqual(
		tools.map((t) => t.id),
		[
			"locdev",
			"herd",
			"yerd",
			"lerd",
			"laragon",
			"ddev",
			"lando",
			"sail",
			"valet",
			"compose",
		],
	);
	assert.match(reviewedOn, /^\d{4}-\d{2}-\d{2}$/);
	for (const group of groups)
		for (const feature of group.features)
			for (const tool of tools) {
				const cell = feature.cells[tool.id];
				assert.ok(cell.status in statusLabels, `${feature.id}/${tool.id}`);
				assert.match(
					cell.source ?? "",
					/^https:\/\//,
					`${feature.id}/${tool.id} has no source`,
				);
			}
});

test("every group, feature and note is translated in all four languages", () => {
	for (const locale of locales) {
		const m = messages[locale];
		for (const status of Object.keys(statusLabels))
			assert.ok(m.status[status], `${locale}: status.${status}`);
		for (const group of groups) {
			assert.ok(m.groups[group.id]?.title, `${locale}: groups.${group.id}`);
			for (const feature of group.features) {
				assert.ok(
					m.features[feature.id]?.label,
					`${locale}: features.${feature.id}`,
				);
				for (const tool of tools)
					if (feature.cells[tool.id].note)
						assert.ok(
							m.notes[group.id]?.[feature.id]?.[tool.id],
							`${locale}: notes.${group.id}.${feature.id}.${tool.id}`,
						);
			}
		}
	}
});

test("the LocDev column matches what the repository ships", () => {
	assert.deepEqual(
		verifyComparisonEvidence(resolve(import.meta.dirname, "../../..")),
		[],
	);
});

test("Windows support names what was validated there", () => {
	// Validated on Windows 11 (2026-11-23); the note says with what.
	const windows = groups
		.flatMap((g) => g.features)
		.find((f) => f.id === "windows");
	assert.equal(windows.cells.locdev.status, "yes");
	assert.match(windows.cells.locdev.note, /Windows 11/);
});

test("regenerating service catalogs preserves surrounding manual content", async () => {
	const { replacePromptServices } = await import(
		"../scripts/replace-prompt-section.mjs"
	);
	const start = "<!-- planned-stack-services:start -->";
	const end = "<!-- planned-stack-services:end -->";
	const prompt = `Manual introduction\n${start}\nOld services\n${end}\nManual comparison instructions\n`;
	const suffix = prompt.slice(prompt.indexOf(end) + end.length);
	const updated = replacePromptServices(
		prompt,
		`${start}\nNew generated services\n${end}`,
	);
	assert.equal(updated.slice(updated.indexOf(end) + end.length), suffix);
	assert.equal(
		updated.slice(0, updated.indexOf(start)),
		prompt.slice(0, prompt.indexOf(start)),
	);
	assert.throws(
		() =>
			replacePromptServices(
				`Manual notes\n${start}\nunfinished`,
				"replacement",
			),
		/refusing to overwrite/,
	);
});

// New content and mobile controls must ship in every supported language.
test("comparison advantages, requirements and mobile controls are localized", () => {
	const expected = Object.keys(messages.en.why);
	for (const locale of locales) {
		const m = messages[locale];
		assert.deepEqual(Object.keys(m.why).sort(), [...expected].sort());
		for (const key of expected)
			assert.ok(m.why[key]?.trim(), `${locale}: why.${key}`);
		assert.ok(m.ui.compareWith?.trim());
		assert.ok(m.ui.mobileHint?.trim());
		assert.ok(m.page.statsFeatures?.trim());
	}
});
