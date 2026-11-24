# Diretórios monitorados

Um diretório monitorado é uma pasta cujos projetos viram sites sozinhos. Monitore `~/Development` e cada projeto dentro dela ganha seu domínio:

```text
~/Development/
├── loja/        → loja.test
├── api/         → api.test
├── portfolio/   → portfolio.test
├── dashboard/   → dashboard.test
└── fotos/       (não é um projeto: ignorada)
```

A lista acompanha a pasta: um projeto criado, clonado ou gerado ali aparece em segundos, e um projeto apagado sai da lista. Os arquivos nunca são tocados.

## Disponibilidade

| Sistema | Estado                                                                            |
| ------- | --------------------------------------------------------------------------------- |
| Linux   | Disponível (testado; eventos via inotify)                                         |
| macOS   | Experimental: o código usa FSEvents, mas não foi validado num Mac                 |
| Windows | Experimental: o código usa ReadDirectoryChangesW, mas não foi validado no Windows |

Em qualquer sistema, se os eventos de arquivos não estiverem disponíveis, a pasta continua sendo varrida a cada 5 minutos.

## O que conta como projeto

Uma subpasta só vira site quando os arquivos dela dizem que é um projeto, pela mesma detecção do `locdev add` ([Frameworks](/docs/frameworks)): `artisan` e `composer.json` (Laravel), `package.json` (Node.js, Next.js, Vite…), `pyproject.toml`, `requirements.txt` ou `manage.py` (Python), `Cargo.toml` (Rust), `index.php`, `index.html` e os demais da tabela de frameworks. O nome da pasta não importa.

Nunca são considerados projetos nem varridos:

