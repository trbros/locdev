# フレームワークとスタック

フォルダーを追加すると、LocDev は**ファイルを読むだけで**プロジェクトの構成を判別します。対象はマニフェスト (`package.json`、`composer.json`、`Gemfile`、`pyproject.toml`、`requirements.txt`、`Cargo.toml`、`go.mod`、`pom.xml`、`build.gradle`、`*.csproj`)、ロックファイル、設定ファイルです。プロジェクトのスクリプトは一切実行しません。

結果は**コンポーネント** (フレームワーク、ランタイム、言語、UI、ツール) の一覧で、それぞれに判定の根拠と、わかる場合はバージョンが付きます。バージョンはインストール済みのもの (`node_modules`、`vendor/composer/installed.json`、`Gemfile.lock`、`uv.lock`、`poetry.lock`、`Cargo.lock`、`pom.xml`)、なければ宣言された範囲です。そのうち 1 つが**メイン**で、開発サーバーの起動方法を知っているアダプターです。

```bash
locdev stack detect ~/Code/api     # 任意のフォルダー
locdev stack inspect shop          # サイト (根拠、バージョン、前提条件付き)
```

## 連携の種類

| 種類                      | 意味                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| ネイティブ                | LocDev がホスト `127.0.0.1` とサイトのポート (`{port}`) でコマンドを組み立てる                                                                    |
| package.json のスクリプト | プロジェクトのスクリプト (`dev`、`start` など) を実行する。フレームワークが引数でポートを受け取らない場合は、アプリが `PORT` 変数を読む必要がある |
| カスタムコマンド          | フレームワークは認識したが、起動コマンドはあなたが指定する (`locdev edit <site> --command`)                                                       |
| コンポーネント            | ほかのものを通して動く (React は Vite 経由、Tailwind はビルド経由)                                                                                |

一部のアダプターはフレームワーク独自の変数でもポートを渡します。Spring Boot には `SERVER_PORT` と `SERVER_ADDRESS`、.NET には `ASPNETCORE_URLS=http://127.0.0.1:<port>` を渡します (`launchSettings.json` がアドレスを変えないよう `--no-launch-profile` 付き)。`.venv/` (または `venv/`) のある Python プロジェクトは、仮想環境を `PATH` の先頭に置き、`VIRTUAL_ENV` を設定して動かします。

複数のフレームワークが一致した場合 (NestJS には Express も含まれる)、**このプロジェクトを起動できるもの**のうち優先度が最も高いものが選ばれます。たとえば、Vite がインストールされていても実行するスクリプトがなければメインにはならず、汎用の Node.js が使われます。アダプターがサポートするより古いバージョンは警告として表示されますが、ブロックはしません。

## モノレポ

npm/Yarn のワークスペース (`workspaces`)、`pnpm-workspace.yaml`、Cargo の `[workspace]`、`go.work` を読み、各パッケージがそれぞれのフレームワークとともに表示されます。各パッケージは別々のサイトになります (`locdev add apps/web`)。`[package]` のない Cargo ワークスペースのルートでは、コマンドは提案されません。

## 手動での選択

検出だけでは足りないとき:

```bash
locdev stack add tailwind shop                 # ファイルからは判定できないコンポーネント
locdev stack remove python shop                # 検出されたコンポーネントを隠す
locdev stack use flask shop --apply-command    # アダプターを選び、そのコマンドを使う
locdev stack use auto shop                     # 検出に戻す
```

選択は `projects.toml` (`adapter`、`stacks`、`hidden_stacks`) に保存されます。アプリでは、サイト概要の **スタック** セクションに同じ操作があります (**編集…**)。

## 更新可能な定義

各フレームワークが何であるか (検出ルール、バージョンの出どころ、コマンド、設定ファイル、`.env` のルール、公式ドキュメントへのリンク) は**定義**にまとめられています。コードとは別の純粋なデータです。LocDev には組み込みのセットがあり、アプリを更新しなくても新しいセットをダウンロードできます。

- セットは minisign で署名して公開され、LocDev は同梱の公開鍵と署名が一致するセットだけを使います。
- 形式にはバージョンがあります。このバージョンにない機能を必要とするセットは拒否され、未知のフィールドも拒否されます。
- コマンドが使えるのは固定リストのプログラム (`npm`、`php`、`python3`、`bin/rails`、`cargo`、`dotnet` など) だけで、シェルのメタ文字は使えず、パスはプロジェクト内に限られます。署名済みのセットでも、それ以外は実行できません。
- 現在のものを置き換えられるのは新しいリビジョンだけで、これまでに見た最新より古いリビジョンは拒否されます。
- `locdev stack definitions --rollback` で前のセット (または組み込みのセット) に戻せます。戻したリビジョンは自動チェックでスキップされます。
- チェックは毎日 (ジョブキュー経由) 行われ、オフラインモードを尊重し、無効にもできます: `locdev settings set automation.definitions off`。

```bash
locdev stack definitions             # 使用中のリビジョン、出どころ、最終チェック
locdev stack definitions --refresh   # 今すぐチェック
```

状態は `doctor` と **Settings → Frameworks** にも表示されます。公開は `tools/definitions` ツール (ビルド、署名、検証) で行います。このツールはレジストリ (npm、Packagist、PyPI、RubyGems) も確認し、フレームワークの新しいメジャーバージョンを知らせます。更新が公開されていない間は、LocDev に同梱された定義を警告なしで使います。

## 対応スタック

<!-- generated:frameworks -->

各スタックがよく使うサービス (データベース、キャッシュ) は `locdev stack inspect` に表示され、LocDev でそのインスタンスを作成できます ([サービス](/docs/services))。キュー、スケジューラー、WebSocket サーバーはサイトの[補助プロセス](/docs/servers)になります。
