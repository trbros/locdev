# Carpetas monitoreadas

Una carpeta monitoreada es una carpeta cuyos proyectos se convierten en sitios solos. Monitorea `~/Development` y cada proyecto dentro de ella recibe su dominio:

```text
~/Development/
├── tienda/      → tienda.test
├── api/         → api.test
├── portfolio/   → portfolio.test
├── dashboard/   → dashboard.test
└── fotos/       (no es un proyecto: se omite)
```

La lista sigue a la carpeta: un proyecto creado, clonado o generado ahí aparece en segundos, y uno borrado sale de la lista. Los archivos nunca se tocan.

## Disponibilidad

| Sistema | Estado                                                                          |
| ------- | ------------------------------------------------------------------------------- |
| Linux   | Disponible (probado; eventos vía inotify)                                       |
| macOS   | Experimental: el código usa FSEvents, pero no se validó en un Mac               |
| Windows | Experimental: el código usa ReadDirectoryChangesW, pero no se validó en Windows |

En cualquier sistema, si los eventos de archivos no están disponibles, la carpeta se sigue revisando cada 5 minutos.

## Qué cuenta como proyecto

Una subcarpeta solo se vuelve sitio cuando sus archivos dicen que es un proyecto, con la misma detección de `locdev add` ([Frameworks](/docs/frameworks)): `artisan` y `composer.json` (Laravel), `package.json` (Node.js, Next.js, Vite…), `pyproject.toml`, `requirements.txt` o `manage.py` (Python), `Cargo.toml` (Rust), `index.php`, `index.html` y el resto de la tabla de frameworks. El nombre de la carpeta no importa.

Nunca se consideran proyectos ni se recorren:

