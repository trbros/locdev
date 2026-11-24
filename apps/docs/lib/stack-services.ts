import { repository } from "./repository.ts";
import type { Stack } from "./stacks";

export type ServiceCategory =
	| "database"
	| "background"
	| "realtime"
	| "tooling";
export type StackService = {
	id: string;
	name: string;
	category: ServiceCategory;
	command: string | null;
	requires: string;
	docs: string;
	mode: "process" | "embedded" | "external";
};

/** What LocDev already does for a service, from the real code:
 * - instances: LocDev runs its own instances (crates/locdev-services, ServiceKind);
 * - process: an adapter offers it as an auxiliary process (`processes` in
 *   crates/locdev-frameworks/definitions/frameworks.toml);
 * - devServer: the auxiliary dev server next to the main one (Vite with Laravel);
 * - mailbox: LocDev's own capture-only SMTP takes this role;
 * - browser: the Database Manager opens it (a file, nothing to run).
 * Anything missing here is still a proposal. tests/stack-services.test.mjs
 * checks this map against those files. */
export type LocdevSupport =
	| "instances"
	| "process"
	| "devServer"
	| "mailbox"
	| "browser"
	| "proposed";
const support: Record<string, LocdevSupport> = {
	postgres: "instances",
	mysql: "instances",
	mariadb: "instances",
	mongodb: "instances",
	redis: "instances",
	valkey: "instances",
	queue: "process",
	scheduler: "process",
	horizon: "process",
	reverb: "process",
	solidqueue: "process",
	sidekiq: "process",
	celery: "process",
	beat: "process",
	storybook: "process",
	vite: "devServer",
	mailpit: "mailbox",
	sqlite: "browser",
};
export function locdevSupport(id: string): LocdevSupport {
	return support[id] ?? "proposed";
}

/** Commands assume installed/configured dependencies. null means
 * project-specific configuration; never invent a runnable command. */
