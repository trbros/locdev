import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

const languages = ["br", "en", "es", "ja"];
const guides = ["troubleshooting", "updating", "uninstalling"];
const read = (locale, slug) =>
	readFileSync(
		new URL(`../content/docs/${locale}/${slug}.md`, import.meta.url),
		"utf8",
	);
const commands = (source) =>
	[...source.matchAll(/```bash\n([\s\S]*?)```/g)].flatMap((match) =>
		match[1].trim().split("\n"),
	);
const supported = new Set([
	"locdev version",
	"locdev status",
	"locdev doctor",
	"locdev list",
	"locdev start",
	"locdev stop",
	"locdev logs acme",
	"locdev domain acme",
	"locdev reload acme",
	"locdev dns status",
	"locdev dns install",
	"locdev tls status",
	"locdev tls trust",
	"locdev down acme",
	"locdev remove acme",
	"locdev startup status",
	"locdev startup uninstall",
	"locdev dns uninstall",
	"locdev tls untrust",
	"locdev update",
]);

// Each allowed recipe must also be a real command in the CLI catalog, so a
// renamed command (as `service` → `startup`) fails here instead of in the docs.
const catalog = JSON.parse(
	readFileSync(new URL("../generated/cli.json", import.meta.url), "utf8"),
);
const usages = new Set(
	catalog.categories.flatMap((category) =>
		category.commands.map((command) => command.usage),
	),
);

test("allowed maintenance commands exist in the CLI catalog", () => {
	for (const command of supported) {
		const words = command.split(" ").filter((word) => word !== "acme");
		assert.ok(usages.has(words.join(" ")), command);
	}
});

test("maintenance recipes use audited CLI commands and preserve review prompts", () => {
	for (const locale of languages)
		for (const slug of guides) {
			const source = read(locale, slug);
			for (const command of commands(source))
				assert.ok(supported.has(command), `${locale}/${slug}: ${command}`);
			assert.ok(!source.includes("--yes"));
			assert.ok(!source.includes("rm -rf"));
		}
});
test("full removal documents system cleanup before stopping and removing the daemon", () => {
	for (const locale of languages) {
		const list = commands(read(locale, "uninstalling"));
		const stop = list.indexOf("locdev stop");
		assert.ok(stop > 0);
		for (const step of [
			"locdev dns uninstall",
			"locdev tls untrust",
			"locdev startup uninstall",
		]) {
			assert.ok(list.indexOf(step) >= 0);
			assert.ok(
				list.indexOf(step) < stop,
				`${locale}: ${step} must precede stop`,
			);
		}
	}
});
test("new documentation links resolve and use the same English slugs in every language", () => {
	for (const locale of languages)
		for (const slug of guides) {
			const source = read(locale, slug);
			assert.equal((source.match(/^# /gm) ?? []).length, 1);
			for (const match of source.matchAll(/\]\((\/[^)]+)\)/g)) {
				const path = match[1].split("#")[0];
				if (path.startsWith("/docs/"))
					assert.ok(
						existsSync(
							new URL(
								`../content/docs/${locale}/${path.slice(6)}.md`,
								import.meta.url,
							),
						),
						`${locale}: ${path}`,
					);
				else
					assert.ok(
						existsSync(
							new URL(
								`../app/[locale]/(site)${path}/page.tsx`,
								import.meta.url,
							),
						),
						path,
					);
			}
		}
});
