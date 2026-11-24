# クイックスタート

## 1. デーモンを起動する

アプリは開くときに自動で起動します。ターミナルからは次のとおりです。

```bash
locdev start
locdev status
```

```text
● daemon running  pid 41628 · up 3s
  version   0.6.0 (protocol 2)
  projects  0 registered, 0 running
  proxy     127.0.0.1:80 · *.localhost
  dns       127.0.0.1:10053 · *.test not resolving
  https     127.0.0.1:443 · CA not trusted (locdev tls trust)
```

## 2. プロジェクトを登録する

プロジェクトのフォルダー内で実行します。

```bash
locdev add . --start
```

LocDev はプロジェクトの種類を検出し、開発サーバーのコマンドを提案し、空いているポート (8100 から) を選んでサイトを起動します。アプリでは `Ctrl` `N` か **+** ボタンを使います。

![サイトの追加: 必須なのはフォルダーだけです。](screenshot:add-site)

## 3. ブラウザーで開く

インストール直後から、サイトは `http://名前.localhost` で応答します。ブラウザーは `*.localhost` を自分で解決するので、設定は不要です。

警告なしの HTTPS で `名前.test` を使うには、2 つの設定を一度だけ行います。

```bash
locdev dns install   # *.test を LocDev に解決させる
locdev tls trust     # システムとブラウザーがローカル CA を信頼する
```

どちらのコマンドも、実行する内容を正確に表示してから確認を求めます。[プロキシと DNS](/docs/domains) と [HTTPS](/docs/https) を参照してください。

## 4. 環境を確認する

```bash
locdev doctor
```

`doctor` は使用中のポート、未設定のリゾルバー、信頼されていない CA、足りないツールを、それぞれ解決するコマンドとともに示します。

## 次のステップ

チュートリアルを 1 つ選んで、最初から最後まで進めてみましょう。

- [Laravel サイトをゼロから](/docs/tutorial-laravel): LocDev がダウンロードする PHP、新規プロジェクト、HTTPS、Mailbox。
- [既存の Node.js プロジェクト](/docs/tutorial-node): フォルダー、Node.js のバージョン、見せるためのトンネル。
- [プロジェクトごととターミナルのバージョン](/docs/tutorial-versions): インストール、固定、ターミナルでのデフォルト。
