# Tinker

Los sitios Laravel tienen la pestaña **Tinker** para ejecutar PHP dentro de la app con la versión de PHP del sitio, su entorno (variables compartidas, configuración de correo y de dumps) y su `.env`, exactamente como su servidor.

- **Ejecutar** (`Ctrl`/`⌘` `Enter`) ejecuta el código del editor con `php artisan tinker --execute`. Cada ejecución empieza de cero, con hasta 30 segundos y 256 KB de salida.
- **Consola** abre el REPL interactivo (`php artisan tinker`) en una terminal dentro de la app; mantiene las variables entre líneas hasta que sales.

Necesita `laravel/tinker` (incluido en el esqueleto de Laravel) y las dependencias instaladas (`composer install`). El código solo se ejecuta cuando lo pides, y es tu app: accede a la base de datos y los servicios del sitio.

## En la terminal

```bash
locdev tinker -e 'App\Models\User::count()'
echo 'config("app.name")' | locdev tinker
locdev tinker --site tienda    # un prompt; cada línea se ejecuta sola
```
