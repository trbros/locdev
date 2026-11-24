# Configuración de PHP

En **Runtimes → PHP → configuración** (o `locdev php`), ajustas `memory_limit`, el tamaño de subida, la zona horaria y las demás directivas de cada versión de PHP, sin editar el `php.ini` del sistema.

![Configuración de PHP: el valor del propio PHP, el de LocDev y el que está en uso.](screenshot:php-settings)

## Cómo funciona

Los sitios de LocDev se ejecutan en el **servidor integrado de PHP** (`php -S`, `artisan serve`), que lee los mismos archivos que el comando `php`: el `php.ini` cargado y los archivos de la carpeta de escaneo (`conf.d`). LocDev no cambia ninguno. Mantiene **un archivo propio por versión** (`config/php/<versión>/90-locdev.ini`, con la marca "Managed by LocDev") y lo añade a los procesos que ejecuta mediante `PHP_INI_SCAN_DIR`, después de la carpeta de escaneo de PHP. Por eso el valor de LocDev gana, y solo para los sitios de LocDev: el `php` de tu terminal sigue igual.

Para cada directiva la pantalla muestra tres valores:

| Columna    | De dónde viene                                                                      |
| ---------- | ----------------------------------------------------------------------------------- |
| **LocDev** | el archivo de LocDev (vacío: no lo toca)                                            |
| **De PHP** | lo que PHP lee sin LocDev: `php.ini`, la carpeta de escaneo o el valor de PHP mismo |
| **En uso** | lo que PHP carga de verdad con el archivo de LocDev (resaltado cuando difiere)      |

Nada tiene un valor predeterminado inventado: mientras no cambies una directiva, vale la de tu PHP.

Las versiones se agrupan por línea (`8.3`, `8.4`): la configuración de 8.3 vale para cualquier 8.3.x que usen los sitios. La pantalla muestra qué instalación se leyó, el `php.ini` cargado y los archivos de la carpeta de escaneo, que puedes abrir **en solo lectura**.

## Directivas

| Directiva                | Tipo                                          |
| ------------------------ | --------------------------------------------- |
| `memory_limit`           | tamaño (`512M`, `2G`, `-1` = sin límite)      |
| `upload_max_filesize`    | tamaño                                        |
| `post_max_size`          | tamaño                                        |
| `max_file_uploads`       | número                                        |
| `max_execution_time`     | segundos (`0` = sin límite)                   |
| `max_input_time`         | segundos (`-1` = usa el límite de ejecución)  |
| `max_input_vars`         | número                                        |
| `display_errors`         | On/Off                                        |
| `display_startup_errors` | On/Off                                        |
| `log_errors`             | On/Off                                        |
| `error_reporting`        | constantes (`E_ALL & ~E_DEPRECATED`) o número |
| `date.timezone`          | zona horaria (`America/Sao_Paulo`, `UTC`)     |
| `default_socket_timeout` | segundos                                      |
| `realpath_cache_size`    | tamaño                                        |
| `opcache.enable_cli`     | On/Off (OPcache en el servidor integrado)     |

Los valores con formato incorrecto se rechazan antes de escribir nada. La pantalla avisa de combinaciones que no funcionan juntas (`upload_max_filesize` mayor que `post_max_size`, `post_max_size` mayor que `memory_limit`) y de una zona horaria que PHP no conoce.

> [!NOTE]
> La línea de comandos de PHP fuerza `max_execution_time = 0` y `max_input_time = -1`, pero el servidor integrado, que sirve los sitios, usa el valor de los archivos. La pantalla muestra el valor que reciben los sitios. Los comandos (`artisan`, `composer`) siguen sin límite de tiempo.

## Aplicar un cambio

1. Cambia los valores en la columna **LocDev** (vaciar un campo vuelve al valor de PHP).
2. **Revisar cambios** muestra las líneas que entran y salen del archivo de LocDev y qué sitios en ejecución se reiniciarán.
3. Al **Aplicar**, LocDev guarda el archivo actual en el historial, escribe el nuevo y **comprueba con el propio PHP** lo que carga. Si PHP lo rechaza (error de sintaxis, valor ignorado), vuelve el archivo anterior y nada más cambia.
4. Solo se reinician los sitios en ejecución con esa versión de PHP, cada uno con todos sus procesos (servidor, Vite, colas). La app y el daemon siguen abiertos.

**Usar los valores de PHP** borra el archivo de LocDev de esa versión (también pasa por la revisión).

## Configuración avanzada

**Configuración avanzada** abre el archivo de LocDev en un editor, con resaltado y `Ctrl+S`. Puede ir cualquier directiva que PHP acepte (`xdebug.mode`, `opcache.*`…), y las líneas y comentarios que escribiste se mantienen cuando cambias algo desde el formulario. Antes de aplicarlo, PHP comprueba el archivo en una carpeta temporal: un error de sintaxis se muestra y no se escribe nada. Si el archivo cambió desde que lo abriste (en otra ventana o desde la CLI), guardar se rechaza para que lo recargues.

El `php.ini` del sistema y los archivos de la carpeta de escaneo aparecen **en solo lectura**: para cambiar uno de sus valores, sobrescríbelo en el archivo de LocDev.

## Historial

Cada cambio guarda el archivo que reemplazó (los últimos 30 por versión, en `data/state/php-ini-history/<versión>/`). **Historial → Restaurar…** vuelve a una versión anterior, con la misma revisión y comprobación.

## En la línea de comandos

```bash
locdev php lines                                   # versiones y si LocDev configura alguna
locdev php show                                    # la versión predeterminada de LocDev
locdev php show --line 8.3
locdev php set memory_limit=512M upload_max_filesize=64M post_max_size=64M
locdev php set date.timezone=America/Sao_Paulo --line 8.3 --yes
locdev php unset memory_limit
locdev php reset --line 8.3
locdev php edit                                    # el archivo de LocDev en $EDITOR
locdev php file /etc/php/8.3/cli/php.ini           # un archivo que carga PHP, solo lectura
locdev php history
locdev php restore 1793830885840
```

`set`, `unset`, `reset`, `edit` y `restore` muestran el cambio y preguntan antes (`--yes` omite la pregunta).

## Límites

- Vale para los procesos que ejecuta LocDev (sitios, servidores auxiliares, colas). El `php` que ejecutas en la terminal no recibe el archivo de LocDev.
- LocDev no usa PHP-FPM ni Apache/Nginx; la diferencia entre la CLI y el servidor web está solo en los dos límites de tiempo de arriba.
- Validado en Linux con el PHP del sistema y el de Yerd (un sitio real servido por el servidor integrado recibiendo los valores nuevos después del reinicio). Windows y macOS no se probaron; en Windows el separador de `PHP_INI_SCAN_DIR` es `;`.
