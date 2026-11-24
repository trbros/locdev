# Config Files

La pestaña **Config** de cada sitio reúne los archivos de configuración que el proyecto ya tiene (`vite.config.ts`, `next.config.mjs`, `tsconfig.json`, `package.json`, `composer.json`…), con un formulario para las opciones que LocDev entiende, edición de texto, historial y un análisis basado en las versiones instaladas. En la CLI es `locdev config`.

![La pestaña Config: análisis, archivos del proyecto y el formulario de opciones.](screenshot:config-files)

```bash
locdev config list                                   # los archivos del sitio
locdev config show vite.config.ts                    # opciones tal como las lee LocDev
locdev config set vite.config.ts server.port 5180    # muestra el diff y pregunta antes de guardar
locdev config check                                  # análisis: no se modifica nada
locdev config history vite.config.ts
locdev config revert vite.config.ts <id>
```

## Solo lo que existe

La lista muestra solo los archivos presentes en el proyecto. **No se crea ningún archivo para llenar la pantalla.** Tailwind CSS 4, que se configura en el CSS (`@theme`), aparece por el archivo CSS que importa `tailwindcss`, no por un `tailwind.config.js` que el proyecto no usa.

| Archivo                                                                                                                                                                                                                                            | Formulario                                           | Paquete que define las versiones     |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------ |
| `vite.config.*`                                                                                                                                                                                                                                    | servidor, preview, build                             | `vite`                               |
| `next.config.*`                                                                                                                                                                                                                                    | React, seguridad, build, rutas, imágenes, TypeScript | `next`                               |
| `nuxt.config.*`, `astro.config.*`, `react-router.config.*`                                                                                                                                                                                         | renderizado y servidor                               | `nuxt`, `astro`, `@react-router/dev` |
| `tsconfig.json`, `tsconfig.*.json`, `jsconfig.json`                                                                                                                                                                                                | comprobación, módulos, emisión                       | `typescript`                         |
| `package.json`, `composer.json`                                                                                                                                                                                                                    | tipo de módulo, engines, estabilidad, autoloader     | —                                    |
| `biome.json(c)`, `.prettierrc*`                                                                                                                                                                                                                    | formato                                              | `@biomejs/biome`, `prettier`         |
| `svelte.config.*`, `angular.json`, `nest-cli.json`, `eslint.config.*`, `.eslintrc*`, `.babelrc`, `postcss.config.*`, `tailwind.config.*`, `phpunit.xml`, `vitest.config.*`, `playwright.config.*`, `deno.json(c)`, `wrangler.*`, CSS de Tailwind 4 | solo texto (con validación de sintaxis)              | —                                    |

## Cómo la edición preserva el archivo

- **JSON y JSONC** pasan por un árbol de sintaxis concreto: cambiar un valor mantiene los comentarios, las líneas en blanco, la sangría y el estilo de coma final. `package.json` y `composer.json` siguen siendo JSON estricto.
- **JavaScript y TypeScript** se leen con un parser (oxc) y **nunca se ejecutan**. LocDev encuentra el objeto de configuración en `export default`, `defineConfig(...)`, `module.exports`, una `const` exportada o una función que lo devuelve, y reemplaza **solo el fragmento del valor**. Una opción nueva entra al final del objeto, con la sangría y las comillas del archivo.
- Un valor calculado en código (`process.env.HOST`, una variable, una llamada) aparece como código y **el formulario no lo edita**: se muestra el motivo y el cambio queda para el editor de texto.
- Todo cambio muestra el **diff antes de guardar**. Si la edición de texto elimina comentarios, se listan.
- El archivo solo se escribe si sigue siendo el que revisaste (hash). Una sintaxis inválida no se guarda.
- La versión anterior va al **historial** (`data/state/config-history`, hasta 50 por archivo), que permite comparar y revertir.
- La escritura es atómica y mantiene los permisos del archivo.

El botón **Mostrar** abre la carpeta del archivo. La aplicación no abre el archivo con el programa predeterminado del sistema porque, en Windows, lo predeterminado para `.js` es ejecutar el script.

## Compatibilidad por versión

Cada opción sabe en qué versiones existe, y la versión viene de lo instalado (`node_modules`, `vendor`), luego del lockfile y por último del rango declarado. Una opción que la versión del proyecto no tiene aparece desactivada con el motivo, por ejemplo "needs next 15.5.0 or newer (this project has 15.2.4)". El daemon rechaza el cambio aunque un cliente lo intente.

## Análisis

`locdev config check` (y el panel **Análisis** de la pestaña) aplica reglas verificables. Cada hallazgo indica a qué versiones se aplica, comparadas con la instalada, y enlaza la fuente oficial. Cuando hay una corrección sencilla, se ofrece para revisión y **nunca se aplica sola**.

| Regla                                    | Cuándo                                                         |
| ---------------------------------------- | -------------------------------------------------------------- |
| `vite.https-boolean`                     | `server.https: true` con Vite ≥ 5 (eliminado)                  |
| `vite.host-exposed`                      | `server.host` expuesto en la red (LocDev usa 127.0.0.1)        |
| `next.powered-by`                        | Next.js enviando `X-Powered-By`                                |
| `next.images-domains`                    | `images.domains` con Next.js ≥ 14 (usa `remotePatterns`)       |
| `next.experimental-turbo`                | `experimental.turbo` con Next.js ≥ 15.3 (ahora `turbopack`)    |
| `next.typed-routes-moved` / `-available` | `typedRoutes` con Next.js ≥ 15.5                               |
| `next.cache-components`                  | `experimental.dynamicIO`/`ppr` con Next.js ≥ 16                |
| `next.ignore-build-errors`               | builds que pasan con errores de TypeScript                     |
| `tailwind.v4-js-config`                  | `tailwind.config.js` sin `@config` con Tailwind ≥ 4            |
| `tailwind.v4-directives`                 | `@tailwind base` con Tailwind ≥ 4                              |
| `tailwind.v4-postcss-plugin`             | plugin `tailwindcss` en PostCSS con Tailwind ≥ 4               |
| `eslint.eslintrc-ignored`                | `.eslintrc*` sin flat config con ESLint ≥ 9                    |
| `typescript.import-flags`                | `importsNotUsedAsValues`/`preserveValueImports` con TS ≥ 5.0   |
| `typescript.bundler-resolution`          | `moduleResolution: node` en un proyecto con bundler, TS ≥ 5.0  |
| `composer.unstable`                      | `minimum-stability` por debajo de `stable` sin `prefer-stable` |
| `package.manager-mismatch`               | `packageManager` distinto del lockfile presente                |

## Limitaciones

- El formulario cubre opciones escalares (texto, número, verdadero/falso, elección). Los objetos y listas (plugins, `paths`, `remotePatterns`) se editan en el texto; los plugins de Vite aparecen como solo lectura.
- El editor de texto tiene resaltado de sintaxis para cada tipo de archivo (JSON, JS/TS, CSS, XML…), búsqueda y deshacer, y valida la sintaxis al guardar (`Ctrl+S` revisa), pero aún no tiene autocompletado de las opciones.
- Las opciones que vienen de un objeto expandido (`...base`) no aparecen en el formulario; un aviso lo indica.