- pastas ocultas (`.git`, `.cache`…);
- dependências e saída de build: `node_modules`, `vendor`, `bower_components`, `target`, `dist`, `build`, `out`, `coverage`, `tmp`, `temp`, `cache`, `storage`, `venv`, `env`, `__pycache__`, `site-packages`, `Pods`, `DerivedData`;
- o que você excluir (veja [Regras](#regras)).

A busca nunca entra num projeto: as subpastas de um projeto (os pacotes de um monorepo, por exemplo) não viram sites separados. Links simbólicos são seguidos, mas cada pasta real é visitada uma vez só, então um link que aponta para cima não cria laço nem site duplicado.

## Monitorar uma pasta

Pelo app: **Configurações → Diretórios monitorados → Monitorar um diretório**. Escolha a pasta e as regras.

![Diretórios monitorados](screenshot:parked-folders)

Pela CLI (sem caminho, vale a pasta atual):

```bash
locdev park
locdev park ~/Development
locdev park ~/Clientes --depth 2 --exclude 'arquivo-*'
```

Não é possível monitorar:

- uma pasta que já é um projeto (cadastre-a com `locdev add`);
- uma pasta dentro de outra monitorada, ou que contém uma (cada projeto só pode ser encontrado por um caminho);
- a raiz de um disco.

## Regras

![Monitorar um diretório](screenshot:park-folder)

| Regra                                  | Opções                                                                                    | Padrão              |
| -------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------- |
| Procurar projetos                      | nas pastas dela (1), até 2 ou 3 níveis abaixo                                             | 1                   |
| Quando surge um projeto novo           | **Transformar em site** ou **Sugerir** (aparece na lista e você adiciona)                 | Transformar em site |
| Quando a pasta de um projeto é apagada | **Remover o site** ou **Manter, marcado como ausente**                                    | Remover o site      |
| Ignorar pastas                         | nomes ou caminhos relativos, `*` casa com qualquer coisa (`arquivo-*`, `clientes/antigo`) | nenhuma             |

Para mudar as regras, rode `locdev park` de novo na mesma pasta: só o que você passar muda.

```bash
locdev park ~/Development --suggest          # projetos novos só são sugeridos
locdev park ~/Development --keep-missing     # site de pasta apagada fica
locdev park ~/Development --no-exclude       # limpa a lista de exclusões
```

Com profundidade 2 ou 3, a busca desce só por pastas que não são projetos: em `~/Clientes/acme/loja`, `acme` precisa não ser um projeto para `loja` ser encontrado.

## Sites encontrados

Cada projeto aparece na lista do diretório com um estado:

| Estado (app)     | CLI                    | Significa                                                                |
| ---------------- | ---------------------- | ------------------------------------------------------------------------ |
| site             | `site`                 | virou site por causa desta pasta                                         |
| adicionado à mão | `site (added by hand)` | já era um site seu; continua como está                                   |
| sugerido         | `suggested`            | esperando você adicionar (**Adicionar** no app, ou `locdev add <pasta>`) |
| ignorado         | `ignored`              | você não quer este projeto como site                                     |
| não adicionado   | `not added`            | não deu para criar o site (o motivo aparece ao lado)                     |

```bash
locdev park list
```

```text
/home/voce/Development  watching · depth 1 · new: become sites · gone: removed · scanned 12s ago
  api        api.test                      Rust                site
  dashboard  dashboard.test                Next.js             site
  loja       loja.test                     Laravel             site
  portfolio  portfolio.test                HTML5               site
```

Sites novos começam **parados** e sem início automático; nada roda sem você pedir. A porta, o comando e o runtime vêm da detecção, como num `locdev add`.

### Nomes repetidos

O nome do site é o da pasta. Se ele já existe, o LocDev usa o nome da pasta de cima (`loja-clientes` para `~/Clientes/loja`, ou `loja-development` para `~/Development/loja`) e, se também estiver ocupado, um número (`loja-2` até `loja-9`). O motivo aparece ao lado do projeto. Uma vez cadastrado, o nome fica: varreduras seguintes não renomeiam sites.

### Ignorar um projeto

```bash
locdev park ignore ~/Development/rascunho
locdev park unignore ~/Development/rascunho
```

Ignorar remove o site (se ele veio do diretório) e impede que ele volte. Remover um site que veio de um diretório monitorado pelo app ou com `locdev remove` faz o mesmo: ele passa a ser ignorado ali, senão voltaria na próxima varredura.

## Varreduras e eventos

O LocDev varre a pasta quando você a monitora ou muda as regras, quando algo muda nela (eventos de arquivos, agrupados até a pasta ficar 1,5 s quieta, para um `git clone` ou um `laravel new` contar como uma mudança só) e, para recuperar eventos perdidos, a cada 5 minutos. Para forçar:

```bash
locdev park scan
locdev park scan ~/Development
```

Os eventos vêm só da pasta monitorada e das pastas intermediárias (que não são projetos); o LocDev não observa o interior dos projetos, para não gastar o limite de observadores do sistema com `node_modules`.

Se a própria pasta monitorada sumir (um disco externo desconectado, uma pasta renomeada), nenhum site é removido: o diretório mostra "pasta não encontrada" e os sites voltam a funcionar quando ela reaparece.

## Pausar e parar de monitorar

```bash
locdev park disable ~/Development    # sem varreduras nem eventos; os sites ficam
locdev park enable ~/Development
locdev park remove ~/Development     # os sites que vieram dela saem do LocDev
locdev park remove ~/Development --keep-sites
```

`park remove` para e remove só os sites que vieram do diretório; sites que você adicionou à mão continuam. Com `--keep-sites`, os sites ficam como sites comuns. Em nenhum caso uma pasta ou arquivo é apagado.

## Onde fica

A lista de diretórios e as regras ficam em `config/parked.toml` na pasta de configuração do LocDev ([Configuração](/docs/configuration)). Cada site que veio de um diretório guarda a pasta de origem em `parked` no `projects.toml`.

## Problemas conhecidos

- **"is itself a project":** você tentou monitorar a pasta de um projeto. Cadastre-a com `locdev add` ou monitore a pasta acima.
- **"overlaps the parked folder":** a pasta está dentro de outra monitorada (ou contém uma). Aumente a profundidade da de cima, em vez disso.
- **"rescanned every 5 minutes" / "watching unavailable":** o sistema recusou os eventos de arquivos (no Linux, normalmente o limite `fs.inotify.max_user_watches`). Tudo continua funcionando, só com atraso de até 5 minutos.
- **Um projeto não aparece:** confira se ele tem um dos arquivos que a detecção reconhece, se não está numa pasta ignorada, se a profundidade alcança ele e se ele não está com o estado "ignorado".
