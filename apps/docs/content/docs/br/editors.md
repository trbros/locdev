# Editor de código

O LocDev abre um site, ou um arquivo dele numa linha, no seu editor de código: pelo menu de um site (**Abrir no VS Code**, com o nome do editor escolhido), por `Ctrl` `E` na página Sites, pelos botões das abas **Config** e **Email Studio**, ou com `locdev editor open`.

![Configurações → Integrações: os editores encontrados e o escolhido.](screenshot:editor-settings)

## Editores

O LocDev procura o comando de linha de comando de cada editor no `PATH`, nas pastas que os instaladores usam (`/usr/local/bin`, Homebrew, `~/.local/bin`, `/snap/bin`, os scripts do JetBrains Toolbox) e nos locais de instalação conhecidos.

| Editor                                                | Comando                                                | Abrir numa linha                   |
| ----------------------------------------------------- | ------------------------------------------------------ | ---------------------------------- |
| Visual Studio Code                                    | `code`                                                 | `code --goto arquivo:linha:coluna` |
| Cursor                                                | `cursor`                                               | como o VS Code                     |
| Windsurf                                              | `windsurf`                                             | como o VS Code                     |
| VSCodium                                              | `codium`                                               | como o VS Code                     |
| Zed                                                   | `zed` (em algumas distribuições Linux, `zeditor`)      | `zed arquivo:linha:coluna`         |
| Sublime Text                                          | `subl`                                                 | `subl arquivo:linha:coluna`        |
| PhpStorm, WebStorm, IntelliJ IDEA, PyCharm, RustRover | `phpstorm`, `webstorm`, `idea`, `pycharm`, `rustrover` | `--line N --column M arquivo`      |

Fontes: [linha de comando do VS Code](https://code.visualstudio.com/docs/configure/command-line), [CLI do Zed](https://zed.dev/docs/reference/cli), [linha de comando do Sublime Text](https://www.sublimetext.com/docs/command_line.html), [abrir arquivos pela linha de comando nas IDEs da JetBrains](https://www.jetbrains.com/help/idea/opening-files-from-command-line.html).

O editor não aparece? Confira se o comando dele existe num terminal. No Linux e no Windows, os instaladores do VS Code e dos derivados já o colocam no `PATH`; no macOS, use a paleta de comandos do editor → **Shell Command: Install 'code' command in PATH**. Nas IDEs da JetBrains, os scripts vêm do JetBrains Toolbox (opção de gerar scripts de shell nas configurações dele). Depois, **Procurar de novo** (ou `locdev editor list --refresh`).

## Escolher o editor

Em **Configurações → Integrações → Editor de código**, clique no editor. Sem escolha, abre o primeiro encontrado na ordem da tabela. A escolha fica em `[editor] preferred` no `settings.toml`; se o editor escolhido for desinstalado, o LocDev usa o primeiro encontrado e avisa.

```bash
locdev editor list
locdev editor set zed
locdev editor open shop
locdev editor open shop app/Http/Kernel.php:42
```

`locdev editor set none` volta ao primeiro encontrado, e `--editor cursor` abre num editor diferente só daquela vez.

## Segurança

- Só a pasta do site ou um arquivo dentro dela: caminho relativo, sem `..` e sem links que saiam da pasta (o daemon confere o caminho real).
- O editor roda sem shell, com os argumentos documentados por ele, e por conta própria: o LocDev não espera ele fechar.
- Só os editores da tabela: não existe comando personalizado.

## Limitações

- O daemon abre o editor no ambiente dele. Se o daemon foi iniciado pelo login (sem a sessão gráfica no ambiente), um editor gráfico pode não abrir; reinicie o daemon pelo app.
- Editores de terminal (Vim, Neovim, Helix, Emacs no terminal) não estão na lista, porque precisam de um terminal para rodar.
- Testado no Linux com um editor simulado e com a detecção do VS Code instalado por snap; Windows e macOS não foram testados de verdade.
