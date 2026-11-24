# デプロイアシスタント

各サイトの **デプロイ** タブ (CLI では `locdev deploy`) はスタックを分析し、プロジェクトをどのプラットフォームで本番運用できるか、何をなぜ変える必要があるかを示します。分析は推奨と準備で、どのプラットフォームにも何も送信しません。環境変数は名前だけが表示されます。実際のデプロイを確認・実行するには、Vercel か Netlify を接続します (下の「接続したプラットフォーム」を参照)。

![Deploy タブ: スタック、適合度順のプラットフォーム、準備状況。](screenshot:deploy)

```bash
locdev deploy analyze                 # スタック、プラットフォーム、準備状況
locdev deploy analyze --json          # すべて (各情報源のリンク付き)
locdev deploy config render           # LocDev が書く render.yaml を表示
locdev deploy config render --write   # ファイルを作成 (既存のファイルは上書きしない)
```

## 分析に使うもの

|                                        | 出どころ                                                                                                                                                                     |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| フレームワーク、バージョン、ランタイム | スタックの検出 (インストール済みのバージョン)                                                                                                                                |
| ランタイムのバージョン                 | `engines.node`、`.nvmrc`、composer.json の `require.php`                                                                                                                     |
| レンダリング                           | next.config の `output: 'export'`、Astro の `output`/アダプター、Nuxt の `ssr`、SvelteKit の `adapter-static` など。アプリケーションサーバー (Laravel、Django、Express など) |
| WebSocket                              | Reverb、socket.io、ws、Channels などのインストール、または LocDev で有効にした WebSocket プロセス                                                                            |
| キューとワーカー                       | Horizon、BullMQ、Celery、Sidekiq など、Laravel で `sync` 以外の `QUEUE_CONNECTION`、または有効にしたワーカー                                                                 |
| スケジューラー                         | `routes/console.php` のタスク、node-cron、celery beat など                                                                                                                   |
| データベースとキャッシュ               | Database Manager が `.env` で見つけた接続 (PostgreSQL、MySQL、SQLite、Redis) と MongoDB のドライバー                                                                         |
| ディスク上のファイル                   | `storage/app/public` にファイルがある local/public の `FILESYSTEM_DISK`                                                                                                      |
| ビルドと起動                           | ロックファイルのマネージャーで実行する package.json のスクリプト。PHP は composer                                                                                            |
| 変数                                   | `.env.example` と `.env` の名前 (値が分析の外に出ることはない)                                                                                                               |

## プラットフォーム

Vercel、Netlify、Cloudflare Workers、Railway、Render、Fly.io、Laravel Forge、自分のサーバー (VPS)、GitHub Pages。それぞれに判定が付きます。

- **適合**: そのまま動きます。「このサイトの場合」の一覧に設定すべきもの (ワーカーサービス、cron、マネージドデータベースなど) が表示されます。
- **変更が必要**: 何かを変えれば可能です (Dockerfile、外部サービスでの WebSocket、オブジェクトストレージへのアップロード、一時ディスク以外の SQLite など)。
- **不適合**: そのプラットフォームはこの種類のアプリを動かせません (サーバーレスでの PHP、GitHub Pages でのアプリケーションサーバー)。

並び順は判定と変更の数だけで決まり、**人気は関係ありません**。各機能と制限には、その根拠となったプラットフォームのドキュメントへのリンクがあります。情報をまとめた日付も画面に表示されます。プラットフォームは変わるので (たとえば Vercel は Fluid compute で WebSocket に対応しました)、決める前に情報源を確認してください。

## 準備状況

プラットフォームに関係なく行うチェック:

- **コミットされた `.env`:** シークレットが Git の履歴に残るため、唯一の `fail` 判定です。`.gitignore` に入っていない場合も警告します。
- **`.env.example`:** ない場合や、すべての変数が載っていない場合。
- **ロックファイルとバージョン:** ロックファイルがない、ランタイムのバージョンが宣言されていない。
- **Git:** コミットされていない変更。Git からのデプロイには含まれません。
- **Laravel:** ローカルの `APP_DEBUG=true`。本番でオフにするためのリマインダーです。
- **SQLite:** メインのデータベースにしていると、永続ディスクのないプラットフォームではファイルが失われます。

## 生成するファイル

形式が文書化されていて安定しているものだけです。

