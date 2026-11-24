# Atualizar o LocDev

Em **Configurações → Atualizações** o app procura versões novas, instala com a assinatura conferida e volta para uma versão anterior se a nova der problema. O daemon e a CLI vêm dentro do app; uma CLI instalada sozinha se atualiza pelo script de instalação. `locdev update` mostra o que foi publicado.

![Atualizações: o app, o daemon e a CLI, e as versões anteriores.](screenshot:updates)

> [!WARNING]
> Ainda não há nenhuma release publicada do LocDev. Até a primeira, a tela mostra que não encontrou release, e o restante desta página descreve como a atualização funciona quando houver uma. As releases precisam ficar num repositório público: o app e o daemon as leem sem token.

## O app

O app usa o atualizador do Tauri. Cada release traz um `latest.json` e os arquivos de atualização **assinados pela chave de release do LocDev**; o app só instala o que a assinatura (a chave pública vem dentro do app) confirma, e recusa o resto. A interface só escolhe a versão: o endereço é montado pelo próprio app, sempre nas releases do LocDev no GitHub.

| Sistema                 | Atualização pelo app                                                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Windows                 | Sim (instalador NSIS, sem pedir administrador). O daemon para antes de instalar, porque o Windows não substitui um programa em execução. |
| macOS                   | Sim.                                                                                                                                     |
| Linux, AppImage         | Sim.                                                                                                                                     |
| Linux, `.deb` ou `.rpm` | Não: o atualizador do Tauri só substitui AppImages. A tela avisa e leva à página da release, de onde você baixa o pacote novo.           |

Uma cópia de desenvolvimento (`make app`) também não se atualiza.

- **Procurar atualizações automaticamente** (ligado por padrão): ao abrir o app e uma vez por dia. Uma versão nova aparece como aviso, com o botão para abrir a tela.
- **Instalar atualizações automaticamente** (desligado por padrão): baixa e instala assim que encontra. Em nenhum caso o LocDev reinicia sozinho.

Depois de instalar, **Reiniciar o LocDev** fecha o app, para o daemon e abre a versão nova, que inicia o daemon que veio com ela. Os sites param por um instante.

Essas duas opções ficam no app (por janela, no armazenamento local), não no `settings.toml`.

## O daemon e a CLI

O daemon lê as releases publicadas no GitHub, sem token, e compara com a própria versão. Cada release traz um `versions.json` com as versões do app e do daemon que ela contém (o workflow de release recusa uma tag diferente da versão do app).

```bash
locdev update
```

A resposta fica guardada por 6 horas; `--refresh` pergunta ao GitHub na hora e `--json` dá a saída para scripts.

- Uma vez por dia (não no modo offline), o daemon confere sozinho e manda **uma notificação por release** nova. Desligue em **Notificar sobre novas releases** ou com `[automation] updates = false` no `settings.toml`.
- O daemon **nunca instala nada**. Com o app, atualizar o app atualiza o daemon e a CLI. Se o daemon em execução for diferente do que veio com o app (por exemplo, logo depois de atualizar), a tela mostra as duas versões e o botão **Reiniciar o daemon**.
- A CLI avulsa se atualiza com o mesmo script da instalação, que confere o arquivo contra o `SHA256SUMS` da release:

```sh
curl -fsSL https://github.com/trbros/locdev/releases/latest/download/install.sh | sh
```

```powershell
irm https://github.com/trbros/locdev/releases/latest/download/install.ps1 | iex
```

`LOCDEV_VERSION=v0.14.0` instala uma versão específica (também serve para voltar).

O repositório consultado é `[updates] repo` no `config.toml` (padrão `trbros/locdev`); veja [Configuração](/docs/configuration).

## Voltar para uma versão anterior

Em **Versões anteriores**, **Voltar para esta versão** instala uma release mais antiga pelo mesmo caminho de uma atualização: o `latest.json` daquela release, com a mesma conferência de assinatura. Seus sites, configurações e dados ficam onde estão.

Uma versão mais antiga não conhece o que as mais novas acrescentaram: ela ignora as configurações que não conhece e pode descartá-las ao salvar, e a fila de tarefas e as notificações gravadas num formato mais novo são deixadas de lado numa cópia (`.broken`) em vez de apagadas. Se estiver em dúvida, pare o daemon e faça backup da pasta de configuração antes.

## Atualizar à mão

Para pacotes `.deb`/`.rpm`, uma compilação local ou quando preferir:

```bash
locdev version
locdev stop
```

Com o daemon parado, copie `config.toml`, `projects.toml` e os dados que quiser guardar (caminhos em [Configuração](/docs/configuration)). Os dados da CA local contêm material privado: não os envie a issues ou chats públicos. Baixe o pacote da release, confira o SHA-256 no `SHA256SUMS` e instale com o mesmo instalador usado da primeira vez. Para a CLI avulsa, troque `locdev` e `locdevd` juntos. Para uma compilação local, siga [Instalação](/docs/installation).

Se o caminho do executável mudar, revise o [início automático](/docs/service). Uma atualização não exige reinstalar o DNS nem confiar de novo na CA.

## Depois de atualizar

```bash
locdev start
locdev version
locdev doctor
locdev list
```

O início pode reabrir os projetos com `auto_start`. Confira as versões do cliente e do daemon e teste um projeto conhecido. Veja [Solução de problemas](/docs/troubleshooting) se algo falhar.

## Limitações

- Não validado com uma release real: ainda não existe nenhuma. Foram testados a leitura das releases (contra uma API falsa) e a tela com o backend simulado; o download e a troca do app pelo atualizador do Tauri dependem da primeira release assinada.
- `.deb` e `.rpm` não se atualizam pelo app.
- A volta de versão só alcança releases que já traziam o `latest.json` assinado.
