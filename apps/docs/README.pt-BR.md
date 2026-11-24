# Site e documentação do LocDev

[English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [日本語](README.ja.md)

O site do produto e o manual completo ficam neste app Next.js. Documentação pública: **https://locdev.app/docs**. Para recursos e instalação do app, consulte o [README do repositório](../../README.pt-BR.md).

## Stack e requisitos

Next.js 16 App Router, React 19, TypeScript, Tailwind CSS 4, Base UI, next-intl, next-themes, React Markdown, remark-gfm e Shiki no servidor. O Biome cuida de formatação/lint; os testes do Node verificam conteúdo e contratos. Use Node.js 22.18+ (os testes importam TypeScript diretamente) e a versão do pnpm indicada em `package.json`.

## Executar e validar

Nesta pasta, copie `.env.example` para `.env`, instale as dependências e inicie o site. No PowerShell, use `Copy-Item .env.example .env` no lugar de `cp`, se necessário.

```sh
cp .env.example .env
pnpm install
pnpm dev -p 1440
```

```sh
pnpm test
pnpm typecheck
pnpm exec biome check .
pnpm check:comparison
pnpm check:routes
pnpm build
```

## Idiomas e rotas

Todas as páginas públicas têm prefixo: `/pt-BR`, `/en`, `/es`, `/ja`. `/` detecta o idioma e usa português brasileiro como fallback. Os arquivos-fonte em português usam `br` (`messages/br.json`, `content/docs/br/`); o locale público é `pt-BR`. Rotas antigas redirecionam com 308. Os testes cobrem redirecionamentos e alternativas de idioma. `@triooo/named-routes` gera `lib/routes.generated.ts`; rode `pnpm routes` após mudar rotas e `pnpm check:routes` para validar.

## Artigos e referências geradas

Os artigos ficam em `content/docs/{br,en,es,ja}/<slug>.md`, com os mesmos slugs em inglês nos quatro idiomas. A navegação fica em `docGroups`, em `lib/docs.ts`, e os rótulos em `lib/docs-labels.ts`. Atualize os quatro idiomas juntos. Links de artigos usam `/docs/<slug>`; o renderizador adiciona o locale. Capturas usam `![legenda](screenshot:<nome>)` em uma linha própria; avisos usam `> [!NOTE]` ou `> [!WARNING]`. As tabelas de CLI e frameworks usam `<!-- generated:cli -->` e `<!-- generated:frameworks -->`.

Na raiz, `make cli-catalog` regenera os catálogos de CLI, frameworks e projetos iniciais pelo Rust e copia os quatro changelogs. Para alterar somente o changelog, rode `node apps/docs/scripts/sync-changelogs.mjs`; `--check` confere as cópias sem gravar. As fontes são `CHANGELOG.md`, `CHANGELOG.pt-BR.md`, `CHANGELOG.es.md` e `CHANGELOG.ja.md`. As cópias do site ficam em `generated/`, e as do desktop em `../desktop/src/generated/`; ambos exibem o idioma escolhido. Preserve títulos de releases, versões, datas e marcadores de estado nas traduções.

## Ambiente e publicação

`NEXT_PUBLIC_GITHUB_REPOSITORY_URL` deve ser a URL HTTPS da raiz de um repositório GitHub, sem credenciais, query ou fragmento. Links, issues, instruções de clone e API de releases derivam dela. `NEXT_PUBLIC_SITE_URL` indica a URL pública do site. Variáveis públicas são incorporadas no build; reinicie e gere outro build após alterá-las. Nunca versione `.env` ou `.env.local`.

Na Vercel, use Root Directory `apps/docs` com acesso a arquivos fora dela: `next.config.ts` lê o `Cargo.toml` da raiz. Configure no build as variáveis de `.env.example`. O job `docs` de `.github/workflows/ci.yml` executa lint, typecheck, testes, comparativo, rotas e build de produção.

## Privacidade, segurança e apoio

As configurações de hospedagem e contato são `SITE_HOSTING_PROVIDER`, `SITE_PRIVACY_CONTACT_URL`, `SITE_SECURITY_REPORT_URL` e `SITE_SECURITY_REPORTING_CONFIRMED`. O relato de segurança exige contato válido e confirmação explícita; sem isso, a rota não fica disponível nem aparece no sitemap. `SITE_HOSTING_LOG_RETENTION` é opcional e deve descrever o comportamento verificado da hospedagem. Não invente prazos de retenção.

`/about`, `/support`, `/privacy`, `/security`, `/contribute` e `/donate` trazem informações públicas do projeto. O apoio é um convite à comunidade; o site não processa doações. Downloads dependem de arquivos realmente publicados. Afirmações do comparativo precisam corresponder às evidências no repositório (`pnpm check:comparison`).

## Manutenção da documentação

Mantenha artigos, referências geradas e traduções de acordo com o comportamento implementado. O manual inclui diagnóstico, atualizações e desinstalação. Em uma remoção completa, desfaça DNS e confiança da CA enquanto o daemon ainda está disponível. Gere capturas pelo mock do desktop nos quatro idiomas e nos dois temas. Não substitua licenças ou atribuições de terceiros por traduções dos originais oficiais.