| プラットフォーム | ファイル                  | 条件                                                                                                                          |
| ---------------- | ------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Render           | `render.yaml` (Blueprint) | Node.js アプリ: Web サービス、`fromDatabase`/`fromService` でつないだ Postgres データベースと Key Value、`sync: false` の変数 |
| Netlify          | `netlify.toml`            | 静的ビルドまたは SPA (`index.html` へのリダイレクト付き)                                                                      |

`.env` の値はファイルに入りません。シークレットはプラットフォームが入力を求めるように宣言されます。ファイルは先に表示され、確認したときのままの場合だけ作成されます。**既存のファイルが置き換えられることはありません。**

ほかのプラットフォームはそれぞれのツールで設定し、その方法は分析に表示されます。
- **Fly.io:** `fly launch`。
- **Cloudflare:** `npx @opennextjs/cloudflare migrate` または `npm create cloudflare`。
- **Railway:** ダッシュボード、または `railway config plan/apply` で適用する `.railway/railway.ts`。古い `railway.json` は非推奨になり、2026-12-01 に使えなくなるため、LocDev は生成しません。
- **Vercel:** フレームワークを自動で検出します。

## 接続したプラットフォーム

**Vercel** か **Netlify** を接続すると、デプロイ タブの分析の上に**連携中のプロジェクト**が表示されます。各デプロイの状態、ブランチ、コミット、日時、公開中のもの、ドメイン、変数の**名前**です (LocDev がプラットフォームに値を求めることはありません)。

![連携中のプロジェクト: デプロイ、公開中のもの、ここから送られる内容。](screenshot:deploy-linked)

1. **接続:** **設定 → 連携 → デプロイ先 → 接続…** でトークンを貼り付けます。保存前にプラットフォームで確認し、**システムのキーリング**に保存します。ディスクにはアカウント名だけが残ります。Vercel: _Account Settings → Tokens_ で、プロジェクトを持つチームに限定 (チームのスラッグを入力)。Netlify: _User settings → Applications → Personal access tokens_。
2. **連携:** サイトのデプロイ タブで、プロジェクト (Vercel) かサイト (Netlify) を選びます。連携は `projects.toml` (`deploy`) に保存されます。
3. **デプロイ:** **プレビューをデプロイ** か **本番にデプロイ**。プラットフォームは、サイトのリポジトリが **GitHub にプッシュ済み**の内容をビルドします。ここのコードと一致しない場合は LocDev が拒否します: 未コミットの変更、未プッシュのコミット、detached HEAD、別のリポジトリ、プロジェクトの本番ブランチ以外からの本番デプロイ。
4. **確認:** ビルド中は一覧が自動で更新されます。実行中のデプロイの**キャンセル**、**ビルドログ**の表示、以前のデプロイへの**本番のロールバック**ができます (Vercel: 本番をそのデプロイに向ける。Netlify: そのデプロイを再公開)。

|                          | Vercel                     | Netlify                                              |
| ------------------------ | -------------------------- | ---------------------------------------------------- |
| デプロイ                 | そのコミット (`gitSource`) | Netlify の Git 連携によるブランチのビルド            |
| キャンセル、ロールバック | 可                         | 可                                                   |
| ビルドログ               | 可                         | **公開 API になし**: LocDev はデプロイのページを開く |
| 変数                     | 名前と環境                 | 名前とコンテキスト                                   |
| ドメイン                 | 可                         | 可                                                   |

出典: [Vercel REST API](https://vercel.com/docs/rest-api) と [Netlify API](https://open-api.netlify.com/) (2026-11-04 に確認)。変数の変更はまだ LocDev からはできません (プラットフォームのダッシュボードで編集)。Render、Railway、Fly.io、Laravel Forge、Cloudflare は分析のみです。

```bash
locdev deploy platforms
locdev deploy connect vercel --team acme        # トークンは引数ではなくターミナルから読む
locdev deploy projects vercel
locdev deploy link vercel shop
locdev deploy status
locdev deploy run                                # プレビュー。本番は --production
locdev deploy logs dpl_…
locdev deploy rollback dpl_…
locdev deploy disconnect vercel
```

キーリングがない環境 (サーバー、CI) では、`config.toml` の `[deploy] token_store = "file"` でトークンを `deploy-tokens.toml` (`0600`) に保存します。

## 制限

- 分析は静的です。ファイルと依存関係を読むだけで、プロジェクトは実行しません。
- Laravel 以外の Node.js ワーカーや定期タスクには既知のコマンドがないため、`render.yaml` ではメモに記載するだけです。
- Render での PHP、Python、Ruby には Dockerfile が必要ですが、LocDev は生成しません。
