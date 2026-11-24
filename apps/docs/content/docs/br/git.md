# Git, GitHub e proteção

## Git local

![Aba Git de um site, com arquivos alterados e o diff](screenshot:git)

A aba **Git** de cada site trabalha no repositório da pasta do site, usando o `git` instalado na máquina (é preciso tê-lo no `PATH`). Ela tem seis partes:

| Parte    | O que faz                                                                                                              |
| -------- | ---------------------------------------------------------------------------------------------------------------------- |
| Changes  | arquivos alterados, separados em _staged_ e não _staged_, com o diff de cada um; _stage_/_unstage_, descartar e commit |
| History  | histórico paginado; cada commit mostra arquivos e diff, e as ações a partir dele                                       |
| Branches | locais e remotas, com upstream e ↑/↓; criar, trocar, fazer merge na atual e apagar                                     |
| Stashes  | guardar as mudanças (com ou sem arquivos não rastreados), aplicar, _pop_ e descartar                                   |
| Remotes  | adicionar, mudar a URL e remover                                                                                       |
| Activity | o que o LocDev executou (e recusou) nesse repositório, e o _reflog_ do HEAD                                            |

Fetch, Pull e Push ficam no topo. O **Pull é sempre `--ff-only`**: ele nunca cria um merge sozinho. Se o branch divergiu, ele falha e você decide (merge, rebase) no terminal ou na aba Branches. Durante um merge com conflitos, um aviso mostra o estado e oferece **Abort merge**.

Todos os comandos rodam sem shell, com argumentos separados, `--end-of-options` antes de nomes vindos do usuário e validação de nomes de branch (`git check-ref-format`). URLs de remoto com `ext::` ou começando com `-` são recusadas. O Git nunca pede senha interativamente (`GIT_TERMINAL_PROMPT=0`; SSH em `BatchMode`): use o agente SSH ou o _credential helper_ que você já usa.

### Voltar atrás: três operações diferentes

No detalhe de um commit, as opções aparecem lado a lado, cada uma explicando o que faz:

| Operação               | O que acontece                                                                                                                         | Histórico                                                          |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| **Reverter**           | cria um commit novo que desfaz aquele                                                                                                  | preservado; seguro em branches compartilhados                      |
| **Restaurar arquivos** | os arquivos marcados voltam à versão daquele commit, como mudanças não commitadas                                                      | intocado                                                           |
| **Reset**              | o branch atual passa a apontar para aquele commit (`soft`: mudanças ficam _staged_; `mixed`: ficam na árvore; `hard`: são descartadas) | os commits posteriores saem do branch (o _reflog_ ainda os guarda) |

Há também **Novo branch aqui**, que só cria um branch naquele ponto.

## Operações destrutivas

![Confirmação de um reset hard, com o que será perdido](screenshot:git-confirm)

Estas operações pedem confirmação explícita antes de rodar:

- descartar mudanças (inclusive apagar arquivos não rastreados);
- `reset` (qualquer modo; o `hard` também lista os arquivos que perdem mudanças);
- _force push_ (sempre `--force-with-lease`; lista os commits do remoto que somem);
- apagar um branch que não foi integrado;
- descartar um stash;
- restaurar arquivos que têm mudanças;
- abortar um merge;
- remover um remoto.

O daemon calcula a prévia: o que será perdido (arquivos, commits, o stash), o que será salvo antes e se a operação está bloqueada (por exemplo, `reset` durante um merge, ou apagar o branch atual). Para confirmar, você digita uma frase que depende da operação (o nome do branch, o SHA curto do destino do reset, `discard`, `drop`…). **Não existe `--yes` que pule essa etapa.** A verificação acontece de novo no daemon, então um cliente não consegue contorná-la.

### Backups

Com **Fazer backup antes** ligado (padrão), antes de uma operação destrutiva o LocDev:

- grava um _snapshot_ das mudanças rastreadas (`git stash create`) e uma ref para o commit de onde o branch sai, em `refs/locdev/backups/<data>-<operação>`;
- copia os arquivos não rastreados que seriam apagados para `~/.local/share/locdev/backups/<site>/<data>/` (pasta de dados do LocDev em cada SO).

Para recuperar: `git log refs/locdev/backups/...` ou `git stash apply <sha>` no terminal, ou crie um branch a partir do _reflog_ na aba Activity. Os backups não são apagados automaticamente.

### Senha do LocDev

![Configurações → Security](screenshot:settings-security)

Em **Settings → Security** (ou `locdev security password`) você pode definir uma senha para as operações destrutivas.

