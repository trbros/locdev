# Email Studio

A aba **Email Studio** de cada site reúne os templates e componentes de e-mail do projeto, com um editor visual de blocos, preview em tempo real e envio de teste para o [Mailbox](/docs/mailbox). Os arquivos editados são os do projeto, e o Blade continua intacto.

![Email Studio: blocos, estrutura com o código Blade preservado e o preview.](screenshot:email-studio)

```bash
locdev new email contact                   # Laravel: make:mail + view com blocos
locdev new email receipt --markdown        # Markdown mailable
locdev new email component button          # <x-mail.button>
locdev email list                          # templates, mailables e componentes
locdev email preview <caminho> --framework # renderizado pelo próprio Laravel
locdev email send-test <caminho>           # para o Mailbox, nunca para fora
```

## O que é descoberto

| Stack         | Templates                                                                                                                                                                                                                                                                                                              | Componentes                                                                                                                                                                                                                              |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel       | views em `resources/views` dentro de pastas `mail`, `mails`, `email` ou `emails`, e qualquer view que um Mailable de `app/Mail` use (`view:`, `html:`, `text:`, `markdown:`). Mostra o Mailable, as variáveis (propriedades públicas e chaves de `with:`) e o formato: Blade, Markdown (`<x-mail::message>`) ou texto. | `resources/views/components/mail/*.blade.php`, usados como `<x-mail.nome>`. As propriedades vêm do próprio `@props([...])` do Blade. Os componentes de Markdown do Laravel (`x-mail::button`, `panel`…) aparecem nos templates Markdown. |
| Outras stacks | `emails/`, `templates/emails/` (Django), `app/views/*_mailer/` (Rails), `.mjml` e React Email (`.tsx`)                                                                                                                                                                                                                 | `emails/components/*.html` com `<!-- locdev:component {"props":[...]} -->`                                                                                                                                                               |

Nada é executado para descobrir: os arquivos só são lidos. Templates MJML, React Email, ERB e Django são editados como código; os blocos visuais funcionam em Blade, Markdown mailables e HTML.

## Como a edição preserva o template

O editor trabalha com **segmentos**:

- **Blocos do LocDev** (cabeçalho, título, texto, botão, imagem, colunas, divisória, espaço, rodapé, painel, componente). Cada um fica entre dois comentários do próprio template, que não vão para o e-mail enviado:

  ```blade
  {{-- locdev:block {"type":"button","props":{"text":"Pagar","url":"{{ $url }}"},"h":"3fa1c2d0"} --}}
  <tr><td …><a href="{{ $url }}" …>Pagar</a></td></tr>
  {{-- /locdev:block --}}
  ```

  Os textos aceitam expressões Blade (`{{ $user->name }}`), que vão para o arquivo como foram escritas; o resto é escapado. Em componentes, um valor que começa com `$` vira binding (`:url="$url"`).
- **Código:** tudo o que está fora dos blocos, como `@if`, `@foreach`, `{!! !!}`, `@props` e o layout, é guardado exatamente como está. O código pode ser arrastado e editado como texto, mas **o editor visual nunca o reescreve**.

Abrir e salvar sem mudanças devolve o arquivo byte a byte. Ao mudar um bloco, só as linhas dele mudam.

**Editou no IDE?** Cada bloco guarda um hash da marcação que gerou. Se ela foi alterada fora do LocDev, o bloco aparece como _editado_ e é mantido como você escreveu, até você pedir **Regenerar a partir das propriedades**.

## Sincronização com o IDE

- **Mudanças de fora:** o app confere o arquivo a cada 2 segundos. Se ele mudou e você não tem alterações pendentes, a tela recarrega sozinha. Se tem, um aviso oferece **Carregar as deles** (descarta as suas) ou **Manter as minhas** (ao salvar, o diff mostra o que vai mudar em relação ao arquivo novo).
- **Salvar:** sempre mostra o diff antes e só grava se o arquivo ainda for o que você revisou. A versão anterior vai para o histórico, e a gravação é atômica.
- **Desfazer e refazer:** `Ctrl+Z` e `Ctrl+Y` funcionam dentro do editor.

## Preview e dados de exemplo

- **Preview estático:** o LocDev substitui os dados de exemplo (`$user->name`, `$order['id']`, `?? 'padrão'`, `config()`, `__()`, `route()`, `date('Y')`) e expande os componentes do projeto, **sem executar nada**. A lógica do Blade não é avaliada, então os dois lados de um `@if` aparecem, e a tela avisa. O bloco selecionado fica destacado no preview.
- **Preview pelo framework (Laravel):**
  - quem renderiza é o próprio Laravel, com `view()` ou o `Markdown::render` dos mailables;
  - roda num **processo separado**, com ambiente reduzido (o PHP do site, `HOME`, `APP_ENV=local` e o e-mail apontado para o Mailbox), limite de 25 segundos e saída limitada;
  - o conteúdo não salvo vai para um arquivo temporário do LocDev, nunca para o projeto;
  - exige `vendor/` instalado.
- **Visualizações:** desktop ou mobile, o HTML gerado, a versão em texto e o arquivo que seria gravado.
- **Dados de exemplo:** são sugeridos a partir das variáveis do template, editáveis em JSON e guardados pelo LocDev, fora do projeto. **Mandar teste** renderiza e manda para o Mailbox.

## Gerador

`locdev new email <nome>` segue a estrutura do framework e **nunca sobrescreve** um arquivo existente:

- **Laravel:** roda `php artisan make:mail <Nome>Mail --view=emails.<nome>` (ou `--markdown`), o gerador do próprio Laravel, e preenche a view recém-criada com um modelo em blocos. Sem PHP ou `vendor/`, cria só a view e mostra o comando para criar o Mailable.
- **Componentes:** `resources/views/components/mail/<nome>.blade.php`, com `@props` e uma descrição em comentário. `button`, `header` e `footer` já vêm com um modelo próprio.
- **Outras stacks:** `emails/<nome>.html` e `emails/components/<nome>.html`.

No app, o mesmo fica em **Novo template ou componente**. O projeto pode ser escolhido explicitamente com `--site`.

## Limitações

- Componentes de HTML puro ainda não entram como bloco visual; edite-os como código.
- O editor de código tem realce de sintaxe (Blade, HTML, Markdown), busca (`Ctrl+F`) e desfazer, mas não tem autocomplete das variáveis do template.
- O preview pelo framework existe só para Laravel. Nas outras stacks, use o preview estático ou a ferramenta do próprio framework.
