# Servidores de desenvolvimento

O LocDev roda o servidor de desenvolvimento de cada site como um processo supervisionado pelo daemon: ele sobe com a versão certa de Node.js/PHP, recebe uma porta, fica atrás do proxy em `https://<site>.test` e continua rodando com a janela do app fechada.

## Detecção

Ao registrar uma pasta, o LocDev **lê** os arquivos do projeto (`package.json`, lockfiles, `vite.config.*`, `artisan`, `composer.json`, `wp-config.php`…) para escolher o comando. Ele nunca executa um script do projeto só para descobrir a tecnologia.

| Projeto                            | Servidor principal                                                                                         | Como a porta é passada                        |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Laravel                            | `php artisan serve --no-reload` (sem o `--no-reload`, o Laravel descarta as variáveis que o LocDev injeta) | `--port {port}`                               |
| Laravel + Vite                     | `artisan serve` + **Vite como auxiliar** (com consentimento)                                               | `--port` no Vite                              |
| React/Vue/Svelte + Vite, SvelteKit | o script `dev`                                                                                             | `--port {port} --strictPort --host 127.0.0.1` |
| Next.js                            | o script `dev`                                                                                             | `--hostname 127.0.0.1 --port {port}`          |
| Nuxt, Astro                        | o script `dev`                                                                                             | `--port {port} --host 127.0.0.1`              |
| Node.js genérico                   | `dev` ou `start`                                                                                           | variável `PORT`                               |
| PHP, WordPress                     | `php -S`                                                                                                   | na linha de comando                           |
| HTML/CSS/JS                        | nenhum processo                                                                                            | —                                             |

