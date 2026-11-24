# Historial de cambios

[English](CHANGELOG.md) · [Português (Brasil)](CHANGELOG.pt-BR.md) · [Español](CHANGELOG.es.md) · [日本語](CHANGELOG.ja.md)

La primera versión pública de LocDev es la versión inicial 0.19.0: aplicación de escritorio 0.19.0 con daemon y CLI 0.18.0. Las versiones desde 0.1.0 pasaron por pruebas internas y con usuarios seleccionados y se reúnen en esta versión inicial. A partir de aquí, cada release tendrá su propia entrada.

El daemon/CLI y la aplicación de escritorio tienen versiones separadas: los cambios solo en la interfaz o el shell modifican la versión de la aplicación; los cambios en el daemon o la CLI modifican la suya. El título indica ambas cuando son distintas.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/). Secciones: Añadido, Modificado, Corregido, Seguridad y Cambios incompatibles. Los instaladores dependen de las [releases publicadas](https://github.com/trbros/locdev/releases). [Documentación completa](https://locdev.app/es/docs).

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
