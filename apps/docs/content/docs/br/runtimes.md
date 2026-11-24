# Runtimes e gerenciadores de pacotes

Cada site pode rodar com a sua versão de Node.js e de PHP, e com a sua versão de pnpm, Yarn, Bun ou Composer, sem mexer no ambiente global da máquina. O LocDev coloca as versões certas no início do `PATH` do processo do site.

O LocDev separa três coisas:

- **Runtimes** executam o código: Node.js e PHP.
- **Gerenciadores de pacotes** instalam dependências: npm, pnpm, Yarn, Bun e Composer. Eles rodam sobre o runtime do site (o pnpm usa o `node` do site; o Composer, o `php` do site).
- **Ferramentas de build** (Vite, webpack, Next…) são dependências do próprio projeto e aparecem na página de dependências, não aqui.

![Versões instaladas pelo LocDev e as que já estavam na máquina, com o default de cada runtime.](screenshot:runtimes)

## De onde vêm as versões

- **Gerenciadas pelo LocDev** (selo `locdev`): baixadas da distribuição oficial e **verificadas com o SHA-256 publicado** antes de instalar. A instalação acontece numa pasta temporária e só entra no lugar com um `rename`, então nunca fica pela metade.
- **Detectadas no sistema** (selo com a origem): pacotes do sistema (`/usr/bin/php8.3`), Homebrew (`php@8.2`, `node@20`), nvm, fnm e outras ferramentas no `PATH`. O LocDev só usa essas versões; nunca as altera nem remove.

| Ferramenta | Instalação pelo LocDev                                                                            | Detecção                                             |
| ---------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Node.js    | nodejs.org, verificado por `SHASUMS256.txt`                                                       | nvm, fnm, Homebrew, `PATH`                           |
| PHP        | binários prontos do StaticPHP (Linux/macOS) e do windows.php.net (Windows), SHA-256 do publicador | apt (`php8.x`), Homebrew, outras ferramentas, `PATH` |
| npm        | vem com cada Node.js e segue a versão dele                                                        | o npm de cada Node.js gerenciado, `PATH`             |
| pnpm       | registry npm, verificado pelo `integrity` (sha512)                                                | `~/.local/share/pnpm`, `PATH`                        |
| Yarn       | registry npm (`yarn` 1.x, `@yarnpkg/cli-dist` 2+), sha512                                         | `~/.yarn/bin`, `PATH`                                |
| Composer   | getcomposer.org, verificado pelo SHA-256 publicado                                                | `PATH`                                               |
| Bun        | não instala (use o instalador do bun.sh)                                                          | `~/.bun/bin`, `PATH`                                 |

pnpm, Yarn e Composer não são binários nativos: cada versão instalada ganha pequenos _launchers_ (`locdev-bin/pnpm`, `pnpm.cmd` no Windows) que chamam o `node`/`php` do `PATH`, isto é, o do site. Se a versão exigir um runtime mais novo (o `engines.node` do pnpm, o PHP mínimo do Composer), o `doctor` avisa.

## Suporte das versões

Cada versão mostra a fase de suporte do publicador: o calendário oficial do Node.js (`nodejs/Release`) e os estados das branches do PHP (`php.net/releases/states.php`). Versões em _manutenção_, _só segurança_ e _fim da vida_ ganham um selo, e o `doctor` avisa quando um site roda numa versão sem suporte. Uma cópia desses dados vem com o LocDev para funcionar offline; o daemon a atualiza em segundo plano (nunca com `[network] offline = true`).

## PHP do LocDev

O LocDev não compila PHP: ele baixa binários prontos de quem os publica, sempre conferindo o SHA-256 do publicador antes de instalar.

