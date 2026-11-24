# Editor de código

LocDev abre un sitio, o uno de sus archivos en una línea, en tu editor de código: desde el menú de un sitio (**Abrir en VS Code**, con el nombre del editor elegido), con `Ctrl` `E` en la página Sitios, desde los botones de las pestañas **Config** y **Email Studio**, o con `locdev editor open`.

![Configuración → Integraciones: los editores encontrados y el elegido.](screenshot:editor-settings)

## Editores

LocDev busca el comando de línea de comandos de cada editor en el `PATH`, en las carpetas que usan los instaladores (`/usr/local/bin`, Homebrew, `~/.local/bin`, `/snap/bin`, los scripts de JetBrains Toolbox) y en sus ubicaciones de instalación habituales.

| Editor                                                | Comando                                                | Abrir en una línea                  |
| ----------------------------------------------------- | ------------------------------------------------------ | ----------------------------------- |
| Visual Studio Code                                    | `code`                                                 | `code --goto archivo:línea:columna` |
| Cursor                                                | `cursor`                                               | como VS Code                        |
| Windsurf                                              | `windsurf`                                             | como VS Code                        |
| VSCodium                                              | `codium`                                               | como VS Code                        |
| Zed                                                   | `zed` (`zeditor` en algunas distribuciones Linux)      | `zed archivo:línea:columna`         |
| Sublime Text                                          | `subl`                                                 | `subl archivo:línea:columna`        |
| PhpStorm, WebStorm, IntelliJ IDEA, PyCharm, RustRover | `phpstorm`, `webstorm`, `idea`, `pycharm`, `rustrover` | `--line N --column M archivo`       |

Fuentes: [línea de comandos de VS Code](https://code.visualstudio.com/docs/configure/command-line), [CLI de Zed](https://zed.dev/docs/reference/cli), [línea de comandos de Sublime Text](https://www.sublimetext.com/docs/command_line.html), [abrir archivos desde la línea de comandos en los IDE de JetBrains](https://www.jetbrains.com/help/idea/opening-files-from-command-line.html).

¿No aparece el editor? Comprueba que su comando funciona en una terminal. En Linux y Windows, los instaladores de VS Code y sus derivados ya lo ponen en el `PATH`; en macOS, usa la paleta de comandos del editor → **Shell Command: Install 'code' command in PATH**. En los IDE de JetBrains, los scripts vienen de JetBrains Toolbox (una opción de su configuración genera scripts de shell). Después, **Buscar de nuevo** (o `locdev editor list --refresh`).

## Elegir el editor

En **Configuración → Integraciones → Editor de código**, haz clic en el editor. Sin elección, se abre el primero encontrado (en el orden de la tabla). La elección es `[editor] preferred` en `settings.toml`; si desinstalas el editor elegido, LocDev usa el primero encontrado y lo avisa.

```bash
locdev editor list
locdev editor set zed
locdev editor open shop
locdev editor open shop app/Http/Kernel.php:42
```

`locdev editor set none` vuelve al primero encontrado, y `--editor cursor` abre en otro editor solo esa vez.

## Seguridad

- Solo la carpeta del sitio o un archivo dentro de ella: ruta relativa, sin `..` y sin enlaces que salgan de la carpeta (el daemon comprueba la ruta real).
- El editor se ejecuta sin shell, con los argumentos que él documenta, por su cuenta: LocDev no espera a que se cierre.
- Solo los editores de la tabla: no hay comando personalizado.

## Limitaciones

- El daemon abre el editor en su propio entorno. Si el daemon se inició al iniciar sesión (sin la sesión gráfica en su entorno), puede que un editor gráfico no se abra; reinicia el daemon desde la app.
- Los editores de terminal (Vim, Neovim, Helix, Emacs en la terminal) no están en la lista, porque necesitan una terminal.
- Probado en Linux con un editor simulado y detectando VS Code instalado como snap; Windows y macOS no se probaron de verdad.
