# チュートリアル: Laravel サイトをゼロから

このチュートリアルでは、選んだ PHP のバージョンで新しい Laravel プロジェクトを作り、最後には `https://blog.test` でサイトが開き、SQLite のデータベースが使え、プロジェクトのメールが Mailbox に届く状態にします。PHP や Composer をインストールしておく必要はありません。LocDev が公式の配布元から両方をダウンロードします。

所要時間: 約 5 分（ほとんどがダウンロード）。

## 1. PHP をインストールする

アプリでは **ランタイム → PHP → バージョンをインストール…** から系列（たとえば 8.4）を選びます。ターミナルでは:

```bash
locdev runtime install php 8.4
locdev runtime install composer latest
```

LocDev は StaticPHP（Linux/macOS）または windows.php.net（Windows）からビルド済みの PHP をダウンロードし、公開されている SHA-256 と一致したときだけインストールします。[ランタイム](/docs/runtimes) を参照してください。

![PHP のカタログ: 各系列の最新版。](screenshot:runtimes-install)

## 2. プロジェクトを作る

アプリでは **ツール → 新しいプロジェクト → Laravel**。名前、スターターキット（なし、React、Vue、Svelte、Livewire）、データベースを選びます。実行前に、プランですべてのコマンドを確認できます。

![新しいプロジェクト: フォームと、実際のコマンドを示すプラン。](screenshot:new-project)

ターミナルでは同じことを 1 行で:

```bash
locdev new project laravel blog --method composer --php 8.4 --in ~/Code -y
```

`--php 8.4` を付けると、その PHP でプロジェクトを作り、サイトもそのバージョンに固定します。手順はアプリ内のターミナルで実行され、最後に LocDev がサイトを登録します。

![アプリのターミナルで実行される手順。](screenshot:new-project-run)

## 3. サイトを起動する

```bash
locdev up blog
locdev open blog
```

Laravel はサイトのポートで `php artisan serve` として動き、プロキシがすぐに `http://blog.localhost` で配信します。Laravel のデフォルトのデータベースは SQLite で、プロジェクト作成時にマイグレーション済みです。

## 4. `.test` と HTTPS（最初の 1 回だけ）

```bash
locdev dns install
locdev tls trust
```

どちらのコマンドも、実行するスクリプトを表示してから確認を求めます。その後は `https://blog.test` が警告なしでブラウザーに開きます。詳しくは [プロキシと DNS](/docs/domains) と [HTTPS](/docs/https) を参照してください。

## 5. メールを Mailbox で受け取る

Mailbox はサイトが送るメールをキャプチャーし、どこにも配送しません。

```bash
locdev mail setup blog     # 変わる内容: MAIL_MAILER=smtp、LocDev のホストとポート
locdev mail enable blog
locdev reload blog
```

変数はサイトのプロセスの環境にだけ渡され、プロジェクトの `.env` は変更されません。キューワーカーとスケジューラーにも同じ変数が渡ります。Laravel からメールを送り、**ツール → Mailbox**（または `locdev mail list`）を開いてください。

![Mailbox: サイトが送ったメッセージ。HTML、テキスト、問題点を表示。](screenshot:mailbox)

## 6. 拡張機能を確認する

```bash
locdev runtime extensions blog
```

LocDev は、プロジェクトが必要とするもの（`composer.json`、`.env` のデータベース、Laravel の要件）と、サイトの PHP が読み込んでいるものを比べ、足りないものの解決方法を示します。

## 次のステップ

- [Debug Center](/docs/debug-center): 例外、`dump()`、Laravel のログのエラーを 1 つのタイムラインに。
- [サービス](/docs/services): SQLite の代わりにローカルの PostgreSQL や Redis を使う。
- [開発サーバー](/docs/servers): アセット用に Laravel の隣で Vite を動かす。
