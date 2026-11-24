# Asistente de deploy

La pestaña **Deploy** de cada sitio (y `locdev deploy` en la CLI) analiza la stack e indica en qué plataformas el proyecto puede ir a producción, con lo que necesita cambiar y por qué. El análisis es recomendación y preparación: no envía nada a ninguna plataforma, y las variables de entorno aparecen solo por nombre. Para seguir y hacer deploys de verdad, conecta Vercel o Netlify ([Plataformas conectadas](#plataformas-conectadas)).

![La pestaña Deploy: la stack, las plataformas por compatibilidad y la preparación.](screenshot:deploy)

```bash
locdev deploy analyze                 # stack, plataformas, preparación
locdev deploy analyze --json          # todo, con el enlace de cada fuente
locdev deploy config render           # muestra el render.yaml que escribiría LocDev
locdev deploy config render --write   # crea el archivo (nunca sobre uno existente)
```

## Qué entra en el análisis

|                             | De dónde viene                                                                                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Framework, versión, runtime | la detección de stack (versión instalada)                                                                                                                           |
| Versión del runtime         | `engines.node`, `.nvmrc`, `require.php` de composer.json                                                                                                            |
| Renderizado                 | `output: 'export'` en next.config, `output`/adaptador en Astro, `ssr` en Nuxt, `adapter-static` en SvelteKit…; servidores de aplicación (Laravel, Django, Express…) |
| WebSockets                  | Reverb, socket.io, ws, Channels… instalados, o un proceso de WebSocket activado en LocDev                                                                           |
| Colas y workers             | Horizon, BullMQ, Celery, Sidekiq…, un `QUEUE_CONNECTION` distinto de `sync` en Laravel, o un worker activado                                                        |
| Programador                 | tareas en `routes/console.php`, node-cron, celery beat…                                                                                                             |
| Bases de datos y caché      | las conexiones que el Database Manager encuentra en el `.env` (PostgreSQL, MySQL, SQLite, Redis) y drivers de MongoDB                                               |
| Archivos en disco           | `FILESYSTEM_DISK` local/public con archivos en `storage/app/public`                                                                                                 |
| Build y arranque            | scripts de package.json con el gestor del lockfile; composer para PHP                                                                                               |
| Variables                   | nombres de `.env.example` y `.env` (los valores nunca salen del análisis)                                                                                           |

## Plataformas

Vercel, Netlify, Cloudflare Workers, Railway, Render, Fly.io, Laravel Forge, servidor propio (VPS) y GitHub Pages. Cada una recibe un veredicto:

- **encaja**: funciona tal cual; la lista "para este sitio" indica qué configurar (servicio de worker, cron, base gestionada…);
- **necesita cambios**: se puede, cambiando algo (Dockerfile, WebSockets en un servicio externo, uploads en object storage, SQLite fuera de disco efímero…);
- **no adecuada**: la plataforma no ejecuta ese tipo de aplicación (PHP en plataforma serverless, servidor de aplicación en GitHub Pages).

El orden viene solo del veredicto y de la cantidad de cambios, **nunca de la popularidad**. Cada capacidad y limitación enlaza la documentación de la plataforma de donde salió. La fecha de compilación aparece en pantalla. Las plataformas cambian (Vercel, por ejemplo, pasó a aceptar WebSockets con Fluid compute), así que revisa la fuente antes de decidir.

## Preparación

Comprobaciones que no dependen de la plataforma:

- **`.env` en el repositorio:** es el único resultado `fail`, porque los secretos quedan en el historial de Git. También avisa cuando no está en `.gitignore`.
- **`.env.example`:** si falta, o si no lista todas las variables.
- **Lockfile y versiones:** lockfile ausente y versión del runtime no declarada.
- **Git:** cambios sin commit, que un deploy desde Git no lleva.
- **Laravel:** `APP_DEBUG=true` local, para recordar desactivarlo en producción.
- **SQLite:** como base principal, el archivo se pierde en plataformas sin disco persistente.

## Archivos generados

Solo donde el formato está documentado y es estable:

| Plataforma | Archivo                   | Cuándo                                                                                                                                 |
| ---------- | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Render     | `render.yaml` (Blueprint) | aplicaciones Node.js: servicio web, base Postgres y Key Value conectados con `fromDatabase`/`fromService`, variables con `sync: false` |
| Netlify    | `netlify.toml`            | builds estáticos o SPA (con la redirección a `index.html`)                                                                             |

Ningún valor del `.env` entra en el archivo: los secretos se declaran para que la plataforma los pida. El archivo se muestra antes y solo se crea si sigue siendo el que revisaste. **Un archivo existente nunca se reemplaza.**

Las demás plataformas se configuran con sus propias herramientas, y el análisis dice cómo:
- **Fly.io:** `fly launch`.
- **Cloudflare:** `npx @opennextjs/cloudflare migrate` o `npm create cloudflare`.
- **Railway:** por el panel, o con `.railway/railway.ts` aplicado con `railway config plan/apply`. El antiguo `railway.json` quedó obsoleto y deja de funcionar el 2026-12-01, por eso LocDev no lo genera.
- **Vercel:** detecta el framework por sí sola.

## Plataformas conectadas

Con **Vercel** o **Netlify** conectada, la pestaña Deploy muestra el **proyecto vinculado** encima del análisis: los deploys con estado, rama, commit y cuándo, cuál está en vivo, los dominios y los **nombres** de las variables (LocDev nunca pide los valores a la plataforma).

![El proyecto vinculado: deploys, lo que está en vivo y lo que se enviaría desde aquí.](screenshot:deploy-linked)

1. **Conectar:** en **Configuración → Integraciones → plataformas de deploy → Conectar…**, pega un token. Se comprueba con la plataforma antes de guardarlo, en el **llavero del sistema**; en disco solo queda el nombre de la cuenta. Vercel: _Account Settings → Tokens_, limitado al equipo dueño de los proyectos (indica el slug del equipo). Netlify: _User settings → Applications → Personal access tokens_.
2. **Vincular:** en la pestaña Deploy del sitio, elige el proyecto (Vercel) o el sitio (Netlify). El vínculo queda en `projects.toml` (`deploy`).
3. **Desplegar:** **Deploy de vista previa** o **Deploy a producción**. La plataforma compila lo que el repositorio del sitio **ya envió a GitHub**. LocDev lo rechaza cuando eso no coincide con el código de aquí: cambios sin commit, commits sin enviar, HEAD suelto, otro repositorio, o producción desde una rama que no es la de producción del proyecto.
4. **Seguir:** la lista se actualiza sola mientras algo compila. Puedes **cancelar** un deploy en curso, abrir su **log de build** y **revertir producción** a un deploy anterior (Vercel: apunta producción a él; Netlify: vuelve a publicar ese deploy).

|                    | Vercel                         | Netlify                                                         |
| ------------------ | ------------------------------ | --------------------------------------------------------------- |
| Deploy             | el commit exacto (`gitSource`) | build de la rama por la integración Git de Netlify              |
| Cancelar, revertir | sí                             | sí                                                              |
| Log de build       | sí                             | **no está en la API pública**: LocDev abre la página del deploy |
| Variables          | nombres y entornos             | nombres y contextos                                             |
| Dominios           | sí                             | sí                                                              |

Fuentes: [API REST de Vercel](https://vercel.com/docs/rest-api) y [API de Netlify](https://open-api.netlify.com/), comprobadas el 2026-11-04. Cambiar variables todavía no se hace desde LocDev (lo edita el panel de la plataforma). Render, Railway, Fly.io, Laravel Forge y Cloudflare siguen solo en el análisis.

```bash
locdev deploy platforms
locdev deploy connect vercel --team acme        # el token se lee de la terminal, no de un argumento
locdev deploy projects vercel
locdev deploy link vercel tienda
locdev deploy status
locdev deploy run                                # vista previa; --production para producción
locdev deploy logs dpl_…
locdev deploy rollback dpl_…
locdev deploy disconnect vercel
```

Sin llavero (servidores, CI), `[deploy] token_store = "file"` en `config.toml` guarda los tokens en `deploy-tokens.toml` (`0600`).

## Limitaciones

- El análisis es estático: lee archivos y dependencias, no ejecuta el proyecto.
- Los workers de Node.js y las tareas programadas fuera de Laravel no tienen un comando conocido, así que `render.yaml` solo los menciona en notas.
- PHP, Python y Ruby en Render requieren un Dockerfile, y LocDev no genera uno.
