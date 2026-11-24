# Iniciar com o sistema

Por padrão, o daemon sobe quando você abre o app ou roda `locdev start`. Para que ele suba sozinho no login, use:

```bash
locdev startup install
locdev startup status
locdev startup uninstall
```

No app, isso corresponde ao interruptor **Iniciar o LocDev no login** em Settings → daemon. A configuração é **por usuário** e não pede senha:

| SO      | Mecanismo                | Onde                                                        |
| ------- | ------------------------ | ----------------------------------------------------------- |
| Linux   | unit do `systemd --user` | `~/.config/systemd/user/locdevd.service`                    |
| macOS   | LaunchAgent              | `~/Library/LaunchAgents/dev.locdev.locdevd.plist`           |
| Windows | chave Run                | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\LocDev` |

O daemon se recusa a rodar duas vezes, então o login item e o app nunca entram em conflito. O serviço reinicia o daemon se ele cair, mas não depois de um `locdev stop`.

## Logs

Quando não está em um terminal, o daemon escreve o log em `<dados>/logs/locdevd.log`, girando para `locdevd.log.1` ao passar de 10 MiB. Ajuste o nível com `LOCDEV_LOG` (por exemplo, `LOCDEV_LOG=debug`).
