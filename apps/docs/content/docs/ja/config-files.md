# Config Files

各サイトの **Config** タブには、プロジェクトにすでにある設定ファイル (`vite.config.ts`、`next.config.mjs`、`tsconfig.json`、`package.json`、`composer.json` など) がまとまっています。LocDev が理解できるオプションのフォーム、テキスト編集、履歴、インストール済みバージョンに基づく分析があります。CLI では `locdev config` です。

![Config タブ: 分析、プロジェクトのファイル、オプションのフォーム。](screenshot:config-files)

```bash
locdev config list                                   # サイトのファイル
locdev config show vite.config.ts                    # LocDev が読んだとおりのオプション
locdev config set vite.config.ts server.port 5180    # 差分を表示し、保存前に確認
locdev config check                                  # 分析 (何も変更しない)
locdev config history vite.config.ts
locdev config revert vite.config.ts <id>
```

## あるものだけ

一覧に出るのはプロジェクトにあるファイルだけです。**画面を埋めるためにファイルを作ることはありません。** CSS (`@theme`) で設定する Tailwind CSS 4 は、プロジェクトが使っていない `tailwind.config.js` ではなく、`tailwindcss` をインポートしている CSS ファイルとして表示されます。

| ファイル                                                                                                                                                                                                                                           | フォーム                                              | バージョンを決めるパッケージ         |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------ |
| `vite.config.*`                                                                                                                                                                                                                                    | サーバー、プレビュー、ビルド                          | `vite`                               |
| `next.config.*`                                                                                                                                                                                                                                    | React、セキュリティ、ビルド、ルート、画像、TypeScript | `next`                               |
| `nuxt.config.*`、`astro.config.*`、`react-router.config.*`                                                                                                                                                                                         | レンダリングとサーバー                                | `nuxt`、`astro`、`@react-router/dev` |
| `tsconfig.json`、`tsconfig.*.json`、`jsconfig.json`                                                                                                                                                                                                | チェック、モジュール、出力                            | `typescript`                         |
| `package.json`、`composer.json`                                                                                                                                                                                                                    | モジュールの種類、engines、安定性、オートローダー     | —                                    |
| `biome.json(c)`、`.prettierrc*`                                                                                                                                                                                                                    | フォーマット                                          | `@biomejs/biome`、`prettier`         |
| `svelte.config.*`、`angular.json`、`nest-cli.json`、`eslint.config.*`、`.eslintrc*`、`.babelrc`、`postcss.config.*`、`tailwind.config.*`、`phpunit.xml`、`vitest.config.*`、`playwright.config.*`、`deno.json(c)`、`wrangler.*`、Tailwind 4 の CSS | テキストのみ (構文の検証付き)                         | —                                    |

## 編集がファイルを保つしくみ

- **JSON と JSONC** は具象構文木を通して編集します。値を変えても、コメント、空行、インデント、末尾カンマのスタイルは保たれます。`package.json` と `composer.json` は厳密な JSON のままです。
- **JavaScript と TypeScript** はパーサー (oxc) で読み、**実行しません**。LocDev は `export default`、`defineConfig(...)`、`module.exports`、export された `const`、それを返す関数から設定オブジェクトを見つけ、**値の部分だけ**を置き換えます。新しいオプションは、ファイルのインデントと引用符でオブジェクトの末尾に追加します。
- コードで計算される値 (`process.env.HOST`、変数、関数呼び出し) はコードとして表示され、**フォームでは編集できません**。理由が表示され、変更はテキストエディターで行います。
- すべての変更は**保存前に差分を表示**します。テキスト編集でコメントが消える場合は、その一覧が出ます。
- ファイルは確認したときのまま (ハッシュ) の場合だけ書き込まれます。構文が不正な場合は保存しません。
- 前のバージョンは**履歴** (`data/state/config-history`、ファイルごとに最大 50 件) に入り、比較や復元ができます。
- 書き込みはアトミックで、ファイルの権限を保ちます。

**表示** ボタンはファイルのフォルダーを開きます。Windows では `.js` のデフォルト動作がスクリプトの実行なので、アプリはシステムのデフォルトのプログラムでファイルを開きません。

## バージョンごとの互換性

各オプションは自分が存在するバージョンを知っていて、バージョンはインストール済みのもの (`node_modules`、`vendor`)、次にロックファイル、最後に宣言された範囲から取ります。プロジェクトのバージョンにないオプションは理由付きで無効表示になります (例: "needs next 15.5.0 or newer (this project has 15.2.4)")。クライアントが試みても、デーモンが変更を拒否します。

## 分析

`locdev config check` (とタブの **分析** パネル) は検証可能なルールを適用します。各指摘には、インストール済みのバージョンと比べた適用範囲と、公式の情報源へのリンクがあります。簡単な修正がある場合は確認用に提示されますが、**自動で適用されることはありません**。

| ルール                                   | 条件                                                              |
| ---------------------------------------- | ----------------------------------------------------------------- |
| `vite.https-boolean`                     | Vite ≥ 5 で `server.https: true` (削除済み)                       |
| `vite.host-exposed`                      | `server.host` がネットワークに公開 (LocDev は 127.0.0.1 を使用)   |
| `next.powered-by`                        | Next.js が `X-Powered-By` を送信                                  |
| `next.images-domains`                    | Next.js ≥ 14 で `images.domains` (`remotePatterns` を使用)        |
| `next.experimental-turbo`                | Next.js ≥ 15.3 で `experimental.turbo` (現在は `turbopack`)       |
| `next.typed-routes-moved` / `-available` | Next.js ≥ 15.5 で `typedRoutes`                                   |
| `next.cache-components`                  | Next.js ≥ 16 で `experimental.dynamicIO`/`ppr`                    |
| `next.ignore-build-errors`               | TypeScript のエラーがあってもビルドが通る                         |
| `tailwind.v4-js-config`                  | Tailwind ≥ 4 で `@config` のない `tailwind.config.js`             |
| `tailwind.v4-directives`                 | Tailwind ≥ 4 で `@tailwind base`                                  |
| `tailwind.v4-postcss-plugin`             | Tailwind ≥ 4 で PostCSS の `tailwindcss` プラグイン               |
| `eslint.eslintrc-ignored`                | ESLint ≥ 9 で flat config のない `.eslintrc*`                     |
| `typescript.import-flags`                | TS ≥ 5.0 で `importsNotUsedAsValues`/`preserveValueImports`       |
| `typescript.bundler-resolution`          | TS ≥ 5.0、バンドラーを使うプロジェクトで `moduleResolution: node` |
| `composer.unstable`                      | `prefer-stable` なしで `minimum-stability` が `stable` 未満       |
| `package.manager-mismatch`               | `packageManager` が存在するロックファイルと異なる                 |

## 制限

- フォームが対応するのはスカラーのオプション (テキスト、数値、真偽値、選択肢) です。オブジェクトやリスト (プラグイン、`paths`、`remotePatterns`) はテキストで編集します。Vite のプラグインは読み取り専用で表示されます。
- テキストエディターにはファイルの種類ごとのシンタックスハイライト (JSON、JS/TS、CSS、XML など)、検索、元に戻す機能があり、保存時に構文を検証します (`Ctrl+S` で確認) が、オプションの補完はまだありません。
- 展開されたオブジェクト (`...base`) から来るオプションはフォームに表示されず、その旨のお知らせが出ます。
