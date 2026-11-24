# Túneles

En **Herramientas → Túneles**, desde el menú de un sitio (**Compartir en internet…**) o con `locdev tunnel`, das a un sitio en ejecución una **dirección pública** temporal, para mostrarlo a alguien o recibir un webhook.

![Túneles: los sitios compartidos, la dirección pública y el proveedor.](screenshot:tunnels)

## Cómo funciona

LocDev no tiene una infraestructura de túneles propia: ejecuta el programa del proveedor (`cloudflared`) para un sitio, bajo el supervisor (`@tunnel:<sitio>`), y lee la dirección pública en el servidor de métricas local del programa. Solo se comparte **el servidor de ese sitio** (`http://127.0.0.1:<puerto del sitio>`, con el dominio del sitio como `Host`): nunca el proxy, una base de datos, el panel ni el propio LocDev. El sitio tiene que estar en ejecución.

Los túneles terminan cuando los detienes, cuando se quita el sitio y cuando LocDev se detiene. No vuelven solos tras un reinicio, porque la dirección cambia en cada inicio.

## Proveedor: Cloudflare Tunnel (Quick Tunnel)

|           |                                                                                                                                              |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Cuenta    | no hace falta (ni dominio)                                                                                                                   |
| Dirección | aleatoria, `https://….trycloudflare.com`, cambia en cada inicio                                                                              |
| Programa  | `cloudflared`, Apache-2.0                                                                                                                    |
| Límites   | hasta 200 peticiones a la vez (más reciben 429); Server-Sent Events no funcionan; sin garantía de disponibilidad                             |
| Uso       | para pruebas y desarrollo; sujeto a los [Términos de Uso de los Servicios en Línea de Cloudflare](https://www.cloudflare.com/website-terms/) |

Fuente: [documentación de los Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/).

Por qué este primero: es gratuito, no necesita cuenta ni dominio, el programa es open source y tiene builds oficiales para Linux, macOS y Windows. Otros proyectos open source (frp, zrok) requieren un servidor propio o una cuenta en el servicio; la arquitectura de proveedores de LocDev queda abierta para ellos, pero **hoy solo está disponible Cloudflare**.

### Protección por correo

En **Quién puede abrirlo**, indica correos (`ana@ejemplo.com`) o un dominio entero (`*@acme.com`). El visitante escribe su correo y lo confirma con un **PIN de un solo uso** antes de ver el sitio; no necesita cuenta en Cloudflare. Sin correos, **cualquiera con la dirección** puede abrir el sitio, y la pantalla lo avisa. Para cambiar quién tiene acceso, detén y vuelve a compartir. Requiere `cloudflared` 2026.9.2 o más reciente.

## Instalar cloudflared

**Herramientas → Túneles → Instalar** (o `locdev tunnel install`) descarga `cloudflared` de la release oficial de Cloudflare en GitHub y solo lo instala cuando **dos fuentes de checksum coinciden**: el SHA-256 que Cloudflare lista en las notas de la release y el que GitHub calcula del archivo publicado. La descarga se comprueba contra ese valor; si algo no coincide, no se instala nada. El programa queda en `data/tools/cloudflared/<versión>/`.

Un `cloudflared` que ya está en el `PATH` (instalado con Homebrew, apt, winget…) se usa tal cual.

> [!NOTE]
> En la release 2026.10.7, el checksum de `cloudflared-darwin-arm64.tgz` en las notas no coincide con el archivo que sirve GitHub. En ese caso LocDev rechaza la instalación; en macOS, instálalo con `brew install cloudflared`.

## Compartir

En la app: **Compartir en internet…** en el menú del sitio o en la barra de direcciones, o **Compartir un sitio** en Herramientas → Túneles. El diálogo muestra lo que significa compartir (los límites de arriba) antes de empezar, y la dirección cuando llega, con copiar y abrir.

![Compartir en internet: quién puede abrirlo y los límites del proveedor.](screenshot:tunnel-share)

```bash
locdev tunnel providers                               # proveedores, límites y el programa
locdev tunnel install cloudflare
locdev tunnel start tienda                            # pregunta antes y muestra la dirección
locdev tunnel start tienda --allow ana@ejemplo.com --allow '*@acme.com'
locdev tunnel list
locdev tunnel logs tienda
locdev tunnel stop tienda
```

`start` muestra los límites y pregunta antes de exponer el sitio (`--yes` omite la pregunta).

## Límites

- Los servidores auxiliares que la página carga desde otro origen (Vite en modo dev junto a Laravel, por ejemplo) no pasan por el túnel: el visitante no alcanza `*.test`. Usa los assets compilados (`npm run build`) al compartir.
- Lo que el propio sitio permite sigue valiendo: el túnel no añade login a un sitio compartido sin correos.
- Validado en Linux con un programa en lugar de `cloudflared` que imita su servidor de métricas (inicio, dirección, correos, detener, quitar el sitio). El `cloudflared` real y la descarga verificada no se ejecutaron en esta máquina. Windows y macOS no se probaron.
