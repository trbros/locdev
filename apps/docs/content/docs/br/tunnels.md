# Túneis

Em **Ferramentas → Túneis**, no menu de um site (**Compartilhar na internet…**) ou com `locdev tunnel`, você dá a um site em execução um **endereço público** temporário, para mostrar a alguém ou receber um webhook.

![Túneis: os sites compartilhados, o endereço público e o provedor.](screenshot:tunnels)

## Como funciona

O LocDev não cria uma infraestrutura própria de túneis: ele roda o programa do provedor (`cloudflared`) para um site, sob o supervisor (`@tunnel:<site>`), e lê o endereço público no servidor de métricas local do programa. Só o **servidor daquele site** é compartilhado (`http://127.0.0.1:<porta do site>`, com o domínio do site como `Host`): nunca o proxy, um banco de dados, o painel ou o próprio LocDev. O site precisa estar rodando.

Os túneis terminam quando você para, quando o site é removido e quando o LocDev para. Eles não voltam sozinhos ao reiniciar, porque o endereço muda a cada início.

## Provedor: Cloudflare Tunnel (Quick Tunnel)

|          |                                                                                                                                         |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Conta    | não precisa (nem domínio)                                                                                                               |
| Endereço | aleatório, `https://….trycloudflare.com`, muda a cada início                                                                            |
| Programa | `cloudflared`, Apache-2.0                                                                                                               |
| Limites  | até 200 requisições ao mesmo tempo (acima disso, 429); Server-Sent Events não funcionam; sem garantia de disponibilidade                |
| Uso      | para testes e desenvolvimento; sujeito aos [Termos de Uso dos Serviços Online da Cloudflare](https://www.cloudflare.com/website-terms/) |

Fonte: [documentação dos Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/).

Por que este primeiro: é gratuito, não exige conta nem domínio, o programa é open source e tem builds oficiais para Linux, macOS e Windows. Outros projetos open source (frp, zrok) exigem um servidor próprio ou uma conta no serviço; a arquitetura de provedores do LocDev fica aberta para eles, mas **hoje só o Cloudflare está disponível**.

### Proteção por e-mail

Em **Quem pode abrir**, informe e-mails (`ana@exemplo.com`) ou um domínio inteiro (`*@acme.com`). O visitante digita o e-mail e confirma com um **PIN de uso único** antes de ver o site; não precisa de conta na Cloudflare. Sem e-mails, **qualquer pessoa com o endereço** pode abrir o site, e a tela avisa isso. Para mudar quem tem acesso, pare e compartilhe de novo. Exige `cloudflared` 2026.9.2 ou mais novo.

## Instalar o cloudflared

**Ferramentas → Túneis → Instalar** (ou `locdev tunnel install`) baixa o `cloudflared` da release oficial no GitHub da Cloudflare e só o instala quando **duas fontes de checksum concordam**: o SHA-256 que a Cloudflare lista nas notas da release e o que o GitHub calcula do arquivo publicado. O download é conferido contra esse valor; se algo não bater, nada é instalado. O programa fica em `data/tools/cloudflared/<versão>/`.

Um `cloudflared` que já está no `PATH` (instalado pelo Homebrew, apt, winget…) é usado como está.

> [!NOTE]
> Na release 2026.10.7, o checksum do `cloudflared-darwin-arm64.tgz` nas notas não bate com o arquivo que o GitHub serve. Nesse caso o LocDev recusa a instalação; no macOS, instale com `brew install cloudflared`.

## Compartilhar

Pelo app: **Compartilhar na internet…** no menu do site ou na barra de endereço, ou **Compartilhar um site** em Ferramentas → Túneis. O diálogo mostra o que compartilhar significa (os limites acima) antes de começar, e o endereço quando ele chega, com copiar e abrir.

![Compartilhar na internet: quem pode abrir e os limites do provedor.](screenshot:tunnel-share)

```bash
locdev tunnel providers                               # provedores, limites e o programa
locdev tunnel install cloudflare
locdev tunnel start loja                              # pergunta antes e mostra o endereço
locdev tunnel start loja --allow ana@exemplo.com --allow '*@acme.com'
locdev tunnel list
locdev tunnel logs loja
locdev tunnel stop loja
```

`start` mostra os limites e pergunta antes de expor o site (`--yes` pula a pergunta).

## Limites

- Servidores auxiliares que a página carrega de outra origem (o Vite em modo dev ao lado do Laravel, por exemplo) não passam pelo túnel: o visitante não alcança `*.test`. Use os assets compilados (`npm run build`) ao compartilhar.
- O que o próprio site permite continua valendo: o túnel não adiciona login a um site sem e-mails configurados.
- Validado no Linux com um programa no lugar do `cloudflared` que imita o servidor de métricas (início, endereço, e-mails, parar, remover o site). O `cloudflared` real e o download verificado não foram executados nesta máquina. Windows e macOS não foram testados.
