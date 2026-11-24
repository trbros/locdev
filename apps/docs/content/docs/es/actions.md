# GitHub Actions builder

![Actions builder: plantillas y archivos de workflow del sitio](screenshot:actions-builder)

En **Tools → Actions builder** creas y editas los workflows de GitHub Actions de un sitio. El resultado se guarda en `.github/workflows/` en la carpeta del sitio. Para que GitHub ejecute el workflow, haces commit y push (por la pestaña Git o en la terminal).

## Plantillas

Las plantillas se arman a partir de lo que LocDev encuentra en los archivos del proyecto. No se ejecuta nada del proyecto. Las que coinciden con el sitio aparecen primero, marcadas con **encaja**, con el motivo.

| Plantilla              | Qué genera                                                                                                                                                                                                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node.js CI             | instala con el gestor del proyecto (npm, pnpm, Yarn 1 o Berry, Bun) y ejecuta los scripts `lint`, `typecheck`, `test` y `build` que existan. La versión de Node viene de `.nvmrc`/`.node-version` o de `engines` |
| PHP / Laravel CI       | `setup-php` con la versión de `composer.json`, `composer install`, build del front-end cuando hay `package.json`, y luego `php artisan test`, Pest o PHPUnit                                                     |
| Deploy to GitHub Pages | build y publicación en Pages (dos jobs; solo el de deploy tiene `pages: write`)                                                                                                                                  |
| Dependency review      | bloquea pull requests que agregan dependencias con vulnerabilidades conocidas                                                                                                                                    |
| CodeQL                 | análisis estático de JavaScript/TypeScript en push, PR y una vez por semana                                                                                                                                      |
| Release on tag         | crea una release con notas generadas cuando se envía un tag `v*`                                                                                                                                                 |
| Docker image           | build del `Dockerfile` en PRs; build y push a GHCR en la rama principal y en tags                                                                                                                                |

Todas empiezan con `permissions: contents: read` arriba, y cada job recibe solo lo que necesita.

![Editor visual con las comprobaciones y el YAML al lado](screenshot:actions-editor)

## Editor visual y YAML

El **editor visual** cubre:

- el nombre del workflow;
- disparadores: push y pull request con ramas y tags, manual (`workflow_dispatch`), programado (`cron`) y release publicada;
- permisos del token, arriba y por job;
- cancelación de ejecuciones antiguas;
- jobs: runner, timeout, entorno, dependencias entre jobs (`needs`) y pasos, con nombre, acción o comandos, `with`, `env` e `if`.

Los pasos se pueden reordenar, y el menú **Agregar paso** lista las acciones conocidas en su versión actual.

El YAML al lado lo genera el daemon en cada cambio. La pestaña **YAML** permite editar el texto directamente. Para volver al editor visual, el archivo debe usar solo lo que cubre el editor. Los archivos con otros recursos (`workflow_dispatch` con `inputs`, `matrix` con `include`, workflows reutilizables…) se abren en modo YAML, con el motivo, y no se descarta nada.

## Comprobaciones

LocDev revisa el YAML mientras editas:

| Regla                                                                                                                                      | Nivel       |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ----------- |
| sintaxis, `on:`, jobs sin `runs-on` o sin pasos, paso con `uses` y `run` a la vez, `needs` inexistente o en ciclo, cron inválido           | error       |
| **inyección de script**: `${{ github.event.pull_request.title }}`, `github.head_ref`, cuerpo de issue/comentario… pegados dentro de `run:` | error       |
| `pull_request_target`/`workflow_run` que hace checkout del código del PR                                                                   | error       |
| acción sin versión (`uses: dueño/accion`)                                                                                                  | error       |
| acción que sigue una rama (`@main`)                                                                                                        | aviso       |
| sin `permissions:` (el token queda con el predeterminado del repositorio) o `write-all`                                                    | aviso       |
| versión major más antigua que la actual de una acción conocida                                                                             | aviso       |
| acción de terceros fijada por tag en lugar de SHA; secreto interpolado en `run:`                                                           | información |