export const services: StackService[] = [
	{
		id: "sqlserver",
		name: "SQL Server",
		category: "database",
		command: null,
		requires: "SQL Server · supported platform · local credentials",
		docs: "https://learn.microsoft.com/sql/linux/sql-server-linux-setup",
		mode: "external",
	},
	{
		id: "postgres",
		name: "PostgreSQL",
		category: "database",
		command: null,
		requires: "PostgreSQL · data directory · local credentials",
		docs: "https://www.postgresql.org/docs/current/server-start.html",
		mode: "external",
	},
	{
		id: "mysql",
		name: "MySQL",
		category: "database",
		command: null,
		requires: "MySQL · data directory · local credentials",
		docs: "https://dev.mysql.com/doc/refman/8.4/en/starting-server.html",
		mode: "external",
	},
	{
		id: "mariadb",
		name: "MariaDB",
		category: "database",
		command: null,
		requires: "MariaDB · data directory · local credentials",
		docs: "https://mariadb.com/docs/server",
		mode: "external",
	},
	{
		id: "sqlite",
		name: "SQLite",
		category: "database",
		command: null,
		requires: "SQLite driver · database file",
		docs: "https://www.sqlite.org/serverless.html",
		mode: "embedded",
	},
	{
		id: "redis",
		name: "Redis",
		category: "database",
		command: "redis-server --bind 127.0.0.1 --port 6379",
		requires: "Redis · local development configuration",
		docs: "https://redis.io/docs/latest/operate/oss_and_stack/management/config/",
		mode: "process",
	},
	{
		id: "valkey",
		name: "Valkey",
		category: "database",
		command: "valkey-server --bind 127.0.0.1 --port 6379",
		requires: "Valkey · local development configuration",
		docs: "https://valkey.io/topics/config/",
		mode: "process",
	},
	{
		id: "mongodb",
		name: "MongoDB",
		category: "database",
		command: null,
		requires: "MongoDB · data directory · local credentials",
		docs: "https://www.mongodb.com/docs/manual/administration/install-community/",
		mode: "external",
	},
	{
		id: "rabbitmq",
		name: "RabbitMQ",
		category: "background",
		command: null,
		requires: "RabbitMQ · Erlang · local credentials",
		docs: "https://www.rabbitmq.com/docs",
		mode: "external",
	},
	{
		id: "mailpit",
		name: "Mailpit",
		category: "tooling",
		command: "mailpit --listen 127.0.0.1:8025 --smtp 127.0.0.1:1025",
		requires: "Mailpit · SMTP configuration",
		docs: "https://mailpit.axllent.org/docs/configuration/runtime-options/",
		mode: "process",
	},
	{
		id: "meilisearch",
		name: "Meilisearch",
		category: "tooling",
		command: null,
		requires: "Meilisearch · local master key · data directory",
		docs: "https://www.meilisearch.com/docs/learn/self_hosted/getting_started_with_self_hosted_meilisearch",
		mode: "external",
	},
	{
		id: "s3",
		name: "S3-compatible storage",
		category: "tooling",
		command: null,
		requires: "Local S3 endpoint · bucket · credentials",
		docs: "https://docs.localstack.cloud/aws/services/s3/",
		mode: "external",
	},
	{
		id: "reverb",
		name: "Laravel Reverb",
		category: "realtime",
		command: "php artisan reverb:start --host=127.0.0.1 --port=8080",
		requires: "laravel/reverb · broadcasting configuration",
		docs: "https://laravel.com/docs/reverb",
		mode: "process",
	},
	{
		id: "queue",
		name: "Laravel Queue Worker",
		category: "background",
		command: "php artisan queue:work --sleep=3 --tries=3",
		requires: "Queue connection · job classes · database or Redis",
		docs: "https://laravel.com/docs/queues",
		mode: "process",
	},
	{
		id: "horizon",
		name: "Laravel Horizon",
		category: "background",
		command: "php artisan horizon",
		requires: "laravel/horizon · Redis · supported OS",
		docs: "https://laravel.com/docs/horizon",
		mode: "process",
	},
	{
		id: "scheduler",
		name: "Laravel Scheduler",
		category: "background",
		command: "php artisan schedule:work",
		requires: "Scheduled tasks · one scheduler per project",
		docs: "https://laravel.com/docs/scheduling",
		mode: "process",
	},
	{
		id: "vite",
		name: "Vite assets",
		category: "tooling",
		command: "npm run dev",
		requires: "Vite · package.json dev script · project lockfile",
		docs: "https://vite.dev/guide/cli.html",
		mode: "process",
	},
	{
		id: "solidqueue",
		name: "Active Job / Solid Queue",
		category: "background",
		command: "bin/jobs start",
		requires: "Solid Queue · bin/jobs · queue database · recurring.yml",
		docs: "https://guides.rubyonrails.org/active_job_basics.html",
		mode: "process",
	},
	{
		id: "sidekiq",
		name: "Sidekiq",
		category: "background",
		command: "bundle exec sidekiq",
		requires: "Sidekiq · Redis · job classes",
		docs: "https://github.com/sidekiq/sidekiq/wiki/Getting-Started",
		mode: "process",
	},
	{
		id: "cable",
		name: "Action Cable",
		category: "realtime",
		command: null,
		requires: "Rails · config/cable.yml · configured adapter",
		docs: "https://guides.rubyonrails.org/action_cable_overview.html",
		mode: "embedded",
	},
	{
		id: "celery",
		name: "Celery Worker",
		category: "background",
		command: "celery -A <module> worker --loglevel=info",
		requires: "Celery · application module · Redis or RabbitMQ broker",
		docs: "https://docs.celeryq.dev/en/stable/reference/cli.html",
		mode: "process",
	},
	{
		id: "beat",
		name: "Celery Beat",
		category: "background",
		command: "celery -A <module> beat --loglevel=info",
		requires: "Celery · application module · schedules · single Beat instance",
		docs: "https://docs.celeryq.dev/en/stable/userguide/periodic-tasks.html",
		mode: "process",
	},
	{
		id: "channels",
		name: "Django Channels",
		category: "realtime",
		command: null,
		requires: "Channels · ASGI server · optional channels_redis",
		docs: "https://channels.readthedocs.io/en/stable/",
		mode: "embedded",
	},
	{
		id: "bullmq",
		name: "BullMQ workers / Job Schedulers",
		category: "background",
		command: null,
		requires: "BullMQ · Redis · project worker entrypoint",
		docs: "https://docs.bullmq.io/guide/workers",
		mode: "process",
	},
	{
		id: "socketio",
		name: "Socket.IO",
		category: "realtime",
		command: null,
		requires: "Socket.IO · backend entrypoint · WebSocket proxy",
		docs: "https://socket.io/docs/v4/server-initialization/",
		mode: "embedded",
	},
	{
		id: "nestcron",
		name: "NestJS Schedule",
		category: "background",
		command: null,
		requires: "@nestjs/schedule · configured application module",
		docs: "https://docs.nestjs.com/techniques/task-scheduling",
		mode: "embedded",
	},
	{
		id: "wpcron",
		name: "WP-Cron",
		category: "background",
		command: "wp cron event run --due-now",
		requires: "WP-CLI · WordPress database · explicit schedule",
		docs: "https://developer.wordpress.org/cli/commands/cron/event/run/",
		mode: "process",
	},
	{
		id: "messenger",
		name: "Symfony Messenger",
		category: "background",
		command: "php bin/console messenger:consume async",
		requires: "Symfony Messenger · configured async transport",
		docs: "https://symfony.com/doc/current/messenger.html",
		mode: "process",
	},
	{
		id: "asynq",
		name: "Asynq workers / Scheduler",
		category: "background",
		command: null,
		requires: "Asynq · Redis · Go worker entrypoint",
		docs: "https://github.com/hibiken/asynq",
		mode: "process",
	},
	{
		id: "apalis",
		name: "Apalis workers",
		category: "background",
		command: null,
		requires: "Apalis · configured storage · Rust worker binary",
		docs: "https://docs.rs/apalis/latest/apalis/",
		mode: "process",
	},
	{
		id: "springjobs",
		name: "Spring Batch / Scheduling",
		category: "background",
		command: null,
		requires: "Spring Batch or @Scheduled · configured application",
		docs: "https://docs.spring.io/spring-framework/reference/integration/scheduling.html",
		mode: "embedded",
	},
	{
		id: "hangfire",
		name: "Hangfire / Quartz.NET",
		category: "background",
		command: null,
		requires: "Configured .NET worker host · selected storage provider",
		docs: "https://docs.hangfire.io/en/latest/background-processing/processing-background-jobs.html",
		mode: "embedded",
	},
	{
		id: "signalr",
		name: "ASP.NET Core SignalR",
		category: "realtime",
		command: null,
		requires: "ASP.NET Core · configured hubs",
		docs: "https://learn.microsoft.com/aspnet/core/signalr/introduction",
		mode: "embedded",
	},
	{
		id: "storybook",
		name: "Storybook",
		category: "tooling",
		command: "npm run storybook -- --host 127.0.0.1 --port 6006",
		requires:
			"Storybook · package.json storybook script · compatible framework",
		docs: "https://storybook.js.org/docs/api/cli-options",
		mode: "process",
	},
	{
		id: "mock",
		name: "Mock API / MSW",
		category: "tooling",
		command: null,
		requires: "MSW · project request handlers",
		docs: "https://mswjs.io/docs/",
		mode: "embedded",
	},
];
const data = ["postgres", "mysql", "sqlite", "redis", "valkey"];
const tools = ["mailpit", "s3"];
const backend = [...data, ...tools, "mongodb", "rabbitmq", "meilisearch"];
const frontend = ["storybook", "mock"];
const node = [...backend, "bullmq", "socketio"];
const python = [...backend, "celery", "beat"];
const profiles: Record<Stack["id"], readonly string[]> = {
	laravel: [
		...data,
		...tools,
		"mariadb",
		"meilisearch",
		"reverb",
		"queue",
		"horizon",
		"scheduler",
		"vite",
	],
	rails: [...data, ...tools, "solidqueue", "sidekiq", "cable"],
	wordpress: ["mysql", "mariadb", "redis", "mailpit", "wpcron"],
	php: [...backend, "mariadb", "messenger"],
	nextjs: node,
	nuxt: node,
	svelte: node,
	astro: node,
	qwik: node,
	remix: node,
	redwood: node,
	nodejs: node,
	express: node,
	fastify: node,
	nestjs: [...node, "nestcron"],
	hono: node,
	elysia: [...data, ...tools, "mock"],
	bun: [...data, ...tools, "mock"],
	deno: [...data, ...tools, "mock"],
	react: frontend,
	vue: frontend,
	angular: frontend,
	preact: frontend,
	solid: frontend,
	ember: ["mock"],
	gatsby: ["mock"],
	vite: frontend,
	javascript: frontend,
	typescript: frontend,
	html: ["mock"],
	hugo: ["mock"],
	python: python,
	django: [...python, "channels"],
	flask: python,
	fastapi: python,
	ruby: [...backend, "sidekiq"],
	go: [...backend, "asynq"],
	rust: [...backend, "apalis"],
	java: [...backend, "sqlserver"],
	spring: [...backend, "sqlserver", "springjobs"],
	dotnet: [...backend, "sqlserver", "hangfire", "signalr"],
};
export function getStackServices(id: Stack["id"]): StackService[] {
	return profiles[id].map((key) => {
		const service = services.find((s) => s.id === key);
		if (!service) throw new Error(`Unknown service: ${key}`);
		return service;
	});
}

/** Opens a draft for the visitor to review; never submits an issue. */
export function serviceRequestUrl(stackName: string) {
	const url = new URL(repository.issues);
	url.searchParams.set("title", `Service integration: ${stackName}`);
	url.searchParams.set(
		"body",
		`Stack: ${stackName}\n\nService / library:\n\nDevelopment use case:\n\nOfficial documentation:\n\nOS and prerequisites:\n\nSuggested command (without credentials):\n`,
	);
	return url.toString();
}
