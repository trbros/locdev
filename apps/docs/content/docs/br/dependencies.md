# Dependências, jobs e notificações

## Dependências do site

![Aba Dependências de um site](screenshot:dependencies)

A aba **Dependências** de cada site (ou `locdev deps list`) lê os arquivos do projeto: `package.json` com `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock` (Yarn 1 e 2+) ou `bun.lock`, e `composer.json` com `composer.lock`. Para cada pacote, ela separa três versões:

| Coluna     | De onde vem                                                                                          |
| ---------- | ---------------------------------------------------------------------------------------------------- |
| Declarado  | a faixa do manifesto (`^1.2.0`)                                                                      |
| Instalado  | o que está em `node_modules`/`vendor`; sem isso, o do lockfile (um `*` marca quando os dois diferem) |
| Disponível | a versão estável mais nova; abaixo, a mais nova que a sua faixa permite                              |

Listar nunca usa a rede: os dados dos registros vêm da última verificação. O `bun.lockb` antigo é binário e não é lido (o Bun 1.2+ grava `bun.lock` em texto).

## Verificação

**Verificar agora** (ou `locdev deps check`) consulta o registro npm, o Packagist e o [osv.dev](https://osv.dev) (que reúne os avisos do GitHub, do npm e do PHP). Pré-releases (alpha, beta, RC, nightly) nunca são sugeridas.

A compatibilidade de uma atualização é classificada, **nunca garantida**:

| Resultado     | Quando                                                                                                                                   |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| sem conflitos | nenhuma verificação encontrou problema (`engines`, peers, PHP); não é uma garantia                                                       |
| possivelmente | um major novo (ou um minor da série 0.x), algo que não pôde ser verificado, ou uma versão marcada como obsoleta                          |
| incompatível  | um requisito não é atendido: `engines.node` contra o Node do site, um peer contra a versão instalada, `require.php` contra o PHP do site |
| indeterminado | faltam dados                                                                                                                             |

O Node.js e o PHP usados nessa análise são os que o site realmente usa (pin, arquivos de versão, default ou `PATH`).

## Atualizar

Selecione pacotes e clique em **Atualizar** (ou `locdev deps update <pacote>…`). Antes de qualquer coisa rodar, o LocDev mostra:

- versão atual → proposta, o tipo de salto e a compatibilidade com os motivos;
- links para as notas de release, o repositório e a página no registro;
- o **comando exato** que vai rodar, com o gerenciador e o lockfile do projeto;
- os arquivos que devem mudar.

![Plano de atualização antes de confirmar](screenshot:dependency-update)

Por padrão, a atualização fica dentro da sua faixa (`npm update`, `pnpm update`, `composer update --with-dependencies`). **Permitir majors** troca a faixa no manifesto mantendo o estilo (`^`, `~` ou exata). Scripts de instalação (`postinstall`…) **não rodam** a menos que você marque a opção; nenhuma migração ou outro comando é executado. O resultado mostra a saída do gerenciador e os arquivos que mudaram.

Os comandos rodam sem shell (programa e argumentos separados), na pasta do site e com os runtimes do site no `PATH`.

## Monitoramento automático

Em **Configurações → Automações** (ou `locdev settings set automation.audit <valor>`) você escolhe: desativado, só manual, a cada 6 ou 12 horas, diário (padrão) ou semanal.

![Configurações → Automações](screenshot:automations)

- As verificações entram numa **fila persistente** (`data/state/jobs.json`). Se o computador estava desligado na hora, ela roda quando ele volta; um job interrompido no meio volta para a fila.
- Os sites são escalonados (não começam juntos), cada um consulta os pacotes um por vez, e há um limite de requisições por segundo somando todos os jobs.
- Os dados dos registros ficam em cache por 6 horas. Falhas de rede e `429` são repetidas com espera crescente (ou o `Retry-After` do registro).
- Uma verificação já na fila não é duplicada.
- Ao **desativar** (ou ligar o **modo offline**), os jobs automáticos na fila são cancelados e um que esteja rodando para no próximo pacote. Nenhuma consulta de monitoramento continua.
- "Verificar agora" continua funcionando mesmo com o monitoramento desativado: é um pedido seu, não monitoramento.

`locdev jobs` lista a fila; `locdev jobs cancel|pause|resume|retry <id>` controla um job. A mesma fila roda as instalações de runtime.

## Notificações

![Notificações](screenshot:notifications)

O LocDev avisa sobre atualizações disponíveis, vulnerabilidades, servidores que falham e o resultado do que você iniciou. As notificações ficam gravadas no disco: sobrevivem ao fechamento do app, ao reinício do daemon e do computador, até você dispensá-las. Cada uma tem uma chave: a mesma atualização ou o mesmo aviso nunca aparece duas vezes (uma versão mais nova gera outra).

- **Pendentes / Histórico:** marcar como lida, dispensar, abrir o recurso (o site, a aba de dependências ou a saída do servidor).
- **Preferências:** categorias que geram notificações (inclusive **e-mails capturados** pelo Mailbox) e as **notificações do sistema**, ligadas por padrão. O próprio daemon as mostra sempre que nenhuma janela do LocDev está na frente: app escondido na bandeja, fechado ou atrás de outras janelas. O que chegou enquanto nada podia mostrá-las aparece quando o computador volta (várias de uma vez viram um resumo).
- Com a janela do app na frente, as novas aparecem como toasts dentro dele, nunca as duas coisas: o app avisa ao daemon quando está em foco.
- O [ponto colorido na bandeja](/docs/app) mostra o que está sem ler.
- Tudo é local: nenhuma notificação passa por serviço externo.

```bash
locdev notifications            # pendentes
locdev notifications --all      # inclui dispensadas
locdev notifications read       # tudo lido
locdev settings set notifications.native on
locdev notifications test
```
