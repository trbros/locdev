# 新しいプロジェクト

**ツール → 新しいプロジェクト** (または `locdev new`) では、**フレームワーク自身のインストーラー** (`laravel new`、`composer create-project`、`npm create vite`、`create-vue`、`ng new`、`rails new`、`cargo init` など) を使い、そのインストーラーが文書化しているオプションでプロジェクトを作成し、最後にフォルダーをサイトとして登録します。コマンドを思い出すためにフレームワークのドキュメントを開く必要はありません。

![新しいプロジェクト: フレームワーク、作成方法、オプション、LocDev のデータベース、パッケージ、実行されるコマンド。](screenshot:new-project)

## しくみ

1. **フレームワークを選びます。** 専用の画面でフォームが開きます (**← フレームワーク** で戻ります)。それぞれに作成方法とオプションがあります (下の表)。
2. **名前とフォルダー。** 名前がフォルダーとサイトになります (`shop` → `shop.test`)。フォルダーが存在していてはいけません。LocDev が既存のものの上に作成することはありません。アプリでは新しいプロジェクトの既定の場所は `~/LocDev/Sites` です (初回に作成され、別のフォルダーも選べます)。`locdev new` は現在のフォルダーを使います。
3. **作成方法。** たとえば Laravel はインストーラーか Composer、JavaScript のプロジェクトは npm、pnpm、Bun です。各選択肢に、ツールがインストールされているかとそのバージョンが表示されます。
4. **ランタイム** (PHP と JavaScript): インストーラーを実行する PHP または Node.js のバージョンを、インストール済みのものから選びます。新しいサイトはこのバージョンに固定されます。**デフォルト** は LocDev のデフォルトのバージョンです。
5. **オプション。** スターターキット、認証、テスト、TypeScript、Tailwind、リンター、ルーティング、テンプレートなど。選んだ方法で使えるものだけが表示されます。
6. **データベース** (Laravel): LocDev のインスタンス (PostgreSQL、MySQL、MariaDB)。新規でも既存でもかまいません。
7. **パッケージ:** フレームワークに応じて npm または Packagist を検索し、作成直後にインストールします。
8. **実行される内容:** LocDev が書くファイル (ある場合)、各コマンドそのもの、その間に LocDev 自身が行うこと。**プロジェクトを作成** を押すまで何も実行されません。

その後、手順は**アプリ内のターミナル**で実行されます。インストーラーの出力が見え、質問に答えることもできます。› で始まる行は LocDev のもの (ファイル、データベース、サイト) です。完了するとサイトが登録され、**自動で起動します**。ボタンには **実行中** と表示され (クリックで停止)、隣に **サイトを開く** があります。

![アプリのターミナルで手順ごとに実行されるインストーラー。](screenshot:new-project-run)

## ジェネレーターのないフレームワーク

素の PHP、Flask、FastAPI、Axum、Actix Web には、すぐに配信できるプロジェクトを作る公式コマンドがありません。これらでは、LocDev がコマンドの前に**最小限の初期ファイル**を書きます。計画にはそれぞれが表示されます (クリックで内容を表示)。

| フレームワーク | LocDev のファイル                                                 | その後                                                            |
| -------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------- |
| PHP            | `public/index.php`                                                | `git init` (任意)                                                 |
| Flask          | `app.py`、`requirements.txt` (`flask`)、`.gitignore`              | `.venv` + `pip install -r requirements.txt`、`git init`           |
| FastAPI        | `main.py`、`requirements.txt` (`fastapi[standard]`)、`.gitignore` | `.venv` + `pip install -r requirements.txt`、`git init`           |
| Axum           | `src/main.rs`                                                     | `cargo init`、`cargo add axum`、`cargo add tokio --features full` |
| Actix Web      | `src/main.rs`                                                     | `cargo init`、`cargo add actix-web`                               |

LocDev はフォルダーを作り、何も上書きせずにファイルを書いてから、公式のツールを実行します。`cargo init` はすでにある `src/main.rs` をそのまま使い、`pip` は `requirements.txt` にあるものをインストールします。ファイルは検出がフレームワークを認識するように選んであり (テストで確認)、サイトは正しいコマンドで起動します: `php -S … -t public`、`flask --app app run`、`fastapi dev main.py`、`cargo run`。Rust のサーバーはポートを `PORT` から読み、これは LocDev が設定します。

WordPress は WP-CLI (`wp core download`。ダウンロードのハッシュを検証します) を使います。セットアップは最初のアクセス時にブラウザーで完了します。

## コマンド以外に LocDev が行うこと

LocDev のデータベースを選んだ場合、作成とマイグレーションの間に次を行います。

