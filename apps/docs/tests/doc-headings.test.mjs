import assert from "node:assert/strict";
import test from "node:test";
import { getDocHeadings } from "../lib/doc-headings.ts";

test("article anchors support localized text, duplicates and code examples", () => {
	const headings = getDocHeadings(
		"# Title\n## Configuração\n### `Node.js`\n## Configuração\n```md\n## Example heading\n```\n## 日本語の設定",
	);
	assert.deepEqual(
		headings.map(({ id }) => id),
		["configuracao", "nodejs", "configuracao-1", "日本語の設定"],
	);
	assert.equal(headings[1].level, 3);
	assert.equal(headings[1].title, "Node.js");
});
