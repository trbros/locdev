# Tour pelo app

O app é organizado em páginas principais, cada uma com as suas subpáginas. À esquerda fica a barra lateral; embaixo, uma barra de status mostra o daemon, o proxy, o DNS e o HTTPS de relance (cada trecho leva à configuração correspondente).

## Abertura

Ao abrir, uma janela pequena mostra o símbolo do LocDev se desenhando e o **traço de inicialização**: daemon → proxy → DNS → HTTPS. Cada nó acende com o estado real lido do daemon (verde quando está tudo certo, âmbar quando funciona com ressalvas, como HTTPS ainda não confiável, vermelho quando falhou). Quando a interface termina de carregar os dados, o satélite do símbolo "sai pela órbita" e a janela principal entra no lugar. A splash nunca segura a abertura por mais de 8 segundos, e respeita o tema e a preferência de animações.

## Navegação

![O Dashboard: o ambiente numa faixa, os sites no ar e o que precisa de atenção.](screenshot:dashboard)

A barra lateral tem dois modos, **expandida** (ícones e nomes) e **recolhida** (só ícones, com dicas). Alterne com `Ctrl` `B` ou pelo botão no rodapé dela; a escolha fica salva. Em janelas estreitas ela fica sempre recolhida. As setas `↑` `↓` percorrem os itens.

![Barra lateral recolhida](screenshot:sidebar-collapsed)

Só **Ferramentas** tem submenu na barra lateral. As demais subpáginas ficam dentro da própria página: abas em Runtimes e em cada site, navegação lateral em Configurações. Toda página mostra o caminho (_breadcrumbs_) e botões de voltar/avançar (`Alt` `←` / `Alt` `→`, ou os botões laterais do mouse).

| Página        | O que tem                                                                                                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Dashboard     | Estado do ambiente, ambientes ativos, alertas (sites com falha, Doctor, runtimes) e atividade recente                                                                                |
| Sites         | Lista de sites e o detalhe de cada um: Visão geral, Servidor, Dependências, Ambiente, Config, Email Studio, Deploy, Git, GitHub (com remoto no GitHub) e Configurações               |
| Runtimes      | Visão geral (o que cada site usa e por quê), Node.js, PHP e gerenciadores de pacotes                                                                                                 |
| Services      | Instalações e instâncias de bancos e caches ([Serviços](/docs/services))                                                                                                             |
| Ferramentas   | [Novo projeto e Instaladores](/docs/new-project), Processos e logs, [Database](/docs/database), [Mailbox](/docs/mailbox), [Actions builder](/docs/actions), Linha de comando, Doctor |
| Notificações  | Pendentes, histórico e preferências ([detalhes](/docs/dependencies))                                                                                                                 |
| Configurações | Visão geral, Daemon, Proxy, DNS, Diretórios monitorados, HTTPS, Automações, Frameworks, Ambiente, Segurança, Integrações, Atualizações, Aparência, Atalhos e Sobre                   |

## Barra de título

A barra de cima é do próprio app, não do sistema: à esquerda, um LED com o estado do daemon e onde você está (`locdev › sites · loja`); no meio, a busca, que abre a paleta de comandos; à direita, minimizar, maximizar e fechar. Arraste a barra para mover a janela e dê dois cliques para maximizar. Fechar só esconde o app na bandeja, como antes. No macOS ficam os botões do próprio sistema, sobre a barra.

## Sites

![Tela de sites](screenshot:sites)

