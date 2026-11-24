# Configuration

## `config.toml`

Created with default values on first run. After editing it, run `locdev restart`.

```toml
version = 1

[ports]
start = 8100        # range used for the sites' ports
end = 8999

[proxy]
enabled = true
http_port = 80      # used if free and allowed
fallback_port = 7080

[dns]
enabled = true
port = 10053        # 53 on Windows (an NRPT requirement)
addresses = ["127.0.0.1"]   # on Windows: ["127.0.0.1", "127.53.0.1"], the first free one

[tls]
enabled = true
https_port = 443
fallback_port = 7443

[logs]
lines = 2000        # lines kept in memory per process
file_mb = 5         # size of logs/sites/<site>.log before it rotates

[mail]
enabled = true      # the Mailbox's capture SMTP
port = 1025         # if taken (Mailpit uses the same one), tries up to port + 9

[env]
key_store = "keyring"   # or "file": env-vault.key (0600) next to this file

[deploy]
token_store = "keyring"      # or "file": platform tokens in deploy-tokens.toml (0600)

[updates]
repo = "trbros/locdev"       # releases read by `locdev update` (no token)

[runtimes]
# php_flavor = "bulk"      # PHP extensions on Linux/macOS: bulk or common
# php_static_url = "…"     # mirror of dl.static-php.dev/v3/php-bin
# php_windows_url = "…"    # mirror of downloads.php.net/~windows/releases

[definitions]
# url = "…"                # forks only: another source of framework definitions
# public_key = "…"
```

Invalid values (for example `ports.start` below 1024) keep the daemon from starting, with a message that points to the file and the field.

> [!NOTE]
> On Linux, ports below `net.ipv4.ip_unprivileged_port_start` (usually 1024) need privileges. If port 80 or 443 can't be used, LocDev falls back to the fallback port and `doctor` suggests `setcap cap_net_bind_service` on `locdevd`.

## `settings.toml`

The settings the app and the CLI change at runtime (without restarting), written by the daemon. Prefer **Settings → Automations**, **Notifications → Preferences** or `locdev settings set`.

```toml
[automation]
audit = "daily"            # off, manual, 6h, 12h, daily, weekly
concurrency = 2            # jobs at the same time
requests_per_second = 8    # registry requests, across all jobs
definitions = true         # daily check of the framework definitions
updates = true             # daily notice of a new LocDev release (nothing is installed)

[observe]
enabled = true             # Debug Center: collect events from the sites
retention_days = 7         # days kept in data/observe
max_mb = 100               # disk space; the oldest days go first
logs_from = "warn"         # plain log lines from: info, warn or error

[notifications]
native = true              # also show through the operating system

[notifications.categories]  # dependency_update, security, process_failure, runtime,
dependency_update = true    # certificate, job, workflow, action, mail
security = true

[network]
offline = false            # no background network use

[integrations]
github_client_id = ""      # the device flow's OAuth App (optional)
github_watch_runs = true   # warn about workflow failures

[env]
conflicts = "active_files" # or "any_file": skip the key if any .env* has it
```

## `projects.toml`

The site registry. It can be edited by hand with the daemon stopped, but the usual way is `locdev add/edit/remove` or the app.

```toml
version = 1

[[project]]
name = "blog"
domain = "blog.test"
path = "/home/you/Code/blog"
kind = "next"
command = "pnpm run dev --hostname 127.0.0.1 --port {port}"
port = 8101
auto_start = true
```

## `parked.toml`

The [parked folders](/docs/parked-folders) and each one's rules. Normally changed with `locdev park` or the app.

```toml
version = 1

[[dir]]
path = "/home/you/Development"
enabled = true
depth = 1
exclude = ["archive-*"]
ignored = ["scratch"]
on_new = "register"        # or "suggest"
on_missing = "unregister"  # or "keep"
```

A site that came from a parked folder keeps that folder in `projects.toml` (`parked = "/home/you/Development"`).

Also in `data/state/`: the job queue (`jobs.json`), notifications (`notifications.json`), the registry cache (`registry-cache.json`) and the result of each site's last check (`deps/<site>.json`). They're written atomically; a damaged file is kept beside it (`.broken`) instead of deleted.

Files with a `version` (or `format`) field follow the same rule: a newer LocDev migrates the format. An older version refuses to read a file written by a newer one, instead of corrupting it.

## Where files live

|                 | Linux                                  | macOS                                             | Windows                             |
| --------------- | -------------------------------------- | ------------------------------------------------- | ----------------------------------- |
| Configuration   | `~/.config/locdev`                     | `~/Library/Application Support/dev.locdev.locdev` | `%APPDATA%\locdev\locdev\config`    |
| Data (CA, logs) | `~/.local/share/locdev`                | `~/Library/Application Support/dev.locdev.locdev` | `%LOCALAPPDATA%\locdev\locdev\data` |
| IPC socket      | `$XDG_RUNTIME_DIR/locdev/locdevd.sock` | `…/run/locdevd.sock`                              | `\\.\pipe\locdev-<user>-<hash>`     |

With `LOCDEV_HOME=/path`, everything goes into `config/`, `data/`, `logs/` and `run/` inside that path.
