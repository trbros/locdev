# Arquitetura

```text
          ┌────────────┐     ┌──────────────┐
          │ app (Tauri)│     │  locdev CLI  │
          └─────┬──────┘     └──────┬───────┘
                │   IPC local (JSON) │
                └─────────┬──────────┘
                    ┌─────▼──────┐
                    │  locdevd   │  sem privilégios
                    └─┬───┬───┬──┘
        ┌─────────────┘   │   └──────────────┐
  ┌─────▼──────┐   ┌──────▼──────┐   ┌───────▼──────┐
  │ supervisor │   │ proxy HTTP/S│   │  DNS *.test  │
  │ processos  │   │ SNI + CA    │   │ 127.0.0.1    │
  └────────────┘   └─────────────┘   └──────────────┘
```

## Crates

| Crate               | Responsabilidade                                                                                               |
| ------------------- | -------------------------------------------------------------------------------------------------------------- |
| `locdev-core`       | Modelos, erros, paths, detecção de projetos, parsing de hosts e o protocolo IPC.                               |
| `locdev-ipc`        | Transporte local, cliente e launcher do daemon (compartilhado pela CLI e pelo app).                            |
| `locdev-daemon`     | `locdevd`: registro, supervisor, proxy, DNS, TLS e doctor.                                                     |
| `locdev-proxy`      | Proxy reverso por hostname, com WebSocket e HTTPS.                                                             |
| `locdev-dns`        | Servidor DNS autoritativo mínimo, escrito à mão.                                                               |
| `locdev-tls`        | CA local, emissão de certificados e verificação de confiança.                                                  |
| `locdev-platform`   | Integração com o SO: resolvedor, trust store, login item e elevação.                                           |
| `locdev-runtimes`   | Versões de Node.js, PHP e gerenciadores: catálogo, download verificado, extração segura, detecção e shims.     |
| `locdev-frameworks` | Definições de frameworks (dados assinados) e o motor de detecção.                                              |
| `locdev-env`        | Variáveis compartilhadas: leitura de `.env`, regras por framework, resolução, sincronização e o cofre cifrado. |
| `locdev-services`   | Bancos e caches: instalações, instâncias e backup a frio.                                                      |
| `locdev-db`         | Database Manager: drivers de PostgreSQL, MySQL/MariaDB, SQLite e Redis/Valkey.                                 |
| `locdev-mail`       | Mailbox: SMTP só de captura, armazenamento e análise das mensagens.                                            |
| `locdev-email`      | Email Studio: blocos entre comentários, descoberta, preview e gerador.                                         |
| `locdev-config`     | Config Files: catálogo de opções por versão, edição de JSON e JS/TS sem executar, regras.                      |
| `locdev-deploy`     | Assistente de deploy: perfil da stack, plataformas com fontes, preparo e arquivos gerados.                     |
| `locdev-deps`       | Dependências: leitura local, registros npm/Packagist, OSV e comandos de atualização.                           |
| `locdev-git`        | O `git` do sistema sem shell: status, diff, commit, branches, stash, remotos, reset/revert/restore e backups.  |
| `locdev-github`     | Token no keyring, cliente da API (só `api.github.com`) e device flow.                                          |
| `locdev-actions`    | GitHub Actions builder: templates, YAML ↔ modelo, validação e gravação segura.                                 |
| `apps/locdev`       | A CLI.                                                                                                         |
| `apps/desktop`      | O app Tauri + React (cliente fino do daemon).                                                                  |

Todo o código Rust compila com `unsafe_code = "forbid"`.

## Protocolo IPC

As mensagens são JSON com prefixo de 4 bytes de tamanho, sobre um socket Unix (`0600`, em um diretório `0700` verificado) ou um named pipe por usuário que rejeita clientes remotos. Cada mensagem leva a versão do protocolo:

```json
{ "protocol": 2, "body": { "type": "start_project", "name": "blog" } }
```

Uma versão incompatível vira um erro claro (`INCOMPATIBLE_PROTOCOL`), sem derrubar o daemon. O app e a CLI enviam exatamente os mesmos pedidos.

## Operações privilegiadas

O daemon nunca roda como administrador. Quando algo precisa de privilégio (resolvedor, trust store), o `locdev-platform` gera um **plano**: um script legível, com uma etapa como administrador e, opcionalmente, outra como usuário. A CLI e o app mostram o plano e só o executam depois da confirmação, pelo prompt do próprio SO (`sudo`, polkit, diálogo do macOS ou UAC). O app recalcula o plano no lado Rust, então o que roda é o que foi mostrado.
