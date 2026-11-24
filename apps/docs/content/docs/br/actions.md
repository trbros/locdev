# GitHub Actions Builder

![Actions builder: templates e arquivos de workflow do site](screenshot:actions-builder)

Em **Ferramentas → Actions builder** você cria e edita os workflows do GitHub Actions de um site. O resultado é salvo em `.github/workflows/` na pasta do site. Para o GitHub executar o workflow, você faz o commit e o push (pela aba Git ou no terminal).

## Templates

Os templates são montados a partir do que o LocDev encontra nos arquivos do projeto. Nada do projeto é executado. Os que combinam com o site aparecem primeiro, marcados com **serve**, com o motivo.

| Template               | O que gera                                                                                                                                                                                                     |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node.js CI             | instala com o gerenciador do projeto (npm, pnpm, Yarn 1 ou Berry, Bun) e roda os scripts `lint`, `typecheck`, `test` e `build` que existirem. A versão do Node vem do `.nvmrc`/`.node-version` ou do `engines` |
| PHP / Laravel CI       | `setup-php` com a versão do `composer.json`, `composer install`, build do front-end quando há `package.json`, e então `php artisan test`, Pest ou PHPUnit                                                      |
| Deploy to GitHub Pages | build e publicação no Pages (dois jobs; só o de deploy tem `pages: write`)                                                                                                                                     |
| Dependency review      | barra pull requests que adicionam dependências com vulnerabilidades conhecidas                                                                                                                                 |
| CodeQL                 | análise estática de JavaScript/TypeScript em push, PR e uma vez por semana                                                                                                                                     |
| Release on tag         | cria uma release com notas geradas quando uma tag `v*` é enviada                                                                                                                                               |
| Docker image           | build do `Dockerfile` em PRs; build e push para o GHCR no branch principal e em tags                                                                                                                           |

Todos começam com `permissions: contents: read` no topo, e cada job recebe só o que precisa.

![Editor visual com as verificações e o YAML ao lado](screenshot:actions-editor)

## Editor visual e YAML

O **editor visual** cobre:

- nome do workflow;
- gatilhos: push e pull request com branches e tags, manual (`workflow_dispatch`), agendamento (`cron`) e release publicada;
- permissões do token, no topo e por job;
- cancelamento de execuções antigas;
- jobs: runner, timeout, ambiente, dependências entre jobs (`needs`) e passos, com nome, ação ou comandos, `with`, `env` e `if`.

Os passos podem ser reordenados, e o menu **Adicionar passo** lista as ações conhecidas na versão atual.

O YAML ao lado é gerado pelo daemon a cada mudança. A aba **YAML** permite editar o texto direto. Para voltar ao editor visual, o arquivo precisa usar só o que o editor cobre. Arquivos com outros recursos (`workflow_dispatch` com `inputs`, `matrix` com `include`, workflows reutilizáveis…) abrem no modo YAML, com o motivo, e nada é descartado.

## Verificações

O LocDev verifica o YAML enquanto você edita:

| Regra                                                                                                                                   | Nível      |
| --------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| sintaxe, `on:`, jobs sem `runs-on` ou sem passos, passo com `uses` e `run` juntos, `needs` inexistente ou em ciclo, cron inválido       | erro       |
| **injeção de script**: `${{ github.event.pull_request.title }}`, `github.head_ref`, corpo de issue/comentário… colados dentro de `run:` | erro       |
| `pull_request_target`/`workflow_run` que faz checkout do código do PR                                                                   | erro       |
| ação sem versão (`uses: dono/acao`)                                                                                                     | erro       |
| ação seguindo um branch (`@main`)                                                                                                       | aviso      |
| sem `permissions:` (o token fica com o padrão do repositório) ou `write-all`                                                            | aviso      |
| versão major mais antiga que a atual de uma ação conhecida                                                                              | aviso      |
| ação de terceiros fixada por tag em vez de SHA; segredo interpolado em `run:`                                                           | informação |

