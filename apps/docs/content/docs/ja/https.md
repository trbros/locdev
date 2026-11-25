# HTTPS とローカル CA

## 内部で起きていること

初回起動時に、デーモンは `<data>/tls/` に**ローカル認証局 (CA)** を作成し (鍵は `0600`)、`127.0.0.1:443` (443 が使用中なら 7443) で HTTPS の提供を始めます。

各サイトの証明書は、最初のアクセス時にハンドシェイクで要求された名前 (SNI) をもとにメモリ上で発行され、次をカバーします。

- `blog.test` と `*.blog.test`
- `blog.localhost` と `*.blog.localhost`

証明書の有効期間は 397 日で、デーモンを再起動するたびに再発行されます。

## HTTPS でのリンクとアセット (_mixed content_)

プロキシは TLS を終端し、サイトの開発サーバーとは平文の HTTP で通信します。元のスキームとポートは `X-Forwarded-Proto` と `X-Forwarded-Port` (ほかに `X-Forwarded-Host` と `X-Forwarded-For`) で伝えます。フレームワークは、信頼するよう設定されたプロキシ (Laravel の `TrustProxies`、Symfony の `trusted_proxies`) からしかこれらのヘッダーを読みません。そのままでは `https://` で開いたページが自分のアセットを `http://` で参照し、ブラウザが _mixed content_ としてブロックします。LocDev はすべてのプロジェクトで、ファイルを変更せずにこれに対処します。

- **PHP (Laravel、Symfony、WordPress、Bedrock、素の PHP):** すべての PHP サイトが LocDev の `auto_prepend_file` (`<data>/php-prepend/` にあり、"Managed by LocDev" の印付き。`PHP_INI_SCAN_DIR` で追加) を読み込みます。HTTPS でプロキシに届いたリクエストでは、TLS サーバーと同じように `$_SERVER['HTTPS'] = 'on'`、`REQUEST_SCHEME`、`SERVER_PORT` を設定します。これで `asset()` や `url()`、Symfony のツールバー (`/_wdt`)、WordPress の `is_ssl()` が `https://` を生成します。これらのヘッダーを信頼するのは、プロキシが接続してくるループバックからの接続だけで、プロキシは常にヘッダーを上書きします。PHP にすでに `auto_prepend_file` があった場合、そのすぐ後に引き続き実行されます。
- **リダイレクト (すべてのスタック):** HTTPS のリクエストで、`Location: http://<同じホスト>/…` を返すレスポンスは `https://` を指すように書き換えられます。`http://` の設定 (Bedrock の `WP_HOME` など) から URL を組み立てるアプリに対応し、HTTPS のみのサイトが 2 つのスキームの間を行き来するのを防ぎます。ほかのホストや相対パスは変わりません。
- **Next.js:** LocDev は開発サーバーを `--hostname localhost` で起動します (`NODE_OPTIONS=--dns-result-order=ipv4first` により `127.0.0.1` で待ち受けたままです)。`127.0.0.1` だと、Next は自身のアドレスをそれで組み立てる一方でミドルウェアは `localhost` と見るため、ミドルウェアのリライト (next-intl のロケール接頭辞) が外部扱いになり、Next が自分自身にプロキシしていました。HTTPS では 500 (`EPROTO … wrong version number`)、接頭辞なしの既定ロケール (`as-needed`) ではリダイレクトのループになります。以前のコマンドで保存されたサイトはデーモン起動時に新しいコマンドへ移行します。自分で書いたコマンドはそのままです。
- **Laravel と並ぶ Vite:** Vite に渡す証明書は `localhost`、`127.0.0.1`、`::1` もカバーします (**開発サーバー** を参照)。

## この CA を信頼してもリスクが小さい理由

CA は **Name Constraints** (`permitted: test, localhost`) と `pathlen:0` 付きで発行されます。ブラウザーと TLS ライブラリは、この CA がほかのドメイン用に署名した証明書をすべて拒否します。万一鍵が漏れても、`google.com` や銀行の通信を傍受することはできません。

これは自動テストで確認しています。CA 自身が署名した `example.com` の証明書は、rustls の検証で _NameConstraint_ エラーとして拒否されます。

```bash
openssl x509 -in ~/.local/share/locdev/tls/ca.pem -noout -ext nameConstraints
# X509v3 Name Constraints: critical
#     Permitted:
#       DNS:test
#       DNS:localhost
```

## CA を信頼する

信頼は**任意で、明示的な操作が必要**です。

```bash
locdev tls trust
```

計画は 2 段階で、どちらも実行前に表示されます。

| OS               | 管理者として              | あなたとして                          |
| ---------------- | ------------------------- | ------------------------------------- |
| Debian/Ubuntu    | `update-ca-certificates`  | Chrome と Firefox の NSS (`certutil`) |
| Fedora/RHEL/Arch | `update-ca-trust`         | Chrome と Firefox の NSS (`certutil`) |
| macOS            | System キーチェーン       | (ブラウザーはキーチェーンを使用)      |
| Windows          | `Cert:\LocalMachine\Root` | (ブラウザーは Windows のストアを使用) |

![Settings → https: システムとブラウザーでの状態、CA の SHA-256 フィンガープリント。](screenshot:settings-https)

![2 段階からなる CA 信頼の計画](screenshot:authorize-tls)

信頼したあとはブラウザーを再起動してください。サイトの URL は自動で `https://` になります。

```bash
locdev tls status    # ポート、CA のパスとフィンガープリント、信頼の状態
locdev tls untrust   # システムとブラウザーから削除
```

> [!NOTE]
> Linux では Chrome と Firefox が独自の証明書データベース (NSS) を持っています。LocDev は snap 版や flatpak 版も含め、Chrome/Chromium のデータベースと Firefox のプロファイルを見つけます。更新には `certutil` (`libnss3-tools`) が必要です。

## フィンガープリントの確認

SHA-256 フィンガープリントは `locdev tls status` とアプリに表示されます。`.test` サイトの証明書をブラウザーで確認したときの発行者と比べてください。

## `locdev secure`

`locdev secure [site]` はサイトを HTTPS 専用にし、そのために必要なものをすべて確認します。HTTPS リスナー、サイトの証明書、`*.test` の解決、CA の信頼です。管理者権限が必要なもの (CA の信頼、`*.test` のルーティング) は `locdev tls trust` と `locdev dns install` と同じ計画を使います。スクリプトが表示され、確認したあと (または `--yes` 指定時) にだけ実行されます。二重にインストールされることはなく、TLS の実装も 1 つだけ、同じローカル CA です。

```bash
cd ~/Code/shop
locdev secure          # 確認し、承認を得て修正し、HTTPS 専用にする
locdev secure --off    # HTTP も再び提供する
```

HTTPS 専用にすると、`http://shop.test` へのリクエストは `https://shop.test` への `307` リダイレクトを受け取ります (`307` はメソッドと本文を保つので、POST は POST のままです)。アプリでは、サイトの **設定** → **HTTPS** に同じ機能があり、サイトごとの診断も表示されます。
