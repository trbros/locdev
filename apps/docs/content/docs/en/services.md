# Services: databases and caches

LocDev runs PostgreSQL, MariaDB, MySQL, MongoDB, Valkey and Redis for your projects, without Docker. Each **instance** has its own data folder, port, configuration and log, runs supervised by the daemon (restarting on failure) and listens **only on `127.0.0.1`**.

```bash
locdev service list                              # services, installed versions and instances
locdev service install postgresql                # download and verify the newest version
locdev service create db --service postgres --start
locdev service status
locdev service backup db
locdev service stop postgresql                   # the service name works when there's a single instance
```

## Where the binaries come from

LocDev uses installs that already exist on the machine (on `PATH`, in `/usr/lib/postgresql/*`, in Homebrew, in `Program Files`) **without changing them**, and can download its own versions. A download is only installed when its SHA-256 matches the one the source publishes:

| Service    | Download by LocDev                      | Source                                                                                                                                     |
| ---------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| PostgreSQL | Linux, macOS, Windows                   | official source built by [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) (with provenance attestation) |
| MariaDB    | Linux, Windows                          | MariaDB Foundation (downloads API, SHA-256 per file)                                                                                       |
| MongoDB    | Ubuntu 22.04+/Debian 12, macOS, Windows | MongoDB, Inc. (community server, a `.sha256` next to each file)                                                                            |
| Valkey     | Linux                                   | download.valkey.io (Ubuntu builds)                                                                                                         |
| MySQL      | no                                      | Oracle only publishes MD5, which LocDev doesn't accept: use a system MySQL or MariaDB                                                      |
| Redis      | no                                      | there are no official binaries: use a system Redis or Valkey (compatible)                                                                  |

Downloaded versions live in `~/.local/share/locdev/services/bin/` and can be removed with `locdev service uninstall` (never system ones, and never a version an instance uses).

## Instances

- **Data** in `services/instances/<name>/data`, never mixed between instances; the folder is created with `0700` permissions.
- **Port**: the service's default when free (5432, 3306, 27017, 6379), otherwise the next free one. Two instances never fight over the same port, and Redis and Valkey (which share 6379 by default) get different ports.
- **Access**: `127.0.0.1` only, local users without a password (`locdev` on PostgreSQL, `root` on MariaDB/MySQL). It's a development environment; don't expose these ports.
- **Connection**: `locdev service create` and the **Services** page show the URL and the usual variables (`DATABASE_URL`, `DB_HOST`, `DB_PORT`, `REDIS_URL`…) to copy into the project.
- **Start with LocDev**: `--autostart` or `locdev service autostart db true`. Instances start before sites.
- **Ready** means it really accepts connections; the PID alone isn't enough.

## Backup and removal

`locdev service backup` makes a **cold copy**: it stops the instance, archives the data folder and starts it again. `locdev service restore <instance> <id>` puts a copy back and **moves the current data to `data.before-<time>`**, without deleting.

Removing an instance **keeps the data**. Deleting it needs `--delete-data` and typing the instance's name; backups remain.

## SQLite

SQLite is a file inside the project (`database/database.sqlite`, for example): there's nothing to install, start or stop. To back it up, copy the file while the application isn't writing.

## PHP extensions

`locdev runtime extensions <site>` compares what the project requires with what the site's PHP loads:

- `ext-*` in `composer.json` and in the `composer.lock` packages;
- the framework's requirements (Laravel: ctype, curl, dom, fileinfo, filter, hash, mbstring, openssl, pcre, pdo, session, tokenizer, xml);
- the drivers the `.env` uses (`DB_CONNECTION=pgsql` → `pdo_pgsql`; Redis with the phpredis client → `redis`).

Nothing is installed automatically. The command says how to fix it for that PHP: the apt package (`php8.3-pgsql`), Homebrew's `pecl`, the `php.ini` line on Windows. LocDev's PHP builds are static, with a fixed set of extensions; for one they lack, use a system PHP that has it. The command exits with code 1 when something is missing.

## Limits

- Validated for real on Linux with the system's PostgreSQL 18.6 and Redis 8.10. Managed installation (download, verification and extraction) has tests, but hasn't been exercised end to end yet.
- Upgrading an instance to another major version (PostgreSQL 16 → 17, for example) isn't automatic: create a new instance, export and import the data.
- Windows and macOS haven't been tested yet.

## Other versions from your package manager

Where LocDev can't download a verified build (MySQL and Redis publish no SHA-256), or for more versions, **Services** offers what your system's package manager has. The command is shown first and runs in a terminal in the app:

| System                          | Offered                                                                                                                                                         |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS (and Linux with Homebrew) | `mysql`, `mysql@8.4`, `mysql@8.0`, `redis`, `redis@8.2`, `postgresql@15` to `@17`, `mongodb-community@7.0` and `@8.0` (tap `mongodb/brew`), `mariadb`, `valkey` |
| Debian and Ubuntu               | The distribution's `mysql-server`, `redis-server`, `postgresql` and `mariadb-server` (`sudo` asks in the terminal)                                              |
| Windows                         | MySQL (latest, 8.4 and 8.0) from winget (`Oracle.MySQL`)                                                                                                        |

The package manager verifies what it installs. The result is a system install that LocDev detects (Homebrew's versioned formulae under `opt/` too) and never changes or removes. Redis has no official Windows build: use WSL or Memurai there.

```bash
locdev service packages mysql
locdev service install-package mysql mysql@8.4
```
