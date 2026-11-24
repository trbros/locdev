# Tutorial: un sitio Laravel desde cero

En este tutorial creas un proyecto Laravel nuevo con la versión de PHP que elijas y terminas con el sitio en `https://blog.test`, una base de datos SQLite y los correos del proyecto llegando al Mailbox. No necesitas tener PHP ni Composer instalados: LocDev descarga los dos de sus fuentes oficiales.

Tiempo: unos 5 minutos, casi todo descargas.

## 1. Instala PHP

En la app: **Runtimes → PHP → Instalar versión…** y elige la línea (8.4, por ejemplo). Desde la terminal:

```bash
locdev runtime install php 8.4
locdev runtime install composer latest
```

LocDev descarga PHP listo de StaticPHP (Linux/macOS) o de windows.php.net (Windows) y solo lo instala si su SHA-256 coincide con el publicado. Consulta [Runtimes](/docs/runtimes).

![El catálogo de PHP: la versión más nueva de cada línea.](screenshot:runtimes-install)

## 2. Crea el proyecto

En la app: **Herramientas → Nuevo proyecto → Laravel**. Elige el nombre, el kit (ninguno, React, Vue, Svelte o Livewire) y la base de datos; el plan muestra cada comando antes de ejecutarlo.

![Nuevo proyecto: el formulario y el plan, con los comandos exactos.](screenshot:new-project)

Desde la terminal, lo mismo en una línea:

```bash
locdev new project laravel blog --method composer --php 8.4 --in ~/Code -y
```

`--php 8.4` crea el proyecto con ese PHP y fija el sitio en él. Los pasos se ejecutan en una terminal dentro de la app y, al final, LocDev registra el sitio.

![Los pasos ejecutándose en la terminal de la app.](screenshot:new-project-run)

## 3. Inicia el sitio

```bash
locdev up blog
locdev open blog
```

Laravel se ejecuta con `php artisan serve` en el puerto del sitio, y el proxy lo sirve en `http://blog.localhost` al instante. La base de datos predeterminada de Laravel es SQLite, ya migrada al crear el proyecto.

## 4. `.test` y HTTPS, una sola vez

```bash
locdev dns install
locdev tls trust
```

Cada comando muestra el script que va a ejecutar y pide confirmación. Después, `https://blog.test` abre en el navegador sin advertencias. Detalles en [Proxy y DNS](/docs/domains) y [HTTPS](/docs/https).

## 5. Correos en el Mailbox

El Mailbox captura lo que envía el sitio y no entrega nada:

```bash
locdev mail setup blog     # lo que cambia: MAIL_MAILER=smtp, host y puerto de LocDev
locdev mail enable blog
locdev reload blog
```

Las variables van solo al entorno de los procesos del sitio; el `.env` del proyecto no se modifica. Los workers de cola y el scheduler reciben las mismas variables. Envía un correo desde Laravel y abre **Herramientas → Mailbox** (o `locdev mail list`).

![Mailbox: los mensajes que enviaron los sitios, con HTML, texto y problemas.](screenshot:mailbox)

## 6. Revisa las extensiones

```bash
locdev runtime extensions blog
```

LocDev compara lo que pide el proyecto (`composer.json`, la base de datos del `.env`, los requisitos de Laravel) con lo que carga el PHP del sitio, y dice cómo resolver lo que falte.

## Próximos pasos

- [Debug Center](/docs/debug-center): excepciones, `dump()` y errores del log de Laravel en una línea de tiempo.
- [Servicios](/docs/services): PostgreSQL o Redis locales en lugar de SQLite.
- [Servidores de desarrollo](/docs/servers): Vite junto a Laravel para los assets.
