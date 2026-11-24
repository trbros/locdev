# Nuevo proyecto

En **Herramientas → Nuevo proyecto** (o `locdev new`), LocDev crea un proyecto con **el instalador del propio framework** (`laravel new`, `composer create-project`, `npm create vite`, `create-vue`, `ng new`, `rails new`, `cargo init`…), con las opciones que ese instalador documenta, y al final registra la carpeta como sitio. No hace falta abrir la documentación del framework para recordar los comandos.

![Nuevo proyecto: el framework, cómo crearlo, las opciones, la base de datos de LocDev, paquetes y los comandos que se ejecutarán.](screenshot:new-project)

## Cómo funciona

1. **Elige el framework.** Cada uno tiene sus formas de crearse y sus opciones (tabla de abajo).
2. **Nombre y carpeta.** El nombre se convierte en la carpeta y el sitio (`tienda` → `tienda.test`). La carpeta no puede existir: LocDev nunca crea encima de nada.
3. **Cómo crearlo.** Por ejemplo, Laravel con su instalador o con Composer; los proyectos JavaScript con npm, pnpm o Bun. Cada opción muestra si la herramienta está instalada y su versión.
4. **Runtime** (PHP y JavaScript): la versión de PHP o de Node.js con la que se ejecuta el instalador, entre las instaladas. El sitio nuevo queda fijado a ella. **Predeterminada** usa la versión predeterminada de LocDev.
5. **Opciones.** Kit inicial, autenticación, pruebas, TypeScript, Tailwind, linter, enrutamiento, plantilla… Solo aparecen las que acepta el método elegido.
6. **Base de datos** (Laravel): una instancia de LocDev (PostgreSQL, MySQL o MariaDB), nueva o existente.
7. **Paquetes:** búsqueda en npm o en Packagist, según el framework, para instalarlos justo después de crear.
8. **Qué se ejecutará:** los archivos que escribe LocDev (cuando los hay), cada comando exacto y lo que LocDev mismo hace entre ellos. Nada se ejecuta antes de **Crear proyecto**.

Después, los pasos se ejecutan en una **terminal dentro de la app**: ves la salida del instalador y puedes responder lo que pregunte. Las líneas que empiezan con › son de LocDev (archivos, base de datos, sitio). Al terminar, la app ofrece abrir e iniciar el sitio.

![El instalador ejecutándose en la terminal de la app, paso a paso.](screenshot:new-project-run)

## Frameworks sin generador

PHP puro, Flask, FastAPI, Axum y Actix Web no tienen un comando oficial que cree un proyecto listo para servir. Para ellos, LocDev escribe **archivos iniciales mínimos** antes de los comandos, y el plan muestra cada uno (haz clic para ver el contenido):

| Framework | Archivos de LocDev                                                | Después                                                           |
| --------- | ----------------------------------------------------------------- | ----------------------------------------------------------------- |
| PHP       | `public/index.php`                                                | `git init` (opcional)                                             |
| Flask     | `app.py`, `requirements.txt` (`flask`), `.gitignore`              | `.venv` + `pip install -r requirements.txt`, `git init`           |
| FastAPI   | `main.py`, `requirements.txt` (`fastapi[standard]`), `.gitignore` | `.venv` + `pip install -r requirements.txt`, `git init`           |
| Axum      | `src/main.rs`                                                     | `cargo init`, `cargo add axum`, `cargo add tokio --features full` |
| Actix Web | `src/main.rs`                                                     | `cargo init`, `cargo add actix-web`                               |

LocDev crea la carpeta, escribe los archivos sin sobrescribir nunca nada y solo entonces ejecuta las herramientas oficiales: `cargo init` adopta el `src/main.rs` que ya está ahí, y `pip` instala lo que lista `requirements.txt`. Los archivos están elegidos para que la detección reconozca el framework (una prueba lo comprueba), así que el sitio arranca con el comando correcto: `php -S … -t public`, `flask --app app run`, `fastapi dev main.py` o `cargo run`. Los servidores Rust leen el puerto de `PORT`, que define LocDev.

