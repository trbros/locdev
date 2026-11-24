# Servidores de desarrollo

LocDev ejecuta el servidor de desarrollo de cada sitio como un proceso supervisado por el daemon: arranca con la versión correcta de Node.js/PHP, recibe un puerto, queda detrás del proxy en `https://<sitio>.test` y sigue corriendo con la ventana de la aplicación cerrada.

## Detección

Al registrar una carpeta, LocDev **lee** los archivos del proyecto (`package.json`, lockfiles, `vite.config.*`, `artisan`, `composer.json`, `wp-config.php`…) para elegir el comando. Nunca ejecuta un script del proyecto solo para descubrir la tecnología.

| Proyecto                           | Servidor principal                                                                                     | Cómo se pasa el puerto                        |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------ | --------------------------------------------- |
| Laravel                            | `php artisan serve --no-reload` (sin `--no-reload`, Laravel descarta las variables que inyecta LocDev) | `--port {port}`                               |
| Laravel + Vite                     | `artisan serve` + **Vite como auxiliar** (con consentimiento)                                          | `--port` en Vite                              |
| React/Vue/Svelte + Vite, SvelteKit | el script `dev`                                                                                        | `--port {port} --strictPort --host 127.0.0.1` |
| Next.js                            | el script `dev`                                                                                        | `--hostname 127.0.0.1 --port {port}`          |
| Nuxt, Astro                        | el script `dev`                                                                                        | `--port {port} --host 127.0.0.1`              |
| Node.js genérico                   | `dev` o `start`                                                                                        | variable `PORT`                               |
| PHP, WordPress                     | `php -S`                                                                                               | en la línea de comandos                       |
| HTML/CSS/JS                        | ningún proceso                                                                                         | —                                             |

Vite ignora la variable `PORT`; por eso LocDev pasa `--port` y `--strictPort` (con el puerto ocupado, Vite falla en lugar de saltar a otro que el proxy no conoce). El gestor de paquetes sale del lockfile (`pnpm-lock.yaml`, `yarn.lock`, `bun.lock`, si no npm). `locdev info` muestra lo detectado, y el detalle del sitio avisa cuando el comando registrado difiere del que LocDev usaría hoy.

## El auxiliar (Vite junto a Laravel)

En un proyecto Laravel con Vite, PHP sirve las páginas y Vite sirve los assets con recarga en caliente: son dos procesos. LocDev detecta Vite, pero **no ejecuta nada hasta que decidas**:

![Pregunta sobre el Vite detectado](screenshot:dev-prompt)

| Opción               | Efecto                                                             |
| -------------------- | ------------------------------------------------------------------ |
| Iniciar ahora        | Ejecuta Vite ahora; las próximas veces, lo inicias cuando quieras. |
| Iniciar con el sitio | Arranca y se detiene junto con el sitio.                           |
| Preguntar de nuevo   | No se ejecuta nada; la pregunta sigue.                             |
| No gestionar         | LocDev deja de preguntar y nunca ejecuta Vite.                     |

La decisión se puede cambiar en cualquier momento en la pestaña **Servidor** del sitio o por la CLI:

```bash
locdev dev mode auto            # inicia con el sitio
locdev dev mode manual --start  # ahora, y después solo a mano
locdev dev mode never
locdev dev status | start | stop | restart
locdev dev logs -f
```

El auxiliar recibe su propio puerto y su propia salida (`locdev logs --dev`). Detener el sitio detiene ambos; Vite recibe SIGTERM (en Windows, el cierre del árbol), así que `laravel-vite-plugin` borra el archivo `public/hot` por sí mismo.

## Proxy, HMR y HTTPS

- **Hosts permitidos:** Vite rechaza hosts desconocidos (`.localhost` siempre se acepta). LocDev pasa al proceso la variable `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS=<sitio>.test,.<sitio>.test`, que Vite lee, sin editar el `vite.config` del proyecto y sin permitir otros hosts.
- **HMR:** el cliente de Vite se conecta al mismo host y protocolo de la página, así que el WebSocket pasa por el proxy (`ws://` o `wss://`). Validado con Vite 8: el handshake `101` y el `full-reload` llegan por el proxy al editar un archivo.
- **Laravel + Vite en HTTPS:** la página viene de `https://<sitio>.test` y los assets de Vite. Cuando la CA de LocDev es confiable y `*.test` resuelve a LocDev, el proceso de Vite recibe `VITE_DEV_SERVER_CERT`, `VITE_DEV_SERVER_KEY` (un certificado del sitio emitido por la CA local, con la clave legible solo por tu usuario) y `APP_URL=https://<sitio>.test`. `laravel-vite-plugin` entonces sirve los assets por HTTPS, sin _mixed content_. Sin esas condiciones, Vite queda en `http://127.0.0.1:<puerto>`, que Chrome y Firefox aceptan en una página HTTPS por ser loopback.
- El plugin también habilita CORS para orígenes `*.test`.

Nada de esto modifica archivos del proyecto: todo va solo al entorno del proceso.

