# Quick start

## 1. Start the daemon

The app does it when it opens. From the terminal:

```bash
locdev start
locdev status
```

```text
● daemon running  pid 41628 · up 3s
  version   0.6.0 (protocol 2)
  projects  0 registered, 0 running
  proxy     127.0.0.1:80 · *.localhost
  dns       127.0.0.1:10053 · *.test not resolving
  https     127.0.0.1:443 · CA not trusted (locdev tls trust)
```

## 2. Register a project

Inside the project folder:

```bash
locdev add . --start
```

LocDev detects the project type, suggests the dev server command, picks a free port (from 8100) and starts the site. In the app, use `Ctrl` `N` or the **+** button.

![Add site: only the folder is required.](screenshot:add-site)

## 3. Open it in the browser

Right after installing, the site already answers on `http://name.localhost`. Browsers resolve `*.localhost` by themselves, with nothing to set up.

To use `name.test` with HTTPS and no warnings, set up two things, once:

```bash
locdev dns install   # *.test resolves to LocDev
locdev tls trust     # the system and browsers trust the local CA
```

Both commands show exactly what will run and ask before doing it. See [Proxy and DNS](/docs/domains) and [HTTPS](/docs/https).

## 4. Check the environment

```bash
locdev doctor
```

`doctor` points out busy ports, an unconfigured resolver, an untrusted CA and missing tools, each with the command that fixes it.

## Next steps

Follow one of the tutorials from start to finish:

- [A Laravel site from scratch](/docs/tutorial-laravel): PHP downloaded by LocDev, a new project, HTTPS and the Mailbox.
- [A Node.js project you already have](/docs/tutorial-node): the folder, its Node.js version and a tunnel to show it.
- [Versions per project and in the terminal](/docs/tutorial-versions): install, pin and use the default in the terminal.
