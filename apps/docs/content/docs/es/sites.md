# Sitios y procesos

Un **sitio** es una carpeta registrada en LocDev, con un nombre, un comando de servidor de desarrollo y un puerto. El registro está en `projects.toml`; quitar un sitio de LocDev no borra sus archivos, salvo que elijas mover su carpeta a la papelera al quitarlo (**Mover también la carpeta a la papelera**, o `locdev remove <nombre> --delete-files`; se rechaza para una carpeta que contiene otros sitios, la carpeta personal o la raíz de un disco). Un sitio cuya carpeta se borra fuera de LocDev se detiene y se quita en segundos, con una notificación; uno en un disco o recurso compartido desconectado, no.

Para no agregarlos uno por uno, monitorea la carpeta donde guardas tus proyectos: cada proyecto dentro de ella se vuelve un sitio y la lista sigue lo que se crea y se borra ([Carpetas monitoreadas](/docs/parked-folders)).

## Detección automática

Al agregar una carpeta, LocDev lee sus manifiestos, lockfiles y archivos de configuración (sin ejecutar nada del proyecto) y elige el adaptador que sabe iniciarlo: Laravel, Next.js, Django, Rails, Vite y otras 40 stacks. El comando sugerido usa el host `127.0.0.1` y el puerto del sitio, y el gestor de paquetes (`npm`, `pnpm`, `yarn` o `bun`) sigue el lockfile. La lista completa y las reglas están en [Frameworks](/docs/frameworks).

Puedes cambiar el comando cuando quieras:

```bash
locdev edit blog --command "pnpm dev --port {port}"
```

## Puertos y variables de entorno

- `{port}` en el comando se reemplaza por el puerto del sitio.
- El proceso recibe `PORT`, `HOST=127.0.0.1`, `LOCDEV=1` y `LOCDEV_PROJECT=<nombre>`.
- Si no indicas un puerto, LocDev elige el primero libre en el rango `ports` de [config.toml](/docs/configuration) (8100–8999 por defecto).

## Ciclo de vida

```bash
locdev up blog       # inicia
locdev down blog     # detiene
locdev reload blog   # reinicia
locdev logs blog -f  # sigue la salida
```

- Cada sitio corre en su propio grupo de procesos (Unix) o árbol (Windows). **Detenerlo termina todo lo que abrió el comando**, por ejemplo el `node` iniciado por `npm`.
- LocDev envía `SIGTERM` y espera hasta 8 segundos. Si el proceso no termina, envía `SIGKILL`. En Windows usa `taskkill /T /F`.
- Un sitio con `--auto-start` (o _Iniciar con LocDev_ en la aplicación) arranca junto con el daemon.
- La salida va a `logs/sites/<sitio>.log`, que rota al llegar a `[logs] file_mb` (5 MB por defecto); el daemon guarda las últimas líneas en memoria (`[logs] lines`, 2000 por defecto) y las vuelve a leer del archivo tras reiniciar.

## Direcciones de un sitio

| Campo   | Ejemplo                 | Qué es                                                               |
| ------- | ----------------------- | -------------------------------------------------------------------- |
| URL     | `https://blog.test`     | La dirección preferida: HTTPS cuando la CA es confiable, si no HTTP. |
| HTTPS   | `https://blog.test`     | Siempre disponible cuando HTTPS está activo.                         |
| Directa | `http://127.0.0.1:8100` | El servidor de desarrollo, sin el proxy.                             |

Los subdominios funcionan automáticamente: `api.blog.test` también llega al sitio `blog`.

## Sitios estáticos

Una carpeta con `index.html` (o `public/index.html`) y sin framework es un sitio estático. LocDev lo ejecuta con su propio servidor estático, `locdevd static --port {port}` (`--root public` para `public/index.html`), así que no necesita Python ni Node:

- escucha solo en 127.0.0.1, detrás del proxy como cualquier sitio (`<sitio>.test`, HTTPS);
- URLs limpias: `/about` sirve `about.html` o `about/index.html`;
- un archivo que no existe recibe tu `404.html`, si lo hay;
- `Cache-Control: no-cache`, para que una edición aparezca al recargar;
- nada fuera de la carpeta se sirve (`..`, rutas absolutas y escapes codificados se rechazan).

Para usar otro servidor, cambia el comando: `locdev edit <sitio> --command "…"`.
