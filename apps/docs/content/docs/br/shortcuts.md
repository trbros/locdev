# Atalhos de teclado

Em **Configurações → Atalhos** ficam todos os atalhos do app, agrupados por categoria: dá para buscar, mudar, acrescentar uma segunda combinação, remover e voltar ao padrão. `Ctrl` `/` abre essa página.

![Atalhos: busca, categorias e as combinações de cada ação.](screenshot:shortcuts)

## Como funcionam

- **Só dentro do LocDev.** Os atalhos funcionam com uma janela do LocDev em primeiro plano. O LocDev não registra atalhos globais no sistema operacional, então nunca toma teclas de outros apps.
- **No macOS, `Ctrl` vira `⌘`.** A exceção é a troca de abas de site (`Ctrl` `Tab`), porque `⌘` `Tab` troca de app no macOS e nunca chega ao LocDev.
- **Em campos de texto** só disparam combinações com `Ctrl`/`⌘`; `Alt` com as setas e teclas sozinhas continuam editando o texto.
- **Onde valem:** alguns atalhos só existem numa página (marcados com _em Sites_): a aba de site, o filtro e as ações do site na aba da frente.
- **Onde ficam salvos:** neste computador, no armazenamento do app (não no `settings.toml`). Uma mudança vale na hora para todas as janelas do LocDev.

## Mudar um atalho

1. Clique em **+** ao lado da ação e pressione a combinação. `Esc` cancela.
2. A combinação precisa de `Ctrl`, `⌘` ou `Alt` (ou ser uma tecla de função, `F1`…`F12`), para que digitar nunca dispare um atalho.
3. Se outra ação já usa a combinação, o LocDev mostra qual e pergunta: **Usar aqui** tira a combinação da outra ação.
4. Passe o mouse sobre uma combinação para removê-la (**×**); o botão de voltar restaura o padrão daquela ação, e **Restaurar todos os padrões** desfaz tudo.

Uma combinação marcada em âmbar tem um aviso:

- **Conflito:** duas ações com a mesma combinação; só uma delas roda (a que vem antes nesta lista).
- **Reservada:** o sistema ou a edição de texto usa essa combinação, e ela pode nunca chegar ao LocDev. Exemplos: `Alt` `F4` e `Alt` `Tab` (Windows e Linux), `Ctrl` `Alt` `T` (terminal no Ubuntu), `⌘` `Q`, `⌘` `H` e `⌘` `Tab` (macOS), combinações com a tecla Windows/Super, e `Ctrl` `C`/`V`/`X`/`A`/`Z` nos campos de texto.

Atalhos embutidos (com cadeado) aparecem só para consulta: desfazer e refazer no Email Studio, e `Esc` para fechar um diálogo ou limpar um filtro.

## Padrões

| Categoria          | Ação                                                                                   | Atalho                              |
| ------------------ | -------------------------------------------------------------------------------------- | ----------------------------------- |
| Geral              | Paleta de comandos                                                                     | `Ctrl` `K`                          |
| Geral              | Configurações                                                                          | `Ctrl` `,`                          |
| Geral              | Atalhos de teclado                                                                     | `Ctrl` `/`                          |
| Geral              | Recolher ou expandir a barra lateral                                                   | `Ctrl` `B`                          |
| Geral              | Voltar / avançar                                                                       | `Alt` `←` / `Alt` `→`               |
| Navegação          | Ir para Dashboard, Sites, Runtimes, Serviços, Ferramentas, Notificações, Configurações | `Ctrl` `1` … `Ctrl` `7`             |
| Projetos           | Adicionar um site                                                                      | `Ctrl` `N`                          |
| Projetos           | Criar um projeto novo                                                                  | `Ctrl` `Shift` `N`                  |
| Projetos           | Filtrar sites (em Sites)                                                               | `Ctrl` `F`                          |
| Projetos           | Nova aba / fechar a aba (em Sites)                                                     | `Ctrl` `T` / `Ctrl` `W`             |
| Projetos           | Próxima / anterior aba de site (em Sites)                                              | `Ctrl` `Tab` / `Ctrl` `Shift` `Tab` |
| Projetos           | Abrir o site no [editor](/docs/editors) (em Sites)                                     | `Ctrl` `E`                          |
| Logs e diagnóstico | Abrir o [Debug Center](/docs/debug-center)                                             | `Ctrl` `Shift` `D`                  |

Sem atalho por padrão, para você escolher: abrir o site no navegador, mostrar a pasta do site, iniciar ou parar, reiniciar e compartilhar o site (em Sites), e abrir cada ferramenta (Linha de comando, Novo projeto, Instaladores, Banco de dados, Mailbox, Túneis, Actions builder, Processos e logs, Doctor).

A paleta de comandos, os menus e as dicas dos botões mostram sempre a combinação atual, inclusive a que você mudou.

## Limitações

- Não há atalhos globais (fora do LocDev) nem um atalho para mostrar a janela a partir de outro app.
- A TUI (`locdev` no terminal) tem as teclas próprias dela, que não mudam por aqui.
- No Linux, o atalho de abrir o terminal e os de trocar de área de trabalho variam conforme o ambiente gráfico; a lista de reservadas cobre os padrões mais comuns (GNOME/Ubuntu), não todos.
