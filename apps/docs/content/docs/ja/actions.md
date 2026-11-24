# GitHub Actions ビルダー

![Actions ビルダー: テンプレートとサイトのワークフローファイル](screenshot:actions-builder)

**Tools → Actions builder** で、サイトの GitHub Actions ワークフローを作成・編集します。結果はサイトのフォルダーの `.github/workflows/` に保存されます。GitHub でワークフローを実行するには、(Git タブかターミナルで) コミットしてプッシュします。

## テンプレート

テンプレートは、LocDev がプロジェクトのファイルで見つけた内容から組み立てます。プロジェクトのものは何も実行しません。サイトに合うものが先に表示され、**適合** の印と理由が付きます。

| テンプレート           | 生成するもの                                                                                                                                                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node.js CI             | プロジェクトのマネージャー (npm、pnpm、Yarn 1 または Berry、Bun) でインストールし、存在する `lint`、`typecheck`、`test`、`build` スクリプトを実行します。Node のバージョンは `.nvmrc`/`.node-version` または `engines` から取ります |
| PHP / Laravel CI       | `composer.json` のバージョンで `setup-php`、`composer install`、`package.json` があればフロントエンドのビルド、そのあと `php artisan test`、Pest、PHPUnit のいずれか                                                                |
| Deploy to GitHub Pages | ビルドと Pages への公開 (2 つのジョブ。`pages: write` を持つのはデプロイのジョブだけ)                                                                                                                                               |
| Dependency review      | 既知の脆弱性がある依存関係を追加するプルリクエストを止めます                                                                                                                                                                        |
| CodeQL                 | push、PR、週 1 回の JavaScript/TypeScript の静的解析                                                                                                                                                                                |
| Release on tag         | `v*` タグがプッシュされたときに、生成したノート付きのリリースを作ります                                                                                                                                                             |
| Docker image           | PR では `Dockerfile` のビルド、メインブランチとタグではビルドと GHCR へのプッシュ                                                                                                                                                   |

どれも先頭が `permissions: contents: read` で、各ジョブには必要なものだけが与えられます。

![チェックと YAML を並べたビジュアルエディター](screenshot:actions-editor)

## ビジュアルエディターと YAML

**ビジュアルエディター**で扱えるもの:

- ワークフローの名前。
- トリガー: ブランチとタグ付きの push とプルリクエスト、手動 (`workflow_dispatch`)、スケジュール (`cron`)、公開されたリリース。
- トークンの権限 (先頭とジョブごと)。
- 古い実行のキャンセル。
- ジョブ: ランナー、タイムアウト、環境、ジョブ間の依存 (`needs`)、ステップ (名前、アクションまたはコマンド、`with`、`env`、`if`)。

ステップは並べ替えられ、**ステップを追加** メニューには既知のアクションが現在のバージョンで一覧表示されます。

横の YAML は、変更のたびにデーモンが生成します。**YAML** タブではテキストを直接編集できます。ビジュアルエディターに戻るには、ファイルがエディターの扱える範囲だけを使っている必要があります。ほかの機能 (`inputs` 付きの `workflow_dispatch`、`include` 付きの `matrix`、再利用可能なワークフローなど) を使うファイルは理由付きで YAML モードで開き、何も捨てません。

## チェック

LocDev は編集中に YAML をチェックします。

| ルール                                                                                                                                          | レベル |
| ----------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| 構文、`on:`、`runs-on` やステップのないジョブ、`uses` と `run` を両方持つステップ、存在しないか循環する `needs`、不正な cron                    | エラー |
| **スクリプトインジェクション**: `${{ github.event.pull_request.title }}`、`github.head_ref`、issue やコメントの本文などを `run:` に直接埋め込む | エラー |
| PR のコードをチェックアウトする `pull_request_target`/`workflow_run`                                                                            | エラー |
| バージョンのないアクション (`uses: owner/action`)                                                                                               | エラー |
| ブランチを追うアクション (`@main`)                                                                                                              | 警告   |
| `permissions:` がない (トークンがリポジトリのデフォルトのまま) か `write-all`                                                                   | 警告   |
| 既知のアクションで、現在より古いメジャーバージョン                                                                                              | 警告   |
| SHA ではなくタグで固定したサードパーティのアクション、`run:` に埋め込んだシークレット                                                           | 情報   |

