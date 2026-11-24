# Debug Center

**ツール → Debug Center**(`Ctrl` `Shift` `D`)または `locdev debug` で、サイトで起きた問題を 1 か所に集めます。スタックトレース付きの例外、PHP の `dump()`/`dd()`、警告・エラーのログ行、5xx 応答、落ちたプロセス、失敗したビルド、エラーになった LocDev のジョブです。秘密情報は保存前に隠されます。

![Debug Center: タイムライン、考えられる原因とスタックトレース付きの例外。](screenshot:debug-center)

## イベントの出どころ

| 出どころ                                                         | イベントになるもの                                                             |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| プロセスの出力(サイト、補助開発サーバー、補助プロセス、サービス) | 例外とスタックトレース、ビルドエラー、PHP の警告、警告・エラーと明示された行   |
| Laravel の `storage/logs`(`laravel.log` と当日の日次ファイル)    | 選んだレベル以上のエントリー(例外とスタック付き)。サイト起動後に書かれた分のみ |
| LocDev のプロキシ                                                | 5xx 応答と応答しなかったサイト(メソッド、アドレス、ステータス、時間)           |
| スーパーバイザー                                                 | エラーで終了したプロセス(最後の出力行付き)                                     |
| LocDev のダンプサーバー                                          | 取り込みを有効にした PHP サイトの `dump()` と `dd()`                           |
| ジョブキュー                                                     | 失敗したジョブ(依存関係の確認、インストールなど)                               |

LocDev は読み取るだけで、プロジェクトのログ設定やファイルは変更しません。

### ランタイム別

| ランタイム | LocDev が認識するもの                                                                                                                                                                                   |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PHP        | `Stack trace:` 付きの `PHP Fatal error: Uncaught …`、ファイルと行付きの `PHP Warning`/`Deprecated`、組み込みサーバーの形式(`[500]: GET /path - Uncaught …`)、Laravel のログ、Symfony VarDumper のダンプ |
| Node.js    | `at 関数 (ファイル:行:列)` 行付きの `TypeError: …`、Vite のエラー(`[vite] Internal server error`)、Next.js のエラー(`⨯ …`、`Module not found`)、TypeScript のエラー(`error TS2322`)                     |
| Python     | `Traceback (most recent call last):` から `型: メッセージ` の行まで、`logging` の既定形式(`ERROR:logger:メッセージ`)                                                                                    |
| Rust       | パニック(`thread 'main' panicked at ファイル:行:列:`)、cargo のエラー(`error[E0425]`)、`tracing` の既定形式                                                                                             |

依存関係のフレーム(`vendor`、`node_modules`、`site-packages`、cargo のレジストリ)はスタック内で折りたたまれます。それ以外の行は、明示的なレベル(`ERROR`、`[warn]`、`npm ERR!` など)があり、選んだレベル(既定: 警告)以上の場合にイベントになります。

## 画面

- **タイムライン:** 新しいイベントが先頭で、ライブ更新されます(**ライブ** で一時停止)。
- **問題:** 繰り返しをまとめて表示します(同じエラー種別、数値や値を除いた同じメッセージ、プロジェクトのコード内の同じ場所)。合計数と過去 24 時間の時間別グラフ付き。
- **フィルター:** サイト(左側)、種類、レベル、メッセージ・エラー種別・ファイル・URL での検索。
- **詳細:** メッセージ、エラー種別、フレームワーク、ランタイム、ファイルと行、発生日時、リクエスト(メソッド、URL、ステータス、時間、わかる場合はコントローラー)、スタックトレース、ダンプの値、コンテキスト、出力そのままのテキスト。
- **考えられる原因:** 既知のエラー(データベースが接続を拒否、ポート使用中、`APP_KEY` がない、クラスやモジュールが見つからない、Vite のマニフェストがない、PHP のメモリ上限など)には、よくある原因のヒントを表示します。
- **操作:** **コピー**(issue に貼るテキスト)、**JSON**、**発生**(その問題の繰り返しのみ)、**エクスポート**(JSON Lines)、[コードエディター](/docs/editors)でファイルを該当行で開く(イベントと、サイト内の各スタックフレームから)。

## PHP のダンプ(`dump()` と `dd()`)

![ダンプ: 可視性を示したオブジェクトのツリー。機密の値は隠されています。](screenshot:debug-dump)

Symfony VarDumper を使う PHP サイト(Laravel、Symfony)では、左でサイトを選んで **dump() と dd() を取り込む** をオンにします(または `locdev debug dumps <サイト> on`)。サイトは再起動します。以後、ダンプはページではなく Debug Center にツリーで表示されます(配列、public `+`・protected `#`・private `-` のプロパティを持つオブジェクト、再帰の印)。

