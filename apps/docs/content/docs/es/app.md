# Recorrido por la aplicación

La aplicación se organiza en páginas principales, cada una con sus subpáginas. A la izquierda está la barra lateral; abajo, una barra de estado muestra de un vistazo el daemon, el proxy, el DNS y el HTTPS (cada parte lleva a su configuración).

## Apertura

Al abrir, una ventana pequeña dibuja el símbolo de LocDev y el **trazo de arranque**: daemon → proxy → DNS → HTTPS. Cada nodo se enciende con el estado real leído del daemon (verde cuando todo está bien, ámbar cuando funciona con salvedades, como HTTPS aún no confiable, rojo cuando falló). Cuando la interfaz tiene sus datos, el satélite del símbolo "sale de su órbita" y la ventana principal toma su lugar. La pantalla de inicio nunca retiene la apertura más de 8 segundos, y respeta el tema y la preferencia de animaciones.

## Navegación

![El Dashboard: el entorno en una franja, los sitios activos y lo que requiere atención.](screenshot:dashboard)

La barra lateral tiene dos modos, **expandida** (iconos y nombres) y **contraída** (solo iconos, con descripciones). Cambia con `Ctrl` `B` o con el botón al pie; la elección se guarda. En ventanas estrechas siempre está contraída. Las flechas `↑` `↓` recorren los elementos.

![Barra lateral contraída](screenshot:sidebar-collapsed)

Solo **Herramientas** tiene submenú en la barra lateral. Las demás subpáginas están dentro de su página: pestañas en Runtimes y en cada sitio, navegación lateral en Configuración. Cada página muestra su ruta (_breadcrumbs_) y botones de atrás/adelante (`Alt` `←` / `Alt` `→`, o los botones laterales del ratón).

| Página         | Qué tiene                                                                                                                                                                              |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dashboard      | Estado del entorno, entornos activos, alertas (sitios con fallos, Doctor, runtimes) y actividad reciente                                                                               |
| Sitios         | La lista de sitios y el detalle de cada uno: Resumen, Servidor, Dependencias, Entorno, Config, Email Studio, Deploy, Git, GitHub (con remoto en GitHub) y Configuración                |
| Runtimes       | Resumen (qué usa cada sitio y por qué), Node.js, PHP y gestores de paquetes                                                                                                            |
| Services       | Instalaciones e instancias de bases de datos y cachés ([Servicios](/docs/services))                                                                                                    |
| Herramientas   | [Nuevo proyecto e Instaladores](/docs/new-project), Procesos y logs, [Database](/docs/database), [Mailbox](/docs/mailbox), [Actions builder](/docs/actions), Línea de comandos, Doctor |
| Notificaciones | Pendientes, historial y preferencias ([detalles](/docs/dependencies))                                                                                                                  |
| Configuración  | Resumen, Daemon, Proxy, DNS, Carpetas monitoreadas, HTTPS, Automatizaciones, Frameworks, Entorno, Seguridad, Integraciones, Actualizaciones, Apariencia, Atajos y Acerca de            |

## Barra de título

La barra superior es de la propia aplicación, no del sistema: a la izquierda, un LED con el estado del daemon y dónde estás (`locdev › sites · tienda`); en el centro, la búsqueda, que abre la paleta de comandos; a la derecha, minimizar, maximizar y cerrar. Arrastra la barra para mover la ventana y haz doble clic para maximizarla. Cerrar solo oculta la aplicación en la bandeja, como antes. En macOS quedan los botones del propio sistema, sobre la barra.

## Sitios

![Pantalla de sitios](screenshot:sites)

