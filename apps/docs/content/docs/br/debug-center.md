# Debug Center

Em **Ferramentas → Debug Center** (`Ctrl` `Shift` `D`) ou com `locdev debug`, o LocDev junta o que dá errado nos seus sites num lugar só: exceções com stack trace, `dump()`/`dd()` do PHP, linhas de log de aviso e erro, respostas 5xx, processos que caíram, builds que falharam e tarefas do LocDev com erro. Os segredos são escondidos antes de qualquer coisa ser guardada.

![Debug Center: a linha do tempo, uma exceção com a causa provável e a stack trace.](screenshot:debug-center)

## De onde vêm os eventos

| Fonte                                                                         | O que vira evento                                                                                        |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Saída dos processos (site, servidor auxiliar, processos auxiliares, serviços) | Exceções e stack traces, erros de build, avisos do PHP e linhas marcadas como aviso ou erro              |
| `storage/logs` do Laravel (`laravel.log` e o diário do dia)                   | Entradas a partir do nível escolhido, com a exceção e a pilha; só o que é escrito depois que o site sobe |
| Proxy do LocDev                                                               | Respostas 5xx e sites que não responderam (com método, endereço, status e tempo)                         |
| Supervisor                                                                    | Processos que saíram com erro, com as últimas linhas da saída                                            |
| Servidor de dumps do LocDev                                                   | `dump()` e `dd()` dos sites PHP com a captura ligada                                                     |
| Fila de tarefas                                                               | Tarefas que falharam (verificação de dependências, instalações…)                                         |

O LocDev só lê: não altera a configuração de log do projeto nem os arquivos dele.

### Por runtime

| Runtime | O que o LocDev reconhece                                                                                                                                                                                            |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PHP     | `PHP Fatal error: Uncaught …` com `Stack trace:`; `PHP Warning`/`Deprecated` com arquivo e linha; o formato do servidor embutido (`[500]: GET /caminho - Uncaught …`); o log do Laravel; dumps do Symfony VarDumper |
| Node.js | `TypeError: …` com as linhas `at função (arquivo:linha:coluna)`; erros do Vite (`[vite] Internal server error`) e do Next.js (`⨯ …`, `Module not found`); erros do TypeScript (`error TS2322`)                      |
| Python  | `Traceback (most recent call last):` até a linha `Tipo: mensagem`; o formato padrão do `logging` (`ERROR:logger:mensagem`)                                                                                          |
| Rust    | Panics (`thread 'main' panicked at arquivo:linha:coluna:`); erros do cargo (`error[E0425]`); o formato padrão do `tracing`                                                                                          |

Quadros de dependências (`vendor`, `node_modules`, `site-packages`, o registro do cargo) ficam recolhidos na pilha. Outras linhas viram evento a partir do nível escolhido (padrão: avisos), quando trazem um nível explícito (`ERROR`, `[warn]`, `npm ERR!`…).

## A tela

- **Linha do tempo:** os eventos mais novos primeiro, atualizados ao vivo (pause com **Ao vivo**).
- **Problemas:** as repetições agrupadas (mesmo tipo de erro, mesma mensagem sem números e valores, mesmo lugar no código do projeto), com o total e um gráfico por hora das últimas 24 horas.
- **Filtros:** site (à esquerda), tipo, nível e busca por mensagem, tipo de erro, arquivo ou URL.
- **Detalhe:** mensagem, tipo de erro, framework, runtime, arquivo e linha, quando aconteceu, a requisição (método, URL, status, tempo e controller, quando conhecidos), a stack trace, o valor do dump, o contexto e o texto como foi escrito.
- **Causa provável:** para erros conhecidos (banco recusando conexão, porta em uso, `APP_KEY` faltando, classe ou módulo não encontrado, manifesto do Vite ausente, memória do PHP…), uma dica do que costuma causar aquilo.
- **Ações:** **Copiar** (texto para colar numa issue), **JSON**, **Ocorrências** (só as repetições daquele problema), **Exportar** (JSON Lines) e abrir o arquivo e a linha no [editor de código](/docs/editors) (no evento e em cada quadro da pilha dentro do site).

## Dumps do PHP (`dump()` e `dd()`)

![Um dump: o objeto em árvore, com as visibilidades e os valores sensíveis escondidos.](screenshot:debug-dump)

Para sites PHP com o Symfony VarDumper (Laravel, Symfony), escolha o site à esquerda e ligue **Capturar dump() e dd()** (ou `locdev debug dumps <site> on`). O site reinicia. A partir daí os dumps aparecem no Debug Center em árvore (arrays, objetos com propriedades públicas `+`, protegidas `#` e privadas `-`, recursões marcadas) em vez de na página.

