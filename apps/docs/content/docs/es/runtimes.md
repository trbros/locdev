# Runtimes y gestores de paquetes

Cada sitio puede correr con su propia versión de Node.js y de PHP, y con su propia versión de pnpm, Yarn, Bun o Composer, sin tocar el entorno global de la máquina. LocDev pone las versiones correctas al inicio del `PATH` del proceso del sitio.

LocDev separa tres cosas:

- **Runtimes** ejecutan el código: Node.js y PHP.
- **Gestores de paquetes** instalan dependencias: npm, pnpm, Yarn, Bun y Composer. Corren sobre el runtime del sitio (pnpm usa el `node` del sitio; Composer, el `php` del sitio).
- **Herramientas de build** (Vite, webpack, Next…) son dependencias del propio proyecto y aparecen en la página de dependencias, no aquí.

![Versiones instaladas por LocDev y las que ya estaban en la máquina, con la predeterminada de cada runtime.](screenshot:runtimes)

## De dónde vienen las versiones

- **Gestionadas por LocDev** (insignia `locdev`): descargadas de la distribución oficial y **verificadas con el SHA-256 publicado** antes de instalar. La instalación ocurre en una carpeta temporal y solo entra en su lugar con un `rename`, así que nunca queda a medias.
- **Detectadas en el sistema** (insignia con el origen): paquetes del sistema (`/usr/bin/php8.3`), Homebrew (`php@8.2`, `node@20`), nvm, fnm y otras herramientas en el `PATH`. LocDev solo usa esas versiones; nunca las modifica ni elimina.

| Herramienta | Instalación por LocDev                                                                         | Detección                                            |
| ----------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Node.js     | nodejs.org, verificado con `SHASUMS256.txt`                                                    | nvm, fnm, Homebrew, `PATH`                           |
| PHP         | binarios listos de StaticPHP (Linux/macOS) y windows.php.net (Windows), SHA-256 del publicador | apt (`php8.x`), Homebrew, otras herramientas, `PATH` |
| npm         | viene con cada Node.js y sigue su versión                                                      | el npm de cada Node.js gestionado, `PATH`            |
| pnpm        | registro npm, verificado con `integrity` (sha512)                                              | `~/.local/share/pnpm`, `PATH`                        |
| Yarn        | registro npm (`yarn` 1.x, `@yarnpkg/cli-dist` 2+), sha512                                      | `~/.yarn/bin`, `PATH`                                |
| Composer    | getcomposer.org, verificado con el SHA-256 publicado                                           | `PATH`                                               |
| Bun         | no lo instala (usa el instalador de bun.sh)                                                    | `~/.bun/bin`, `PATH`                                 |

pnpm, Yarn y Composer no son binarios nativos: cada versión instalada recibe pequeños _launchers_ (`locdev-bin/pnpm`, `pnpm.cmd` en Windows) que llaman al `node`/`php` del `PATH`, es decir, el del sitio. Si la versión exige un runtime más nuevo (el `engines.node` de pnpm, el PHP mínimo de Composer), el `doctor` avisa.

## Soporte de las versiones

Cada versión muestra la fase de soporte del publicador: el calendario oficial de Node.js (`nodejs/Release`) y los estados de las ramas de PHP (`php.net/releases/states.php`). Las versiones en _mantenimiento_, _solo seguridad_ y _fin de vida_ reciben una insignia, y el `doctor` avisa cuando un sitio corre en una versión sin soporte. Una copia de estos datos viene con LocDev para funcionar sin conexión; el daemon la actualiza en segundo plano (nunca con `[network] offline = true`).

## PHP de LocDev

LocDev no compila PHP: descarga binarios listos de quienes los publican, siempre comprobando el SHA-256 del publicador antes de instalar.

