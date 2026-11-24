# Início rápido

## 1. Suba o daemon

O app faz isso ao abrir. Pelo terminal:

```bash
locdev start
locdev status
```

```text
● daemon running  pid 41628 · up 3s
  version   0.6.0 (protocol 2)
  projects  0 registered, 0 running
  proxy     127.0.0.1:80 · *.localhost
  dns       127.0.0.1:10053 · *.test not resolving
  https     127.0.0.1:443 · CA not trusted (locdev tls trust)
```

## 2. Registre um projeto

Dentro da pasta do projeto:

```bash
locdev add . --start
```

O LocDev detecta o tipo do projeto, sugere o comando do servidor de desenvolvimento, escolhe uma porta livre (a partir da 8100) e já inicia o site. No app, use `Ctrl` `N` ou o botão **+**.

![Adicionar site: só a pasta é obrigatória.](screenshot:add-site)

## 3. Abra no navegador

Logo depois da instalação, o site já responde em `http://nome.localhost`. Navegadores resolvem `*.localhost` sozinhos, sem configurar nada.

Para usar `nome.test` com HTTPS sem avisos, faça duas configurações, uma vez só:

```bash
locdev dns install   # *.test passa a resolver para o LocDev
locdev tls trust     # o sistema e os navegadores confiam na CA local
```

Os dois comandos mostram exatamente o que será executado e pedem confirmação antes. Veja [Proxy e DNS](/docs/domains) e [HTTPS](/docs/https).

## 4. Confira a saúde do ambiente

```bash
locdev doctor
```

O `doctor` aponta portas ocupadas, resolvedor não configurado, CA não confiável e ferramentas ausentes, cada item com o comando que resolve.

## Próximos passos

Siga um dos tutoriais, do começo ao fim:

- [Um site Laravel do zero](/docs/tutorial-laravel): PHP baixado pelo LocDev, projeto novo, HTTPS e Mailbox.
- [Um projeto Node.js que você já tem](/docs/tutorial-node): a pasta, a versão de Node.js e um túnel para mostrar.
- [Versões por projeto e no terminal](/docs/tutorial-versions): instalar, fixar e usar a padrão no terminal.
