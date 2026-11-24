# Sites and processes

A **site** is a folder registered in LocDev, with a name, a dev server command and a port. The registry lives in `projects.toml`; removing a site from LocDev never deletes its files.

To skip adding them one by one, park the folder that holds your projects: each project inside becomes a site and the list follows what's created and deleted ([Parked folders](/docs/parked-folders)).

## Automatic detection

When you add a folder, LocDev reads its manifests, lockfiles and config files (without running anything from the project) and picks the adapter that knows how to start it: Laravel, Next.js, Django, Rails, Vite and 40 more stacks. The suggested command uses host `127.0.0.1` and the site's port, and the package manager (`npm`, `pnpm`, `yarn` or `bun`) follows the lockfile. The full list and the rules are in [Frameworks](/docs/frameworks).

You can change the command at any time:

```bash
locdev edit blog --command "pnpm dev --port {port}"
```

## Ports and environment variables

- `{port}` in the command is replaced with the site's port.
- The process gets `PORT`, `HOST=127.0.0.1`, `LOCDEV=1` and `LOCDEV_PROJECT=<name>`.
- If you don't give a port, LocDev picks the first free one in the `ports` range of [config.toml](/docs/configuration) (8100–8999 by default).

## Lifecycle

```bash
locdev up blog       # start
locdev down blog     # stop
locdev reload blog   # restart
locdev logs blog -f  # follow the output
```

- Each site runs in its own process group (Unix) or tree (Windows). **Stopping ends everything the command started**, such as the `node` launched by `npm`.
- LocDev sends `SIGTERM` and waits up to 8 seconds. If the process doesn't exit, it sends `SIGKILL`. On Windows it uses `taskkill /T /F`.
- A site with `--auto-start` (or _Start with LocDev_ in the app) starts with the daemon.
- The output goes to `logs/sites/<site>.log`, which rotates when it reaches `[logs] file_mb` (5 MB by default); the daemon keeps the last lines in memory (`[logs] lines`, 2000 by default) and reads them back from the file after a restart.

## A site's addresses

| Field  | Example                 | What it is                                                           |
| ------ | ----------------------- | -------------------------------------------------------------------- |
| URL    | `https://blog.test`     | The preferred address: HTTPS when the CA is trusted, HTTP otherwise. |
| HTTPS  | `https://blog.test`     | Always available when HTTPS is on.                                   |
| Direct | `http://127.0.0.1:8100` | The dev server, without the proxy.                                   |

Subdomains work automatically: `api.blog.test` also reaches the `blog` site.

## Static sites

A folder with an `index.html` (or `public/index.html`) and no framework is a static site. LocDev runs it with its own static server, `locdevd static --port {port}` (`--root public` for `public/index.html`), so it needs neither Python nor Node:

- it listens on 127.0.0.1 only, behind the proxy like any site (`<site>.test`, HTTPS);
- clean URLs: `/about` serves `about.html` or `about/index.html`;
- a missing file gets your `404.html` when there is one;
- `Cache-Control: no-cache`, so an edit shows on reload;
- nothing outside the folder is ever served (`..`, absolute paths and encoded escapes are refused).

To use another server, change the command: `locdev edit <site> --command "…"`.
