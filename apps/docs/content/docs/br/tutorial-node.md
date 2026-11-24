# Tutorial: um projeto Node.js que você já tem

Neste tutorial você traz para o LocDev um projeto existente (Vite, Next.js, Nuxt, Astro, Express…), com a versão de Node.js que ele pede, e o abre em `https://<nome>.test`. Nada no projeto é alterado.

## 1. Registre a pasta

No app, `Ctrl` `N` (ou o **+** da lista de sites) e escolha a pasta. Pelo terminal, dentro dela:

```bash
locdev add . --start
```

![Adicionar site: só a pasta é obrigatória.](screenshot:add-site)

O LocDev reconhece o framework pelos arquivos (`package.json`, `vite.config.*`, `next.config.*`…), sem executar nada do projeto, e sugere o comando de dev com a porta do site (`{port}`). Ele também passa `PORT` e `HOST=127.0.0.1` para o processo, e libera o host `.test` no Vite.

## 2. A versão de Node.js do projeto

Se o projeto tem `.nvmrc`, `.node-version`, `.tool-versions`, `engines.node` ou `volta` no `package.json`, o LocDev usa essa versão. Se ela não estiver instalada, o site não sobe e o erro diz o comando que resolve:

```bash
locdev runtime install node 20
locdev reload <nome>
```

Para fixar uma versão sem mexer no projeto, use a aba **Configurações** do site ou:

```bash
locdev edit <nome> --node 22
locdev reload <nome>
```

![As configurações do site: comando, porta, runtimes e início.](screenshot:site-settings)

O gerenciador de pacotes segue o mesmo caminho: `packageManager` no `package.json` (pnpm, Yarn, Bun) ou `locdev edit <nome> --pnpm 10`.

## 3. Acompanhe o servidor

```bash
locdev logs <nome>       # a saída do servidor de dev
locdev info <nome>       # endereços, versões resolvidas e de onde vieram
```

No app, a aba **Servidor** mostra a saída ao vivo, e o [Debug Center](/docs/debug-center) junta os erros de todos os sites.

## 4. Mostre para alguém (opcional)

Para compartilhar o site rodando com quem está fora da sua rede:

```bash
locdev tunnel start <nome>
```

O LocDev usa um Cloudflare Quick Tunnel, sem conta, e mostra o endereço público. O túnel acaba quando você para o site. Veja [Túneis](/docs/tunnels).

## Próximos passos

- [Variáveis compartilhadas](/docs/environment): segredos injetados no processo, sem `.env` no repositório.
- [Subdomínios](/docs/subdomains): `api.<nome>.test` para outro processo ou porta.
- [Dependências](/docs/dependencies): versões, atualizações e vulnerabilidades do projeto.
