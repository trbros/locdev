# HTTPS y CA local

## Qué pasa por debajo

En la primera ejecución, el daemon crea una **autoridad certificadora (CA) local** en `<datos>/tls/`, con la clave en `0600`, y empieza a servir HTTPS en `127.0.0.1:443` (o 7443 si el 443 está ocupado).

El certificado de cada sitio se emite en memoria en el primer acceso, a partir del nombre pedido en el handshake (SNI), y cubre:

- `blog.test` y `*.blog.test`
- `blog.localhost` y `*.blog.localhost`

Los certificados valen 397 días y se vuelven a emitir en cada reinicio del daemon.

## Enlaces y assets en HTTPS (_mixed content_)

El proxy termina el TLS y habla HTTP simple con el servidor de desarrollo del sitio, indicando el protocolo y el puerto originales en `X-Forwarded-Proto` y `X-Forwarded-Port` (además de `X-Forwarded-Host` y `X-Forwarded-For`). Los frameworks solo leen esas cabeceras de proxies en los que se les indicó confiar (el `TrustProxies` de Laravel, el `trusted_proxies` de Symfony). Sin eso, una página abierta en `https://` enlaza sus propios assets con `http://`, y el navegador los bloquea como _mixed content_. LocDev lo resuelve en todos los proyectos, sin modificar sus archivos:

- **PHP (Laravel, Symfony, WordPress, Bedrock, PHP puro):** todo sitio PHP carga el `auto_prepend_file` de LocDev (en `<datos>/php-prepend/`, marcado "Managed by LocDev" y añadido mediante `PHP_INI_SCAN_DIR`). En una petición que llegó al proxy por HTTPS, define `$_SERVER['HTTPS'] = 'on'`, `REQUEST_SCHEME` y `SERVER_PORT` como lo haría un servidor TLS. Así, `asset()` y `url()`, la barra de Symfony (`/_wdt`) y el `is_ssl()` de WordPress generan `https://`. Solo confía en esas cabeceras en conexiones desde loopback, desde donde habla el proxy, y el proxy siempre las sobrescribe. Si tu PHP ya tenía un `auto_prepend_file`, sigue ejecutándose justo después.
- **Redirecciones (todas las stacks):** en una petición HTTPS, una respuesta con `Location: http://<el mismo host>/…` pasa a apuntar a `https://`. Esto cubre las apps que arman la URL a partir de una configuración en `http://` (como el `WP_HOME` de Bedrock) y evita que un sitio solo HTTPS rebote entre los dos protocolos. Otros hosts y las direcciones relativas no cambian.
- **Next.js:** al servidor de desarrollo se le dice que la petición llegó por HTTP. Next arma su propia dirección con `127.0.0.1` mientras su middleware ve `localhost`, así que un rewrite del middleware (el prefijo de idioma de next-intl, por ejemplo) parece externo y Next se hace proxy a sí mismo, por TLS cuando se le dice HTTPS: `Failed to proxy … EPROTO … wrong version number`. Sus redirecciones siguen siendo relativas, y el navegador sigue en HTTPS.
- **Vite junto a Laravel:** el certificado que recibe Vite cubre también `localhost`, `127.0.0.1` y `::1` (ver **Servidores de desarrollo**).

## Por qué confiar en esta CA es de bajo riesgo

La CA se emite con **Name Constraints** (`permitted: test, localhost`) y `pathlen:0`. Los navegadores y las bibliotecas TLS rechazan cualquier certificado firmado por ella para otro dominio. Aunque la clave se filtrara, no serviría para interceptar `google.com` ni tu banco.

Esto tiene una prueba automatizada: un certificado `example.com` firmado por la propia CA es rechazado por el verificador de rustls con un error de _NameConstraint_.

```bash
openssl x509 -in ~/.local/share/locdev/tls/ca.pem -noout -ext nameConstraints
# X509v3 Name Constraints: critical
#     Permitted:
#       DNS:test
#       DNS:localhost
```

## Confiar en la CA

Confiar es **opcional y explícito**:

```bash
locdev tls trust
```

El plan tiene dos pasos, ambos mostrados antes de ejecutarse:

| SO               | Como administrador        | Como tú                                      |
| ---------------- | ------------------------- | -------------------------------------------- |
| Debian/Ubuntu    | `update-ca-certificates`  | NSS de Chrome y Firefox (`certutil`)         |
| Fedora/RHEL/Arch | `update-ca-trust`         | NSS de Chrome y Firefox (`certutil`)         |
| macOS            | System keychain           | (los navegadores usan el keychain)           |
| Windows          | `Cert:\LocalMachine\Root` | (los navegadores usan el almacén de Windows) |

![Settings → https: estado en el sistema y en los navegadores, y la huella SHA-256 de la CA.](screenshot:settings-https)

![Plan de confianza de la CA con sus dos pasos](screenshot:authorize-tls)

Después de confiar, reinicia el navegador. Las URL de los sitios pasan a usar `https://` automáticamente.

```bash
locdev tls status    # puerto, ruta y huella de la CA, estado de la confianza
locdev tls untrust   # la quita del sistema y de los navegadores
```

> [!NOTE]
> En Linux, Chrome y Firefox mantienen su propia base de certificados (NSS). LocDev encuentra las bases de Chrome/Chromium y de los perfiles de Firefox, incluidas las versiones snap y flatpak. Para actualizarlas se necesita `certutil` (`libnss3-tools`).

## Comprobar la huella

La huella SHA-256 aparece en `locdev tls status` y en la aplicación. Compárala con el emisor que muestra el navegador al inspeccionar el certificado de un sitio `.test`.

## `locdev secure`

`locdev secure [sitio]` deja un sitio solo en HTTPS y comprueba todo lo que eso requiere: el listener HTTPS, el certificado del sitio, la resolución de `*.test` y la confianza en la CA. Lo que requiere administrador (confiar en la CA, enrutar `*.test`) usa los mismos planes que `locdev tls trust` y `locdev dns install`: el script se muestra y solo se ejecuta tras tu confirmación (o con `--yes`). Nada se instala dos veces y no hay una segunda implementación de TLS: es la misma CA local.

```bash
cd ~/Code/shop
locdev secure          # comprueba, corrige con confirmación y activa HTTPS-only
locdev secure --off    # vuelve a servir también HTTP
```

Con HTTPS-only, una petición `http://shop.test` recibe una redirección `307` a `https://shop.test` (el `307` mantiene el método y el cuerpo, así que un POST sigue siendo POST). En la aplicación, lo mismo está en **Configuración** del sitio → **HTTPS**, con el diagnóstico por sitio.
