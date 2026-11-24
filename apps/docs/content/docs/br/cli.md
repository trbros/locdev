# CLI

`locdev` controla o daemon pelo terminal: tudo o que o app faz também funciona daqui, porque os dois falam com o mesmo daemon. Todos os comandos aceitam `--json` para saída legível por máquina, e `locdev <comando> --help` mostra as opções.

Os comandos de site (`up`, `down`, `reload`, `logs`, `info`, `open`, `secure`, `dev …`) aceitam o nome do site ou, sem nome, usam o site registrado que contém o diretório atual:

```bash
cd ~/Code/shop
locdev up            # o mesmo que locdev up shop
locdev logs -f
locdev secure
```

Comandos marcados **admin** mostram a alteração exata (o script) e pedem confirmação antes de usar privilégios de administrador; `--yes` pula a pergunta. Nada é instalado no sistema sem esse passo.

## Interface de terminal (TUI)

`locdev` sozinho, num terminal interativo, abre a TUI (o mesmo que `locdev tui`; `--light` usa as cores claras). Ela tem a cara do app: barra lateral com as páginas, os sites e os serviços com seus LEDs, o estado do proxy, do DNS, do HTTPS e do Mailbox embaixo, e a página no centro (Dashboard, Sites, Serviços, Runtimes, Processos, Mailbox, Atividade, Doctor e Sobre, que mostra a versão da CLI e do daemon).

| Tecla         | O que faz                                                            |
| ------------- | -------------------------------------------------------------------- |
| `↑` `↓` `tab` | Move na lista; `tab` alterna entre a barra lateral e a página        |
| `enter`       | Abre a página, a saída do site ou serviço, a mensagem                |
| `s` · `R`     | Inicia/para · reinicia o site, serviço ou processo selecionado       |
| `o` · `l`     | Abre o site no navegador · mostra a saída                            |
| `ctrl+p`      | "Ir para ou fazer": páginas, sites, ações e todos os comandos da CLI |
| `:`           | Roda qualquer comando `locdev` aqui mesmo, com confirmações e senhas |
| `/` · `?`     | Filtra as listas · mostra todas as teclas                            |
| `q`           | Sai (o daemon e os sites continuam rodando)                          |

O que não tem tela própria na TUI roda pelo `:` (por exemplo `:git commit -m "…"`), que devolve o terminal ao comando e volta para a TUI ao final. Sem terminal interativo (num pipe, ou com `--json`), `locdev` sem comando mostra a ajuda.

## Códigos de saída

| Código | Quando                                                                        |
| ------ | ----------------------------------------------------------------------------- |
| `0`    | Sucesso.                                                                      |
| `1`    | Erro (a mensagem diz o que fazer). Também quando o `doctor` encontra um erro. |
| `3`    | `locdev status` com o daemon parado.                                          |

## Referência

A referência abaixo é gerada a partir das definições da própria CLI (`make cli-catalog`); a página **Ferramentas → Command line** do app mostra o mesmo catálogo, com busca. Um teste falha se esta cópia ficar desatualizada.

<!-- generated:cli -->
