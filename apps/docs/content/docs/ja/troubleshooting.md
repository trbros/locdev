# トラブルシューティング

実際に確認した問題から次の操作を選びます。各診断項目は [Doctor のリファレンス](/docs/doctor)を参照してください。コマンドの `acme` は例です。登録済みのプロジェクト名に置き換えてください。サイトがコマンドを実行することはありません。

## 証拠を集める

```bash
locdev version
locdev status
locdev doctor
locdev list
```

バージョン、OS、アーキテクチャ、最初に失敗した項目を記録します。環境全体を再インストールする前に、問題がある層を特定してください。

## デーモンに接続できない

```bash
locdev start
locdev status
```

改善しない場合は、`locdev` と `locdevd` が同じインストールに属するか確認します。[設定](/docs/configuration)に記載されたデータディレクトリのデーモンログを確認してください。バージョン不一致は[更新](/docs/updating)を参照してください。デーモンを起動すると、`auto_start` が有効なプロジェクトも起動する場合があります。

## プロジェクトが起動しない

```bash
locdev logs acme
locdev domain acme
```

登録先、実行コマンド、ランタイム、依存関係を確認します。依存関係はプロジェクトのパッケージマネージャーとロックファイルに従い、サイト外でインストールしてください。原因を修正した後、そのプロジェクトだけを再起動します。

```bash
locdev reload acme
```

## ポートとプロキシ

Doctor はプロジェクトのサーバーポートとプロキシのポートを区別します。80/443 が使用中の場合、既定の設定では 7080/7443 にフォールバックできます。アプリが示す実際のポートを使ってください。ポート指定のない URL は別のサーバーに到達する場合があります。

変更前に[設定](/docs/configuration)を確認してください。所有者が分かるサーバーだけを停止し、不明なプロセスを終了したり権限を自動変更したりしないでください。

## .test ドメインが解決しない

```bash
locdev dns status
```

別のツールがリゾルバーを管理している場合、先に競合の原因を調べます。`.test` を LocDev に向ける場合に限り、次の操作で提案される変更を確認してください。

```bash
locdev dns install
```

`.localhost` と OS ごとの詳細は[ドメイン](/docs/domains)を参照してください。

## HTTPS 証明書の警告

```bash
locdev tls status
```

ローカル CA とブラウザーの証明書ストアを確認します。正しい CA が信頼されていない場合は、[HTTPS](/docs/https)と次のコマンドが提案する変更を確認してください。

```bash
locdev tls trust
```

変更後はブラウザーを開き直します。想定と違う証明書の警告を無視しないでください。

## 開発サーバーがホストを拒否する

LocDev が起動するプロセスでは、Vite は変数 `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS` でサイトの名前をすでに受け取っています ([開発サーバー](/docs/servers)を参照)。LocDev の外で Vite を動かす場合は、自分が管理する `acme.test` などの特定ドメインを `server.allowedHosts` に追加します。すべてを許可する `true` は使わないでください。[Vite の公式設定](https://vite.dev/config/server-options.html#server-allowedhosts)を確認し、導入済みのフレームワークに合わせてください。

## サポートが必要な場合

[サポート](/support)へ再現手順と機密情報を除いたログを共有してください。パスワード、トークン、`.env`、個人情報は含めないでください。脆弱性の可能性は公開 issue ではなく[非公開窓口](/security)へ報告してください。
