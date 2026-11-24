# LocDev website and documentation

[English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [日本語](README.ja.md)

The product website and complete manual live in this Next.js app. Public documentation: **https://locdev.app/docs**. For app features and installation, see the [repository README](../../README.md).

## Stack and requirements

Next.js 16 App Router, React 19, TypeScript, Tailwind CSS 4, Base UI, next-intl, next-themes, React Markdown, remark-gfm and server-side Shiki. Biome handles formatting/linting; Node’s test runner checks content and contracts. Use Node.js 22.18+ (tests import TypeScript directly) and the pnpm version in `package.json`.

## Run and validate

From this directory, copy `.env.example` to `.env`, install dependencies and run the site. In PowerShell, use `Copy-Item .env.example .env` instead of `cp` if needed.

```sh
cp .env.example .env
pnpm install
pnpm dev -p 1440
```

```sh
pnpm test
pnpm typecheck
pnpm exec biome check .
pnpm check:comparison
pnpm check:routes
pnpm build
```

## Languages and routing

All public pages have a prefix: `/pt-BR`, `/en`, `/es`, `/ja`. `/` detects the language and falls back to Brazilian Portuguese. Portuguese source files use `br` (`messages/br.json`, `content/docs/br/`); the public locale is `pt-BR`. Legacy routes redirect with 308. Routing tests cover redirects and language alternates. `@triooo/named-routes` generates `lib/routes.generated.ts`; run `pnpm routes` after route changes and `pnpm check:routes` to validate.

## Articles and generated references

Articles live in `content/docs/{br,en,es,ja}/<slug>.md`, with identical English slugs in every language. Navigation is `docGroups` in `lib/docs.ts`, with translated labels in `lib/docs-labels.ts`. Update all four languages together. Use `/docs/<slug>` for internal article links; the renderer adds the locale. Screenshots use `![caption](screenshot:<name>)` on a separate line; notices use `> [!NOTE]` or `> [!WARNING]`. CLI and framework tables use `<!-- generated:cli -->` and `<!-- generated:frameworks -->`.

From the repository root, `make cli-catalog` regenerates CLI, framework and starter catalogs from Rust and copies all four root changelogs. For changelog-only changes, run `node apps/docs/scripts/sync-changelogs.mjs`; `--check` verifies the copies without writing. Root files are `CHANGELOG.md`, `CHANGELOG.pt-BR.md`, `CHANGELOG.es.md` and `CHANGELOG.ja.md`. Site copies are in `generated/`, desktop copies in `../desktop/src/generated/`; both products display the selected language. Preserve release headings, versions, dates and status markers across translations.

## Environment and deployment

`NEXT_PUBLIC_GITHUB_REPOSITORY_URL` must be an HTTPS GitHub repository root without credentials, query or fragment. Repository links, issues, clone instructions and release API endpoints derive from it. `NEXT_PUBLIC_SITE_URL` identifies the public site URL. Public variables are embedded at build time; restart and rebuild after changing them. Never commit `.env` or `.env.local`.

Vercel uses Root Directory `apps/docs` with access to files outside that directory: `next.config.ts` reads the app version from `apps/desktop/package.json`. Configure the variables listed in `.env.example` in the build environment. The `docs` job in `.github/workflows/ci.yml` runs lint, typecheck, tests, comparison, routes and production build.

## Privacy, security and support

Hosting and contact settings are `SITE_HOSTING_PROVIDER`, `SITE_PRIVACY_CONTACT_URL`, `SITE_SECURITY_REPORT_URL` and `SITE_SECURITY_REPORTING_CONFIRMED`. Security reporting is enabled only when a valid contact and explicit confirmation exist; otherwise the security route is unavailable and absent from the sitemap. `SITE_HOSTING_LOG_RETENTION` is optional and must describe verified hosting behavior. Do not invent retention periods.

`/about`, `/support`, `/privacy`, `/security`, `/contribute` and `/donate` contain public project information. Support is a community invitation; the site does not process donations. Release downloads depend on real published assets. Comparison claims must match repository evidence (`pnpm check:comparison`).

## Documentation maintenance

Keep source content, generated references and translations consistent with implemented behavior. The manual includes diagnosis, updates and uninstall guides. DNS and CA trust must be reverted while the daemon is still available during a full uninstall. Generate screenshots from the desktop mock in all four languages and both themes. Do not translate vendor attribution or licenses in place of their authoritative originals.
