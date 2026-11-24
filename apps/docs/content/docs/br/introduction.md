# O que é o LocDev

LocDev é um ambiente de desenvolvimento local **open source** para Linux, macOS e Windows. Ele registra as pastas dos seus projetos, executa os servidores de desenvolvimento e entrega cada um em um domínio próprio, como `blog.test`, com HTTPS válido.

![A tela principal: seus sites à esquerda e o selecionado, com a saída do processo ao vivo, à direita.](screenshot:sites)

## Como ele é organizado

O LocDev tem três peças, e todas falam a mesma língua:

- **`locdevd`**: o daemon. Roda em segundo plano, sem privilégios de administrador, e é dono de tudo: registro de projetos, processos, proxy, DNS e certificados. Os sites continuam no ar mesmo com o app fechado.
- **`locdev`**: a CLI. Controla o daemon pelo terminal e aceita `--json` para automação.
- **O app desktop**: uma interface (Tauri + React) sobre as mesmas operações da CLI.

Nenhuma regra vive só na interface. Se o app consegue fazer algo, a CLI também consegue.

## O que ele faz

- **Sites:** detecção de 46 stacks (Laravel, Next.js, Django, Rails, Vite, Spring, .NET, Go…), servidores de desenvolvimento supervisionados com logs, processos auxiliares (filas, agendador, WebSocket) e início junto com o LocDev. Veja [Sites](/docs/sites) e [Frameworks](/docs/frameworks).
- **Rede:** proxy HTTP/HTTPS por hostname com WebSocket, DNS próprio para `*.test`, [subdomínios e wildcards](/docs/subdomains) e uma CA local com restrição de nome.
- **Runtimes:** versões de Node.js, PHP e gerenciadores de pacotes por site, baixadas com verificação de SHA-256 ([Runtimes](/docs/runtimes)).
- **Ambiente e serviços:** [variáveis compartilhadas e secrets](/docs/environment) injetados nos processos, e instâncias de PostgreSQL, MySQL, MariaDB, MongoDB, Redis e Valkey ([Serviços](/docs/services)).
- **Ferramentas:** [Database Manager](/docs/database), [Mailbox](/docs/mailbox), [Email Studio](/docs/email-studio), [Config Files](/docs/config-files), [assistente de deploy](/docs/deploy), [dependências](/docs/dependencies), [Git e GitHub](/docs/git) e [GitHub Actions](/docs/actions).
- **Diagnóstico:** o [Doctor](/docs/doctor) aponta cada problema com o remédio.

O que mudou em cada versão está no [Changelog](/changelog).

## Princípios

- **Sem paywall:** o ambiente local e seus recursos essenciais são livres.
- **Nativo primeiro:** processos nativos, sem containers.
- **Funciona offline** depois de instalado.
- **Seguro por padrão:** privilégios mínimos e confirmação explícita para qualquer mudança no sistema.