- **Lista à esquerda:** os sites ficam agrupados em _rodando_ e _parado_, com um LED de estado, o tipo do projeto e a porta. Filtre com `Ctrl` `F`.
- **Abas:** cada site abre numa aba, como num navegador. Clicar num site da lista mostra ele na aba da frente (ou vai para a aba que já o tem); `Ctrl`+clique ou o clique do meio abre numa aba nova, e **+** abre uma aba vazia com todos os sites para escolher. Arraste as abas para reordenar; o clique do meio fecha. Cada aba lembra a subpágina em que estava (Visão geral, Servidor, Git…), e as abas voltam ao reabrir o app.
- **Barra de endereço:** o endereço do site, com o cadeado (verde quando o HTTPS é confiável; âmbar leva a confiar na CA), o framework, o estado e os controles (iniciar, parar, reiniciar, abrir no navegador, revelar a pasta e o menu **⋯**). Clique no endereço e digite parte do nome de um site para ir até ele: `Enter` abre na aba atual, `Ctrl` `Enter` numa aba nova. Um subdomínio (`api.loja.test`) leva ao site dele.
- **Visão geral:** caminho, comando, processo e os runtimes que o site usa, com a origem de cada versão (pin, `.nvmrc`, `engines`, default) e avisos de faixa não atendida ou versão sem suporte.
- **Servidor:** a saída do processo ao vivo, com stderr em âmbar e as notas do próprio LocDev em itálico. Dá para limpar a visualização, copiar a saída e pausar o auto-scroll.
- **Configurações:** comando, porta, versões de runtime e de gerenciador de pacotes, início junto com o LocDev, e os domínios do site com o estado do HTTPS.

![Configurações de um site](screenshot:site-settings)

Quando um processo falha, o motivo aparece em destaque e leva à saída:

![Um WordPress que falhou por falta da extensão mysqli.](screenshot:site-failed)

Clique com o botão direito em um site da lista, ou use o menu **⋯**, para as mesmas ações: abrir no navegador, iniciar/parar, reiniciar, abrir em nova aba, editar, revelar a pasta, copiar URL ou caminho e remover. Remover pede confirmação e **nunca apaga arquivos**.

## Bandeja

Enquanto o app está aberto, o LocDev fica na bandeja do sistema. No Linux e no Windows, o ícone fica em brasa com o daemon rodando e cinza com ele parado; no macOS ele é monocromático, como os demais ícones da barra de menus, e o estado aparece no menu. O menu mostra quantos sites estão no ar e traz, para cada site, _Abrir no navegador_, _Iniciar_/_Parar_ e _Reiniciar_, além de abrir o app e iniciar, parar ou reiniciar o daemon.

### O ponto colorido

Quando há algo para ler, um ponto aparece no canto do ícone da bandeja. A cor diz o quê:

| Cor     | Significa                                                                 |
| ------- | ------------------------------------------------------------------------- |
| Azul    | Notificações não lidas (atualizações, falhas, resultados de jobs…)        |
| Verde   | E-mails capturados pelo [Mailbox](/docs/mailbox) que você ainda não abriu |
| Violeta | As duas coisas                                                            |

A dica do ícone e o menu dizem quantos são (por exemplo, _3 unread notifications, 1 unread email (violet dot)_), e os itens do menu abrem as Notificações ou o Mailbox. O ponto some quando tudo foi lido. No macOS, com o ponto aceso, o ícone deixa de ser monocromático para a cor aparecer.

Fechar a janela só esconde o app: ele continua na bandeja. Para sair, use _Sair do LocDev_ no menu. Sair fecha só o app; o daemon continua servindo os sites, como acontece com a CLI (`locdev stop` para o daemon). Abrir o LocDev de novo traz a janela que já estava aberta, o que também serve de caminho de volta em desktops sem bandeja.

> [!NOTE]
> No GNOME, ícones de bandeja dependem da extensão AppIndicator, que já vem ativa no Ubuntu. Em outras distros, instale a extensão _AppIndicator and KStatusNotifierItem Support_.

## Paleta de comandos

`Ctrl` `K` (ou `⌘` `K` no macOS) abre a paleta: iniciar, parar e abrir sites, controlar o daemon e ir para qualquer página ou subpágina.

![Paleta de comandos](screenshot:palette)

Os atalhos principais: `Ctrl` `K` (paleta), `Ctrl` `N` (adicionar site), `Ctrl` `1` … `7` (páginas), `Ctrl` `,` (configurações), `Ctrl` `T`/`Ctrl` `W`/`Ctrl` `Tab` (abas de site) e `Alt` `←`/`→` (voltar/avançar). Todos podem ser mudados em **Configurações → Atalhos**; a lista completa está em [Atalhos de teclado](/docs/shortcuts).

