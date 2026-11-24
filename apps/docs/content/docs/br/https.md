# HTTPS e CA local

## O que acontece por baixo

Na primeira execução, o daemon cria uma **autoridade certificadora (CA) local** em `<dados>/tls/`, com a chave em `0600`, e passa a servir HTTPS em `127.0.0.1:443` (ou 7443, se a 443 estiver ocupada).

O certificado de cada site é emitido em memória no primeiro acesso, a partir do nome pedido no handshake (SNI), e cobre:

- `blog.test` e `*.blog.test`
- `blog.localhost` e `*.blog.localhost`

Os certificados valem 397 dias e são reemitidos a cada reinício do daemon.

## Links e assets em HTTPS (_mixed content_)

O proxy encerra o TLS e fala HTTP simples com o servidor de desenvolvimento do site, informando o protocolo e a porta originais em `X-Forwarded-Proto` e `X-Forwarded-Port` (além de `X-Forwarded-Host` e `X-Forwarded-For`). Os frameworks só leem esses cabeçalhos de proxies em que foram configurados para confiar (o `TrustProxies` do Laravel, o `trusted_proxies` do Symfony). Sem isso, uma página aberta em `https://` gera links `http://` para os próprios assets, e o navegador os bloqueia como _mixed content_. O LocDev resolve isso em todos os projetos, sem alterar arquivos deles:

- **PHP (Laravel, Symfony, WordPress, Bedrock, PHP puro):** todo site PHP carrega o `auto_prepend_file` do LocDev (em `<dados>/php-prepend/`, marcado "Managed by LocDev" e adicionado pelo `PHP_INI_SCAN_DIR`). Numa requisição que chegou ao proxy por HTTPS, ele define `$_SERVER['HTTPS'] = 'on'`, `REQUEST_SCHEME` e `SERVER_PORT` como um servidor TLS faria. Com isso, `asset()` e `url()`, a toolbar do Symfony (`/_wdt`) e o `is_ssl()` do WordPress passam a gerar `https://`. Ele só confia nesses cabeçalhos quando a conexão vem do loopback, que é de onde o proxy fala, e o proxy sempre os sobrescreve. Se o seu PHP já tinha um `auto_prepend_file`, ele continua rodando logo depois.
- **Redirecionamentos (todas as stacks):** numa requisição HTTPS, uma resposta com `Location: http://<o mesmo host>/…` passa a apontar para `https://`. Isso cobre apps que montam a URL a partir de uma configuração em `http://` (como o `WP_HOME` do Bedrock) e evita que um site HTTPS-only fique alternando entre os dois protocolos. Outros hosts e endereços relativos não mudam.
- **Vite ao lado do Laravel:** o certificado entregue ao Vite cobre também `localhost`, `127.0.0.1` e `::1` (veja **Servidores de desenvolvimento**).

## Por que confiar nessa CA é de baixo risco

A CA é emitida com **Name Constraints** (`permitted: test, localhost`) e `pathlen:0`. Navegadores e bibliotecas TLS rejeitam qualquer certificado assinado por ela para outro domínio. Mesmo que a chave vazasse, ela não serviria para interceptar `google.com` ou o seu banco.

Isso tem teste automatizado: um certificado `example.com` assinado pela própria CA é recusado pelo verificador do rustls com erro de _NameConstraint_.

```bash
openssl x509 -in ~/.local/share/locdev/tls/ca.pem -noout -ext nameConstraints
# X509v3 Name Constraints: critical
#     Permitted:
#       DNS:test
#       DNS:localhost
```

## Confiando na CA

Confiar é **opcional e explícito**:

```bash
locdev tls trust
```

O plano tem duas etapas, ambas mostradas antes de rodar:

| SO               | Como administrador        | Como você                                |
| ---------------- | ------------------------- | ---------------------------------------- |
| Debian/Ubuntu    | `update-ca-certificates`  | NSS do Chrome e do Firefox (`certutil`)  |
| Fedora/RHEL/Arch | `update-ca-trust`         | NSS do Chrome e do Firefox (`certutil`)  |
| macOS            | System keychain           | (os navegadores usam o keychain)         |
| Windows          | `Cert:\LocalMachine\Root` | (os navegadores usam o store do Windows) |

![Settings → https: estado no sistema e nos navegadores e o fingerprint SHA-256 da CA.](screenshot:settings-https)

![Plano de confiança da CA com as duas etapas](screenshot:authorize-tls)

Depois de confiar, reinicie o navegador. As URLs dos sites passam a usar `https://` automaticamente.

```bash
locdev tls status    # porta, caminho e fingerprint da CA, estado da confiança
locdev tls untrust   # remove do sistema e dos navegadores
```

> [!NOTE]
> No Linux, Chrome e Firefox mantêm o próprio banco de certificados (NSS). O LocDev encontra os bancos do Chrome/Chromium e dos perfis do Firefox, inclusive nas versões snap e flatpak. Para atualizá-los, é preciso o `certutil` (`libnss3-tools`).

## Conferindo o fingerprint

O fingerprint SHA-256 aparece em `locdev tls status` e no app. Compare-o com o emissor mostrado pelo navegador ao inspecionar o certificado de um site `.test`.

## `locdev secure`

`locdev secure [site]` deixa um site só em HTTPS e verifica tudo o que isso exige: o listener HTTPS, o certificado do site, a resolução de `*.test` e a confiança na CA. O que precisa de administrador (confiar na CA, rotear `*.test`) usa os mesmos planos de `locdev tls trust` e `locdev dns install`: o script é mostrado e só roda depois da sua confirmação (ou com `--yes`). Nada é instalado duas vezes, e não há uma segunda implementação de TLS: é a mesma CA local.

```bash
cd ~/Code/shop
locdev secure          # verifica, corrige com confirmação, e ativa HTTPS-only
locdev secure --off    # volta a servir HTTP também
```

Com HTTPS-only, uma requisição `http://shop.test` recebe um redirecionamento `307` para `https://shop.test` (o `307` mantém o método e o corpo, então um POST continua POST). No app, o mesmo fica em **Configurações** do site → **HTTPS**, com o diagnóstico por site.