- インスタンスがまだなければ作成し、起動します。
- 接続を受け付けるまで待ち、プロジェクト名のデータベースを作成します (`my-shop` → `my_shop`)。
- **新しく作成したプロジェクトの** `.env` にある `DB_CONNECTION`、`DB_HOST`、`DB_PORT`、`DB_DATABASE`、`DB_USERNAME`、`DB_PASSWORD` をそのインスタンスに向けます (既存の行はその場で置き換え、Laravel がコメントアウトした行も対象です。ファイルの残りはそのままです)。
- `php artisan migrate` を実行します。

Laravel のインストーラーでは、ドライバー (`--database=pgsql`、`mysql`、`mariadb`) は選んだデータベースに合わせます。

## フレームワークとオプション

<!-- generated:starters -->

各インストーラーのフラグはそのソースコードか公式ドキュメントに基づいており、すべての組み合わせで生成されるコマンドをテストで確認しています。コマンドはシェルを使わず、固定のリストにあるプログラム (`laravel`、`composer`、`npm`、`npx`、`pnpm`、`bun`、`bunx`、`rails`、`dotnet`、`python3`、`cargo`、`wp`、`git` など) だけで実行され、プロジェクト名とパッケージ名はコマンドに入る前に検証されます。

すべてに共通するコマンドの細かな点が 2 つあります。

- `npm create --yes x@latest` は、ジェネレーターをダウンロードしてよいか npm が尋ねて止まるのを防ぎます。
- インストーラーは質問を省略するオプションなしで実行します (`laravel new`、Composer、`artisan migrate` の `--no-interaction`、`create-next-app` と `create-astro` の `--yes`、`ng new` の `--defaults`)。LocDev のフラグで決まらない項目はインストーラーがターミナルで質問し、出力やエラーもそこに表示されます。
- Angular CLI は `NG_CLI_ANALYTICS=false` (コマンドに表示) で実行します。使用状況データを送らず、作成の途中でそれについて尋ねられないための、文書化された方法です。

## インストーラー

**ツール → インストーラー** (または `locdev tools list`) には、プロジェクトに必要なツール、インストール済みかどうか、バージョン、場所が表示されます。

