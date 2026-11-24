# HTTPS e CA local

## O que acontece por baixo

Na primeira execução, o daemon cria uma **autoridade certificadora (CA) local** em `<dados>/tls/`, com a chave em `0600`, e passa a servir HTTPS em `127.0.0.1:443` (ou 7443, se a 443 estiver ocupada).

O certificado de cada site é emitido em memória no primeiro acesso, a partir do nome pedido no handshake (SNI), e cobre:

- `blog.test` e `*.blog.test`
- `blog.localhost` e `*.blog.localhost`

Os certificados valem 397 dias e são reemitidos a cada reinício do daemon.

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
