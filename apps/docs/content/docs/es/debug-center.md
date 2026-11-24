# Debug Center

En **Herramientas → Debug Center** (`Ctrl` `Shift` `D`) o con `locdev debug`, LocDev reúne en un solo lugar lo que sale mal en tus sitios: excepciones con stack trace, `dump()`/`dd()` de PHP, líneas de log de aviso y error, respuestas 5xx, procesos que se cayeron, builds que fallaron y tareas de LocDev con error. Los secretos se ocultan antes de guardar nada.

![Debug Center: la línea de tiempo, una excepción con su causa probable y el stack trace.](screenshot:debug-center)

## De dónde vienen los eventos

| Fuente                                                                            | Qué se convierte en evento                                                                                          |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Salida de los procesos (sitio, servidor auxiliar, procesos auxiliares, servicios) | Excepciones y stack traces, errores de build, avisos de PHP y líneas marcadas como aviso o error                    |
| `storage/logs` de Laravel (`laravel.log` y el diario del día)                     | Entradas desde el nivel elegido, con la excepción y la pila; solo lo que se escribe después de que el sitio arranca |
| Proxy de LocDev                                                                   | Respuestas 5xx y sitios que no respondieron (método, dirección, estado y tiempo)                                    |
| Supervisor                                                                        | Procesos que terminaron con error, con sus últimas líneas de salida                                                 |
| Servidor de dumps de LocDev                                                       | `dump()` y `dd()` de los sitios PHP con la captura activada                                                         |
| Cola de tareas                                                                    | Tareas que fallaron (comprobación de dependencias, instalaciones…)                                                  |

LocDev solo lee: no cambia la configuración de log del proyecto ni sus archivos.

### Por runtime

| Runtime | Qué reconoce LocDev                                                                                                                                                                                                  |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PHP     | `PHP Fatal error: Uncaught …` con `Stack trace:`; `PHP Warning`/`Deprecated` con archivo y línea; el formato del servidor integrado (`[500]: GET /ruta - Uncaught …`); el log de Laravel; dumps de Symfony VarDumper |
| Node.js | `TypeError: …` con las líneas `at función (archivo:línea:columna)`; errores de Vite (`[vite] Internal server error`) y de Next.js (`⨯ …`, `Module not found`); errores de TypeScript (`error TS2322`)                |
| Python  | `Traceback (most recent call last):` hasta la línea `Tipo: mensaje`; el formato por defecto de `logging` (`ERROR:logger:mensaje`)                                                                                    |
| Rust    | Panics (`thread 'main' panicked at archivo:línea:columna:`); errores de cargo (`error[E0425]`); el formato por defecto de `tracing`                                                                                  |

Los marcos de dependencias (`vendor`, `node_modules`, `site-packages`, el registro de cargo) se pliegan en la pila. Las demás líneas se convierten en eventos desde el nivel elegido (por defecto: avisos) cuando traen un nivel explícito (`ERROR`, `[warn]`, `npm ERR!`…).

## La pantalla

- **Línea de tiempo:** los eventos más nuevos primero, actualizados en vivo (pausa con **En vivo**).
- **Problemas:** las repeticiones agrupadas (mismo tipo de error, mismo mensaje sin números ni valores, mismo lugar en el código del proyecto), con el total y un gráfico por hora de las últimas 24 horas.
- **Filtros:** sitio (a la izquierda), tipo, nivel y búsqueda por mensaje, tipo de error, archivo o URL.
- **Detalle:** mensaje, tipo de error, framework, runtime, archivo y línea, cuándo ocurrió, la petición (método, URL, estado, tiempo y controller, cuando se conocen), el stack trace, el valor del dump, el contexto y el texto tal como se escribió.
- **Causa probable:** para errores conocidos (la base de datos rechaza la conexión, puerto en uso, falta `APP_KEY`, clase o módulo no encontrado, falta el manifiesto de Vite, memoria de PHP…), una pista de lo que suele causarlo.
- **Acciones:** **Copiar** (texto para pegar en una issue), **JSON**, **Apariciones** (solo las repeticiones de ese problema), **Exportar** (JSON Lines) y abrir el archivo en la línea en tu [editor de código](/docs/editors) (en el evento y en cada marco de la pila dentro del sitio).

## Dumps de PHP (`dump()` y `dd()`)

![Un dump: el objeto en árbol, con las visibilidades y los valores sensibles ocultos.](screenshot:debug-dump)

Para sitios PHP con Symfony VarDumper (Laravel, Symfony), elige el sitio a la izquierda y activa **Capturar dump() y dd()** (o `locdev debug dumps <sitio> on`). El sitio se reinicia. Desde entonces los dumps aparecen en el Debug Center en árbol (arrays, objetos con propiedades públicas `+`, protegidas `#` y privadas `-`, recursiones marcadas) en lugar de en la página.

