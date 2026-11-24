# Tutorial: a Node.js project you already have

In this tutorial you bring an existing project (Vite, Next.js, Nuxt, Astro, Express…) into LocDev, with the Node.js version it asks for, and open it at `https://<name>.test`. Nothing in the project is changed.

## 1. Register the folder

In the app, `Ctrl` `N` (or the **+** of the site list) and pick the folder. From the terminal, inside it:

```bash
locdev add . --start
```

![Add site: only the folder is required.](screenshot:add-site)

LocDev recognizes the framework from its files (`package.json`, `vite.config.*`, `next.config.*`…), without running anything from the project, and suggests the dev command with the site's port (`{port}`). It also passes `PORT` and `HOST=127.0.0.1` to the process, and allows the `.test` host in Vite.

## 2. The project's Node.js version

If the project has `.nvmrc`, `.node-version`, `.tool-versions`, `engines.node` or `volta` in `package.json`, LocDev uses that version. If it isn't installed, the site doesn't start and the error names the command that fixes it:

```bash
locdev runtime install node 20
locdev reload <name>
```

To pin a version without touching the project, use the site's **Settings** tab or:

```bash
locdev edit <name> --node 22
locdev reload <name>
```

![The site's settings: command, port, runtimes and startup.](screenshot:site-settings)

The package manager follows the same path: `packageManager` in `package.json` (pnpm, Yarn, Bun) or `locdev edit <name> --pnpm 10`.

## 3. Follow the server

```bash
locdev logs <name>       # the dev server's output
locdev info <name>       # addresses, resolved versions and where they came from
```

In the app, the **Server** tab shows the output live, and the [Debug Center](/docs/debug-center) gathers every site's errors.

## 4. Show it to someone (optional)

To share the running site with someone outside your network:

```bash
locdev tunnel start <name>
```

LocDev uses a Cloudflare Quick Tunnel, with no account, and shows the public address. The tunnel ends when you stop the site. See [Tunnels](/docs/tunnels).

## Next steps

- [Shared variables](/docs/environment): secrets injected into the process, no `.env` in the repository.
- [Subdomains](/docs/subdomains): `api.<name>.test` for another process or port.
- [Dependencies](/docs/dependencies): the project's versions, updates and vulnerabilities.
