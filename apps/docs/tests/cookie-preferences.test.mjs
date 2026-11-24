import assert from "node:assert/strict";
import test from "node:test";
import { parseCookiePreferences } from "../components/cookie-consent/preferences.ts";

test("only an explicit, current preference record is recognized", () => {
	for (const raw of [
		null,
		"1",
		"dismissed",
		"{}",
		"null",
		"[]",
		"bad JSON",
		JSON.stringify({
			version: 2,
			necessary: true,
			remember: true,
			updatedAt: 1,
		}),
		JSON.stringify({
			version: 1,
			necessary: false,
			remember: true,
			updatedAt: 1,
		}),
		JSON.stringify({
			version: 1,
			necessary: true,
			remember: "yes",
			updatedAt: 1,
		}),
		JSON.stringify({
			version: 1,
			necessary: true,
			remember: true,
			updatedAt: 0,
		}),
	])
		assert.equal(parseCookiePreferences(raw), null);
});

test("persistent and tab-only choices round-trip without retaining unrelated input", () => {
	for (const remember of [true, false])
		for (const analytics of [true, false]) {
			const preference = {
				version: 1,
				necessary: true,
				remember,
				analytics,
				updatedAt: 1730000000000,
			};
			assert.deepEqual(
				parseCookiePreferences(
					JSON.stringify({ ...preference, unrelated: "ignored" }),
				),
				preference,
			);
		}
});

test("a choice saved before analytics existed never grants it", () => {
	const old = { version: 1, necessary: true, remember: true, updatedAt: 1 };
	assert.equal(parseCookiePreferences(JSON.stringify(old)).analytics, false);
	assert.equal(
		parseCookiePreferences(JSON.stringify({ ...old, analytics: "yes" }))
			.analytics,
		false,
	);
});
