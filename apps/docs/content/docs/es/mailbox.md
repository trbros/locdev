# Mailbox

LocDev tiene su propia bandeja de entrada de pruebas: un servidor SMTP que **solo captura**. Escucha en `127.0.0.1:1025` (o en el siguiente puerto libre, si Mailpit ya está en él) y guarda todo lo que recibe, sin entregar nada a nadie. No hay herramienta extra que instalar.

![Tools → Mailbox: bandejas por sitio, el HTML aislado y la pestaña de debug.](screenshot:mailbox)

```bash
locdev mail enable tienda        # el correo de tienda va al Mailbox
locdev mail list                 # los mensajes, del más nuevo al más antiguo
locdev mail show <id>            # sobre, texto, enlaces, adjuntos y problemas
locdev mail raw <id> -o msg.eml  # el mensaje exactamente como llegó
locdev mail test                 # envía un mensaje de prueba al Mailbox
```

En la aplicación: **Herramientas → Mailbox**. Para activar un sitio, usa **Site → Settings → mail**.

## Cómo se conecta cada framework

LocDev no intercepta tráfico. Configura el transporte SMTP de forma explícita, **solo en el entorno de los procesos del sitio** (servidor, workers, scheduler). El `.env` del proyecto nunca se edita. Después de activarlo o desactivarlo, reinicia el sitio.

| Stack                          | Cómo                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel                        | `MAIL_MAILER=smtp`, `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME` (el nombre del sitio), `MAIL_PASSWORD`, más `MAIL_SCHEME` (Laravel 11+) o `MAIL_ENCRYPTION=null` (10 y anteriores), según lo que lea el `config/mail.php` del proyecto. Un `MAIL_URL` existente se anula, porque tiene prioridad sobre `MAIL_HOST`.                                                                                     |
| WordPress y PHP                | El `mail()` de PHP pasa por `sendmail_path`, que viene de un `.ini` de LocDev agregado con `PHP_INI_SCAN_DIR` (los `.ini` del sistema siguen aplicándose). En Windows, el `.ini` usa `SMTP`/`smtp_port`. Los plugins de SMTP (WP Mail SMTP y similares) no pasan por `mail()` y también deben apuntar a `127.0.0.1`.                                                                                  |
| Node.js, Django, Rails y otros | Si el `.env`/`.env.example` del proyecto ya tiene variables `SMTP_*`, `MAIL_*`, `EMAIL_*` o `MAILER_*` (`HOST`, `PORT`, `USER`, `PASS`, `SECURE`, `URL`…), LocDev define esas mismas variables. Si el framework lee el SMTP directamente del código (`settings.py`, `development.rb`, `nodemailer.createTransport`), la pantalla muestra el fragmento exacto que agregar en el entorno de desarrollo. |

El usuario SMTP de cada sitio es su nombre, y así el Mailbox sabe de qué proyecto vino cada mensaje. El servidor acepta cualquier contraseña y no ofrece STARTTLS (es loopback).

### SMTP real: solo con tu autorización

Si el proyecto apunta a un servidor real, activar el Mailbox **exige consentimiento explícito**: `locdev mail enable --override-real`, o la casilla "Allow replacing it" en la aplicación. Esto vale para un host que no es local o para un transporte de API como SES, Postmark, Mailgun o Resend. Las contraseñas del `.env` nunca aparecen en pantalla.

## La bandeja de entrada

- **Organización:** bandeja general y una por sitio, con búsqueda por asunto, remitente, destinatario o fragmento, y filtros de no leídos y con adjuntos.
- **Vistas:** HTML, texto, encabezados, enlaces, adjuntos (con descarga) y el `.eml` original.
- **Debug:** muestra lo que informó realmente la sesión SMTP:
  - el sobre (`MAIL FROM`, `RCPT TO`) y el `HELO`;
  - el protocolo (`ESMTPA` cuando se autenticó) y el usuario del `AUTH`;
  - la dirección de origen, la hora de llegada, el `Date` del mensaje, el tamaño y el framework del sitio.
- **Problemas de formato:** comprobaciones objetivas.
  - Falta de `From`, `Date`, `Message-ID` o `Subject`, y `Date` inválido (RFC 5322).
  - Líneas de más de 998 caracteres y contenido de 8 bits sin MIME.
  - Errores de decodificación (base64, quoted-printable, charset).
  - HTML sin alternativa en texto y `cid:` sin la parte correspondiente.

### HTML aislado

El HTML del correo se muestra en un `iframe` con `sandbox` vacío, sin scripts, formularios, navegación ni ventanas emergentes. Además, una política de seguridad bloquea cualquier carga desde la red, así que los píxeles de rastreo y las imágenes remotas no cargan. La pantalla indica cuántos recursos remotos se bloquearon y la pestaña Debug los lista. Las imágenes incrustadas (`cid:`) se ven normalmente. Los enlaces no se abren al hacer clic dentro del correo; usa la pestaña Links, que abre la dirección en el navegador.

Cada mensaje capturado también se convierte en una notificación (categoría _Correos capturados_), que el sistema muestra cuando la aplicación no está al frente, y enciende el punto verde en el icono de la bandeja ([Recorrido por la aplicación](/docs/app)). Abrir o borrar el mensaje marca la notificación como leída.

## Almacenamiento

Los mensajes quedan en `data/mail/messages` (carpeta `0700`), cada uno como `.eml` más un `.json` con el sobre. Se guardan los últimos 1.000 y, además, se rechazan los mensajes de más de 25 MB. Para borrar: `locdev mail delete`, `locdev mail clear [sitio]` o los botones de la aplicación.

> [!WARNING]
> En Laravel, LocDev ejecuta `artisan serve` con `--no-reload`. Sin eso, `serve` descarta las variables que inyecta LocDev antes de iniciar el servidor PHP (los builds de PHP vienen con `variables_order=EGPCS`). Después de editar el `.env`, reinicia el sitio.
