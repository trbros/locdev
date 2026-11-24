# Variáveis de ambiente e secrets

O LocDev guarda variáveis que você quer usar em vários projetos (chaves do Stripe, DSN do Sentry, tokens de APIs) e as entrega aos processos dos sites **sem alterar os arquivos `.env`**. Arquivos só mudam quando você pede, com `locdev env sync`.

![A aba Environment de um site: variáveis aplicadas, arquivos .env e conflitos.](screenshot:site-environment)

## Onde ficam

| O quê                                                      | Onde                                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Perfis, nomes, valores não secretos, escolhas de cada site | `config/env.toml` (`0600`)                                                                             |
| Valores secretos                                           | `data/state/env-vault.json`, cifrados com ChaCha20-Poly1305                                            |
| Chave do cofre                                             | keyring do sistema (Secret Service no Linux, Keychain no macOS, Gerenciador de Credenciais no Windows) |
| Histórico de mudanças                                      | `data/state/env-history.json`, **sem valores**                                                         |
| Backups de `.env`                                          | `data/state/env-backups/<site>/`, cifrados com a mesma chave                                           |

Cada valor secreto é selado junto com o próprio nome (dados associados do AEAD): um valor copiado para outra chave do arquivo não abre. Se a chave sumir do keyring, o LocDev **não cria outra** (isso apagaria os secrets na prática): ele avisa no `doctor`, as variáveis comuns continuam funcionando e as secretas deixam de ser injetadas.

Máquinas sem keyring (um servidor sem sessão gráfica, por exemplo) podem guardar a chave num arquivo:

```toml
# config.toml
[env]
key_store = "file"   # env-vault.key (0600) ao lado deste arquivo
```

Nesse modo, o arquivo da chave é tão sensível quanto os próprios secrets, e o `doctor` mostra um aviso.

Por padrão, uma variável nova é **secreta**. A exceção são os prefixos que os frameworks embutem no código do navegador (`NEXT_PUBLIC_`, `VITE_`, `NUXT_PUBLIC_`, `PUBLIC_`, `REACT_APP_`, `GATSBY_`, `EXPO_PUBLIC_`, `VUE_APP_`): esses são públicos e **não podem** ser secretos. Também não podem ser compartilhadas variáveis que o LocDev ou o sistema controlam (`PATH`, `HOME`, `PORT`, `HOST`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `LOCDEV_*`…).

## Perfis e herança

- **global**: todo site recebe, a menos que desligue _Receber as variáveis globais_ (`locdev env use <site> --global false`).
- **Outros perfis** (Laravel, Staging, um cliente…): valem só nos sites que os ativam. A ordem é global primeiro, depois os perfis na ordem em que foram ativados; **o último vence**, e o que foi sobrescrito aparece como `overridden`.
- Uma chave pode ser desligada num site (`--disable KEY`), qualquer que seja o perfil.

## Injeção e conflitos

Ao iniciar um site, o daemon resolve as variáveis e as coloca **só no ambiente do processo**. Nada vai para argumentos de linha de comando, logs ou arquivos temporários.

Laravel (phpdotenv), Next.js (`@next/env`), Vite, Nuxt e o pacote `dotenv` dão preferência a uma variável que já está no ambiente do processo sobre o valor do arquivo. Por isso, injetar uma chave que o projeto já define **trocaria silenciosamente o valor do projeto**. O LocDev nunca faz isso: se a chave existe num arquivo que o servidor de desenvolvimento lê, ela é pulada, e o log do site registra:

```text
[LocDev] Warning: STRIPE_SECRET_KEY already exists in .env.local
[LocDev] Skipping shared variable STRIPE_SECRET_KEY (profile global).
```

O que é lido depende do framework (`locdev env check` mostra a ordem de cada arquivo):

| Framework              | Arquivos lidos em desenvolvimento (o primeiro vence)                                                                    |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Next.js                | `.env.development.local`, `.env.local`, `.env.development`, `.env`                                                      |
| Vite, Astro, SvelteKit | `.env.development.local`, `.env.development`, `.env.local`, `.env`                                                      |
| Nuxt                   | `.env`                                                                                                                  |
| Laravel                | `.env` (com `bootstrap/cache/config.php`, o Laravel ignora o ambiente até `php artisan config:clear`, e o LocDev avisa) |
| PHP/WordPress          | `.env` só com `vlucas/phpdotenv`, `symfony/dotenv` ou `roots/wp-config` no `composer.json`                              |
| Node.js                | `.env` só com `dotenv`; o arquivo de `node --env-file=…` quando o comando usa                                           |
| Não reconhecido        | `.env.local` e `.env`, por precaução                                                                                    |

