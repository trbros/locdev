# ランタイムとパッケージマネージャー

各サイトは、マシン全体の環境に手を加えずに、独自の Node.js と PHP のバージョン、独自の pnpm、Yarn、Bun、Composer のバージョンで動かせます。LocDev はサイトのプロセスの `PATH` の先頭に正しいバージョンを置きます。

LocDev は次の 3 つを区別します。

- **ランタイム**はコードを実行します: Node.js と PHP。
- **パッケージマネージャー**は依存関係をインストールします: npm、pnpm、Yarn、Bun、Composer。サイトのランタイム上で動きます (pnpm はサイトの `node`、Composer はサイトの `php` を使用)。
- **ビルドツール** (Vite、webpack、Next など) はプロジェクト自身の依存関係なので、ここではなく依存関係のページに表示されます。

![LocDev がインストールしたバージョンと、もともとマシンにあったバージョン、各ランタイムのデフォルト。](screenshot:runtimes)

## バージョンの出どころ

- **LocDev が管理** (`locdev` バッジ): 公式の配布元からダウンロードし、インストール前に**公開されている SHA-256 で検証**します。インストールは一時フォルダーで行い、`rename` で配置するので、途中で止まった状態は残りません。
- **システムで検出** (出どころのバッジ): システムのパッケージ (`/usr/bin/php8.3`)、Homebrew (`php@8.2`、`node@20`)、nvm、fnm、`PATH` 上のほかのツール。LocDev はこれらを使うだけで、変更も削除もしません。

| ツール   | LocDev によるインストール                                                                   | 検出                                           |
| -------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Node.js  | nodejs.org、`SHASUMS256.txt` で検証                                                         | nvm、fnm、Homebrew、`PATH`                     |
| PHP      | StaticPHP（Linux/macOS）と windows.php.net（Windows）のビルド済みバイナリ、公開元の SHA-256 | apt (`php8.x`)、Homebrew、ほかのツール、`PATH` |
| npm      | 各 Node.js に付属し、そのバージョンに従う                                                   | 管理対象の各 Node.js の npm、`PATH`            |
| pnpm     | npm レジストリ、`integrity` (sha512) で検証                                                 | `~/.local/share/pnpm`、`PATH`                  |
| Yarn     | npm レジストリ (`yarn` 1.x、`@yarnpkg/cli-dist` 2+)、sha512                                 | `~/.yarn/bin`、`PATH`                          |
| Composer | getcomposer.org、公開されている SHA-256 で検証                                              | `PATH`                                         |
| Bun      | インストールしない (bun.sh のインストーラーを使用)                                          | `~/.bun/bin`、`PATH`                           |

pnpm、Yarn、Composer はネイティブバイナリではありません。インストールした各バージョンには小さな*ランチャー* (`locdev-bin/pnpm`、Windows では `pnpm.cmd`) が作られ、`PATH` 上の `node`/`php`、つまりサイトのものを呼び出します。そのバージョンがより新しいランタイムを必要とする場合 (pnpm の `engines.node`、Composer の最低 PHP バージョン)、`doctor` が警告します。

## バージョンのサポート

各バージョンには公開元のサポート段階が表示されます。Node.js の公式スケジュール (`nodejs/Release`) と PHP のブランチ状態 (`php.net/releases/states.php`) です。_メンテナンス_、_セキュリティのみ_、_サポート終了_ のバージョンにはバッジが付き、サポート外のバージョンで動いているサイトがあると `doctor` が警告します。オフラインでも動くようにこのデータのコピーが LocDev に含まれていて、デーモンがバックグラウンドで更新します (`[network] offline = true` のときは更新しません)。

## LocDev の PHP

LocDev は PHP をコンパイルしません。公開元が配布しているビルド済みバイナリをダウンロードし、インストール前に必ず公開元の SHA-256 と照合します。

