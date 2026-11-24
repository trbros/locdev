export type ToolId =
	| "locdev"
	| "herd"
	| "yerd"
	| "lerd"
	| "laragon"
	| "ddev"
	| "lando"
	| "sail"
	| "valet"
	| "compose";

export type Status = "yes" | "partial" | "no" | "planned" | "unknown";

export type Cell = {
	status: Status;
	note?: string;
	source?: string;
};

export type Feature = {
	id: string;
	label: string;
	description: string;
	cells: Record<ToolId, Cell>;
};

export type Group = {
	id: string;
	title: string;
	description: string;
	features: Feature[];
};

const github = "https://github.com/trbros/locdev/blob/main/";

export const tools: {
	id: ToolId;
	name: string;
	url: string;
	source: string;
}[] = [
	{
		id: "locdev",
		name: "LocDev",
		url: "https://github.com/trbros/locdev",
		source: `${github}README.md`,
	},
	{
		id: "herd",
		name: "Herd",
		url: "https://herd.laravel.com/",
		source: "https://herd.laravel.com/",
	},
	{
		id: "yerd",
		name: "Yerd",
		url: "https://yerd.app/",
		source: "https://yerd.app/guide/getting-started",
	},
	{
		id: "lerd",
		name: "Lerd",
		url: "https://lerd.sh/",
		source: "https://lerd.sh/features/",
	},
	{
		id: "laragon",
		name: "Laragon",
		url: "https://laragon.org/",
		source: "https://laragon.org/docs",
	},
	{
		id: "ddev",
		name: "DDEV",
		url: "https://ddev.com/",
		source: "https://docs.ddev.com/en/stable/",
	},
	{
		id: "lando",
		name: "Lando",
		url: "https://lando.dev/",
		source: "https://docs.lando.dev/",
	},
	{
		id: "sail",
		name: "Sail",
		url: "https://laravel.com/docs/sail",
		source: "https://laravel.com/framework/docs/sail",
	},
	{
		id: "valet",
		name: "Valet",
		url: "https://laravel.com/docs/valet",
		source: "https://laravel.com/framework/docs/valet",
	},
	{
		id: "compose",
		name: "Compose",
		url: "https://docs.docker.com/compose/",
		source: "https://docs.docker.com/compose/",
	},
];

const sources: Record<string, string> = {
	git: `${github}crates/locdev-git/src/lib.rs`,
	github: `${github}crates/locdev-daemon/src/daemon/github.rs`,
	actions: `${github}crates/locdev-actions/src/lib.rs`,
	emailstudio: `${github}crates/locdev-email/src/lib.rs`,
	debugcenter: `${github}crates/locdev-daemon/src/daemon/debug.rs`,
	redaction: `${github}crates/locdev-observe/src/redact.rs`,
	dumps: `${github}crates/locdev-daemon/src/daemon/debug.rs`,
	dbbrowser: `${github}crates/locdev-db/src/lib.rs`,
	tunnels: `${github}crates/locdev-daemon/src/daemon/tunnels.rs`,
	deploy: `${github}crates/locdev-deploy/src/remote/mod.rs`,

	detect: `${github}crates/locdev-frameworks/definitions/frameworks.toml`,
	services: `${github}crates/locdev-services/src/lib.rs`,
	mailbox: `${github}crates/locdev-mail/src/lib.rs`,
	env: `${github}crates/locdev-env/src/lib.rs`,
	scaffold: `${github}crates/locdev-scaffold/src/catalog.rs`,
	doctor: `${github}crates/locdev-daemon/src/doctor.rs`,
	protocol: `${github}crates/locdev-core/src/protocol.rs`,
	runtime: `${github}crates/locdev-daemon/src/runtimes.rs`,
	proxy: `${github}crates/locdev-proxy/src/lib.rs`,
	dns: `${github}crates/locdev-dns/src/lib.rs`,
	tls: `${github}crates/locdev-tls/src/lib.rs`,
	daemon: `${github}crates/locdev-daemon/src/daemon.rs`,
	readme: `${github}README.md`,
	license: `${github}LICENSE`,
	staticserver: `${github}crates/locdev-proxy/src/files.rs`,
	worktrees: `${github}crates/locdev-daemon/src/daemon/git.rs`,
	metrics: `${github}crates/locdev-proxy/src/metrics.rs`,
	tinker: `${github}crates/locdev-daemon/src/daemon/tinker.rs`,
	phpext: `${github}crates/locdev-daemon/src/daemon/extensions.rs`,
	workflowaudit: `${github}crates/locdev-actions/src/validate.rs`,
	installers: `${github}crates/locdev-scaffold/src/tools.rs`,
	servicepackages: `${github}crates/locdev-services/src/packages.rs`,
	herdLicense: "https://herd.laravel.com/pricing",
	locdev: `${github}README.md`,
	herd: "https://herd.laravel.com/",
	yerd: "https://yerd.app/",
	lerd: "https://lerd.sh/features/",
	laragon: "https://laragon.org/docs",
	ddev: "https://docs.ddev.com/en/stable/",
	ddevNode: "https://docs.ddev.com/en/stable/users/quickstart/",
	lando: "https://docs.lando.dev/plugins/node/",
	sail: "https://laravel.com/framework/docs/sail",
	valet: "https://laravel.com/framework/docs/valet",
	compose: "https://docs.docker.com/compose/",
};