- **Lista a la izquierda:** los sitios se agrupan en _en ejecución_ e _inactivo_, con un LED de estado, el tipo de proyecto y el puerto. Filtra con `Ctrl` `F`.
- **Pestañas:** cada sitio se abre en una pestaña, como en un navegador. Hacer clic en un sitio de la lista lo muestra en la pestaña del frente (o va a la pestaña que ya lo tiene); `Ctrl`+clic o el clic central lo abre en una pestaña nueva, y **+** abre una pestaña vacía con todos los sitios para elegir. Arrastra las pestañas para reordenarlas; el clic central cierra una. Cada pestaña recuerda su subpágina (Resumen, Servidor, Git…), y las pestañas vuelven al reabrir la app.
- **Barra de direcciones:** la dirección del sitio con un candado (verde cuando el HTTPS es de confianza; ámbar lleva a confiar en la CA), el framework, el estado y los controles (iniciar, detener, reiniciar, abrir en el navegador, mostrar la carpeta y el menú **⋯**). Haz clic en la dirección y escribe parte del nombre de un sitio para ir a él: `Enter` lo abre en esta pestaña, `Ctrl` `Enter` en una nueva. Un subdominio (`api.tienda.test`) lleva a su sitio.
- **Resumen:** ruta, comando, proceso y los runtimes que usa el sitio, con el origen de cada versión (pin, `.nvmrc`, `engines`, predeterminada) y avisos de rango no cumplido o versión sin soporte.
- **Servidor:** la salida del proceso en vivo, con stderr en ámbar y las notas de LocDev en cursiva. Puedes limpiar la vista, copiar la salida y pausar el desplazamiento automático.
- **Configuración:** comando, puerto, versiones de runtime y de gestor de paquetes, inicio junto con LocDev, y los dominios del sitio con el estado de HTTPS.

![Configuración de un sitio](screenshot:site-settings)

Cuando un proceso falla, el motivo aparece destacado y lleva a la salida:

![Un WordPress que falló por falta de la extensión mysqli.](screenshot:site-failed)

Haz clic derecho en un sitio de la lista, o usa el menú **⋯**, para las mismas acciones: abrir en el navegador, iniciar/detener, reiniciar, abrir en una pestaña nueva, editar, mostrar la carpeta, copiar la URL o la ruta y eliminar. Eliminar pide confirmación y **nunca borra archivos**.

## Bandeja

Mientras la aplicación está abierta, LocDev queda en la bandeja del sistema. En Linux y Windows el icono es color brasa con el daemon en marcha y gris cuando está detenido; en macOS es monocromo como los demás iconos de la barra de menús, y el estado aparece en el menú. El menú muestra cuántos sitios están activos y, para cada sitio, _Abrir en el navegador_, _Iniciar_/_Detener_ y _Reiniciar_, además de abrir la aplicación e iniciar, detener o reiniciar el daemon.

### El punto de color

Cuando hay algo para leer, aparece un punto en la esquina del icono de la bandeja. Su color indica qué:

| Color   | Significa                                                              |
| ------- | ---------------------------------------------------------------------- |
| Azul    | Notificaciones sin leer (actualizaciones, fallos, resultados de jobs…) |
| Verde   | Correos capturados por el [Mailbox](/docs/mailbox) que aún no abriste  |
| Violeta | Ambas cosas                                                            |

La descripción del icono y el menú dicen cuántos son (por ejemplo, _3 unread notifications, 1 unread email (violet dot)_), y los elementos del menú abren Notificaciones o el Mailbox. El punto desaparece cuando todo está leído. En macOS, con el punto encendido, el icono deja de ser monocromo para que se vea el color.

Cerrar la ventana solo oculta la aplicación: sigue en la bandeja. Para salir, usa _Salir de LocDev_ en el menú. Salir cierra solo la aplicación; el daemon sigue sirviendo los sitios, como con la CLI (`locdev stop` detiene el daemon). Abrir LocDev de nuevo trae la ventana que ya estaba abierta, que también es el camino de vuelta en escritorios sin bandeja.

> [!NOTE]
> En GNOME, los iconos de la bandeja dependen de la extensión AppIndicator, que Ubuntu ya trae activa. En otras distribuciones, instala la extensión _AppIndicator and KStatusNotifierItem Support_.

## Paleta de comandos

`Ctrl` `K` (o `⌘` `K` en macOS) abre la paleta: iniciar, detener y abrir sitios, controlar el daemon e ir a cualquier página o subpágina.

![Paleta de comandos](screenshot:palette)

Los atajos principales: `Ctrl` `K` (paleta), `Ctrl` `N` (añadir un sitio), `Ctrl` `1` … `7` (páginas), `Ctrl` `,` (configuración), `Ctrl` `T`/`Ctrl` `W`/`Ctrl` `Tab` (pestañas de sitio) y `Alt` `←`/`→` (atrás/adelante). Todos se pueden cambiar en **Configuración → Atajos**; la lista completa está en [Atajos de teclado](/docs/shortcuts).

