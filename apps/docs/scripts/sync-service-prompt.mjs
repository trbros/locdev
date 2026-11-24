import { readFile, writeFile } from "node:fs/promises";
import { getStackServices, services } from "../lib/stack-services.ts";
import { stacks } from "../lib/stacks.ts";
import { replacePromptServices } from "./replace-prompt-section.mjs";

const marker = "<!-- planned-stack-services:start -->";
const target = new URL("../PROMPT.md", import.meta.url);
const current = await readFile(target, "utf8");
const categories = {
	database: "Bancos e cache",
	background: "Filas, jobs e agendamentos",
	realtime: "Tempo real",
	tooling: "Ferramentas",
};
const modes = {
	process: "processo separado",
	embedded: "integrado à aplicação/driver",
	external: "instalação externa configurável",
};
const header = `
${marker}
## Serviços por stack: especificação para implementação futura

**Todos os serviços desta seção estão planejados no site, ainda não implementados como integrações de serviços no LocDev.** A presença de um framework no detector não implica detecção de filas, bancos, workers ou scheduler. Não anunciar suporte entregue antes de validar daemon, CLI e desktop.

### Modelo de serviços e critérios de entrega

1. Auditar a arquitetura atual e definir um modelo de serviço associado ao projeto: id, tipo, modo (processo separado, embutido ou conexão externa), cwd, executável e argumentos estruturados, ambiente, dependências, portas, health check, política de restart, timeout e persistência. Versionar a configuração e migrar sem quebrar registros existentes. O site não define um schema TOML de serviços nem novos subcomandos: só documentar sintaxe depois de implementá-la e testá-la no app.
2. Detectar bibliotecas e configurações reais: composer.json, Gemfile/lockfile, package.json/lockfile, pyproject/requirements, Cargo.toml, go.mod, csproj e pom/Gradle. Confirmar integração e versão no projeto. Nunca concluir que todo Laravel usa Horizon ou que todo Node usa BullMQ. Em monorepos, cada processo deve ter cwd e runtime corretos.
3. Diferenciar job (unidade de trabalho), fila (transporte/armazenamento), worker (consumidor) e scheduler (produtor agendado). Não tratar um job como daemon. Modelar comandos pontuais, como WP-CLI cron, separadamente de processos persistentes.
4. Iniciar dependências antes dos consumidores, aguardar readiness real, detectar ciclos e conflitos de portas, parar na ordem inversa e permitir serviços compartilhados sem pará-los enquanto outro projeto os utiliza. Distinguir indisponível, iniciando, pronto, degradado, parando e erro; não usar apenas PID como prova de saúde.
5. Supervisão: logs por processo, cancelamento, reinício manual, backoff limitado, limite de tentativas, encerramento gracioso e timeout. Workers devem concluir ou liberar jobs sem perder mensagens. Não duplicar scheduler/Beat por projeto. Horizon e queue:work, Solid Queue e Sidekiq são alternativas configuráveis; não iniciar todos na mesma fila automaticamente.
6. Instalação de pacotes, migrations, seeds, criação de filas/tabelas, confiança TLS, provisionamento de bancos e alterações de arquivos exigem ação explícita no app. Nunca executar isso como efeito colateral da detecção. Exibir comando e pré-requisitos antes de iniciar; não interpolar entrada do usuário em shell nem registrar segredos nos logs.
7. Bancos: reutilizar instalações locais, conexões externas ou adaptadores de containers opcionais. Não tornar Docker obrigatório. Se implementar containers, exigir runtime disponível, versões/imagens fixadas e revisadas, volumes persistentes e bind em loopback. Nunca remover volumes/dados ao parar ou excluir um projeto; remoção de dados exige confirmação específica.
8. SQLite é arquivo/driver, não daemon. Expor caminho, existência, permissões e operações explícitas de backup; não inventar start/stop do SQLite. PostgreSQL/MySQL/MariaDB/MongoDB/SQL Server precisam de configuração específica de versão, autenticação e diretório. Redis e Valkey são alternativas e compartilham a porta padrão 6379; não iniciar ambos nessa porta. Respeitar suporte real de cada cliente/biblioteca e sistema operacional.
9. Credenciais: armazenamento seguro local, ocultação no desktop e CLI e exportação com placeholders. Não reescrever .env silenciosamente. Sugestões de variáveis como DB_HOST, REDIS_URL, SMTP_HOST ou S3_ENDPOINT dependem do framework e devem ser revisadas. Limitar bind a loopback por padrão e permitir CORS/origins/proxy WebSocket explicitamente.
10. Serviços embutidos, como Action Cable, SignalR, Nest Schedule e Spring Scheduling, podem compartilhar o processo HTTP. Criar processo adicional somente se o projeto fornecer um entrypoint separado. Frontends estáticos/client-side não acessam bancos/filas diretamente: configurar o backend associado. Não prometer BullMQ compatível com qualquer runtime sem validar versões e bibliotecas.
11. Mailpit captura mensagens de desenvolvimento; não entrega e-mails reais. S3 local requer endpoint/bucket/credenciais próprios e não deve acessar buckets de produção por padrão. Meilisearch precisa de chave local e diretório persistente. Storybook/MSW são opcionais e dependem de suporte real da versão do framework.
12. Entregar controles coerentes no desktop e na CLI apenas quando implementados: listar, configurar, iniciar, parar, reiniciar, inspecionar saúde e acompanhar logs. Mostrar disponibilidade por OS (Linux/macOS/Windows/WSL), pré-requisitos e erro acionável. Permitir comando customizado revisável sem confundir isso com integração nativa.
13. Testar: detecção positiva/negativa por versão, precedência de monorepos, argumentos e caminhos com espaços, dependência ausente, readiness com falha, conflitos, scheduler único, retries, processo que trava, parada graciosa, redaction de segredos e persistência após reinício. Validar fluxos completos em cada OS suportado. Atualizar documentação e status no site somente conforme a entrega real.

### Catálogo de serviços de referência

Comandos pressupõem ferramentas e dependências já instaladas e projeto configurado. As portas são exemplos editáveis, não reservas do LocDev. Substitua \`<module>\` pelo módulo Python real. Não executar placeholders. Onde não há comando universal, usar a configuração concreta do projeto, não inventar um binário ou script. Respeitar o gerenciador e lockfile em vez de impor npm.
`;
let body = header;
for (const service of services) {
	body += `\n#### Serviço: ${service.name} (\`${service.id}\`)\n\n- Categoria: ${categories[service.category]}.\n- Modo: ${modes[service.mode]}.\n- Pré-requisitos técnicos: ${service.requires}.\n- Referência oficial: ${service.docs}\n`;
	body += service.command
		? `\nComando de referência:\n\n\`\`\`bash\n${service.command}\n\`\`\`\n`
		: `\nSem comando universal: ${service.mode === "embedded" ? "configurar dentro da aplicação ou driver; não criar daemon fictício" : "revisar instalação, entrypoint, caminhos, portas, credenciais e armazenamento"}.\n`;
}
body +=
	"\n### Matriz de serviços das 41 stacks\n\nCada lista é uma seleção opcional para implementar, não uma lista de processos que devem ser iniciados juntos. Os IDs remetem ao catálogo acima, incluindo pré-requisitos, comandos e documentação. Para stacks full-stack, separar os serviços do backend dos processos de interface.\n";
for (const stack of stacks) {
	body += `\n#### Stack: ${stack.name} (\`${stack.id}\`)\n\n`;
	const items = getStackServices(stack.id);
	for (const [category, label] of Object.entries(categories)) {
		const matching = items.filter((s) => s.category === category);
		if (matching.length)
			body += `- ${label}: ${matching.map((s) => `${s.name} (\`${s.id}\`)`).join(", ")}.\n`;
	}
}
body += "\n<!-- planned-stack-services:end -->\n";
await writeFile(target, replacePromptServices(current, body));
console.log(
	`PROMPT.md: ${stacks.length} stacks, ${services.length} serviços planejados.`,
);
