# CLI

`locdev` controls the daemon from the terminal: everything the app does also works from here, because both talk to the same daemon. Every command accepts `--json` for machine-readable output, and `locdev <command> --help` shows the options.

Site commands (`up`, `down`, `reload`, `logs`, `info`, `open`, `secure`, `dev …`) take the site name or, without one, use the registered site that contains the current directory:

```bash
cd ~/Code/shop
locdev up            # same as locdev up shop
locdev logs -f
locdev secure
```

Commands marked **admin** show the exact change (the script) and ask for confirmation before using administrator privileges; `--yes` skips the question. Nothing is installed on the system without that step.

## Terminal UI (TUI)

`locdev` alone, in an interactive terminal, opens the TUI (the same as `locdev tui`; `--light` uses the light colors). It looks like the app: a sidebar with the pages, the sites and the services with their LEDs, the proxy, DNS, HTTPS and Mailbox state at the bottom, and the page in the middle (Dashboard, Sites, Services, Runtimes, Processes, Mailbox, Activity, Doctor, and About, which shows the CLI's and the daemon's versions).

| Key           | What it does                                                               |
| ------------- | -------------------------------------------------------------------------- |
| `↑` `↓` `tab` | Moves in the list; `tab` switches between the sidebar and the page         |
| `enter`       | Opens the page, the site's or service's output, the message                |
| `s` · `R`     | Starts/stops · restarts the selected site, service or process              |
| `o` · `l`     | Opens the site in the browser · shows its output                           |
| `ctrl+p`      | "Go to or do": pages, sites, actions and every CLI command                 |
| `:`           | Runs any `locdev` command right here, confirmations and passwords included |
| `/` · `?`     | Filters lists · shows every key                                            |
| `q`           | Quits (the daemon and your sites keep running)                             |

Whatever has no screen of its own in the TUI runs from `:` (for example `:git commit -m "…"`), which hands the terminal to the command and comes back to the TUI when it ends. Without an interactive terminal (in a pipe, or with `--json`), `locdev` without a command prints the help.

## Exit codes

| Code | When                                                                    |
| ---- | ----------------------------------------------------------------------- |
| `0`  | Success.                                                                |
| `1`  | Error (the message says what to do). Also when `doctor` finds an error. |
| `3`  | `locdev status` with the daemon stopped.                                |

## Reference

The reference below is generated from the CLI's own definitions (`make cli-catalog`); the app's **Tools → Command line** page shows the same catalog, with search. A test fails if this copy goes stale.

<!-- generated:cli -->
