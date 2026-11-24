# Tutorial: un proyecto Node.js que ya tienes

En este tutorial traes a LocDev un proyecto existente (Vite, Next.js, Nuxt, Astro, Express…), con la versión de Node.js que pide, y lo abres en `https://<nombre>.test`. No se modifica nada del proyecto.

## 1. Registra la carpeta

En la app, `Ctrl` `N` (o el **+** de la lista de sitios) y elige la carpeta. Desde la terminal, dentro de ella:

```bash
locdev add . --start
```

![Añadir sitio: solo la carpeta es obligatoria.](screenshot:add-site)

LocDev reconoce el framework por sus archivos (`package.json`, `vite.config.*`, `next.config.*`…), sin ejecutar nada del proyecto, y sugiere el comando de desarrollo con el puerto del sitio (`{port}`). También pasa `PORT` y `HOST=127.0.0.1` al proceso y permite el host `.test` en Vite.

## 2. La versión de Node.js del proyecto

Si el proyecto tiene `.nvmrc`, `.node-version`, `.tool-versions`, `engines.node` o `volta` en el `package.json`, LocDev usa esa versión. Si no está instalada, el sitio no arranca y el error indica el comando que lo resuelve:

```bash
locdev runtime install node 20
locdev reload <nombre>
```

Para fijar una versión sin tocar el proyecto, usa la pestaña **Configuración** del sitio o:

```bash
locdev edit <nombre> --node 22
locdev reload <nombre>
```

![La configuración del sitio: comando, puerto, runtimes e inicio.](screenshot:site-settings)

El gestor de paquetes sigue el mismo camino: `packageManager` en el `package.json` (pnpm, Yarn, Bun) o `locdev edit <nombre> --pnpm 10`.

## 3. Sigue el servidor

```bash
locdev logs <nombre>     # la salida del servidor de desarrollo
locdev info <nombre>     # direcciones, versiones resueltas y de dónde vienen
```

En la app, la pestaña **Servidor** muestra la salida en vivo, y el [Debug Center](/docs/debug-center) reúne los errores de todos los sitios.

## 4. Muéstraselo a alguien (opcional)

Para compartir el sitio en ejecución con alguien fuera de tu red:

```bash
locdev tunnel start <nombre>
```

LocDev usa un Cloudflare Quick Tunnel, sin cuenta, y muestra la dirección pública. El túnel termina cuando detienes el sitio. Consulta [Túneles](/docs/tunnels).

## Próximos pasos

- [Variables compartidas](/docs/environment): secretos inyectados en el proceso, sin `.env` en el repositorio.
- [Subdominios](/docs/subdomains): `api.<nombre>.test` para otro proceso o puerto.
- [Dependencias](/docs/dependencies): versiones, actualizaciones y vulnerabilidades del proyecto.
