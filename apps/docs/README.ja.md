# LocDev のサイトとドキュメント

[English](README.md) · [Português (Brasil)](README.pt-BR.md) · [Español](README.es.md) · [日本語](README.ja.md)

製品サイトと完全なマニュアルは、この Next.js アプリにあります。公開ドキュメント：**https://locdev.app/docs**。アプリの機能とインストールは[リポジトリの README](../../README.ja.md) を参照してください。

## 技術構成と要件

Next.js 16 App Router、React 19、TypeScript、Tailwind CSS 4、Base UI、next-intl、next-themes、React Markdown、remark-gfm、サーバー側 Shiki を使用します。Biome で整形と lint、Node のテストランナーでコンテンツと契約を検証します。TypeScript を直接読み込むテストのため Node.js 22.18 以降と、`package.json` に指定された pnpm が必要です。

## 実行と検証

このディレクトリで `.env.example` を `.env` にコピーし、依存関係をインストールして起動します。PowerShell では必要に応じて `cp` の代わりに `Copy-Item .env.example .env` を使用してください。

```sh
cp .env.example .env
pnpm install
pnpm dev -p 1440
```

```sh
pnpm test
pnpm typecheck
pnpm exec biome check .
pnpm check:comparison
pnpm check:routes
pnpm build
```

## 言語とルーティング

すべての公開ページに `/pt-BR`、`/en`、`/es`、`/ja` の接頭辞を付けます。`/` は言語を判定し、既定でブラジルポルトガル語へ移動します。ポルトガル語のソースファイルは `br`（`messages/br.json`、`content/docs/br/`）を使用しますが、公開 locale は `pt-BR` です。旧 URL は 308 で転送し、テストで転送と代替言語を確認します。`@triooo/named-routes` が `lib/routes.generated.ts` を生成します。ルート変更後は `pnpm routes`、検証には `pnpm check:routes` を実行してください。

## 記事と生成リファレンス

記事は `content/docs/{br,en,es,ja}/<slug>.md` にあり、全言語で同じ英語の slug を使用します。ナビゲーションは `lib/docs.ts` の `docGroups`、翻訳ラベルは `lib/docs-labels.ts` です。4 言語を同時に更新してください。内部記事リンクは `/docs/<slug>` とし、レンダラーが locale を付けます。画像は `![説明](screenshot:<name>)` を単独行に、注意書きは `> [!NOTE]` や `> [!WARNING]` を使用します。CLI とフレームワーク表は `<!-- generated:cli -->` と `<!-- generated:frameworks -->` を使用します。

ルートで `make cli-catalog` を実行すると、Rust から CLI・フレームワーク・新規プロジェクトのカタログを生成し、4 言語の変更履歴をコピーします。変更履歴だけを更新する場合は `node apps/docs/scripts/sync-changelogs.mjs` を実行します。`--check` は書き込まずに一致を確認します。原本は `CHANGELOG.md`、`CHANGELOG.pt-BR.md`、`CHANGELOG.es.md`、`CHANGELOG.ja.md` です。サイト用コピーは `generated/`、デスクトップ用は `../desktop/src/generated/` にあり、選択された言語で表示します。翻訳でもリリース見出し、バージョン、日付、状態マーカーを保持してください。

## 環境変数とデプロイ

`NEXT_PUBLIC_GITHUB_REPOSITORY_URL` は、認証情報、クエリー、フラグメントを含まない GitHub リポジトリのルート HTTPS URL に設定します。リンク、issue、clone コマンド、リリース API はこの値から生成します。`NEXT_PUBLIC_SITE_URL` は公開サイトの URL です。公開変数はビルド時に組み込まれるため、変更後は再起動と再ビルドが必要です。`.env` と `.env.local` をコミットしないでください。

Vercel では Root Directory を `apps/docs` にし、ディレクトリ外のファイルへのアクセスを有効にします。`next.config.ts` がルートの `Cargo.toml` を読むためです。ビルド環境に `.env.example` の変数を設定してください。`.github/workflows/ci.yml` の `docs` ジョブが lint、型検査、テスト、比較、ルート、プロダクションビルドを実行します。

## プライバシー・セキュリティ・支援

ホスティングと連絡先は `SITE_HOSTING_PROVIDER`、`SITE_PRIVACY_CONTACT_URL`、`SITE_SECURITY_REPORT_URL`、`SITE_SECURITY_REPORTING_CONFIRMED` で設定します。セキュリティ報告は有効な連絡先と明示的な確認がある場合のみ有効です。それ以外ではルートを公開せず、サイトマップにも含めません。`SITE_HOSTING_LOG_RETENTION` は任意で、確認済みのホスティング設定のみを記述し、保存期間を推測しないでください。

`/about`、`/support`、`/privacy`、`/security`、`/contribute`、`/donate` は公開のプロジェクト情報です。支援ページはコミュニティへの協力の呼びかけで、サイト内で寄付の決済は行いません。ダウンロードには実際に公開されたファイルが必要です。他ツールとの比較はリポジトリの根拠と一致させてください（`pnpm check:comparison`）。

## ドキュメントの保守

記事、生成リファレンス、翻訳は実装済みの動作に合わせてください。診断、更新、アンインストールのガイドも含まれます。完全に削除する場合は、デーモンが利用できる間に DNS と CA の信頼を元に戻します。デスクトップの mock から、4 言語と両テーマのスクリーンショットを生成してください。第三者のライセンスや帰属表示の公式原本を翻訳版で置き換えないでください。
