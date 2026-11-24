import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const root = new URL("../../../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const languages = { en: "", "pt-BR": ".pt-BR", es: ".es", ja: ".ja" };

test("public READMEs link every manual article in their own language", () => {
	for (const [locale, suffix] of Object.entries(languages)) {
		const readme = read(`README${suffix}.md`);
		const fileKey = locale === "pt-BR" ? "br" : locale;
		const slugs = read("apps/docs/lib/docs.ts")
			.split("] as const;")[0]
			.matchAll(/"([a-z][a-z-]+)"/g);
		for (const [, slug] of slugs) {
			if (
				existsSync(
					new URL(`apps/docs/content/docs/${fileKey}/${slug}.md`, root),
				)
			)
				assert.ok(
					readme.includes(`https://locdev.app/${locale}/docs/${slug})`),
					`${locale}: ${slug}`,
				);
		}
		assert.ok(readme.includes("https://locdev.app/docs"));
		assert.ok(readme.includes(`CHANGELOG${suffix}.md`));
		assert.ok(readme.includes(`https://locdev.app/${locale}/donate`));
		assert.ok(readme.includes('alt="LocDev"'));
		assert.ok(readme.includes('alt="triooo"'));
	}
});

test("README language menus, local links, images and section anchors resolve", () => {
	for (const folder of ["", "apps/docs/"]) {
		for (const suffix of Object.values(languages)) {
			const file = new URL(`${folder}README${suffix}.md`, root);
			const text = readFileSync(file, "utf8");
			for (const otherSuffix of Object.values(languages))
				assert.ok(text.includes(`](README${otherSuffix}.md)`));
			const prose = text.replace(/`[^`]*`/g, "");
			for (const [, target] of prose.matchAll(/\]\(([^)]+)\)/g)) {
				if (/^https?:/.test(target)) continue;
				if (target.startsWith("#")) {
					assert.ok(
						text.includes(`id="${target.slice(1)}"`),
						`${file}: ${target}`,
					);
				} else {
					assert.ok(existsSync(new URL(target, file)), `${file}: ${target}`);
				}
			}
			for (const [, target] of text.matchAll(/(?:src|srcset)="([^"]+)"/g))
				assert.ok(existsSync(new URL(target, file)), `${file}: ${target}`);
			assert.ok(!text.includes("\uFFFD"), `${file}: damaged UTF-8`);
		}
	}
});
