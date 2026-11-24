# Start at login

By default, the daemon starts when you open the app or run `locdev start`. To have it start by itself at login, use:

```bash
locdev startup install
locdev startup status
locdev startup uninstall
```

In the app, this is the **Start LocDev at login** switch in Settings → daemon. The setting is **per user** and doesn't ask for a password:

| OS      | Mechanism               | Where                                                       |
| ------- | ----------------------- | ----------------------------------------------------------- |
| Linux   | a `systemd --user` unit | `~/.config/systemd/user/locdevd.service`                    |
| macOS   | LaunchAgent             | `~/Library/LaunchAgents/dev.locdev.locdevd.plist`           |
| Windows | Run key                 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\LocDev` |

The daemon refuses to run twice, so the login item and the app never conflict. The service restarts the daemon if it crashes, but not after a `locdev stop`.

## Logs

When it isn't in a terminal, the daemon writes its log to `<data>/logs/locdevd.log`, rotating to `locdevd.log.1` past 10 MiB. Set the level with `LOCDEV_LOG` (for example `LOCDEV_LOG=debug`).
