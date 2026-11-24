# サービス: データベースとキャッシュ

LocDev は Docker を使わずに、プロジェクト用の PostgreSQL、MariaDB、MySQL、MongoDB、Valkey、Redis を動かします。各**インスタンス**は専用のデータフォルダー、ポート、設定、ログを持ち、デーモンに監視されて動き (失敗すると再起動)、**`127.0.0.1` だけ**で待ち受けます。

```bash
locdev service list                              # サービス、インストール済みバージョン、インスタンス
locdev service install postgresql                # 最新バージョンをダウンロードして検証
locdev service create db --service postgres --start
locdev service status
locdev service backup db
locdev service stop postgresql                   # インスタンスが 1 つならサービス名でも指定できる
```

## バイナリの出どころ

LocDev はマシンにすでにあるインストール (`PATH`、`/usr/lib/postgresql/*`、Homebrew、`Program Files`) を**変更せずに**使い、独自のバージョンをダウンロードすることもできます。ダウンロードしたものは、SHA-256 が配布元の公開値と一致したときだけインストールされます。

| サービス   | LocDev によるダウンロード               | 配布元                                                                                                                      |
| ---------- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| PostgreSQL | Linux、macOS、Windows                   | [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) がビルドした公式ソース (来歴の証明付き) |
| MariaDB    | Linux、Windows                          | MariaDB Foundation (ダウンロード API、ファイルごとの SHA-256)                                                               |
| MongoDB    | Ubuntu 22.04+/Debian 12、macOS、Windows | MongoDB, Inc. (community server、各ファイルの隣に `.sha256`)                                                                |
| Valkey     | Linux                                   | download.valkey.io (Ubuntu 向けビルド)                                                                                      |
| MySQL      | なし                                    | Oracle は MD5 しか公開しておらず、LocDev はそれを受け付けません。システムの MySQL か MariaDB を使ってください               |
| Redis      | なし                                    | 公式バイナリがありません。システムの Redis か Valkey (互換) を使ってください                                                |

ダウンロードしたバージョンは `~/.local/share/locdev/services/bin/` に置かれ、`locdev service uninstall` で削除できます (システムのものや、インスタンスが使っているバージョンは削除できません)。

## インスタンス

- **データ**は `services/instances/<name>/data` にあり、インスタンス間で混ざることはありません。フォルダーは `0700` の権限で作られます。
- **ポート**: 空いていればサービスのデフォルト (5432、3306、27017、6379)、そうでなければ次の空きポート。2 つのインスタンスが同じポートを取り合うことはなく、Redis と Valkey (どちらもデフォルトは 6379) には別々のポートが割り当てられます。
- **アクセス**: `127.0.0.1` のみ、パスワードなしのローカルユーザー (PostgreSQL は `locdev`、MariaDB/MySQL は `root`)。開発環境なので、これらのポートを外部に公開しないでください。
- **接続**: `locdev service create` と **サービス** ページに、プロジェクトにコピーするための URL とよく使う変数 (`DATABASE_URL`、`DB_HOST`、`DB_PORT`、`REDIS_URL` など) が表示されます。
- **LocDev と一緒に起動**: `--autostart` または `locdev service autostart db true`。インスタンスはサイトより先に起動します。
- **準備完了**は実際に接続を受け付けることを意味します。PID があるだけでは不十分です。

## バックアップと削除

`locdev service backup` は**コールドコピー**を作ります。インスタンスを止め、データフォルダーを圧縮し、再び起動します。`locdev service restore <instance> <id>` はコピーを戻し、**現在のデータを `data.before-<time>` に移します** (削除はしません)。

インスタンスを削除しても**データは残ります**。データを消すには `--delete-data` とインスタンス名の入力が必要で、バックアップは残ります。

## SQLite

SQLite はプロジェクト内のファイル (たとえば `database/database.sqlite`) なので、インストールも起動も停止も不要です。バックアップするには、アプリケーションが書き込んでいない間にファイルをコピーしてください。

## PHP の拡張

`locdev runtime extensions <site>` は、プロジェクトが必要とするものとサイトの PHP が読み込むものを比べます。

- `composer.json` と `composer.lock` のパッケージにある `ext-*`。
- フレームワークの要件 (Laravel: ctype、curl、dom、fileinfo、filter、hash、mbstring、openssl、pcre、pdo、session、tokenizer、xml)。
- `.env` が使うドライバー (`DB_CONNECTION=pgsql` → `pdo_pgsql`、phpredis クライアントの Redis → `redis`)。

自動では何もインストールしません。その PHP での解決方法を表示します: apt のパッケージ (`php8.3-pgsql`)、Homebrew の `pecl`、Windows の `php.ini` の行。LocDev の PHP ビルドは静的で拡張のセットが固定なので、含まれていない拡張が必要なら、それを持つシステムの PHP を使ってください。足りない拡張があると、コマンドは終了コード 1 で終わります。

## 制限

- Linux 上で、システムの PostgreSQL 18.6 と Redis 8.10 で実際に検証しました。管理対象のインストール (ダウンロード、検証、展開) にはテストがありますが、端から端までの動作はまだ試していません。
- インスタンスを別のメジャーバージョンに上げる (PostgreSQL 16 → 17 など) のは自動ではありません。新しいインスタンスを作り、データをエクスポートしてインポートしてください。
- Windows と macOS はまだテストしていません。

## パッケージマネージャーからの他のバージョン

LocDev が検証済みビルドをダウンロードできない場合 (MySQL と Redis は SHA-256 を公開していません) や、より多くのバージョンが必要な場合、**サービス** はシステムのパッケージマネージャーにあるものを提示します。コマンドを先に表示し、アプリ内のターミナルで実行します。

| システム                             | 提示されるもの                                                                                                                                                   |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| macOS (および Homebrew のある Linux) | `mysql`、`mysql@8.4`、`mysql@8.0`、`redis`、`redis@8.2`、`postgresql@15` から `@17`、`mongodb-community@7.0` と `@8.0` (tap `mongodb/brew`)、`mariadb`、`valkey` |
| Debian と Ubuntu                     | ディストリビューションの `mysql-server`、`redis-server`、`postgresql`、`mariadb-server` (`sudo` はターミナルでパスワードを求めます)                              |
| Windows                              | winget の MySQL (最新版、8.4、8.0) (`Oracle.MySQL`)                                                                                                              |

パッケージマネージャーがインストールするものを検証します。結果は LocDev が検出するシステムのインストール (Homebrew の `opt/` 以下のバージョン付きフォーミュラも含む) になり、LocDev が変更や削除をすることはありません。Redis には Windows の公式ビルドがないため、WSL か Memurai を使ってください。

```bash
locdev service packages mysql
locdev service install-package mysql mysql@8.4
```
