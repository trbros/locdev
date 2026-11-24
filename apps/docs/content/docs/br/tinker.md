# Tinker

Sites Laravel têm a aba **Tinker** para rodar PHP dentro do app com a versão do PHP do site, o ambiente dele (variáveis compartilhadas, configuração de e-mail e de dumps) e o `.env`, exatamente como o servidor.

- **Executar** (`Ctrl`/`⌘` `Enter`) roda o código do editor com `php artisan tinker --execute`. Cada execução começa do zero, com até 30 segundos e 256 KB de saída.
- **Console** abre o REPL interativo (`php artisan tinker`) num terminal dentro do app; ele mantém as variáveis entre as linhas até você sair.

Precisa do `laravel/tinker` (que vem no esqueleto do Laravel) e das dependências instaladas (`composer install`). O código só roda quando você pede, e é o seu app: ele acessa o banco e os serviços do site.

## No terminal

```bash
locdev tinker -e 'App\Models\User::count()'
echo 'config("app.name")' | locdev tinker
locdev tinker --site loja      # um prompt; cada linha roda sozinha
```
