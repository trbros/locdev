import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { createTranslator } from "next-intl";
import { repository } from "../lib/repository.ts";
import {
	getStackServices,
	serviceRequestUrl,
	services,
} from "../lib/stack-services.ts";
import { stacks } from "../lib/stacks.ts";

test("every stack has a valid optional service profile without duplicate entries", () => {
	assert.equal(new Set(services.map((s) => s.id)).size, services.length);
	const used = new Set();
	for (const stack of stacks) {
		const items = getStackServices(stack.id);
		assert.ok(items.length, stack.id);
		assert.equal(new Set(items.map((s) => s.id)).size, items.length, stack.id);
		for (const item of items) {
			used.add(item.id);
			assert.equal(new URL(item.docs).protocol, "https:");
		}
	}
	assert.equal(used.size, services.length, "orphaned catalog entries");
});

test("profiles distinguish backend workers and embedded storage from UI tools", () => {
	const ids = (id) => getStackServices(id).map((s) => s.id);
	for (const id of ["reverb", "queue", "horizon", "scheduler"])
		assert.ok(ids("laravel").includes(id));
	assert.ok(ids("rails").includes("solidqueue"));
	assert.ok(ids("django").includes("beat"));
	assert.ok(ids("go").includes("asynq"));
	assert.ok(ids("rust").includes("apalis"));
	assert.ok(ids("dotnet").includes("sqlserver"));
	assert.deepEqual(ids("html"), ["mock"]);
	assert.ok(!ids("wordpress").includes("postgres"));
	const sqlite = services.find((s) => s.id === "sqlite");
	assert.equal(sqlite.mode, "embedded");
	assert.equal(sqlite.command, null);
});

test("localized prerequisites cover the shared service catalog", () => {
	for (const locale of ["br", "en", "es", "ja"]) {
		const messages = JSON.parse(
			readFileSync(new URL(`../messages/${locale}.json`, import.meta.url)),
		);
		for (const service of services)
			assert.ok(messages.Services.requirements[service.id]?.length > 0);
	}
});

test("service request preserves user text in a reviewable issue draft", () => {
	const value = "Laravel & title=changed #日本語";
	const url = new URL(serviceRequestUrl(value));
	assert.equal(url.pathname, new URL(repository.issues).pathname);
	assert.equal(url.searchParams.get("title"), `Service integration: ${value}`);
	assert.ok(url.searchParams.get("body").includes(value));
	assert.equal(url.hash, "");
});

test("service examples interpolate angle brackets without parsing them as rich text", () => {
	for (const locale of ["br", "en", "es", "ja"]) {
		const messages = JSON.parse(
			readFileSync(new URL(`../messages/${locale}.json`, import.meta.url)),
		);
		const errors = [];
		const t = createTranslator({
			locale: locale === "br" ? "pt-BR" : locale,
			messages,
			namespace: "Services",
			onError: (error) => errors.push(error),
		});
		assert.ok(t("example", { module: "<module>" }).includes("<module>"));
		assert.deepEqual(errors, []);
	}
});
test("SVGL light and dark logo variants are local, valid SVG assets", () => {
	const logos = JSON.parse(
		readFileSync(new URL("../lib/service-logos.json", import.meta.url)),
	);
	for (const logo of Object.values(logos))
		for (const path of Object.values(logo.paths)) {
			assert.ok(path.startsWith("/brands/services/"));
			const file = new URL(`../public${path}`, import.meta.url);
			assert.ok(existsSync(file));
			const svg = readFileSync(file, "utf8");
			assert.ok(svg.includes("<svg"));
			assert.ok(!svg.includes("<script"));
		}
});

test("services marked as working in LocDev match the repository", async () => {
	const { locdevSupport } = await import("../lib/stack-services.ts");
	const root = new URL("../../../", import.meta.url);
	const kinds = readFileSync(
		new URL("crates/locdev-core/src/services.rs", root),
		"utf8",
	);
	const definitions = readFileSync(
		new URL("crates/locdev-frameworks/definitions/frameworks.toml", root),
		"utf8",
	);
	// Catalog ids that differ from the process id in the definitions.
	const processId = { solidqueue: "jobs" };
	for (const service of services) {
		const kind = locdevSupport(service.id);
		if (kind === "instances")
			assert.match(
				kinds,
				new RegExp(`=> "${service.id}"`),
				`${service.id} is not a ServiceKind`,
			);
		if (kind === "process")
			assert.match(
				definitions,
				new RegExp(
					`\\{ id = "${processId[service.id] ?? service.id}", name = `,
				),
				`${service.id} is not an auxiliary process in frameworks.toml`,
			);
		if (kind === "mailbox")
			assert.ok(existsSync(new URL("crates/locdev-mail/src/smtp.rs", root)));
		if (kind === "browser")
			assert.match(
				readFileSync(new URL("crates/locdev-db/Cargo.toml", root), "utf8"),
				/^rusqlite = /m,
			);
	}
	for (const locale of ["br", "en", "es", "ja"]) {
		const m = JSON.parse(
			readFileSync(
				new URL(`../messages/${locale}.json`, import.meta.url),
				"utf8",
			),
		).Services;
		for (const kind of [
			"instances",
			"process",
			"devServer",
			"mailbox",
			"browser",
			"proposed",
		])
			for (const key of ["support", "supportShort", "supportNote"])
				assert.ok(m[key]?.[kind], `${locale}: ${key}.${kind}`);
	}
});
