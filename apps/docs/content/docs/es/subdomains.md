# Subdominios y comodines

Cada sitio responde en su propio dominio (`tienda.test`) y, por defecto, en **cualquier subdominio**: `admin.tienda.test`, `api.tienda.test`, `cliente1.tienda.test`. Ese es el comodín `*.tienda.test`. El proxy pasa el encabezado `Host` intacto (más `X-Forwarded-Host`), así que una aplicación multi-tenant descubre el tenant por el nombre.

## Reglas

Una regla envía un subdominio a otro lugar:

| Destino             | Ejemplo                | Uso                                                                |
| ------------------- | ---------------------- | ------------------------------------------------------------------ |
| otro puerto         | `api → 127.0.0.1:9000` | una API que corre fuera de LocDev                                  |
| un proceso auxiliar | `ws → reverb`          | el servidor WebSocket del sitio, en el puerto que le asignó LocDev |
| el propio sitio     | `admin → site`         | con el comodín desactivado, permitir solo algunos subdominios      |

El patrón es la parte anterior al dominio del sitio: `api`, `admin.eu` o un comodín al inicio, como `*.tenants`, que vale para **uno o más niveles** (`acme.tenants`, `a.b.tenants`).

Cuando varias reglas coinciden, gana la más específica: la exacta vence al comodín y, entre comodines, gana el sufijo más largo. Con `*.tenants`, `vip.tenants` y `*.eu.tenants`, el nombre `vip.tenants.tienda.test` va a `vip.tenants`, y `acme.eu.tenants.tienda.test` va a `*.eu.tenants`.

```bash
locdev subdomain add api tienda --port 9000
locdev subdomain add '*.tenants' tienda
locdev subdomain add ws tienda --process reverb
locdev subdomain list tienda
locdev subdomain remove api tienda
```

En la aplicación, la sección está en **Site → Settings → subdomains**.

## Desactivar el comodín

```bash
locdev subdomain wildcard off tienda
```

Con el comodín desactivado, solo responden el dominio del sitio y los subdominios con regla. Los demás reciben una página 404 que explica el motivo.

## HTTPS

El certificado de cada nombre lo emite al momento la CA local, a partir del nombre pedido en la conexión (SNI). Cubre `tienda.test`, `*.tienda.test` y, para nombres más profundos, el comodín del nivel correcto: `cliente1.tenants.tienda.test` recibe un certificado con `*.tenants.tienda.test`. Un comodín solo cubre un nivel, por eso `*.tienda.test` solo no bastaría. La CA solo emite para `.test` y `.localhost` (restricción de nombres), y confiar en ella sigue requiriendo tu consentimiento (`locdev tls trust`).

Los WebSockets pasan por el proxy en todos los subdominios, por HTTP y HTTPS.

## Probar un nombre

```bash
locdev route api.tienda.test
```

```text
api.tienda.test
  ok   dns    127.0.0.1 through LocDev's DNS server
  ok   route  tienda: port 9000 (rule api)
  ok   https  served on :443, certificate for tienda.test, *.tienda.test, …
```

Cada línea es un paso separado:
- **dns:** ¿el nombre llega a esta máquina? ¿Y respondió el DNS de LocDev?
- **route:** a dónde envía el proxy el nombre, y si algo responde en ese puerto ahora.
- **https:** qué nombres cubre el certificado, y si la CA es confiable.

El comando termina con error cuando falla el DNS o la ruta. En la aplicación, la misma prueba está al final de la sección de subdominios.
