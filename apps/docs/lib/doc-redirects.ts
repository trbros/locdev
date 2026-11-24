/** Old docs slugs (the Portuguese ones of the first docs site, and earlier
 *  names) and pages, so published links keep working. */
export const legacyDocSlugs = {
	introducao: "introduction",
	instalacao: "installation",
	"primeiros-passos": "getting-started",
	dominios: "domains",
	subdominios: "subdomains",
	servico: "service",
	servicos: "services",
	servidores: "servers",
	ambiente: "environment",
	"banco-de-dados": "database",
	dependencias: "dependencies",
	configuracao: "configuration",
	arquitetura: "architecture",
	seguranca: "security",
} as const;

/** Pages that moved (after the locale prefix). */
export const legacyPaths = {
	"/roadmap": "/changelog",
	"/docs/roadmap": "/changelog",
} as const;