WordPress usa WP-CLI (`wp core download`, que comprueba el hash de la descarga); la instalación termina en el navegador, en la primera visita.

## Lo que hace LocDev además de los comandos

Con una base de datos de LocDev elegida, entre la creación y las migraciones:

- crea la instancia si todavía no existe, y la inicia;
- espera a que acepte conexiones y crea la base de datos con el nombre del proyecto (`mi-tienda` → `mi_tienda`);
- apunta `DB_CONNECTION`, `DB_HOST`, `DB_PORT`, `DB_DATABASE`, `DB_USERNAME` y `DB_PASSWORD` del `.env` **del proyecto recién creado** a ella (las líneas existentes se reemplazan en su lugar, incluidas las que Laravel deja comentadas; el resto del archivo queda como estaba);
- ejecuta `php artisan migrate`.

Con el instalador de Laravel, el driver (`--database=pgsql`, `mysql` o `mariadb`) sigue la base de datos elegida.

## Frameworks y opciones

<!-- generated:starters -->

Las flags de cada instalador vienen de su código fuente o de su documentación oficial, y una prueba comprueba los comandos generados para cada combinación. Los comandos se ejecutan sin shell, solo con programas de una lista fija (`laravel`, `composer`, `npm`, `npx`, `pnpm`, `bun`, `bunx`, `rails`, `dotnet`, `python3`, `cargo`, `wp`, `git`…), y el nombre del proyecto y de los paquetes se valida antes de entrar en un comando.

Dos detalles de comando que valen para todos:

- `npm create --yes x@latest` evita que npm se detenga a preguntar si puede descargar el generador.
- Angular CLI se ejecuta con `NG_CLI_ANALYTICS=false` (se muestra en el comando), la forma documentada de no enviar datos de uso y no preguntar por ello en medio de la creación.

## Instaladores

En **Herramientas → Instaladores** (o `locdev tools list`) están las herramientas que necesitan los proyectos, si están instaladas, en qué versión y dónde:

| Herramienta                                                                      | Cómo la instala LocDev                                                                |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| PHP, Composer, Node.js, npm, pnpm                                                | en [Runtimes](/docs/runtimes), con descarga verificada                                |
| Instalador de Laravel                                                            | `composer global require laravel/installer`, en una terminal, tras mostrar el comando |
| Rails                                                                            | `gem install rails` (necesita Ruby)                                                   |
| Symfony CLI, Bun, Python, uv, pipx, Poetry, Cargo, rustup, WP-CLI, Git, .NET SDK | enlace al instalador oficial                                                          |

Las herramientas también se encuentran donde las dejan sus instaladores oficiales, incluso fuera del `PATH`: la carpeta global de Composer (`~/.config/composer/vendor/bin`, `~/.composer/vendor/bin` o `%APPDATA%\Composer\vendor\bin`), `~/.bun/bin`, `~/.cargo/bin` (rustup) y `~/.local/bin`. Los pasos se ejecutan con los runtimes de LocDev primero en el `PATH`.

## En la línea de comandos

```bash
locdev new                                        # pregunta qué crear, paso a paso
locdev new list                                   # frameworks, métodos y opciones
locdev new laravel tienda --php 8.4 -o kit=react --database postgres:db
locdev new nextjs web --package-manager pnpm -o linter=biome -p zod
locdev new vite ui --typescript
locdev new django api --in ~/Code --yes
locdev new rust borde -o framework=actix
locdev tools list
locdev tools install laravel
```

`locdev new <framework> <nombre>` es lo mismo que `locdev new project <framework> <nombre>` (que sigue funcionando), con las mismas opciones:

| Opción                           | Hace                                                                         |
| -------------------------------- | ---------------------------------------------------------------------------- |
| `--method` / `--package-manager` | cómo crearlo (`installer`, `composer`, `npm`, `pnpm`, `bun`…)                |
| `-o CLAVE=VALOR`                 | una opción del framework (`locdev new list` las muestra todas)               |
| `--typescript`                   | TypeScript, en los frameworks que lo ofrecen                                 |
| `--php` / `--node`               | la versión con la que se ejecuta el instalador; el sitio queda fijado a ella |
| `-p PAQUETE`                     | un paquete para añadir después                                               |
| `--database SERVICIO:INSTANCIA`  | una base de datos de LocDev (Laravel)                                        |
| `--in CARPETA`                   | dónde crearlo (la carpeta actual por defecto)                                |
| `--no-site`                      | no registrarlo como sitio                                                    |
| `--yes`                          | no preguntar antes de ejecutar                                               |

