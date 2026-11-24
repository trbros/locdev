# トンネル

**ツール → トンネル**、サイトのメニュー (**インターネットで共有…**)、または `locdev tunnel` で、実行中のサイトに一時的な**公開アドレス**を付けられます。誰かに見せたり、Webhook を受け取ったりするのに使います。

![トンネル: 共有中のサイト、公開アドレス、プロバイダー。](screenshot:tunnels)

## しくみ

LocDev は独自のトンネル基盤を持ちません。プロバイダーのプログラム (`cloudflared`) を 1 つのサイトのためにスーパーバイザー (`@tunnel:<サイト>`) の下で動かし、プログラムのローカルのメトリクスサーバーから公開アドレスを読み取ります。共有されるのは**そのサイトのサーバーだけ** (`http://127.0.0.1:<サイトのポート>`、`Host` はサイトのドメイン) で、プロキシ、データベース、ダッシュボード、LocDev 自体は共有しません。サイトが実行中である必要があります。

トンネルは、停止したとき、サイトを外したとき、LocDev が止まったときに終わります。アドレスは起動のたびに変わるため、再起動後に自動で戻ることはありません。

## プロバイダー: Cloudflare Tunnel (Quick Tunnel)

|            |                                                                                                                  |
| ---------- | ---------------------------------------------------------------------------------------------------------------- |
| アカウント | 不要 (ドメインも不要)                                                                                            |
| アドレス   | ランダムな `https://….trycloudflare.com`。起動のたびに変わる                                                     |
| プログラム | `cloudflared`、Apache-2.0                                                                                        |
| 制限       | 同時リクエストは最大 200 (超えると 429)。Server-Sent Events は使えない。稼働の保証なし                           |
| 用途       | テストと開発用。[Cloudflare のオンラインサービス利用規約](https://www.cloudflare.com/website-terms/)が適用される |

出典: [Quick Tunnels のドキュメント](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/)。

最初にこれを選んだ理由: 無料で、アカウントもドメインも不要、プログラムはオープンソースで、Linux、macOS、Windows の公式ビルドがあります。ほかのオープンソースプロジェクト (frp、zrok) は自前のサーバーかサービスのアカウントが必要です。LocDev のプロバイダーの仕組みはそれらにも対応できますが、**現時点で使えるのは Cloudflare だけです**。

### メールによる保護

**開ける人** にメールアドレス (`ana@example.com`) かドメイン全体 (`*@acme.com`) を入力します。訪問者はメールアドレスを入力し、**ワンタイム PIN** で確認してからサイトを見られます。Cloudflare のアカウントは不要です。メールアドレスを指定しないと、**アドレスを知っていれば誰でも**サイトを開けます。画面にもそう表示されます。アクセスできる人を変えるには、停止してから共有し直してください。`cloudflared` 2026.9.2 以降が必要です。

## cloudflared のインストール

**ツール → トンネル → インストール** (または `locdev tunnel install`) は、Cloudflare の公式 GitHub リリースから `cloudflared` をダウンロードし、**2 つのチェックサムが一致する**場合だけインストールします。Cloudflare がリリースノートに載せている SHA-256 と、公開ファイルについて GitHub が計算したものです。ダウンロードはその値で検証し、一致しなければ何もインストールしません。プログラムは `data/tools/cloudflared/<バージョン>/` に置かれます。

すでに `PATH` にある `cloudflared` (Homebrew、apt、winget などでインストールしたもの) はそのまま使います。

> [!NOTE]
> リリース 2026.10.7 では、リリースノートにある `cloudflared-darwin-arm64.tgz` のチェックサムが、GitHub が配信するファイルと一致しません。その場合 LocDev はインストールを拒否します。macOS では `brew install cloudflared` でインストールしてください。

## 共有する

アプリでは、サイトのメニューかアドレスバーの **インターネットで共有…**、または ツール → トンネル の **サイトを共有** を使います。開始前に共有の意味 (上記の制限) を表示し、アドレスが届いたらコピーと表示ができます。

![インターネットで共有: 開ける人とプロバイダーの制限。](screenshot:tunnel-share)

```bash
locdev tunnel providers                               # プロバイダー、制限、プログラム
locdev tunnel install cloudflare
locdev tunnel start shop                              # 確認してからアドレスを表示
locdev tunnel start shop --allow ana@example.com --allow '*@acme.com'
locdev tunnel list
locdev tunnel logs shop
locdev tunnel stop shop
```

`start` は制限を表示し、サイトを公開する前に確認を求めます (`--yes` で確認を省略)。

## 制限

- ページが別のオリジンから読み込む補助サーバー (たとえば Laravel と一緒に動く開発モードの Vite) はトンネルを通りません。訪問者は `*.test` に届きません。共有するときはビルド済みのアセット (`npm run build`) を使ってください。
- サイト自体の許可設定はそのまま適用されます。メールアドレスなしで共有したサイトに、トンネルがログインを追加することはありません。
- Linux で、メトリクスサーバーを模した `cloudflared` の代役プログラムを使って検証済みです (起動、アドレス、メールアドレス、停止、サイトの削除)。本物の `cloudflared` と検証付きダウンロードはこのマシンでは実行していません。Windows と macOS はテストしていません。