Como funciona, sem mudar nenhum arquivo do projeto:

- O LocDev define `VAR_DUMPER_FORMAT=server` e `VAR_DUMPER_SERVER=127.0.0.1:<porta>` **só no ambiente do processo do site**. É o formato `server` do próprio VarDumper, e o Laravel o respeita.
- O servidor embutido do PHP (`php -S`, que é o que `artisan serve` usa) não repassa variáveis de ambiente para `$_SERVER`, que é onde o VarDumper procura. Por isso o LocDev acrescenta, só para esses sites, um `auto_prepend_file` próprio (em `data/php-dumps/`, marcado "Managed by LocDev") que copia essas duas variáveis para `$_SERVER`. Se o seu PHP já tinha um `auto_prepend_file` (do Herd, por exemplo), ele continua rodando logo depois.
- Se o servidor de dumps do LocDev não responder, o VarDumper imprime o dump normalmente, como antes.
- O `dd()` continua encerrando a requisição: o LocDev mostra o valor, mas não preserva o que viria depois.

O LocDev lê o formato de serialização do PHP sozinho, sem executar PHP nem chamar `unserialize`. Os processos auxiliares que o LocDev roda para o site (filas, agendador) recebem o mesmo ambiente, então os dumps deles também chegam, com a linha de comando no contexto.

## Segredos

Antes de guardar ou mostrar qualquer evento, o LocDev esconde (`[redacted]`):

- os valores do `.env` e do `.env.local` do site, menos os que são só configuração (hosts, portas, drivers, nomes, níveis, `true`/`false`, números);
- os valores das [variáveis compartilhadas e secrets](/docs/environment) que o LocDev injeta no site;
- cabeçalhos `Authorization`, `Cookie`, `Set-Cookie` e `X-Api-Key`; tokens `Bearer`; JWTs; `password=`, `token=`, `secret=`, `api_key=` e parecidos em URLs e textos; senhas em URLs (`postgres://user:senha@…`);
- em JSON e em dumps, os valores de chaves com nomes como `password`, `token`, `secret`, `api_key`, `cookie`, `session` ou terminadas em `_key`.

Isso cobre o que é identificável; um segredo sem nome reconhecível, num lugar sem padrão, pode passar. Os eventos ficam só nesta máquina.

## Guarda e limpeza

Os eventos ficam em `data/observe/<site>/<dia>.jsonl`, com os mais recentes também em memória. Em **Configurações → Automações → Debug Center**:

- **Coletar eventos** (ligado por padrão);
- **Guardar por** (padrão: 7 dias) e **Usar no máximo** (padrão: 100 MB): quando um limite é atingido, os dias mais antigos são apagados primeiro;
- **Linhas de log comuns a partir de:** info, avisos (padrão) ou erros.

**Limpar** apaga os eventos de um site (ou de todos), em memória e em disco. As mesmas opções existem no `settings.toml` (`[observe] enabled`, `retention_days`, `max_mb`, `logs_from`).

## Linha de comando

```bash
locdev debug events
locdev debug errors
locdev debug status
```

- `locdev debug events -f` acompanha ao vivo; `--site`, `--kind` (repetível), `--level` e `--search` filtram; `--json` para scripts.
- `locdev debug errors` agrupa as repetições, com o gráfico das últimas 24 horas.
- `locdev debug show <id>` mostra um evento inteiro (pilha, requisição, dump, causa provável).
- `locdev debug export` escreve JSON Lines; `locdev debug clear --site <site>` limpa; `locdev debug dumps <site> on|off` liga e desliga a captura de dumps.

## Limitações

- Validado no Linux: o pipeline inteiro com um site Python real (traceback, 500 no proxy, segredo do `.env` escondido, agrupamento) e com PHP 8.5 + Symfony VarDumper 7.4 de verdade pelo servidor embutido (dump capturado sem aparecer na página, exceção com a requisição). Não foi testado com uma aplicação Laravel completa, nem no Windows e no macOS.
- Os formatos de Node.js, Next.js, Vite, Python e Rust seguem a saída padrão de cada um; logs com formato próprio (JSON estruturado, por exemplo) só viram evento quando trazem um nível explícito.
- O log do Laravel é lido para os canais `single` e `daily` em `storage/logs`; outros destinos (Papertrail, syslog, stderr) não.
- Erros só no navegador (JavaScript do front-end) não passam pelo LocDev.
