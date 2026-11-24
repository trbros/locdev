# Variables de entorno y secrets

LocDev guarda variables que quieres usar en varios proyectos (claves de Stripe, el DSN de Sentry, tokens de API) y las entrega a los procesos de los sitios **sin modificar los archivos `.env`**. Los archivos solo cambian cuando lo pides, con `locdev env sync`.

![La pestaña Environment de un sitio: variables aplicadas, archivos .env y conflictos.](screenshot:site-environment)

## Dónde se guardan

| Qué                                                              | Dónde                                                                                                         |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Perfiles, nombres, valores no secretos, elecciones de cada sitio | `config/env.toml` (`0600`)                                                                                    |
| Valores secretos                                                 | `data/state/env-vault.json`, cifrados con ChaCha20-Poly1305                                                   |
| Clave del almacén                                                | el llavero del sistema (Secret Service en Linux, Keychain en macOS, Administrador de credenciales en Windows) |
| Historial de cambios                                             | `data/state/env-history.json`, **sin valores**                                                                |
| Copias de `.env`                                                 | `data/state/env-backups/<sitio>/`, cifradas con la misma clave                                                |

Cada valor secreto se sella junto con su propio nombre (datos asociados del AEAD): un valor copiado a otra clave del archivo no se abre. Si la clave desaparece del llavero, LocDev **no crea otra** (en la práctica, eso borraría los secrets): avisa en el `doctor`, las variables comunes siguen funcionando y las secretas dejan de inyectarse.

Las máquinas sin llavero (un servidor sin sesión gráfica, por ejemplo) pueden guardar la clave en un archivo:

```toml
# config.toml
[env]
key_store = "file"   # env-vault.key (0600) junto a este archivo
```

En ese modo, el archivo de la clave es tan sensible como los propios secrets, y el `doctor` muestra un aviso.

Por defecto, una variable nueva es **secreta**. La excepción son los prefijos que los frameworks incrustan en el código del navegador (`NEXT_PUBLIC_`, `VITE_`, `NUXT_PUBLIC_`, `PUBLIC_`, `REACT_APP_`, `GATSBY_`, `EXPO_PUBLIC_`, `VUE_APP_`): esos son públicos y **no pueden** ser secretos. Tampoco se pueden compartir variables que controla LocDev o el sistema (`PATH`, `HOME`, `PORT`, `HOST`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `LOCDEV_*`…).

## Perfiles y herencia

- **global**: todos los sitios lo reciben, salvo que desactiven _Recibir las variables globales_ (`locdev env use <sitio> --global false`).
- **Otros perfiles** (Laravel, Staging, un cliente…): valen solo en los sitios que los activan. El orden es primero global, después los perfiles en el orden en que se activaron; **gana el último**, y lo sobrescrito aparece como `overridden`.
- Una clave se puede desactivar en un sitio (`--disable KEY`), sea cual sea su perfil.

## Inyección y conflictos

Al iniciar un sitio, el daemon resuelve las variables y las pone **solo en el entorno del proceso**. Nada va a argumentos de línea de comandos, logs ni archivos temporales.

Laravel (phpdotenv), Next.js (`@next/env`), Vite, Nuxt y el paquete `dotenv` dan preferencia a una variable que ya está en el entorno del proceso sobre el valor del archivo. Por eso, inyectar una clave que el proyecto ya define **reemplazaría en silencio el valor del proyecto**. LocDev nunca lo hace: si la clave existe en un archivo que lee el servidor de desarrollo, se omite, y el log del sitio registra:

```text
[LocDev] Warning: STRIPE_SECRET_KEY already exists in .env.local
[LocDev] Skipping shared variable STRIPE_SECRET_KEY (profile global).
```

Lo que se lee depende del framework (`locdev env check` muestra el orden de cada archivo):

| Framework              | Archivos leídos en desarrollo (gana el primero)                                                                       |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Next.js                | `.env.development.local`, `.env.local`, `.env.development`, `.env`                                                    |
| Vite, Astro, SvelteKit | `.env.development.local`, `.env.development`, `.env.local`, `.env`                                                    |
| Nuxt                   | `.env`                                                                                                                |
| Laravel                | `.env` (con `bootstrap/cache/config.php`, Laravel ignora el entorno hasta `php artisan config:clear`, y LocDev avisa) |
| PHP/WordPress          | `.env` solo con `vlucas/phpdotenv`, `symfony/dotenv` o `roots/wp-config` en `composer.json`                           |
| Node.js                | `.env` solo con `dotenv`; el archivo de `node --env-file=…` cuando el comando lo usa                                  |
| No reconocido          | `.env.local` y `.env`, por precaución                                                                                 |