## Doctor

En **Herramientas → Doctor**.

![Cada comprobación trae su detalle y, cuando hay algo que hacer, el comando que lo resuelve (clic para copiar).](screenshot:doctor)

## Configuración

![Configuración: resumen](screenshot:settings)

El resumen muestra el estado de cada área; cada fila lleva a su subpágina:

- **Daemon:** proceso, tiempo activo, endpoint IPC, iniciar/detener/reiniciar, _Iniciar LocDev al iniciar sesión_ y las carpetas de configuración y logs;
- **Proxy:** el puerto y los dominios que atiende;
- **[DNS](/docs/domains)** y **[HTTPS](/docs/https)** (con el estado HTTPS de cada sitio);
- **Apariencia:** idioma de la aplicación, tema, modo de la barra lateral y animaciones (seguir el sistema, reducidas o completas);
- **Integraciones:** la cuenta de GitHub, el [editor de código](/docs/editors) y las [plataformas de deploy](/docs/deploy);
- **[Actualizaciones](/docs/updating)** y **[Atajos](/docs/shortcuts)**;
- **Acerca de:** el logo, las versiones de la app, el daemon y la CLI, el commit y la fecha de compilación, el sistema y la arquitectura, el canal de actualización, la licencia (Apache-2.0), la lista de software de terceros con sus licencias, el historial de versiones (el changelog) y los enlaces a la documentación y al repositorio; debajo, la CLI en el `PATH`.

## Runtimes

![Runtimes: resumen](screenshot:runtimes)

El resumen muestra los valores predeterminados y una tabla de lo que usa cada sitio y de dónde viene la versión. Las pestañas Node.js, PHP y Gestores listan las versiones gestionadas y detectadas, con insignias de soporte y uso. Consulta [Runtimes](/docs/runtimes).

## Services

![Services: instalaciones del sistema e instancias](screenshot:services)

Instalaciones detectadas o descargadas por LocDev, y las instancias con su propio puerto, datos y logs. Consulta [Servicios](/docs/services).

## Idioma

La aplicación está en portugués, inglés, español y japonés. Por defecto sigue el idioma del sistema; en **Configuración → Apariencia** se puede fijar uno. Cambiar el idioma recarga la ventana. El menú de la bandeja sigue el idioma de la aplicación una vez que se abre. Los mensajes que vienen del daemon (errores, logs, resultados del doctor) y los datos de los proyectos siguen en inglés.

## Tema claro y oscuro

La aplicación sigue el tema del sistema o fija _Paper_ (claro) o _Carbon_ (oscuro). Las capturas de esta documentación están en el idioma de la página y siguen el tema del sitio.

## Accesibilidad

La app sigue WCAG 2.2 nivel AA, la referencia de EN 301 549, la European Accessibility Act y la Section 508:

- **Teclado:** todo funciona sin ratón. `Tab` recorre los controles con el foco siempre visible, las flechas se mueven por la barra lateral y las pestañas, y los atajos están en **Configuración → Atajos**.
- **Lectores de pantalla:** los botones con solo un icono tienen nombre, las pestañas siguen el patrón `tablist`/`tab`, y las áreas con desplazamiento (código, mensajes) reciben el foco.
- **Contraste:** textos y botones superan 4.5:1 en los dos temas.
- **Movimiento:** la app respeta `prefers-reduced-motion`, y **Configuración → Apariencia** desactiva las animaciones.

Cada versión pasa por una comprobación automática (axe-core) de todas las pantallas, en los temas claro y oscuro, incluida la navegación solo con teclado: `make test-a11y`.

## Identidad visual

El logo de LocDev es un PNG oficial, usado tal cual. La única fuente es `apps/desktop/branding/logo.png`; `pnpm icons` lo recorta, centra y redimensiona para generar los iconos de Windows (`.ico`), macOS (`.icns`), Linux y Tauri, los iconos de la bandeja, el `logo.png` de la aplicación y de la pantalla de inicio, y el `favicon.ico`.