`locdev new` solo, en una terminal, pregunta el framework, el nombre, la carpeta, el método, cada opción y la versión del runtime, muestra el plan y pide confirmación. La salida aparece en la terminal, y lo que escribes va al instalador.

## Límites

- Validado de punta a punta en Linux, creando el proyecto y sirviendo el sitio con LocDev: Laravel 13 con el instalador de Laravel (con una instancia nueva de PostgreSQL, la base de datos creada, el `.env` apuntado y las migraciones), Vue, Angular, Flask, FastAPI, PHP puro, Axum y Actix Web, y la creación con una versión de PHP elegida (fijada en el sitio). Los demás frameworks tienen los comandos probados, pero no se ejecutaron aquí; WordPress no se ejecutó porque WP-CLI no estaba instalado.
- Para Django, Flask y FastAPI, un `.venv` dentro del proyecto recibe los paquetes con `pip`; el paquete de configuración de Django se llama `config`.
- `create-vue` solo omite sus preguntas cuando alguna opción está activada; con todo desactivado y JavaScript, pregunta en la terminal.
- Los paquetes añadidos ejecutan los scripts de instalación que su gestor ejecutaría normalmente.
- Windows y macOS aún no se probaron (la terminal usa ConPTY en Windows).

## Laravel: las preguntas del propio instalador

LocDev pregunta lo que pregunta `laravel new`, en el mismo orden:

1. **¿Quieres usar un starter kit?**
2. Sin uno: el stack de frontend (Blade, o React, Vue, Svelte y Livewire como kits en blanco, sin autenticación).
3. Con uno: el stack del kit, el proveedor de autenticación (el de Laravel o WorkOS) y, con el de Laravel, componentes Livewire de un solo archivo y soporte de equipos.
4. Luego: pruebas (Pest o PHPUnit), base de datos, dependencias de frontend, Laravel Boost y Git.

El instalador se ejecuta con las flags correspondientes (`--react --workos`, `--vue --no-authentication`, `--livewire-class-components`, `--teams`…). Con Composer, las mismas elecciones se convierten en el paquete y la rama del kit (`laravel/react-starter-kit:dev-teams`, `--stability=dev`).

## Instaladores desde la app

**Herramientas → Instaladores** lista las herramientas que usan los starters y las instala sin salir de la app. PHP, Composer, Node.js y pnpm se instalan como runtimes verificados; las demás, con el comando documentado por su publicador para tu sistema, que se muestra antes y se ejecuta en una terminal dentro de la app:

| Herramienta           | Comando                                                                                                              |
| --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Instalador de Laravel | `composer global require laravel/installer`                                                                          |
| Symfony CLI           | `scoop install symfony-cli` (Windows), `brew install symfony-cli/tap/symfony-cli` (macOS), el script oficial (Linux) |
| Bun                   | `npm install -g bun`                                                                                                 |
| uv                    | `winget install --id=astral-sh.uv -e` (Windows), el script oficial (macOS, Linux)                                    |
| Poetry                | El instalador oficial                                                                                                |
| Rust (rustup, Cargo)  | El script oficial (macOS, Linux)                                                                                     |
| WP-CLI                | `composer global require wp-cli/wp-cli-bundle`                                                                       |
| Git                   | `winget install --id Git.Git -e --source winget` (Windows)                                                           |
| .NET SDK              | `winget install Microsoft.DotNet.SDK.10` (Windows), `dotnet-install.sh --channel LTS` (macOS, Linux)                 |
| Rails                 | `gem install rails`                                                                                                  |

Una herramienta sin comando para tu sistema mantiene el enlace a su página de instalación. `locdev tools install <id>` hace lo mismo desde la terminal.
