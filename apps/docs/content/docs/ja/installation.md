# インストール

## インストーラー

各バージョンには 3 つのプラットフォーム向けのインストーラーと、ダウンロードを確認するための `SHA256SUMS` ファイルが含まれます。サイトの[ダウンロード](/download)ページはお使いのシステムで開き、最新バージョンのファイルを表示します。各バージョンの内容は[変更履歴](/changelog)にあります。自分でビルドする場合は、下記のソースを参照してください。

| プラットフォーム | ファイル                                    | 備考                                                                           |
| ---------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| Linux            | `.deb`、`.rpm`、`.AppImage` (x86_64, ARM64) | `.deb` と `.rpm` は `locdev` と `locdevd` を `/usr/bin` にインストールします。 |
| macOS            | `.dmg` (Apple Silicon と Intel)             | macOS 11 以降が必要です。                                                      |
| Windows          | `.msi` と NSIS インストーラー               | NSIS はユーザー単位でインストールし、管理者権限は不要です。                    |

アプリにはデーモンと CLI が含まれます。実行ファイルの隣にインストールされ、初回起動時にアプリがデーモンを自動で起動します。

> [!WARNING]
> バイナリに署名されるまでは、初回実行時に macOS (Gatekeeper) と Windows (SmartScreen) が警告を表示します。続ける前に `SHA256SUMS` でハッシュを確認してください。

```bash
# ダウンロードの確認
sha256sum -c SHA256SUMS --ignore-missing
```

### CLI のみ

ターミナルだけで使う場合、各バージョンにはアプリを含まない `locdev` と `locdevd` の `locdev-<バージョン>-<ターゲット>.tar.gz` (Linux x86_64 / ARM64、macOS Apple Silicon / Intel) と `.zip` (Windows) があります。インストールスクリプトはお使いのプラットフォームのアーカイブをダウンロードし、バージョンの `SHA256SUMS` で SHA-256 を確認してから、管理者権限なしで 2 つのプログラムをユーザーのフォルダーにコピーします。

```bash
# Linux と macOS: ~/.local/bin にインストール (LOCDEV_INSTALL_DIR で変更可)
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
# Windows: %LOCALAPPDATA%\LocDev\cli にインストールし、ユーザーの Path に追加
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.8.0` でバージョンを固定できます。スクリプトを使わない場合は、アーカイブを展開して `locdev` と `locdevd` を `PATH` の通った同じフォルダーに置きます。

### アプリに同梱の CLI

アプリにも同じ CLI が入っています。`.deb` と `.rpm` はすでに `/usr/bin` に配置します。その他の形式では **設定 → このアプリについて → コマンドライン** に同梱 CLI のバージョンと **locdev をターミナルに追加** ボタンがあり、ユーザーごとの小さなスクリプト (`~/.local/bin/locdev`、Windows では `%LOCALAPPDATA%\LocDev\bin` の `locdev.cmd` とそのフォルダーの `Path` 追加) を作成します。同じボタンで削除でき、LocDev のものではない `locdev` は置き換えません。

## 任意の依存関係

- **Linux の Chrome と Firefox:** これらのブラウザーにローカル CA を信頼させるには `certutil` をインストールします (Debian/Ubuntu ではパッケージ `libnss3-tools`、Fedora では `nss-tools`)。
- **ランタイム:** LocDev はマシンにある PHP、Node、パッケージマネージャーを使い、サイトごとに検証済みのバージョンもインストールします ([ランタイム](/docs/runtimes))。見つかったものは `doctor` が表示します。

## ソースから

安定版の Rust、Node.js 20 以降、pnpm が必要です。Linux でアプリをビルドする場合は WebKitGTK のライブラリも必要です。

```bash
# Linux: デスクトップアプリの依存関係
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

cargo build --release              # target/release に locdev と locdevd
cd apps/desktop && pnpm install
pnpm tauri build                   # target/release/bundle にインストーラー
```

開発中は `pnpm tauri dev` がサイドカーをデバッグモードでビルドし、アプリを開きます。
