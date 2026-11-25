# Histórico de alterações

[English](CHANGELOG.md) · [Português (Brasil)](CHANGELOG.pt-BR.md) · [Español](CHANGELOG.es.md) · [日本語](CHANGELOG.ja.md)

A primeira versão pública do LocDev é a versão inicial 0.19.0: app desktop 0.19.0 com daemon e CLI 0.18.0. As versões de 0.1.0 até ela passaram por testes internos e com usuários selecionados e estão reunidas nesta versão inicial. A partir daqui, cada release recebe sua própria entrada.

O daemon/CLI e o app desktop têm numeração separada: mudanças somente na interface ou no shell alteram a versão do app; mudanças no daemon ou na CLI alteram a versão deles. O título identifica ambas quando são diferentes.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/). Seções: Adicionado, Alterado, Corrigido, Segurança e Mudanças incompatíveis. A disponibilidade dos instaladores depende das [releases publicadas](https://github.com/trbros/locdev/releases). [Documentação completa](https://locdev.app/pt-BR/docs).

## [0.19.2] - 2026-11-25

App 0.19.2 com daemon e CLI 0.18.2.

### Adicionado

- Os instaladores ganharam arte própria, feita a partir do logo: um ícone de
  instalador com um selo de download e as imagens de boas-vindas e das
  páginas no Windows (NSIS e MSI), e uma janela com uma seta para Aplicativos
  no `.dmg` do macOS.
- Cópias instaladas pelo `.deb` ou `.rpm` se atualizam sozinhas: o app
  instala o pacote da release com a ferramenta de pacotes do sistema, que
  pede sua senha (`pkexec`). Uma cópia instalada antes disso precisa de uma
  atualização manual.
- Ferramentas → Instaladores: comandos de instalação para Python, pipx e Git
  no Windows e no macOS.
- Projetos novos criados pelo app vão por padrão para `~/LocDev/Sites`
  (criada no primeiro uso).
- Remover um site pode também mover a pasta para a lixeira (uma opção no
  diálogo, `locdev remove <nome> --delete-files`); recusado para uma pasta
  que contém outros sites, a pasta pessoal ou a raiz de um disco.
- Um site cuja pasta é apagada fora do LocDev é parado e removido em
  segundos, com uma notificação (não um num disco ou compartilhamento
  desconectado).

### Alterado

- Novo projeto: escolher um framework abre o formulário numa tela própria;
  ao terminar, o site inicia sozinho e o botão dele mostra Rodando.
- Os instaladores rodam sem as opções que pulam as perguntas deles
  (`--no-interaction` no `laravel new`, no Composer e no `artisan migrate`,
  `--yes` no `create-next-app` e no `create-astro`, `--defaults` no
  `ng new`): o que as escolhas do LocDev não respondem, o instalador
  pergunta no terminal, onde aparecem a saída e os erros.
- O Bun é instalado pelo script dele (em `~/.bun/bin`) em vez de
  `npm install -g`, que exige administrador com um Node.js do sistema. O
  Rails é instalado por usuário quando a pasta de gems do Ruby não aceita
  escrita.

### Corrigido

- Next.js em HTTPS: rewrites do middleware, como o prefixo de idioma do
  next-intl, respondiam 500 (`Failed to proxy … EPROTO … wrong version
  number`). O proxy informa ao servidor de desenvolvimento do Next que a
  requisição veio por HTTP.
- Switches: clicar no texto de um switch podia alternar outro ou nenhum em
  alguns motores web; agora o texto alterna o próprio switch. Os selects do
  Novo projeto mostravam a opção escolhida em inglês.
- O pnpm 11 e posteriores não iniciavam (`SyntaxError: Invalid or
  unexpected token`): o LocDev rodava pelo Node.js o arquivo provisório que
  vem no pacote. Agora ele instala o binário nativo do pnpm, conferido no
  registro como o pacote, e reinstala as cópias quebradas quando o daemon
  inicia.
- O aviso de atualização mostrava "Invalid Date" ao lado da versão.
- Ao adicionar um site, clicar em "Iniciar com o LocDev" alternava
  "Iniciar agora" (e clicar num arquivo de um commit marcava o primeiro).
- Novo projeto: escolher um framework rola até os campos dele.
- Ferramentas → Instaladores: o Symfony CLI aparecia como não instalado
  depois do instalador (ele foi para `~/.config/symfony-cli/bin`);
  ferramentas das pastas dos instaladores passavam na frente dos runtimes
  do LocDev; a lista agora verifica de novo quando uma instalação termina.
- Textos que apareciam só em inglês: frameworks, opções e ferramentas em
  Novo projeto e Instaladores, os passos do plano, o aviso do cofre de
  segredos, a página da CLI e rótulos em todo o site.
- Sites PHP abertos em HTTPS (Laravel, Symfony, WordPress, Bedrock, PHP
  puro) apontavam os próprios assets, fontes e a toolbar do Symfony com
  `http://`, e o navegador os bloqueava como _mixed content_. Todo site PHP
  agora carrega o `auto_prepend_file` do LocDev, que dá às requisições que
  chegaram ao proxy por HTTPS as variáveis de servidor que um servidor TLS
  define (`HTTPS`, `REQUEST_SCHEME`, `SERVER_PORT`). Nenhum arquivo do
  projeto muda.
- Numa requisição HTTPS, um redirecionamento para `http://` no mesmo host
  passa a apontar para `https://`, então apps com URL configurada em
  `http://` (o `WP_HOME` do Bedrock) não levam mais o navegador de volta ao
  HTTP.
- Laravel com o auxiliar Vite: no Vite 8.3 ou mais novo, a página carregava
  os assets de `https://127.0.0.1:<porta>`, que o certificado do Vite não
  cobria. O certificado agora inclui `localhost`, `127.0.0.1` e `::1`.
- Nuxt 4.1 ou mais novo: o selo de erros do modo dev (`/__nuxt_dev__/…`)
  respondia 403 em `<site>.test`.
- Django: projetos novos respondiam 400 (`DisallowedHost`) em
  `<site>.test`. O LocDev agora acrescenta `.test` ao `ALLOWED_HOSTS` e o
  protocolo do proxy (`SECURE_PROXY_SSL_HEADER`) às configurações dos
  projetos novos, dentro de `if DEBUG:`; a nota para projetos existentes
  diz o que acrescentar.
- ASP.NET Core: os assets estáticos respondiam 500, porque o `dotnet run
  --no-launch-profile` rodava em Production. O LocDev define
  `ASPNETCORE_ENVIRONMENT=Development`, a menos que o site defina.
- NestJS: a criação do projeto parava nas perguntas novas do Nest CLI
  (`@nestjs/observe`, skills de agentes).
- Bedrock: era detectado como PHP genérico e servido da raiz do projeto em
  vez de `web/`. Agora é um site WordPress servido de `web/`, e projetos
  novos recebem um `.env` com o `WP_HOME` do site.

## [0.19.1] - 2026-11-24

App 0.19.1 com daemon e CLI 0.18.1.

### Alterado

- As URLs dos sites sempre usam `.test`. Enquanto `*.test` ainda não
  resolve, `<site>.localhost` continua funcionando como alternativa.

### Corrigido

- Windows: quando outro programa (como o Acrylic DNS Proxy) ocupa a porta 53,
  o servidor DNS agora informa qual é, em vez de dizer que precisa de
  privilégios elevados, e o Doctor sugere liberar a porta.
- Windows: recusar o pedido de administrador é informado como cancelado, e
  não mais como uma alteração bem-sucedida.

## [0.19.0] - 2026-11-24

Versão inicial. App 0.19.0 com daemon e CLI 0.18.0, para Linux (x86_64 e ARM64), macOS (Apple Silicon e Intel) e Windows, com código aberto sob a licença Apache 2.0.

### Adicionado

#### Base

- Um daemon (`locdevd`) concentra as operações; a CLI (`locdev`) e o app desktop (Tauri) usam o mesmo protocolo. As operações do app têm comandos na CLI, e `locdev` sem argumentos abre uma interface de terminal em tela cheia (`ctrl+p` acessa páginas, sites e comandos).
- Execução nativa, sem exigir Docker ou WSL. Proxy reverso HTTP, HTTPS e WebSockets, DNS para `*.test` (systemd-resolved, `/etc/resolver`, NRPT no Windows) e CA local com restrições de nomes, certificados por site e subdomínios profundos. `locdev doctor` verifica a configuração, e o LocDev pode iniciar no login.
- Alterações do sistema mostram um plano e pedem confirmação. Tudo o que o LocDev grava no sistema recebe uma marca; a desinstalação remove somente o que foi marcado.

#### Sites

- Detecção por 46 definições de frameworks: Laravel, WordPress, Symfony, Next.js, Nuxt, Astro, SvelteKit, Vite, NestJS, Django, Flask, FastAPI, Rails, Go, Rust, .NET e outros. As definições são assinadas e podem ser atualizadas sem uma nova versão do app.
- Servidores de desenvolvimento supervisionados, com logs, reinício com espera progressiva, CPU e memória. O Vite auxiliar do Laravel só inicia após sua decisão; processos como filas, Horizon, scheduler e Reverb são oferecidos por framework.
- Sites estáticos (`index.html`) no servidor do LocDev: somente loopback, URLs limpas, `404.html`, sem exigir Python ou Node.
- Diretórios monitorados: projetos dentro de uma pasta monitorada viram sites, acompanhando a criação e remoção das pastas.
- Subdomínios por site, com regras para encaminhar a outra porta ou processo (`locdev subdomain`, `locdev route`).
- Abas de sites com barra de endereço, menu compartilhado entre o botão ⋯ e o clique direito, e editor de código preferido: VS Code, Cursor, Windsurf, VSCodium, Zed, Sublime Text e JetBrains.
- Métricas por site (aba Métricas, `locdev metrics`): classes de status, latências p50/p95/p99, requisições por minuto, caminhos mais acessados e requisições recentes, contadas pelo proxy e mantidas somente em memória.
- Laravel Tinker (aba Tinker, `locdev tinker`): execução de código ou console interativo com o PHP e o ambiente do site.

#### Runtimes

- Versões de Node.js e PHP por site (fixada, padrão ou do sistema), além de npm, pnpm, Yarn, Bun e Composer. Downloads são instalados após conferir o checksum publicado pela fonte: Node.js do nodejs.org e PHP do StaticPHP (Linux/macOS) e windows.php.net (Windows).
- Detecção de versões já instaladas por apt, Homebrew, nvm, fnm e PATH, sem remover versões do sistema. As versões padrão também podem ser usadas no terminal (`locdev runtime terminal on`).
- Configuração do PHP por versão: memória, uploads, limites de tempo, OPcache e outras opções. Cada mudança tem revisão por diff, backup, validação pelo PHP e reversão se o PHP a rejeitar.
- Extensões do PHP: módulos carregados por cada PHP, arquivos da pasta de extensões ainda não carregados e, por site, extensões obrigatórias e recomendadas pelos pacotes (`locdev runtime modules`, `locdev runtime extensions`).

#### Novos projetos e ferramentas

- Novos projetos pelos instaladores oficiais quando disponíveis, com prévia do comando e execução em um terminal no app: Laravel, Symfony, Bedrock, WordPress, Next.js, Vite, Vue, Angular, Nuxt, Astro, SvelteKit, NestJS, Django, Flask, FastAPI, Rust (Axum e Actix Web), Rails e ASP.NET Core. Opcionalmente, cria um banco e cadastra a pasta como site. Stacks sem gerador adequado recebem arquivos iniciais mínimos.
- Laravel faz as perguntas de `laravel new`, na mesma ordem: starter kit, stack, provedor de autenticação, componentes Livewire em arquivo único e equipes.
- Instaladores no app: PHP, Composer, Node.js e pnpm como runtimes verificados; instalador Laravel, Symfony CLI, Bun, uv, Poetry, rustup, WP-CLI, Git e SDK .NET com o comando documentado pelo publicador para o sistema.

#### Serviços e dados

- Instâncias gerenciadas de PostgreSQL, MariaDB, MySQL, MongoDB, Redis e Valkey, cada uma com dados, porta, logs e backups próprios. Downloads diretos usam o SHA-256 publicado pela fonte. Outras instalações, como MySQL 8.0/8.4, Redis e mais versões de PostgreSQL/MongoDB, podem usar Homebrew, apt ou winget pelo app.
- Database Manager (`locdev db`): tabelas, linhas, SQL com verificações de comandos destrutivos e navegador de chaves Redis/Valkey. As credenciais ficam no daemon.
- Variáveis de ambiente e secrets compartilhados, injetados nos processos sem editar o `.env`, com cofre cifrado.
- Mailbox: servidor SMTP somente de captura em loopback, com integração por framework.
- Email Studio: edição visual de templates, Mailables e componentes, preservando o Blade fora dos blocos gerenciados byte a byte, com prévias e histórico.
- Config Files: formulários para opções conhecidas dos arquivos de configuração do projeto, edição de JSON e JS/TS sem executar código do projeto, histórico e análise.

#### Código, CI e deploy

- Git no app: alterações, commits, branches, worktrees (cada branch como site próprio), stashes e remotos. Operações destrutivas mostram as perdas, exigem frase de confirmação e senha opcional e fazem backup antes.
- GitHub: conta no keyring do sistema, issues, pull requests e execuções de workflows.
- Editor de GitHub Actions: templates por stack, edição visual e YAML. Verificações de segurança e boas práticas com sugestões de correção. Instalação de actionlint pelo app e versões locais de cada workflow.
- Dependências: atualizações, compatibilidade e vulnerabilidades pelo OSV, com planos de atualização confirmados pelo usuário.
- Assistente de deploy: plataformas ordenadas por compatibilidade, checagens de preparo e geração de `render.yaml`/`netlify.toml`; deploy, rollback e logs da Vercel/Netlify pelas APIs públicas.
- Túneis: compartilhamento de sites em execução com Cloudflare Quick Tunnel, com proteção opcional por e-mail.

#### Depuração e app

- Debug Center: exceções, logs de erro, respostas HTTP 5xx, falhas de processos e `dump()`/`dd()` do PHP, agrupados por problema com causas prováveis. Secrets reconhecíveis são ocultados antes de armazenar os eventos.
- Tarefas em segundo plano e notificações no app, no sistema e no ícone da bandeja, com modo offline e configuração por automação.
- App em português brasileiro, inglês, espanhol e japonês, temas claro/escuro, barra de título e barras de rolagem próprias, atalhos editáveis e verificações de acessibilidade WCAG 2.2 AA nas telas.
- Atualizações consultadas nas releases do GitHub e instaladas por escolha do usuário, com verificação da chave de assinatura do LocDev. CLI avulsa com `install.sh`/`install.ps1` e conferência de `SHA256SUMS`.
- Site de documentação em quatro idiomas, com tutoriais, referência da CLI gerada do código e comparação com outras ferramentas conferida contra o repositório.