- **Linux e macOS:** os builds estáticos do [StaticPHP](https://static-php.dev) (o projeto static-php-cli), em `dl.static-php.dev`. O índice `versions.json`/`version_<MM>.json` traz a versão mais nova de cada linha com o SHA-256; as versões anteriores aparecem com o arquivo `.sha256` publicado ao lado de cada uma. Versão sem checksum publicado não aparece e não é instalada.
- **Windows:** os zips oficiais do [windows.php.net](https://windows.php.net/download/) (NTS x64), pelo `releases.json`, que traz o SHA-256 de cada zip. Ali só a versão mais nova de cada linha tem checksum publicado, então só ela é oferecida. Os zips oficiais precisam do Visual C++ Redistributable instalado.

O LocDev usa o conjunto de extensões `bulk` do StaticPHP, que cobre Laravel, Symfony e WordPress: `bcmath`, `curl`, `gd`, `imagick`, `intl`, `mbstring`, `mysqli`, `opcache`, `openssl`, `pdo_mysql`, `redis`, `sodium`, `zip` e outras. Os drivers PDO de SQLite e PostgreSQL vêm pelos hooks do Swoole incluídos no build (`PDO::getAvailableDrivers()` lista `sqlite` e `pgsql`), então `pdo_sqlite`/`pdo_pgsql` não aparecem no `php -m`, mas o Laravel com SQLite funciona normalmente. O conjunto `common` (`[runtimes] php_flavor = "common"` em `config.toml`) é menor e traz `pdo_sqlite`/`pdo_pgsql` como extensões, mas não tem `intl`, `opcache`, `sodium` nem `mysqli`. A aba de extensões do site (`locdev runtime extensions`) mostra o que falta para cada projeto.

Ao instalar, o LocDev grava um `php.ini` ao lado do binário (`bin/php.ini`, onde o PHP procura primeiro) apontando `curl.cainfo` e `openssl.cafile` para o pacote de certificados do sistema: os builds estáticos não sabem onde ele fica, e sem isso o Composer e o Guzzle falham em HTTPS. No Windows, o `php.ini` gerado habilita as extensões incluídas no zip.

```bash
locdev runtime available php          # a mais nova de cada linha (8.2, 8.3, 8.4, 8.5…)
locdev runtime available php --all    # todas as versões com checksum publicado
locdev runtime install php 8.4
```

> [!NOTE]
> Um mirror com a mesma estrutura pode ser usado em `config.toml`: `[runtimes] php_static_url` (Linux/macOS) e `php_windows_url` (Windows). O checksum continua vindo do índice do mirror, então use só um mirror em que você confia.

Para ajustar `memory_limit`, upload, fuso e outras diretivas de cada versão sem mexer no `php.ini` do sistema, veja [Configurações do PHP](/docs/php-settings).

## Fixando versões

Um _pin_ pode ser uma linha (`22`, `8.3`) ou uma versão exata (`22.11.0`). Uma linha escolhe a versão mais nova instalada daquela linha.

```bash
locdev runtime install node 22        # a 22.x mais recente
locdev runtime install node lts       # a LTS atual
locdev runtime default node 22        # default para sites sem pin
locdev edit blog --node 20            # pin só deste site
locdev edit shop --php 8.3
locdev edit blog --node ""            # volta a usar o default
locdev reload blog                    # aplica
```

No app, os pins ficam em **Editar configurações…** do site, e o default fica na tela **Runtimes**, pelo menu **⋯** ou pelo clique direito em uma versão.

Ordem de resolução, por runtime ou gerenciador:

1. o pin do próprio site (`locdev edit`);
2. senão, um **arquivo de versão** do projeto: `.nvmrc`, `.node-version`, `.php-version`, `.tool-versions`, `volta` e `packageManager` do `package.json`, `config.platform.php` do `composer.json`. O `.nvmrc` aceita `lts/*`, `lts/iron` e `node`;
3. senão, se o projeto declara uma **faixa** (`engines.node`, `require.php`) e o default não a satisfaz, a versão instalada mais nova dentro dela;
4. senão, o default;
5. senão, o que vier primeiro no `PATH` do sistema, que é o comportamento sem LocDev.

Se um pin ou arquivo de versão aponta para um Node.js ou PHP que não está instalado, o site **não sobe** e o erro diz o comando que resolve. Um `reload` nessa situação mantém o processo atual no ar. Para gerenciadores de pacotes isso é só um aviso: o do `PATH` é usado. Quando a versão escolhida está fora da faixa que o projeto declara, o detalhe do site e o `doctor` mostram a diferença.

## No terminal

As versões padrão valem para os sites sem pin. Para usá-las também no terminal (`php`, `node`, `npm`, `composer`…), ligue **Usar as versões padrão no terminal** em **Runtimes → Visão geral**, ou:

```bash
locdev runtime default php 8.4
locdev runtime default node 22
locdev runtime terminal on      # abra um terminal novo depois
locdev runtime terminal         # estado, comandos e arquivos alterados
locdev runtime terminal off
```

O LocDev mantém uma pasta só dele (`~/.local/share/locdev/bin` no Linux) com um pequeno script por comando, que chama a versão padrão. Trocar o default no app troca o que o terminal usa, sem reabrir nada. Um tipo sem default não ganha script, e o terminal encontra o do sistema.

- **Linux e macOS:** um bloco marcado no fim do `~/.bashrc` e do `~/.zshrc` (os que existem, mais o do seu shell) põe a pasta no início do `PATH`; no fish, um arquivo próprio em `~/.config/fish/conf.d/locdev.fish`. Desligar remove exatamente esse bloco.
- **Windows:** a pasta vai para o início do `Path` do usuário. O `Path` do sistema vem antes dele, então um PHP ou Node.js instalado para todos os usuários continua tendo prioridade.

## Remoção segura

`locdev runtime uninstall` recusa remover uma versão que um site **em execução** usa. Se sites parados ou o default resolvem para ela, a remoção pede `--force` (no app, uma confirmação que lista os sites). Versões do sistema nunca são removidas.

## Atualizações

Quando há uma versão mais nova de uma linha que você instalou (por exemplo, Node.js 22.23.4 → 22.23.6), a lista mostra a atualização. `locdev runtime update node 22` instala a nova versão; a antiga continua instalada até você removê-la, para não quebrar sites que fixaram a versão exata.

## Comandos

| Comando                                                                   | Descrição                                                                                                        |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `locdev runtime list [--refresh]`                                         | Versões gerenciadas e detectadas, defaults, suporte, uso e atualizações. `--refresh` procura de novo na máquina. |
| `locdev runtime available <tipo> [--all]`                                 | O catálogo (a mais nova de cada linha, ou todas).                                                                |
| `locdev runtime install <tipo> <versão>`                                  | Baixa, verifica e instala. Aceita `22`, `8.3`, `22.11.0`, `lts`, `latest`.                                       |
| `locdev runtime update <tipo> [linha]`                                    | Instala a versão mais nova das linhas já instaladas.                                                             |
| `locdev runtime uninstall <tipo> <versão> [--force]`                      | Remove uma versão gerenciada.                                                                                    |
| `locdev runtime default <tipo> [versão]`                                  | Define o default (`--clear` remove).                                                                             |
| `locdev runtime terminal [on\|off]`                                       | Usa as versões padrão nos terminais novos (sem argumento, mostra o estado).                                      |
| `locdev edit <site> --node/--php/--pnpm/--yarn/--composer/--bun <versão>` | Pin do site (`""` volta ao default).                                                                             |

`<tipo>` é `node`, `php`, `pnpm`, `yarn`, `composer` ou `bun` (`npm` só aparece na lista). O processo do site também recebe `LOCDEV_NODE_VERSION`, `LOCDEV_PNPM_VERSION` etc. com a versão resolvida.

## Extensões do PHP

**Runtimes → PHP → Extensões** (ou `locdev runtime modules [versão]`) lista todas as extensões que um PHP desta máquina carrega, com a versão de cada uma, o `php.ini` lido e os arquivos que a pasta de extensões tem mas não carrega (Windows: `php_*.dll` em `ext`; ative no `php.ini`). O LocDev só executa um PHP que ele mesmo detectou.

Para um site, **Visão geral → Extensões do PHP** (ou `locdev runtime extensions <site>`) compara o que o projeto precisa com o que o PHP dele carrega: `ext-*` no `composer.json` e no `composer.lock`, os requisitos do framework e o driver de banco que o `.env` cita. As que faltam vêm com o jeito de obtê-las para aquele PHP.

Também lista as extensões que os pacotes instalados **recomendam**: as entradas `suggest` do `composer.lock` (por exemplo `intervention/image` → `gd` ou `imagick`), cada uma marcada como carregada ou ausente, com o motivo dado pelo próprio pacote. Recomendação nunca bloqueia nada.

```bash
locdev runtime modules 8.4
locdev runtime extensions loja
```
