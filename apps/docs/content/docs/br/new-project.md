# Novo projeto

Em **Ferramentas → Novo projeto** (ou `locdev new`), o LocDev cria um projeto com o **instalador do próprio framework** (`laravel new`, `composer create-project`, `npm create vite`, `create-vue`, `ng new`, `rails new`, `cargo init`…), com as opções que esse instalador documenta, e no fim cadastra a pasta como site. Você não precisa abrir a documentação do framework para lembrar dos comandos.

![Novo projeto: o framework, como criar, as opções, o banco do LocDev, pacotes e os comandos que vão rodar.](screenshot:new-project)

## Como funciona

1. **Escolha o framework.** O formulário dele abre numa tela própria (**← Frameworks** volta). Cada framework tem as próprias formas de criar e as próprias opções (tabela abaixo).
2. **Nome e pasta.** O nome vira a pasta e o site (`loja` → `loja.test`). A pasta não pode existir: o LocDev nunca cria por cima de nada. No app, os projetos novos vão por padrão para `~/LocDev/Sites` (criada no primeiro uso; dá para escolher outra pasta); o `locdev new` usa a pasta atual.
3. **Como criar.** Por exemplo, Laravel pelo instalador ou pelo Composer; os projetos JavaScript por npm, pnpm ou Bun. Cada opção mostra se a ferramenta está instalada e a versão.
4. **Runtime** (PHP e JavaScript): a versão do PHP ou do Node.js com que o instalador roda, entre as instaladas. O site novo fica fixado nela. **Padrão** usa a versão padrão do LocDev.
5. **Opções.** Kit inicial, autenticação, testes, TypeScript, Tailwind, linter, roteamento, template… Só aparecem as que o método escolhido aceita.
6. **Banco de dados** (Laravel): uma instância do LocDev (PostgreSQL, MySQL ou MariaDB), nova ou existente.
7. **Pacotes:** busca no npm ou no Packagist, conforme o framework, para instalar logo depois de criar.
8. **O que vai rodar:** os arquivos que o LocDev escreve (quando houver), cada comando exato e o que o próprio LocDev faz entre eles. Nada roda antes de **Criar projeto**.

Depois disso, os passos rodam num **terminal dentro do app**: você vê a saída do instalador e pode responder o que ele perguntar. As linhas que começam com › são do LocDev (arquivos, banco, site). Ao terminar, o site é registrado e **inicia sozinho**: o botão dele mostra **Rodando** (clique para parar), ao lado de **Abrir site**.

![O instalador rodando no terminal do app, passo a passo.](screenshot:new-project-run)

## Frameworks sem gerador

PHP puro, Flask, FastAPI, Axum e Actix Web não têm um comando oficial que crie um projeto pronto para servir. Para eles, o LocDev escreve **arquivos iniciais mínimos** antes dos comandos, e o plano mostra cada um (clique para ver o conteúdo):

| Framework | Arquivos do LocDev                                                | Depois                                                            |
| --------- | ----------------------------------------------------------------- | ----------------------------------------------------------------- |
| PHP       | `public/index.php`                                                | `git init` (opcional)                                             |
| Flask     | `app.py`, `requirements.txt` (`flask`), `.gitignore`              | `.venv` + `pip install -r requirements.txt`, `git init`           |
| FastAPI   | `main.py`, `requirements.txt` (`fastapi[standard]`), `.gitignore` | `.venv` + `pip install -r requirements.txt`, `git init`           |
| Axum      | `src/main.rs`                                                     | `cargo init`, `cargo add axum`, `cargo add tokio --features full` |
| Actix Web | `src/main.rs`                                                     | `cargo init`, `cargo add actix-web`                               |

O LocDev cria a pasta, escreve os arquivos sem nunca sobrescrever nada e só então roda as ferramentas oficiais: o `cargo init` adota o `src/main.rs` que já está lá, e o `pip` instala o que o `requirements.txt` lista. Os arquivos são escolhidos para a detecção reconhecer o framework (um teste confere), então o site sobe com o comando certo: `php -S … -t public`, `flask --app app run`, `fastapi dev main.py` ou `cargo run`. Os servidores Rust leem a porta de `PORT`, que o LocDev define.

O WordPress usa o WP-CLI (`wp core download`, que confere o hash do download); a instalação termina no navegador, na primeira visita.

## O que o LocDev faz além dos comandos

Com um banco do LocDev escolhido, entre a criação e as migrações:

