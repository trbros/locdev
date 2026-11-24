import type { Stack } from "./stacks";

/** Diagnostic/native commands, run inside an existing project with its tools installed. */
export const frameworkCommands: Record<Stack["id"], string> = {
	laravel: "php artisan route:list",
	nextjs: "npx next info",
	vue: "npm ls vue",
	rails: "bin/rails routes",
	rust: "cargo check",
	react: "npm ls react",
	nuxt: "npm ls nuxt",
	svelte: "npm ls svelte",
	astro: "npx astro --version",
	angular: "npx ng version",
	preact: "npm ls preact",
	solid: "npm ls solid-js",
	qwik: "npm ls @builder.io/qwik",
	remix: "npm ls @remix-run/dev",
	gatsby: "npx gatsby info",
	ember: "npx ember version",
	redwood: "yarn rw info",
	wordpress: "wp core version",
	php: "php --version",
	nodejs: "node --version",
	express: "npm ls express",
	fastify: "npm ls fastify",
	nestjs: "npm ls @nestjs/core",
	hono: "npm ls hono",
	elysia: "bun pm ls",
	bun: "bun --version",
	deno: "deno info",
	python: "python -m pip check",
	django: "python manage.py check",
	flask: "flask --app app routes",
	fastapi: "fastapi --help",
	ruby: "ruby --version",
	go: "go vet ./...",
	hugo: "hugo version",
	java: "java --version",
	spring: "./mvnw dependency:tree",
	dotnet: "dotnet --info",
	vite: "npx vite --version",
	html: "python --version",
	javascript: "npm ls vite",
	typescript: "npx tsc --noEmit",
};

function quoteShell(value: string) {
	return `'${value.replaceAll("'", "'\\''")}'`;
}

/** Audited against apps/locdev/src/main.rs. No new CLI subcommands are implied. */
export function getLocdevCommands(stack: Stack): string[] {
	const command = stack.commands.run;
	return [
		"locdev start",
		`locdev add . --name ${stack.id}${command ? ` --command ${quoteShell(command)}` : ""}`,
		...(!command
			? [`locdev edit ${stack.id} --command '<HTTP_SERVER_COMMAND>'`]
			: []),
		`locdev up ${stack.id}`,
		`locdev domain ${stack.id}`,
		`locdev logs ${stack.id} -f`,
	];
}
