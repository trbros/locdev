# Subdomains and wildcards

Every site answers on its own domain (`shop.test`) and, by default, on **any subdomain**: `admin.shop.test`, `api.shop.test`, `client1.shop.test`. That's the `*.shop.test` wildcard. The proxy passes the `Host` header through untouched (plus `X-Forwarded-Host`), so a multi-tenant app finds the tenant by name.

## Rules

A rule sends a subdomain somewhere else:

| Destination          | Example                | Use                                                     |
| -------------------- | ---------------------- | ------------------------------------------------------- |
| another port         | `api → 127.0.0.1:9000` | an API that runs outside LocDev                         |
| an auxiliary process | `ws → reverb`          | the site's WebSocket server, on the port LocDev gave it |
| the site itself      | `admin → site`         | with the wildcard off, allow only some subdomains       |

The pattern is the part before the site's domain: `api`, `admin.eu` or a leading wildcard such as `*.tenants`, which matches **one or more levels** (`acme.tenants`, `a.b.tenants`).

When several rules match, the most specific wins: an exact rule beats a wildcard, and among wildcards the longest suffix wins. With `*.tenants`, `vip.tenants` and `*.eu.tenants`, the name `vip.tenants.shop.test` goes to `vip.tenants`, and `acme.eu.tenants.shop.test` goes to `*.eu.tenants`.

```bash
locdev subdomain add api shop --port 9000
locdev subdomain add '*.tenants' shop
locdev subdomain add ws shop --process reverb
locdev subdomain list shop
locdev subdomain remove api shop
```

In the app, the section is in **Site → Settings → subdomains**.

## Turning the wildcard off

```bash
locdev subdomain wildcard off shop
```

With the wildcard off, only the site's domain and the subdomains with a rule answer. Others get a 404 page that explains why.

## HTTPS

Each name's certificate is issued on the fly by the local CA, from the name requested in the connection (SNI). It covers `shop.test`, `*.shop.test` and, for deeper names, the wildcard at the right level: `client1.tenants.shop.test` gets a certificate with `*.tenants.shop.test`. A wildcard only covers one level, which is why `*.shop.test` alone wouldn't do. The CA only issues for `.test` and `.localhost` (name constraints), and trusting it still needs your consent (`locdev tls trust`).

WebSockets go through the proxy on every subdomain, over HTTP and HTTPS.

## Testing a name

```bash
locdev route api.shop.test
```

```text
api.shop.test
  ok   dns    127.0.0.1 through LocDev's DNS server
  ok   route  shop: port 9000 (rule api)
  ok   https  served on :443, certificate for shop.test, *.shop.test, …
```

Each line is a separate step:
- **dns:** does the name reach this machine? And was it LocDev's DNS that answered?
- **route:** where the proxy sends the name, and whether something answers on that port right now.
- **https:** which names the certificate covers, and whether the CA is trusted.

The command exits with an error when DNS or the route fails. In the app, the same test is at the end of the subdomains section.
