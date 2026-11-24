# Configuración

## `config.toml`

Se crea con los valores predeterminados en la primera ejecución. Después de editarlo, ejecuta `locdev restart`.

```toml
version = 1

[ports]
start = 8100        # rango usado para los puertos de los sitios
end = 8999

[proxy]
enabled = true
http_port = 80      # se usa si está libre y permitido
fallback_port = 7080

[dns]
enabled = true
port = 10053        # 53 en Windows (exigencia de NRPT)
addresses = ["127.0.0.1"]   # en Windows: ["127.0.0.1", "127.53.0.1"], el primero libre

[tls]
enabled = true
https_port = 443
fallback_port = 7443

[logs]
lines = 2000        # líneas guardadas en memoria por proceso
file_mb = 5         # tamaño de logs/sites/<sitio>.log antes de rotar

[mail]
enabled = true      # el SMTP de captura del Mailbox
port = 1025         # si está ocupado (Mailpit usa el mismo), prueba hasta port + 9

[env]
key_store = "keyring"   # o "file": env-vault.key (0600) junto a este archivo

[deploy]
token_store = "keyring"      # o "file": tokens de las plataformas en deploy-tokens.toml (0600)

[updates]
repo = "trbros/locdev"       # releases que lee `locdev update` (sin token)

[runtimes]
# php_flavor = "bulk"      # extensiones de PHP en Linux/macOS: bulk o common
# php_static_url = "…"     # mirror de dl.static-php.dev/v3/php-bin
# php_windows_url = "…"    # mirror de downloads.php.net/~windows/releases

[definitions]
# url = "…"                # solo para forks: otra fuente de definiciones de frameworks
# public_key = "…"
```

Los valores inválidos (por ejemplo, `ports.start` por debajo de 1024) impiden que el daemon arranque, con un mensaje que señala el archivo y el campo.

> [!NOTE]
> En Linux, los puertos por debajo de `net.ipv4.ip_unprivileged_port_start` (normalmente 1024) requieren privilegios. Si no se puede usar el puerto 80 o el 443, LocDev pasa al puerto de respaldo y el `doctor` sugiere `setcap cap_net_bind_service` en `locdevd`.

## `settings.toml`

Las configuraciones que la aplicación y la CLI cambian en tiempo real (sin reiniciar), escritas por el daemon. Prefiere **Settings → Automations**, **Notifications → Preferences** o `locdev settings set`.

```toml
[automation]
audit = "daily"            # off, manual, 6h, 12h, daily, weekly
concurrency = 2            # jobs a la vez
requests_per_second = 8    # peticiones a los registros, sumando todos los jobs
definitions = true         # comprobación diaria de las definiciones de frameworks
updates = true             # aviso diario de una nueva release de LocDev (no se instala nada)

[observe]
enabled = true             # Debug Center: recopilar eventos de los sitios
retention_days = 7         # días guardados en data/observe
max_mb = 100               # espacio máximo; los días más antiguos salen primero
logs_from = "warn"         # líneas de log normales desde: info, warn o error

[notifications]
native = true              # mostrar también por el sistema operativo

[notifications.categories]  # dependency_update, security, process_failure, runtime,
dependency_update = true    # certificate, job, workflow, action, mail
security = true

[network]
offline = false            # ningún uso de red en segundo plano

[integrations]
github_client_id = ""      # la OAuth App del device flow (opcional)
github_watch_runs = true   # avisar de fallos de workflow

[env]
conflicts = "active_files" # o "any_file": omitir la clave si cualquier .env* la tiene
```

## `projects.toml`

El registro de los sitios. Se puede editar a mano con el daemon detenido, pero lo normal es usar `locdev add/edit/remove` o la aplicación.

```toml
version = 1

[[project]]
name = "blog"
domain = "blog.test"
path = "/home/tu/Code/blog"
kind = "next"
command = "pnpm run dev --hostname 127.0.0.1 --port {port}"
port = 8101
auto_start = true
```

## `parked.toml`

Las [carpetas monitoreadas](/docs/parked-folders) y las reglas de cada una. Lo normal es usar `locdev park` o la app.

```toml
version = 1

[[dir]]
path = "/home/tu/Development"
enabled = true
depth = 1
exclude = ["archivo-*"]
ignored = ["borrador"]
on_new = "register"        # o "suggest"
on_missing = "unregister"  # o "keep"
```

Un sitio que vino de una carpeta monitoreada guarda esa carpeta en `projects.toml` (`parked = "/home/tu/Development"`).

También quedan en `data/state/`: la cola de jobs (`jobs.json`), las notificaciones (`notifications.json`), la caché de los registros (`registry-cache.json`) y el resultado de la última comprobación de cada sitio (`deps/<sitio>.json`). Se escriben de forma atómica; un archivo dañado se guarda al lado (`.broken`) en lugar de borrarse.

Los archivos con campo `version` (o `format`) siguen la misma regla: una versión más nueva de LocDev migra el formato. Una versión más antigua se niega a leer un archivo escrito por una más nueva, en lugar de corromperlo.

## Dónde están los archivos

|                  | Linux                                  | macOS                                             | Windows                             |
| ---------------- | -------------------------------------- | ------------------------------------------------- | ----------------------------------- |
| Configuración    | `~/.config/locdev`                     | `~/Library/Application Support/dev.locdev.locdev` | `%APPDATA%\locdev\locdev\config`    |
| Datos (CA, logs) | `~/.local/share/locdev`                | `~/Library/Application Support/dev.locdev.locdev` | `%LOCALAPPDATA%\locdev\locdev\data` |
| Socket IPC       | `$XDG_RUNTIME_DIR/locdev/locdevd.sock` | `…/run/locdevd.sock`                              | `\\.\pipe\locdev-<usuario>-<hash>`  |

Con `LOCDEV_HOME=/ruta`, todo va a `config/`, `data/`, `logs/` y `run/` dentro de esa ruta.
