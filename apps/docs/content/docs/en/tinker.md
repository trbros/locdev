# Tinker

Laravel sites have a **Tinker** tab to run PHP inside the app with the site's PHP version, its environment (shared variables, mail and dump settings) and its `.env`, exactly like its server.

- **Run** (`Ctrl`/`⌘` `Enter`) runs the editor's code with `php artisan tinker --execute`. Each run starts fresh, for up to 30 seconds and 256 KB of output.
- **Console** opens the interactive REPL (`php artisan tinker`) in a terminal in the app; it keeps variables between lines until you leave it.

It needs `laravel/tinker` (part of Laravel's skeleton) and the installed dependencies (`composer install`). Code runs only when you ask, and it's your app: it reaches the site's database and services.

## In the terminal

```bash
locdev tinker -e 'App\Models\User::count()'
echo 'config("app.name")' | locdev tinker
locdev tinker --site shop      # a prompt; each line runs on its own
```
