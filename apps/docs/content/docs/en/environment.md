# Environment variables and secrets

LocDev keeps variables you want to use across projects (Stripe keys, a Sentry DSN, API tokens) and hands them to the sites' processes **without changing `.env` files**. Files only change when you ask, with `locdev env sync`.

![A site's Environment tab: applied variables, .env files and conflicts.](screenshot:site-environment)

## Where they live

| What                                                    | Where                                                                                          |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Profiles, names, non-secret values, each site's choices | `config/env.toml` (`0600`)                                                                     |
| Secret values                                           | `data/state/env-vault.json`, encrypted with ChaCha20-Poly1305                                  |
| Vault key                                               | the system keyring (Secret Service on Linux, Keychain on macOS, Credential Manager on Windows) |
| Change history                                          | `data/state/env-history.json`, **without values**                                              |
| `.env` backups                                          | `data/state/env-backups/<site>/`, encrypted with the same key                                  |

Each secret value is sealed together with its own name (AEAD associated data): a value copied to another key in the file won't open. If the key disappears from the keyring, LocDev **doesn't create another** (that would effectively wipe the secrets): it warns in `doctor`, plain variables keep working and secret ones stop being injected.

Machines without a keyring (a server with no graphical session, for example) can keep the key in a file:

```toml
# config.toml
[env]
key_store = "file"   # env-vault.key (0600) next to this file
```

In that mode the key file is as sensitive as the secrets themselves, and `doctor` shows a warning.

By default, a new variable is **secret**. The exception is the prefixes frameworks embed in browser code (`NEXT_PUBLIC_`, `VITE_`, `NUXT_PUBLIC_`, `PUBLIC_`, `REACT_APP_`, `GATSBY_`, `EXPO_PUBLIC_`, `VUE_APP_`): those are public and **can't** be secret. Variables LocDev or the system controls (`PATH`, `HOME`, `PORT`, `HOST`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `LOCDEV_*`…) can't be shared either.

## Profiles and inheritance

- **global**: every site gets it, unless it turns off _Take the global variables_ (`locdev env use <site> --global false`).
- **Other profiles** (Laravel, Staging, a client…): apply only to the sites that turn them on. The order is global first, then the profiles in the order they were turned on; **the last one wins**, and what was overridden shows as `overridden`.
- A key can be turned off for a site (`--disable KEY`), whatever its profile.

## Injection and conflicts

When a site starts, the daemon resolves the variables and puts them **only in the process environment**. Nothing goes into command-line arguments, logs or temporary files.

Laravel (phpdotenv), Next.js (`@next/env`), Vite, Nuxt and the `dotenv` package prefer a variable that's already in the process environment over the file's value. So injecting a key the project already defines **would silently replace the project's value**. LocDev never does that: if the key exists in a file the dev server reads, it's skipped, and the site log records:

```text
[LocDev] Warning: STRIPE_SECRET_KEY already exists in .env.local
[LocDev] Skipping shared variable STRIPE_SECRET_KEY (profile global).
```

What gets read depends on the framework (`locdev env check` shows each file's order):

| Framework              | Files read in development (the first one wins)                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Next.js                | `.env.development.local`, `.env.local`, `.env.development`, `.env`                                                             |
| Vite, Astro, SvelteKit | `.env.development.local`, `.env.development`, `.env.local`, `.env`                                                             |
| Nuxt                   | `.env`                                                                                                                         |
| Laravel                | `.env` (with `bootstrap/cache/config.php`, Laravel ignores the environment until `php artisan config:clear`, and LocDev warns) |
| PHP/WordPress          | `.env` only with `vlucas/phpdotenv`, `symfony/dotenv` or `roots/wp-config` in `composer.json`                                  |
| Node.js                | `.env` only with `dotenv`; the file in `node --env-file=…` when the command uses it                                            |
| Not recognized         | `.env.local` and `.env`, to be safe                                                                                            |

Files the server doesn't read (`.env.example`, `.env.production`, `.env.test`) show up as an informational conflict and don't prevent injection. To skip in that case too:

```bash
locdev settings set env.conflicts any_file
```

Also skipped, with a warning, are keys already in the environment the daemon starts processes with, and keys LocDev itself sets for the process (such as `APP_URL` on Laravel's Vite). A secret with a public prefix is **blocked**.

Changes reach a running process only after it restarts; the site's **Environment** tab warns when the process has an old set. Public variables (`NEXT_PUBLIC_`, `VITE_`…) are embedded in the code at build time: restart the dev server and rebuild for production.

## Syncing with files

`locdev env sync` copies into a project file the variables it's missing. Without a site, it applies to every registered site.

```bash
locdev env sync --dry-run              # all sites, only shows
locdev env sync shop                   # the shop site's .env
locdev env sync shop --file .env.local
locdev env sync shop --file .env.example
```

- It only **appends** lines at the end, under a LocDev comment. The file's comments, order, BOM and line endings (`\r\n`) stay as they were.
- It skips keys the file already has and keys defined in another file the server reads.
- It **never** copies secrets into `.env.production`/`.env.staging` (nor their `.local` variants).
- `.env.example` (and `.sample`, `.dist`, `.template`) gets only the names, with an empty value, secret or not, because those files usually go into Git.
- The preview masks secrets. Writing only happens if the file and the variables are still the ones in the preview; otherwise LocDev asks for a new review.
- Before writing, it keeps an encrypted copy of the file (up to 20 per site): `locdev env backups shop` and `locdev env restore <id> shop`. Restoring also backs up the current file first.
- It writes atomically, keeps the file's permissions and refuses symbolic links and files that aren't UTF-8.

## Commands

```bash
locdev env list                         # variables (secrets masked) and the vault
locdev env add STRIPE_SECRET_KEY        # the value comes from standard input (out of shell history)
locdev env add SENTRY_DSN https://… --plain
locdev env add QUEUE_CONNECTION redis --plain --profile laravel
locdev env show STRIPE_SECRET_KEY --reveal
locdev env remove SENTRY_DSN
locdev env profile add laravel --label Laravel
locdev env use shop --add-profile laravel --disable SENTRY_DSN
locdev env check shop                   # what the process gets, files and conflicts
locdev env conflicts shop
locdev env history                      # changes, without values
```

`--reveal` asks for the LocDev password when `locdev security` requires one, and is recorded in the history. The password protects what goes through LocDev, not the vault file.

## In the app

- **Settings → Environment**: variables with search and a profile filter, reveal/hide secrets, create, edit and remove (with confirmation), profiles, conflict policy, history and bulk sync.
- **Site → Environment**: the site's profiles and their order, each variable's state (`injected`, `skipped`, `overridden`, `off`, `blocked`) with the reason, a per-key switch, conflicts, the `.env` files with their read order, sync and backups.

## Limits

- Injection applies to processes started by LocDev. A `php artisan` or `npm run` run in your terminal doesn't get the variables.
- Only the `.env*` files at the project root are read; in monorepos, each registered site is analyzed in its own folder.
- The keyring was tested for real on Linux (Secret Service). Keychain and the Windows Credential Manager use the same library, but haven't been validated on those systems yet.
