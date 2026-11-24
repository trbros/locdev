import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { test } from "node:test";

const root = resolve(import.meta.dirname, "..");
// The published list is `docGroups` in lib/docs.ts; read it from there so the
// test follows the navigation instead of a second copy.
const docsSource = readFileSync(resolve(root, "lib/docs.ts"), "utf8");
const slugs = [
	...docsSource
		.slice(
			docsSource.indexOf("export const docGroups"),
			docsSource.indexOf("] as const"),
		)
		.matchAll(/"([a-z-]+)"/g),
]
	.map((m) => m[1])
	.filter(
		(s) =>
			![
				"start",
				"tutorials",
				"core",
				"tools",
				"maintenance",
				"reference",
			].includes(s),
	);
const locales = ["br", "en", "es", "ja"];

test("documentation has full locale routes for every page in the navigation", () => {
	assert.ok(
		existsSync(resolve(root, "app/[locale]/(site)/docs/[slug]/page.tsx")),
	);
	assert.ok(slugs.length >= 30, `only ${slugs.length} slugs found`);
	for (const locale of locales)
		for (const slug of slugs) {
			const file = resolve(root, `content/docs/${locale}/${slug}.md`);
			assert.ok(existsSync(file), `${locale}/${slug}`);
			assert.ok(
				readFileSync(file, "utf8").trim().length > 250,
				`${locale}/${slug} must be substantive`,
			);
		}
});
test("i18n docs share identical slugs and have no untranslated PT titles", () => {
	const lists = locales.map((locale) =>
		readdirSync(resolve(root, `content/docs/${locale}`)).sort(),
	);
	for (const list of lists) assert.deepEqual(list, lists[0]);
	assert.match(
		readFileSync(resolve(root, "content/docs/en/introduction.md"), "utf8"),
		/What LocDev is/,
	);
	assert.match(
		readFileSync(resolve(root, "content/docs/es/introduction.md"), "utf8"),
		/Qu[eé] es LocDev/,
	);
	assert.match(
		readFileSync(resolve(root, "content/docs/ja/introduction.md"), "utf8"),
		/LocDev とは/,
	);
});
test("documentation screenshots come from the app in its language and both themes", () => {
	assert.ok(existsSync(resolve(root, "components/docs/screenshot.tsx")));
	// Content folders → the app's language codes (the screenshot folders).
	const appLocale = { br: "pt-BR", en: "en", es: "es", ja: "ja" };
	for (const locale of locales)
		for (const slug of slugs) {
			const content = readFileSync(
				resolve(root, `content/docs/${locale}/${slug}.md`),
				"utf8",
			);
			for (const [, name] of content.matchAll(/\]\(screenshot:([a-z0-9-]+)\)/g))
				for (const theme of ["dark", "light"])
					assert.ok(
						existsSync(
							resolve(
								root,
								`public/screenshots/${appLocale[locale]}/${theme}/${name}.webp`,
							),
						),
						`${locale}/${slug} uses ${appLocale[locale]}/${theme}/${name}.webp, which apps/desktop/scripts/screenshots.mjs does not produce`,
					);
		}
});
test("root layout is not broken by an orphan app/page.tsx", () => {
	assert.equal(existsSync(resolve(root, "app/page.tsx")), false);
	assert.match(
		readFileSync(resolve(root, "app/[locale]/layout.tsx"), "utf8"),
		/<html\s+lang=\{languageTags\[locale\]\}/,
	);
});
test("every locale uses the same screenshots on each page", () => {
	for (const slug of slugs) {
		const names = locales.map((locale) =>
			[
				...readFileSync(
					resolve(root, `content/docs/${locale}/${slug}.md`),
					"utf8",
				).matchAll(/\]\(screenshot:([a-z0-9-]+)\)/g),
			].map((m) => m[1]),
		);
		for (const list of names) assert.deepEqual(list, names[0], slug);
	}
});
test("sitemap covers all localized articles", () => {
	assert.match(
		readFileSync(resolve(root, "app/sitemap.ts"), "utf8"),
		/docSlugs\.map/,
	);
});

test("all localized article links resolve to canonical English slugs", () => {
	for (const locale of locales)
		for (const slug of slugs) {
			const content = readFileSync(
				resolve(root, `content/docs/${locale}/${slug}.md`),
				"utf8",
			);
			for (const [, target] of content.matchAll(
				/\]\(\/docs\/([a-z-]+)(?:#[^)]*)?\)/g,
			)) {
				assert.ok(
					slugs.includes(target),
					`${locale}/${slug} links to unknown slug ${target}`,
				);
				assert.ok(
					existsSync(resolve(root, `content/docs/${locale}/${target}.md`)),
				);
			}
		}
});
