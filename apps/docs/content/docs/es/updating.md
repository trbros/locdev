# Actualizar LocDev

En **Configuración → Actualizaciones** la app busca versiones nuevas, las instala comprobando la firma y vuelve a una versión anterior si la nueva da problemas. El daemon y la CLI vienen dentro de la app; una CLI instalada por separado se actualiza con su script de instalación. `locdev update` muestra lo que se ha publicado.

![Actualizaciones: la app, el daemon y la CLI, y las versiones anteriores.](screenshot:updates)

> [!WARNING]
> Todavía no se ha publicado ninguna release de LocDev. Hasta la primera, la pantalla indica que no encontró ninguna, y el resto de esta página describe cómo funciona la actualización cuando la haya. Las releases deben estar en un repositorio público: la app y el daemon las leen sin token.

## La app

La app usa el actualizador de Tauri. Cada release trae un `latest.json` y los archivos de actualización **firmados con la clave de release de LocDev**; la app solo instala lo que la firma confirma (la clave pública viene dentro de la app) y rechaza lo demás. La interfaz solo elige la versión: la propia app arma la dirección, siempre en las releases de LocDev en GitHub.

| Sistema                | Actualización desde la app                                                                                                                   |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows                | Sí (instalador NSIS, sin pedir administrador). El daemon se detiene antes de instalar, porque Windows no reemplaza un programa en ejecución. |
| macOS                  | Sí.                                                                                                                                          |
| Linux, AppImage        | Sí.                                                                                                                                          |
| Linux, `.deb` o `.rpm` | Sí: la app descarga el paquete del mismo tipo de la release y lo instala con la herramienta de paquetes del sistema (`pkexec dpkg -i` o `rpm -U`), que pide tu contraseña. Una copia instalada antes de este soporte (0.19.1 o anterior, o las primeras builds de la 0.19.2) necesita una actualización manual. |

Una copia de desarrollo (`make app`) tampoco se actualiza.

- **Buscar actualizaciones automáticamente** (activado por defecto): al abrir la app y una vez al día. Una versión nueva aparece como aviso, con un botón que abre la pantalla.
- **Instalar actualizaciones automáticamente** (desactivado por defecto): descarga e instala en cuanto encuentra una. LocDev nunca se reinicia solo.

Tras instalar, **Reiniciar LocDev** cierra la app, detiene el daemon y abre la nueva versión, que inicia el daemon que trae. Los sitios se detienen un momento.

Ambas opciones viven en la app (almacenamiento local), no en `settings.toml`.

## El daemon y la CLI

El daemon lee las releases publicadas en GitHub, sin token, y las compara con su propia versión. Cada release trae un `versions.json` con las versiones de la app y del daemon que contiene (el workflow de release rechaza una tag que no sea la versión de la app).

```bash
locdev update
```

La respuesta se guarda 6 horas; `--refresh` pregunta a GitHub en el momento y `--json` da la salida para scripts.

- Una vez al día (no en modo sin conexión), el daemon comprueba por su cuenta y envía **una notificación por cada release nueva**. Desactívalo con **Avisar de nuevas releases** o con `[automation] updates = false` en `settings.toml`.
- El daemon **nunca instala nada**. Con la app, actualizar la app actualiza el daemon y la CLI. Si el daemon en ejecución es distinto del que vino con la app (justo después de actualizar, por ejemplo), la pantalla muestra ambas versiones y el botón **Reiniciar el daemon**.
- La CLI independiente se actualiza con el mismo script de instalación, que comprueba el archivo contra el `SHA256SUMS` de la release:

```sh
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.14.0` instala una versión concreta (también sirve para volver atrás).

El repositorio consultado es `[updates] repo` en `config.toml` (por defecto `trbros/locdev`); consulta [Configuración](/docs/configuration).

## Volver a una versión anterior

En **Versiones anteriores**, **Volver a esta versión** instala una release más antigua por el mismo camino que una actualización: el `latest.json` de esa release, con la misma comprobación de firma. Tus sitios, ajustes y datos se quedan donde están.

Una versión anterior no conoce lo que añadieron las nuevas: ignora los ajustes que no conoce y puede descartarlos al guardar, y la cola de tareas y las notificaciones escritas en un formato más nuevo se apartan en una copia (`.broken`) en lugar de borrarse. Si dudas, detén el daemon y haz una copia de la carpeta de configuración antes.

## Actualizar a mano

Para una compilación local, un `.deb`/`.rpm` instalado antes de ese soporte, o cuando lo prefieras:

```bash
locdev version
locdev stop
```

Con el daemon detenido, copia `config.toml`, `projects.toml` y los datos que quieras conservar (rutas en [Configuración](/docs/configuration)). Los datos de la CA local contienen material privado: no los publiques en issues ni chats públicos. Descarga el paquete de la release, comprueba su SHA-256 en `SHA256SUMS` e instálalo con el mismo instalador que usaste la primera vez. Para la CLI independiente, reemplaza `locdev` y `locdevd` juntos. Para una compilación local, sigue [Instalación](/docs/installation).

Si cambia la ruta del ejecutable, revisa el [inicio automático](/docs/service). Una actualización no exige reinstalar el DNS ni volver a confiar en la CA.

## Después de actualizar

```bash
locdev start
locdev version
locdev doctor
locdev list
```

El inicio puede reabrir los proyectos con `auto_start`. Comprueba las versiones del cliente y del daemon y prueba un proyecto conocido. Consulta [Solución de problemas](/docs/troubleshooting) si algo falla.

## Limitaciones

- No validado con una release real: aún no existe ninguna. Se probaron la lectura de las releases (contra una API falsa) y la pantalla (con el backend simulado); la descarga y el reemplazo de la app con el actualizador de Tauri dependen de la primera release firmada.
- La actualización de `.deb` y `.rpm` desde la app aún no se ejecutó con una release real: la primera es la actualización siguiente a la versión que la trajo.
- Volver atrás solo alcanza releases que ya traían el `latest.json` firmado.
