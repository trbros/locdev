# Email Studio

La pestaña **Email Studio** de cada sitio reúne las plantillas y componentes de correo del proyecto, con un editor visual de bloques, vista previa en tiempo real y envío de prueba al [Mailbox](/docs/mailbox). Los archivos editados son los del proyecto, y el Blade queda intacto.

![Email Studio: bloques, la estructura con el código Blade preservado y la vista previa.](screenshot:email-studio)

```bash
locdev new email contact                   # Laravel: make:mail + vista con bloques
locdev new email receipt --markdown        # Markdown mailable
locdev new email component button          # <x-mail.button>
locdev email list                          # plantillas, mailables y componentes
locdev email preview <ruta> --framework    # renderizado por el propio Laravel
locdev email send-test <ruta>              # al Mailbox, nunca hacia fuera
```

## Qué se descubre

| Stack        | Plantillas                                                                                                                                                                                                                                                                                                                   | Componentes                                                                                                                                                                                                                                     |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel      | vistas en `resources/views` dentro de carpetas `mail`, `mails`, `email` o `emails`, y cualquier vista que use un Mailable de `app/Mail` (`view:`, `html:`, `text:`, `markdown:`). Muestra el Mailable, las variables (propiedades públicas y claves de `with:`) y el formato: Blade, Markdown (`<x-mail::message>`) o texto. | `resources/views/components/mail/*.blade.php`, usados como `<x-mail.nombre>`. Las propiedades vienen del propio `@props([...])` de Blade. Los componentes Markdown de Laravel (`x-mail::button`, `panel`…) aparecen en las plantillas Markdown. |
| Otras stacks | `emails/`, `templates/emails/` (Django), `app/views/*_mailer/` (Rails), `.mjml` y React Email (`.tsx`)                                                                                                                                                                                                                       | `emails/components/*.html` con `<!-- locdev:component {"props":[...]} -->`                                                                                                                                                                      |

No se ejecuta nada para descubrirlos: los archivos solo se leen. Las plantillas MJML, React Email, ERB y Django se editan como código; los bloques visuales funcionan en Blade, Markdown mailables y HTML.

## Cómo la edición preserva la plantilla

El editor trabaja con **segmentos**:

- **Bloques de LocDev** (encabezado, título, texto, botón, imagen, columnas, divisor, espacio, pie, panel, componente). Cada uno está entre dos comentarios de la propia plantilla, que no van al correo enviado:

  ```blade
  {{-- locdev:block {"type":"button","props":{"text":"Pagar","url":"{{ $url }}"},"h":"3fa1c2d0"} --}}
  <tr><td …><a href="{{ $url }}" …>Pagar</a></td></tr>
  {{-- /locdev:block --}}
  ```

  Los textos aceptan expresiones Blade (`{{ $user->name }}`), que van al archivo tal como se escribieron; el resto se escapa. En los componentes, un valor que empieza con `$` se convierte en binding (`:url="$url"`).
- **Código:** todo lo que está fuera de los bloques, como `@if`, `@foreach`, `{!! !!}`, `@props` y el layout, se guarda exactamente como está. El código se puede arrastrar y editar como texto, pero **el editor visual nunca lo reescribe**.

Abrir y guardar sin cambios devuelve el archivo byte a byte. Al cambiar un bloque, solo cambian sus líneas.

**¿Editaste en el IDE?** Cada bloque guarda un hash del marcado que generó. Si se modificó fuera de LocDev, el bloque aparece como _editado_ y se mantiene como lo escribiste, hasta que pidas **Regenerar a partir de las propiedades**.

## Sincronización con el IDE

- **Cambios externos:** la aplicación revisa el archivo cada 2 segundos. Si cambió y no tienes cambios pendientes, la pantalla se recarga sola. Si los tienes, un aviso ofrece **Cargar las suyas** (descarta los tuyos) o **Mantener las mías** (al guardar, el diff muestra qué cambiará respecto al archivo nuevo).
- **Guardar:** siempre muestra el diff antes y solo escribe si el archivo sigue siendo el que revisaste. La versión anterior va al historial, y la escritura es atómica.
- **Deshacer y rehacer:** `Ctrl+Z` y `Ctrl+Y` funcionan dentro del editor.

## Vista previa y datos de ejemplo

- **Vista previa estática:** LocDev sustituye los datos de ejemplo (`$user->name`, `$order['id']`, `?? 'predeterminado'`, `config()`, `__()`, `route()`, `date('Y')`) y expande los componentes del proyecto, **sin ejecutar nada**. La lógica de Blade no se evalúa, así que aparecen ambos lados de un `@if`, y la pantalla lo avisa. El bloque seleccionado queda resaltado en la vista previa.
- **Vista previa por el framework (Laravel):**
  - quien renderiza es el propio Laravel, con `view()` o el `Markdown::render` de los mailables;
  - corre en un **proceso separado**, con entorno reducido (el PHP del sitio, `HOME`, `APP_ENV=local` y el correo apuntado al Mailbox), límite de 25 segundos y salida limitada;
  - el contenido sin guardar va a un archivo temporal de LocDev, nunca al proyecto;
  - requiere `vendor/` instalado.
- **Vistas:** escritorio o móvil, el HTML generado, la versión en texto y el archivo que se escribiría.
- **Datos de ejemplo:** se sugieren a partir de las variables de la plantilla, se editan en JSON y LocDev los guarda fuera del proyecto. **Enviar prueba** renderiza y envía al Mailbox.

## Generador

`locdev new email <nombre>` sigue la estructura del framework y **nunca sobrescribe** un archivo existente:

- **Laravel:** ejecuta `php artisan make:mail <Nombre>Mail --view=emails.<nombre>` (o `--markdown`), el generador del propio Laravel, y llena la vista recién creada con una plantilla en bloques. Sin PHP o `vendor/`, crea solo la vista y muestra el comando para crear el Mailable.
- **Componentes:** `resources/views/components/mail/<nombre>.blade.php`, con `@props` y una descripción en comentario. `button`, `header` y `footer` traen su propia plantilla.
- **Otras stacks:** `emails/<nombre>.html` y `emails/components/<nombre>.html`.

En la aplicación, lo mismo está en **Nueva plantilla o componente**. El proyecto se puede elegir explícitamente con `--site`.

## Limitaciones

- Los componentes de HTML puro aún no entran como bloque visual; edítalos como código.
- El editor de código tiene resaltado de sintaxis (Blade, HTML, Markdown), búsqueda (`Ctrl+F`) y deshacer, pero no autocompletado de las variables de la plantilla.
- La vista previa por el framework existe solo para Laravel. En las demás stacks, usa la vista previa estática o la herramienta del propio framework.
