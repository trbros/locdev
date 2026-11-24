# Subdomínios e wildcards

Todo site responde no próprio domínio (`loja.test`) e, por padrão, em **qualquer subdomínio**: `admin.loja.test`, `api.loja.test`, `cliente1.loja.test`. Esse é o wildcard `*.loja.test`. O proxy repassa o cabeçalho `Host` intacto (mais `X-Forwarded-Host`), então um app multi-tenant descobre o tenant pelo nome.

## Regras

Uma regra manda um subdomínio para outro lugar:

| Destino              | Exemplo                | Uso                                                              |
| -------------------- | ---------------------- | ---------------------------------------------------------------- |
| outra porta          | `api → 127.0.0.1:9000` | uma API que roda fora do LocDev                                  |
| um processo auxiliar | `ws → reverb`          | o servidor de WebSocket do site, na porta que o LocDev deu a ele |
| o próprio site       | `admin → site`         | com o wildcard desligado, liberar só alguns subdomínios          |

O padrão é a parte antes do domínio do site: `api`, `admin.eu` ou um curinga no começo, como `*.tenants`, que vale para **um ou mais níveis** (`acme.tenants`, `a.b.tenants`).

Quando várias regras casam, ganha a mais específica: a exata vence o curinga, e entre curingas vence o de sufixo mais longo. Com `*.tenants`, `vip.tenants` e `*.eu.tenants`, o nome `vip.tenants.loja.test` vai para `vip.tenants`, e `acme.eu.tenants.loja.test` vai para `*.eu.tenants`.

```bash
locdev subdomain add api loja --port 9000
locdev subdomain add '*.tenants' loja
locdev subdomain add ws loja --process reverb
locdev subdomain list loja
locdev subdomain remove api loja
```

No app, a seção fica em **Site → Settings → subdomains**.

## Desligar o wildcard

```bash
locdev subdomain wildcard off loja
```

Com o wildcard desligado, só o domínio do site e os subdomínios com regra respondem. Os outros recebem uma página 404 explicando o motivo.

## HTTPS

O certificado de cada nome é emitido na hora pela CA local, a partir do nome pedido na conexão (SNI). Ele cobre `loja.test`, `*.loja.test` e, para nomes mais fundos, o curinga do nível certo: `cliente1.tenants.loja.test` recebe um certificado com `*.tenants.loja.test`. Um curinga só vale para um nível, por isso `*.loja.test` sozinho não bastaria. A CA só emite para `.test` e `.localhost` (restrição de nome), e confiar nela continua exigindo o seu consentimento (`locdev tls trust`).

WebSockets passam pelo proxy em todos os subdomínios, em HTTP e HTTPS.

## Testar um nome

```bash
locdev route api.loja.test
```

```text
api.loja.test
  ok   dns    127.0.0.1 through LocDev's DNS server
  ok   route  loja: port 9000 (rule api)
  ok   https  served on :443, certificate for loja.test, *.loja.test, …
```

Cada linha é uma etapa separada:
- **dns:** o nome chega a esta máquina? E foi o DNS do LocDev que respondeu?
- **route:** para onde o proxy manda o nome, e se algo responde naquela porta agora.
- **https:** quais nomes o certificado cobre, e se a CA é confiável.

O comando termina com erro quando o DNS ou a rota falham. No app, o mesmo teste fica no fim da seção de subdomínios.
