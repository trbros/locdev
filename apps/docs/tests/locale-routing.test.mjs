import assert from "node:assert/strict";
import test from "node:test";
import {
	languageTags,
	localeFileKey,
	localizedPathname,
} from "../i18n/locales.ts";
import { canonicalRedirect } from "../i18n/redirects.ts";

test("every language, Brazilian Portuguese included, has its own URL prefix", () => {
	for (const locale of ["pt-BR", "en", "es", "ja"]) {
		assert.equal(localizedPathname("/", locale), `/${locale}`);
		assert.equal(localizedPathname("/donate", locale), `/${locale}/donate`);
		assert.equal(
			localizedPathname("/docs/getting-started", locale),
			`/${locale}/docs/getting-started`,
		);
	}
	assert.equal(localeFileKey("pt-BR"), "br");
	assert.equal(languageTags["pt-BR"], "pt-BR");
});

test("old URLs redirect permanently to their prefixed pages", () => {
	for (const [path, canonical] of [
		["/pt", "/pt-BR"],
		["/br", "/pt-BR"],
		["/br/donate", "/pt-BR/donate"],
		["/br/docs/getting-started", "/pt-BR/docs/getting-started"],
		["/pt/docs/introducao", "/pt-BR/docs/introduction"],
		["/donate", "/pt-BR/donate"],
		["/docs/introduction", "/pt-BR/docs/introduction"],
		["/docs/instalacao", "/pt-BR/docs/installation"],
		["/en/docs/dominios", "/en/docs/domains"],
		["/roadmap", "/pt-BR/changelog"],
		["/en/roadmap", "/en/changelog"],
		["/ja/docs/roadmap", "/ja/changelog"],
	])
		assert.equal(canonicalRedirect(path), canonical, path);
	// The root is left to locale detection; canonical paths stay as they are.
	for (const path of [
		"/",
		"/pt-BR",
		"/pt-BR/donate",
		"/en/donate",
		"/ja/docs/cli",
	])
		assert.equal(canonicalRedirect(path), null, path);
});
