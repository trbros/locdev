# PHP の設定

**ランタイム → PHP → 設定** (または `locdev php`) では、システムの `php.ini` を編集せずに、PHP の各バージョンの `memory_limit`、アップロードサイズ、タイムゾーンなどのディレクティブを調整できます。

![PHP の設定: PHP 自身の値、LocDev の値、使用中の値。](screenshot:php-settings)

## しくみ

LocDev のサイトは **PHP の組み込みサーバー** (`php -S`、`artisan serve`) で動き、`php` コマンドと同じファイルを読みます。読み込まれた `php.ini` と、スキャンフォルダー (`conf.d`) のファイルです。LocDev はそのどれも変更しません。**バージョンごとに自分のファイルを 1 つ** (`config/php/<バージョン>/90-locdev.ini`、"Managed by LocDev" の印付き) 持ち、`PHP_INI_SCAN_DIR` を使って、PHP のスキャンフォルダーのあとに LocDev が動かすプロセスへ追加します。そのため LocDev の値が優先され、それは LocDev のサイトだけに適用されます。ターミナルの `php` はそのままです。

ディレクティブごとに 3 つの値を表示します。

| 列           | 値の出どころ                                                                           |
| ------------ | -------------------------------------------------------------------------------------- |
| **LocDev**   | LocDev のファイル (空なら変更しない)                                                   |
| **PHP の値** | LocDev なしで PHP が読む値: `php.ini`、スキャンフォルダー、または PHP 自身のデフォルト |
| **使用中**   | LocDev のファイル込みで PHP が実際に読み込む値 (異なる場合は強調表示)                  |

でっち上げたデフォルト値はありません。ディレクティブを変更するまでは、お使いの PHP の値が適用されます。

バージョンはライン (`8.3`、`8.4`) ごとにまとめられます。8.3 の設定は、サイトが使うどの 8.3.x にも適用されます。画面には、どのインストールを読んだか、読み込まれた `php.ini`、スキャンフォルダーのファイルが表示され、**読み取り専用**で開けます。

## ディレクティブ

| ディレクティブ           | 種類                                      |
| ------------------------ | ----------------------------------------- |
| `memory_limit`           | サイズ (`512M`、`2G`、`-1` = 無制限)      |
| `upload_max_filesize`    | サイズ                                    |
| `post_max_size`          | サイズ                                    |
| `max_file_uploads`       | 数値                                      |
| `max_execution_time`     | 秒 (`0` = 無制限)                         |
| `max_input_time`         | 秒 (`-1` = 実行時間の上限を使用)          |
| `max_input_vars`         | 数値                                      |
| `display_errors`         | On/Off                                    |
| `display_startup_errors` | On/Off                                    |
| `log_errors`             | On/Off                                    |
| `error_reporting`        | 定数 (`E_ALL & ~E_DEPRECATED`) または数値 |
| `date.timezone`          | タイムゾーン (`America/Sao_Paulo`、`UTC`) |
| `default_socket_timeout` | 秒                                        |
| `realpath_cache_size`    | サイズ                                    |
| `opcache.enable_cli`     | On/Off (組み込みサーバーの OPcache)       |

形式が正しくない値は、何かを書き込む前に拒否されます。一緒に使うと機能しない組み合わせ (`upload_max_filesize` が `post_max_size` より大きい、`post_max_size` が `memory_limit` より大きい) や、PHP が知らないタイムゾーンは警告されます。

> [!NOTE]
> PHP のコマンドラインは `max_execution_time = 0` と `max_input_time = -1` を強制しますが、サイトを配信する組み込みサーバーはファイルの値を使います。画面にはサイトが受け取る値を表示します。コマンド (`artisan`、`composer`) は引き続き時間制限なしで動きます。

## 変更を適用する

1. **LocDev** 列の値を変更します (欄を空にすると PHP の値に戻ります)。
2. **変更を確認** で、LocDev のファイルに追加・削除される行と、再起動される実行中のサイトが表示されます。
3. **適用** すると、LocDev は現在のファイルを履歴に保存し、新しいファイルを書き、PHP 自身に実際に読み込まれる値を**確認**します。PHP が受け付けなければ (構文エラー、無視された値)、元のファイルに戻し、ほかは何も変わりません。
4. 再起動するのは、その PHP バージョンで動いているサイトだけで、それぞれのすべてのプロセス (サーバー、Vite、キュー) が対象です。アプリとデーモンは開いたままです。

**PHP の値に戻す** は、そのバージョンの LocDev のファイルを削除します (これも確認を経ます)。

## 詳細設定

**詳細設定** では、LocDev のファイルをハイライト付きのエディターで開き、`Ctrl+S` で保存できます。PHP が受け付けるディレクティブなら何でも書けます (`xdebug.mode`、`opcache.*` など)。自分で書いた行やコメントは、フォームから変更しても残ります。適用前に PHP が一時フォルダーでファイルを確認し、構文エラーがあれば表示して何も書き込みません。開いてからファイルが変わった場合 (別のウィンドウや CLI から) は、保存を拒否するので再読み込みしてください。

システムの `php.ini` とスキャンフォルダーのファイルは**読み取り専用**で表示されます。それらの値を変えるには、LocDev のファイルで上書きしてください。

## 履歴

変更のたびに、置き換えたファイルを保存します (バージョンごとに最新 30 件、`data/state/php-ini-history/<バージョン>/`)。**履歴 → 復元…** で以前のバージョンに戻せます。確認と検証も同じように行われます。

## コマンドラインから

```bash
locdev php lines                                   # バージョンと LocDev の設定の有無
locdev php show                                    # LocDev のデフォルトのバージョン
locdev php show --line 8.3
locdev php set memory_limit=512M upload_max_filesize=64M post_max_size=64M
locdev php set date.timezone=America/Sao_Paulo --line 8.3 --yes
locdev php unset memory_limit
locdev php reset --line 8.3
locdev php edit                                    # LocDev のファイルを $EDITOR で
locdev php file /etc/php/8.3/cli/php.ini           # PHP が読み込むファイル、読み取り専用
locdev php history
locdev php restore 1793830885840
```

`set`、`unset`、`reset`、`edit`、`restore` は変更内容を表示して確認を求めます (`--yes` で確認を省略)。

## 制限

- LocDev が動かすプロセス (サイト、補助サーバー、キュー) に適用されます。ターミナルで実行する `php` には LocDev のファイルは渡りません。
- LocDev は PHP-FPM や Apache/Nginx を使いません。CLI と Web サーバーの違いは、上記の 2 つの時間制限だけです。
- Linux で、システムの PHP と Yerd の PHP で検証済みです (組み込みサーバーで配信される実際のサイトが、再起動後に新しい値を受け取ることを確認)。Windows と macOS はテストしていません。Windows では `PHP_INI_SCAN_DIR` の区切り文字は `;` です。
