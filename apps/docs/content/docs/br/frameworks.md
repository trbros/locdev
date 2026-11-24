# Frameworks e stacks

Ao adicionar uma pasta, o LocDev descobre do que o projeto é feito **só lendo arquivos**: manifestos (`package.json`, `composer.json`, `Gemfile`, `pyproject.toml`, `requirements.txt`, `Cargo.toml`, `go.mod`, `pom.xml`, `build.gradle`, `*.csproj`), lockfiles e arquivos de configuração. Nenhum script do projeto é executado.

O resultado é uma lista de **componentes** (framework, runtime, linguagem, UI, ferramentas), cada um com a evidência que o provou e a versão, quando dá para saber: a instalada (`node_modules`, `vendor/composer/installed.json`, `Gemfile.lock`, `uv.lock`, `poetry.lock`, `Cargo.lock`, `pom.xml`) ou, sem ela, a faixa declarada. Um deles é o **principal**, o adaptador que sabe iniciar o servidor de desenvolvimento.

```bash
locdev stack detect ~/Code/api     # uma pasta qualquer
locdev stack inspect shop          # um site, com evidências, versões e pré-requisitos
```

## Integração

| Tipo                   | Significa                                                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| nativa                 | o LocDev monta o comando com host `127.0.0.1` e a porta do site (`{port}`)                                                            |
| script do package.json | roda um script do projeto (`dev`, `start`…); quando o framework não aceita a porta por argumento, o app precisa ler a variável `PORT` |
| comando personalizado  | o framework foi reconhecido, mas o comando de início tem que vir de você (`locdev edit <site> --command`)                             |
| componente             | roda através de outro (React pelo Vite, Tailwind pelo build)                                                                          |

Alguns adaptadores passam a porta também nas variáveis do próprio framework: o Spring Boot recebe `SERVER_PORT` e `SERVER_ADDRESS`, e o .NET recebe `ASPNETCORE_URLS=http://127.0.0.1:<porta>` (com `--no-launch-profile`, para o `launchSettings.json` não trocar o endereço), além de `ASPNETCORE_ENVIRONMENT=Development`, que o perfil de inicialização definiria, a menos que o site já tenha essa variável. Projetos Python com `.venv/` (ou `venv/`) rodam com o ambiente virtual primeiro no `PATH` e `VIRTUAL_ENV` definido.

Quando vários frameworks combinam (um NestJS também tem Express), vence o de maior prioridade **que sabe iniciar este projeto**. Um Vite instalado sem script que o rode, por exemplo, não vira o principal; o Node.js genérico assume. Versões mais antigas que as suportadas pelo adaptador aparecem como aviso, sem bloquear.

## Monorepos

Workspaces do npm/Yarn (`workspaces`), `pnpm-workspace.yaml`, `[workspace]` do Cargo e `go.work` são lidos, e cada pacote aparece com o próprio framework. Cada pacote vira um site separado (`locdev add apps/web`). Na raiz de um workspace Cargo sem `[package]`, nenhum comando é sugerido.

## Escolhas manuais

Quando a detecção não basta:

```bash
locdev stack add tailwind shop                 # um componente que os arquivos não provam
locdev stack remove python shop                # esconde um componente detectado
locdev stack use flask shop --apply-command    # escolhe o adaptador e usa o comando dele
locdev stack use auto shop                     # volta para a detecção
```

As escolhas ficam no `projects.toml` (`adapter`, `stacks`, `hidden_stacks`). No app, a seção **Stack** da visão geral do site tem o mesmo controle (**Editar…**).

## Definições atualizáveis

O que cada framework é (regras de detecção, origem da versão, comandos, arquivos de configuração, regras de `.env`, links da documentação oficial) fica em **definições**: dados puros, separados do código. O LocDev traz um conjunto embutido e pode baixar um mais novo sem precisar de outra versão do app:

- o conjunto é publicado assinado com minisign, e o LocDev só usa um conjunto cuja assinatura bate com a chave pública que ele traz;
- o formato é versionado. Um conjunto que exige capacidades que esta versão não tem é recusado, e campos desconhecidos também;
- comandos só podem usar programas de uma lista fixa (`npm`, `php`, `python3`, `bin/rails`, `cargo`, `dotnet`…), sem metacaracteres de shell, e caminhos só dentro do projeto. Nem um conjunto assinado consegue executar outra coisa;
- só uma revisão mais nova substitui a atual, e uma revisão mais antiga que a mais nova já vista é recusada;
- `locdev stack definitions --rollback` volta ao conjunto anterior (ou ao embutido), e a revisão abandonada é pulada nas verificações automáticas;
- a verificação é diária (pela fila de jobs), respeita o modo offline e pode ser desligada: `locdev settings set automation.definitions off`.

```bash
locdev stack definitions             # revisão em uso, origem, última verificação
locdev stack definitions --refresh   # verifica agora
```

O estado também aparece no `doctor` e em **Settings → Frameworks**. A publicação é feita pela ferramenta `tools/definitions` (build, assinatura, verificação), que também consulta os registros (npm, Packagist, PyPI, RubyGems) para avisar quando sai um major novo de algum framework. Enquanto nenhuma atualização é publicada, o LocDev usa as definições que vêm com ele, sem aviso.

## Stacks reconhecidas

<!-- generated:frameworks -->

Os serviços que cada stack costuma usar (bancos, caches) aparecem no `locdev stack inspect`, e o LocDev pode criar instâncias deles ([Serviços](/docs/services)). Filas, agendadores e servidores de WebSocket viram [processos auxiliares](/docs/servers) do site.