## Doctor

Em **Ferramentas → Doctor**.

![Cada verificação traz o detalhe e, quando há algo a fazer, o comando que resolve (clique para copiar).](screenshot:doctor)

## Configurações

![Configurações: visão geral](screenshot:settings)

A visão geral resume o estado de cada área; cada linha leva à subpágina dela:

- **Daemon:** processo, uptime, endpoint IPC, iniciar/parar/reiniciar, _Iniciar o LocDev no login_ e as pastas de configuração e logs;
- **Proxy:** porta e os domínios que ele atende;
- **[DNS](/docs/domains)** e **[HTTPS](/docs/https)** (com o estado de cada site em HTTPS);
- **Aparência:** idioma do app, tema, modo da barra lateral e animações (seguir o sistema, reduzidas ou completas);
- **Integrações:** a conta do GitHub, o [editor de código](/docs/editors) e as [plataformas de deploy](/docs/deploy);
- **[Atualizações](/docs/updating)** e **[Atalhos](/docs/shortcuts)**;
- **Sobre:** a logo, as versões do app, do daemon e da CLI, o commit e a data da compilação, o sistema e a arquitetura, o canal de atualização, a licença (Apache-2.0), a lista de software de terceiros com as licenças, o histórico de versões (o changelog) e os links da documentação e do repositório; logo abaixo, a CLI no `PATH`.

## Runtimes

![Runtimes: visão geral](screenshot:runtimes)

A visão geral mostra os defaults e uma tabela com o que cada site usa e de onde veio a versão. As abas Node.js, PHP e Gerenciadores listam as versões gerenciadas e detectadas, com selos de suporte e de uso. Veja [Runtimes](/docs/runtimes).

## Services

![Tela de Services: instalações do sistema e instâncias](screenshot:services)

Instalações detectadas ou baixadas pelo LocDev, e as instâncias com porta, dados e logs próprios. Veja [Serviços](/docs/services).

## Idioma

O app está em português, inglês, espanhol e japonês. Por padrão ele segue o idioma do sistema; em **Configurações → Aparência** dá para fixar um. Trocar o idioma recarrega a janela. O menu da bandeja acompanha o idioma do app depois que ele abre. Mensagens que vêm do daemon (erros, logs, resultados do doctor) e os dados dos projetos continuam em inglês.

## Tema claro e escuro

O app segue o tema do sistema ou fixa _Paper_ (claro) ou _Carbon_ (escuro). As capturas desta documentação estão no idioma da página e acompanham o tema do site.

## Acessibilidade

O app segue o WCAG 2.2 nível AA, a referência usada pela EN 301 549, pelo European Accessibility Act e pela Section 508:

- **Teclado:** tudo funciona sem mouse. `Tab` percorre os controles com o foco sempre visível, as setas andam pela barra lateral e pelas abas, e os atalhos ficam em **Configurações → Atalhos**.
- **Leitores de tela:** botões só com ícone têm nome, as abas seguem o padrão `tablist`/`tab`, e as áreas roláveis (código, mensagens) recebem foco.
- **Contraste:** textos e botões passam de 4.5:1 nos dois temas.
- **Movimento:** o app respeita `prefers-reduced-motion`, e **Configurações → Aparência** desliga as animações.

Cada versão passa por uma verificação automática (axe-core) em todas as telas, nos temas claro e escuro, incluindo a navegação só por teclado: `make test-a11y`.

## Identidade visual

O logo do LocDev é um PNG oficial, usado como é. A fonte única fica em `apps/desktop/branding/logo.png`; `pnpm icons` recorta, centraliza e redimensiona esse arquivo para gerar os ícones de Windows (`.ico`), macOS (`.icns`), Linux e Tauri, os ícones da bandeja, o `logo.png` usado no app e na splash e o `favicon.ico`.
