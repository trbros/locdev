import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const locales = ["br", "en", "es", "ja"];
const flatten = (node, prefix = "") =>
	Object.entries(node).flatMap(([key, value]) =>
		typeof value === "string"
			? [`${prefix}${key}`]
			: flatten(value, `${prefix}${key}.`),
	);

for (const locale of locales) {
	test(`${locale}: contains the same translation keys as br`, () => {
		const base = JSON.parse(
			readFileSync(resolve(root, "messages/br.json"), "utf8"),
		);
		const translated = JSON.parse(
			readFileSync(resolve(root, `messages/${locale}.json`), "utf8"),
		);
		assert.deepEqual(flatten(translated).sort(), flatten(base).sort());
		assert.ok(flatten(translated).length > 75);
	});
}

test("all locales expose real platform and development messages", () => {
	for (const locale of locales) {
		const msgs = JSON.parse(
			readFileSync(resolve(root, `messages/${locale}.json`), "utf8"),
		);
		assert.ok(msgs.Hero.titlePrimary.length > 6);
		assert.ok(msgs.Nav.download.length > 2);
		assert.ok(msgs.Download.developmentNotice.length > 15);
	}
});