- **Linux y macOS:** los builds estáticos de [StaticPHP](https://static-php.dev) (el proyecto static-php-cli), desde `dl.static-php.dev`. El índice `versions.json`/`version_<MM>.json` trae la versión más nueva de cada línea con su SHA-256; las versiones anteriores aparecen con el archivo `.sha256` publicado junto a cada una. Una versión sin checksum publicado no aparece y no se instala.
- **Windows:** los zips oficiales de [windows.php.net](https://windows.php.net/download/) (NTS x64), mediante `releases.json`, que trae el SHA-256 de cada zip. Allí solo la versión más nueva de cada línea tiene checksum publicado, así que solo esa se ofrece. Los zips oficiales necesitan el Visual C++ Redistributable instalado.

LocDev usa el conjunto de extensiones `bulk` de StaticPHP, que cubre Laravel, Symfony y WordPress: `bcmath`, `curl`, `gd`, `imagick`, `intl`, `mbstring`, `mysqli`, `opcache`, `openssl`, `pdo_mysql`, `redis`, `sodium`, `zip` y otras. Los drivers PDO de SQLite y PostgreSQL vienen de los hooks de Swoole incluidos en el build (`PDO::getAvailableDrivers()` lista `sqlite` y `pgsql`), así que `pdo_sqlite`/`pdo_pgsql` no aparecen en `php -m`, pero Laravel con SQLite funciona como siempre. El conjunto `common` (`[runtimes] php_flavor = "common"` en `config.toml`) es más pequeño y trae `pdo_sqlite`/`pdo_pgsql` como extensiones, pero no tiene `intl`, `opcache`, `sodium` ni `mysqli`. La vista de extensiones del sitio (`locdev runtime extensions`) muestra lo que le falta a cada proyecto.

Al instalar, LocDev escribe un `php.ini` junto al binario (`bin/php.ini`, donde PHP busca primero) que apunta `curl.cainfo` y `openssl.cafile` al paquete de certificados del sistema: los builds estáticos no saben dónde está, y sin eso Composer y Guzzle fallan por HTTPS. En Windows, el `php.ini` generado habilita las extensiones incluidas en el zip.

```bash
locdev runtime available php          # la más nueva de cada línea (8.2, 8.3, 8.4, 8.5…)
locdev runtime available php --all    # todas las versiones con checksum publicado
locdev runtime install php 8.4
```

> [!NOTE]
> Se puede usar un mirror con la misma estructura en `config.toml`: `[runtimes] php_static_url` (Linux/macOS) y `php_windows_url` (Windows). El checksum sigue viniendo del índice del mirror, así que usa solo un mirror en el que confíes.

Para ajustar `memory_limit`, subidas, zona horaria y otras directivas de cada versión sin tocar el `php.ini` del sistema, consulta [Configuración de PHP](/docs/php-settings).

## Fijar versiones

Un _pin_ puede ser una línea (`22`, `8.3`) o una versión exacta (`22.11.0`). Una línea elige la versión más nueva instalada de esa línea.

```bash
locdev runtime install node 22        # la 22.x más reciente
locdev runtime install node lts       # la LTS actual
locdev runtime default node 22        # predeterminada para sitios sin pin
locdev edit blog --node 20            # pin solo de este sitio
locdev edit shop --php 8.3
locdev edit blog --node ""            # vuelve a la predeterminada
locdev reload blog                    # aplica
```

En la aplicación, los pins están en **Editar configuración…** del sitio, y la predeterminada en la pantalla **Runtimes**, con el menú **⋯** o con clic derecho en una versión.

Orden de resolución, por runtime o gestor:

1. el pin del propio sitio (`locdev edit`);
2. si no, un **archivo de versión** del proyecto: `.nvmrc`, `.node-version`, `.php-version`, `.tool-versions`, `volta` y `packageManager` de `package.json`, `config.platform.php` de `composer.json`. `.nvmrc` acepta `lts/*`, `lts/iron` y `node`;
3. si no, si el proyecto declara un **rango** (`engines.node`, `require.php`) y la predeterminada no lo cumple, la versión instalada más nueva dentro de él;
4. si no, la predeterminada;
5. si no, lo que aparezca primero en el `PATH` del sistema, que es el comportamiento sin LocDev.

Si un pin o archivo de versión apunta a un Node.js o PHP que no está instalado, el sitio **no arranca** y el error indica el comando que lo resuelve. Un `reload` en esa situación mantiene el proceso actual activo. Para los gestores de paquetes es solo un aviso: se usa el del `PATH`. Cuando la versión elegida está fuera del rango que declara el proyecto, el detalle del sitio y el `doctor` muestran la diferencia.

## En la terminal

Las versiones predeterminadas valen para los sitios sin versión fijada. Para usarlas también en la terminal (`php`, `node`, `npm`, `composer`…), activa **Usar las versiones predeterminadas en la terminal** en **Runtimes → Resumen**, o:

```bash
locdev runtime default php 8.4
locdev runtime default node 22
locdev runtime terminal on      # luego abre una terminal nueva
locdev runtime terminal         # estado, comandos y archivos modificados
locdev runtime terminal off
```

LocDev mantiene una carpeta propia (`~/.local/share/locdev/bin` en Linux) con un pequeño script por comando que ejecuta la versión predeterminada. Cambiar la predeterminada en la app cambia lo que usa la terminal, sin reabrir nada. Un tipo sin predeterminada no recibe script, y la terminal encuentra el del sistema.

- **Linux y macOS:** un bloque marcado al final de `~/.bashrc` y `~/.zshrc` (los que existen, más el de tu shell) pone la carpeta al principio del `PATH`; en fish, un archivo propio en `~/.config/fish/conf.d/locdev.fish`. Al desactivarlo se quita exactamente ese bloque.
- **Windows:** la carpeta va al principio del `Path` del usuario. El `Path` del sistema va antes, así que un PHP o Node.js instalado para todos los usuarios sigue teniendo prioridad.

## Eliminación segura

`locdev runtime uninstall` se niega a quitar una versión que usa un sitio **en ejecución**. Si sitios detenidos o la predeterminada resuelven a ella, la eliminación pide `--force` (en la aplicación, una confirmación que lista los sitios). Las versiones del sistema nunca se eliminan.

## Actualizaciones

Cuando hay una versión más nueva de una línea que instalaste (por ejemplo, Node.js 22.23.4 → 22.23.6), la lista muestra la actualización. `locdev runtime update node 22` instala la nueva versión; la anterior sigue instalada hasta que la quites, para no romper sitios que fijaron la versión exacta.

## Comandos

| Comando                                                                     | Descripción                                                                                                                     |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `locdev runtime list [--refresh]`                                           | Versiones gestionadas y detectadas, predeterminadas, soporte, uso y actualizaciones. `--refresh` vuelve a buscar en la máquina. |
| `locdev runtime available <tipo> [--all]`                                   | El catálogo (la más nueva de cada línea, o todas).                                                                              |
| `locdev runtime install <tipo> <versión>`                                   | Descarga, verifica e instala. Acepta `22`, `8.3`, `22.11.0`, `lts`, `latest`.                                                   |
| `locdev runtime update <tipo> [línea]`                                      | Instala la versión más nueva de las líneas ya instaladas.                                                                       |
| `locdev runtime uninstall <tipo> <versión> [--force]`                       | Quita una versión gestionada.                                                                                                   |
| `locdev runtime default <tipo> [versión]`                                   | Define la predeterminada (`--clear` la quita).                                                                                  |
| `locdev runtime terminal [on\|off]`                                         | Usa las versiones predeterminadas en las terminales nuevas (sin argumento, muestra el estado).                                  |
| `locdev edit <sitio> --node/--php/--pnpm/--yarn/--composer/--bun <versión>` | Pin del sitio (`""` vuelve a la predeterminada).                                                                                |

`<tipo>` es `node`, `php`, `pnpm`, `yarn`, `composer` o `bun` (`npm` solo aparece en la lista). El proceso del sitio también recibe `LOCDEV_NODE_VERSION`, `LOCDEV_PNPM_VERSION`, etc. con la versión resuelta.

## Extensiones de PHP

**Runtimes → PHP → Extensiones** (o `locdev runtime modules [versión]`) lista todas las extensiones que carga un PHP de esta máquina, con su versión, el `php.ini` que leyó y los archivos que su carpeta de extensiones tiene pero no carga (Windows: `php_*.dll` en `ext`; actívalas en `php.ini`). LocDev solo ejecuta un PHP que detectó él mismo.

Para un sitio, **Resumen → Extensiones de PHP** (o `locdev runtime extensions <sitio>`) compara lo que el proyecto necesita con lo que carga su PHP: `ext-*` en `composer.json` y `composer.lock`, los requisitos del framework y el driver de base de datos que nombra el `.env`. Las que faltan vienen con cómo obtenerlas para ese PHP.

También lista las extensiones que los paquetes instalados **recomiendan**: sus entradas `suggest` en `composer.lock` (por ejemplo `intervention/image` → `gd` o `imagick`), cada una marcada como cargada o ausente, con el motivo del propio paquete. Las recomendaciones nunca bloquean nada.

```bash
locdev runtime modules 8.4
locdev runtime extensions tienda
```
