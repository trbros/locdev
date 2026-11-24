# Instalação

## Instaladores

Cada versão traz instaladores para as três plataformas e um arquivo `SHA256SUMS` para conferir os downloads. A página [Download](/download) do site já abre no seu sistema, com os arquivos da versão mais recente; o [Changelog](/changelog) mostra o que existe em cada versão. Para compilar você mesmo, veja o código-fonte abaixo.

| Plataforma | Arquivos                                    | Observações                                                      |
| ---------- | ------------------------------------------- | ---------------------------------------------------------------- |
| Linux      | `.deb`, `.rpm`, `.AppImage` (x86_64, ARM64) | O `.deb` e o `.rpm` instalam `locdev` e `locdevd` em `/usr/bin`. |
| macOS      | `.dmg` (Apple Silicon e Intel)              | Requer macOS 11 ou mais recente.                                 |
| Windows    | `.msi` e instalador NSIS                    | O NSIS instala por usuário, sem precisar de administrador.       |

O app inclui o daemon e a CLI: eles são instalados ao lado do executável, e o app inicia o daemon sozinho na primeira abertura.

> [!WARNING]
> Enquanto os binários não forem assinados, o macOS (Gatekeeper) e o Windows (SmartScreen) mostram um aviso na primeira execução. Confira o hash com `SHA256SUMS` antes de prosseguir.

```bash
# conferir um download
sha256sum -c SHA256SUMS --ignore-missing
```

### CLI avulsa

Para usar só pelo terminal, cada versão traz `locdev-<versão>-<alvo>.tar.gz` (Linux x86_64 e ARM64, macOS Apple Silicon e Intel) e `.zip` (Windows), com `locdev` e `locdevd`, sem o app. Os scripts de instalação baixam o arquivo da sua plataforma, conferem o SHA-256 contra o `SHA256SUMS` da versão e copiam os dois programas para uma pasta sua, sem administrador:

```bash
# Linux e macOS: instala em ~/.local/bin (LOCDEV_INSTALL_DIR muda a pasta)
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
# Windows: instala em %LOCALAPPDATA%\LocDev\cli e põe a pasta no Path do usuário
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.8.0` fixa uma versão. Sem os scripts, descompacte o arquivo e coloque `locdev` e `locdevd` juntos em uma pasta do seu `PATH`.

### A CLI que vem com o app

O app traz a mesma CLI. O `.deb` e o `.rpm` já a põem em `/usr/bin`; nos demais formatos, **Configurações → Sobre → linha de comando** mostra a versão da CLI incluída e o botão **Adicionar o locdev ao terminal**, que cria um pequeno script por usuário (`~/.local/bin/locdev`, ou `locdev.cmd` em `%LOCALAPPDATA%\LocDev\bin` no Windows, com a pasta no `Path`). O mesmo botão remove o script; um `locdev` que não seja do LocDev nunca é substituído.

## Dependências opcionais

- **Linux, Chrome e Firefox:** para que esses navegadores confiem na CA local, instale o `certutil` (pacote `libnss3-tools` no Debian/Ubuntu, `nss-tools` no Fedora).
- **Runtimes:** o LocDev usa o PHP, o Node e os gerenciadores de pacotes que já estão na máquina e também instala versões verificadas por site ([Runtimes](/docs/runtimes)). O `doctor` mostra o que encontrou.

## A partir do código-fonte

Você vai precisar de Rust estável, Node.js 20+ e pnpm. Para o app no Linux, também das bibliotecas do WebKitGTK.

```bash
# Linux: dependências do app desktop
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

cargo build --release              # locdev + locdevd em target/release
cd apps/desktop && pnpm install
pnpm tauri build                   # gera os instaladores em target/release/bundle
```

Durante o desenvolvimento, `pnpm tauri dev` compila os sidecars em modo debug e abre o app.
