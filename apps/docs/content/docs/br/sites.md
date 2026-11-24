# Sites e processos

Um **site** é uma pasta registrada no LocDev, com um nome, um comando de servidor de desenvolvimento e uma porta. O registro fica em `projects.toml`; remover um site do LocDev nunca apaga os arquivos dele.

Para não cadastrar um por um, monitore a pasta onde ficam seus projetos: cada projeto dentro dela vira um site e a lista acompanha o que é criado e apagado ([Diretórios monitorados](/docs/parked-folders)).

## Detecção automática

Ao adicionar uma pasta, o LocDev lê os manifestos, lockfiles e arquivos de configuração (sem executar nada do projeto) e escolhe o adaptador que sabe iniciá-lo: Laravel, Next.js, Django, Rails, Vite e mais 40 outras stacks. O comando sugerido usa host `127.0.0.1` e a porta do site, e o gerenciador de pacotes (`npm`, `pnpm`, `yarn` ou `bun`) segue o lockfile. A lista completa e as regras estão em [Frameworks](/docs/frameworks).

Você pode trocar o comando a qualquer momento:

```bash
locdev edit blog --command "pnpm dev --port {port}"
```

## Portas e variáveis de ambiente

- `{port}` no comando é trocado pela porta do site.
- O processo recebe `PORT`, `HOST=127.0.0.1`, `LOCDEV=1` e `LOCDEV_PROJECT=<nome>`.
- Se você não informar uma porta, o LocDev escolhe a primeira livre na faixa `ports` do [config.toml](/docs/configuration) (8100–8999 por padrão).

## Ciclo de vida

```bash
locdev up blog       # inicia
locdev down blog     # para
locdev reload blog   # reinicia
locdev logs blog -f  # acompanha a saída
```

- Cada site roda no próprio grupo de processos (Unix) ou árvore (Windows). **Parar encerra tudo o que o comando abriu**, por exemplo o `node` iniciado pelo `npm`.
- O LocDev envia `SIGTERM` e espera até 8 segundos. Se o processo não terminar, envia `SIGKILL`. No Windows, usa `taskkill /T /F`.
- Um site com `--auto-start` (ou _Iniciar com o LocDev_ no app) sobe junto com o daemon.
- A saída vai para `logs/sites/<site>.log`, que gira ao chegar no tamanho de `[logs] file_mb` (5 MB por padrão); o daemon mantém as últimas linhas em memória (`[logs] lines`, 2000 por padrão) e as relê do arquivo depois de reiniciar.

## Endereços de um site

| Campo  | Exemplo                 | O que é                                                          |
| ------ | ----------------------- | ---------------------------------------------------------------- |
| URL    | `https://blog.test`     | O endereço preferido: HTTPS quando a CA é confiável, senão HTTP. |
| HTTPS  | `https://blog.test`     | Sempre disponível quando o HTTPS está ligado.                    |
| Direto | `http://127.0.0.1:8100` | O servidor de desenvolvimento, sem o proxy.                      |

Subdomínios funcionam automaticamente: `api.blog.test` também chega ao site `blog`.

## Sites estáticos

Uma pasta com `index.html` (ou `public/index.html`) e sem framework é um site estático. O LocDev o roda com o próprio servidor estático, `locdevd static --port {port}` (`--root public` para `public/index.html`), então não precisa de Python nem de Node:

- escuta só em 127.0.0.1, atrás do proxy como qualquer site (`<site>.test`, HTTPS);
- URLs limpas: `/sobre` serve `sobre.html` ou `sobre/index.html`;
- arquivo que não existe recebe o seu `404.html`, quando houver;
- `Cache-Control: no-cache`, para a edição aparecer ao recarregar;
- nada fora da pasta é servido (`..`, caminhos absolutos e escapes codificados são recusados).

Para usar outro servidor, troque o comando: `locdev edit <site> --command "…"`.