- cria a instância, se ainda não existir, e a inicia;
- espera ela aceitar conexões e cria o banco com o nome do projeto (`minha-loja` → `minha_loja`);
- aponta `DB_CONNECTION`, `DB_HOST`, `DB_PORT`, `DB_DATABASE`, `DB_USERNAME` e `DB_PASSWORD` do `.env` **do projeto recém-criado** para ela (linhas existentes trocadas no lugar, inclusive as comentadas pelo Laravel; o resto do arquivo fica como estava);
- roda `php artisan migrate`.

No instalador do Laravel, o driver (`--database=pgsql`, `mysql` ou `mariadb`) segue o banco escolhido.

## Frameworks e opções

<!-- generated:starters -->

As flags de cada instalador vêm do código-fonte ou da documentação oficial dele, e um teste confere os comandos gerados para cada combinação. Os comandos rodam sem shell, só com programas de uma lista fixa (`laravel`, `composer`, `npm`, `npx`, `pnpm`, `bun`, `bunx`, `rails`, `dotnet`, `python3`, `cargo`, `wp`, `git`…), e o nome do projeto e dos pacotes é validado antes de entrar num comando.

Dois detalhes de comando que valem para todos:

- `npm create --yes x@latest` evita que o npm pare perguntando se pode baixar o gerador.
- Os instaladores rodam sem as opções que pulam as perguntas deles (`--no-interaction` no `laravel new`, no Composer e no `artisan migrate`, `--yes` no `create-next-app` e no `create-astro`, `--defaults` no `ng new`): o que as flags do LocDev não respondem, o instalador pergunta no terminal, onde você também vê a saída e qualquer erro.
- O Angular CLI roda com `NG_CLI_ANALYTICS=false` (mostrado no comando), a forma documentada de não enviar dados de uso e não perguntar sobre isso no meio da criação.

## Instaladores

Em **Ferramentas → Instaladores** (ou `locdev tools list`) ficam as ferramentas que os projetos precisam, se estão instaladas, em qual versão e onde:

| Ferramenta                                                                       | Como instalar pelo LocDev                                                              |
| -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| PHP, Composer, Node.js, npm, pnpm                                                | em [Runtimes](/docs/runtimes), com download verificado                                 |
| Instalador do Laravel                                                            | `composer global require laravel/installer`, num terminal, depois de mostrar o comando |
| Rails                                                                            | `gem install rails` (precisa de Ruby; instalação por usuário quando a pasta de gems do Ruby exige administrador) |
| Symfony CLI, Bun, uv, Poetry, Cargo, rustup, WP-CLI, .NET SDK; Python, pipx e Git no Windows e no macOS | o comando documentado pelo autor (veja [Instaladores pelo app](#instaladores-pelo-app)); nos outros casos, link para o instalador oficial |

As ferramentas são encontradas também onde os instaladores oficiais as colocam, mesmo fora do `PATH`: a pasta global do Composer (`~/.config/composer/vendor/bin`, `~/.composer/vendor/bin` ou `%APPDATA%\Composer\vendor\bin`), `~/.bun/bin`, `~/.cargo/bin` (rustup), `~/.local/bin`, a do Symfony CLI (`~/.config/symfony-cli/bin`) e as gems por usuário (`~/.local/share/gem/ruby/*/bin`). Elas vêm depois do `PATH`, então os runtimes do LocDev, primeiro no `PATH`, são os encontrados e usados.

## Na linha de comando

```bash
locdev new                                        # pergunta o que criar, passo a passo
locdev new list                                   # frameworks, métodos e opções
locdev new laravel loja --php 8.4 -o kit=react --database postgres:db
locdev new nextjs web --package-manager pnpm -o linter=biome -p zod
locdev new vite ui --typescript
locdev new django api --in ~/Code --yes
locdev new rust borda -o framework=actix
locdev tools list
locdev tools install laravel
```

`locdev new <framework> <nome>` é o mesmo que `locdev new project <framework> <nome>` (que continua funcionando), com as mesmas opções:

| Opção                            | Faz                                                         |
| -------------------------------- | ----------------------------------------------------------- |
| `--method` / `--package-manager` | como criar (`installer`, `composer`, `npm`, `pnpm`, `bun`…) |
| `-o CHAVE=VALOR`                 | uma opção do framework (`locdev new list` mostra todas)     |
| `--typescript`                   | TypeScript, nos frameworks que oferecem                     |
| `--php` / `--node`               | a versão com que o instalador roda; o site fica fixado nela |
| `-p PACOTE`                      | um pacote para adicionar depois                             |
| `--database SERVIÇO:INSTÂNCIA`   | um banco do LocDev (Laravel)                                |
| `--in PASTA`                     | onde criar (a pasta atual por padrão)                       |
| `--no-site`                      | não cadastrar como site                                     |
| `--yes`                          | não perguntar antes de rodar                                |

`locdev new` sozinho, num terminal, pergunta o framework, o nome, a pasta, o método, cada opção e a versão do runtime, mostra o plano e pede confirmação. A saída aparece no terminal, e o que você digita vai para o instalador.

## Limites

- Validado de ponta a ponta no Linux, criando e servindo o site pelo LocDev: Laravel 13 pelo instalador do Laravel (com uma instância nova de PostgreSQL, o banco criado, o `.env` apontado e as migrações), Vue, Angular, Flask, FastAPI, PHP puro, Axum e Actix Web, e a criação com uma versão de PHP escolhida (fixada no site). Também conferidos no navegador, em HTTPS e HTTP, sem erros no console: Laravel (com e sem starter kit, com o auxiliar Vite), Symfony (Composer), Bedrock, Next.js, Nuxt, Vite, Vue, Astro, SvelteKit, Angular, NestJS, Django (incluindo o login do admin em HTTPS), Flask, FastAPI, Rails, ASP.NET Core e Axum. O WordPress pelo WP-CLI não foi executado porque o WP-CLI não estava instalado.
- Para Django, Flask e FastAPI, um `.venv` dentro do projeto recebe os pacotes pelo `pip`; o pacote de configurações do Django se chama `config`, e o `settings.py` recebe, dentro de `if DEBUG:`, `.test` no `ALLOWED_HOSTS` e o `SECURE_PROXY_SSL_HEADER` para o proxy do LocDev.
- O `create-vue` só pula as perguntas dele quando alguma opção está ligada; com tudo desligado e JavaScript, ele pergunta no terminal.
- Os pacotes adicionados rodam os scripts de instalação que o gerenciador rodaria normalmente.
- Windows e macOS ainda não foram testados (o terminal usa ConPTY no Windows).

## Laravel: as perguntas do próprio instalador

O LocDev pergunta o que o `laravel new` pergunta, na mesma ordem:

1. **Quer usar um starter kit?**
2. Sem starter kit: a stack de frontend (Blade, ou React, Vue, Svelte e Livewire como kits em branco, sem autenticação).
3. Com starter kit: a stack do kit, o provedor de autenticação (o do Laravel ou WorkOS) e, com o do Laravel, componentes Livewire de arquivo único e suporte a times.
4. Depois: testes (Pest ou PHPUnit), banco, dependências de frontend, Laravel Boost e Git.

O instalador roda com as flags correspondentes (`--react --workos`, `--vue --no-authentication`, `--livewire-class-components`, `--teams`…). Com o Composer, as mesmas escolhas viram o pacote e a branch do kit (`laravel/react-starter-kit:dev-teams`, `--stability=dev`).

## Instaladores pelo app

**Ferramentas → Instaladores** lista as ferramentas que os starters usam e as instala sem sair do app. PHP, Composer, Node.js e pnpm são instalados como runtimes verificados; as outras, com o comando documentado pelo publicador para o seu sistema, mostrado antes e executado num terminal dentro do app:

| Ferramenta            | Comando                                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Instalador do Laravel | `composer global require laravel/installer`                                                                         |
| Symfony CLI           | `scoop install symfony-cli` (Windows), `brew install symfony-cli/tap/symfony-cli` (macOS), o script oficial (Linux) |
| Bun                   | o script oficial (`curl -fsSL https://bun.sh/install \| bash`; PowerShell no Windows) |
| uv                    | `winget install --id=astral-sh.uv -e` (Windows), o script oficial (macOS, Linux)                                    |
| Poetry                | O instalador oficial                                                                                                |
| Python                | `winget install -e --id Python.Python.3.13` (Windows), `brew install python` (macOS); no Linux, o pacote da sua distribuição |
| pipx                  | `scoop install pipx` (Windows), `brew install pipx` (macOS); no Linux, o pacote da sua distribuição |
| Rust (rustup, Cargo)  | O script oficial (macOS, Linux)                                                                                     |
| WP-CLI                | `composer global require wp-cli/wp-cli-bundle`                                                                      |
| Git                   | `winget install --id Git.Git -e --source winget` (Windows), `brew install git` (macOS) |
| .NET SDK              | `winget install Microsoft.DotNet.SDK.10` (Windows), `dotnet-install.sh --channel LTS` (macOS, Linux)                |
| Rails                 | `gem install rails`, ou `gem install --user-install rails` quando a pasta de gems do Ruby não aceita escrita (Ruby do sistema) |

Ferramenta sem comando para o seu sistema continua com o link da página de instalação. `locdev tools install <id>` faz o mesmo pelo terminal.