Los archivos que el servidor no lee (`.env.example`, `.env.production`, `.env.test`) aparecen como conflicto informativo y no impiden la inyección. Para omitir también en ese caso:

```bash
locdev settings set env.conflicts any_file
```

También se omiten, con aviso, las claves que ya están en el entorno con que el daemon inicia procesos y las claves que el propio LocDev define para el proceso (como `APP_URL` en el Vite de Laravel). Un secret con prefijo público se **bloquea**.

Los cambios llegan a un proceso en ejecución solo después de reiniciarlo; la pestaña **Entorno** del sitio avisa cuando el proceso tiene un conjunto antiguo. Las variables públicas (`NEXT_PUBLIC_`, `VITE_`…) se incrustan en el código al compilar: reinicia el servidor de desarrollo y vuelve a hacer el build de producción.

## Sincronizar con los archivos

`locdev env sync` copia a un archivo del proyecto las variables que le faltan. Sin sitio, vale para todos los sitios registrados.

```bash
locdev env sync --dry-run              # todos los sitios, solo muestra
locdev env sync shop                   # .env del sitio shop
locdev env sync shop --file .env.local
locdev env sync shop --file .env.example
```

- Solo **agrega** líneas al final, bajo un comentario de LocDev. Los comentarios, el orden, el BOM y los finales de línea (`\r\n`) del archivo quedan como estaban.
- Omite las claves que el archivo ya tiene y las definidas en otro archivo que lee el servidor.
- **Nunca** copia secrets a `.env.production`/`.env.staging` (ni a sus `.local`).
- `.env.example` (y `.sample`, `.dist`, `.template`) recibe solo los nombres, con valor vacío, secretos o no, porque esos archivos suelen ir a Git.
- La vista previa enmascara los secrets. La escritura solo ocurre si el archivo y las variables siguen siendo los de la vista previa; si no, LocDev pide una nueva revisión.
- Antes de escribir, guarda una copia cifrada del archivo (hasta 20 por sitio): `locdev env backups shop` y `locdev env restore <id> shop`. Restaurar también guarda antes el archivo actual.
- Escribe de forma atómica, mantiene los permisos del archivo y rechaza enlaces simbólicos y archivos que no sean UTF-8.

## Comandos

```bash
locdev env list                         # variables (secrets enmascarados) y el almacén
locdev env add STRIPE_SECRET_KEY        # el valor viene de la entrada estándar (fuera del historial del shell)
locdev env add SENTRY_DSN https://… --plain
locdev env add QUEUE_CONNECTION redis --plain --profile laravel
locdev env show STRIPE_SECRET_KEY --reveal
locdev env remove SENTRY_DSN
locdev env profile add laravel --label Laravel
locdev env use shop --add-profile laravel --disable SENTRY_DSN
locdev env check shop                   # lo que recibe el proceso, archivos y conflictos
locdev env conflicts shop
locdev env history                      # cambios, sin valores
```

`--reveal` pide la contraseña de LocDev cuando `locdev security` exige una, y queda registrado en el historial. La contraseña protege lo que pasa por LocDev, no el archivo del almacén.

## En la aplicación

- **Settings → Environment**: variables con búsqueda y filtro por perfil, mostrar/ocultar secrets, crear, editar y eliminar (con confirmación), perfiles, política de conflictos, historial y sincronización en lote.
- **Site → Environment**: los perfiles del sitio y su orden, el estado de cada variable (`injected`, `skipped`, `overridden`, `off`, `blocked`) con el motivo, interruptor por clave, conflictos, los archivos `.env` con su orden de lectura, sincronización y copias.

## Límites

- La inyección vale para los procesos iniciados por LocDev. Un `php artisan` o `npm run` ejecutado en tu terminal no recibe las variables.
- Solo se leen los archivos `.env*` de la raíz del proyecto; en monorepos, cada sitio registrado se analiza en su propia carpeta.
- El llavero se probó de verdad en Linux (Secret Service). Keychain y el Administrador de credenciales de Windows usan la misma biblioteca, pero aún no se validaron en esos sistemas.
