# Domínios .test (DNS)

## Como funciona

O daemon roda um servidor DNS mínimo em `127.0.0.1:10053`, apenas em loopback. Ele responde `*.test → 127.0.0.1` em qualquer profundidade (`blog.test`, `api.blog.test`, `cliente1.tenants.blog.test`) e recusa qualquer outra zona. Para que o sistema operacional o consulte **somente** para `.test`, o LocDev usa o mecanismo nativo de cada SO:

| SO                       | Mecanismo                     | Onde                                            |
| ------------------------ | ----------------------------- | ----------------------------------------------- |
| Linux (systemd-resolved) | domínio de roteamento `~test` | `/etc/systemd/resolved.conf.d/locdev-test.conf` |
| macOS                    | resolvedor por domínio        | `/etc/resolver/test`                            |
| Windows                  | regra NRPT                    | regra com comentário `Managed by LocDev`        |

Um arquivo `hosts` não serve, porque não tem curinga: cada site e cada subdomínio novo exigiriam outra edição como administrador. O LocDev nunca o edita.

DNS e roteamento são duas coisas diferentes. O DNS faz o nome chegar a esta máquina; o proxy decide para qual site e porta vai cada nome ([Subdomínios](/docs/subdomains)). `locdev route <host>` mostra as duas etapas e o certificado de um nome.

## Ativando

```bash
locdev dns install
```

O comando mostra o script exato e só o executa com a sua confirmação, via `sudo`. No app, o mesmo fica em **Settings → dns**, usando o prompt de autorização do sistema.

![Settings → dns: servidor, resolvedor do sistema e conflitos com outras ferramentas.](screenshot:settings-dns)

![Nada roda antes de você ver o script e autorizar.](screenshot:authorize-dns)

Para desfazer:

```bash
locdev dns uninstall
```

O uninstall só remove o que tem a marca `Managed by LocDev`.

## Sem configurar nada: `.localhost`

Enquanto `*.test` não resolve, as URLs dos sites usam `.localhost`, que Chrome, Firefox, Safari e Edge resolvem para a máquina local por conta própria. O proxy atende os dois sufixos: `blog.test` e `blog.localhost` chegam ao mesmo site.

## Conflitos com outras ferramentas

Se outra ferramenta também reivindica `.test` (por exemplo, um `yerd-test.conf` ou `valet.conf` no systemd-resolved), o LocDev mostra o arquivo como conflito, **mas nunca o edita**. Com dois servidores para o mesmo domínio, as consultas podem ficar lentas ou cair na ferramenta errada; remova o arquivo da outra se não a usa mais.

## Provando que funciona

O `doctor` resolve um nome aleatório (`locdev-probe-<n>.test`) pelo sistema e verifica se ele chegou ao servidor do LocDev. Com isso, ele separa três situações:

- `*.test → LocDev`: tudo certo;
- `*.test → another tool`: resolve, mas por outra ferramenta;
- `not resolving`: ainda não configurado.

> [!NOTE]
> O Vite bloqueia hosts desconhecidos por padrão. Nos processos que o LocDev inicia, ele libera os nomes do site (`blog.test` e `.blog.test`) pela variável `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS`, sem mexer no `vite.config`. Se você roda o Vite por fora do LocDev, adicione `server: { allowedHosts: [".test"] }`. O `.localhost` é sempre permitido.

## Windows

A estratégia no Windows é a NRPT (Name Resolution Policy Table), o roteamento por domínio do próprio sistema:

- **Uma regra para `.test` cobre todos os subdomínios**, em qualquer profundidade. É o que o arquivo `hosts` não consegue, e o motivo por que outras ferramentas sofrem com wildcards no Windows.
- **Só `.test` é afetado**: o resto do DNS da máquina continua como estava, e nenhum adaptador de rede é alterado.
- **A regra não aceita porta**, então o servidor DNS do LocDev escuta na porta 53, que no Windows não exige administrador. Ele tenta `127.0.0.1:53` e, se outro programa já ocupa esse endereço (outro servidor DNS local, por exemplo), passa para um endereço de loopback próprio, `127.53.0.1:53`. A regra aponta para o endereço que ficou de fato escutando.
- Se depois os dois divergirem, o `doctor` avisa e `locdev dns install` reescreve a regra. Regras NRPT de outras ferramentas para `.test` aparecem como conflito e nunca são alteradas.
- **Chrome e Edge** detectam regras NRPT e, por causa delas, deixam de usar o resolvedor próprio e passam pelo do Windows. Isso está no código do Chromium (`net/dns/public/win_dns_system_settings.cc` e `dns_config_service_win.cc`). Se o navegador estiver com "DNS seguro" apontando para um provedor fixo, ou se o Firefox usar DNS sobre HTTPS, adicione `test` às exceções (no Firefox, `network.trr.excluded-domains`).
- **Instalar a regra pede administrador.** O app e a CLI mostram o script do PowerShell (`Add-DnsClientNrptRule`) antes de rodar, e o Windows pede a autorização (UAC).

> [!WARNING]
> O comportamento no Windows foi conferido no código e em testes no Linux (a troca de endereço roda igual). Ainda não foi testado num Windows de verdade.
