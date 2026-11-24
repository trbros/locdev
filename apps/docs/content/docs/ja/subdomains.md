# サブドメインとワイルドカード

各サイトは自分のドメイン (`shop.test`) で応答し、デフォルトでは**すべてのサブドメイン** (`admin.shop.test`、`api.shop.test`、`client1.shop.test`) でも応答します。これが `*.shop.test` ワイルドカードです。プロキシは `Host` ヘッダーをそのまま渡す (加えて `X-Forwarded-Host`) ので、マルチテナントのアプリは名前からテナントを判別できます。

## ルール

ルールはサブドメインを別の場所に送ります。

| 送り先       | 例                     | 用途                                                       |
| ------------ | ---------------------- | ---------------------------------------------------------- |
| 別のポート   | `api → 127.0.0.1:9000` | LocDev の外で動く API                                      |
| 補助プロセス | `ws → reverb`          | サイトの WebSocket サーバー (LocDev が割り当てたポート)    |
| サイト自身   | `admin → site`         | ワイルドカードをオフにして、一部のサブドメインだけ許可する |

パターンはサイトのドメインより前の部分です。`api`、`admin.eu`、または先頭のワイルドカード (`*.tenants` など) で、ワイルドカードは **1 階層以上** に一致します (`acme.tenants`、`a.b.tenants`)。

複数のルールが一致した場合は、最も具体的なものが優先されます。完全一致はワイルドカードより強く、ワイルドカード同士ではサフィックスが長いほうが優先です。`*.tenants`、`vip.tenants`、`*.eu.tenants` があるとき、`vip.tenants.shop.test` は `vip.tenants` に、`acme.eu.tenants.shop.test` は `*.eu.tenants` に送られます。

```bash
locdev subdomain add api shop --port 9000
locdev subdomain add '*.tenants' shop
locdev subdomain add ws shop --process reverb
locdev subdomain list shop
locdev subdomain remove api shop
```

アプリでは **Site → Settings → subdomains** にあります。

## ワイルドカードをオフにする

```bash
locdev subdomain wildcard off shop
```

ワイルドカードをオフにすると、応答するのはサイトのドメインとルールのあるサブドメインだけです。ほかは理由を説明する 404 ページになります。

## HTTPS

各名前の証明書は、接続で要求された名前 (SNI) をもとに、ローカル CA がその場で発行します。`shop.test`、`*.shop.test` をカバーし、深い名前には適切な階層のワイルドカードを使います。`client1.tenants.shop.test` には `*.tenants.shop.test` を含む証明書が発行されます。ワイルドカードは 1 階層しかカバーしないので、`*.shop.test` だけでは足りません。CA が発行するのは `.test` と `.localhost` だけ (名前制約) で、CA を信頼するには引き続きあなたの同意が必要です (`locdev tls trust`)。

WebSocket はすべてのサブドメインで、HTTP でも HTTPS でもプロキシを通ります。

## 名前をテストする

```bash
locdev route api.shop.test
```

```text
api.shop.test
  ok   dns    127.0.0.1 through LocDev's DNS server
  ok   route  shop: port 9000 (rule api)
  ok   https  served on :443, certificate for shop.test, *.shop.test, …
```

各行は別々の段階です。
- **dns:** 名前がこのマシンに届くか。応答したのは LocDev の DNS か。
- **route:** プロキシが名前をどこに送るか、そのポートで今何かが応答しているか。
- **https:** 証明書がどの名前をカバーするか、CA が信頼されているか。

DNS かルートが失敗すると、コマンドはエラーで終了します。アプリでは、同じテストがサブドメインのセクションの最後にあります。
