import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => readFileSync(resolve(root, name), "utf8");

test("all translated App Router pages exist", () => {
	for (const route of [
		"page.tsx",
		"features/page.tsx",
		"compare/page.tsx",
		"download/page.tsx",
		"get-started/page.tsx",
		"configure/page.tsx",
		"changelog/page.tsx",
		"contribute/page.tsx",
		"stacks/page.tsx",
		"services/page.tsx",
		"about/page.tsx",
		"support/page.tsx",
		"faq/page.tsx",
		"privacy/page.tsx",
		"security/page.tsx",
		"docs/page.tsx",
		"license/page.tsx",
		"donate/page.tsx",
	]) {
		assert.ok(existsSync(resolve(root, `app/[locale]/(site)/${route}`)), route);
	}
});
test("LocDev design preserves the exact upstream OKLCH identity tokens", () => {
	const css = read("app/globals.css");
	for (const token of [
		"--background: oklch(0.175 0.006 60);",
		"--ember: oklch(0.73 0.16 48);",
		"--phosphor: oklch(0.8 0.17 148);",
		"--chrome: oklch(0.155 0.006 60);",
	]) {
		assert.ok(css.includes(token), `missing upstream token ${token}`);
	}
});
test("Next app uses next-intl proxy and the native bundler, not Vite", () => {
	assert.match(read("proxy.ts"), /next-intl\/middleware/);
	assert.match(read("app/[locale]/layout.tsx"), /NextIntlClientProvider/);
	const pkg = JSON.parse(read("package.json"));
	assert.match(pkg.scripts.dev, /^next dev$/);
	assert.equal(pkg.dependencies.next.startsWith("^16."), true);
	assert.equal(
		"vite" in { ...pkg.dependencies, ...pkg.devDependencies },
		false,
	);
});
