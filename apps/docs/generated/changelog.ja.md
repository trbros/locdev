# 変更履歴

[English](CHANGELOG.md) · [Português (Brasil)](CHANGELOG.pt-BR.md) · [Español](CHANGELOG.es.md) · [日本語](CHANGELOG.ja.md)

LocDev の最初の公開バージョンは 0.19.0 です。デスクトップアプリ 0.19.0 とデーモン・CLI 0.18.0 を含みます。0.1.0 以降の開発版は内部および一部のユーザーによるテストを経て、この初回リリースにまとめています。今後はリリースごとに項目を追加します。

デーモン・CLI とデスクトップアプリのバージョンは別管理です。UI やアプリのシェルだけの変更はアプリ側、デーモンや CLI の変更はそれぞれのバージョンを更新します。異なる場合は見出しに両方を記載します。

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に従います。項目は追加、変更、修正、セキュリティ、互換性のない変更です。インストーラーの提供状況は[公開済みリリース](https://github.com/trbros/locdev/releases)で確認してください。[ドキュメント](https://locdev.app/ja/docs)。

## [0.19.2] - 2026-11-24

アプリ 0.19.2、デーモンと CLI 0.18.2。

### 追加

- インストーラーにロゴから作った専用のアートワークを追加しました。Windows
  (NSIS と MSI) ではダウンロードバッジ付きのセットアップアイコンと、ようこそ
  画面と各ページの画像、macOS の `.dmg` ではアプリケーションへの矢印付きの
  ウィンドウです。

### 修正

- HTTPS で開いた PHP サイト (Laravel、Symfony、WordPress、Bedrock、素の
  PHP) が自分のアセットやフォント、Symfony のツールバーを `http://` で参照し、
  ブラウザが _mixed content_ としてブロックしていました。すべての PHP サイトが
  LocDev の `auto_prepend_file` を読み込むようになり、HTTPS でプロキシに届いた
  リクエストには TLS サーバーが設定するサーバー変数 (`HTTPS`、`REQUEST_SCHEME`、
  `SERVER_PORT`) が渡されます。プロジェクトのファイルは変更しません。
- HTTPS のリクエストで、同じホストの `http://` へのリダイレクトは `https://` を
  指すようになりました。`http://` の URL 設定を持つアプリ (Bedrock の
  `WP_HOME`) でも、ブラウザが HTTP に戻されなくなります。
- Vite 補助付きの Laravel: Vite 8.3 以降ではページが
  `https://127.0.0.1:<port>` からアセットを読み込み、Vite の証明書がその
  アドレスをカバーしていませんでした。証明書に `localhost`、`127.0.0.1`、`::1`
  も含めるようにしました。
- Nuxt 4.1 以降: 開発モードのエラーバッジ (`/__nuxt_dev__/…`) が
  `<site>.test` で 403 を返していました。
- Django: 新しいプロジェクトが `<site>.test` で 400 (`DisallowedHost`) を
  返していました。LocDev は新しいプロジェクトの設定に、`if DEBUG:` の中で
  `ALLOWED_HOSTS` への `.test` とプロキシのスキーム
  (`SECURE_PROXY_SSL_HEADER`) を追加します。既存のプロジェクト向けの注記に、
  追加する内容を記載しています。
- ASP.NET Core: `dotnet run --no-launch-profile` が Production で動いていた
  ため、静的アセットが 500 を返していました。サイトが設定していない場合、
  LocDev は `ASPNETCORE_ENVIRONMENT=Development` を設定します。
- NestJS: プロジェクトの作成が Nest CLI の新しい質問 (`@nestjs/observe`、
  エージェントのスキル) で止まっていました。
- Bedrock: 素の PHP として検出され、`web/` ではなくプロジェクトのルートから
  提供されていました。`web/` から提供される WordPress サイトとして扱うように
  なり、新しいプロジェクトにはサイトの `WP_HOME` を含む `.env` を作成します。

## [0.19.1] - 2026-11-24

アプリ 0.19.1、デーモンと CLI 0.18.1。

### 変更

- サイトの URL は常に `.test` を使います。`*.test` がまだ解決されない間も、
  `<site>.localhost` が代替として使えます。

### 修正

- Windows: 別のプログラム (Acrylic DNS Proxy など) がポート 53 を使用している場合、
  DNS サーバーは管理者権限が必要とは表示せず、そのプログラム名を表示します。
  Doctor はポートを空けるよう提案します。
- Windows: 管理者確認を拒否した場合、変更の成功ではなくキャンセルとして報告します。

## [0.19.0] - 2026-11-24

初回リリース。アプリ 0.19.0、デーモン・CLI 0.18.0。Linux（x86_64、ARM64）、macOS（Apple Silicon、Intel）、Windows 向けに Apache License 2.0 で公開しています。

### 追加

#### 基盤

- デーモン（`locdevd`）に処理を集約し、CLI（`locdev`）とデスクトップアプリ（Tauri）が同じプロトコルを利用します。アプリの操作には対応する CLI コマンドがあり、引数なしの `locdev` で全画面ターミナル UI を開けます（`ctrl+p` でページ、サイト、コマンドにアクセス）。
- Docker や WSL を必須としないネイティブ実行。HTTP、HTTPS、WebSocket のリバースプロキシ、`*.test` 用 DNS（systemd-resolved、`/etc/resolver`、Windows の NRPT）、名前制約付きローカル CA とサイト別・多段サブドメイン用証明書を提供。`locdev doctor` が設定を確認し、ログイン時の自動起動も設定できます。
- システム変更は計画を表示し、確認してから適用します。LocDev がシステムに書き込む設定にはマーカーを付け、アンインストール時にはその設定だけを削除します。

#### サイト

- Laravel、WordPress、Symfony、Next.js、Nuxt、Astro、SvelteKit、Vite、NestJS、Django、Flask、FastAPI、Rails、Go、Rust、.NET など、46 のフレームワーク定義による検出。定義は署名付きで、アプリ本体の新しいリリースなしに更新できます。
- 開発サーバーの監視、ログ、段階的な待機を伴う再起動、CPU・メモリー表示。Laravel の補助 Vite はユーザーの選択後に起動し、キュー、Horizon、scheduler、Reverb などの補助プロセスをフレームワーク別に提案します。
- LocDev 自身のサーバーによる静的サイト（`index.html`）の配信。ループバックのみで待ち受け、クリーン URL、`404.html` に対応し、Python や Node は不要です。
- 監視フォルダー内のプロジェクトをサイトとして登録し、フォルダーの追加・削除に追従します。
- サイト別のサブドメインと、別のポートやプロセスへの転送ルール（`locdev subdomain`、`locdev route`）。
- アドレスバー付きのサイトタブ、⋯ ボタンと右クリックで共通のサイトメニュー、好みのコードエディターへの連携。VS Code、Cursor、Windsurf、VSCodium、Zed、Sublime Text、JetBrains に対応します。
- サイト別リクエストメトリクス（メトリクスタブ、`locdev metrics`）。ステータス分類、p50/p95/p99 レイテンシー、毎分リクエスト数、上位パス、直近のリクエストをプロキシで計測し、メモリー内にのみ保持します。
- Laravel Tinker（Tinker タブ、`locdev tinker`）。サイトの PHP と環境を使用したコード実行と対話コンソール。

#### ランタイム

- サイト別の Node.js と PHP（固定バージョン、既定、システム版）に加え、npm、pnpm、Yarn、Bun、Composer と連携。配布元が公開したチェックサムを検証してからインストールします。Node.js は nodejs.org、PHP は Linux/macOS では StaticPHP、Windows では windows.php.net を使用します。
- apt、Homebrew、nvm、fnm、PATH にある既存バージョンを検出し、システム版は削除しません。既定のバージョンを通常のターミナルでも利用できます（`locdev runtime terminal on`）。
- バージョン別の PHP 設定。メモリー、アップロード、制限時間、OPcache などを変更できます。差分確認、バックアップ、PHP 自身による検証を行い、PHP が拒否した場合は元に戻します。
- 各 PHP が読み込む拡張、拡張フォルダー内の未読み込みファイル、サイトやパッケージの必須・推奨拡張を表示します（`locdev runtime modules`、`locdev runtime extensions`）。

#### 新規プロジェクトとツール

- 利用可能な公式インストーラーを使った新規プロジェクト作成。コマンドを事前表示し、アプリ内ターミナルで実行します。Laravel、Symfony、Bedrock、WordPress、Next.js、Vite、Vue、Angular、Nuxt、Astro、SvelteKit、NestJS、Django、Flask、FastAPI、Rust（Axum、Actix Web）、Rails、ASP.NET Core に対応。任意でデータベースを作成し、サイト登録できます。適切な生成ツールがないスタックには最小限の初期ファイルを用意します。
- Laravel は `laravel new` と同じ順序で、スターターキット、スタック、認証プロバイダー、単一ファイルの Livewire コンポーネント、チーム機能について質問します。
- アプリ内インストーラー。PHP、Composer、Node.js、pnpm は検証付きランタイムとして導入し、Laravel インストーラー、Symfony CLI、Bun、uv、Poetry、rustup、WP-CLI、Git、.NET SDK は各配布元が OS ごとに案内するコマンドを使用します。

#### サービスとデータ

- PostgreSQL、MariaDB、MySQL、MongoDB、Redis、Valkey の管理対象インスタンスに、独立したデータ、ポート、ログ、バックアップを用意。直接ダウンロードする場合は配布元の SHA-256 で検証します。MySQL 8.0/8.4、Redis、追加の PostgreSQL/MongoDB などは、アプリから Homebrew、apt、winget を使って導入できます。
- Database Manager（`locdev db`）。テーブル、行、破壊的な文の検査付き SQL と Redis/Valkey キーブラウザー。認証情報はデーモン内に保持します。
- 共有環境変数とシークレット。暗号化された保管庫を備え、`.env` を編集せずにプロセスへ注入します。
- Mailbox。ループバック上の受信専用 SMTP サーバーと、フレームワーク別の連携。
- Email Studio。テンプレート、Mailable、コンポーネントを視覚的に編集し、管理対象ブロック以外の Blade をバイト単位で保持します。プレビューと履歴も利用できます。
- Config Files。既知の設定項目用フォーム、プロジェクトのコードを実行しない JSON・JS/TS 編集、履歴と解析。

#### コード・CI・デプロイ

- アプリ内 Git。変更、コミット、ブランチ、worktree（各ブランチを独立したサイトとして登録）、stash、リモートを操作できます。破壊的操作は失われる内容を表示し、確認フレーズと任意のパスワードを求め、事前にバックアップします。
- GitHub。OS の keyring に保管するアカウント情報、issue、pull request、ワークフロー実行との連携。
- GitHub Actions エディター。スタック別テンプレート、視覚的編集と YAML 編集、セキュリティ・推奨事項の検査と修正提案。アプリから actionlint を導入でき、各ワークフローのローカル履歴を保持します。
- 依存関係の更新、互換性、OSV による脆弱性確認。更新計画はユーザーが確認してから実行します。
- デプロイアシスタント。適合性によるプラットフォームの並べ替え、準備状態の確認、`render.yaml`/`netlify.toml` の生成と、公開 API 経由の Vercel/Netlify デプロイ、ロールバック、ログ取得。
- トンネル。Cloudflare Quick Tunnel で実行中のサイトを共有し、任意でメール認証による保護を設定できます。

#### デバッグとアプリ

- Debug Center。例外、エラーログ、HTTP 5xx 応答、プロセスの失敗、PHP の `dump()`/`dd()` を問題ごとにまとめ、考えられる原因を表示。認識できるシークレットはイベントの保存前に伏せ字にします。
- バックグラウンドジョブと、アプリ・OS・トレイアイコンの通知。オフラインモードと自動化ごとの設定。
- ブラジルポルトガル語、英語、スペイン語、日本語のアプリ。ライト・ダークテーマ、独自のタイトルバーとスクロールバー、編集可能なショートカット、各画面の WCAG 2.2 AA アクセシビリティ検証。
- GitHub Releases による更新確認と、ユーザーの選択によるインストール。LocDev の署名鍵で検証します。単独 CLI の `install.sh`/`install.ps1` は `SHA256SUMS` を確認します。
- 4 言語のドキュメントサイト。チュートリアル、コードから生成した CLI リファレンス、リポジトリと照合した他ツールとの比較を提供します。
