# Git, GitHub y protección

## Git local

![Pestaña Git de un sitio, con archivos modificados y el diff](screenshot:git)

La pestaña **Git** de cada sitio trabaja en el repositorio de la carpeta del sitio, con el `git` instalado en la máquina (debe estar en el `PATH`). Tiene seis partes:

| Parte    | Qué hace                                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------------------- |
| Changes  | archivos modificados, separados en _staged_ y no _staged_, con el diff de cada uno; _stage_/_unstage_, descartar y commit |
| History  | historial paginado; cada commit muestra sus archivos y su diff, y las acciones a partir de él                             |
| Branches | locales y remotas, con upstream y ↑/↓; crear, cambiar, hacer merge en la actual y borrar                                  |
| Stashes  | guardar los cambios (con o sin archivos no rastreados), aplicar, _pop_ y descartar                                        |
| Remotes  | agregar, cambiar la URL y quitar                                                                                          |
| Activity | lo que LocDev ejecutó (y rechazó) en ese repositorio, y el _reflog_ de HEAD                                               |

Fetch, Pull y Push están arriba. **Pull es siempre `--ff-only`**: nunca crea un merge por su cuenta. Si la rama divergió, falla y decides tú (merge, rebase) en la terminal o en la pestaña Branches. Durante un merge con conflictos, un aviso muestra el estado y ofrece **Abort merge**.

Todos los comandos se ejecutan sin shell, con argumentos separados, `--end-of-options` antes de nombres que vienen del usuario y validación de nombres de rama (`git check-ref-format`). Las URL de remoto con `ext::` o que empiezan con `-` se rechazan. Git nunca pide contraseña de forma interactiva (`GIT_TERMINAL_PROMPT=0`; SSH en `BatchMode`): usa el agente SSH o el _credential helper_ que ya usas.

### Volver atrás: tres operaciones distintas

En el detalle de un commit, las opciones aparecen una al lado de la otra, cada una explicando lo que hace:

| Operación              | Qué pasa                                                                                                                            | Historial                                                             |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| **Revertir**           | crea un commit nuevo que deshace ese                                                                                                | preservado; seguro en ramas compartidas                               |
| **Restaurar archivos** | los archivos marcados vuelven a la versión de ese commit, como cambios sin commit                                                   | intacto                                                               |
| **Reset**              | la rama actual pasa a apuntar a ese commit (`soft`: los cambios quedan _staged_; `mixed`: quedan en el árbol; `hard`: se descartan) | los commits posteriores salen de la rama (el _reflog_ aún los guarda) |

También está **Nueva rama aquí**, que solo crea una rama en ese punto.

## Operaciones destructivas

![Confirmación de un reset hard, con lo que se perderá](screenshot:git-confirm)

Estas operaciones piden confirmación explícita antes de ejecutarse:

- descartar cambios (incluido borrar archivos no rastreados);
- `reset` (cualquier modo; `hard` también lista los archivos que pierden cambios);
- _force push_ (siempre `--force-with-lease`; lista los commits del remoto que desaparecen);
- borrar una rama que no se integró;
- descartar un stash;
- restaurar archivos que tienen cambios;
- abortar un merge;
- quitar un remoto.

El daemon calcula la vista previa: lo que se perderá (archivos, commits, el stash), lo que se guardará antes y si la operación está bloqueada (por ejemplo, `reset` durante un merge, o borrar la rama actual). Para confirmar, escribes una frase que depende de la operación (el nombre de la rama, el SHA corto del destino del reset, `discard`, `drop`…). **No existe un `--yes` que se salte este paso.** La verificación ocurre de nuevo en el daemon, así que un cliente no puede eludirla.

### Copias de seguridad

Con **Hacer copia antes** activado (predeterminado), antes de una operación destructiva LocDev:

- graba un _snapshot_ de los cambios rastreados (`git stash create`) y una ref al commit del que sale la rama, en `refs/locdev/backups/<fecha>-<operación>`;
- copia los archivos no rastreados que se borrarían a `~/.local/share/locdev/backups/<sitio>/<fecha>/` (la carpeta de datos de LocDev en cada SO).

Para recuperar: `git log refs/locdev/backups/...` o `git stash apply <sha>` en la terminal, o crea una rama a partir del _reflog_ en la pestaña Activity. Las copias no se borran automáticamente.

### Contraseña de LocDev

![Settings → Security](screenshot:settings-security)

En **Settings → Security** (o `locdev security password`) puedes definir una contraseña para las operaciones destructivas.