プロジェクトのファイルを変えずに、次のように動作します:

- LocDev は `VAR_DUMPER_FORMAT=server` と `VAR_DUMPER_SERVER=127.0.0.1:<ポート>` を **サイトのプロセスの環境だけに** 設定します。VarDumper 自身の `server` 形式で、Laravel もこれに従います。
- PHP の組み込みサーバー(`php -S`。`artisan serve` もこれを使います)は環境変数を `$_SERVER` に渡しませんが、VarDumper は `$_SERVER` しか見ません。そのため、LocDev が PHP サイトに追加する `auto_prepend_file`(`data/php-prepend/` にあり、"Managed by LocDev" の印付き。HTTPS のリクエストの印付けも行います。**HTTPS とローカル CA** を参照)が、2 つの変数を `$_SERVER` にコピーします。PHP にすでに `auto_prepend_file` があった場合(Herd のものなど)、そのすぐ後に引き続き実行されます。
- LocDev のダンプサーバーが応答しない場合、VarDumper は従来どおりダンプを出力します。
- `dd()` は従来どおりリクエストを終了します。LocDev は値を表示しますが、その後に続くはずだった処理は残しません。

LocDev は PHP のシリアライズ形式を自前で読み、PHP の実行や `unserialize` の呼び出しは行いません。LocDev がサイトのために動かす補助プロセス(キュー、スケジューラー)にも同じ環境が渡されるため、そのダンプもコマンドライン付きで届きます。

## 秘密情報

イベントを保存・表示する前に、LocDev は次のものを隠します(`[redacted]`):

- サイトの `.env` と `.env.local` の値。ただし単なる設定(ホスト、ポート、ドライバー、名前、レベル、`true`/`false`、数値)は除きます。
- LocDev がサイトに注入する[共有変数とシークレット](/docs/environment)の値。
- `Authorization`、`Cookie`、`Set-Cookie`、`X-Api-Key` ヘッダー、`Bearer` トークン、JWT、URL やテキスト内の `password=`、`token=`、`secret=`、`api_key=` など、URL 内のパスワード(`postgres://user:パスワード@…`)。
- JSON とダンプでは、`password`、`token`、`secret`、`api_key`、`cookie`、`session` のような名前や `_key` で終わる名前のキーの値。

これは識別できるものを対象にしています。認識できる名前がなく、パターンのない場所にある秘密情報は隠れないことがあります。イベントはこのマシンにだけ残ります。

## 保存とクリーンアップ

イベントは `data/observe/<サイト>/<日付>.jsonl` に保存され、新しいものはメモリにも保持されます。**設定 → 自動化 → Debug Center** で次を設定できます:

- **イベントを収集**(既定でオン)
- **保存期間**(既定: 7 日)と **最大使用量**(既定: 100 MB)。どちらかの上限に達すると、古い日から削除されます。
- **通常のログ行のレベル:** 情報、警告(既定)、エラー。

**クリア** はサイト(またはすべて)のイベントをメモリとディスクから削除します。同じ設定は `settings.toml` にもあります(`[observe] enabled`、`retention_days`、`max_mb`、`logs_from`)。

## コマンドライン

```bash
locdev debug events
locdev debug errors
locdev debug status
```

- `locdev debug events -f` でライブに追跡します。`--site`、`--kind`(複数可)、`--level`、`--search` で絞り込み、`--json` でスクリプト向けに出力します。
- `locdev debug errors` は繰り返しを過去 24 時間のグラフ付きでまとめます。
- `locdev debug show <id>` はイベント全体(スタック、リクエスト、ダンプ、考えられる原因)を表示します。
- `locdev debug export` は JSON Lines を出力し、`locdev debug clear --site <サイト>` でクリア、`locdev debug dumps <サイト> on|off` でダンプの取り込みを切り替えます。

## 制限事項

- Linux で検証済み: 実際の Python サイトでパイプライン全体(トレースバック、プロキシでの 500、`.env` の秘密情報の非表示、グループ化)、実際の PHP 8.5 + Symfony VarDumper 7.4 で組み込みサーバー経由(ページに出さずにダンプを取り込み、リクエスト付きの例外)。完全な Laravel アプリケーション、Windows、macOS では未テストです。
- Node.js、Next.js、Vite、Python、Rust の形式は各ツールの既定の出力に基づきます。独自形式のログ(構造化 JSON など)は、明示的なレベルがある場合にのみイベントになります。
- Laravel のログは `storage/logs` の `single` と `daily` チャネルのみ読み取ります。ほかの出力先(Papertrail、syslog、stderr)は対象外です。
- ブラウザー内だけで起きるエラー(フロントエンドの JavaScript)は LocDev を通りません。
