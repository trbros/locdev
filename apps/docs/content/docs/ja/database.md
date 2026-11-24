# Database Manager

LocDev は、サイトが使うデータベースと、LocDev 自身が動かすインスタンス ([サービス](/docs/services)を参照) を開けます。テーブルの一覧、構造の表示、行のページ送り、編集、挿入、削除、SQL の実行ができます。Redis と Valkey にはキーブラウザーがあります。アプリでは **Tools → Database**、CLI では `locdev db` です。

![Tools → Database: サイトの接続、テーブル、行。](screenshot:database)

| エンジン        | テーブルと構造      | 行 (絞り込み、並べ替え、編集)          | SQL  | データベースの作成    |
| --------------- | ------------------- | -------------------------------------- | ---- | --------------------- |
| PostgreSQL      | あり (スキーマ付き) | あり                                   | あり | あり                  |
| MySQL / MariaDB | あり                | あり                                   | あり | あり                  |
| SQLite          | あり                | あり                                   | あり | あり (ファイルを作成) |
| Redis / Valkey  | `SCAN` によるキー   | 読み取り、文字列の編集、有効期限、削除 | —    | —                     |

MongoDB のブラウザーはまだありません。インスタンスは **サービス** に表示されます。

## 接続の出どころ

LocDev にパスワードを入力することはありません。デーモンは、プロジェクトが `.env` と `.env.local` ですでに宣言している内容を**使うたびに**読みます。

- **Laravel:** `DB_CONNECTION`、`DB_HOST`、`DB_PORT`、`DB_DATABASE`、`DB_USERNAME`、`DB_PASSWORD` (`sqlite` の場合、`DB_DATABASE` がファイル。なければ `database/database.sqlite`)。
- **`DATABASE_URL`** (Prisma、Rails、Django、Drizzle など): `postgres://`、`mysql://`、`mariadb://`、`file:`、`sqlite:`。Prisma では、Prisma 自身と同じく `file:./dev.db` は `prisma/` フォルダーからの相対パスです。
- **Redis:** `REDIS_URL`、または `REDIS_HOST`/`REDIS_PORT`/`REDIS_USERNAME`/`REDIS_PASSWORD`。
- **既知の SQLite ファイル:** `database/database.sqlite`、`db.sqlite3`、`prisma/dev.db`、`storage/database.sqlite`、`db/development.sqlite3`。
- サイトが受け取る**共有変数** ([環境変数](/docs/environment)を参照) も、`.env` にあるものとして読まれます。
- **LocDev のインスタンス** (各サービスのローカルユーザーで接続)。

同じデータベースが 2 回宣言されていても (たとえば `DB_*` と `DATABASE_URL`)、表示は 1 回だけです。

```bash
locdev db list                       # すべてのサイトとインスタンスの接続
locdev db connect acme               # ホスト、ポート、データベース、ユーザー (パスワードは表示しない)
locdev db tables acme
locdev db describe users acme        # カラム、インデックス、外部キー
locdev db rows users acme --where role=admin --order-by id --desc
locdev db query "select count(*) from users" acme
locdev db create acme                # サイトの .env が指定するデータベースを作成
```

サイトに複数のデータベースがある場合、CLI は最初の SQL データベースを使います。`--conn <id>` (`locdev db list` で確認) で別のものを選べます。

## 守られること

- **認証情報はデーモンの外に出ません。** アプリと CLI が知っているのは識別子 (`project:<site>:<n>` または `instance:<name>`) とパスワードの有無だけです。パスワードはレスポンス、ログ、エラーメッセージに現れません。
- **このマシンの外にあるデータベース** (`127.0.0.1`/`localhost`/`::1` 以外の `DB_HOST`) には **リモート** バッジが付き、明示的に選んだときだけ開きます。アプリでは _それでも開く_ ボタン、CLI では `--allow-remote` です。本番環境かもしれず、そこでの編集はすべて実際に反映されます。
- **頼まれない限り何も作りません。** サーバーは応答するがデータベースがない場合、アプリに _作成_ ボタン付きのお知らせが出ます。名前はサイトの設定から取り、検証します (英字、数字、`_`、`-`)。
- **破壊的なコマンドはデータベース名の入力を求めます。** `DROP`、`TRUNCATE`、`ALTER … DROP`、`GRANT`/`REVOKE`、そして `WHERE` のない、または常に真になる `WHERE` (`1=1`、`true`、`id = id`、`… or 1=1`) を持つ `DELETE`/`UPDATE` です。デーモンは実行時にこの分析をやり直すので、クライアントが確認を飛ばすことはできません。対話的なターミナルの外では、CLI は確認せずに拒否します。
- **値はパラメーターとして渡します。** 絞り込み、編集、挿入で、入力されたテキストから SQL を組み立てることはありません。カラムとテーブルはデータベース自身のカタログから取り、値はパラメーターとして渡します。
- **上限:** 1 回の結果は最大 1000 行 (残りは切り捨てとして表示)、1 回の操作は 35 秒まで。

破壊的な SQL の分析は保守的なヒューリスティックで、完全なパーサーではありません。よくあるケースは捕まえますが、バックアップの代わりにはなりません。LocDev のインスタンスでは、大きな変更の前に `locdev service backup` を使ってください。

## アプリでは

- **Data:** ページ送りのグリッド (1 ページ 50 行)、カラム名のクリックで並べ替え、カラムごとの絞り込み (`=`、`!=`、`<`、`>`、`like`、`is_null` など)、ダブルクリックでの編集 (Enter で保存、Esc で取り消し、_NULL_ で null を書き込む)、挿入 (空のフィールドはカラムのデフォルトを使用)、確認付きの削除。編集、挿入、削除には主キーが必要で、ない場合は SQL を使ってください。
- **Structure:** カラム、型、NULL 許可、デフォルト、インデックス、外部キー。
- **SQL:** ハイライト付きのエディター、**Ctrl+Enter** で実行、テーブル名を挿入するショートカット、ローカルの履歴 (このコンピューターにだけ保存)。
- **Redis/Valkey:** パターンでの検索 (`user:*`)、各キーの型と TTL、型ごとの表示 (string、list、hash、set、zset)、有効期限を保ったままの文字列の編集、有効期限の解除、キーの削除。

## 既知の制限

- TLS には対応していません。SSL が必要なサーバー (`sslmode=require`、`rediss://`) への接続は失敗します。TLS を使うリモートのデータベースには、プロバイダー自身のクライアントを使ってください。
- 値はテキストとして表示されます。バイナリのカラムは情報が欠けたテキストになります。
- Linux 上で SQLite、PostgreSQL 18、Redis 8 で実際に検証しました (`crates/locdev-db/tests/real.rs` のテスト)。MySQL/MariaDB は実装済みですが、実際のサーバーでのテストはまだです。
