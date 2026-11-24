# .test ドメイン (DNS)

## しくみ

デーモンは `127.0.0.1:10053` で最小限の DNS サーバーを動かします (ループバックのみ)。どの深さでも `*.test → 127.0.0.1` を返し (`blog.test`、`api.blog.test`、`client1.tenants.blog.test`)、それ以外のゾーンは拒否します。OS が `.test` の問い合わせ**だけ**をこのサーバーに送るよう、LocDev は各 OS のネイティブな仕組みを使います。

| OS                       | 仕組み                       | 場所                                            |
| ------------------------ | ---------------------------- | ----------------------------------------------- |
| Linux (systemd-resolved) | ルーティングドメイン `~test` | `/etc/systemd/resolved.conf.d/locdev-test.conf` |
| macOS                    | ドメインごとのリゾルバー     | `/etc/resolver/test`                            |
| Windows                  | NRPT ルール                  | コメント `Managed by LocDev` 付きのルール       |

`hosts` ファイルはワイルドカードが使えないので向きません。サイトやサブドメインを追加するたびに管理者として編集が必要になります。LocDev が `hosts` を編集することはありません。

DNS とルーティングは別のものです。DNS は名前をこのマシンに届け、プロキシは各名前をどのサイトとポートに送るかを決めます ([サブドメイン](/docs/subdomains))。`locdev route <host>` で両方の段階と、その名前の証明書を確認できます。

## 有効にする

```bash
locdev dns install
```

コマンドは実行するスクリプトをそのまま表示し、確認したあとで `sudo` を使って実行します。アプリでは **Settings → dns** にあり、システムの認証ダイアログを使います。

![Settings → dns: サーバー、システムのリゾルバー、ほかのツールとの競合。](screenshot:settings-dns)

![スクリプトを確認して許可するまで、何も実行されません。](screenshot:authorize-dns)

元に戻すには:

```bash
locdev dns uninstall
```

アンインストールで消えるのは `Managed by LocDev` の印があるものだけです。

## 設定なしで使う: `.localhost`

`*.test` が解決できない間は、サイトの URL に `.localhost` が使われます。Chrome、Firefox、Safari、Edge は自分で `.localhost` をローカルマシンに解決します。プロキシは両方のサフィックスに対応しているので、`blog.test` と `blog.localhost` は同じサイトに届きます。

## ほかのツールとの競合

ほかのツールも `.test` を使っている場合 (たとえば systemd-resolved の `yerd-test.conf` や `valet.conf`)、LocDev はそのファイルを競合として表示しますが、**編集はしません**。同じドメインに 2 つのサーバーがあると、問い合わせが遅くなったり、別のツールに届いたりします。使っていないツールなら、そのファイルを削除してください。

## 動作の確認

`doctor` はランダムな名前 (`locdev-probe-<n>.test`) をシステム経由で解決し、LocDev のサーバーに届いたかを確かめます。これで次の 3 つを見分けます。

- `*.test → LocDev`: 正常。
- `*.test → another tool`: 解決はできるが、別のツール経由。
- `not resolving`: まだ設定されていない。

> [!NOTE]
> Vite はデフォルトで未知のホストをブロックします。LocDev が起動するプロセスでは、`vite.config` を変えずに、変数 `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS` でサイトの名前 (`blog.test` と `.blog.test`) を許可します。LocDev の外で Vite を動かす場合は `server: { allowedHosts: [".test"] }` を追加してください。`.localhost` は常に許可されています。

## Windows

Windows では NRPT (Name Resolution Policy Table)、つまりシステム自身のドメイン別ルーティングを使います。

- **`.test` のルール 1 つで、どの深さのサブドメインもカバーします。** `hosts` ファイルにはできないことで、ほかのツールが Windows でワイルドカードに苦労する理由でもあります。
- **影響するのは `.test` だけです。** マシンのほかの DNS はそのままで、ネットワークアダプターも変更しません。
- **ルールにはポートを指定できない**ので、LocDev の DNS サーバーはポート 53 で待ち受けます。Windows ではこれに管理者権限は不要です。まず `127.0.0.1:53` を試し、ほかのプログラム (別のローカル DNS サーバーなど) が使っていれば、専用のループバックアドレス `127.53.0.1:53` に切り替えます。ルールは実際に待ち受けたアドレスを指します。
- あとで両者がずれた場合は `doctor` が警告し、`locdev dns install` がルールを書き直します。ほかのツールの `.test` 用 NRPT ルールは競合として表示し、変更しません。
- **Chrome と Edge** は NRPT ルールを検出すると、独自のリゾルバーをやめて Windows のリゾルバーを使います。これは Chromium のソース (`net/dns/public/win_dns_system_settings.cc` と `dns_config_service_win.cc`) にあります。ブラウザーの「セキュア DNS」が固定のプロバイダーを使っている場合や、Firefox が DNS over HTTPS を使っている場合は、`test` を例外に追加してください (Firefox では `network.trr.excluded-domains`)。
- **ルールのインストールには管理者権限が必要です。** アプリと CLI は実行前に PowerShell のスクリプト (`Add-DnsClientNrptRule`) を表示し、Windows が許可 (UAC) を求めます。

> [!WARNING]
> Windows での動作はコードと Linux 上のテストで確認しました (アドレスの切り替えは同じように動きます)。実際の Windows ではまだテストしていません。
