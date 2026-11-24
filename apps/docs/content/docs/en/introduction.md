# What LocDev is

LocDev is an **open source** local development environment for Linux, macOS and Windows. It registers your project folders, runs their development servers and serves each one on its own domain, like `blog.test`, with valid HTTPS.

![The main screen: your sites on the left and the selected one, with its live process output, on the right.](screenshot:sites)

## How it's organized

LocDev has three parts, and they all speak the same language:

- **`locdevd`**: the daemon. It runs in the background without administrator rights and owns everything: the project registry, processes, proxy, DNS and certificates. Sites keep running with the app closed.
- **`locdev`**: the CLI. It controls the daemon from the terminal and accepts `--json` for automation.
- **The desktop app**: an interface (Tauri + React) over the same operations as the CLI.

No rule lives only in the interface. If the app can do something, so can the CLI.

## What it does

- **Sites:** detection of 46 stacks (Laravel, Next.js, Django, Rails, Vite, Spring, .NET, Go…), supervised development servers with logs, auxiliary processes (queues, scheduler, WebSockets) and starting together with LocDev. See [Sites](/docs/sites) and [Frameworks](/docs/frameworks).
- **Network:** an HTTP/HTTPS proxy by hostname with WebSockets, its own DNS for `*.test`, [subdomains and wildcards](/docs/subdomains) and a local CA with name constraints.
- **Runtimes:** Node.js, PHP and package manager versions per site, downloaded with SHA-256 verification ([Runtimes](/docs/runtimes)).
- **Environment and services:** [shared variables and secrets](/docs/environment) injected into processes, and PostgreSQL, MySQL, MariaDB, MongoDB, Redis and Valkey instances ([Services](/docs/services)).
- **Tools:** [Database Manager](/docs/database), [Mailbox](/docs/mailbox), [Email Studio](/docs/email-studio), [Config Files](/docs/config-files), the [deploy assistant](/docs/deploy), [dependencies](/docs/dependencies), [Git and GitHub](/docs/git) and [GitHub Actions](/docs/actions).
- **Diagnosis:** the [Doctor](/docs/doctor) points out each problem with its fix.

What changed in each version is in the [Changelog](/changelog).

## Principles

- **No paywall:** the local environment and its essential features are free.
- **Native first:** native processes, no containers.
- **Works offline** once installed.
- **Secure by default:** least privilege and explicit confirmation for any change to the system.
