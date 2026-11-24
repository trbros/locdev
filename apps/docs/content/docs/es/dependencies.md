# Dependencias, jobs y notificaciones

## Dependencias del sitio

![Pestaña Dependencias de un sitio](screenshot:dependencies)

La pestaña **Dependencias** de cada sitio (o `locdev deps list`) lee los archivos del proyecto: `package.json` con `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock` (Yarn 1 y 2+) o `bun.lock`, y `composer.json` con `composer.lock`. Para cada paquete, separa tres versiones:

| Columna    | De dónde viene                                                                                       |
| ---------- | ---------------------------------------------------------------------------------------------------- |
| Declarada  | el rango del manifiesto (`^1.2.0`)                                                                   |
| Instalada  | lo que hay en `node_modules`/`vendor`; sin eso, la del lockfile (un `*` marca cuando ambas difieren) |
| Disponible | la versión estable más nueva; debajo, la más nueva que permite tu rango                              |

Listar nunca usa la red: los datos de los registros vienen de la última comprobación. El antiguo `bun.lockb` es binario y no se lee (Bun 1.2+ escribe `bun.lock` en texto).

## Comprobación

**Comprobar ahora** (o `locdev deps check`) consulta el registro npm, Packagist y [osv.dev](https://osv.dev) (que reúne los avisos de GitHub, npm y PHP). Las pre-releases (alpha, beta, RC, nightly) nunca se sugieren.

La compatibilidad de una actualización se clasifica, **nunca se garantiza**:

| Resultado      | Cuándo                                                                                                                                         |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| sin conflictos | ninguna comprobación encontró problemas (`engines`, peers, PHP); no es una garantía                                                            |
| posiblemente   | un major nuevo (o un minor de la serie 0.x), algo que no se pudo comprobar, o una versión marcada como obsoleta                                |
| incompatible   | un requisito no se cumple: `engines.node` contra el Node del sitio, un peer contra la versión instalada, `require.php` contra el PHP del sitio |
| indeterminado  | faltan datos                                                                                                                                   |

El Node.js y el PHP que usa este análisis son los que el sitio usa realmente (pin, archivos de versión, predeterminada o `PATH`).

## Actualizar

Selecciona paquetes y haz clic en **Actualizar** (o `locdev deps update <paquete>…`). Antes de ejecutar nada, LocDev muestra:

- versión actual → propuesta, el tipo de salto y la compatibilidad con sus motivos;
- enlaces a las notas de la versión, el repositorio y la página en el registro;
- el **comando exacto** que se ejecutará, con el gestor y el lockfile del proyecto;
- los archivos que deberían cambiar.

![Plan de actualización antes de confirmar](screenshot:dependency-update)

Por defecto, la actualización queda dentro de tu rango (`npm update`, `pnpm update`, `composer update --with-dependencies`). **Permitir majors** cambia el rango en el manifiesto manteniendo el estilo (`^`, `~` o exacto). Los scripts de instalación (`postinstall`…) **no se ejecutan** salvo que marques la opción; no se ejecuta ninguna migración ni otro comando. El resultado muestra la salida del gestor y los archivos que cambiaron.

Los comandos se ejecutan sin shell (programa y argumentos separados), en la carpeta del sitio y con los runtimes del sitio en el `PATH`.

## Monitoreo automático

En **Settings → Automations** (o `locdev settings set automation.audit <valor>`) eliges: desactivado, solo manual, cada 6 o 12 horas, diario (predeterminado) o semanal.

![Settings → Automations](screenshot:automations)

- Las comprobaciones entran en una **cola persistente** (`data/state/jobs.json`). Si el equipo estaba apagado a esa hora, se ejecuta cuando vuelve; un job interrumpido a medias vuelve a la cola.
- Los sitios se escalonan (no empiezan juntos), cada uno consulta sus paquetes de uno en uno, y hay un límite de peticiones por segundo sumando todos los jobs.
- Los datos de los registros se guardan en caché durante 6 horas. Los fallos de red y los `429` se reintentan con esperas crecientes (o el `Retry-After` del registro).
- Una comprobación que ya está en la cola no se duplica.
- Al **desactivarlo** (o activar el **modo sin conexión**), se cancelan los jobs automáticos en cola y uno en ejecución se detiene en el siguiente paquete. Ninguna consulta de monitoreo continúa.
- "Comprobar ahora" sigue funcionando con el monitoreo desactivado: es una petición tuya, no monitoreo.

`locdev jobs` lista la cola; `locdev jobs cancel|pause|resume|retry <id>` controla un job. La misma cola ejecuta las instalaciones de runtimes.

## Notificaciones

![Notificaciones](screenshot:notifications)

LocDev avisa sobre actualizaciones disponibles, vulnerabilidades, servidores que fallan y el resultado de lo que iniciaste. Las notificaciones se guardan en disco: sobreviven al cierre de la aplicación y al reinicio del daemon y del equipo, hasta que las descartes. Cada una tiene una clave: la misma actualización o el mismo aviso nunca aparece dos veces (una versión más nueva genera otra).

- **Pendientes / Historial:** marcar como leída, descartar, abrir el recurso (el sitio, la pestaña de dependencias o la salida del servidor).
- **Preferencias:** las categorías que generan notificaciones (incluidos los **correos capturados** por el Mailbox) y las **notificaciones del sistema**, activadas por defecto. El propio daemon las muestra siempre que ninguna ventana de LocDev está al frente: la aplicación oculta en la bandeja, cerrada o detrás de otras ventanas. Lo que llegó mientras nada podía mostrarlas aparece cuando el equipo vuelve (varias a la vez se convierten en un resumen).
- Con la ventana de la aplicación al frente, las nuevas aparecen como toasts dentro de ella, nunca ambas cosas: la aplicación avisa al daemon cuando tiene el foco.
- El [punto de color en la bandeja](/docs/app) muestra lo que está sin leer.
- Todo es local: ninguna notificación pasa por un servicio externo.

```bash
locdev notifications            # pendientes
locdev notifications --all      # incluye las descartadas
locdev notifications read       # todo leído
locdev settings set notifications.native on
locdev notifications test
```
