# Assistente de deploy

A aba **Deploy** de cada site (e `locdev deploy` na CLI) analisa a stack e diz em quais plataformas o projeto pode ir para produção, com o que precisa mudar e por quê. A análise é recomendação e preparação: ela não envia nada a nenhuma plataforma, e as variáveis de ambiente aparecem só pelo nome. Para acompanhar e fazer deploys de verdade, conecte a Vercel ou a Netlify ([Plataformas conectadas](#plataformas-conectadas)).

![A aba Deploy: a stack, as plataformas por compatibilidade e o preparo.](screenshot:deploy)

```bash
locdev deploy analyze                 # stack, plataformas, preparo
locdev deploy analyze --json          # tudo, com o link de cada fonte
locdev deploy config render           # mostra o render.yaml que o LocDev escreveria
locdev deploy config render --write   # cria o arquivo (nunca sobre um existente)
```

## O que entra na análise

|                            | De onde vem                                                                                                                                                        |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Framework, versão, runtime | a detecção de stack (versão instalada)                                                                                                                             |
| Versão do runtime          | `engines.node`, `.nvmrc`, `require.php` do composer.json                                                                                                           |
| Renderização               | `output: 'export'` no next.config, `output`/adaptador no Astro, `ssr` no Nuxt, `adapter-static` no SvelteKit…; servidores de aplicação (Laravel, Django, Express…) |
| WebSockets                 | Reverb, socket.io, ws, Channels… instalados, ou um processo de WebSocket ligado no LocDev                                                                          |
| Filas e workers            | Horizon, BullMQ, Celery, Sidekiq…, `QUEUE_CONNECTION` diferente de `sync` no Laravel, ou um worker ligado                                                          |
| Agendador                  | tarefas em `routes/console.php`, node-cron, celery beat…                                                                                                           |
| Bancos e cache             | as conexões que o Database Manager encontra no `.env` (PostgreSQL, MySQL, SQLite, Redis) e drivers de MongoDB                                                      |
| Arquivos em disco          | `FILESYSTEM_DISK` local/public com arquivos em `storage/app/public`                                                                                                |
| Build e start              | scripts do package.json com o gerenciador do lockfile; composer para PHP                                                                                           |
| Variáveis                  | nomes do `.env.example` e do `.env` (valores nunca saem da análise)                                                                                                |

## Plataformas

Vercel, Netlify, Cloudflare Workers, Railway, Render, Fly.io, Laravel Forge, servidor próprio (VPS) e GitHub Pages. Cada uma recebe um veredito:

- **serve**: roda como está; a lista "para este site" diz o que configurar (serviço de worker, cron, banco gerenciado…);
- **precisa de mudanças**: dá, mudando algo (Dockerfile, WebSockets num serviço externo, uploads em object storage, SQLite fora de disco efêmero…);
- **não serve**: a plataforma não roda esse tipo de app (PHP em plataforma serverless, servidor de aplicação no GitHub Pages).

A ordem vem só do veredito e da quantidade de mudanças, **nunca da popularidade**. Cada capacidade e limitação traz o link da documentação da plataforma de onde saiu. A data de compilação aparece na tela. Plataformas mudam (a Vercel, por exemplo, passou a aceitar WebSockets com Fluid compute), então confira a fonte antes de decidir.

## Preparo

Checagens que não dependem da plataforma:

- **`.env` commitado:** é o único resultado `fail`, porque os segredos ficam no histórico do Git. Também avisa quando ele não está no `.gitignore`.
- **`.env.example`:** se falta, ou se não lista todas as variáveis.
- **Lockfile e versões:** lockfile ausente e versão do runtime não declarada.
- **Git:** mudanças não commitadas, que um deploy a partir do Git não leva.
- **Laravel:** `APP_DEBUG=true` local, para lembrar de desligar em produção.
- **SQLite:** como banco principal, o arquivo se perde em plataformas sem disco persistente.

## Arquivos gerados

Só onde o formato é documentado e estável:

| Plataforma | Arquivo                   | Quando                                                                                                                      |
| ---------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Render     | `render.yaml` (Blueprint) | apps Node.js: serviço web, banco Postgres e Key Value ligados por `fromDatabase`/`fromService`, variáveis com `sync: false` |
| Netlify    | `netlify.toml`            | builds estáticos ou SPA (com o redirecionamento para `index.html`)                                                          |

Nenhum valor do `.env` entra no arquivo: os segredos são declarados para a plataforma pedir. O arquivo é mostrado antes e só é criado se ainda for o mesmo que você revisou. **Um arquivo existente nunca é substituído.**

As outras plataformas se configuram pelas próprias ferramentas, e a análise diz como:
- **Fly.io:** `fly launch`.
- **Cloudflare:** `npx @opennextjs/cloudflare migrate` ou `npm create cloudflare`.
- **Railway:** pelo painel, ou por `.railway/railway.ts` aplicado com `railway config plan/apply`. O antigo `railway.json` foi depreciado e deixa de funcionar em 2026-12-01, por isso o LocDev não o gera.
- **Vercel:** detecta o framework sozinha.

## Plataformas conectadas

Com a **Vercel** ou a **Netlify** conectada, a aba Deploy mostra o **projeto vinculado** acima da análise: os deploys com estado, branch, commit e quando, qual está no ar, os domínios e os **nomes** das variáveis (o LocDev nunca pede os valores à plataforma).

![O projeto vinculado: deploys, o que está no ar e o que seria enviado daqui.](screenshot:deploy-linked)

1. **Conectar:** em **Configurações → Integrações → plataformas de deploy → Conectar…**, cole um token. Ele é conferido com a plataforma antes de ser guardado, e fica no **keyring do sistema**; em disco vai só o nome da conta. Vercel: _Account Settings → Tokens_, limitado ao time dono dos projetos (informe o slug do time). Netlify: _User settings → Applications → Personal access tokens_.
2. **Vincular:** na aba Deploy do site, escolha o projeto (Vercel) ou o site (Netlify). O vínculo fica no `projects.toml` (`deploy`).
3. **Fazer deploy:** **Deploy de prévia** ou **Deploy em produção**. A plataforma compila o que o repositório do site **já enviou ao GitHub**. O LocDev recusa quando isso não bate com o código daqui: alterações não commitadas, commits não enviados, HEAD solto, outro repositório, ou produção a partir de uma branch que não é a de produção do projeto.
4. **Acompanhar:** a lista se atualiza sozinha enquanto algo compila. Dá para **cancelar** um deploy em andamento, abrir o **log de build** e **reverter a produção** para um deploy anterior (Vercel: aponta a produção para ele; Netlify: publica de novo aquele deploy).

|                    | Vercel                       | Netlify                                                     |
| ------------------ | ---------------------------- | ----------------------------------------------------------- |
| Deploy             | o commit exato (`gitSource`) | build da branch pela integração Git da Netlify              |
| Cancelar, reverter | sim                          | sim                                                         |
| Log de build       | sim                          | **não há na API pública**: o LocDev abre a página do deploy |
| Variáveis          | nomes e ambientes            | nomes e contextos                                           |
| Domínios           | sim                          | sim                                                         |

Fontes: [API REST da Vercel](https://vercel.com/docs/rest-api) e [API da Netlify](https://open-api.netlify.com/), conferidas em 2026-11-04. Mudar variáveis ainda não é feito pelo LocDev (o painel da plataforma edita). Render, Railway, Fly.io, Laravel Forge e Cloudflare continuam só na análise.

```bash
locdev deploy platforms
locdev deploy connect vercel --team acme        # o token é lido do terminal, não de um argumento
locdev deploy projects vercel
locdev deploy link vercel loja
locdev deploy status
locdev deploy run                                # prévia; --production para produção
locdev deploy logs dpl_…
locdev deploy rollback dpl_…
locdev deploy disconnect vercel
```

Sem keyring (servidor, CI), `[deploy] token_store = "file"` no `config.toml` guarda os tokens em `deploy-tokens.toml` (`0600`).

## Limitações

- A análise é estática: lê arquivos e dependências, não executa o projeto.
- Workers de Node.js e tarefas agendadas fora do Laravel não têm comando conhecido, então o `render.yaml` só os indica em notas.
- PHP, Python e Ruby na Render pedem Dockerfile, e o LocDev não gera um.
