# トラブルシューティング

まずは必ずこれを実行します。

```bash
locdev doctor
```

![Doctor](screenshot:doctor)

## チェックされる内容

| チェック                      | 意味                                                                                                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Daemon                        | CLI が `locdevd` と通信できるか、プロトコルのバージョンが一致するか。                                                                                                            |
| Configuration folder          | 設定フォルダーに書き込めるか。                                                                                                                                                   |
| Project folders               | 登録したフォルダーがまだ存在するか。                                                                                                                                             |
| HTTP proxy / HTTPS proxy      | プロキシが待ち受けているか、代替ポートに切り替える必要があったか (とその理由)。                                                                                                  |
| HTTP port 80 / HTTPS port 443 | ポートが空いているか、ほかのプログラムまたは LocDev 自身が使っているか。                                                                                                         |
| Local CA                      | システムがローカル CA を信頼しているか。                                                                                                                                         |
| Browser certificate stores    | Chrome/Firefox (Linux) が CA を信頼しているか。                                                                                                                                  |
| DNS server                    | `*.test` のサーバーが動いているか。                                                                                                                                              |
| .test resolver                | `*.test` が実際にどこへ行くか: LocDev、ほかのツール、どこにも行かない。                                                                                                          |
| PHP, Composer, Node.js, npm   | ツールが `PATH` にあるか (登録済みのサイトがそれを必要とするか)。                                                                                                                |
| 各サイトのランタイム          | インストールされていない固定バージョン、プロジェクトが宣言する範囲外のバージョン、公開元のサポートが終わったバージョン、より新しいランタイムを必要とするパッケージマネージャー。 |
| 各ランタイムのデフォルト      | `runtimes.toml` で設定したデフォルトがまだインストールされているか。                                                                                                             |
| Secrets vault                 | シークレット変数の保管庫の鍵にアクセスできるか (キーリングまたはファイル)。                                                                                                      |
| Framework definitions         | 使用中の定義のリビジョン、出どころ (組み込み、または署名を検証してダウンロード)、最終チェック。                                                                                  |

## よくある問題

### `the daemon is not running`

```bash
locdev start
```

起動しない場合は、データフォルダーにあるデーモンのログ `logs/locdevd.log` を確認してください (Linux では `~/.local/share/locdev/logs/locdevd.log`。フォルダーの一覧は[設定](/docs/configuration)にあります)。

### ポート 80/443 が使用中

ほかのツール (Apache、nginx、Valet、Herd、Yerd など) がポートを使っています。LocDev は 7080/7443 に切り替え、ステータスバーにそれを表示します。ポートを空けるか、`proxy.http_port` / `tls.https_port` を変更してください。

### `*.test → another tool`

ほかのツールが `.test` のリゾルバーを設定しています。`doctor` にそのファイルが表示されます。そのツールを使っていなければファイルを削除し、`locdev dns install` を実行してください。

### ブラウザーがまだ証明書の警告を出す

1. `locdev tls status` に `system trusted` と `browsers trusted` が表示されるか確認します。
2. ブラウザーを完全に再起動します。
3. Linux では `libnss3-tools` をインストールし、もう一度 `locdev tls trust` を実行します。

### Vite のプロジェクトが "Blocked request" を返す

LocDev が起動するプロセスでは、サイトの名前はすでに許可されています ([開発サーバー](/docs/servers)を参照)。LocDev の外で Vite を動かす場合は、`vite.config` に `server: { allowedHosts: [".test"] }` を追加するか、`.localhost` の URL を使ってください。

### サイトが起動に失敗する

アプリでサイトを開くか、`locdev logs <name>` を実行します。プロセスの出力に、PHP の拡張が足りない、依存関係がインストールされていない、などのエラーが表示されます。