- carpetas ocultas (`.git`, `.cache`…);
- dependencias y salida de build: `node_modules`, `vendor`, `bower_components`, `target`, `dist`, `build`, `out`, `coverage`, `tmp`, `temp`, `cache`, `storage`, `venv`, `env`, `__pycache__`, `site-packages`, `Pods`, `DerivedData`;
- lo que excluyas (ver [Reglas](#reglas)).

La búsqueda nunca entra en un proyecto: las subcarpetas de un proyecto (los paquetes de un monorepo, por ejemplo) no se vuelven sitios aparte. Los enlaces simbólicos se siguen, pero cada carpeta real se visita una sola vez, así que un enlace que apunta hacia arriba no crea un ciclo ni un sitio duplicado.

## Monitorear una carpeta

En la app: **Configuración → Carpetas monitoreadas → Monitorear una carpeta**. Elige la carpeta y las reglas.

![Carpetas monitoreadas](screenshot:parked-folders)

Desde la CLI (sin ruta, la carpeta actual):

```bash
locdev park
locdev park ~/Development
locdev park ~/Clientes --depth 2 --exclude 'archivo-*'
```

No se puede monitorear:

- una carpeta que ya es un proyecto (agrégala con `locdev add`);
- una carpeta dentro de otra monitoreada, o que contiene una (cada proyecto solo puede encontrarse por un camino);
- la raíz de un disco.

## Reglas

![Monitorear una carpeta](screenshot:park-folder)

| Regla                                     | Opciones                                                                                   | Por defecto          |
| ----------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------- |
| Buscar proyectos                          | en sus carpetas (1), hasta 2 o 3 niveles abajo                                             | 1                    |
| Cuando aparece un proyecto nuevo          | **Convertirlo en sitio** o **Sugerirlo** (aparece en la lista y tú lo agregas)             | Convertirlo en sitio |
| Cuando se borra la carpeta de un proyecto | **Quitar su sitio** o **Mantenerlo, marcado como ausente**                                 | Quitar su sitio      |
| Omitir carpetas                           | nombres o rutas relativas, `*` coincide con cualquier cosa (`archivo-*`, `clientes/viejo`) | ninguna              |

Para cambiar las reglas, ejecuta `locdev park` de nuevo en la misma carpeta: solo cambia lo que pases.

```bash
locdev park ~/Development --suggest          # los proyectos nuevos solo se sugieren
locdev park ~/Development --keep-missing     # el sitio de una carpeta borrada se queda
locdev park ~/Development --no-exclude       # vacía la lista de exclusiones
```

Con profundidad 2 o 3, la búsqueda solo baja por carpetas que no son proyectos: en `~/Clientes/acme/tienda`, `acme` no puede ser un proyecto para que se encuentre `tienda`.

## Proyectos encontrados

Cada proyecto aparece en la lista de la carpeta con un estado:

| Estado (app)   | CLI                    | Significa                                                                   |
| -------------- | ---------------------- | --------------------------------------------------------------------------- |
| sitio          | `site`                 | se volvió sitio por esta carpeta                                            |
| añadido a mano | `site (added by hand)` | ya era uno de tus sitios; sigue como está                                   |
| sugerido       | `suggested`            | esperando que lo agregues (**Agregar** en la app, o `locdev add <carpeta>`) |
| ignorado       | `ignored`              | no quieres este proyecto como sitio                                         |
| no añadido     | `not added`            | no se pudo crear el sitio (el motivo aparece al lado)                       |

```bash
locdev park list
```

```text
/home/tu/Development  watching · depth 1 · new: become sites · gone: removed · scanned 12s ago
  api        api.test                      Rust                site
  dashboard  dashboard.test                Next.js             site
  portfolio  portfolio.test                HTML5               site
  tienda     tienda.test                   Laravel             site
```

Los sitios nuevos empiezan **detenidos** y sin inicio automático; nada se ejecuta hasta que lo pidas. El puerto, el comando y el runtime vienen de la detección, como en `locdev add`.

### Nombres repetidos

El nombre del sitio es el de la carpeta. Si ya existe, LocDev le agrega el nombre de la carpeta de arriba (`tienda-clientes` para `~/Clientes/tienda`, o `tienda-development` para `~/Development/tienda`) y, si también está ocupado, un número (`tienda-2` a `tienda-9`). El motivo aparece al lado del proyecto. Una vez registrado, el nombre se queda: las revisiones siguientes no renombran sitios.

### Ignorar un proyecto

```bash
locdev park ignore ~/Development/borrador
locdev park unignore ~/Development/borrador
```

Ignorar quita el sitio (si vino de la carpeta) y evita que vuelva. Quitar un sitio que vino de una carpeta monitoreada, en la app o con `locdev remove`, hace lo mismo: pasa a estar ignorado ahí; si no, volvería en la próxima revisión.

## Revisiones y eventos

LocDev revisa la carpeta cuando la monitoreas o cambias sus reglas, cuando algo cambia en ella (eventos de archivos, agrupados hasta que la carpeta queda quieta 1,5 s, para que un `git clone` o un `laravel new` cuente como un solo cambio) y, para recuperar eventos perdidos, cada 5 minutos. Para forzarla:

```bash
locdev park scan
locdev park scan ~/Development
```

Los eventos vienen solo de la carpeta monitoreada y de las carpetas intermedias (que no son proyectos); LocDev no observa el interior de los proyectos, para no gastar el límite de observadores del sistema en `node_modules`.

Si la propia carpeta monitoreada desaparece (un disco externo desconectado, una carpeta renombrada), no se quita ningún sitio: la carpeta muestra "carpeta no encontrada" y los sitios vuelven a funcionar cuando reaparece.

## Pausar y dejar de monitorear

```bash
locdev park disable ~/Development    # sin revisiones ni eventos; los sitios se quedan
locdev park enable ~/Development
locdev park remove ~/Development     # los sitios que vinieron de ella salen de LocDev
locdev park remove ~/Development --keep-sites
```

`park remove` detiene y quita solo los sitios que vinieron de la carpeta; los que agregaste a mano se quedan. Con `--keep-sites`, sus sitios quedan como sitios normales. En ningún caso se borra una carpeta o un archivo.

## Dónde se guarda

Las carpetas y sus reglas están en `config/parked.toml`, en la carpeta de configuración de LocDev ([Configuración](/docs/configuration)). Cada sitio que vino de una carpeta monitoreada guarda su origen en `parked` dentro de `projects.toml`.

## Problemas conocidos

- **"is itself a project":** intentaste monitorear la carpeta de un proyecto. Agrégala con `locdev add` o monitorea la carpeta de arriba.
- **"overlaps the parked folder":** la carpeta está dentro de otra monitoreada (o contiene una). Sube la profundidad de la de afuera.
- **"revisada cada 5 minutos" / "watching unavailable":** el sistema rechazó los eventos de archivos (en Linux, normalmente el límite `fs.inotify.max_user_watches`). Todo sigue funcionando, con hasta 5 minutos de retraso.
- **Un proyecto no aparece:** comprueba que tenga uno de los archivos que reconoce la detección, que no esté en una carpeta omitida, que la profundidad lo alcance y que no esté ignorado.
