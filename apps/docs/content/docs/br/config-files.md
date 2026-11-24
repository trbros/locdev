# Config Files

A aba **Config** de cada site reúne os arquivos de configuração que o projeto já tem (`vite.config.ts`, `next.config.mjs`, `tsconfig.json`, `package.json`, `composer.json`…), com um formulário para as opções que o LocDev entende, edição de texto, histórico e uma análise baseada nas versões instaladas. Na CLI, é `locdev config`.

![A aba Config: análise, arquivos do projeto e o formulário de opções.](screenshot:config-files)

```bash
locdev config list                                   # os arquivos do site
locdev config show vite.config.ts                    # opções como o LocDev as lê
locdev config set vite.config.ts server.port 5180    # mostra o diff e pergunta antes de salvar
locdev config check                                  # análise: nada é alterado
locdev config history vite.config.ts
locdev config revert vite.config.ts <id>
```

## Só o que existe

A lista mostra apenas arquivos presentes no projeto. **Nenhum arquivo é criado para preencher a tela.** O Tailwind CSS 4, que é configurado no CSS (`@theme`), aparece pelo arquivo CSS que importa `tailwindcss`, e não por um `tailwind.config.js` que o projeto não usa.

| Arquivo                                                                                                                                                                                                                                            | Formulário                                          | Pacote que define as versões         |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------ |
| `vite.config.*`                                                                                                                                                                                                                                    | servidor, preview, build                            | `vite`                               |
| `next.config.*`                                                                                                                                                                                                                                    | React, segurança, build, rotas, imagens, TypeScript | `next`                               |
| `nuxt.config.*`, `astro.config.*`, `react-router.config.*`                                                                                                                                                                                         | renderização e servidor                             | `nuxt`, `astro`, `@react-router/dev` |
| `tsconfig.json`, `tsconfig.*.json`, `jsconfig.json`                                                                                                                                                                                                | checagem, módulos, emissão                          | `typescript`                         |
| `package.json`, `composer.json`                                                                                                                                                                                                                    | tipo do módulo, engines, estabilidade, autoloader   | —                                    |
| `biome.json(c)`, `.prettierrc*`                                                                                                                                                                                                                    | formatação                                          | `@biomejs/biome`, `prettier`         |
| `svelte.config.*`, `angular.json`, `nest-cli.json`, `eslint.config.*`, `.eslintrc*`, `.babelrc`, `postcss.config.*`, `tailwind.config.*`, `phpunit.xml`, `vitest.config.*`, `playwright.config.*`, `deno.json(c)`, `wrangler.*`, CSS do Tailwind 4 | só texto (com validação de sintaxe)                 | —                                    |

## Como a edição preserva o arquivo

- **JSON e JSONC** passam por uma árvore de sintaxe concreta: mudar um valor mantém comentários, linhas em branco, indentação e o estilo de vírgula final. `package.json` e `composer.json` continuam JSON estrito.
- **JavaScript e TypeScript** são lidos com um parser (oxc) e **nunca executados**. O LocDev encontra o objeto de configuração em `export default`, `defineConfig(...)`, `module.exports`, numa `const` exportada ou numa função que o retorna, e troca **só o trecho do valor**. Uma opção nova entra no fim do objeto, com a indentação e as aspas do arquivo.
- Valor calculado em código (`process.env.HOST`, uma variável, uma chamada) aparece como código e **não é editado pelo formulário**: o motivo é mostrado e a mudança fica para o editor de texto.
- Toda mudança mostra o **diff antes de salvar**. Se a edição de texto remover comentários, eles são listados.
- O arquivo só é gravado se ainda for o que você revisou (hash). Uma sintaxe inválida não é salva.
- A versão anterior vai para o **histórico** (`data/state/config-history`, até 50 por arquivo), que permite comparar e reverter.
- A gravação é atômica e mantém as permissões do arquivo.

O botão **Mostrar** abre a pasta do arquivo. O app não abre o arquivo com o programa padrão do sistema porque, no Windows, o padrão para `.js` é executar o script.

## Compatibilidade por versão

Cada opção sabe em que versões existe, e a versão vem do que está instalado (`node_modules`, `vendor`), depois do lockfile e por último do range declarado. Uma opção que a versão do projeto não tem aparece desabilitada com o motivo, por exemplo "needs next 15.5.0 or newer (this project has 15.2.4)". O daemon recusa a mudança mesmo que um cliente tente.

## Análise

`locdev config check` (e o quadro **Análise** da aba) aplica regras verificáveis. Cada achado diz a que versões se aplica, comparadas com a instalada, e traz a fonte oficial. Quando há uma correção simples, ela é oferecida para revisão e **nunca é aplicada sozinha**.

| Regra                                    | Quando                                                       |
| ---------------------------------------- | ------------------------------------------------------------ |
| `vite.https-boolean`                     | `server.https: true` com Vite ≥ 5 (removido)                 |
| `vite.host-exposed`                      | `server.host` exposto na rede (o LocDev usa 127.0.0.1)       |
| `next.powered-by`                        | Next.js enviando `X-Powered-By`                              |
| `next.images-domains`                    | `images.domains` com Next.js ≥ 14 (use `remotePatterns`)     |
| `next.experimental-turbo`                | `experimental.turbo` com Next.js ≥ 15.3 (agora `turbopack`)  |
| `next.typed-routes-moved` / `-available` | `typedRoutes` com Next.js ≥ 15.5                             |
| `next.cache-components`                  | `experimental.dynamicIO`/`ppr` com Next.js ≥ 16              |
| `next.ignore-build-errors`               | builds passando com erro de TypeScript                       |
| `tailwind.v4-js-config`                  | `tailwind.config.js` sem `@config` com Tailwind ≥ 4          |
| `tailwind.v4-directives`                 | `@tailwind base` com Tailwind ≥ 4                            |
| `tailwind.v4-postcss-plugin`             | plugin `tailwindcss` no PostCSS com Tailwind ≥ 4             |
| `eslint.eslintrc-ignored`                | `.eslintrc*` sem flat config com ESLint ≥ 9                  |
| `typescript.import-flags`                | `importsNotUsedAsValues`/`preserveValueImports` com TS ≥ 5.0 |
| `typescript.bundler-resolution`          | `moduleResolution: node` em projeto com bundler, TS ≥ 5.0    |
| `composer.unstable`                      | `minimum-stability` abaixo de `stable` sem `prefer-stable`   |
| `package.manager-mismatch`               | `packageManager` diferente do lockfile presente              |

## Limitações

- O formulário cobre opções escalares (texto, número, verdadeiro/falso, escolha). Objetos e listas (plugins, `paths`, `remotePatterns`) se editam no texto; os plugins do Vite aparecem como leitura.
- O editor de texto tem realce de sintaxe para cada tipo de arquivo (JSON, JS/TS, CSS, XML…), busca e desfazer, e valida a sintaxe ao salvar (`Ctrl+S` revisa), mas ainda não tem autocomplete das opções.
- Opções vindas de um objeto espalhado (`...base`) não aparecem no formulário; um aviso indica isso.
