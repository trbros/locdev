# Architecture

```text
          ┌────────────┐     ┌──────────────┐
          │ app (Tauri)│     │  locdev CLI  │
          └─────┬──────┘     └──────┬───────┘
                │  local IPC (JSON)  │
                └─────────┬──────────┘
                    ┌─────▼──────┐
                    │  locdevd   │  unprivileged
                    └─┬───┬───┬──┘
        ┌─────────────┘   │   └──────────────┐
  ┌─────▼──────┐   ┌──────▼──────┐   ┌───────▼──────┐
  │ supervisor │   │ HTTP/S proxy│   │  DNS *.test  │
  │ processes  │   │ SNI + CA    │   │ 127.0.0.1    │
  └────────────┘   └─────────────┘   └──────────────┘
```

## Crates

| Crate               | Responsibility                                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `locdev-core`       | Models, errors, paths, project detection, host parsing and the IPC protocol.                                        |
| `locdev-ipc`        | Local transport, client and daemon launcher (shared by the CLI and the app).                                        |
| `locdev-daemon`     | `locdevd`: registry, supervisor, proxy, DNS, TLS and doctor.                                                        |
| `locdev-proxy`      | Hostname reverse proxy, with WebSocket and HTTPS.                                                                   |
| `locdev-dns`        | A minimal authoritative DNS server, written by hand.                                                                |
| `locdev-tls`        | Local CA, certificate issuing and trust checks.                                                                     |
| `locdev-platform`   | OS integration: resolver, trust store, login item and elevation.                                                    |
| `locdev-runtimes`   | Node.js, PHP and package manager versions: catalog, verified download, safe extraction, detection and shims.        |
| `locdev-frameworks` | Framework definitions (signed data) and the detection engine.                                                       |
| `locdev-env`        | Shared variables: `.env` reading, per-framework rules, resolution, sync and the encrypted vault.                    |
| `locdev-services`   | Databases and caches: installs, instances and cold backups.                                                         |
| `locdev-db`         | Database Manager: PostgreSQL, MySQL/MariaDB, SQLite and Redis/Valkey drivers.                                       |
| `locdev-mail`       | Mailbox: capture-only SMTP, message storage and analysis.                                                           |
| `locdev-email`      | Email Studio: blocks between comments, discovery, preview and generator.                                            |
| `locdev-config`     | Config Files: options catalog by version, JSON and JS/TS editing without running code, rules.                       |
| `locdev-deploy`     | Deploy assistant: stack profile, platforms with sources, readiness and generated files.                             |
| `locdev-deps`       | Dependencies: local reading, npm/Packagist registries, OSV and update commands.                                     |
| `locdev-git`        | The system `git` without a shell: status, diff, commit, branches, stash, remotes, reset/revert/restore and backups. |
| `locdev-github`     | Token in the keyring, the API client (`api.github.com` only) and device flow.                                       |
| `locdev-actions`    | GitHub Actions builder: templates, YAML ↔ model, validation and safe writing.                                       |
| `apps/locdev`       | The CLI.                                                                                                            |
| `apps/desktop`      | The Tauri + React app (a thin daemon client).                                                                       |

All Rust code compiles with `unsafe_code = "forbid"`.

## IPC protocol

Messages are JSON with a 4-byte length prefix, over a Unix socket (`0600`, in a checked `0700` directory) or a per-user named pipe that rejects remote clients. Each message carries the protocol version:

```json
{ "protocol": 2, "body": { "type": "start_project", "name": "blog" } }
```

An incompatible version becomes a clear error (`INCOMPATIBLE_PROTOCOL`), without taking the daemon down. The app and the CLI send exactly the same requests.

## Privileged operations

The daemon never runs as administrator. When something needs privileges (resolver, trust store), `locdev-platform` builds a **plan**: a readable script, with one step as administrator and, optionally, another as the user. The CLI and the app show the plan and only run it after confirmation, through the OS's own prompt (`sudo`, polkit, the macOS dialog or UAC). The app recomputes the plan on the Rust side, so what runs is what was shown.