Arquivos que o servidor não lê (`.env.example`, `.env.production`, `.env.test`) aparecem como conflito informativo e não impedem a injeção. Para pular também nesse caso:

```bash
locdev settings set env.conflicts any_file
```

Também são puladas, com aviso, chaves que já estão no ambiente com que o daemon inicia processos e chaves que o próprio LocDev define para o processo (como `APP_URL` no Vite do Laravel). Um secret com prefixo público é **bloqueado**.

Mudanças chegam a um processo em execução só depois de reiniciá-lo; a aba **Ambiente** do site avisa quando o processo está com um conjunto antigo. Variáveis públicas (`NEXT_PUBLIC_`, `VITE_`…) são embutidas no código na compilação: reinicie o servidor de desenvolvimento e refaça o build de produção.

## Sincronizar com os arquivos

`locdev env sync` copia para um arquivo do projeto as variáveis que faltam nele. Sem site, vale para todos os sites cadastrados.

```bash
locdev env sync --dry-run              # todos os sites, só mostra
locdev env sync shop                   # .env do site shop
locdev env sync shop --file .env.local
locdev env sync shop --file .env.example
```

- Só **acrescenta** linhas no fim, sob um comentário do LocDev. Comentários, ordem, BOM e fins de linha (`\r\n`) do arquivo ficam como estavam.
- Pula chaves que o arquivo já tem e chaves definidas em outro arquivo que o servidor lê.
- **Nunca** copia secrets para `.env.production`/`.env.staging` (nem para os `.local` deles).
- `.env.example` (e `.sample`, `.dist`, `.template`) recebe só os nomes, com valor vazio, secretos ou não, porque esses arquivos costumam ir para o Git.
- A prévia mascara os secrets. A gravação só acontece se o arquivo e as variáveis ainda forem os da prévia; senão, o LocDev pede uma nova revisão.
- Antes de gravar, guarda uma cópia cifrada do arquivo (até 20 por site): `locdev env backups shop` e `locdev env restore <id> shop`. Restaurar também guarda o arquivo atual antes.
- Grava de forma atômica, mantém as permissões do arquivo e recusa links simbólicos e arquivos que não sejam UTF-8.

## Comandos

```bash
locdev env list                         # variáveis (secrets mascarados) e o cofre
locdev env add STRIPE_SECRET_KEY        # o valor vem da entrada padrão (fora do histórico do shell)
locdev env add SENTRY_DSN https://… --plain
locdev env add QUEUE_CONNECTION redis --plain --profile laravel
locdev env show STRIPE_SECRET_KEY --reveal
locdev env remove SENTRY_DSN
locdev env profile add laravel --label Laravel
locdev env use shop --add-profile laravel --disable SENTRY_DSN
locdev env check shop                   # o que o processo recebe, arquivos e conflitos
locdev env conflicts shop
locdev env history                      # mudanças, sem valores
```

`--reveal` pede a senha do LocDev quando `locdev security` exige uma, e fica registrado no histórico. A senha protege o que passa pelo LocDev, não o arquivo do cofre.

## No app

- **Settings → Environment**: variáveis com busca e filtro por perfil, revelar/ocultar secrets, criar, editar e remover (com confirmação), perfis, política de conflitos, histórico e sincronização em lote.
- **Site → Environment**: perfis do site e sua ordem, o estado de cada variável (`injected`, `skipped`, `overridden`, `off`, `blocked`) com o motivo, interruptor por chave, conflitos, os arquivos `.env` com a ordem de leitura, sincronização e backups.

## Limites

- A injeção vale para processos iniciados pelo LocDev. Um `php artisan` ou `npm run` rodado no seu terminal não recebe as variáveis.
- Só os arquivos `.env*` da raiz do projeto são lidos; em monorepos, cada site cadastrado é analisado na sua própria pasta.
- O keyring foi testado de verdade no Linux (Secret Service). Keychain e Gerenciador de Credenciais do Windows usam a mesma biblioteca, mas ainda não foram validados nesses sistemas.
