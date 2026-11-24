# 設定

## `config.toml`

初回起動時にデフォルト値で作成されます。編集したら `locdev restart` を実行してください。

```toml
version = 1

[ports]
start = 8100        # サイトのポートに使う範囲
end = 8999

[proxy]
enabled = true
http_port = 80      # 空いていて使用が許可されていれば使う
fallback_port = 7080

[dns]
enabled = true
port = 10053        # Windows では 53 (NRPT の要件)
addresses = ["127.0.0.1"]   # Windows では ["127.0.0.1", "127.53.0.1"] のうち最初に空いているもの

[tls]
enabled = true
https_port = 443
fallback_port = 7443

[logs]
lines = 2000        # プロセスごとにメモリに保持する行数
file_mb = 5         # logs/sites/<site>.log がローテーションするサイズ

[mail]
enabled = true      # Mailbox のキャプチャ用 SMTP
port = 1025         # 使用中なら (Mailpit も同じポート) port + 9 まで試す

[env]
key_store = "keyring"   # または "file": このファイルの隣に env-vault.key (0600)

[deploy]
token_store = "keyring"      # または "file": プラットフォームのトークンを deploy-tokens.toml (0600) に保存

[updates]
repo = "trbros/locdev"       # `locdev update` が読むリリース(トークンなし)

[runtimes]
# php_flavor = "bulk"      # Linux/macOS の PHP 拡張セット: bulk または common
# php_static_url = "…"     # dl.static-php.dev/v3/php-bin のミラー
# php_windows_url = "…"    # downloads.php.net/~windows/releases のミラー

[definitions]
# url = "…"                # フォーク用: フレームワーク定義の別の配布元
# public_key = "…"
```

不正な値 (たとえば 1024 未満の `ports.start`) があるとデーモンは起動せず、ファイルとフィールドを示すメッセージが表示されます。

> [!NOTE]
> Linux では、`net.ipv4.ip_unprivileged_port_start` (通常は 1024) 未満のポートには権限が必要です。ポート 80 や 443 が使えない場合、LocDev は代替ポートに切り替え、`doctor` が `locdevd` への `setcap cap_net_bind_service` を提案します。

## `settings.toml`

アプリと CLI が実行中に (再起動せずに) 変える設定で、デーモンが書き込みます。**Settings → Automations**、**Notifications → Preferences**、`locdev settings set` を使うのがおすすめです。

```toml
[automation]
audit = "daily"            # off、manual、6h、12h、daily、weekly
concurrency = 2            # 同時に実行するジョブ数
requests_per_second = 8    # すべてのジョブを合わせたレジストリへのリクエスト数
definitions = true         # フレームワーク定義の毎日のチェック
updates = true             # LocDev の新しいリリースを毎日通知(何もインストールしない)

[observe]
enabled = true             # Debug Center: サイトのイベントを収集
retention_days = 7         # data/observe に保存する日数
max_mb = 100               # 最大容量。古い日から削除
logs_from = "warn"         # 通常のログ行のレベル: info、warn、error

[notifications]
native = true              # OS の通知でも表示する

[notifications.categories]  # dependency_update、security、process_failure、runtime、
dependency_update = true    # certificate、job、workflow、action、mail
security = true

[network]
offline = false            # バックグラウンドでネットワークを使わない

[integrations]
github_client_id = ""      # device flow 用の OAuth App (任意)
github_watch_runs = true   # ワークフローの失敗を通知する

[env]
conflicts = "active_files" # または "any_file": どれかの .env* にキーがあればスキップ
```

## `projects.toml`

サイトの登録内容です。デーモンを止めた状態なら手で編集できますが、ふつうは `locdev add/edit/remove` かアプリを使います。

```toml
version = 1

[[project]]
name = "blog"
domain = "blog.test"
path = "/home/you/Code/blog"
kind = "next"
command = "pnpm run dev --hostname 127.0.0.1 --port {port}"
port = 8101
auto_start = true
```

## `parked.toml`

[監視フォルダー](/docs/parked-folders)と、それぞれのルールです。通常は `locdev park` かアプリで変更します。

```toml
version = 1

[[dir]]
path = "/home/you/Development"
enabled = true
depth = 1
exclude = ["archive-*"]
ignored = ["scratch"]
on_new = "register"        # または "suggest"
on_missing = "unregister"  # または "keep"
```

監視フォルダーから追加されたサイトは、`projects.toml` に元のフォルダーを保持します (`parked = "/home/you/Development"`)。

`data/state/` には、ジョブのキュー (`jobs.json`)、通知 (`notifications.json`)、レジストリのキャッシュ (`registry-cache.json`)、各サイトの最新のチェック結果 (`deps/<site>.json`) もあります。書き込みはアトミックで、壊れたファイルは削除せずに隣に保存します (`.broken`)。

`version` (または `format`) フィールドを持つファイルも同じルールに従います。新しい LocDev は形式を移行し、古いバージョンは新しいバージョンが書いたファイルを壊さないよう、読むのを拒否します。

## ファイルの場所

|                   | Linux                                  | macOS                                             | Windows                             |
| ----------------- | -------------------------------------- | ------------------------------------------------- | ----------------------------------- |
| 設定              | `~/.config/locdev`                     | `~/Library/Application Support/dev.locdev.locdev` | `%APPDATA%\locdev\locdev\config`    |
| データ (CA、ログ) | `~/.local/share/locdev`                | `~/Library/Application Support/dev.locdev.locdev` | `%LOCALAPPDATA%\locdev\locdev\data` |
| IPC ソケット      | `$XDG_RUNTIME_DIR/locdev/locdevd.sock` | `…/run/locdevd.sock`                              | `\\.\pipe\locdev-<user>-<hash>`     |

`LOCDEV_HOME=/path` を指定すると、すべてがそのパス内の `config/`、`data/`、`logs/`、`run/` に置かれます。
