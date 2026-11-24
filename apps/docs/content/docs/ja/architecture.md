# アーキテクチャ

```text
          ┌────────────┐     ┌──────────────┐
          │ app (Tauri)│     │  locdev CLI  │
          └─────┬──────┘     └──────┬───────┘
                │ ローカル IPC (JSON) │
                └─────────┬──────────┘
                    ┌─────▼──────┐
                    │  locdevd   │  権限なし
                    └─┬───┬───┬──┘
        ┌─────────────┘   │   └──────────────┐
  ┌─────▼──────┐   ┌──────▼──────┐   ┌───────▼──────┐
  │ supervisor │   │ HTTP/S proxy│   │  DNS *.test  │
  │ processes  │   │ SNI + CA    │   │ 127.0.0.1    │
  └────────────┘   └─────────────┘   └──────────────┘
```

## クレート

| クレート            | 役割                                                                                                                    |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `locdev-core`       | モデル、エラー、パス、プロジェクトの検出、ホストの解析、IPC プロトコル。                                                |
| `locdev-ipc`        | ローカルの通信、クライアント、デーモンのランチャー (CLI とアプリで共有)。                                               |
| `locdev-daemon`     | `locdevd`: 登録、スーパーバイザー、プロキシ、DNS、TLS、doctor。                                                         |
| `locdev-proxy`      | ホスト名によるリバースプロキシ (WebSocket と HTTPS 対応)。                                                              |
| `locdev-dns`        | 手書きの最小限の権威 DNS サーバー。                                                                                     |
| `locdev-tls`        | ローカル CA、証明書の発行、信頼のチェック。                                                                             |
| `locdev-platform`   | OS との連携: リゾルバー、トラストストア、ログイン項目、権限昇格。                                                       |
| `locdev-runtimes`   | Node.js、PHP、パッケージマネージャーのバージョン: カタログ、検証付きダウンロード、安全な展開、検出、shim。              |
| `locdev-frameworks` | フレームワークの定義 (署名付きデータ) と検出エンジン。                                                                  |
| `locdev-env`        | 共有変数: `.env` の読み取り、フレームワークごとのルール、解決、同期、暗号化された保管庫。                               |
| `locdev-services`   | データベースとキャッシュ: インストール、インスタンス、コールドバックアップ。                                            |
| `locdev-db`         | Database Manager: PostgreSQL、MySQL/MariaDB、SQLite、Redis/Valkey のドライバー。                                        |
| `locdev-mail`       | Mailbox: キャプチャ専用の SMTP、メッセージの保存と分析。                                                                |
| `locdev-email`      | Email Studio: コメントで囲んだブロック、検出、プレビュー、ジェネレーター。                                              |
| `locdev-config`     | Config Files: バージョンごとのオプションカタログ、コードを実行しない JSON と JS/TS の編集、ルール。                     |
| `locdev-deploy`     | デプロイアシスタント: スタックの概要、情報源付きのプラットフォーム、準備状況、生成ファイル。                            |
| `locdev-deps`       | 依存関係: ローカルの読み取り、npm/Packagist レジストリ、OSV、更新コマンド。                                             |
| `locdev-git`        | シェルを使わないシステムの `git`: status、diff、commit、ブランチ、stash、リモート、reset/revert/restore、バックアップ。 |
| `locdev-github`     | キーリングのトークン、API クライアント (`api.github.com` のみ)、device flow。                                           |
| `locdev-actions`    | GitHub Actions ビルダー: テンプレート、YAML ↔ モデル、検証、安全な書き込み。                                            |
| `apps/locdev`       | CLI。                                                                                                                   |
| `apps/desktop`      | Tauri + React のアプリ (デーモンの薄いクライアント)。                                                                   |

Rust のコードはすべて `unsafe_code = "forbid"` でコンパイルされます。

## IPC プロトコル

メッセージは 4 バイトの長さを先頭に付けた JSON で、Unix ソケット (検証済みの `0700` ディレクトリ内の `0600`) またはリモートのクライアントを拒否するユーザーごとの名前付きパイプでやり取りします。各メッセージにはプロトコルのバージョンが入ります。

```json
{ "protocol": 2, "body": { "type": "start_project", "name": "blog" } }
```

互換性のないバージョンは、デーモンを落とさずに明確なエラー (`INCOMPATIBLE_PROTOCOL`) になります。アプリと CLI はまったく同じリクエストを送ります。

## 権限が必要な操作

デーモンが管理者として動くことはありません。権限が必要なもの (リゾルバー、トラストストア) があると、`locdev-platform` が**計画**を作ります。読める形のスクリプトで、管理者としての手順と、必要ならユーザーとしての手順があります。CLI とアプリは計画を表示し、確認したあとで OS 自身のダイアログ (`sudo`、polkit、macOS のダイアログ、UAC) を通して実行します。アプリは Rust 側で計画を計算し直すので、実行されるのは表示されたものです。
