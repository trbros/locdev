# Config Files

Each site's **Config** tab brings together the config files the project already has (`vite.config.ts`, `next.config.mjs`, `tsconfig.json`, `package.json`, `composer.json`…), with a form for the options LocDev understands, text editing, history and an analysis based on the installed versions. In the CLI, it's `locdev config`.

![The Config tab: analysis, project files and the options form.](screenshot:config-files)

```bash
locdev config list                                   # the site's files
locdev config show vite.config.ts                    # options as LocDev reads them
locdev config set vite.config.ts server.port 5180    # shows the diff and asks before saving
locdev config check                                  # analysis: nothing is changed
locdev config history vite.config.ts
locdev config revert vite.config.ts <id>
```

## Only what exists

The list only shows files present in the project. **No file is created to fill the screen.** Tailwind CSS 4, which is configured in CSS (`@theme`), shows up through the CSS file that imports `tailwindcss`, not through a `tailwind.config.js` the project doesn't use.

| File                                                                                                                                                                                                                                            | Form                                               | Package that sets the versions       |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- | ------------------------------------ |
| `vite.config.*`                                                                                                                                                                                                                                 | server, preview, build                             | `vite`                               |
| `next.config.*`                                                                                                                                                                                                                                 | React, security, build, routes, images, TypeScript | `next`                               |
| `nuxt.config.*`, `astro.config.*`, `react-router.config.*`                                                                                                                                                                                      | rendering and server                               | `nuxt`, `astro`, `@react-router/dev` |
| `tsconfig.json`, `tsconfig.*.json`, `jsconfig.json`                                                                                                                                                                                             | checking, modules, emit                            | `typescript`                         |
| `package.json`, `composer.json`                                                                                                                                                                                                                 | module type, engines, stability, autoloader        | —                                    |
| `biome.json(c)`, `.prettierrc*`                                                                                                                                                                                                                 | formatting                                         | `@biomejs/biome`, `prettier`         |
| `svelte.config.*`, `angular.json`, `nest-cli.json`, `eslint.config.*`, `.eslintrc*`, `.babelrc`, `postcss.config.*`, `tailwind.config.*`, `phpunit.xml`, `vitest.config.*`, `playwright.config.*`, `deno.json(c)`, `wrangler.*`, Tailwind 4 CSS | text only (with syntax validation)                 | —                                    |

## How editing preserves the file

- **JSON and JSONC** go through a concrete syntax tree: changing a value keeps comments, blank lines, indentation and the trailing comma style. `package.json` and `composer.json` stay strict JSON.
- **JavaScript and TypeScript** are read with a parser (oxc) and **never run**. LocDev finds the config object in `export default`, `defineConfig(...)`, `module.exports`, an exported `const` or a function that returns it, and replaces **only the value's span**. A new option goes at the end of the object, with the file's indentation and quotes.
- A value computed in code (`process.env.HOST`, a variable, a call) shows as code and **isn't edited by the form**: the reason is shown and the change is left to the text editor.
- Every change shows the **diff before saving**. If a text edit removes comments, they're listed.
- The file is only written if it's still the one you reviewed (hash). Invalid syntax isn't saved.
- The previous version goes to the **history** (`data/state/config-history`, up to 50 per file), which lets you compare and revert.
- The write is atomic and keeps the file's permissions.

The **Show** button opens the file's folder. The app doesn't open the file with the system's default program because, on Windows, the default for `.js` is to run the script.

## Version compatibility

Each option knows which versions it exists in, and the version comes from what's installed (`node_modules`, `vendor`), then from the lockfile and finally from the declared range. An option the project's version doesn't have shows disabled with the reason, for example "needs next 15.5.0 or newer (this project has 15.2.4)". The daemon refuses the change even if a client tries.

## Analysis

`locdev config check` (and the tab's **Analysis** panel) applies verifiable rules. Each finding says which versions it applies to, compared with the installed one, and links the official source. When there's a simple fix, it's offered for review and **never applied by itself**.

| Rule                                     | When                                                          |
| ---------------------------------------- | ------------------------------------------------------------- |
| `vite.https-boolean`                     | `server.https: true` with Vite ≥ 5 (removed)                  |
| `vite.host-exposed`                      | `server.host` exposed on the network (LocDev uses 127.0.0.1)  |
| `next.powered-by`                        | Next.js sending `X-Powered-By`                                |
| `next.images-domains`                    | `images.domains` with Next.js ≥ 14 (use `remotePatterns`)     |
| `next.experimental-turbo`                | `experimental.turbo` with Next.js ≥ 15.3 (now `turbopack`)    |
| `next.typed-routes-moved` / `-available` | `typedRoutes` with Next.js ≥ 15.5                             |
| `next.cache-components`                  | `experimental.dynamicIO`/`ppr` with Next.js ≥ 16              |
| `next.ignore-build-errors`               | builds passing with TypeScript errors                         |
| `tailwind.v4-js-config`                  | `tailwind.config.js` without `@config` with Tailwind ≥ 4      |
| `tailwind.v4-directives`                 | `@tailwind base` with Tailwind ≥ 4                            |
| `tailwind.v4-postcss-plugin`             | the `tailwindcss` PostCSS plugin with Tailwind ≥ 4            |
| `eslint.eslintrc-ignored`                | `.eslintrc*` without a flat config with ESLint ≥ 9            |
| `typescript.import-flags`                | `importsNotUsedAsValues`/`preserveValueImports` with TS ≥ 5.0 |
| `typescript.bundler-resolution`          | `moduleResolution: node` in a bundler project, TS ≥ 5.0       |
| `composer.unstable`                      | `minimum-stability` below `stable` without `prefer-stable`    |
| `package.manager-mismatch`               | `packageManager` different from the lockfile present          |

## Limitations

- The form covers scalar options (text, number, true/false, choice). Objects and lists (plugins, `paths`, `remotePatterns`) are edited in the text; Vite plugins show as read-only.
- The text editor has syntax highlighting for each file type (JSON, JS/TS, CSS, XML…), search and undo, and validates syntax on save (`Ctrl+S` reviews), but has no autocomplete for options yet.
- Options coming from a spread object (`...base`) don't show in the form; a notice says so.
