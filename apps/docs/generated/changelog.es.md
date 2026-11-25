# Historial de cambios

[English](CHANGELOG.md) · [Português (Brasil)](CHANGELOG.pt-BR.md) · [Español](CHANGELOG.es.md) · [日本語](CHANGELOG.ja.md)

La primera versión pública de LocDev es la versión inicial 0.19.0: aplicación de escritorio 0.19.0 con daemon y CLI 0.18.0. Las versiones desde 0.1.0 pasaron por pruebas internas y con usuarios seleccionados y se reúnen en esta versión inicial. A partir de aquí, cada release tendrá su propia entrada.

El daemon/CLI y la aplicación de escritorio tienen versiones separadas: los cambios solo en la interfaz o el shell modifican la versión de la aplicación; los cambios en el daemon o la CLI modifican la suya. El título indica ambas cuando son distintas.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/). Secciones: Añadido, Modificado, Corregido, Seguridad y Cambios incompatibles. Los instaladores dependen de las [releases publicadas](https://github.com/trbros/locdev/releases). [Documentación completa](https://locdev.app/es/docs).

## [0.19.2] - 2026-11-25

App 0.19.2 con daemon y CLI 0.18.2.

### Añadido

- Los instaladores tienen arte propio, hecho a partir del logo: un icono de
  instalador con una insignia de descarga y las imágenes de bienvenida y de
  las páginas en Windows (NSIS y MSI), y una ventana con una flecha hacia
  Aplicaciones en el `.dmg` de macOS.
- Las copias instaladas con el `.deb` o el `.rpm` se actualizan solas: la
  app instala el paquete de la release con la herramienta de paquetes del
  sistema, que pide tu contraseña (`pkexec`). Una copia instalada antes
  necesita una actualización manual.
- Herramientas → Instaladores: comandos de instalación para Python, pipx y
  Git en Windows y macOS.
- Los proyectos nuevos creados desde la app van por defecto a
  `~/LocDev/Sites` (se crea en el primer uso).
- Quitar un sitio puede también mover su carpeta a la papelera (una opción
  en el diálogo, `locdev remove <nombre> --delete-files`); se rechaza para
  una carpeta que contiene otros sitios, la carpeta personal o la raíz de un
  disco.
- Un sitio cuya carpeta se borra fuera de LocDev se detiene y se quita en
  segundos, con una notificación (no uno en un disco o recurso compartido
  desconectado).

### Modificado

- Proyecto nuevo: elegir un framework abre su formulario en una pantalla
  propia; al terminar, el sitio se inicia solo y su botón muestra En
  ejecución.
- Los instaladores se ejecutan sin las opciones que saltan sus preguntas
  (`--no-interaction` en `laravel new`, Composer y `artisan migrate`,
  `--yes` en `create-next-app` y `create-astro`, `--defaults` en
  `ng new`): lo que las elecciones de LocDev no responden, el instalador lo
  pregunta en la terminal, donde aparecen su salida y sus errores.
- Bun se instala con su propio script (en `~/.bun/bin`) en lugar de
  `npm install -g`, que requiere administrador con un Node.js del sistema.
  Rails se instala por usuario cuando la carpeta de gems de Ruby no admite
  escritura.

### Corregido

- Next.js con rewrites del middleware, como el prefijo de idioma de
  next-intl: un 500 por HTTPS (`Failed to proxy … EPROTO … wrong version
  number`) o un bucle de redirecciones (`ERR_TOO_MANY_REDIRECTS`) con el
  idioma predeterminado sin prefijo. El servidor de desarrollo ahora arranca
  con `--hostname localhost` (en 127.0.0.1 gracias a
  `NODE_OPTIONS=--dns-result-order=ipv4first`), así que Next ya no se hace
  proxy de esos rewrites a sí mismo. Los sitios guardados con el comando
  anterior pasan al nuevo cuando arranca el daemon.
- Hacer clic en una notificación del sistema (Linux) abre la app en lo que
  trata, como un clic en la lista de notificaciones: la pestaña del sitio,
  el mensaje del Mailbox, la actualización; un resumen de varias abre la
  lista.
- Interruptores: hacer clic en el texto de uno podía cambiar otro o ninguno
  en algunos motores web; ahora el texto cambia su propio interruptor. Los
  selects de Proyecto nuevo mostraban la opción elegida en inglés.
- pnpm 11 y posteriores no arrancaban (`SyntaxError: Invalid or
  unexpected token`): LocDev ejecutaba con Node.js el archivo provisional
  que trae el paquete. Ahora instala el binario nativo de pnpm, comprobado
  en el registro como el paquete, y reinstala las copias rotas al iniciar
  el daemon.
- El aviso de actualización mostraba "Invalid Date" junto a la versión.
- Al añadir un sitio, hacer clic en "Iniciar con LocDev" cambiaba "Iniciar
  ahora" (y hacer clic en un archivo de un commit marcaba el primero).
- Proyecto nuevo: elegir un framework desplaza hasta sus campos.
- Herramientas → Instaladores: Symfony CLI aparecía como no instalado
  después de su instalador (ahora usa `~/.config/symfony-cli/bin`); las
  herramientas de las carpetas de los instaladores pasaban por delante de
  los runtimes de LocDev; la lista vuelve a comprobar al terminar una
  instalación.
- Textos que solo aparecían en inglés: frameworks, opciones y herramientas
  en Proyecto nuevo e Instaladores, los pasos del plan, el aviso del almacén
  de secretos, la página de la CLI y etiquetas en todo el sitio web.
- Los sitios PHP abiertos por HTTPS (Laravel, Symfony, WordPress, Bedrock,
  PHP puro) enlazaban sus propios assets, fuentes y la barra de Symfony con
  `http://`, y el navegador los bloqueaba como _mixed content_. Todo sitio
  PHP carga ahora el `auto_prepend_file` de LocDev, que da a las peticiones
  que llegaron al proxy por HTTPS las variables de servidor que define un
  servidor TLS (`HTTPS`, `REQUEST_SCHEME`, `SERVER_PORT`). Ningún archivo
  del proyecto cambia.
- En una petición HTTPS, una redirección a `http://` en el mismo host pasa
  a apuntar a `https://`, así que las apps con una URL configurada en
  `http://` (el `WP_HOME` de Bedrock) ya no devuelven el navegador a HTTP.
- Laravel con el auxiliar Vite: en Vite 8.3 o posterior, la página cargaba
  sus assets desde `https://127.0.0.1:<puerto>`, que el certificado de Vite
  no cubría. El certificado ahora incluye `localhost`, `127.0.0.1` y `::1`.
- Nuxt 4.1 o posterior: la insignia de errores del modo dev
  (`/__nuxt_dev__/…`) respondía 403 en `<site>.test`.
- Django: los proyectos nuevos respondían 400 (`DisallowedHost`) en
  `<site>.test`. LocDev añade ahora `.test` a `ALLOWED_HOSTS` y el protocolo
  del proxy (`SECURE_PROXY_SSL_HEADER`) a la configuración de los proyectos
  nuevos, dentro de `if DEBUG:`; la nota para proyectos existentes indica
  qué añadir.
- ASP.NET Core: los assets estáticos respondían 500, porque `dotnet run
  --no-launch-profile` se ejecutaba en Production. LocDev define
  `ASPNETCORE_ENVIRONMENT=Development`, salvo que el sitio lo defina.
- NestJS: la creación del proyecto se detenía en las preguntas nuevas de
  Nest CLI (`@nestjs/observe`, skills de agentes).
- Bedrock: se detectaba como PHP genérico y se servía desde la raíz del
  proyecto en lugar de `web/`. Ahora es un sitio WordPress servido desde
  `web/`, y los proyectos nuevos reciben un `.env` con el `WP_HOME` del
  sitio.

## [0.19.1] - 2026-11-24

App 0.19.1 con daemon y CLI 0.18.1.

### Modificado

- Las URLs de los sitios siempre usan `.test`. Mientras `*.test` aún no
  resuelve, `<site>.localhost` sigue funcionando como alternativa.

### Corregido

- Windows: cuando otro programa (como Acrylic DNS Proxy) ocupa el puerto 53,
  el servidor DNS ahora lo indica en lugar de decir que necesita privilegios
  elevados, y Doctor sugiere liberar el puerto.
- Windows: rechazar la solicitud de administrador se informa como cancelado,
  y no como un cambio aplicado.

## [0.19.0] - 2026-11-24

Versión inicial. Aplicación 0.19.0 con daemon y CLI 0.18.0, para Linux (x86_64 y ARM64), macOS (Apple Silicon e Intel) y Windows, con código abierto bajo Apache 2.0.

### Añadido

#### Base

- Un daemon (`locdevd`) concentra las operaciones; la CLI (`locdev`) y la aplicación de escritorio (Tauri) comparten el protocolo. Las operaciones de la aplicación tienen comandos CLI, y `locdev` sin argumentos abre una interfaz de terminal a pantalla completa (`ctrl+p` accede a páginas, sitios y comandos).
- Ejecución nativa sin exigir Docker ni WSL. Proxy inverso HTTP, HTTPS y WebSockets, DNS para `*.test` (systemd-resolved, `/etc/resolver`, NRPT en Windows) y CA local con restricciones de nombres y certificados por sitio y subdominios profundos. `locdev doctor` comprueba la configuración y LocDev puede iniciarse al entrar en la sesión.
- Los cambios del sistema muestran un plan y requieren confirmación. Todo lo que LocDev escribe en el sistema lleva una marca; la desinstalación elimina solo lo marcado.

#### Sitios

- Detección mediante 46 definiciones de frameworks: Laravel, WordPress, Symfony, Next.js, Nuxt, Astro, SvelteKit, Vite, NestJS, Django, Flask, FastAPI, Rails, Go, Rust, .NET y otros. Las definiciones están firmadas y pueden actualizarse sin una nueva versión de la aplicación.
- Servidores de desarrollo supervisados con logs, reinicio con espera progresiva, CPU y memoria. El Vite auxiliar de Laravel solo se inicia tras tu decisión; se ofrecen procesos auxiliares por framework, como colas, Horizon, scheduler y Reverb.
- Sitios estáticos (`index.html`) en el servidor de LocDev: solo loopback, URL limpias, `404.html`, sin necesitar Python ni Node.
- Carpetas supervisadas: los proyectos dentro de una carpeta se convierten en sitios, siguiendo la creación y eliminación de carpetas.
- Subdominios por sitio con reglas para dirigirlos a otro puerto o proceso (`locdev subdomain`, `locdev route`).
- Pestañas de sitios con barra de direcciones, menú compartido entre el botón ⋯ y el clic derecho, y editor preferido: VS Code, Cursor, Windsurf, VSCodium, Zed, Sublime Text y JetBrains.
- Métricas por sitio (pestaña Métricas, `locdev metrics`): clases de estado, latencias p50/p95/p99, peticiones por minuto, rutas más visitadas y peticiones recientes, contadas por el proxy y guardadas solo en memoria.
- Laravel Tinker (pestaña Tinker, `locdev tinker`): ejecución de código o consola interactiva con el PHP y el entorno del sitio.

#### Runtimes

- Versiones de Node.js y PHP por sitio (fijada, predeterminada o del sistema), además de npm, pnpm, Yarn, Bun y Composer. Las descargas se instalan tras verificar el checksum publicado: Node.js de nodejs.org y PHP de StaticPHP (Linux/macOS) y windows.php.net (Windows).
- Detección de versiones instaladas mediante apt, Homebrew, nvm, fnm y PATH, sin eliminar versiones del sistema. Las predeterminadas también pueden usarse en el terminal (`locdev runtime terminal on`).
- Configuración de PHP por versión: memoria, subidas, límites de tiempo, OPcache y otras opciones. Cada cambio incluye revisión del diff, copia de seguridad, validación por PHP y reversión si PHP lo rechaza.
- Extensiones PHP: módulos cargados por cada PHP, archivos de la carpeta de extensiones sin cargar y, por sitio, extensiones obligatorias y recomendadas por los paquetes (`locdev runtime modules`, `locdev runtime extensions`).

#### Proyectos nuevos y herramientas

- Proyectos nuevos con los instaladores oficiales cuando están disponibles, con vista previa del comando y terminal en la aplicación: Laravel, Symfony, Bedrock, WordPress, Next.js, Vite, Vue, Angular, Nuxt, Astro, SvelteKit, NestJS, Django, Flask, FastAPI, Rust (Axum y Actix Web), Rails y ASP.NET Core. Creación opcional de una base de datos y registro como sitio. Las stacks sin generador adecuado reciben archivos iniciales mínimos.
- Laravel hace las preguntas de `laravel new` en el mismo orden: starter kit, stack, proveedor de autenticación, componentes Livewire de archivo único y equipos.
- Instaladores en la aplicación: PHP, Composer, Node.js y pnpm como runtimes verificados; instalador Laravel, Symfony CLI, Bun, uv, Poetry, rustup, WP-CLI, Git y SDK .NET con el comando documentado por su publicador para el sistema.

#### Servicios y datos

- Instancias gestionadas de PostgreSQL, MariaDB, MySQL, MongoDB, Redis y Valkey, cada una con datos, puerto, logs y copias de seguridad propios. Las descargas directas usan el SHA-256 publicado por la fuente. Otras instalaciones, como MySQL 8.0/8.4, Redis y más versiones de PostgreSQL/MongoDB, pueden usar Homebrew, apt o winget desde la aplicación.
- Database Manager (`locdev db`): tablas, filas, SQL con comprobaciones de sentencias destructivas y explorador de claves Redis/Valkey. Las credenciales permanecen en el daemon.
- Variables de entorno y secretos compartidos, inyectados en los procesos sin editar `.env`, con una bóveda cifrada.
- Mailbox: servidor SMTP de captura en loopback, con integración por framework.
- Email Studio: edición visual de plantillas, Mailables y componentes, conservando byte a byte el Blade fuera de los bloques gestionados, con vistas previas e historial.
- Config Files: formularios para opciones conocidas de configuración, edición de JSON y JS/TS sin ejecutar código del proyecto, historial y análisis.

#### Código, CI y deploy

- Git en la aplicación: cambios, commits, ramas, worktrees (cada rama como sitio), stashes y remotos. Las operaciones destructivas muestran las pérdidas, exigen una frase de confirmación y contraseña opcional y hacen una copia de seguridad antes.
- GitHub: cuenta en el keyring del sistema, issues, pull requests y ejecuciones de workflows.
- Editor de GitHub Actions: plantillas por stack, edición visual y YAML. Comprobaciones de seguridad y buenas prácticas con sugerencias de corrección. Instalación de actionlint desde la aplicación y versiones locales de cada workflow.
- Dependencias: actualizaciones, compatibilidad y vulnerabilidades mediante OSV, con planes de actualización confirmados por el usuario.
- Asistente de deploy: plataformas ordenadas por compatibilidad, comprobaciones de preparación y generación de `render.yaml`/`netlify.toml`; deploy, rollback y logs de Vercel/Netlify mediante sus API públicas.
- Túneles: comparte sitios activos mediante Cloudflare Quick Tunnel, con protección opcional por correo electrónico.

#### Depuración y aplicación

- Debug Center: excepciones, logs de error, respuestas HTTP 5xx, fallos de procesos y `dump()`/`dd()` de PHP, agrupados por problema con causas probables. Los secretos reconocibles se ocultan antes de guardar los eventos.
- Tareas en segundo plano y notificaciones en la aplicación, el sistema y el icono de la bandeja, con modo sin conexión y configuración por automatización.
- Aplicación en portugués de Brasil, inglés, español y japonés, temas claro/oscuro, barra de título y barras de desplazamiento propias, atajos editables y comprobaciones de accesibilidad WCAG 2.2 AA en las pantallas.
- Actualizaciones consultadas en GitHub Releases e instaladas por elección del usuario, con verificación de la clave de firma de LocDev. CLI independiente con `install.sh`/`install.ps1` y verificación de `SHA256SUMS`.
- Sitio de documentación en cuatro idiomas con tutoriales, referencia CLI generada del código y comparación con otras herramientas comprobada contra el repositorio.
