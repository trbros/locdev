# Iniciar con el sistema

Por defecto, el daemon arranca cuando abres la aplicación o ejecutas `locdev start`. Para que arranque solo al iniciar sesión, usa:

```bash
locdev startup install
locdev startup status
locdev startup uninstall
```

En la aplicación, esto corresponde al interruptor **Iniciar LocDev al iniciar sesión** en Settings → daemon. La configuración es **por usuario** y no pide contraseña:

| SO      | Mecanismo                | Dónde                                                       |
| ------- | ------------------------ | ----------------------------------------------------------- |
| Linux   | unit de `systemd --user` | `~/.config/systemd/user/locdevd.service`                    |
| macOS   | LaunchAgent              | `~/Library/LaunchAgents/dev.locdev.locdevd.plist`           |
| Windows | clave Run                | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\LocDev` |

El daemon se niega a ejecutarse dos veces, así que el inicio de sesión y la aplicación nunca entran en conflicto. El servicio reinicia el daemon si cae, pero no después de un `locdev stop`.

## Logs

Cuando no está en una terminal, el daemon escribe el log en `<datos>/logs/locdevd.log`, rotando a `locdevd.log.1` al pasar de 10 MiB. Ajusta el nivel con `LOCDEV_LOG` (por ejemplo, `LOCDEV_LOG=debug`).
