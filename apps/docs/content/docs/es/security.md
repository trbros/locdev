# Seguridad

LocDev toca el DNS, los certificados y los procesos, así que se diseñó para requerir poca confianza.

## Privilegios

- El daemon, la CLI y la aplicación corren con tu usuario.
- Los cambios en el sistema (resolvedor, almacén de confianza) solo ocurren con un script **visible** y **confirmado**, ejecutado por el mecanismo de autorización del SO.
- Todo lo que LocDev escribe en el sistema lleva la marca `Managed by LocDev`, y los comandos de desinstalación solo quitan lo que tiene esa marca.
- La configuración de otras herramientas se detecta y se muestra, nunca se edita.

## Red

- El proxy, HTTPS y el DNS escuchan solo en `127.0.0.1` (y `::1`), nunca en la red.
- El servidor DNS solo responde por la zona `.test`, rechaza las demás e ignora los paquetes que ya son respuestas, evitando bucles de reflexión.
- El IPC usa un socket con permiso `0600` en un directorio `0700` de tu usuario (Unix) o un named pipe que rechaza clientes remotos (Windows).

## Certificados

- La clave de la CA queda en `0600` en la carpeta de datos del usuario.
- La CA tiene **Name Constraints** (`test`, `localhost`) y `pathlen:0`: no puede emitir certificados válidos para otros dominios ni CA intermedias.
- Confiar en la CA es opcional; sin eso, HTTPS funciona, pero los navegadores avisan.

## Procesos

- Los sitios corren con tu usuario, en el directorio del proyecto.
- Detener un sitio termina todo el árbol de procesos, para no dejar huérfanos.
- Quitar un sitio nunca borra archivos.

## Git y GitHub

- Las operaciones destructivas de Git (reset, descartar, _force push_…) muestran lo que se perderá y exigen una frase escrita. Opcionalmente, también piden una contraseña (hash Argon2id, bloqueo progresivo). Antes de ejecutarse, LocDev guarda una copia. La contraseña protege solo lo que inicia LocDev, no el repositorio. Consulta [Git, GitHub y protección](/docs/git).
- El token de GitHub queda solo en el llavero del sistema, nunca en un archivo o log. El daemon solo se comunica con `api.github.com`, y toda acción remota pide confirmación.
- `git` se ejecuta sin shell, con argumentos separados y nombres validados, y nunca pide credenciales de forma interactiva.

## Secrets y credenciales

- Las variables secretas se cifran (ChaCha20-Poly1305) con la clave en el llavero del sistema, y solo entran en el entorno de los procesos, nunca en archivos, argumentos ni logs ([Variables de entorno](/docs/environment)).
- Las credenciales de las bases las lee el daemon del proyecto en cada uso y no pasan por el IPC; una base remota solo se abre con una elección explícita, y el SQL destructivo exige el nombre de la base ([Database Manager](/docs/database)).

## Correo

- El Mailbox solo captura, en loopback, y nunca entrega mensajes. Reemplazar un SMTP real del proyecto exige tu consentimiento ([Mailbox](/docs/mailbox)).
- El HTML de los correos se muestra en un `iframe` con `sandbox` vacío y una política que bloquea toda la red.
- El código del proyecto solo se ejecuta cuando lo pides (vista previa por el framework, `make:mail`), en un proceso separado con entorno limpio y tiempo y salida limitados ([Email Studio](/docs/email-studio)).

## Reportar vulnerabilidades

Por favor, reporta los problemas de seguridad de forma privada a los mantenedores antes de divulgarlos públicamente.
