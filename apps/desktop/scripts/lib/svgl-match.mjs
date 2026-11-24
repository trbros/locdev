/** Normalização deliberadamente conservadora: nunca usar substring ou fuzzy matching. */
export const normalizeLogoName = (value) =>
	String(value)
		.normalize("NFKD")
		.replace(/\p{Diacritic}/gu, "")
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "");

/** Alternativas verificáveis de nome para os ids do catálogo do LocDev. */
export const LOGO_ALIASES = {
	laravel: ["Laravel"],
	wordpress: ["WordPress"],
	php: ["PHP"],
	livewire: ["Livewire"],
	inertia: ["Inertia.js", "Inertia"],
	nextjs: ["Next.js", "Nextjs"],
	nuxt: ["Nuxt", "Nuxt.js"],
	astro: ["Astro"],
	sveltekit: ["SvelteKit", "Svelte Kit"],
	remix: ["React Router", "Remix"],
	qwik: ["Qwik"],
	"solid-start": ["SolidStart", "Solid Start"],
	angular: ["Angular"],
	gatsby: ["Gatsby"],
	ember: ["Ember", "Ember.js"],
	redwood: ["RedwoodJS", "Redwood"],
	nestjs: ["NestJS", "Nest.js"],
	fastify: ["Fastify"],
	express: ["Express", "Express.js"],
	hono: ["Hono"],
	elysia: ["ElysiaJS", "Elysia"],
	nodejs: ["Node.js", "NodeJS"],
	bun: ["Bun"],
	deno: ["Deno"],
	vite: ["Vite"],
	react: ["React"],
	vue: ["Vue", "Vue.js"],
	svelte: ["Svelte"],
	solid: ["SolidJS", "Solid.js"],
	preact: ["Preact"],
	storybook: ["Storybook"],
	tailwind: ["Tailwind CSS", "TailwindCSS"],
	typescript: ["TypeScript"],
	javascript: ["JavaScript"],
	html: ["HTML5", "HTML"],
	django: ["Django"],
	flask: ["Flask"],
	fastapi: ["FastAPI"],
	python: ["Python"],
	rails: ["Ruby on Rails", "Rails"],
	ruby: ["Ruby"],
	go: ["Go", "Golang"],
	rust: ["Rust"],
	spring: ["Spring Boot", "SpringBoot"],
	java: ["Java"],
	dotnet: [".NET", "Dotnet"],
	hugo: ["Hugo"],
};

/** Bloqueia URLs externas inesperadas nas rotas retornadas pelo catálogo. */
export function checkedSvgUrl(url) {
	try {
		const parsed = new URL(url);
		if (parsed.protocol !== "https:") return null;
		if (!["svgl.app", "api.svgl.app"].includes(parsed.hostname)) return null;
		if (!parsed.pathname.toLowerCase().endsWith(".svg")) return null;
		return parsed.toString();
	} catch {
		return null;
	}
}

export function chooseRoutes(entry) {
	if (!entry || !entry.route) return null;
	if (typeof entry.route === "string") {
		const url = checkedSvgUrl(entry.route);
		return url ? { light: url, dark: url } : null;
	}
	const light = checkedSvgUrl(entry.route.light);
	const dark = checkedSvgUrl(entry.route.dark);
	if (!light && !dark) return null;
	return { light: light ?? dark, dark: dark ?? light };
}

export function matchLogo(definition, catalog) {
	const aliases = LOGO_ALIASES[definition.id] ?? [definition.name];
	const candidates = [...aliases, definition.name];
	for (const candidate of candidates) {
		const wanted = normalizeLogoName(candidate);
		const matches = catalog.filter(
			(item) => normalizeLogoName(item.title) === wanted && chooseRoutes(item),
		);
		if (matches.length === 1) {
			return { entry: matches[0], routes: chooseRoutes(matches[0]) };
		}
	}
	return null;
}

export function componentName(id) {
	let name = id
		.split(/[^a-zA-Z0-9]+/)
		.map((piece) => (piece ? piece[0].toUpperCase() + piece.slice(1) : ""))
		.join("");
	if (/^[0-9]/.test(name)) name = `Icon${name}`;
	return name || "Unknown";
}

// # Conferir as correspondências sem alterar arquivos
// node scripts/sync-stack-icons.mjs --dry-run

// # Baixar os SVGs e gerar os componentes
// node scripts/sync-stack-icons.mjs
