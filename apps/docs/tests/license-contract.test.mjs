import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parseLicenseDocument } from "../lib/license-document.ts";

// The unmodified plain text published at https://www.apache.org/licenses/LICENSE-2.0.txt.
test("the downloadable Apache-2.0 license preserves the complete official text", () => {
	const text = readFileSync(
		new URL("../public/licenses/Apache-2.0.txt", import.meta.url),
	);
	assert.equal(
		createHash("sha256").update(text).digest("hex"),
		"cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
	);
});

test("the reader sections preserve every byte of the official license", () => {
	const text = readFileSync(
		new URL("../public/licenses/Apache-2.0.txt", import.meta.url),
		"utf8",
	);
	const parsed = parseLicenseDocument(text);
	assert.equal(parsed.sections.length, 10);
	assert.equal(
		parsed.introduction +
			parsed.sections.map((section) => section.raw).join(""),
		text,
	);
	assert.equal(parsed.sections[3].title, "4. Redistribution");
	assert.equal(new Set(parsed.sections.map((section) => section.id)).size, 10);
});