| ツール                                                                           | LocDev からのインストール方法                                                    |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| PHP、Composer、Node.js、npm、pnpm                                                | [ランタイム](/docs/runtimes)で、検証済みのダウンロード                           |
| Laravel インストーラー                                                           | コマンドを表示したあと、ターミナルで `composer global require laravel/installer` |
| Rails                                                                            | `gem install rails` (Ruby が必要。Ruby の gem フォルダーに管理者権限が必要な場合はユーザー単位でインストール) |
| Symfony CLI、Bun、uv、Poetry、Cargo、rustup、WP-CLI、.NET SDK。Windows と macOS では Python、pipx、Git も | 開発元が案内するコマンド ([アプリからのインストール](#アフリからのインストール)を参照)。それ以外は公式インストーラーへのリンク |

ツールは、`PATH` の外でも公式インストーラーが置く場所から見つけます: Composer のグローバルフォルダー (`~/.config/composer/vendor/bin`、`~/.composer/vendor/bin`、`%APPDATA%\Composer\vendor\bin`)、`~/.bun/bin`、`~/.cargo/bin` (rustup)、`~/.local/bin`、Symfony CLI のフォルダー (`~/.config/symfony-cli/bin`)、ユーザー単位の gem (`~/.local/share/gem/ruby/*/bin`)。これらは `PATH` の後ろに置かれるため、`PATH` の先頭にある LocDev のランタイムが見つかり、使われます。

## コマンドラインから

```bash
locdev new                                        # 何を作るか順に尋ねる
locdev new list                                   # フレームワーク、方法、オプション
locdev new laravel shop --php 8.4 -o kit=react --database postgres:db
locdev new nextjs web --package-manager pnpm -o linter=biome -p zod
locdev new vite ui --typescript
locdev new django api --in ~/Code --yes
locdev new rust edge -o framework=actix
locdev tools list
locdev tools install laravel
```

`locdev new <フレームワーク> <名前>` は `locdev new project <フレームワーク> <名前>` (引き続き使えます) と同じで、オプションも同じです。

| オプション                         | 内容                                                           |
| ---------------------------------- | -------------------------------------------------------------- |
| `--method` / `--package-manager`   | 作成方法 (`installer`、`composer`、`npm`、`pnpm`、`bun` など)  |
| `-o キー=値`                       | フレームワークのオプション (`locdev new list` ですべて表示)    |
| `--typescript`                     | TypeScript (対応するフレームワーク)                            |
| `--php` / `--node`                 | インストーラーを実行するバージョン。サイトはこれに固定されます |
| `-p パッケージ`                    | あとで追加するパッケージ                                       |
| `--database サービス:インスタンス` | LocDev のデータベース (Laravel)                                |
| `--in フォルダー`                  | 作成する場所 (デフォルトはカレントフォルダー)                  |
| `--no-site`                        | サイトとして登録しない                                         |
| `--yes`                            | 実行前に確認しない                                             |

ターミナルで `locdev new` だけを実行すると、フレームワーク、名前、フォルダー、方法、各オプション、ランタイムのバージョンを尋ね、計画を表示して確認を求めます。出力はターミナルに表示され、入力した内容はインストーラーに渡されます。

## 制限

- Linux でプロジェクトの作成から LocDev によるサイトの配信までエンドツーエンドで検証済み: Laravel インストーラーによる Laravel 13 (新しい PostgreSQL インスタンス、データベース作成、`.env` の設定、マイグレーションを含む)、Vue、Angular、Flask、FastAPI、素の PHP、Axum、Actix Web、そして PHP のバージョンを選んでの作成 (サイトに固定)。さらに、ブラウザで HTTPS と HTTP の両方をコンソールエラーなしで確認済み: Laravel (スターターキットあり・なし、Vite 補助付き)、Symfony (Composer)、Bedrock、Next.js、Nuxt、Vite、Vue、Astro、SvelteKit、Angular、NestJS、Django (HTTPS での管理画面ログインを含む)、Flask、FastAPI、Rails、ASP.NET Core、Axum。WP-CLI による WordPress は、WP-CLI がインストールされていなかったため実行していません。
- Django、Flask、FastAPI では、プロジェクト内の `.venv` に `pip` でパッケージを入れます。Django の設定パッケージ名は `config` で、その `settings.py` には `if DEBUG:` の中で、LocDev のプロキシ向けに `ALLOWED_HOSTS` への `.test` と `SECURE_PROXY_SSL_HEADER` が追加されます。
- `create-vue` は、何かのオプションがオンのときだけ質問を省略します。すべてオフで JavaScript の場合は、ターミナルで質問します。
- 追加したパッケージは、パッケージマネージャーが通常実行するインストールスクリプトを実行します。
- Windows と macOS はまだテストしていません (Windows のターミナルは ConPTY を使います)。

## Laravel: インストーラー自身の質問

LocDev は `laravel new` と同じ質問を同じ順序で尋ねます。

1. **スターターキットを使いますか?**
2. 使わない場合: フロントエンドのスタック (Blade、または認証なしの空のキットとして React、Vue、Svelte、Livewire)。
3. 使う場合: キットのスタック、認証プロバイダー (Laravel 標準または WorkOS)、Laravel 標準なら単一ファイルの Livewire コンポーネントとチーム機能。
4. その後: テスト (Pest または PHPUnit)、データベース、フロントエンドの依存関係、Laravel Boost、Git。

インストーラーは対応するフラグ (`--react --workos`、`--vue --no-authentication`、`--livewire-class-components`、`--teams`…) で実行されます。Composer の場合、同じ選択がキットのパッケージとブランチ (`laravel/react-starter-kit:dev-teams`、`--stability=dev`) になります。

## アプリからのインストール

**ツール → インストーラー** はスターターに必要なツールを一覧表示し、アプリを離れずにインストールします。PHP、Composer、Node.js、pnpm は検証済みランタイムとしてインストールし、その他は公開元がお使いのシステム向けに文書化したコマンドを、先に表示してからアプリ内のターミナルで実行します。

| ツール                 | コマンド                                                                                                          |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Laravel インストーラー | `composer global require laravel/installer`                                                                       |
| Symfony CLI            | `scoop install symfony-cli` (Windows)、`brew install symfony-cli/tap/symfony-cli` (macOS)、公式スクリプト (Linux) |
| Bun                    | 公式スクリプト (`curl -fsSL https://bun.sh/install \| bash`、Windows は PowerShell) |
| uv                     | `winget install --id=astral-sh.uv -e` (Windows)、公式スクリプト (macOS、Linux)                                    |
| Poetry                 | 公式インストーラー                                                                                                |
| Python                 | `winget install -e --id Python.Python.3.13` (Windows)、`brew install python` (macOS)。Linux ではディストリビューションのパッケージ |
| pipx                   | `scoop install pipx` (Windows)、`brew install pipx` (macOS)。Linux ではディストリビューションのパッケージ |
| Rust (rustup、Cargo)   | 公式スクリプト (macOS、Linux)                                                                                     |
| WP-CLI                 | `composer global require wp-cli/wp-cli-bundle`                                                                    |
| Git                    | `winget install --id Git.Git -e --source winget` (Windows)、`brew install git` (macOS) |
| .NET SDK               | `winget install Microsoft.DotNet.SDK.10` (Windows)、`dotnet-install.sh --channel LTS` (macOS、Linux)              |
| Rails                  | `gem install rails`。Ruby の gem フォルダーに書き込めない場合 (システムの Ruby) は `gem install --user-install rails` |

お使いのシステム向けのコマンドがないツールは、インストールページへのリンクのままです。ターミナルでは `locdev tools install <id>` が同じことをします。