Si [actionlint](https://github.com/rhysd/actionlint) está en el `PATH`, también se ejecuta: el YAML va por la entrada estándar, no se escribe nada, y sus hallazgos aparecen marcados como _actionlint_. Sin él, solo se aplican las reglas anteriores.

**Los errores de LocDev impiden guardar.** Los avisos y los hallazgos de actionlint aparecen, pero no bloquean, porque actionlint también señala estilo y notas de shellcheck.

## Guardar sin sorpresas

**Revisar y guardar…** muestra el diff de lo que se va a escribir y las comprobaciones:

- **Archivo nuevo:** se crea solo si no existe nada con ese nombre (creación atómica).
- **Archivo existente:** hay que marcar "Replace the existing file". El daemon solo lo reemplaza si el archivo sigue teniendo el mismo contenido (SHA-256) que revisaste. Si alguien cambió el archivo entretanto, la escritura se rechaza y vuelves a revisar. El reemplazo es atómico (archivo temporal + rename).
- Nombres solo con el formato `nombre.yml`/`nombre.yaml`. LocDev no escribe fuera de `.github/workflows/` ni a través de symlinks.

## Versiones de las acciones

Las plantillas usan la versión major actual de cada acción, de una lista integrada. **Comprobar** consulta la última release de cada una en GitHub, con la cuenta conectada si la hay, si no de forma anónima. El resultado queda en `data/state/action-versions.json`. La consulta respeta el modo sin conexión.

## En la línea de comandos

```bash
locdev actions list                     # plantillas que coinciden y archivos existentes
locdev actions new node-ci              # imprime el workflow y las comprobaciones
locdev actions new php-ci --save        # crea el archivo (pregunta antes; nunca reemplaza)
locdev actions validate                 # revisa todos los workflows; sale con 1 si hay errores
locdev actions versions --refresh       # consulta GitHub
```

Para reemplazar un archivo que ya existe, usa la aplicación, que muestra el diff.

## Revisión de todos los workflows

**Herramientas → GitHub Actions** revisa todos los archivos de `.github/workflows/` al abrirse (`locdev actions validate` hace lo mismo en la terminal). Además de estructura y seguridad, LocDev sugiere lo que recomienda la comunidad para evitar ejecuciones que fallan o se cuelgan, cada sugerencia con el YAML que la corrige:

| Revisión               | Por qué                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------------------------- |
| `missing-setup`        | `pnpm`, `bun` o `deno` en un `run:` sin su action de setup: no están en los runners de GitHub  |
| `practice-timeout`     | Un job sin `timeout-minutes` corre hasta 6 horas cuando un paso se cuelga                      |
| `practice-concurrency` | Los workflows de pull request sin `concurrency:` mantienen ejecuciones ya superadas            |
| `practice-cache`       | `setup-node` o `setup-python` sin `cache:` descargan todo en cada ejecución                    |
| `practice-npm-ci`      | `npm install` puede cambiar el lockfile en el runner; `npm ci` instala exactamente lo que dice |
| `practice-composer`    | `composer install` en CI debería usar `--no-interaction --no-progress`                         |
| `practice-credentials` | `actions/checkout` deja el token en `.git/config` sin `persist-credentials: false`             |

Las sugerencias nunca bloquean el guardado; solo lo hacen los errores de LocDev. Las plantillas de LocDev ya las siguen.

### actionlint

Sin actionlint instalado, **Instalar actionlint** (o `locdev actions install-actionlint`) lo descarga de la última release de rhysd/actionlint en GitHub a la carpeta de datos de LocDev, solo cuando el SHA-256 del archivo de checksums de la release y el digest del propio GitHub coinciden. La validación lo usa desde entonces.

### Versiones locales

LocDev guarda una copia del workflow cuando lee un archivo que cambió fuera de él, antes de reemplazarlo y después de guardarlo (las últimas 50 por archivo, en `data/state/workflow-history/`). El botón de historial junto a cada archivo las lista; abrir una la carga en el editor, y guardarla pasa por la revisión de siempre.

```bash
locdev actions history ci.yml
locdev actions history ci.yml --show 1763900000000-saved
```
