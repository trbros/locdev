import assert from "node:assert/strict";
import test from "node:test";
import { faqMatches } from "../lib/faq-search.ts";

test("FAQ search matches question and answer terms regardless of accents or case", () => {
	assert.equal(
		faqMatches(
			"Instalação",
			{ p1: "Execute locdev doctor", p2: "Verifique o ambiente" },
			"INSTALACAO doctor",
		),
		true,
	);
	assert.equal(
		faqMatches("Instalação", { p1: "Use HTTPS" }, "instalacao banco"),
		false,
	);
	assert.equal(
		faqMatches(
			"Offline?",
			"Local projects do not require a permanent connection.",
			"projects connection",
		),
		true,
	);
	assert.equal(faqMatches("HTTPS", "Certificados locais", "  "), true);
});
test("FAQ search supports Japanese and safely handles missing answers", () => {
	assert.equal(
		faqMatches("インストール", { p1: "CLI を使用できます。" }, "CLI"),
		true,
	);
	assert.equal(faqMatches("インストール", null, "インストール"), true);
	assert.equal(faqMatches("Question", null, "missing"), false);
});