Se o [actionlint](https://github.com/rhysd/actionlint) estiver no `PATH`, ele também roda: o YAML vai pela entrada padrão, nada é gravado, e os achados dele aparecem marcados como _actionlint_. Sem ele, só as regras acima são aplicadas.

**Os erros do LocDev impedem salvar.** Os avisos e os achados do actionlint aparecem, mas não bloqueiam, porque o actionlint também aponta estilo e notas do shellcheck.

## Salvar sem surpresas

**Revisar e salvar…** mostra o diff do que vai ser gravado e as verificações:

- **Arquivo novo:** é criado só se não existir nada com esse nome (criação atômica).
- **Arquivo existente:** é preciso marcar "Replace the existing file". O daemon só substitui se o arquivo ainda tiver o mesmo conteúdo (SHA-256) que você revisou. Se alguém mudou o arquivo nesse meio-tempo, a gravação é recusada e você revisa de novo. A troca é atômica (arquivo temporário + rename).
- Nomes só no formato `nome.yml`/`nome.yaml`. O LocDev não grava fora de `.github/workflows/` nem através de symlinks.

## Versões das ações

Os templates usam a versão major atual de cada ação, de uma lista embutida. **Verificar** consulta a última release de cada uma no GitHub, com a conta conectada se houver, senão anonimamente. O resultado fica em `data/state/action-versions.json`. A consulta respeita o modo offline.

## Na linha de comando

```bash
locdev actions list                     # templates que combinam e arquivos existentes
locdev actions new node-ci              # imprime o workflow e as verificações
locdev actions new php-ci --save        # cria o arquivo (pergunta antes; nunca substitui)
locdev actions validate                 # verifica todos os workflows; sai com 1 se houver erro
locdev actions versions --refresh       # consulta o GitHub
```

Para substituir um arquivo que já existe, use o app, que mostra o diff.

## Verificação de todos os workflows

**Ferramentas → GitHub Actions** verifica todos os arquivos de `.github/workflows/` ao abrir (`locdev actions validate` faz o mesmo no terminal). Além de estrutura e segurança, o LocDev sugere o que a comunidade recomenda para evitar execuções que falham ou travam, cada sugestão com o YAML que a corrige:

| Verificação            | Por quê                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------- |
| `missing-setup`        | `pnpm`, `bun` ou `deno` num `run:` sem a action de setup: eles não estão nos runners do GitHub |
| `practice-timeout`     | Um job sem `timeout-minutes` roda por até 6 horas quando um passo trava                        |
| `practice-concurrency` | Workflows de pull request sem `concurrency:` mantêm execuções já superadas                     |
| `practice-cache`       | `setup-node` ou `setup-python` sem `cache:` baixam tudo a cada execução                        |
| `practice-npm-ci`      | `npm install` pode mudar o lockfile no runner; `npm ci` instala exatamente o que ele diz       |
| `practice-composer`    | `composer install` no CI deve usar `--no-interaction --no-progress`                            |
| `practice-credentials` | `actions/checkout` deixa o token no `.git/config` sem `persist-credentials: false`             |

Sugestões nunca bloqueiam o salvamento; só os erros do próprio LocDev bloqueiam. Os templates do LocDev já seguem todas elas.

### actionlint

Sem o actionlint instalado, **Instalar actionlint** (ou `locdev actions install-actionlint`) o baixa da release mais recente do rhysd/actionlint no GitHub para a pasta de dados do LocDev, só quando o SHA-256 do arquivo de checksums da release e o digest do próprio GitHub concordam. A validação passa a usá-lo.

### Versões locais

O LocDev guarda uma cópia do workflow quando lê um arquivo alterado fora dele, antes de substituí-lo e depois de salvar (as últimas 50 por arquivo, em `data/state/workflow-history/`). O botão de histórico ao lado de cada arquivo lista as versões; abrir uma a carrega no editor, e salvá-la passa pela revisão de sempre.

```bash
locdev actions history ci.yml
locdev actions history ci.yml --show 1763900000000-saved
```
