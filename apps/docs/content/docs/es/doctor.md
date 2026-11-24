# Solución de problemas

Empieza siempre por:

```bash
locdev doctor
```

![Doctor](screenshot:doctor)

## Qué se comprueba

| Comprobación                   | Qué significa                                                                                                                                                                 |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Daemon                         | Si la CLI puede hablar con `locdevd`, y si coinciden las versiones del protocolo.                                                                                             |
| Configuration folder           | Si se puede escribir en la carpeta de configuración.                                                                                                                          |
| Project folders                | Si las carpetas registradas siguen existiendo.                                                                                                                                |
| HTTP proxy / HTTPS proxy       | Si el proxy está escuchando y si tuvo que pasar al puerto de respaldo (y por qué).                                                                                            |
| HTTP port 80 / HTTPS port 443  | Si los puertos están libres, en uso por otro programa o por el propio LocDev.                                                                                                 |
| Local CA                       | Si el sistema confía en la CA local.                                                                                                                                          |
| Browser certificate stores     | Si Chrome/Firefox (Linux) confían en la CA.                                                                                                                                   |
| DNS server                     | Si el servidor de `*.test` está activo.                                                                                                                                       |
| .test resolver                 | A dónde va realmente `*.test`: LocDev, otra herramienta o ningún lugar.                                                                                                       |
| PHP, Composer, Node.js, npm    | Si las herramientas están en el `PATH` (y si algún sitio registrado las necesita).                                                                                            |
| Runtimes de cada sitio         | Versión fijada que no está instalada, versión fuera del rango que declara el proyecto, versión sin soporte del publicador, gestor de paquetes que exige un runtime más nuevo. |
| Predeterminada de cada runtime | Si la predeterminada definida en `runtimes.toml` sigue instalada.                                                                                                             |
| Secrets vault                  | Si la clave del almacén de variables secretas es accesible (llavero o archivo).                                                                                               |
| Framework definitions          | La revisión de las definiciones en uso, su origen (integrada o descargada con firma verificada) y la última comprobación.                                                     |

## Problemas comunes

### `the daemon is not running`

```bash
locdev start
```

Si no arranca, revisa el log del daemon, `logs/locdevd.log` en la carpeta de datos (en Linux, `~/.local/share/locdev/logs/locdevd.log`; la tabla de carpetas está en [Configuración](/docs/configuration)).

### Puerto 80/443 en uso

Otra herramienta (Apache, nginx, Valet, Herd, Yerd…) ocupa el puerto. LocDev pasa a usar el 7080/7443 y lo muestra en la barra de estado. Libera el puerto o cambia `proxy.http_port` / `tls.https_port`.

### `*.test → another tool`

Otra herramienta configuró el resolvedor para `.test`. El `doctor` muestra su archivo. Elimínalo si ya no usas esa herramienta y ejecuta `locdev dns install`.

### El navegador sigue avisando sobre el certificado

1. Comprueba que `locdev tls status` muestre `system trusted` y `browsers trusted`.
2. Reinicia el navegador por completo.
3. En Linux, instala `libnss3-tools` y vuelve a ejecutar `locdev tls trust`.

### Un proyecto Vite responde "Blocked request"

En los procesos que inicia LocDev, los nombres del sitio ya están permitidos (consulta [Servidores de desarrollo](/docs/servers)). Si ejecutas Vite fuera de LocDev, agrega `server: { allowedHosts: [".test"] }` al `vite.config`, o usa la URL `.localhost`.

### El sitio falla al iniciar

Abre el sitio en la aplicación o ejecuta `locdev logs <nombre>`. La salida del proceso muestra el error, por ejemplo una extensión de PHP que falta o una dependencia no instalada.
