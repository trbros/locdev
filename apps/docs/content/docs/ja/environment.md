# 環境変数とシークレット

LocDev は、複数のプロジェクトで使いたい変数 (Stripe のキー、Sentry の DSN、API トークン) を保管し、**`.env` ファイルを変更せずに**サイトのプロセスへ渡します。ファイルが変わるのは、`locdev env sync` で頼んだときだけです。

![サイトの Environment タブ: 適用された変数、.env ファイル、競合。](screenshot:site-environment)

## 保存場所

| 内容                                                     | 場所                                                                                              |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| プロファイル、名前、シークレットでない値、各サイトの選択 | `config/env.toml` (`0600`)                                                                        |
| シークレットの値                                         | `data/state/env-vault.json` (ChaCha20-Poly1305 で暗号化)                                          |
| 保管庫の鍵                                               | システムのキーリング (Linux は Secret Service、macOS は Keychain、Windows は資格情報マネージャー) |
| 変更履歴                                                 | `data/state/env-history.json` (**値は含まない**)                                                  |
| `.env` のバックアップ                                    | `data/state/env-backups/<site>/` (同じ鍵で暗号化)                                                 |

各シークレットの値は自分の名前と一緒に封印されます (AEAD の関連データ)。ファイル内で別のキーにコピーした値は開けません。キーリングから鍵が消えた場合、LocDev は**新しい鍵を作りません** (作ると事実上シークレットを消すことになるため)。`doctor` で警告し、通常の変数は引き続き使えますが、シークレットは注入されなくなります。

キーリングのないマシン (グラフィカルセッションのないサーバーなど) では、鍵をファイルに置けます。

```toml
# config.toml
[env]
key_store = "file"   # このファイルの隣に env-vault.key (0600)
```

このモードでは鍵ファイルがシークレットそのものと同じくらい機密になり、`doctor` に警告が出ます。

新しい変数はデフォルトで**シークレット**です。例外は、フレームワークがブラウザー向けのコードに埋め込むプレフィックス (`NEXT_PUBLIC_`、`VITE_`、`NUXT_PUBLIC_`、`PUBLIC_`、`REACT_APP_`、`GATSBY_`、`EXPO_PUBLIC_`、`VUE_APP_`) です。これらは公開されるので、シークレットには**できません**。LocDev やシステムが管理する変数 (`PATH`、`HOME`、`PORT`、`HOST`、`NODE_OPTIONS`、`LD_*`、`DYLD_*`、`LOCDEV_*` など) も共有できません。

## プロファイルと継承

- **global**: _グローバル変数を受け取る_ をオフにしない限り (`locdev env use <site> --global false`)、すべてのサイトが受け取ります。
- **ほかのプロファイル** (Laravel、Staging、特定の顧客など): 有効にしたサイトにだけ適用されます。順序は global が最初で、そのあと有効にした順のプロファイルです。**最後のものが優先**され、上書きされたものは `overridden` と表示されます。
- どのプロファイルのキーでも、サイトごとにオフにできます (`--disable KEY`)。

## 注入と競合

サイトの起動時に、デーモンは変数を解決し、**プロセスの環境にだけ**入れます。コマンドライン引数、ログ、一時ファイルには何も書きません。

Laravel (phpdotenv)、Next.js (`@next/env`)、Vite、Nuxt、`dotenv` パッケージは、ファイルの値よりも、すでにプロセスの環境にある変数を優先します。そのため、プロジェクトがすでに定義しているキーを注入すると、**プロジェクトの値が黙って置き換わって**しまいます。LocDev は決してそうしません。開発サーバーが読むファイルにキーがあればスキップし、サイトのログに記録します。

```text
[LocDev] Warning: STRIPE_SECRET_KEY already exists in .env.local
[LocDev] Skipping shared variable STRIPE_SECRET_KEY (profile global).
```

何が読まれるかはフレームワークによります (`locdev env check` で各ファイルの順序を確認できます)。

| フレームワーク         | 開発時に読まれるファイル (最初のものが優先)                                                                                 |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Next.js                | `.env.development.local`、`.env.local`、`.env.development`、`.env`                                                          |
| Vite、Astro、SvelteKit | `.env.development.local`、`.env.development`、`.env.local`、`.env`                                                          |
| Nuxt                   | `.env`                                                                                                                      |
| Laravel                | `.env` (`bootstrap/cache/config.php` があると、`php artisan config:clear` まで Laravel は環境を無視し、LocDev が警告します) |
| PHP/WordPress          | `composer.json` に `vlucas/phpdotenv`、`symfony/dotenv`、`roots/wp-config` がある場合のみ `.env`                            |
| Node.js                | `dotenv` がある場合のみ `.env`。コマンドが `node --env-file=…` を使う場合はそのファイル                                     |
| 認識できない           | 念のため `.env.local` と `.env`                                                                                             |

