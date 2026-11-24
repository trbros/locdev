# Debug Center

In **Tools → Debug Center** (`Ctrl` `Shift` `D`) or with `locdev debug`, LocDev gathers what goes wrong in your sites in one place: exceptions with stack traces, PHP's `dump()`/`dd()`, warning and error log lines, 5xx answers, processes that crashed, builds that failed and LocDev jobs that errored. Secrets are hidden before anything is stored.

![Debug Center: the timeline, an exception with its likely cause and stack trace.](screenshot:debug-center)

## Where events come from

| Source                                                                  | What becomes an event                                                                                         |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Process output (site, helper dev server, auxiliary processes, services) | Exceptions and stack traces, build errors, PHP warnings and lines marked as warning or error                  |
| Laravel's `storage/logs` (`laravel.log` and today's daily file)         | Entries from the chosen level up, with the exception and its stack; only what's written after the site starts |
| LocDev's proxy                                                          | 5xx answers and sites that didn't answer (method, address, status and time)                                   |
| Supervisor                                                              | Processes that exited with an error, with their last output lines                                             |
| LocDev's dump server                                                    | `dump()` and `dd()` from PHP sites with capture on                                                            |
| Job queue                                                               | Jobs that failed (dependency checks, installs…)                                                               |

LocDev only reads: it doesn't change the project's logging setup or its files.

### By runtime

| Runtime | What LocDev recognizes                                                                                                                                                                                   |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PHP     | `PHP Fatal error: Uncaught …` with `Stack trace:`; `PHP Warning`/`Deprecated` with file and line; the built-in server's format (`[500]: GET /path - Uncaught …`); Laravel's log; Symfony VarDumper dumps |
| Node.js | `TypeError: …` with `at function (file:line:column)` lines; Vite errors (`[vite] Internal server error`) and Next.js ones (`⨯ …`, `Module not found`); TypeScript errors (`error TS2322`)                |
| Python  | `Traceback (most recent call last):` through the `Type: message` line; `logging`'s default format (`ERROR:logger:message`)                                                                               |
| Rust    | Panics (`thread 'main' panicked at file:line:column:`); cargo errors (`error[E0425]`); `tracing`'s default format                                                                                        |

Dependency frames (`vendor`, `node_modules`, `site-packages`, the cargo registry) are folded in the stack. Other lines become events from the chosen level up (default: warnings) when they carry an explicit level (`ERROR`, `[warn]`, `npm ERR!`…).

## The page

- **Timeline:** newest events first, updated live (pause with **Live**).
- **Problems:** repeats grouped (same error type, same message once numbers and values are blanked, same place in the project's code), with the total and an hourly chart of the last 24 hours.
- **Filters:** site (on the left), kind, level and search by message, error type, file or URL.
- **Detail:** message, error type, framework, runtime, file and line, when it happened, the request (method, URL, status, time and controller, when known), the stack trace, the dumped value, the context and the text as written.
- **Likely cause:** for errors it knows (a database refusing connections, a port in use, a missing `APP_KEY`, a class or module not found, a missing Vite manifest, PHP's memory limit…), a hint at what usually causes it.
- **Actions:** **Copy** (text to paste in an issue), **JSON**, **Occurrences** (only that problem's repeats), **Export** (JSON Lines) and opening the file at the line in your [code editor](/docs/editors) (on the event and on each stack frame inside the site).

## PHP dumps (`dump()` and `dd()`)

![A dump: the object as a tree, with visibilities shown and sensitive values hidden.](screenshot:debug-dump)

For PHP sites with Symfony VarDumper (Laravel, Symfony), pick the site on the left and turn on **Capture dump() and dd()** (or `locdev debug dumps <site> on`). The site restarts. From then on dumps show up in the Debug Center as a tree (arrays, objects with public `+`, protected `#` and private `-` properties, recursion marked) instead of on the page.

How it works, without changing any project file:

- LocDev sets `VAR_DUMPER_FORMAT=server` and `VAR_DUMPER_SERVER=127.0.0.1:<port>` **in the site's process environment only**. It's VarDumper's own `server` format, which Laravel honors.
- PHP's built-in server (`php -S`, which `artisan serve` uses) doesn't pass environment variables to `$_SERVER`, the only place VarDumper looks. So the `auto_prepend_file` LocDev adds to PHP sites (in `data/php-prepend/`, marked "Managed by LocDev"; it also marks HTTPS requests, see **HTTPS and the local CA**) copies the two variables into `$_SERVER`. If your PHP already had an `auto_prepend_file` (Herd's, for instance), it still runs right after.
- If LocDev's dump server doesn't answer, VarDumper prints the dump as usual.
- `dd()` still ends the request: LocDev shows the value but doesn't keep whatever would have run after it.

LocDev reads PHP's serialization format itself, without running PHP or calling `unserialize`. Auxiliary processes LocDev runs for the site (queues, scheduler) get the same environment, so their dumps arrive too, with the command line in the context.

## Secrets

Before storing or showing any event, LocDev hides (`[redacted]`):

- the values in the site's `.env` and `.env.local`, except plain settings (hosts, ports, drivers, names, levels, `true`/`false`, numbers);
- the values of the [shared variables and secrets](/docs/environment) LocDev injects into the site;
- `Authorization`, `Cookie`, `Set-Cookie` and `X-Api-Key` headers; `Bearer` tokens; JWTs; `password=`, `token=`, `secret=`, `api_key=` and the like in URLs and text; passwords in URLs (`postgres://user:password@…`);
- in JSON and dumps, values of keys named like `password`, `token`, `secret`, `api_key`, `cookie`, `session` or ending in `_key`.

This covers what's identifiable; a secret with no recognizable name, in a place with no pattern, can slip through. Events stay on this machine.

## Storage and cleanup

Events live in `data/observe/<site>/<day>.jsonl`, the newest also in memory. In **Settings → Automations → Debug Center**:

- **Collect events** (on by default);
- **Keep for** (default: 7 days) and **Use at most** (default: 100 MB): when either limit is reached, the oldest days are deleted first;
- **Plain log lines from:** info, warnings (default) or errors.

**Clear** deletes a site's events (or all of them), in memory and on disk. The same options exist in `settings.toml` (`[observe] enabled`, `retention_days`, `max_mb`, `logs_from`).

## Command line

```bash
locdev debug events
locdev debug errors
locdev debug status
```

- `locdev debug events -f` follows live; `--site`, `--kind` (repeatable), `--level` and `--search` filter; `--json` for scripts.
- `locdev debug errors` groups repeats, with the last 24 hours' chart.
- `locdev debug show <id>` prints a whole event (stack, request, dump, likely cause).
- `locdev debug export` writes JSON Lines; `locdev debug clear --site <site>` clears; `locdev debug dumps <site> on|off` turns dump capture on and off.

## Limitations

- Validated on Linux: the whole pipeline with a real Python site (traceback, 500 at the proxy, `.env` secret hidden, grouping) and with real PHP 8.5 + Symfony VarDumper 7.4 through the built-in server (dump captured without showing on the page, exception with its request). Not tested with a full Laravel application, nor on Windows and macOS.
- The Node.js, Next.js, Vite, Python and Rust formats follow each tool's default output; logs in a format of their own (structured JSON, for example) become events only when they carry an explicit level.
- Laravel's log is read for the `single` and `daily` channels in `storage/logs`; other destinations (Papertrail, syslog, stderr) aren't.
- Errors that only happen in the browser (front-end JavaScript) don't go through LocDev.
