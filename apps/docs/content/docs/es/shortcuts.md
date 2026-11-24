# Atajos de teclado

En **Configuración → Atajos** están todos los atajos de la app, agrupados por categoría: puedes buscarlos, cambiarlos, añadir una segunda combinación, quitar una o volver al predeterminado. `Ctrl` `/` abre esa página.

![Atajos: búsqueda, categorías y las combinaciones de cada acción.](screenshot:shortcuts)

## Cómo funcionan

- **Solo dentro de LocDev.** Los atajos funcionan con una ventana de LocDev al frente. LocDev no registra atajos globales del sistema, así que nunca quita teclas a otras apps.
- **En macOS, `Ctrl` pasa a ser `⌘`.** Salvo para cambiar de pestaña de sitio (`Ctrl` `Tab`), porque `⌘` `Tab` cambia de app en macOS y nunca llega a LocDev.
- **En campos de texto** solo se activan las combinaciones con `Ctrl`/`⌘`; `Alt` con las flechas y las teclas solas siguen editando el texto.
- **Dónde valen:** algunos atajos solo existen en una página (marcados con _en Sitios_): las pestañas de sitio, el filtro y las acciones del sitio de la pestaña al frente.
- **Dónde se guardan:** en este equipo, en el almacenamiento de la app (no en `settings.toml`). Un cambio vale al instante para todas las ventanas de LocDev.

## Cambiar un atajo

1. Haz clic en **+** junto a la acción y pulsa la combinación. `Esc` cancela.
2. Necesita `Ctrl`, `⌘` o `Alt` (o una tecla de función, `F1`…`F12`), para que escribir nunca active un atajo.
3. Si otra acción ya la usa, LocDev dice cuál y pregunta: **Usar aquí** se la quita a la otra acción.
4. Pasa el ratón sobre una combinación para quitarla (**×**); el botón de restaurar devuelve el predeterminado de esa acción, y **Restaurar todos los predeterminados** lo deshace todo.

Una combinación marcada en ámbar tiene un aviso:

- **Conflicto:** dos acciones la comparten; solo se ejecuta una (la que va antes en esta lista).
- **Reservada:** el sistema o la edición de texto la usa, y puede que nunca llegue a LocDev. Por ejemplo `Alt` `F4` y `Alt` `Tab` (Windows y Linux), `Ctrl` `Alt` `T` (terminal en Ubuntu), `⌘` `Q`, `⌘` `H` y `⌘` `Tab` (macOS), combinaciones con la tecla Windows/Super, y `Ctrl` `C`/`V`/`X`/`A`/`Z` en los campos de texto.

Los atajos integrados (con candado) aparecen solo como referencia: deshacer y rehacer en Email Studio, y `Esc` para cerrar un diálogo o borrar un filtro.

## Predeterminados

| Categoría          | Acción                                                                               | Atajo                               |
| ------------------ | ------------------------------------------------------------------------------------ | ----------------------------------- |
| General            | Paleta de comandos                                                                   | `Ctrl` `K`                          |
| General            | Configuración                                                                        | `Ctrl` `,`                          |
| General            | Atajos de teclado                                                                    | `Ctrl` `/`                          |
| General            | Contraer o expandir la barra lateral                                                 | `Ctrl` `B`                          |
| General            | Atrás / adelante                                                                     | `Alt` `←` / `Alt` `→`               |
| Navegación         | Ir a Panel, Sitios, Runtimes, Servicios, Herramientas, Notificaciones, Configuración | `Ctrl` `1` … `Ctrl` `7`             |
| Proyectos          | Añadir un sitio                                                                      | `Ctrl` `N`                          |
| Proyectos          | Crear un proyecto nuevo                                                              | `Ctrl` `Shift` `N`                  |
| Proyectos          | Filtrar sitios (en Sitios)                                                           | `Ctrl` `F`                          |
| Proyectos          | Nueva pestaña / cerrar la pestaña (en Sitios)                                        | `Ctrl` `T` / `Ctrl` `W`             |
| Proyectos          | Pestaña de sitio siguiente / anterior (en Sitios)                                    | `Ctrl` `Tab` / `Ctrl` `Shift` `Tab` |
| Proyectos          | Abrir el sitio en el [editor](/docs/editors) (en Sitios)                             | `Ctrl` `E`                          |
| Logs y diagnóstico | Abrir el [Debug Center](/docs/debug-center)                                          | `Ctrl` `Shift` `D`                  |

Sin atajo por defecto, para que elijas: abrir el sitio en el navegador, mostrar la carpeta del sitio, iniciar o detener, reiniciar y compartir el sitio (en Sitios), y abrir cada herramienta (Línea de comandos, Nuevo proyecto, Instaladores, Base de datos, Mailbox, Túneles, Actions builder, Processes and logs, Doctor).

La paleta de comandos, los menús y las pistas de los botones muestran siempre la combinación actual, también la que cambiaste.

## Limitaciones

- No hay atajos globales (fuera de LocDev) ni uno para traer la ventana desde otra app.
- La TUI (`locdev` en la terminal) tiene sus propias teclas, que no cambian aquí.
- En Linux, los atajos de la terminal y de los escritorios dependen del entorno gráfico; la lista de reservadas cubre los predeterminados más comunes (GNOME/Ubuntu), no todos.
