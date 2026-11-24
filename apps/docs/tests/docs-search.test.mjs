import assert from "node:assert/strict";
import test from "node:test";
import { createDocSearchEntries, normalizeSearch } from "../lib/docs-search.ts";

test("documentation search points directly to localized section anchors", () => {
	const entries = createDocSearchEntries(
		"# Configuração\n## DNS local\n## DNS local\n```bash\n## not a heading\n```",
		{
			slug: "configuration",
			title: "Configuração",
			group: "Referência",
			description: "Arquivos TOML",
		},
	);
	assert.deepEqual(
		entries.map((entry) => entry.href),
		[
			"/docs/configuration",
			"/docs/configuration#dns-local",
			"/docs/configuration#dns-local-1",
		],
	);
	assert.equal(entries[1].page, "Configuração");
});

test("search ignores accents and case while preserving Japanese terms", () => {
	assert.equal(normalizeSearch("CONFIGURAÇÃO"), "configuracao");
	assert.ok(
		normalizeSearch("Configuração de domínios").includes(
			normalizeSearch("DOMINIOS"),
		),
	);
	assert.equal(normalizeSearch("日本語の設定"), "日本語の設定");
});
