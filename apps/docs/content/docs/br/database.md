# Database Manager

O LocDev abre os bancos que os seus sites usam e as instâncias que ele mesmo roda (veja [Serviços](/docs/services)): lista tabelas, mostra estrutura, pagina linhas, edita, insere, remove e roda SQL. Para Redis e Valkey há um navegador de chaves. No app fica em **Tools → Database**; na CLI, em `locdev db`.

![Tools → Database: conexões do site, tabelas e linhas.](screenshot:database)

| Motor           | Tabelas e estrutura | Linhas (filtro, ordem, edição)         | SQL | Criar o banco        |
| --------------- | ------------------- | -------------------------------------- | --- | -------------------- |
| PostgreSQL      | sim (com schemas)   | sim                                    | sim | sim                  |
| MySQL / MariaDB | sim                 | sim                                    | sim | sim                  |
| SQLite          | sim                 | sim                                    | sim | sim (cria o arquivo) |
| Redis / Valkey  | chaves por `SCAN`   | ler, editar strings, expiração, apagar | —   | —                    |

MongoDB ainda não tem navegador; as instâncias aparecem em **Serviços**.

## De onde vêm as conexões

Ninguém digita senha no LocDev. O daemon lê, **a cada uso**, o que o projeto já declara no `.env` e no `.env.local`:

- **Laravel:** `DB_CONNECTION`, `DB_HOST`, `DB_PORT`, `DB_DATABASE`, `DB_USERNAME`, `DB_PASSWORD` (com `sqlite`, o `DB_DATABASE` é o arquivo, ou `database/database.sqlite`).
- **`DATABASE_URL`** (Prisma, Rails, Django, Drizzle…): `postgres://`, `mysql://`, `mariadb://`, `file:` e `sqlite:`. No Prisma, um `file:./dev.db` é relativo à pasta `prisma/`, como o próprio Prisma faz.
- **Redis:** `REDIS_URL`, ou `REDIS_HOST`/`REDIS_PORT`/`REDIS_USERNAME`/`REDIS_PASSWORD`.
- **Arquivos SQLite** conhecidos: `database/database.sqlite`, `db.sqlite3`, `prisma/dev.db`, `storage/database.sqlite`, `db/development.sqlite3`.
- **Variáveis compartilhadas** que o site recebe (veja [Variáveis de ambiente](/docs/environment)) entram na leitura como se estivessem no `.env`.
- **Instâncias do LocDev**, com o usuário local de cada serviço.

O mesmo banco declarado duas vezes (por exemplo `DB_*` e `DATABASE_URL`) aparece uma vez só.

```bash
locdev db list                       # todas as conexões, de todos os sites e instâncias
locdev db connect acme               # host, porta, banco e usuário (nunca a senha)
locdev db tables acme
locdev db describe users acme        # colunas, índices e chaves estrangeiras
locdev db rows users acme --where role=admin --order-by id --desc
locdev db query "select count(*) from users" acme
locdev db create acme                # cria o banco que o .env do site nomeia
```

Com mais de um banco no site, a CLI usa o primeiro banco SQL; `--conn <id>` (de `locdev db list`) escolhe outro.

## O que fica protegido

- **Credenciais não saem do daemon.** O app e a CLI só conhecem um identificador (`project:<site>:<n>` ou `instance:<nome>`) e o fato de haver senha. Senhas não aparecem em respostas, logs nem mensagens de erro.
- **Bancos fora desta máquina** (um `DB_HOST` que não é `127.0.0.1`/`localhost`/`::1`) aparecem com o selo **remoto** e só abrem com uma escolha explícita: o botão _Abrir mesmo assim_ no app ou `--allow-remote` na CLI. Pode ser produção, e tudo o que você editar ali é real.
- **Nada é criado sem pedido.** Se o servidor responde mas o banco não existe, o app mostra um aviso com o botão _Criar_. O nome vem da configuração do site e é validado (letras, números, `_` e `-`).
- **Comandos destrutivos pedem o nome do banco.** `DROP`, `TRUNCATE`, `ALTER … DROP`, `GRANT`/`REVOKE`, e `DELETE`/`UPDATE` sem `WHERE` ou com um `WHERE` sempre verdadeiro (`1=1`, `true`, `id = id`, `… or 1=1`). O daemon refaz essa análise ao executar, então um cliente não consegue pular a confirmação. Fora de um terminal interativo, a CLI recusa em vez de perguntar.
- **Valores vão como parâmetros.** Filtros, edições e inserções nunca montam SQL com o texto digitado: colunas e tabelas vêm do catálogo do próprio banco, e os valores seguem como parâmetros.
- **Limites:** no máximo 1000 linhas por resultado (o resto é indicado como truncado) e 35 s por operação.

A análise de SQL destrutivo é uma heurística conservadora, não um parser completo. Ela pega os casos comuns, mas não substitui um backup: para as instâncias do LocDev, use `locdev service backup` antes de uma mudança grande.

## No app

- **Data:** grade paginada (50 por página), ordenação clicando no nome da coluna, filtros por coluna (`=`, `!=`, `<`, `>`, `like`, `is_null`…), edição com duplo clique (Enter salva, Esc cancela, _NULL_ grava nulo), inserção (campos vazios usam o padrão da coluna) e remoção com confirmação. Edição, inserção e remoção exigem chave primária; sem ela, use SQL.
- **Structure:** colunas, tipos, nulidade, padrões, índices e chaves estrangeiras.
- **SQL:** editor com realce, **Ctrl+Enter** para rodar, atalhos para inserir nomes de tabelas e histórico local (guardado só neste computador).
- **Redis/Valkey:** busca por padrão (`user:*`), tipo e TTL de cada chave, leitura por tipo (string, list, hash, set, zset), edição de strings mantendo a expiração, remover a expiração e apagar a chave.

## Limitações conhecidas

- Sem TLS: conexões a servidores que exigem SSL (`sslmode=require`, `rediss://`) falham. Para bancos remotos com TLS, use o cliente do próprio provedor.
- Valores são mostrados como texto. Colunas binárias aparecem como texto com perdas.
- Validado de verdade no Linux com SQLite, PostgreSQL 18 e Redis 8 (testes `crates/locdev-db/tests/real.rs`). MySQL/MariaDB está implementado, mas ainda não foi testado contra um servidor real.