[actionlint](https://github.com/rhysd/actionlint) が `PATH` にあれば、それも実行します。YAML は標準入力で渡し、何も書き込まず、見つかったものには _actionlint_ の印が付きます。なければ、上のルールだけが適用されます。

**LocDev のエラーがあると保存できません。** 警告と actionlint の指摘は表示されますがブロックはしません。actionlint はスタイルや shellcheck の注意も出すからです。

## 驚きのない保存

**確認して保存…** は、書き込まれる内容の差分とチェックを表示します。

- **新しいファイル:** その名前のものが何もない場合にだけ作成します (アトミックな作成)。
- **既存のファイル:** "Replace the existing file" をチェックする必要があります。デーモンは、ファイルの内容 (SHA-256) が確認したときのままの場合だけ置き換えます。その間に誰かがファイルを変えていれば、書き込みは拒否され、もう一度確認することになります。置き換えはアトミックです (一時ファイル + rename)。
- 名前は `name.yml`/`name.yaml` の形式だけです。LocDev は `.github/workflows/` の外や、シンボリックリンク経由では書き込みません。

## アクションのバージョン

テンプレートは、組み込みのリストにある各アクションの現在のメジャーバージョンを使います。**チェック** は GitHub で各アクションの最新リリースを確認します (接続したアカウントがあればそれで、なければ匿名で)。結果は `data/state/action-versions.json` に保存されます。問い合わせはオフラインモードを尊重します。

## コマンドラインでは

```bash
locdev actions list                     # 合うテンプレートと既存のファイル
locdev actions new node-ci              # ワークフローとチェックを表示
locdev actions new php-ci --save        # ファイルを作成 (先に確認、置き換えはしない)
locdev actions validate                 # すべてのワークフローをチェック (エラーがあれば終了コード 1)
locdev actions versions --refresh       # GitHub に問い合わせる
```

既存のファイルを置き換えるには、差分を表示するアプリを使ってください。

## 全ワークフローのチェック

**ツール → GitHub Actions** は開いたときに `.github/workflows/` のすべてのファイルをチェックします (ターミナルでは `locdev actions validate` が同じことをします)。構造とセキュリティに加えて、失敗や停止を防ぐためにコミュニティが推奨することを、修正用の YAML 付きで提案します。

| チェック               | 理由                                                                                                            |
| ---------------------- | --------------------------------------------------------------------------------------------------------------- |
| `missing-setup`        | セットアップ用の action なしで `run:` の中に `pnpm`、`bun`、`deno` がある: GitHub のランナーには入っていません  |
| `practice-timeout`     | `timeout-minutes` のないジョブは、ステップが止まると最大 6 時間動き続けます                                     |
| `practice-concurrency` | `concurrency:` のないプルリクエストのワークフローは、不要になった実行を続けます                                 |
| `practice-cache`       | `cache:` のない `setup-node` や `setup-python` は毎回すべてをダウンロードします                                 |
| `practice-npm-ci`      | `npm install` はランナー上でロックファイルを変えることがあります。`npm ci` はその内容どおりにインストールします |
| `practice-composer`    | CI の `composer install` には `--no-interaction --no-progress` を付けるべきです                                 |
| `practice-credentials` | `persist-credentials: false` がないと、`actions/checkout` はトークンを `.git/config` に残します                 |

提案は保存をブロックしません。ブロックするのは LocDev 自身のエラーだけです。LocDev のテンプレートはすでにこれらに従っています。

### actionlint

actionlint がない場合、**actionlint をインストール** (または `locdev actions install-actionlint`) が rhysd/actionlint の最新 GitHub リリースから LocDev のデータフォルダーにダウンロードします。リリースのチェックサムファイルの SHA-256 と GitHub 自身のダイジェストが一致した場合のみです。以降の検証で使われます。

### ローカルバージョン

LocDev は、外部で変更されたワークフローファイルを読んだとき、置き換える前、保存した後にコピーを保存します (ファイルごとに最新 50 件、`data/state/workflow-history/`)。各ファイルの横の履歴ボタンで一覧表示でき、開くとエディターに読み込まれ、保存は通常のレビューを経ます。

```bash
locdev actions history ci.yml
locdev actions history ci.yml --show 1763900000000-saved
```
