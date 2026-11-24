# Installation

## Installers

Every version ships installers for the three platforms and a `SHA256SUMS` file to check the downloads. The site's [Download](/download) page opens on your system, with the latest version's files; the [Changelog](/changelog) shows what each version has. To build it yourself, see the source below.

| Platform | Files                                       | Notes                                                           |
| -------- | ------------------------------------------- | --------------------------------------------------------------- |
| Linux    | `.deb`, `.rpm`, `.AppImage` (x86_64, ARM64) | `.deb` and `.rpm` install `locdev` and `locdevd` in `/usr/bin`. |
| macOS    | `.dmg` (Apple Silicon and Intel)            | Requires macOS 11 or newer.                                     |
| Windows  | `.msi` and NSIS installer                   | NSIS installs per user, without administrator rights.           |

The app includes the daemon and the CLI: they're installed next to the executable, and the app starts the daemon by itself the first time it opens.

> [!WARNING]
> Until the binaries are signed, macOS (Gatekeeper) and Windows (SmartScreen) show a warning on first run. Check the hash against `SHA256SUMS` before going on.

```bash
# check a download
sha256sum -c SHA256SUMS --ignore-missing
```

### CLI only

To use it from the terminal only, each version ships `locdev-<version>-<target>.tar.gz` (Linux x86_64 and ARM64, macOS Apple Silicon and Intel) and `.zip` (Windows) with `locdev` and `locdevd`, without the app. The install scripts download your platform's archive, check its SHA-256 against the version's `SHA256SUMS` and copy both programs to a folder of yours, without administrator rights:

```bash
# Linux and macOS: installs into ~/.local/bin (LOCDEV_INSTALL_DIR changes it)
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
# Windows: installs into %LOCALAPPDATA%\LocDev\cli and adds it to the user Path
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.8.0` pins a version. Without the scripts, unpack the archive and put `locdev` and `locdevd` together in a folder on your `PATH`.

### The CLI inside the app

The app ships the same CLI. The `.deb` and `.rpm` already put it in `/usr/bin`; with the other formats, **Settings → About → command line** shows the bundled CLI's version and an **Add locdev to the terminal** button, which creates a small per-user script (`~/.local/bin/locdev`, or `locdev.cmd` in `%LOCALAPPDATA%\LocDev\bin` on Windows, with that folder on the `Path`). The same button removes it; a `locdev` that isn't LocDev's is never replaced.

## Optional dependencies

- **Linux, Chrome and Firefox:** for these browsers to trust the local CA, install `certutil` (package `libnss3-tools` on Debian/Ubuntu, `nss-tools` on Fedora).
- **Runtimes:** LocDev uses the PHP, Node and package managers already on the machine, and also installs verified versions per site ([Runtimes](/docs/runtimes)). `doctor` shows what it found.

## From source

You'll need stable Rust, Node.js 20+ and pnpm. For the app on Linux, the WebKitGTK libraries too.

```bash
# Linux: desktop app dependencies
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

cargo build --release              # locdev + locdevd in target/release
cd apps/desktop && pnpm install
pnpm tauri build                   # installers in target/release/bundle
```

During development, `pnpm tauri dev` builds the sidecars in debug mode and opens the app.
