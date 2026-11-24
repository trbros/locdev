# Mailbox

O LocDev tem a própria caixa de entrada de testes: um servidor SMTP que **só captura**. Ele escuta em `127.0.0.1:1025` (ou na próxima porta livre, se o Mailpit já estiver nela) e guarda tudo o que recebe, sem entregar nada a ninguém. Não há ferramenta extra para instalar.

![Tools → Mailbox: caixas por site, o HTML isolado e a aba de debug.](screenshot:mailbox)

```bash
locdev mail enable loja          # o e-mail da loja vai para o Mailbox
locdev mail list                 # as mensagens, da mais nova para a mais antiga
locdev mail show <id>            # envelope, texto, links, anexos e problemas
locdev mail raw <id> -o msg.eml  # a mensagem exatamente como chegou
locdev mail test                 # manda uma mensagem de teste para o Mailbox
```

No app: **Ferramentas → Mailbox**. Para ligar um site, use **Site → Settings → mail**.

## Como cada framework é ligado

O LocDev não intercepta tráfego. Ele configura o transporte SMTP de forma explícita, **só no ambiente dos processos do site** (servidor, workers, scheduler). O `.env` do projeto nunca é editado. Depois de ligar ou desligar, reinicie o site.

| Stack                           | Como                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel                         | `MAIL_MAILER=smtp`, `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME` (o nome do site), `MAIL_PASSWORD`, além de `MAIL_SCHEME` (Laravel 11+) ou `MAIL_ENCRYPTION=null` (10 e anteriores), conforme o que o `config/mail.php` do projeto lê. Um `MAIL_URL` existente é anulado, porque ele vence o `MAIL_HOST`.                                                                                |
| WordPress e PHP                 | O `mail()` do PHP passa pelo `sendmail_path`, que vem de um `.ini` do LocDev adicionado com `PHP_INI_SCAN_DIR` (os `.ini` do sistema continuam valendo). No Windows, o `.ini` usa `SMTP`/`smtp_port`. Plugins de SMTP (WP Mail SMTP e afins) não passam pelo `mail()` e precisam apontar para `127.0.0.1` também.                                                                     |
| Node.js, Django, Rails e outros | Se o `.env`/`.env.example` do projeto já tem variáveis `SMTP_*`, `MAIL_*`, `EMAIL_*` ou `MAILER_*` (`HOST`, `PORT`, `USER`, `PASS`, `SECURE`, `URL`…), o LocDev define essas mesmas variáveis. Se o framework lê o SMTP direto do código (`settings.py`, `development.rb`, `nodemailer.createTransport`), a tela mostra o trecho exato para adicionar no ambiente de desenvolvimento. |

O usuário SMTP de cada site é o nome dele, e é assim que o Mailbox sabe de qual projeto veio cada mensagem. O servidor aceita qualquer senha e não oferece STARTTLS (é loopback).

### SMTP real: só com a sua autorização

Se o projeto aponta para um servidor de verdade, ligar o Mailbox **exige consentimento explícito**: `locdev mail enable --override-real`, ou a caixa "Allow replacing it" no app. Isso vale para um host que não é local ou para um transporte de API como SES, Postmark, Mailgun ou Resend. Senhas do `.env` nunca aparecem na tela.

## A caixa de entrada

- **Organização:** caixa geral e uma caixa por site, com busca por assunto, remetente, destinatário ou trecho, e filtros de não lidas e com anexos.
- **Visualizações:** HTML, texto, cabeçalhos, links, anexos (com download) e o `.eml` original.
- **Debug:** mostra o que a sessão SMTP de fato informou:
  - o envelope (`MAIL FROM`, `RCPT TO`) e o `HELO`;
  - o protocolo (`ESMTPA` quando autenticou) e o usuário do `AUTH`;
  - o endereço de origem, o horário de chegada, o `Date` da mensagem, o tamanho e o framework do site.
- **Problemas de formatação:** verificações objetivas.
  - Ausência de `From`, `Date`, `Message-ID` ou `Subject`, e `Date` inválido (RFC 5322).
  - Linhas acima de 998 caracteres e conteúdo 8-bit sem MIME.
  - Erros de decodificação (base64, quoted-printable, charset).
  - HTML sem alternativa em texto e `cid:` sem a parte correspondente.

### HTML isolado

O HTML do e-mail é mostrado num `iframe` com `sandbox` vazio, sem scripts, formulários, navegação nem pop-ups. Além disso, uma política de segurança bloqueia qualquer carregamento da rede, então pixels de rastreamento e imagens remotas não carregam. A tela diz quantos recursos remotos foram bloqueados e a aba Debug lista quais são. Imagens embutidas (`cid:`) aparecem normalmente. Links não abrem pelo clique dentro do e-mail; use a aba Links, que abre o endereço no navegador.

Cada mensagem capturada também vira uma notificação (categoria _E-mails capturados_), mostrada pelo sistema quando o app não está na frente, e acende o ponto verde no ícone da bandeja ([Tour pelo app](/docs/app)). Abrir ou apagar a mensagem marca a notificação como lida.

## Armazenamento

As mensagens ficam em `data/mail/messages` (pasta `0700`), cada uma como `.eml` mais um `.json` com o envelope. São guardadas as últimas 1.000 e, além disso, mensagens com mais de 25 MB são recusadas. Para apagar: `locdev mail delete`, `locdev mail clear [site]` ou os botões do app.

> [!WARNING]
> No Laravel, o LocDev roda o `artisan serve` com `--no-reload`. Sem isso, o `serve` descarta as variáveis que o LocDev injeta antes de iniciar o servidor PHP (os builds de PHP vêm com `variables_order=EGPCS`). Depois de editar o `.env`, reinicie o site.
