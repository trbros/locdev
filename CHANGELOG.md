# Changelog

[English](CHANGELOG.md) · [Português (Brasil)](CHANGELOG.pt-BR.md) · [Español](CHANGELOG.es.md) · [日本語](CHANGELOG.ja.md)

LocDev's first public version is its initial release, 0.19.0: the desktop
app 0.19.0 with the daemon and CLI 0.18.0. Everything from 0.1.0 up to it
went through internal testing and testing with selected users, so it is all
listed here as part of the initial release. From now on, each release gets
its own entry.

The daemon/CLI and the desktop app have separate version numbers: a change
only in the UI or the app shell moves the app's, a change in the daemon or
the CLI moves theirs. A heading names both when they differ.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Sections: Added, Changed, Fixed, Security, Breaking.

Installer availability depends on [published releases](https://github.com/trbros/locdev/releases). [Full documentation](https://locdev.app/en/docs).

## [0.19.2] - 2026-11-24

App 0.19.2 with daemon and CLI 0.18.2.

### Fixed

- PHP sites opened over HTTPS (Laravel, Symfony, WordPress, Bedrock, plain
  PHP) linked their own assets, fonts and Symfony's toolbar with `http://`,
  which the browser blocked as mixed content. Every PHP site now loads
  LocDev's `auto_prepend_file`, which gives requests that reached the proxy
  over HTTPS the server variables a TLS server sets (`HTTPS`,
  `REQUEST_SCHEME`, `SERVER_PORT`). No project file changes.
- On an HTTPS request, a redirect to `http://` on the same host now points
  to `https://`, so apps with an `http://` URL setting (Bedrock's `WP_HOME`)
  no longer drop the browser back to HTTP.
- Laravel with the Vite helper: on Vite 8.3+ the page loaded its assets from
  `https://127.0.0.1:<port>`, which the Vite certificate didn't cover. The
  certificate now also names `localhost`, `127.0.0.1` and `::1`.
- Nuxt 4.1+: the dev error badge (`/__nuxt_dev__/…`) answered 403 at
  `<site>.test`.
- Django: new projects answered 400 (`DisallowedHost`) at `<site>.test`.
  LocDev now adds `.test` to `ALLOWED_HOSTS` and the proxy's scheme
  (`SECURE_PROXY_SSL_HEADER`) to new projects' settings, under `if DEBUG:`;
  the note for existing projects says what to add.
- ASP.NET Core: static assets answered 500, because `dotnet run
  --no-launch-profile` ran in Production. LocDev sets
  `ASPNETCORE_ENVIRONMENT=Development` unless the site sets it.
- NestJS: creating a project stopped at the Nest CLI's new questions
  (`@nestjs/observe`, agent skills).
- Bedrock: it was detected as plain PHP and served from the project root
  instead of `web/`. It's now a WordPress site served from `web/`, and new
  projects get a `.env` with the site's `WP_HOME`.

## [0.19.1] - 2026-11-24

App 0.19.1 with daemon and CLI 0.18.1.

### Changed

- Site URLs always use `.test`. While `*.test` doesn't resolve yet,
  `<site>.localhost` keeps working as a fallback.

### Fixed

- Windows: when another program (such as Acrylic DNS Proxy) holds port 53,
  the DNS server now names it instead of claiming it needs elevated
  privileges, and Doctor suggests freeing the port.
- Windows: declining the administrator prompt is reported as cancelled
  instead of as a successful change.

## [0.19.0] - 2026-11-24

Initial release. App 0.19.0 with daemon and CLI 0.18.0, for Linux (x86_64
and ARM64), macOS (Apple Silicon and Intel) and Windows, open source under
the Apache-2.0 license.

### Added

#### Foundation

- A daemon (`locdevd`) that owns everything, a CLI (`locdev`) and a desktop
  app (Tauri), all speaking the same protocol. Everything the app does has a
  CLI command, and `locdev` alone opens a full-screen terminal UI (`ctrl+p`
  reaches any page, site or command).
- Native execution with no Docker or WSL required. A reverse proxy (HTTP,
  HTTPS and WebSockets), DNS for `*.test` (systemd-resolved, `/etc/resolver`,
  NRPT on Windows) and a local CA with name constraints, with certificates per
  site and for deeper subdomains. `locdev doctor` checks the whole setup, and
  LocDev can start at login.
- System changes are always shown as a plan and confirmed first. Everything
  LocDev writes carries a marker, and uninstalling removes only that.

#### Sites

- Framework detection from 46 definitions (Laravel, WordPress, Symfony,
  Next.js, Nuxt, Astro, SvelteKit, Vite, NestJS, Django, Flask, FastAPI,
  Rails, Go, Rust, .NET and more). The definitions are signed and can be
  updated without a new release.
- Supervised dev servers with logs, restart with backoff, CPU and memory.
  The Vite helper next to Laravel runs only after you decide, and auxiliary
  processes (queue, Horizon, scheduler, Reverb…) are offered per framework.
- Static sites (`index.html`) on LocDev's own static server: loopback only,
  clean URLs, `404.html`, no Python or Node needed.
- Parked folders: every project inside a parked folder becomes a site and is
  kept in step as projects come and go.
- Per-site subdomains with rules that send them to another port or process
  (`locdev subdomain`, `locdev route`).
- Site tabs with an address bar, a site menu shared by the ⋯ button and the
  right click, and the code editor of your choice (VS Code, Cursor,
  Windsurf, VSCodium, Zed, Sublime Text, JetBrains).
- Request metrics per site (Metrics tab, `locdev metrics`): status classes,
  p50/p95/p99 latency, requests per minute, busiest paths and the latest
  requests, counted by the proxy and kept in memory only.
- Laravel Tinker (Tinker tab, `locdev tinker`): run code or open the
  interactive console with the site's PHP and environment.

#### Runtimes

- Node.js and PHP versions per site (a pin, a default, or the system's),
  plus npm, pnpm, Yarn, Bun and Composer. Downloads are installed only when
  the publisher's checksum matches: Node.js from nodejs.org, and PHP from
  StaticPHP (Linux and macOS) and windows.php.net (Windows).
- Versions already installed are detected (apt, Homebrew, nvm, fnm, PATH),
  and system versions are never removed. The default versions can also run
  in your own terminal (`locdev runtime terminal on`).
- PHP settings per version (memory, uploads, time limits, OPcache…). Each
  change is reviewed as a diff, backed up, checked by PHP and rolled back if
  PHP refuses it.
- PHP extensions: what each PHP on the machine loads and what its extension
  folder holds without loading, and for each site, the extensions it requires
  and the ones its packages recommend (`locdev runtime modules`,
  `locdev runtime extensions`).

#### New projects and tools

- New projects through official installers where available, shown first and
  run in a terminal inside the app: Laravel, Symfony, Bedrock, WordPress,
  Next.js, Vite, Vue, Angular, Nuxt, Astro, SvelteKit, NestJS, Django,
  Flask, FastAPI, Rust (Axum, Actix Web), Rails and ASP.NET Core. Optionally
  a database is created and the folder registered as a site. Stacks without a
  suitable generator receive minimal initial files.
- Laravel asks the same questions as `laravel new`, in the same order: a
  starter kit or not, the stack, the authentication provider, single-file
  Livewire components and teams.
- Installers inside the app: PHP, Composer, Node.js and pnpm as verified
  runtimes, and the Laravel installer, Symfony CLI, Bun, uv, Poetry, rustup,
  WP-CLI, Git and the .NET SDK with each publisher's documented command for
  your system.

#### Services and data

- Managed instances of PostgreSQL, MariaDB, MySQL, MongoDB, Redis and Valkey,
  each with its own data, port, logs and backups. Downloads are verified with
  the publisher's SHA-256. Other versions (MySQL 8.0 and 8.4, Redis, more
  PostgreSQL and MongoDB) can be installed from the app through Homebrew, apt
  or winget.
- Database Manager (`locdev db`): tables, rows, SQL with checks on
  destructive statements, and a Redis/Valkey key browser. Credentials stay in
  the daemon.
- Shared environment variables and secrets, injected into the sites'
  processes without editing `.env`, with an encrypted vault.
- Mailbox: a capture-only SMTP server on loopback, wired per framework.
- Email Studio: templates, Mailables and components edited visually, with
  Blade outside managed blocks kept byte for byte, plus previews and history.
- Config Files: the project's config files with forms for known options
  (JSON and JS/TS edited without running project code), history and
  analysis.

#### Code, CI and deploys

- Git in the app: changes, commits, branches, worktrees (each branch as its
  own site), stashes and remotes. Destructive operations show what is lost,
  ask for a confirmation phrase (and an optional password) and back up first.
- GitHub: the account in the system keyring, plus issues, pull requests and
  workflow runs.
- GitHub Actions builder: templates per stack, a visual editor and YAML.
  Every workflow is checked for security and community practices, with
  fixes. actionlint can be installed from the app, and local versions of
  each workflow are kept.
- Dependencies: updates, compatibility and vulnerabilities (OSV), with update
  plans you confirm.
- Deploy assistant (platforms ranked by fit, readiness checks,
  `render.yaml`/`netlify.toml`), and Vercel and Netlify deploys, rollbacks and
  logs through their public APIs.
- Tunnels: share a running site through a Cloudflare Quick Tunnel, optionally
  protected by e-mail.

#### Debugging and the app

- Debug Center: exceptions, error logs, HTTP 5xx answers, process failures
  and PHP `dump()`/`dd()` from every site, grouped into problems with likely
  causes, and recognizable secrets hidden before anything is stored.
- Background jobs and notifications (in the app, in the system and as a dot
  on the tray icon), with offline mode and settings for each automation.
- The app in Portuguese (Brazil), English, Spanish and Japanese, with light
  and dark themes, its own title bar and scrollbars, keyboard shortcuts you
  can change, and WCAG 2.2 AA accessibility checked on every screen.
- Updates checked against LocDev's GitHub releases and installed only when
  you choose, signed with LocDev's release key. A standalone CLI with
  `install.sh`/`install.ps1` that check `SHA256SUMS`.
- The documentation site in four languages, with tutorials, a CLI reference
  generated from the code and a comparison with other tools checked against
  the repository.