const Y: Status = "yes";
const P: Status = "partial";
const N: Status = "no";
const U: Status = "unknown";

const order: ToolId[] = [
	"locdev",
	"herd",
	"yerd",
	"lerd",
	"laragon",
	"ddev",
	"lando",
	"sail",
	"valet",
	"compose",
];

function row(
	id: string,
	label: string,
	description: string,
	values: Status[],
	sourceKeys: Partial<Record<ToolId, string>> = {},
	notes: Partial<Record<ToolId, string>> = {},
): Feature {
	if (values.length !== order.length) {
		throw new Error(`Invalid comparison row: ${id}`);
	}

	const cells = Object.fromEntries(
		order.map((tool, index) => [
			tool,
			{
				status: values[index],
				source: sources[sourceKeys[tool] ?? tool],
				note: notes[tool],
			},
		]),
	) as Record<ToolId, Cell>;

	return { id, label, description, cells };
}

export const groups: Group[] = [
	{
		id: "platforms",
		title: "Sistemas operacionais",
		description: "Disponibilidade da ferramenta principal por plataforma.",
		features: [
			row(
				"linux",
				"Linux",
				"Suporte oficial em Linux",
				[Y, N, Y, Y, N, Y, Y, Y, N, Y],
				{ locdev: "readme", lerd: "lerd" },
			),
			row("macos", "macOS", "Suporte oficial em macOS", [
				Y,
				Y,
				Y,
				Y,
				N,
				Y,
				Y,
				Y,
				Y,
				Y,
			]),
			row(
				"opensource",
				"Código aberto",
				"Código-fonte publicado sob licença aberta",
				[Y, N, U, U, U, Y, Y, Y, Y, Y],
				{ locdev: "license", herd: "herdLicense" },
				{ locdev: "Apache-2.0, sem planos pagos nem recursos bloqueados." },
			),
			row(
				"windows",
				"Windows",
				"Suporte oficial em Windows",
				[Y, Y, N, P, Y, Y, Y, Y, N, Y],
				{},
				{
					locdev:
						"Nativo, sem WSL nem Docker: validado no Windows 11 com sites estáticos e Laravel pelo proxy, DNS .test (NRPT), Tinker, worktrees, métricas e o instalador do Laravel.",
					lerd: "Via WSL2, em beta.",
					sail: "Via WSL2.",
				},
			),
		],
	},
	{
		id: "stacks",
		title: "Frameworks e stacks",
		description: "Detecção nativa ou capacidade de executar projetos.",
		features: [
			row(
				"laravel",
				"Laravel",
				"Executar aplicações Laravel",
				[Y, Y, Y, Y, Y, Y, Y, Y, Y, Y],
				{ locdev: "detect" },
			),
			row(
				"wordpress",
				"WordPress",
				"Executar projetos WordPress",
				[Y, Y, Y, Y, Y, Y, Y, P, Y, Y],
				{ locdev: "detect", valet: "valet" },
				{ sail: "Exige adaptação; Sail é voltado a Laravel." },
			),
			row(
				"php",
				"PHP genérico",
				"Executar aplicações PHP",
				[Y, Y, Y, Y, Y, Y, Y, P, Y, Y],
				{ locdev: "detect" },
			),
			row(
				"next",
				"Next.js",
				"Servidor de desenvolvimento Next.js",
				[Y, P, U, U, P, P, Y, P, N, Y],
				{ locdev: "detect", ddev: "ddevNode", lando: "lando" },
				{
					locdev: "Detecta Next.js e sugere comando de execução.",
					herd: "Suporta Node.js; integração específica deve ser configurada conforme o projeto.",
					laragon: "Suporta Node.js; fluxo Next.js depende do projeto.",
					ddev: "Pode usar servidor genérico; requer configuração.",
					sail: "Possível via customização dos contêineres.",
				},
			),
			row(
				"node",
				"Node.js",
				"Aplicações e servidores Node",
				[Y, Y, Y, Y, Y, Y, Y, P, N, Y],
				{ locdev: "detect", ddev: "ddevNode" },
			),
			row(
				"react",
				"React / Vite",
				"Projetos React com servidor Vite",
				[Y, P, U, U, P, P, Y, P, N, Y],
				{ locdev: "detect", ddev: "ddevNode" },
				{
					locdev:
						"Integração nativa com o Vite: porta, host, hosts permitidos e HMR pelo proxy.",
					ddev: "Requer configuração de portas e servidor.",
				},
			),
			row(
				"python",
				"Python",
				"Executar aplicações Python",
				[Y, U, U, U, Y, P, P, P, N, Y],
				{ locdev: "detect" },
				{
					locdev:
						"Django, Flask e FastAPI com comando de desenvolvimento e o venv do projeto; sem gerenciar versões do Python.",
					laragon: "Gerenciamento de runtimes Python documentado.",
				},
			),
			row(
				"static",
				"Sites estáticos",
				"Servir HTML e arquivos estáticos",
				[Y, Y, Y, Y, Y, Y, Y, P, Y, Y],
				{ locdev: "staticserver", valet: "valet" },
				{
					locdev:
						"Detecta index.html e serve com o servidor estático próprio (URLs limpas, 404.html), sem Python nem Node.",
				},
			),
		],
	},
	{
		id: "network",
		title: "Rede e infraestrutura",
		description: "Como as aplicações são expostas no ambiente local.",
		features: [
			row(
				"native",
				"Execução sem contêiner obrigatório",
				"Fluxo principal sem Docker ou Podman",
				[Y, Y, Y, N, Y, N, N, N, Y, N],
				{ locdev: "daemon", lerd: "lerd" },
			),
			row(
				"domains",
				"Domínios .test",
				"Endereços locais com resolução automática",
				[Y, Y, Y, Y, Y, Y, P, N, Y, P],
				{ locdev: "dns" },
			),
			row(
				"https",
				"HTTPS local",
				"TLS para desenvolvimento local",
				[Y, Y, Y, Y, Y, Y, Y, P, Y, P],
				{ locdev: "tls" },
			),
			row(
				"websocket",
				"Proxy WebSocket",
				"Encaminhamento para HMR e conexões WebSocket",
				[Y, U, U, U, U, Y, P, P, N, Y],
				{ locdev: "proxy" },
			),
			row(
				"subdomains",
				"Subdomínios locais",
				"Endereços como api.projeto.test",
				[Y, U, U, U, U, P, P, P, P, P],
				{ locdev: "readme" },
			),
		],
	},
	{
		id: "workflow",
		title: "Gerenciamento e produtividade",
		description: "Ferramentas para controlar o desenvolvimento diário.",
		features: [
			row(
				"cli",
				"CLI",
				"Interface de linha de comando",
				[Y, Y, Y, Y, Y, Y, Y, Y, Y, Y],
				{ locdev: "protocol" },
				{
					locdev:
						"Todos os recursos do app também pelo terminal, mais uma TUI em tela cheia (`locdev`).",
				},
			),
			row(
				"desktop",
				"Interface desktop",
				"Aplicativo gráfico dedicado",
				[Y, Y, Y, P, Y, N, N, N, N, Y],
				{ locdev: "readme" },
				{
					lerd: "Disponibiliza interfaces web, TUI e integração desktop.",
					compose: "Docker Desktop oferece interface gráfica.",
				},
			),
			row(
				"daemon",
				"Gerenciamento em segundo plano",
				"Processos ou serviços supervisionados",
				[Y, Y, Y, Y, Y, Y, Y, Y, Y, Y],
				{ locdev: "daemon" },
			),
			row(
				"logs",
				"Logs dos projetos",
				"Visualização de saída dos processos",
				[Y, Y, Y, Y, Y, Y, Y, Y, P, Y],
				{ locdev: "protocol" },
			),
			row(
				"runtime",
				"Gerenciar versões PHP",
				"Detectar, selecionar ou instalar versões PHP",
				[Y, Y, Y, Y, Y, Y, Y, P, P, P],
				{ locdev: "runtime" },
				{
					locdev:
						"Detecção, pins por site e instalação de builds oficiais (StaticPHP, windows.php.net) só com o SHA-256 do publicador; lista as extensões de cada versão.",
				},
			),
			row(
				"nodeversions",
				"Gerenciar versões Node.js",
				"Instalar ou selecionar versões Node",
				[Y, Y, Y, Y, Y, Y, Y, P, N, P],
				{ locdev: "runtime" },
			),
			row(
				"worktrees",
				"Git worktrees integradas",
				"Worktrees gerenciadas pela própria ferramenta",
				[Y, Y, U, Y, U, U, U, N, N, N],
				{ locdev: "worktrees", herd: "herd", lerd: "lerd" },
				{
					locdev:
						"Cada branch numa pasta ao lado, opcionalmente como site próprio (.test) e com os .env copiados; remoção com prévia e backup.",
				},
			),
			row(
				"diagnostics",
				"Diagnóstico integrado",
				"Verificação de problemas do ambiente",
				[Y, U, Y, Y, U, Y, U, P, P, Y],
				{ locdev: "readme" },
			),
		],
	},
	{
		id: "services",
		title: "Serviços de desenvolvimento",
		description: "Serviços provisionados ou integrados diretamente.",
		features: [
			row(
				"mysql",
				"MySQL / MariaDB",
				"Gerenciamento de instâncias locais",
				[Y, Y, Y, Y, Y, Y, Y, Y, N, Y],
				{ locdev: "services" },
				{
					locdev:
						"MariaDB baixada com SHA-256 verificado; MySQL 8.0, 8.4 ou o mais recente instalado pelo app via Homebrew, apt ou winget.",
				},
			),
			row(
				"postgres",
				"PostgreSQL",
				"Gerenciamento de PostgreSQL",
				[Y, Y, Y, Y, Y, Y, Y, P, N, Y],
				{ locdev: "services" },
			),
			row(
				"redis",
				"Redis / Valkey",
				"Serviço de cache gerenciado",
				[Y, Y, Y, Y, P, Y, Y, Y, N, Y],
				{ locdev: "services" },
				{
					locdev:
						"Valkey baixado com SHA-256 verificado; Redis (várias versões) instalado pelo app via Homebrew ou apt.",
				},
			),
			row(
				"mail",
				"Captura de e-mails",
				"Mailpit ou alternativa integrada",
				[Y, Y, Y, Y, P, Y, Y, Y, N, Y],
				{ locdev: "mailbox" },
				{ locdev: "Mailbox próprio: SMTP só de captura em loopback." },
			),
		],
	},
	{
		id: "advanced",
		title: "Ferramentas avançadas",
		description: "Observabilidade, debug e automação.",
		features: [
			row(
				"git",
				"Git visual",
				"Diffs, staging, commits, branches e stashes na interface",
				[Y, U, U, U, U, N, N, N, N, N],
				{ locdev: "git" },
			),
			row(
				"github",
				"GitHub integrado",
				"Conta, repositório e operações GitHub na interface",
				[Y, U, U, U, U, N, N, N, N, N],
				{ locdev: "github" },
			),
			row(
				"actions",
				"Editor de GitHub Actions",
				"Templates, validação e prévia antes de salvar workflows",
				[Y, U, U, U, U, N, N, N, N, N],
				{ locdev: "actions" },
			),
			row(
				"emailstudio",
				"Email Studio",
				"Descoberta, edição, prévia e histórico de templates de e-mail",
				[Y, U, U, U, U, N, N, N, N, N],
				{ locdev: "emailstudio" },
			),
			row(
				"debugcenter",
				"Debug Center multistack",
				"Erros de processos, Laravel, proxy e jobs agrupados por projeto",
				[Y, P, P, Y, U, P, U, N, N, N],
				{ locdev: "debugcenter" },
			),
			row(
				"redaction",
				"Ocultação de segredos no debug",
				"Valores sensíveis removidos dos eventos antes do armazenamento",
				[Y, U, U, U, U, U, U, U, U, U],
				{ locdev: "redaction" },
			),
			row(
				"dumps",
				"Captura de dump() / dd()",
				"Inspeção de dumps PHP no aplicativo",
				[Y, Y, U, Y, U, U, U, N, N, N],
				{ locdev: "dumps" },
			),
			row(
				"dbbrowser",
				"Explorador de banco de dados",
				"Conexões, tabelas e consultas pela interface do aplicativo",
				[Y, Y, U, Y, P, P, P, N, N, P],
				{ locdev: "dbbrowser" },
			),
			row(
				"tunnels",
				"Túneis públicos",
				"Compartilhamento de projetos com provedores externos",
				[Y, Y, Y, Y, Y, Y, P, Y, Y, P],
				{ locdev: "tunnels" },
			),
			row(
				"deploy",
				"Deploy Vercel / Netlify",
				"Vincular projetos, iniciar deploys e solicitar rollback",
				[Y, U, U, U, U, U, U, N, N, N],
				{ locdev: "deploy" },
			),
			row(
				"metrics",
				"Métricas de requisições",
				"Latência e análise de rotas na interface",
				[Y, P, P, Y, U, P, U, N, N, P],
				{ locdev: "metrics", lerd: "lerd" },
				{
					locdev:
						"Pelo proxy: status por classe, p50/p95/p99, requisições por minuto, caminhos mais acessados e as últimas requisições.",
				},
			),
			row(
				"tinker",
				"Laravel Tinker integrado",
				"REPL disponibilizado pela ferramenta",
				[Y, P, P, Y, U, P, P, P, N, P],
				{ locdev: "tinker", lerd: "lerd" },
				{
					locdev:
						"Editor com execução (php artisan tinker --execute) e console interativo, com o PHP e o ambiente do site.",
				},
			),
			row(
				"env",
				"Variáveis globais compartilhadas",
				"Secrets injetados em múltiplos projetos sem editar .env",
				[Y, U, U, U, U, U, U, U, U, P],
				{ locdev: "env" },
				{
					compose:
						"Suporta env_file e variáveis; não equivale a um gerenciador global de secrets.",
				},
			),
			row(
				"creator",
				"Assistente de criação multistack",
				"Criação de novos projetos por interface nativa",
				[Y, P, P, P, Y, P, P, N, N, N],
				{ locdev: "scaffold" },
				{
					locdev:
						"Laravel (com as mesmas perguntas do laravel new: starter kit, autenticação, Livewire, times), Symfony, Bedrock, Next.js, Vite, Nuxt, Astro, SvelteKit, NestJS, Django, Rails e ASP.NET Core pelo instalador oficial de cada um.",
				},
			),
			row(
				"phpext",
				"Extensões PHP por pacote",
				"Extensões exigidas e recomendadas pelos pacotes do projeto contra o que o PHP carrega",
				[Y, U, U, U, U, U, U, N, N, N],
				{ locdev: "phpext" },
			),
			row(
				"workflowaudit",
				"Auditoria e histórico de workflows",
				"Todos os workflows verificados (segurança e boas práticas com correção) e versões locais",
				[Y, U, U, U, U, N, N, N, N, N],
				{ locdev: "workflowaudit" },
			),
			row(
				"installers",
				"Instaladores pelo aplicativo",
				"CLIs e versões de serviços instalados sem abrir um terminal",
				[Y, P, U, U, P, N, N, N, N, N],
				{ locdev: "installers" },
				{
					locdev:
						"Comando oficial de cada ferramenta (Laravel, Symfony, Bun, uv, WP-CLI, Rust, .NET…) num terminal do app, depois de mostrado.",
				},
			),
		],
	},
];

export const statusLabels: Record<Status, string> = {
	yes: "Suportado",
	partial: "Parcial / configurável",
	no: "Não integrado",
	planned: "Planejado",
	unknown: "Não verificado",
};

export const reviewedOn = "2026-11-23";
