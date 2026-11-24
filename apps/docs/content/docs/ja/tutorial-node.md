# チュートリアル: 既存の Node.js プロジェクト

このチュートリアルでは、手元のプロジェクト（Vite、Next.js、Nuxt、Astro、Express など）を、必要な Node.js のバージョンで LocDev に取り込み、`https://<名前>.test` で開きます。プロジェクトの中身は何も変更されません。

## 1. フォルダーを登録する

アプリでは `Ctrl` `N`（またはサイト一覧の **+**）でフォルダーを選びます。ターミナルでは、そのフォルダーの中で:

```bash
locdev add . --start
```

![サイトを追加: 必須なのはフォルダーだけ。](screenshot:add-site)

LocDev はファイル（`package.json`、`vite.config.*`、`next.config.*` など）からフレームワークを判別し、プロジェクトのものは何も実行せずに、サイトのポート（`{port}`）付きの開発コマンドを提案します。プロセスには `PORT` と `HOST=127.0.0.1` を渡し、Vite では `.test` のホストを許可します。

## 2. プロジェクトの Node.js バージョン

プロジェクトに `.nvmrc`、`.node-version`、`.tool-versions`、`package.json` の `engines.node` や `volta` があれば、LocDev はそのバージョンを使います。インストールされていなければサイトは起動せず、エラーに解決のためのコマンドが表示されます。

```bash
locdev runtime install node 20
locdev reload <名前>
```

プロジェクトに手を加えずにバージョンを固定するには、サイトの **設定** タブか次のコマンドを使います。

```bash
locdev edit <名前> --node 22
locdev reload <名前>
```

![サイトの設定: コマンド、ポート、ランタイム、起動。](screenshot:site-settings)

パッケージマネージャーも同じ仕組みです。`package.json` の `packageManager`（pnpm、Yarn、Bun）か、`locdev edit <名前> --pnpm 10` を使います。

## 3. サーバーを見守る

```bash
locdev logs <名前>       # 開発サーバーの出力
locdev info <名前>       # アドレス、解決されたバージョンとその出どころ
```

アプリでは **サーバー** タブに出力がリアルタイムで表示され、[Debug Center](/docs/debug-center) にすべてのサイトのエラーが集まります。

## 4. 誰かに見せる（任意）

動いているサイトをネットワークの外の人と共有するには:

```bash
locdev tunnel start <名前>
```

LocDev はアカウント不要の Cloudflare Quick Tunnel を使い、公開アドレスを表示します。サイトを止めるとトンネルも終了します。[トンネル](/docs/tunnels) を参照してください。

## 次のステップ

- [共有変数](/docs/environment): リポジトリに `.env` を置かずに、シークレットをプロセスに注入。
- [サブドメイン](/docs/subdomains): `api.<名前>.test` を別のプロセスやポートへ。
- [依存関係](/docs/dependencies): プロジェクトのバージョン、更新、脆弱性。
