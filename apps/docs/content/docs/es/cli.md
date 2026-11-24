# CLI

`locdev` controla el daemon desde la terminal: todo lo que hace la aplicación también funciona desde aquí, porque ambas hablan con el mismo daemon. Todos los comandos aceptan `--json` para una salida legible por máquinas, y `locdev <comando> --help` muestra las opciones.

Los comandos de sitio (`up`, `down`, `reload`, `logs`, `info`, `open`, `secure`, `dev …`) aceptan el nombre del sitio o, sin nombre, usan el sitio registrado que contiene el directorio actual:

```bash
cd ~/Code/shop
locdev up            # lo mismo que locdev up shop
locdev logs -f
locdev secure
```

Los comandos marcados como **admin** muestran el cambio exacto (el script) y piden confirmación antes de usar privilegios de administrador; `--yes` omite la pregunta. No se instala nada en el sistema sin ese paso.

## Interfaz de terminal (TUI)

`locdev` solo, en una terminal interactiva, abre la TUI (lo mismo que `locdev tui`; `--light` usa los colores claros). Tiene el aspecto de la app: barra lateral con las páginas, los sitios y los servicios con sus LED, el estado del proxy, el DNS, el HTTPS y el Mailbox abajo, y la página en el centro (Dashboard, Sitios, Servicios, Runtimes, Procesos, Mailbox, Actividad, Doctor y Acerca de, que muestra la versión de la CLI y del daemon).

| Tecla         | Qué hace                                                                        |
| ------------- | ------------------------------------------------------------------------------- |
| `↑` `↓` `tab` | Se mueve en la lista; `tab` alterna entre la barra lateral y la página          |
| `enter`       | Abre la página, la salida del sitio o servicio, el mensaje                      |
| `s` · `R`     | Inicia/detiene · reinicia el sitio, servicio o proceso seleccionado             |
| `o` · `l`     | Abre el sitio en el navegador · muestra su salida                               |
| `ctrl+p`      | "Ir a o hacer": páginas, sitios, acciones y todos los comandos de la CLI        |
| `:`           | Ejecuta cualquier comando `locdev` aquí mismo, con confirmaciones y contraseñas |
| `/` · `?`     | Filtra las listas · muestra todas las teclas                                    |
| `q`           | Sale (el daemon y los sitios siguen funcionando)                                |

Lo que no tiene pantalla propia en la TUI se ejecuta con `:` (por ejemplo `:git commit -m "…"`), que cede la terminal al comando y vuelve a la TUI al terminar. Sin terminal interactiva (en un pipe, o con `--json`), `locdev` sin comando muestra la ayuda.

## Códigos de salida

| Código | Cuándo                                                                            |
| ------ | --------------------------------------------------------------------------------- |
| `0`    | Éxito.                                                                            |
| `1`    | Error (el mensaje dice qué hacer). También cuando el `doctor` encuentra un error. |
| `3`    | `locdev status` con el daemon detenido.                                           |

## Referencia

La referencia de abajo se genera a partir de las definiciones de la propia CLI (`make cli-catalog`); la página **Tools → Command line** de la aplicación muestra el mismo catálogo, con búsqueda. Una prueba falla si esta copia queda desactualizada.

<!-- generated:cli -->