O Vite ignora a variável `PORT`; por isso o LocDev passa `--port` e `--strictPort` (com a porta ocupada, o Vite falha em vez de pular para outra que o proxy não conhece). O gerenciador de pacotes vem do lockfile (`pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, senão npm). `locdev info` mostra o que foi detectado, e o detalhe do site avisa quando o comando registrado difere do que o LocDev usaria hoje.

## O auxiliar (Vite ao lado do Laravel)

Num projeto Laravel com Vite, o PHP serve as páginas e o Vite serve os assets com hot reload: são dois processos. O LocDev detecta o Vite, mas **não roda nada até você decidir**:

![Pergunta sobre o Vite detectado](screenshot:dev-prompt)

| Opção              | Efeito                                                            |
| ------------------ | ----------------------------------------------------------------- |
| Iniciar agora      | Roda o Vite agora; nas próximas vezes, você inicia quando quiser. |
| Iniciar com o site | Sobe e para junto com o site.                                     |
| Perguntar de novo  | Nada roda; a pergunta continua.                                   |
| Não gerenciar      | O LocDev para de perguntar e nunca roda o Vite.                   |

A decisão pode ser mudada a qualquer momento na aba **Servidor** do site ou pela CLI:

```bash
locdev dev mode auto            # inicia com o site
locdev dev mode manual --start  # agora, e depois só à mão
locdev dev mode never
locdev dev status | start | stop | restart
locdev dev logs -f
```

O auxiliar ganha a própria porta e a própria saída (`locdev logs --dev`). Parar o site para os dois; o Vite recebe SIGTERM (no Windows, o encerramento da árvore), então o `laravel-vite-plugin` apaga o arquivo `public/hot` sozinho.

## Proxy, HMR e HTTPS

- **Hosts permitidos:** o Vite recusa hosts desconhecidos (`.localhost` é sempre aceito). O LocDev passa ao processo a variável `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS=<site>.test,.<site>.test`, que o Vite lê, sem editar o `vite.config` do projeto e sem liberar outros hosts.
- **HMR:** o cliente do Vite conecta no mesmo host e protocolo da página, então o WebSocket passa pelo proxy (`ws://` ou `wss://`). Validado com o Vite 8: o handshake `101` e o `full-reload` chegam pelo proxy ao editar um arquivo.
- **Laravel + Vite em HTTPS:** a página vem de `https://<site>.test` e os assets do Vite. Quando a CA do LocDev é confiável e `*.test` resolve para o LocDev, o processo do Vite recebe `VITE_DEV_SERVER_CERT`, `VITE_DEV_SERVER_KEY` (um certificado do site emitido pela CA local, com a chave legível só pelo seu usuário) e `APP_URL=https://<site>.test`. O `laravel-vite-plugin` então serve os assets em HTTPS, sem _mixed content_. Sem essas condições, o Vite fica em `http://127.0.0.1:<porta>`, que Chrome e Firefox aceitam numa página HTTPS por ser loopback.
- O plugin também libera CORS para origens `*.test`.

Nada disso altera arquivos do projeto: tudo vai só para o ambiente do processo.

> [!WARNING]
> Next.js 15+ pode avisar sobre requisições de desenvolvimento vindas de outra origem. Se o hot reload não funcionar por `<site>.test`, adicione o domínio em `allowedDevOrigins` no `next.config`. O LocDev não altera esse arquivo por você.

## Supervisão

- **Uma instância:** iniciar um site que já roda não sobe outro processo.
- **Porta ocupada:** antes de iniciar, o LocDev verifica a porta e, se ela estiver em uso, diz quem a ocupa (processo e PID).
- **Árvore de processos:** cada site roda no próprio grupo de processos (Unix) ou árvore (Windows); parar derruba tudo o que o comando criou (npm → node, artisan → php). Depois de 8 s sem encerrar, o processo é morto.
- **Reiniciar se falhar:** opcional por site (Configurações do site, ou `locdev edit <site> --restart-on-failure true`), com espera crescente (1 s, 2 s, 4 s…) e no máximo 5 tentativas em 2 minutos.
- **Depois de uma queda do daemon:** o LocDev registra os processos em execução; um daemon novo encerra o que o anterior deixou (a saída não pode ser religada) e o auto-start sobe de novo o que estava marcado.

## Processos e logs

![Ferramentas → Processos e logs](screenshot:processes)

Em **Ferramentas → Processos e logs** (ou `locdev ps`): todos os processos, com PID, porta, runtime, tempo de execução, estado, reinicializações, CPU e memória (somados na árvore do processo). O console mostra stdout, stderr e as notas do LocDev, com filtros por stream e por nível, busca, pausa, auto-scroll e cópia.

O nível (`error`, `warn`, `info`, `debug`) só aparece quando a própria linha o declara: `ERROR:`, `[warn]`, `npm ERR!`, `PHP Fatal error:`, `local.ERROR:` do Laravel. Saída comum fica sem nível, e stderr não é tratado como erro por si só.

A saída fica em memória (2000 linhas por processo) e em disco, em `logs/sites/<site>.log` (gira em 5 MB, guardando um arquivo anterior). Por isso ela continua visível depois que o processo termina e depois de reiniciar o daemon. Os limites ficam em `config.toml`:

```toml
[logs]
lines = 2000
file_mb = 5
```

Isso é diagnóstico pela saída do processo. Depuração interativa (Xdebug, o inspector do Node) não faz parte do LocDev hoje.

## Processos auxiliares

Além do servidor de desenvolvimento e do auxiliar Vite, um site pode ter **processos auxiliares**: workers de fila, schedulers, servidores WebSocket e ferramentas com servidor próprio. Os adaptadores de framework oferecem os que o projeto tem instalado:

| Stack       | Processos oferecidos                                                             | Condição                                                |
| ----------- | -------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Laravel     | Queue worker (`php artisan queue:work`), Scheduler (`php artisan schedule:work`) | sempre                                                  |
| Laravel     | Horizon (`php artisan horizon`)                                                  | `laravel/horizon` no `composer.json`                    |
| Laravel     | Reverb (`php artisan reverb:start`)                                              | `laravel/reverb` no `composer.json`                     |
| Rails       | Solid Queue (`bin/jobs`)                                                         | gem `solid_queue` e `bin/jobs`                          |
| Rails       | Sidekiq (`bundle exec sidekiq`)                                                  | gem `sidekiq`                                           |
| Django      | Celery worker e Celery beat (`celery -A <projeto> …`)                            | `celery` nas dependências; o projeto vem do `manage.py` |
| Qualquer JS | Storybook (`<pm> run storybook --port <porta> …`)                                | `storybook` e um script `storybook`                     |

Como o auxiliar Vite, **nada roda antes de uma decisão**: cada processo oferecido começa como _ask_. Os modos são _manual_ (você inicia), _com o site_ (inicia junto com o site) e _nunca_.

Alternativas que consumiriam os mesmos jobs (Horizon e `queue:work`, Solid Queue e Sidekiq) não rodam juntas: iniciar uma com a outra rodando é recusado, e escolher uma como _with site_ tira a outra desse modo. Workers e schedulers reiniciam sozinhos se caírem (com o mesmo backoff do servidor). Cada processo tem o próprio log, recebe os runtimes e as variáveis compartilhadas do site e para junto com ele.

Também dá para acrescentar processos seus (um worker de outra fila, um watcher); `{port}` no comando recebe uma porta livre:

```bash
locdev process list
locdev process mode queue auto
locdev process start scheduler
locdev process mode queue manual --command "php artisan queue:work --queue=high,default"
locdev process add mailer --command "php artisan queue:work --queue=mail" --label "Mail queue"
locdev process logs queue -f
```

No app, a aba **Servidor** do site lista os processos com o modo, os controles e o console de cada um. Em **Tools → Processes & logs** aparecem os que estão ligados.
