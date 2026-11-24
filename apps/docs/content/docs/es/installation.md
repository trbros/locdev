# Instalación

## Instaladores

Cada versión trae instaladores para las tres plataformas y un archivo `SHA256SUMS` para verificar las descargas. La página [Descargar](/download) del sitio se abre en tu sistema, con los archivos de la versión más reciente; el [Changelog](/changelog) muestra lo que hay en cada versión. Para compilarlo tú mismo, consulta el código fuente más abajo.

| Plataforma | Archivos                                    | Notas                                                        |
| ---------- | ------------------------------------------- | ------------------------------------------------------------ |
| Linux      | `.deb`, `.rpm`, `.AppImage` (x86_64, ARM64) | `.deb` y `.rpm` instalan `locdev` y `locdevd` en `/usr/bin`. |
| macOS      | `.dmg` (Apple Silicon e Intel)              | Requiere macOS 11 o posterior.                               |
| Windows    | `.msi` e instalador NSIS                    | NSIS instala por usuario, sin permisos de administrador.     |

La aplicación incluye el daemon y la CLI: se instalan junto al ejecutable, y la aplicación inicia el daemon sola la primera vez que se abre.

> [!WARNING]
> Mientras los binarios no estén firmados, macOS (Gatekeeper) y Windows (SmartScreen) muestran un aviso en la primera ejecución. Comprueba el hash con `SHA256SUMS` antes de continuar.

```bash
# verificar una descarga
sha256sum -c SHA256SUMS --ignore-missing
```

### Solo la CLI

Para usarla solo desde la terminal, cada versión trae `locdev-<versión>-<destino>.tar.gz` (Linux x86_64 y ARM64, macOS Apple Silicon e Intel) y `.zip` (Windows), con `locdev` y `locdevd`, sin la app. Los scripts de instalación descargan el archivo de tu plataforma, comprueban su SHA-256 con el `SHA256SUMS` de la versión y copian ambos programas a una carpeta tuya, sin administrador:

```bash
# Linux y macOS: instala en ~/.local/bin (LOCDEV_INSTALL_DIR la cambia)
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
# Windows: instala en %LOCALAPPDATA%\LocDev\cli y la agrega al Path del usuario
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.8.0` fija una versión. Sin los scripts, descomprime el archivo y coloca `locdev` y `locdevd` juntos en una carpeta de tu `PATH`.

### La CLI incluida en la app

La app trae la misma CLI. El `.deb` y el `.rpm` ya la ponen en `/usr/bin`; en los demás formatos, **Configuración → Acerca de → línea de comandos** muestra la versión de la CLI incluida y el botón **Añadir locdev a la terminal**, que crea un pequeño script por usuario (`~/.local/bin/locdev`, o `locdev.cmd` en `%LOCALAPPDATA%\LocDev\bin` en Windows, con la carpeta en el `Path`). El mismo botón lo quita; un `locdev` que no sea de LocDev nunca se reemplaza.

## Dependencias opcionales

- **Linux, Chrome y Firefox:** para que estos navegadores confíen en la CA local, instala `certutil` (paquete `libnss3-tools` en Debian/Ubuntu, `nss-tools` en Fedora).
- **Runtimes:** LocDev usa el PHP, el Node y los gestores de paquetes que ya hay en la máquina, y también instala versiones verificadas por sitio ([Runtimes](/docs/runtimes)). `doctor` muestra lo que encontró.

## Desde el código fuente

Necesitarás Rust estable, Node.js 20+ y pnpm. Para la aplicación en Linux, también las bibliotecas de WebKitGTK.

```bash
# Linux: dependencias de la aplicación de escritorio
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

cargo build --release              # locdev + locdevd en target/release
cd apps/desktop && pnpm install
pnpm tauri build                   # instaladores en target/release/bundle
```

Durante el desarrollo, `pnpm tauri dev` compila los sidecars en modo debug y abre la aplicación.
