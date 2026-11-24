# ログイン時に起動

デフォルトでは、デーモンはアプリを開いたときか `locdev start` を実行したときに起動します。ログイン時に自動で起動させるには、次を使います。

```bash
locdev startup install
locdev startup status
locdev startup uninstall
```

アプリでは、Settings → daemon の **ログイン時に LocDev を起動** スイッチにあたります。設定は**ユーザーごと**で、パスワードは不要です。

| OS      | 仕組み                      | 場所                                                        |
| ------- | --------------------------- | ----------------------------------------------------------- |
| Linux   | `systemd --user` のユニット | `~/.config/systemd/user/locdevd.service`                    |
| macOS   | LaunchAgent                 | `~/Library/LaunchAgents/dev.locdev.locdevd.plist`           |
| Windows | Run キー                    | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\LocDev` |

デーモンは二重に起動しないので、ログイン項目とアプリが衝突することはありません。サービスはデーモンが落ちると再起動しますが、`locdev stop` のあとは再起動しません。

## ログ

ターミナルで動いていないとき、デーモンはログを `<data>/logs/locdevd.log` に書き、10 MiB を超えると `locdevd.log.1` にローテーションします。レベルは `LOCDEV_LOG` で設定します (例: `LOCDEV_LOG=debug`)。