- Solo se guarda el hash **Argon2id**, en `security.toml` en la carpeta de configuración, con permiso `0600`. La contraseña no queda en memoria entre peticiones.
- Tras 5 errores, los nuevos intentos se bloquean durante 1 minuto, duplicándose con cada error hasta 1 hora. El bloqueo sobrevive al reinicio del daemon.
- Cambiar o quitar la contraseña, o desactivar "Pedir la contraseña"/"Hacer copia antes", exige la contraseña actual.
- No hay recuperación. Si olvidas la contraseña, borra `security.toml`. Eso quita la protección, y es coherente con su objetivo: protege contra descuidos, no contra ti.

> [!WARNING]
> **Límite importante:** la contraseña protege las operaciones iniciadas **por LocDev** (aplicación y CLI). No protege el repositorio. El `git` de la terminal, el editor o cualquier otra herramienta siguen funcionando normalmente.

## GitHub

![Pestaña GitHub de un sitio, con ejecuciones de workflow](screenshot:github)

La pestaña **GitHub** aparece en los sitios cuyo remoto apunta a github.com. Muestra un resumen (visibilidad, rama predeterminada, _comprobaciones_ del commit actual), issues, pull requests, ejecuciones de workflow, workflows y releases. Los elementos se abren en el navegador.

Las acciones que cambian algo en GitHub siempre piden confirmación e indican con qué cuenta se ejecutarán:

- **Volver a ejecutar los fallidos**: vuelve a ejecutar los jobs que fallaron;
- **Cancelar**: cancela una ejecución en curso;
- **Run workflow**: dispara un workflow con disparador `workflow_dispatch` en una rama o tag.

### Conectar una cuenta

En **Settings → Integrations** (o `locdev github connect`), hay tres formas:

| Forma          | Cómo                                                                                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Token personal | pega un token _fine-grained_ (recomendado, limitado a los repositorios que elijas) o clásico. En la CLI se lee de la entrada estándar, nunca de un argumento |
| GitHub CLI     | copia una vez el token de un `gh` ya autenticado (`gh auth token`)                                                                                           |
| Navegador      | el _device flow_ de OAuth. Requiere el _client id_ de una OAuth App tuya con device flow habilitado; LocDev no distribuye una                                |

Para el token _fine-grained_, da lectura en _Metadata_, _Issues_, _Pull requests_ y _Actions_. Para volver a ejecutar, cancelar o disparar workflows, da también escritura en _Actions_.

El token se valida en GitHub y se guarda **solo en el llavero del sistema**: Secret Service en Linux (GNOME Keyring, KWallet), Keychain en macOS y el Administrador de credenciales en Windows. Nunca va a un archivo ni a un log. Sin un llavero disponible, la conexión falla en lugar de recurrir a un archivo. El daemon solo hace peticiones a `api.github.com`, y **Desconectar** borra el token del llavero.

### Fallos de workflow

Con la cuenta conectada, cada 15 minutos el daemon revisa las ejecuciones de los sitios con remoto en GitHub y crea una notificación (categoría _Fallos de workflow_) por cada ejecución nueva que falló. La primera revisión solo registra lo que ya existe, para no generar una avalancha de fallos antiguos. Se puede desactivar en Integrations y se detiene en modo sin conexión.

## En la línea de comandos

```bash
locdev git status            # rama, upstream y archivos modificados
locdev git log -n 50
locdev git branches
locdev git history           # lo que ejecutó LocDev + reflog
locdev security password     # define/cambia la contraseña
locdev github connect --gh   # usa el token de GitHub CLI
locdev github status
```

La CLI solo lee el repositorio. Para cambiarlo, usa el propio `git` o la aplicación.

## Worktrees

**Git → Worktrees** (o `locdev git worktree`) hace checkout de una rama en su propia carpeta, junto al repositorio como `<repo>-<rama>`, para que dos ramas se ejecuten a la vez.

- **Registrarlo como sitio propio**: la carpeta se convierte en un sitio (`<repo>-<rama>.test`) con su propio puerto y proceso.
- **Copiar los archivos .env que falten**: se copian los `.env*` sin seguimiento de la carpeta principal (no están en Git); nada se sobrescribe.

```bash
locdev git worktree add feature/login --create --site-for-it --copy-env
locdev git worktree list
locdev git worktree remove ../tienda-feature-login
locdev git worktree prune
```

Eliminar uno tiene la misma protección que las otras operaciones destructivas de Git: la vista previa lista la carpeta y cada cambio sin commit, pide el nombre de la carpeta (y la contraseña de LocDev, si hay una), guarda una instantánea de los cambios con seguimiento en `refs/locdev/backups/` y elimina el sitio registrado para esa carpeta. Los cambios sin commit lo bloquean salvo que lo fuerces. La rama y sus commits siguen en el repositorio.