> [!WARNING]
> Next.js 15+ puede avisar sobre peticiones de desarrollo desde otro origen. Si la recarga en caliente no funciona por `<sitio>.test`, agrega el dominio en `allowedDevOrigins` del `next.config`. LocDev no modifica ese archivo por ti.

## Supervisión

- **Una instancia:** iniciar un sitio que ya corre no levanta otro proceso.
- **Puerto ocupado:** antes de iniciar, LocDev verifica el puerto y, si está en uso, dice quién lo ocupa (proceso y PID).
- **Árbol de procesos:** cada sitio corre en su propio grupo de procesos (Unix) o árbol (Windows); detenerlo tumba todo lo que creó el comando (npm → node, artisan → php). Tras 8 s sin terminar, el proceso se mata.
- **Reiniciar si falla:** opcional por sitio (Settings del sitio, o `locdev edit <sitio> --restart-on-failure true`), con esperas crecientes (1 s, 2 s, 4 s…) y como máximo 5 intentos en 2 minutos.
- **Tras una caída del daemon:** LocDev registra los procesos en ejecución; un daemon nuevo termina lo que dejó el anterior (su salida no se puede reconectar) y el auto-start vuelve a levantar lo que estaba marcado.

## Procesos y logs

![Herramientas → Procesos y logs](screenshot:processes)

En **Tools → Processes & logs** (o `locdev ps`): todos los procesos, con PID, puerto, runtime, tiempo activo, estado, reinicios, CPU y memoria (sumados en el árbol del proceso). La consola muestra stdout, stderr y las notas de LocDev, con filtros por stream y por nivel, búsqueda, pausa, desplazamiento automático y copia.

El nivel (`error`, `warn`, `info`, `debug`) solo aparece cuando la propia línea lo declara: `ERROR:`, `[warn]`, `npm ERR!`, `PHP Fatal error:`, el `local.ERROR:` de Laravel. La salida común queda sin nivel, y stderr no se trata como error por sí solo.

La salida queda en memoria (2000 líneas por proceso) y en disco, en `logs/sites/<sitio>.log` (rota a los 5 MB, guardando un archivo anterior). Por eso sigue visible después de que el proceso termina y tras reiniciar el daemon. Los límites están en `config.toml`:

```toml
[logs]
lines = 2000
file_mb = 5
```

Esto es diagnóstico por la salida del proceso. La depuración interactiva (Xdebug, el inspector de Node) no forma parte de LocDev hoy.

## Procesos auxiliares

Además del servidor de desarrollo y del auxiliar Vite, un sitio puede tener **procesos auxiliares**: workers de colas, schedulers, servidores WebSocket y herramientas con servidor propio. Los adaptadores de framework ofrecen los que el proyecto tiene instalados:

| Stack        | Procesos ofrecidos                                                               | Condición                                                     |
| ------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Laravel      | Queue worker (`php artisan queue:work`), Scheduler (`php artisan schedule:work`) | siempre                                                       |
| Laravel      | Horizon (`php artisan horizon`)                                                  | `laravel/horizon` en `composer.json`                          |
| Laravel      | Reverb (`php artisan reverb:start`)                                              | `laravel/reverb` en `composer.json`                           |
| Rails        | Solid Queue (`bin/jobs`)                                                         | gem `solid_queue` y `bin/jobs`                                |
| Rails        | Sidekiq (`bundle exec sidekiq`)                                                  | gem `sidekiq`                                                 |
| Django       | Celery worker y Celery beat (`celery -A <proyecto> …`)                           | `celery` en las dependencias; el proyecto sale de `manage.py` |
| Cualquier JS | Storybook (`<pm> run storybook --port <puerto> …`)                               | `storybook` y un script `storybook`                           |

Como el auxiliar Vite, **nada se ejecuta antes de una decisión**: cada proceso ofrecido empieza como _ask_. Los modos son _manual_ (lo inicias tú), _con el sitio_ (arranca con el sitio) y _nunca_.

Las alternativas que consumirían los mismos jobs (Horizon y `queue:work`, Solid Queue y Sidekiq) no corren juntas: iniciar una con la otra en marcha se rechaza, y elegir una como _with site_ saca a la otra de ese modo. Workers y schedulers se reinician solos si caen (con el mismo backoff del servidor). Cada proceso tiene su propio log, recibe los runtimes y las variables compartidas del sitio y se detiene con él.

También puedes agregar procesos propios (un worker de otra cola, un watcher); `{port}` en el comando recibe un puerto libre:

```bash
locdev process list
locdev process mode queue auto
locdev process start scheduler
locdev process mode queue manual --command "php artisan queue:work --queue=high,default"
locdev process add mailer --command "php artisan queue:work --queue=mail" --label "Mail queue"
locdev process logs queue -f
```

En la aplicación, la pestaña **Servidor** del sitio lista los procesos con el modo, los controles y la consola de cada uno. En **Tools → Processes & logs** aparecen los que están activos.
