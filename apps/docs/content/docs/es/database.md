# Database Manager

LocDev abre las bases de datos que usan tus sitios y las instancias que ejecuta él mismo (consulta [Servicios](/docs/services)): lista tablas, muestra la estructura, pagina filas, edita, inserta, elimina y ejecuta SQL. Para Redis y Valkey hay un navegador de claves. En la aplicación está en **Tools → Database**; en la CLI, `locdev db`.

![Tools → Database: conexiones del sitio, tablas y filas.](screenshot:database)

| Motor           | Tablas y estructura | Filas (filtro, orden, edición)           | SQL | Crear la base        |
| --------------- | ------------------- | ---------------------------------------- | --- | -------------------- |
| PostgreSQL      | sí (con schemas)    | sí                                       | sí  | sí                   |
| MySQL / MariaDB | sí                  | sí                                       | sí  | sí                   |
| SQLite          | sí                  | sí                                       | sí  | sí (crea el archivo) |
| Redis / Valkey  | claves por `SCAN`   | leer, editar strings, expiración, borrar | —   | —                    |

MongoDB aún no tiene navegador; sus instancias aparecen en **Servicios**.

## De dónde vienen las conexiones

Nadie escribe contraseñas en LocDev. El daemon lee, **en cada uso**, lo que el proyecto ya declara en `.env` y `.env.local`:

- **Laravel:** `DB_CONNECTION`, `DB_HOST`, `DB_PORT`, `DB_DATABASE`, `DB_USERNAME`, `DB_PASSWORD` (con `sqlite`, `DB_DATABASE` es el archivo, o `database/database.sqlite`).
- **`DATABASE_URL`** (Prisma, Rails, Django, Drizzle…): `postgres://`, `mysql://`, `mariadb://`, `file:` y `sqlite:`. En Prisma, un `file:./dev.db` es relativo a la carpeta `prisma/`, como hace el propio Prisma.
- **Redis:** `REDIS_URL`, o `REDIS_HOST`/`REDIS_PORT`/`REDIS_USERNAME`/`REDIS_PASSWORD`.
- **Archivos SQLite conocidos:** `database/database.sqlite`, `db.sqlite3`, `prisma/dev.db`, `storage/database.sqlite`, `db/development.sqlite3`.
- **Las variables compartidas** que recibe el sitio (consulta [Variables de entorno](/docs/environment)) se leen como si estuvieran en el `.env`.
- **Instancias de LocDev**, con el usuario local de cada servicio.

La misma base declarada dos veces (por ejemplo `DB_*` y `DATABASE_URL`) aparece una sola vez.

```bash
locdev db list                       # todas las conexiones, de todos los sitios e instancias
locdev db connect acme               # host, puerto, base y usuario (nunca la contraseña)
locdev db tables acme
locdev db describe users acme        # columnas, índices y claves foráneas
locdev db rows users acme --where role=admin --order-by id --desc
locdev db query "select count(*) from users" acme
locdev db create acme                # crea la base que nombra el .env del sitio
```

Con más de una base en el sitio, la CLI usa la primera base SQL; `--conn <id>` (de `locdev db list`) elige otra.

## Qué queda protegido

- **Las credenciales no salen del daemon.** La aplicación y la CLI solo conocen un identificador (`project:<sitio>:<n>` o `instance:<nombre>`) y si hay contraseña. Las contraseñas no aparecen en respuestas, logs ni mensajes de error.
- **Las bases fuera de esta máquina** (un `DB_HOST` que no es `127.0.0.1`/`localhost`/`::1`) aparecen con la insignia **remoto** y solo se abren con una elección explícita: el botón _Abrir de todos modos_ en la aplicación o `--allow-remote` en la CLI. Puede ser producción, y todo lo que edites ahí es real.
- **No se crea nada sin pedirlo.** Si el servidor responde pero la base no existe, la aplicación muestra un aviso con el botón _Crear_. El nombre viene de la configuración del sitio y se valida (letras, números, `_` y `-`).
- **Los comandos destructivos piden el nombre de la base.** `DROP`, `TRUNCATE`, `ALTER … DROP`, `GRANT`/`REVOKE`, y `DELETE`/`UPDATE` sin `WHERE` o con un `WHERE` siempre verdadero (`1=1`, `true`, `id = id`, `… or 1=1`). El daemon rehace este análisis al ejecutar, así que un cliente no puede saltarse la confirmación. Fuera de una terminal interactiva, la CLI rechaza en lugar de preguntar.
- **Los valores van como parámetros.** Los filtros, ediciones e inserciones nunca arman SQL con el texto escrito: columnas y tablas vienen del catálogo de la propia base, y los valores van como parámetros.
- **Límites:** como máximo 1000 filas por resultado (el resto se indica como truncado) y 35 s por operación.

El análisis de SQL destructivo es una heurística conservadora, no un parser completo. Detecta los casos comunes, pero no reemplaza una copia de seguridad: para las instancias de LocDev, usa `locdev service backup` antes de un cambio grande.

## En la aplicación

- **Data:** cuadrícula paginada (50 por página), orden al hacer clic en el nombre de la columna, filtros por columna (`=`, `!=`, `<`, `>`, `like`, `is_null`…), edición con doble clic (Enter guarda, Esc cancela, _NULL_ escribe nulo), inserción (los campos vacíos usan el valor predeterminado de la columna) y eliminación con confirmación. Editar, insertar y eliminar requieren clave primaria; sin ella, usa SQL.
- **Structure:** columnas, tipos, nulabilidad, valores predeterminados, índices y claves foráneas.
- **SQL:** editor con resaltado, **Ctrl+Enter** para ejecutar, atajos para insertar nombres de tablas e historial local (guardado solo en este equipo).
- **Redis/Valkey:** búsqueda por patrón (`user:*`), tipo y TTL de cada clave, lectura por tipo (string, list, hash, set, zset), edición de strings manteniendo la expiración, quitar la expiración y borrar la clave.

## Limitaciones conocidas

- Sin TLS: las conexiones a servidores que exigen SSL (`sslmode=require`, `rediss://`) fallan. Para bases remotas con TLS, usa el cliente del propio proveedor.
- Los valores se muestran como texto. Las columnas binarias aparecen como texto con pérdidas.
- Validado de verdad en Linux con SQLite, PostgreSQL 18 y Redis 8 (pruebas `crates/locdev-db/tests/real.rs`). MySQL/MariaDB está implementado, pero aún no se probó contra un servidor real.