- Só o hash **Argon2id** é guardado, em `security.toml` na pasta de configuração, com permissão `0600`. A senha não fica em memória entre pedidos.
- Depois de 5 erros, novas tentativas ficam bloqueadas por 1 minuto, dobrando a cada erro até 1 hora. O bloqueio sobrevive ao reinício do daemon.
- Trocar ou remover a senha, ou desligar "Pedir a senha"/"Fazer backup antes", exige a senha atual.
- Não há recuperação. Se você esquecer a senha, apague o `security.toml`. Isso remove a proteção, e é coerente com o objetivo: ela protege contra descuidos, não contra você.

> [!WARNING]
> **Limite importante:** a senha protege operações iniciadas **pelo LocDev** (app e CLI). Ela não protege o repositório. O `git` no terminal, o editor ou qualquer outra ferramenta continuam funcionando normalmente.

## GitHub

![Aba GitHub de um site, com execuções de workflow](screenshot:github)

A aba **GitHub** aparece nos sites cujo remoto aponta para o github.com. Ela mostra visão geral (visibilidade, branch padrão, _verificações_ do commit atual), issues, pull requests, execuções de workflow, workflows e releases. Os itens abrem no navegador.

Ações que mudam algo no GitHub sempre pedem confirmação e dizem com qual conta vão rodar:

- **Re-executar as que falharam**: roda de novo os jobs que falharam;
- **Cancelar**: cancela uma execução em andamento;
- **Run workflow**: dispara um workflow com gatilho `workflow_dispatch` num branch ou tag.

### Conectar uma conta

Em **Settings → Integrations** (ou `locdev github connect`), há três formas:

| Forma         | Como                                                                                                                                                             |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Token pessoal | cole um token _fine-grained_ (recomendado, limitado aos repositórios que você escolher) ou clássico. Na CLI, ele é lido da entrada padrão, nunca de um argumento |
| GitHub CLI    | copia uma vez o token de um `gh` já logado (`gh auth token`)                                                                                                     |
| Navegador     | _device flow_ do OAuth. Exige o _client id_ de um OAuth App seu com device flow habilitado; o LocDev não distribui um                                            |

Para o token _fine-grained_, dê leitura em _Metadata_, _Issues_, _Pull requests_ e _Actions_. Para re-executar, cancelar ou disparar workflows, dê também escrita em _Actions_.

O token é validado no GitHub e guardado **só no keyring do sistema**: Secret Service no Linux (GNOME Keyring, KWallet), Keychain no macOS e Gerenciador de Credenciais no Windows. Ele nunca vai para um arquivo nem para um log. Sem um keyring disponível, a conexão falha em vez de cair para um arquivo. O daemon só faz requisições para `api.github.com`, e **Desconectar** apaga o token do keyring.

### Falhas de workflow

Com a conta conectada, a cada 15 minutos o daemon verifica as execuções dos sites com remoto no GitHub e cria uma notificação (categoria _Falhas de workflow_) para cada execução nova que falhou. A primeira verificação só registra o que já existe, para não gerar uma enxurrada de falhas antigas. Isso pode ser desligado em Integrations e para no modo offline.

## Na linha de comando

```bash
locdev git status            # branch, upstream e arquivos alterados
locdev git log -n 50
locdev git branches
locdev git history           # o que o LocDev rodou + reflog
locdev security password     # define/troca a senha
locdev github connect --gh   # usa o token do GitHub CLI
locdev github status
```

A CLI só lê o repositório. Para mudá-lo, use o próprio `git` ou o app.

## Worktrees

**Git → Worktrees** (ou `locdev git worktree`) faz o checkout de uma branch numa pasta própria, ao lado do repositório como `<repo>-<branch>`, para duas branches rodarem ao mesmo tempo.

- **Registrar como site próprio**: a pasta vira um site (`<repo>-<branch>.test`) com porta e processo próprios.
- **Copiar os arquivos .env que faltarem**: os `.env*` não rastreados da pasta principal (que não estão no Git) são copiados; nada é sobrescrito.

```bash
locdev git worktree add feature/login --create --site-for-it --copy-env
locdev git worktree list
locdev git worktree remove ../loja-feature-login
locdev git worktree prune
```

Remover um worktree tem a mesma proteção das outras operações destrutivas do Git: a prévia lista a pasta e cada mudança não commitada, pede o nome da pasta (e a senha do LocDev, quando houver), guarda um snapshot das mudanças rastreadas em `refs/locdev/backups/` e remove o site registrado para aquela pasta. Mudanças não commitadas bloqueiam a remoção, a menos que ela seja forçada. A branch e os commits continuam no repositório.
