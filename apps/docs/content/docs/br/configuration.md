# Configuração

## `config.toml`

Criado com os valores padrão na primeira execução. Depois de editá-lo, rode `locdev restart`.

```toml
version = 1

[ports]
start = 8100        # faixa usada para as portas dos sites
end = 8999

[proxy]
enabled = true
http_port = 80      # usada se estiver livre e permitida
fallback_port = 7080

[dns]
enabled = true
port = 10053        # 53 no Windows (exigência do NRPT)
addresses = ["127.0.0.1"]   # no Windows: ["127.0.0.1", "127.53.0.1"], o primeiro livre

[tls]
enabled = true
https_port = 443
fallback_port = 7443

[logs]
lines = 2000        # linhas mantidas em memória por processo
file_mb = 5         # tamanho de logs/sites/<site>.log antes de girar

[mail]
enabled = true      # o SMTP de captura do Mailbox
port = 1025         # se ocupada (o Mailpit usa a mesma), tenta até port + 9

[env]
key_store = "keyring"   # ou "file": env-vault.key (0600) ao lado deste arquivo

[deploy]
token_store = "keyring"      # ou "file": tokens das plataformas em deploy-tokens.toml (0600)

[updates]
repo = "trbros/locdev"       # releases lidas por `locdev update` (sem token)

[runtimes]
# php_flavor = "bulk"      # extensões do PHP no Linux/macOS: bulk ou common
# php_static_url = "…"     # mirror do dl.static-php.dev/v3/php-bin
# php_windows_url = "…"    # mirror do downloads.php.net/~windows/releases

[definitions]
# url = "…"                # só para forks: outra fonte das definições de frameworks
# public_key = "…"
```

Valores inválidos (por exemplo, `ports.start` abaixo de 1024) impedem o daemon de subir, com uma mensagem que aponta o arquivo e o campo.

> [!NOTE]
> No Linux, portas abaixo de `net.ipv4.ip_unprivileged_port_start` (geralmente 1024) exigem privilégio. Se a porta 80 ou a 443 não puder ser usada, o LocDev cai para a porta de fallback e o `doctor` sugere o `setcap cap_net_bind_service` no `locdevd`.

## `settings.toml`

As configurações que o app e a CLI mudam em tempo real (sem reiniciar), gravadas pelo daemon. Prefira **Configurações → Automações**, **Notificações → Preferências** ou `locdev settings set`.

```toml
[automation]
audit = "daily"            # off, manual, 6h, 12h, daily, weekly
concurrency = 2            # jobs ao mesmo tempo
requests_per_second = 8    # requisições aos registros, somando todos os jobs
definitions = true         # verificação diária das definições de frameworks
updates = true             # aviso diário de nova release do LocDev (nada é instalado)

[observe]
enabled = true             # Debug Center: coletar eventos dos sites
retention_days = 7         # dias guardados em data/observe
max_mb = 100               # espaço máximo; os dias mais antigos saem primeiro
logs_from = "warn"         # linhas de log comuns a partir de: info, warn ou error

[notifications]
native = true              # também mostrar pelo sistema operacional

[notifications.categories]  # dependency_update, security, process_failure, runtime,
dependency_update = true    # certificate, job, workflow, action, mail
security = true

[network]
offline = false            # nenhum uso de rede em segundo plano

[integrations]
github_client_id = ""      # o OAuth App do device flow (opcional)
github_watch_runs = true   # avisar sobre falhas de workflow

[env]
conflicts = "active_files" # ou "any_file": pular a chave se qualquer .env* a tiver
```

## `projects.toml`

O registro dos sites. Pode ser editado à mão com o daemon parado, mas o normal é usar `locdev add/edit/remove` ou o app.

```toml
version = 1

[[project]]
name = "blog"
domain = "blog.test"
path = "/home/voce/Code/blog"
kind = "next"
command = "pnpm run dev --hostname 127.0.0.1 --port {port}"
port = 8101
auto_start = true
```

## `parked.toml`

Os [diretórios monitorados](/docs/parked-folders) e as regras de cada um. O normal é usar `locdev park` ou o app.

```toml
version = 1

[[dir]]
path = "/home/voce/Development"
enabled = true
depth = 1
exclude = ["arquivo-*"]
ignored = ["rascunho"]
on_new = "register"        # ou "suggest"
on_missing = "unregister"  # ou "keep"
```

Um site que veio de um diretório guarda a pasta de origem no `projects.toml` (`parked = "/home/voce/Development"`).

Também ficam em `data/state/`: a fila de jobs (`jobs.json`), as notificações (`notifications.json`), o cache dos registros (`registry-cache.json`) e o resultado da última verificação de cada site (`deps/<site>.json`). São gravados de forma atômica; um arquivo danificado é guardado ao lado (`.broken`) em vez de apagado.

Os arquivos com campo `version` (ou `format`) seguem a mesma regra: uma versão mais nova do LocDev migra o formato. Uma versão mais antiga se recusa a ler um arquivo escrito por uma mais nova, em vez de corrompê-lo.

## Onde ficam os arquivos

|                  | Linux                                  | macOS                                             | Windows                             |
| ---------------- | -------------------------------------- | ------------------------------------------------- | ----------------------------------- |
| Configuração     | `~/.config/locdev`                     | `~/Library/Application Support/dev.locdev.locdev` | `%APPDATA%\locdev\locdev\config`    |
| Dados (CA, logs) | `~/.local/share/locdev`                | `~/Library/Application Support/dev.locdev.locdev` | `%LOCALAPPDATA%\locdev\locdev\data` |
| Socket IPC       | `$XDG_RUNTIME_DIR/locdev/locdevd.sock` | `…/run/locdevd.sock`                              | `\\.\pipe\locdev-<usuário>-<hash>`  |

Com `LOCDEV_HOME=/caminho`, tudo vai para `config/`, `data/`, `logs/` e `run/` dentro desse caminho.