サーバーが読まないファイル (`.env.example`、`.env.production`、`.env.test`) は情報としての競合で、注入を妨げません。その場合もスキップするには:

```bash
locdev settings set env.conflicts any_file
```

デーモンがプロセスを起動するときの環境にすでにあるキーと、LocDev 自身がプロセスに設定するキー (Laravel の Vite の `APP_URL` など) も、警告付きでスキップされます。公開プレフィックスを持つシークレットは**ブロック**されます。

変更が実行中のプロセスに届くのは再起動後です。プロセスが古いセットを使っていると、サイトの **環境** タブに表示されます。公開変数 (`NEXT_PUBLIC_`、`VITE_` など) はビルド時にコードへ埋め込まれるので、開発サーバーを再起動し、本番ビルドもやり直してください。

## ファイルとの同期

`locdev env sync` は、プロジェクトのファイルに足りない変数をコピーします。サイトを指定しない場合は、登録されたすべてのサイトが対象です。

```bash
locdev env sync --dry-run              # すべてのサイト、表示のみ
locdev env sync shop                   # shop サイトの .env
locdev env sync shop --file .env.local
locdev env sync shop --file .env.example
```

- 末尾に LocDev のコメント付きで行を**追加するだけ**です。ファイルのコメント、順序、BOM、改行コード (`\r\n`) はそのまま残ります。
- ファイルにすでにあるキーと、サーバーが読むほかのファイルで定義されたキーはスキップします。
- `.env.production`/`.env.staging` (およびその `.local`) にシークレットを**コピーすることはありません**。
- `.env.example` (と `.sample`、`.dist`、`.template`) には、シークレットかどうかに関係なく名前だけを空の値で書きます。これらのファイルはふつう Git に入るからです。
- プレビューではシークレットが伏せられます。書き込みは、ファイルと変数がプレビュー時のままの場合にだけ行われ、変わっていれば LocDev が再確認を求めます。
- 書き込む前に、ファイルの暗号化コピーを保存します (サイトごとに最大 20 個): `locdev env backups shop` と `locdev env restore <id> shop`。復元するときも、先に現在のファイルを保存します。
- 書き込みはアトミックで、ファイルの権限を保ち、シンボリックリンクと UTF-8 でないファイルは拒否します。

## コマンド

```bash
locdev env list                         # 変数 (シークレットは伏せ字) と保管庫
locdev env add STRIPE_SECRET_KEY        # 値は標準入力から (シェルの履歴に残らない)
locdev env add SENTRY_DSN https://… --plain
locdev env add QUEUE_CONNECTION redis --plain --profile laravel
locdev env show STRIPE_SECRET_KEY --reveal
locdev env remove SENTRY_DSN
locdev env profile add laravel --label Laravel
locdev env use shop --add-profile laravel --disable SENTRY_DSN
locdev env check shop                   # プロセスが受け取るもの、ファイル、競合
locdev env conflicts shop
locdev env history                      # 変更 (値なし)
```

`locdev security` でパスワードが必要な場合、`--reveal` は LocDev のパスワードを求め、履歴に記録されます。パスワードが守るのは LocDev を通る操作で、保管庫のファイルそのものではありません。

## アプリでは

- **Settings → Environment**: 検索とプロファイルでの絞り込みができる変数一覧、シークレットの表示/非表示、作成・編集・削除 (確認付き)、プロファイル、競合ポリシー、履歴、一括同期。
- **Site → Environment**: サイトのプロファイルとその順序、各変数の状態 (`injected`、`skipped`、`overridden`、`off`、`blocked`) と理由、キーごとのスイッチ、競合、読み込み順付きの `.env` ファイル、同期、バックアップ。

## 制限

- 注入されるのは LocDev が起動したプロセスだけです。ターミナルで実行した `php artisan` や `npm run` には変数が渡りません。
- 読むのはプロジェクトのルートにある `.env*` ファイルだけです。モノレポでは、登録された各サイトをそれぞれのフォルダーで分析します。
- キーリングは Linux (Secret Service) で実際にテストしました。Keychain と Windows の資格情報マネージャーは同じライブラリを使いますが、それぞれのシステムではまだ検証していません。
