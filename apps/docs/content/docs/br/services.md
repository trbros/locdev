# Serviços: bancos e caches

O LocDev roda PostgreSQL, MariaDB, MySQL, MongoDB, Valkey e Redis para os seus projetos, sem Docker. Cada **instância** tem a própria pasta de dados, porta, configuração e log, roda supervisionada pelo daemon (com reinício em caso de falha) e escuta **só em `127.0.0.1`**.

```bash
locdev service list                              # serviços, versões instaladas e instâncias
locdev service install postgresql                # baixa e verifica a versão mais nova
locdev service create db --service postgres --start
locdev service status
locdev service backup db
locdev service stop postgresql                   # o nome do serviço vale quando há uma só instância
```

## De onde vêm os binários

O LocDev usa instalações que já existem na máquina (no `PATH`, em `/usr/lib/postgresql/*`, no Homebrew, em `Program Files`) **sem alterá-las**, e pode baixar versões próprias. Um download só é instalado quando o SHA-256 bate com o publicado pela fonte:

| Serviço    | Download pelo LocDev                    | Fonte                                                                                                                                             |
| ---------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| PostgreSQL | Linux, macOS, Windows                   | código oficial compilado pelo [theseus-rs/postgresql-binaries](https://github.com/theseus-rs/postgresql-binaries) (com atestação de proveniência) |
| MariaDB    | Linux, Windows                          | MariaDB Foundation (API de downloads, SHA-256 por arquivo)                                                                                        |
| MongoDB    | Ubuntu 22.04+/Debian 12, macOS, Windows | MongoDB, Inc. (community server, `.sha256` ao lado de cada arquivo)                                                                               |
| Valkey     | Linux                                   | download.valkey.io (builds para Ubuntu)                                                                                                           |
| MySQL      | não                                     | a Oracle publica só MD5, que o LocDev não aceita: use um MySQL do sistema ou a MariaDB                                                            |
| Redis      | não                                     | não há binários oficiais: use um Redis do sistema ou o Valkey (compatível)                                                                        |

Versões baixadas ficam em `~/.local/share/locdev/services/bin/` e podem ser removidas com `locdev service uninstall` (nunca as do sistema, e nunca uma versão em uso por uma instância).

## Instâncias

- **Dados** em `services/instances/<nome>/data`, nunca misturados entre instâncias; a pasta é criada com permissão `0700`.
- **Porta**: a padrão do serviço quando livre (5432, 3306, 27017, 6379), senão a próxima livre. Duas instâncias nunca disputam a mesma porta, e Redis e Valkey (que compartilham a 6379 por padrão) ganham portas diferentes.
- **Acesso**: só `127.0.0.1`, usuários locais sem senha (`locdev` no PostgreSQL, `root` na MariaDB/MySQL). É um ambiente de desenvolvimento; não exponha essas portas.
- **Conexão**: `locdev service create` e a página **Serviços** mostram a URL e as variáveis usuais (`DATABASE_URL`, `DB_HOST`, `DB_PORT`, `REDIS_URL`…) para copiar para o projeto.
- **Início com o LocDev**: `--autostart` ou `locdev service autostart db true`. As instâncias sobem antes dos sites.
- **Pronta** quer dizer que aceita conexões de verdade; o PID sozinho não basta.

## Backup e remoção

`locdev service backup` faz uma **cópia a frio**: para a instância, compacta a pasta de dados e sobe de novo. `locdev service restore <instância> <id>` coloca uma cópia de volta e **move os dados atuais para `data.before-<hora>`**, sem apagar.

Remover uma instância **mantém os dados**. Apagá-los exige `--delete-data` e digitar o nome da instância; os backups continuam.

## SQLite

SQLite é um arquivo dentro do projeto (`database/database.sqlite`, por exemplo): não há nada para instalar, iniciar ou parar. Para fazer backup, copie o arquivo enquanto a aplicação não está gravando.

## Extensões do PHP

`locdev runtime extensions <site>` compara o que o projeto exige com o que o PHP do site carrega:

- `ext-*` no `composer.json` e nos pacotes do `composer.lock`;
- os requisitos do framework (Laravel: ctype, curl, dom, fileinfo, filter, hash, mbstring, openssl, pcre, pdo, session, tokenizer, xml);
- os drivers que o `.env` usa (`DB_CONNECTION=pgsql` → `pdo_pgsql`; Redis com o cliente phpredis → `redis`).

Nada é instalado automaticamente. O comando diz como resolver para aquele PHP: o pacote do apt (`php8.3-pgsql`), o `pecl` do Homebrew, a linha do `php.ini` no Windows. Os builds de PHP do LocDev são estáticos, com um conjunto fixo de extensões; para uma que falte neles, use um PHP do sistema que a tenha. O comando sai com código 1 quando falta alguma.

## Limites

- Validado de verdade no Linux com PostgreSQL 18.6 e Redis 8.10 do sistema. A instalação gerenciada (download, verificação e extração) tem testes, mas ainda não foi exercitada de ponta a ponta.
- Atualizar uma instância para outra versão major (PostgreSQL 16 → 17, por exemplo) não é automático: crie uma instância nova, exporte e importe os dados.
- Windows e macOS ainda não foram testados.

## Outras versões pelo gerenciador de pacotes

Onde o LocDev não consegue baixar um build verificado (MySQL e Redis não publicam SHA-256), ou para ter mais versões, **Serviços** oferece o que o gerenciador de pacotes do sistema tem. O comando aparece antes e roda num terminal dentro do app:

| Sistema                      | Oferecido                                                                                                                                                    |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| macOS (e Linux com Homebrew) | `mysql`, `mysql@8.4`, `mysql@8.0`, `redis`, `redis@8.2`, `postgresql@15` a `@17`, `mongodb-community@7.0` e `@8.0` (tap `mongodb/brew`), `mariadb`, `valkey` |
| Debian e Ubuntu              | `mysql-server`, `redis-server`, `postgresql` e `mariadb-server` da distribuição (o `sudo` pede a senha no terminal)                                          |
| Windows                      | MySQL (mais recente, 8.4 e 8.0) pelo winget (`Oracle.MySQL`)                                                                                                 |

O gerenciador de pacotes verifica o que instala. O resultado é uma instalação do sistema que o LocDev detecta (inclusive as fórmulas versionadas do Homebrew em `opt/`) e nunca altera nem remove. O Redis não tem build oficial para Windows: use WSL ou Memurai.

```bash
locdev service packages mysql
locdev service install-package mysql mysql@8.4
```
