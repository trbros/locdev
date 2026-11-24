# Tutorial: um site Laravel do zero

Neste tutorial você cria um projeto Laravel novo, com a versão de PHP que escolher, e termina com o site em `https://blog.test`, banco SQLite e os e-mails do projeto caindo no Mailbox. Não precisa ter PHP nem Composer instalados: o LocDev baixa os dois das fontes oficiais.

Tempo: uns 5 minutos, quase tudo download.

## 1. Instale o PHP

No app: **Runtimes → PHP → Instalar versão…** e escolha a linha (8.4, por exemplo). Pelo terminal:

```bash
locdev runtime install php 8.4
locdev runtime install composer latest
```

O LocDev baixa o PHP pronto do StaticPHP (Linux/macOS) ou do windows.php.net (Windows) e só instala se o SHA-256 bater com o publicado. Veja [Runtimes](/docs/runtimes).

![O catálogo do PHP: a versão mais nova de cada linha.](screenshot:runtimes-install)

## 2. Crie o projeto

No app: **Ferramentas → Novo projeto → Laravel**. Escolha o nome, o kit (nenhum, React, Vue, Svelte ou Livewire) e o banco; o plano mostra cada comando antes de rodar.

![Novo projeto: o formulário e o plano, com os comandos exatos.](screenshot:new-project)

Pelo terminal, o mesmo em uma linha:

```bash
locdev new project laravel blog --method composer --php 8.4 --in ~/Code -y
```

`--php 8.4` cria o projeto com esse PHP e fixa o site nele. Os passos rodam num terminal dentro do app, e no fim o LocDev registra o site.

![Os passos rodando no terminal do app.](screenshot:new-project-run)

## 3. Suba o site

```bash
locdev up blog
locdev open blog
```

O Laravel roda com `php artisan serve` na porta do site, e o proxy entrega em `http://blog.localhost` na hora. O banco padrão do Laravel é SQLite, já migrado pela criação do projeto.

## 4. `.test` e HTTPS, uma vez só

```bash
locdev dns install
locdev tls trust
```

Cada comando mostra o script que vai rodar e pede confirmação. Depois disso, `https://blog.test` abre sem aviso no navegador. Detalhes em [Proxy e DNS](/docs/domains) e [HTTPS](/docs/https).

## 5. E-mails no Mailbox

O Mailbox captura o que o site envia, sem entregar nada:

```bash
locdev mail setup blog     # o que muda: MAIL_MAILER=smtp, host e porta do LocDev
locdev mail enable blog
locdev reload blog
```

As variáveis vão só para o ambiente dos processos do site; o `.env` do projeto não é alterado. Workers de fila e o scheduler recebem as mesmas variáveis. Envie um e-mail pelo Laravel e abra **Ferramentas → Mailbox** (ou `locdev mail list`).

![Mailbox: as mensagens que os sites enviaram, com HTML, texto e problemas.](screenshot:mailbox)

## 6. Confira as extensões

```bash
locdev runtime extensions blog
```

O LocDev compara o que o projeto pede (`composer.json`, o banco do `.env`, os requisitos do Laravel) com o que o PHP do site carrega, e diz como resolver o que faltar.

## Próximos passos

- [Debug Center](/docs/debug-center): exceções, `dump()` e erros do log do Laravel numa linha do tempo.
- [Serviços](/docs/services): PostgreSQL ou Redis locais no lugar do SQLite.
- [Servidores de dev](/docs/servers): o Vite ao lado do Laravel para os assets.
