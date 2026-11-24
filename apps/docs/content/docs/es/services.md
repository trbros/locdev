# Servicios: bases de datos y cachés

LocDev ejecuta PostgreSQL, MariaDB, MySQL, MongoDB, Valkey y Redis para tus proyectos, sin Docker. Cada **instancia** tiene su propia carpeta de datos, puerto, configuración y log, corre supervisada por el daemon (con reinicio en caso de fallo) y escucha **solo en `127.0.0.1`**.

```bash
locdev service list                              # servicios, versiones instaladas e instancias
locdev service install postgresql                # descarga y verifica la versión más nueva
locdev service create db --service postgres --start
locdev service status
locdev service backup db
locdev service stop postgresql                   # el nombre del servicio vale cuando hay una sola instancia
```

## De dónde vienen los binarios

LocDev usa instalaciones que ya existen en la máquina (en el `PATH`, en `/usr/lib/postgresql/*`, en Homebrew, en `Program Files`) **sin modificarlas**, y puede descargar versiones propias. Una descarga solo se instala cuando su SHA-256 coincide con el que publica la fuente:

| Servicio   | Descarga por LocDev                     | Fuente                                                                                                                                           |
| ---------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| PostgreSQL | Linux, macOS, Windows                   | código oficial compilado por [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) (con atestación de procedencia) |
| MariaDB    | Linux, Windows                          | MariaDB Foundation (API de descargas, SHA-256 por archivo)                                                                                       |
| MongoDB    | Ubuntu 22.04+/Debian 12, macOS, Windows | MongoDB, Inc. (community server, un `.sha256` junto a cada archivo)                                                                              |
| Valkey     | Linux                                   | download.valkey.io (builds para Ubuntu)                                                                                                          |
| MySQL      | no                                      | Oracle solo publica MD5, que LocDev no acepta: usa un MySQL del sistema o MariaDB                                                                |
| Redis      | no                                      | no hay binarios oficiales: usa un Redis del sistema o Valkey (compatible)                                                                        |

Las versiones descargadas quedan en `~/.local/share/locdev/services/bin/` y se pueden quitar con `locdev service uninstall` (nunca las del sistema, ni una versión en uso por una instancia).

## Instancias

- **Datos** en `services/instances/<nombre>/data`, nunca mezclados entre instancias; la carpeta se crea con permiso `0700`.
- **Puerto**: el predeterminado del servicio si está libre (5432, 3306, 27017, 6379), si no el siguiente libre. Dos instancias nunca compiten por el mismo puerto, y Redis y Valkey (que comparten el 6379 por defecto) reciben puertos distintos.
- **Acceso**: solo `127.0.0.1`, usuarios locales sin contraseña (`locdev` en PostgreSQL, `root` en MariaDB/MySQL). Es un entorno de desarrollo; no expongas esos puertos.
- **Conexión**: `locdev service create` y la página **Servicios** muestran la URL y las variables habituales (`DATABASE_URL`, `DB_HOST`, `DB_PORT`, `REDIS_URL`…) para copiar al proyecto.
- **Inicio con LocDev**: `--autostart` o `locdev service autostart db true`. Las instancias arrancan antes que los sitios.
- **Lista** significa que acepta conexiones de verdad; el PID solo no basta.

## Copias de seguridad y eliminación

`locdev service backup` hace una **copia en frío**: detiene la instancia, comprime la carpeta de datos y la vuelve a iniciar. `locdev service restore <instancia> <id>` repone una copia y **mueve los datos actuales a `data.before-<hora>`**, sin borrarlos.

Eliminar una instancia **conserva los datos**. Borrarlos exige `--delete-data` y escribir el nombre de la instancia; las copias se mantienen.

## SQLite

SQLite es un archivo dentro del proyecto (`database/database.sqlite`, por ejemplo): no hay nada que instalar, iniciar ni detener. Para respaldarlo, copia el archivo mientras la aplicación no está escribiendo.

## Extensiones de PHP

`locdev runtime extensions <sitio>` compara lo que exige el proyecto con lo que carga el PHP del sitio:

- `ext-*` en `composer.json` y en los paquetes de `composer.lock`;
- los requisitos del framework (Laravel: ctype, curl, dom, fileinfo, filter, hash, mbstring, openssl, pcre, pdo, session, tokenizer, xml);
- los drivers que usa el `.env` (`DB_CONNECTION=pgsql` → `pdo_pgsql`; Redis con el cliente phpredis → `redis`).

No se instala nada automáticamente. El comando dice cómo resolverlo para ese PHP: el paquete de apt (`php8.3-pgsql`), el `pecl` de Homebrew, la línea del `php.ini` en Windows. Los builds de PHP de LocDev son estáticos, con un conjunto fijo de extensiones; para una que les falte, usa un PHP del sistema que la tenga. El comando sale con código 1 cuando falta alguna.

## Límites

- Validado de verdad en Linux con PostgreSQL 18.6 y Redis 8.10 del sistema. La instalación gestionada (descarga, verificación y extracción) tiene pruebas, pero aún no se ejercitó de punta a punta.
- Actualizar una instancia a otra versión major (PostgreSQL 16 → 17, por ejemplo) no es automático: crea una instancia nueva, exporta e importa los datos.
- Windows y macOS aún no se probaron.

## Otras versiones desde tu gestor de paquetes

Donde LocDev no puede descargar un build verificado (MySQL y Redis no publican SHA-256), o para tener más versiones, **Servicios** ofrece lo que tiene el gestor de paquetes del sistema. El comando se muestra antes y se ejecuta en una terminal dentro de la app:

| Sistema                      | Ofrecido                                                                                                                                                     |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| macOS (y Linux con Homebrew) | `mysql`, `mysql@8.4`, `mysql@8.0`, `redis`, `redis@8.2`, `postgresql@15` a `@17`, `mongodb-community@7.0` y `@8.0` (tap `mongodb/brew`), `mariadb`, `valkey` |
| Debian y Ubuntu              | `mysql-server`, `redis-server`, `postgresql` y `mariadb-server` de la distribución (`sudo` pide la contraseña en la terminal)                                |
| Windows                      | MySQL (la más reciente, 8.4 y 8.0) con winget (`Oracle.MySQL`)                                                                                               |

El gestor de paquetes verifica lo que instala. El resultado es una instalación del sistema que LocDev detecta (también las fórmulas con versión de Homebrew en `opt/`) y nunca cambia ni elimina. Redis no tiene build oficial para Windows: usa WSL o Memurai allí.

```bash
locdev service packages mysql
locdev service install-package mysql mysql@8.4
```
