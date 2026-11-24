# Qué es LocDev

LocDev es un entorno de desarrollo local **de código abierto** para Linux, macOS y Windows. Registra las carpetas de tus proyectos, ejecuta sus servidores de desarrollo y entrega cada uno en su propio dominio, como `blog.test`, con HTTPS válido.

![La pantalla principal: tus sitios a la izquierda y el seleccionado, con la salida del proceso en vivo, a la derecha.](screenshot:sites)

## Cómo está organizado

LocDev tiene tres piezas, y todas hablan el mismo idioma:

- **`locdevd`**: el daemon. Se ejecuta en segundo plano, sin privilegios de administrador, y es dueño de todo: el registro de proyectos, los procesos, el proxy, el DNS y los certificados. Los sitios siguen funcionando con la aplicación cerrada.
- **`locdev`**: la CLI. Controla el daemon desde la terminal y acepta `--json` para automatización.
- **La aplicación de escritorio**: una interfaz (Tauri + React) sobre las mismas operaciones que la CLI.

Ninguna regla vive solo en la interfaz. Si la aplicación puede hacer algo, la CLI también.

## Qué hace

- **Sitios:** detección de 46 stacks (Laravel, Next.js, Django, Rails, Vite, Spring, .NET, Go…), servidores de desarrollo supervisados con logs, procesos auxiliares (colas, programador, WebSockets) e inicio junto con LocDev. Consulta [Sitios](/docs/sites) y [Frameworks](/docs/frameworks).
- **Red:** proxy HTTP/HTTPS por nombre de host con WebSockets, DNS propio para `*.test`, [subdominios y comodines](/docs/subdomains) y una CA local con restricción de nombres.
- **Runtimes:** versiones de Node.js, PHP y gestores de paquetes por sitio, descargadas con verificación SHA-256 ([Runtimes](/docs/runtimes)).
- **Entorno y servicios:** [variables compartidas y secretos](/docs/environment) inyectados en los procesos, e instancias de PostgreSQL, MySQL, MariaDB, MongoDB, Redis y Valkey ([Servicios](/docs/services)).
- **Herramientas:** [Database Manager](/docs/database), [Mailbox](/docs/mailbox), [Email Studio](/docs/email-studio), [Config Files](/docs/config-files), el [asistente de despliegue](/docs/deploy), [dependencias](/docs/dependencies), [Git y GitHub](/docs/git) y [GitHub Actions](/docs/actions).
- **Diagnóstico:** el [Doctor](/docs/doctor) señala cada problema con su solución.

Lo que cambió en cada versión está en el [Changelog](/changelog).

## Principios

- **Sin muro de pago:** el entorno local y sus funciones esenciales son libres.
- **Nativo primero:** procesos nativos, sin contenedores.
- **Funciona sin conexión** una vez instalado.
- **Seguro por defecto:** privilegios mínimos y confirmación explícita para cualquier cambio en el sistema.
