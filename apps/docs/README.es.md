# Sitio y documentación de LocDev

[English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [日本語](README.ja.md)

El sitio del producto y el manual completo están en esta aplicación Next.js. Documentación pública: **https://locdev.app/docs**. Para funciones e instalación, consulta el [README del repositorio](../../README.es.md).

## Stack y requisitos

Next.js 16 App Router, React 19, TypeScript, Tailwind CSS 4, Base UI, next-intl, next-themes, React Markdown, remark-gfm y Shiki en el servidor. Biome gestiona formato y lint; los tests de Node comprueban contenido y contratos. Usa Node.js 22.18+ (los tests importan TypeScript directamente) y la versión de pnpm de `package.json`.

## Ejecutar y validar

En esta carpeta, copia `.env.example` a `.env`, instala las dependencias e inicia el sitio. En PowerShell puedes usar `Copy-Item .env.example .env` en lugar de `cp`.

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

## Idiomas y rutas

Todas las páginas públicas tienen prefijo: `/pt-BR`, `/en`, `/es`, `/ja`. `/` detecta el idioma y usa portugués de Brasil como alternativa. Los archivos en portugués usan `br` (`messages/br.json`, `content/docs/br/`); el locale público es `pt-BR`. Las rutas antiguas redirigen con 308. Los tests cubren redirecciones y alternativas de idioma. `@triooo/named-routes` genera `lib/routes.generated.ts`; ejecuta `pnpm routes` después de cambiar rutas y `pnpm check:routes` para validarlas.

## Artículos y referencias generadas

Los artículos están en `content/docs/{br,en,es,ja}/<slug>.md`, con los mismos slugs en inglés en todos los idiomas. La navegación está en `docGroups`, en `lib/docs.ts`, y las etiquetas en `lib/docs-labels.ts`. Actualiza los cuatro idiomas juntos. Los enlaces internos usan `/docs/<slug>`; el renderizador añade el locale. Las capturas usan `![leyenda](screenshot:<nombre>)` en una línea independiente; los avisos usan `> [!NOTE]` o `> [!WARNING]`. Las tablas de CLI y frameworks usan `<!-- generated:cli -->` y `<!-- generated:frameworks -->`.

Desde la raíz, `make cli-catalog` regenera los catálogos de CLI, frameworks y proyectos iniciales desde Rust y copia los cuatro historiales. Para cambios solo del historial, ejecuta `node apps/docs/scripts/sync-changelogs.mjs`; `--check` comprueba las copias sin escribir. Las fuentes son `CHANGELOG.md`, `CHANGELOG.pt-BR.md`, `CHANGELOG.es.md` y `CHANGELOG.ja.md`. Las copias del sitio están en `generated/`, y las del escritorio en `../desktop/src/generated/`; ambos muestran el idioma elegido. Conserva encabezados de versión, números, fechas y marcadores de estado en las traducciones.

## Entorno y despliegue

`NEXT_PUBLIC_GITHUB_REPOSITORY_URL` debe ser la URL HTTPS raíz de un repositorio GitHub sin credenciales, query ni fragmento. De ella derivan enlaces, issues, instrucciones de clonado y API de releases. `NEXT_PUBLIC_SITE_URL` indica la URL pública. Las variables públicas se incorporan al build; reinicia y recompila al cambiarlas. Nunca incluyas `.env` ni `.env.local` en Git.

En Vercel, usa Root Directory `apps/docs` con acceso a archivos externos a esa carpeta: `next.config.ts` lee el `Cargo.toml` raíz. Configura las variables de `.env.example` en el entorno de build. El job `docs` de `.github/workflows/ci.yml` ejecuta lint, typecheck, tests, comparación, rutas y build de producción.

## Privacidad, seguridad y apoyo

Las opciones de alojamiento y contacto son `SITE_HOSTING_PROVIDER`, `SITE_PRIVACY_CONTACT_URL`, `SITE_SECURITY_REPORT_URL` y `SITE_SECURITY_REPORTING_CONFIRMED`. El canal de seguridad exige un contacto válido y confirmación explícita; sin ellos la ruta no está disponible ni aparece en el sitemap. `SITE_HOSTING_LOG_RETENTION` es opcional y debe describir el comportamiento verificado del alojamiento. No inventes plazos de retención.

`/about`, `/support`, `/privacy`, `/security`, `/contribute` y `/donate` contienen información pública del proyecto. El apoyo es una invitación a la comunidad; el sitio no procesa donaciones. Las descargas dependen de archivos realmente publicados. Las afirmaciones de comparación deben corresponder a las pruebas del repositorio (`pnpm check:comparison`).

## Mantenimiento de la documentación

Mantén artículos, referencias generadas y traducciones coherentes con el comportamiento implementado. El manual incluye diagnóstico, actualizaciones y desinstalación. En una eliminación completa, revierte DNS y confianza de la CA mientras el daemon siga disponible. Genera capturas con el mock del escritorio en los cuatro idiomas y ambos temas. No sustituyas licencias ni atribuciones de terceros por traducciones de sus originales oficiales.