Cómo funciona, sin cambiar ningún archivo del proyecto:

- LocDev define `VAR_DUMPER_FORMAT=server` y `VAR_DUMPER_SERVER=127.0.0.1:<puerto>` **solo en el entorno del proceso del sitio**. Es el formato `server` del propio VarDumper, que Laravel respeta.
- El servidor integrado de PHP (`php -S`, que es lo que usa `artisan serve`) no pasa las variables de entorno a `$_SERVER`, el único lugar donde mira VarDumper. Por eso, solo para esos sitios, LocDev añade su propio `auto_prepend_file` (en `data/php-dumps/`, marcado "Managed by LocDev") que copia esas dos variables a `$_SERVER`. Si tu PHP ya tenía un `auto_prepend_file` (el de Herd, por ejemplo), sigue ejecutándose justo después.
- Si el servidor de dumps de LocDev no responde, VarDumper imprime el dump como siempre.
- `dd()` sigue terminando la petición: LocDev muestra el valor, pero no conserva lo que vendría después.

LocDev lee el formato de serialización de PHP por sí mismo, sin ejecutar PHP ni llamar a `unserialize`. Los procesos auxiliares que LocDev ejecuta para el sitio (colas, programador) reciben el mismo entorno, así que sus dumps también llegan, con la línea de comandos en el contexto.

## Secretos

Antes de guardar o mostrar cualquier evento, LocDev oculta (`[redacted]`):

- los valores del `.env` y del `.env.local` del sitio, salvo los que son solo configuración (hosts, puertos, drivers, nombres, niveles, `true`/`false`, números);
- los valores de las [variables compartidas y secretos](/docs/environment) que LocDev inyecta en el sitio;
- las cabeceras `Authorization`, `Cookie`, `Set-Cookie` y `X-Api-Key`; tokens `Bearer`; JWTs; `password=`, `token=`, `secret=`, `api_key=` y similares en URLs y textos; contraseñas en URLs (`postgres://user:contraseña@…`);
- en JSON y en dumps, los valores de claves con nombres como `password`, `token`, `secret`, `api_key`, `cookie`, `session` o que terminan en `_key`.

Esto cubre lo que es identificable; un secreto sin nombre reconocible, en un lugar sin patrón, puede pasar. Los eventos se quedan solo en esta máquina.

## Almacenamiento y limpieza

Los eventos se guardan en `data/observe/<sitio>/<día>.jsonl`, y los más recientes también en memoria. En **Configuración → Automatizaciones → Debug Center**:

- **Recopilar eventos** (activado por defecto);
- **Conservar durante** (por defecto: 7 días) y **Usar como máximo** (por defecto: 100 MB): cuando se alcanza un límite, se borran primero los días más antiguos;
- **Líneas de log normales desde:** info, avisos (por defecto) o errores.

**Limpiar** borra los eventos de un sitio (o de todos), en memoria y en disco. Las mismas opciones están en `settings.toml` (`[observe] enabled`, `retention_days`, `max_mb`, `logs_from`).

## Línea de comandos

```bash
locdev debug events
locdev debug errors
locdev debug status
```

- `locdev debug events -f` sigue en vivo; `--site`, `--kind` (repetible), `--level` y `--search` filtran; `--json` para scripts.
- `locdev debug errors` agrupa las repeticiones, con el gráfico de las últimas 24 horas.
- `locdev debug show <id>` muestra un evento completo (pila, petición, dump, causa probable).
- `locdev debug export` escribe JSON Lines; `locdev debug clear --site <sitio>` limpia; `locdev debug dumps <sitio> on|off` activa y desactiva la captura de dumps.

## Limitaciones

- Validado en Linux: todo el pipeline con un sitio Python real (traceback, 500 en el proxy, secreto del `.env` oculto, agrupación) y con PHP 8.5 + Symfony VarDumper 7.4 reales mediante el servidor integrado (dump capturado sin aparecer en la página, excepción con la petición). No se probó con una aplicación Laravel completa, ni en Windows ni en macOS.
- Los formatos de Node.js, Next.js, Vite, Python y Rust siguen la salida por defecto de cada uno; los logs con un formato propio (JSON estructurado, por ejemplo) solo se convierten en eventos cuando traen un nivel explícito.
- El log de Laravel se lee para los canales `single` y `daily` en `storage/logs`; otros destinos (Papertrail, syslog, stderr) no.
- Los errores que solo ocurren en el navegador (JavaScript del front-end) no pasan por LocDev.