- **Linux と macOS:** [StaticPHP](https://static-php.dev)（static-php-cli プロジェクト）の静的ビルドを `dl.static-php.dev` から取得します。`versions.json`/`version_<MM>.json` のインデックスに各系列の最新版と SHA-256 があり、古いバージョンはそれぞれの横に公開された `.sha256` ファイルとともに表示されます。チェックサムが公開されていないバージョンは表示も、インストールもされません。
- **Windows:** [windows.php.net](https://windows.php.net/download/) の公式 zip（NTS x64）を、各 zip の SHA-256 を含む `releases.json` 経由で取得します。チェックサムが公開されているのは各系列の最新版だけなので、提供されるのもその版だけです。公式 zip には Visual C++ 再頒布可能パッケージが必要です。

LocDev は StaticPHP の `bulk` 拡張セットを使います。Laravel、Symfony、WordPress に必要な `bcmath`、`curl`、`gd`、`imagick`、`intl`、`mbstring`、`mysqli`、`opcache`、`openssl`、`pdo_mysql`、`redis`、`sodium`、`zip` などが含まれます。SQLite と PostgreSQL の PDO ドライバーはビルドに含まれる Swoole のフックが提供するため（`PDO::getAvailableDrivers()` に `sqlite` と `pgsql` が出ます）、`php -m` には `pdo_sqlite`/`pdo_pgsql` が表示されませんが、SQLite を使う Laravel は通常どおり動きます。`common` セット（`config.toml` の `[runtimes] php_flavor = "common"`）は小さく、`pdo_sqlite`/`pdo_pgsql` を拡張として含みますが、`intl`、`opcache`、`sodium`、`mysqli` はありません。サイトの拡張機能ビュー（`locdev runtime extensions`）で、各プロジェクトに足りないものを確認できます。

インストール時、LocDev はバイナリの横（PHP が最初に探す `bin/php.ini`）に `php.ini` を書き、`curl.cainfo` と `openssl.cafile` をシステムの証明書バンドルに向けます。静的ビルドはその場所を知らないため、これがないと Composer や Guzzle の HTTPS が失敗します。Windows では、生成した `php.ini` で zip に含まれる拡張を有効にします。

```bash
locdev runtime available php          # 各系列の最新版（8.2、8.3、8.4、8.5…）
locdev runtime available php --all    # チェックサムが公開されているすべてのバージョン
locdev runtime install php 8.4
```

> [!NOTE]
> 同じ構成のミラーを `config.toml` で指定できます: `[runtimes] php_static_url`（Linux/macOS）と `php_windows_url`（Windows）。チェックサムもミラーのインデックスから読むため、信頼できるミラーだけを使ってください。

システムの `php.ini` に触れずに、バージョンごとの `memory_limit`、アップロード、タイムゾーンなどを調整するには [PHP の設定](/docs/php-settings) を参照してください。

## バージョンの固定

*ピン*はライン (`22`、`8.3`) か正確なバージョン (`22.11.0`) で指定します。ラインを指定すると、そのラインでインストール済みの最新バージョンが選ばれます。

```bash
locdev runtime install node 22        # 最新の 22.x
locdev runtime install node lts       # 現在の LTS
locdev runtime default node 22        # ピンのないサイトのデフォルト
locdev edit blog --node 20            # このサイトだけのピン
locdev edit shop --php 8.3
locdev edit blog --node ""            # デフォルトに戻す
locdev reload blog                    # 反映する
```

アプリでは、ピンはサイトの **設定を編集…** に、デフォルトは **ランタイム** 画面 (**⋯** メニューまたはバージョンの右クリック) にあります。

ランタイムまたはマネージャーごとの解決順序:

1. サイト自身のピン (`locdev edit`)。
2. なければ、プロジェクトの**バージョンファイル**: `.nvmrc`、`.node-version`、`.php-version`、`.tool-versions`、`package.json` の `volta` と `packageManager`、`composer.json` の `config.platform.php`。`.nvmrc` は `lts/*`、`lts/iron`、`node` も受け付けます。
3. なければ、プロジェクトが**範囲** (`engines.node`、`require.php`) を宣言していてデフォルトがそれを満たさない場合、範囲内でインストール済みの最新バージョン。
4. なければ、デフォルト。
5. なければ、システムの `PATH` で最初に見つかるもの (LocDev がない場合と同じ動作)。

ピンやバージョンファイルがインストールされていない Node.js や PHP を指している場合、サイトは**起動せず**、エラーに解決するためのコマンドが表示されます。この状態で `reload` しても、現在のプロセスは動いたままです。パッケージマネージャーの場合は警告だけで、`PATH` のものが使われます。選ばれたバージョンがプロジェクトの宣言する範囲外の場合は、サイトの詳細と `doctor` に差異が表示されます。

## ターミナルで使う

デフォルトのバージョンは、バージョンを固定していないサイトに適用されます。ターミナル（`php`、`node`、`npm`、`composer`…）でも使うには、**Runtimes → 概要** で **ターミナルでもデフォルトのバージョンを使う** をオンにするか、次を実行します。

```bash
locdev runtime default php 8.4
locdev runtime default node 22
locdev runtime terminal on      # そのあと新しいターミナルを開く
locdev runtime terminal         # 状態、コマンド、変更したファイル
locdev runtime terminal off
```

LocDev は専用のフォルダー（Linux では `~/.local/share/locdev/bin`）に、デフォルトのバージョンを実行する小さなスクリプトをコマンドごとに置きます。アプリでデフォルトを変えると、何も開き直さずにターミナルで使われるバージョンも変わります。デフォルトのない種類にはスクリプトを作らないので、ターミナルはシステムのものを使います。

- **Linux と macOS:** `~/.bashrc` と `~/.zshrc`（存在するものと、使っているシェルのもの）の末尾に、フォルダーを `PATH` の先頭に置く印付きのブロックを追加します。fish では専用ファイル `~/.config/fish/conf.d/locdev.fish` を使います。オフにすると、そのブロックだけを削除します。
- **Windows:** フォルダーをユーザーの `Path` の先頭に置きます。システムの `Path` がその前に来るため、全ユーザー向けにインストールされた PHP や Node.js が優先されます。

## 安全な削除

`locdev runtime uninstall` は、**実行中**のサイトが使っているバージョンの削除を拒否します。停止中のサイトやデフォルトがそのバージョンに解決される場合は `--force` が必要です (アプリでは該当サイトを一覧にした確認が出ます)。システムのバージョンが削除されることはありません。

## 更新

インストールしたラインに新しいバージョンがあると (例: Node.js 22.23.4 → 22.23.6)、一覧に更新が表示されます。`locdev runtime update node 22` で新しいバージョンをインストールします。正確なバージョンを固定したサイトが壊れないよう、古いバージョンは削除するまで残ります。

## コマンド

| コマンド                                                                   | 説明                                                                                                     |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `locdev runtime list [--refresh]`                                          | 管理対象と検出されたバージョン、デフォルト、サポート、使用状況、更新。`--refresh` でマシンを再スキャン。 |
| `locdev runtime available <kind> [--all]`                                  | カタログ (各ラインの最新、またはすべて)。                                                                |
| `locdev runtime install <kind> <version>`                                  | ダウンロード、検証、インストール。`22`、`8.3`、`22.11.0`、`lts`、`latest` を受け付けます。               |
| `locdev runtime update <kind> [line]`                                      | インストール済みのラインの最新バージョンをインストール。                                                 |
| `locdev runtime uninstall <kind> <version> [--force]`                      | 管理対象のバージョンを削除。                                                                             |
| `locdev runtime default <kind> [version]`                                  | デフォルトを設定 (`--clear` で解除)。                                                                    |
| `locdev runtime terminal [on\|off]`                                        | 新しいターミナルでデフォルトのバージョンを使う（引数なしで状態を表示）。                                 |
| `locdev edit <site> --node/--php/--pnpm/--yarn/--composer/--bun <version>` | サイトのピン (`""` でデフォルトに戻す)。                                                                 |

`<kind>` は `node`、`php`、`pnpm`、`yarn`、`composer`、`bun` のいずれかです (`npm` は一覧にだけ表示されます)。サイトのプロセスには、解決されたバージョンを示す `LOCDEV_NODE_VERSION`、`LOCDEV_PNPM_VERSION` なども渡されます。

## PHP 拡張

**ランタイム → PHP → 拡張** (または `locdev runtime modules [バージョン]`) は、このマシンの PHP が読み込むすべての拡張を、バージョン、読み込んだ `php.ini`、拡張フォルダーにあるが読み込まれていないファイル (Windows では `ext` の `php_*.dll`。`php.ini` で有効化) とともに一覧表示します。LocDev は自分で検出した PHP だけを実行します。

サイトでは **概要 → PHP 拡張** (または `locdev runtime extensions <サイト>`) が、プロジェクトに必要なものと、その PHP が読み込むものを比較します。`composer.json` と `composer.lock` の `ext-*`、フレームワークの要件、`.env` が指定するデータベースドライバーが対象です。足りないものには、その PHP での入手方法が示されます。

インストール済みパッケージが **推奨** する拡張も一覧表示します。`composer.lock` の `suggest` エントリー (例: `intervention/image` → `gd` または `imagick`) を、パッケージ自身の理由とともに、読み込み済みか未導入かで示します。推奨が何かをブロックすることはありません。

```bash
locdev runtime modules 8.4
locdev runtime extensions shop
```
