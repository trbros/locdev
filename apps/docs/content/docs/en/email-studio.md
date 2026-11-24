# Email Studio

Each site's **Email Studio** tab brings together the project's email templates and components, with a visual block editor, live preview and test sends to the [Mailbox](/docs/mailbox). The files edited are the project's own, and Blade stays intact.

![Email Studio: blocks, the structure with Blade code kept, and the preview.](screenshot:email-studio)

```bash
locdev new email contact                   # Laravel: make:mail + a view with blocks
locdev new email receipt --markdown        # Markdown mailable
locdev new email component button          # <x-mail.button>
locdev email list                          # templates, mailables and components
locdev email preview <path> --framework    # rendered by Laravel itself
locdev email send-test <path>              # to the Mailbox, never outside
```

## What is discovered

| Stack        | Templates                                                                                                                                                                                                                                                                                                  | Components                                                                                                                                                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel      | views in `resources/views` inside `mail`, `mails`, `email` or `emails` folders, and any view a Mailable in `app/Mail` uses (`view:`, `html:`, `text:`, `markdown:`). Shows the Mailable, the variables (public properties and `with:` keys) and the format: Blade, Markdown (`<x-mail::message>`) or text. | `resources/views/components/mail/*.blade.php`, used as `<x-mail.name>`. Properties come from Blade's own `@props([...])`. Laravel's Markdown components (`x-mail::button`, `panel`…) show up in Markdown templates. |
| Other stacks | `emails/`, `templates/emails/` (Django), `app/views/*_mailer/` (Rails), `.mjml` and React Email (`.tsx`)                                                                                                                                                                                                   | `emails/components/*.html` with `<!-- locdev:component {"props":[...]} -->`                                                                                                                                         |

Nothing runs to discover them: files are only read. MJML, React Email, ERB and Django templates are edited as code; visual blocks work in Blade, Markdown mailables and HTML.

## How editing preserves the template

The editor works with **segments**:

- **LocDev blocks** (header, heading, text, button, image, columns, divider, spacer, footer, panel, component). Each sits between two comments in the template itself, which don't go into the sent email:

  ```blade
  {{-- locdev:block {"type":"button","props":{"text":"Pay","url":"{{ $url }}"},"h":"3fa1c2d0"} --}}
  <tr><td …><a href="{{ $url }}" …>Pay</a></td></tr>
  {{-- /locdev:block --}}
  ```

  Texts accept Blade expressions (`{{ $user->name }}`), which go into the file as written; the rest is escaped. In components, a value starting with `$` becomes a binding (`:url="$url"`).
- **Code:** everything outside the blocks, such as `@if`, `@foreach`, `{!! !!}`, `@props` and the layout, is kept exactly as it is. Code can be dragged and edited as text, but **the visual editor never rewrites it**.

Opening and saving without changes gives back the file byte for byte. When a block changes, only its lines change.

**Edited in your IDE?** Each block keeps a hash of the markup it generated. If that markup was changed outside LocDev, the block shows as _edited_ and is kept as you wrote it, until you ask for **Regenerate from properties**.

## Syncing with the IDE

- **Outside changes:** the app checks the file every 2 seconds. If it changed and you have no pending changes, the screen reloads by itself. If you do, a notice offers **Load theirs** (discards yours) or **Keep mine** (when saving, the diff shows what will change relative to the new file).
- **Saving:** always shows the diff first and only writes if the file is still the one you reviewed. The previous version goes to the history, and the write is atomic.
- **Undo and redo:** `Ctrl+Z` and `Ctrl+Y` work inside the editor.

## Preview and sample data

- **Static preview:** LocDev substitutes sample data (`$user->name`, `$order['id']`, `?? 'default'`, `config()`, `__()`, `route()`, `date('Y')`) and expands the project's components, **without running anything**. Blade logic isn't evaluated, so both sides of an `@if` show, and the screen says so. The selected block is highlighted in the preview.
- **Framework preview (Laravel):**
  - Laravel itself renders it, with `view()` or the mailables' `Markdown::render`;
  - it runs in a **separate process**, with a reduced environment (the site's PHP, `HOME`, `APP_ENV=local` and mail pointed at the Mailbox), a 25-second limit and limited output;
  - unsaved content goes to a LocDev temporary file, never into the project;
  - it needs `vendor/` installed.
- **Views:** desktop or mobile, the generated HTML, the text version and the file that would be written.
- **Sample data:** suggested from the template's variables, editable as JSON and kept by LocDev, outside the project. **Send test** renders and sends to the Mailbox.

## Generator

`locdev new email <name>` follows the framework's structure and **never overwrites** an existing file:

- **Laravel:** runs `php artisan make:mail <Name>Mail --view=emails.<name>` (or `--markdown`), Laravel's own generator, and fills the new view with a block template. Without PHP or `vendor/`, it creates only the view and shows the command to create the Mailable.
- **Components:** `resources/views/components/mail/<name>.blade.php`, with `@props` and a description in a comment. `button`, `header` and `footer` come with their own template.
- **Other stacks:** `emails/<name>.html` and `emails/components/<name>.html`.

In the app, the same lives in **New template or component**. The project can be chosen explicitly with `--site`.

## Limitations

- Plain HTML components don't become visual blocks yet; edit them as code.
- The code editor has syntax highlighting (Blade, HTML, Markdown), search (`Ctrl+F`) and undo, but no autocomplete for the template's variables.
- The framework preview only exists for Laravel. On other stacks, use the static preview or the framework's own tool.
