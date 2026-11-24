# LocDev とは

LocDev は Linux、macOS、Windows 向けの**オープンソース**のローカル開発環境です。プロジェクトのフォルダーを登録し、開発サーバーを実行して、それぞれを `blog.test` のような専用ドメインで、有効な HTTPS 付きで提供します。

![メイン画面: 左にサイト一覧、右に選択中のサイトとプロセスのライブ出力。](screenshot:sites)

## 構成

LocDev は 3 つの部分でできていて、すべて同じ言葉で通信します。

- **`locdevd`**: デーモン。管理者権限なしでバックグラウンドで動き、すべてを管理します。プロジェクトの登録、プロセス、プロキシ、DNS、証明書です。アプリを閉じてもサイトは動き続けます。
- **`locdev`**: CLI。ターミナルからデーモンを操作し、自動化のための `--json` に対応します。
- **デスクトップアプリ**: CLI と同じ操作を行うインターフェース (Tauri + React)。

インターフェースだけにあるルールはありません。アプリでできることは CLI でもできます。

## できること

- **サイト:** 46 のスタック (Laravel、Next.js、Django、Rails、Vite、Spring、.NET、Go…) の検出、ログ付きで監視される開発サーバー、補助プロセス (キュー、スケジューラー、WebSocket)、LocDev と一緒の起動。[サイト](/docs/sites)と[フレームワーク](/docs/frameworks)を参照してください。
- **ネットワーク:** WebSocket に対応したホスト名ベースの HTTP/HTTPS プロキシ、`*.test` 用の独自 DNS、[サブドメインとワイルドカード](/docs/subdomains)、名前制約付きのローカル CA。
- **ランタイム:** サイトごとの Node.js、PHP、パッケージマネージャーのバージョン。SHA-256 で検証してダウンロードします ([ランタイム](/docs/runtimes))。
- **環境とサービス:** プロセスに注入される[共有変数とシークレット](/docs/environment)、PostgreSQL、MySQL、MariaDB、MongoDB、Redis、Valkey のインスタンス ([サービス](/docs/services))。
- **ツール:** [Database Manager](/docs/database)、[Mailbox](/docs/mailbox)、[Email Studio](/docs/email-studio)、[Config Files](/docs/config-files)、[デプロイアシスタント](/docs/deploy)、[依存関係](/docs/dependencies)、[Git と GitHub](/docs/git)、[GitHub Actions](/docs/actions)。
- **診断:** [Doctor](/docs/doctor) が問題ごとに対処法を示します。

各バージョンの変更点は[変更履歴](/changelog)にあります。

## 原則

- **ペイウォールなし:** ローカル環境と基本機能は無料です。
- **ネイティブ優先:** コンテナを使わず、ネイティブのプロセスで動きます。
- インストール後は**オフラインで動作**します。
- **デフォルトで安全:** 最小限の権限で動き、システムへの変更には必ず明示的な確認を求めます。
