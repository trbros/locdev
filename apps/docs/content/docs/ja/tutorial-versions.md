# チュートリアル: プロジェクトごと、ターミナルでの PHP と Node.js のバージョン

プロジェクトはそれぞれ自分のバージョンで動かせ、ターミナルではアプリで選んだデフォルトのバージョンを使えます。このチュートリアルでは 2 つのバージョンをインストールし、1 つをサイトに固定して、ターミナルでデフォルトを使い始めます。

## 1. バージョンをインストールする

```bash
locdev runtime available php        # 各系列の最新版
locdev runtime install php 8.4
locdev runtime install php 8.3
locdev runtime install node lts
```

アプリでは **ランタイム → PHP**（または **Node.js**）**→ バージョンをインストール…**。マシンにすでにあるバージョン（システムのパッケージ、Homebrew、nvm、fnm）も表示され、使うことができます。LocDev がそれらを変更することはありません。

## 2. サイトにバージョンを固定する

```bash
locdev edit shop --php 8.3
locdev reload shop
locdev info shop                     # php 8.3.x (projects.toml 8.3)
```

固定するバージョンは系列（`8.3`。インストール済みのその系列の最新版に従います）でも、正確なバージョン（`8.3.37`）でもかまいません。固定がなければ、LocDev はプロジェクトのバージョンファイル（`.php-version`、`.nvmrc` など）、次にプロジェクトが宣言する範囲（`require.php`、`engines.node`）、最後にデフォルトのバージョンを使います。

## 3. デフォルトのバージョンを選ぶ

```bash
locdev runtime default php 8.4
locdev runtime default node 22
```

デフォルトはバージョンを固定していないサイトに適用されます。アプリでは各バージョンの **⋯** メニューにあります。

## 4. ターミナルでデフォルトを使う

**ランタイム → 概要** で **ターミナルでもデフォルトのバージョンを使う** をオンにするか、次を実行します。

```bash
locdev runtime terminal on
```

![ランタイム: デフォルトのバージョンとターミナル。](screenshot:runtimes-terminal)

新しいターミナルを開くと、`php -v`、`node -v`、`npm`、`composer` がデフォルトのバージョンになります。アプリでデフォルトを変えると、何も開き直さずにターミナルで使われるバージョンも変わります。元に戻すには:

```bash
locdev runtime terminal off
```

Linux と macOS では、LocDev は `~/.bashrc` と `~/.zshrc` に印付きのブロックを追加します（fish では専用ファイル）。オフにすると、そのブロックだけを削除します。Windows では、フォルダーをユーザーの `Path` の先頭に置きます。

## 5. 安全に更新・削除する

```bash
locdev runtime update php 8.4        # その系列の最新パッチをインストール
locdev runtime uninstall php 8.3.37
```

更新後も古いバージョンはインストールされたままなので、正確なバージョンに固定したサイトは動き続けます。動いているサイトが使っているバージョンは削除できません。
