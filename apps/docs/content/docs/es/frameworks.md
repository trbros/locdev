# Frameworks y stacks

Al agregar una carpeta, LocDev descubre de qué está hecho el proyecto **solo leyendo archivos**: manifiestos (`package.json`, `composer.json`, `Gemfile`, `pyproject.toml`, `requirements.txt`, `Cargo.toml`, `go.mod`, `pom.xml`, `build.gradle`, `*.csproj`), lockfiles y archivos de configuración. No se ejecuta ningún script del proyecto.

El resultado es una lista de **componentes** (framework, runtime, lenguaje, UI, herramientas), cada uno con la evidencia que lo probó y la versión cuando se puede saber: la instalada (`node_modules`, `vendor/composer/installed.json`, `Gemfile.lock`, `uv.lock`, `poetry.lock`, `Cargo.lock`, `pom.xml`) o, sin ella, el rango declarado. Uno de ellos es el **principal**, el adaptador que sabe iniciar el servidor de desarrollo.

```bash
locdev stack detect ~/Code/api     # cualquier carpeta
locdev stack inspect shop          # un sitio, con evidencias, versiones y requisitos
```

## Integración

| Tipo                   | Significa                                                                                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| nativa                 | LocDev arma el comando con host `127.0.0.1` y el puerto del sitio (`{port}`)                                                                         |
| script de package.json | ejecuta un script del proyecto (`dev`, `start`…); cuando el framework no acepta el puerto como argumento, la aplicación debe leer la variable `PORT` |
| comando personalizado  | el framework se reconoció, pero el comando de inicio tiene que venir de ti (`locdev edit <sitio> --command`)                                         |
| componente             | corre a través de otro (React por Vite, Tailwind por el build)                                                                                       |

Algunos adaptadores también pasan el puerto en las variables del propio framework: Spring Boot recibe `SERVER_PORT` y `SERVER_ADDRESS`, y .NET recibe `ASPNETCORE_URLS=http://127.0.0.1:<puerto>` (con `--no-launch-profile`, para que `launchSettings.json` no cambie la dirección), además de `ASPNETCORE_ENVIRONMENT=Development`, que el perfil de inicio definiría, salvo que el sitio ya tenga esa variable. Los proyectos Python con `.venv/` (o `venv/`) corren con el entorno virtual primero en el `PATH` y `VIRTUAL_ENV` definido.

Cuando varios frameworks coinciden (un NestJS también tiene Express), gana el de mayor prioridad **que sabe iniciar este proyecto**. Un Vite instalado sin script que lo ejecute, por ejemplo, no pasa a ser el principal; lo asume el Node.js genérico. Las versiones más antiguas que las que admite el adaptador aparecen como aviso, sin bloquear.

## Monorepos

Se leen los workspaces de npm/Yarn (`workspaces`), `pnpm-workspace.yaml`, el `[workspace]` de Cargo y `go.work`, y cada paquete aparece con su propio framework. Cada paquete se convierte en un sitio separado (`locdev add apps/web`). En la raíz de un workspace de Cargo sin `[package]` no se sugiere ningún comando.

## Elecciones manuales

Cuando la detección no basta:

```bash
locdev stack add tailwind shop                 # un componente que los archivos no prueban
locdev stack remove python shop                # oculta un componente detectado
locdev stack use flask shop --apply-command    # elige el adaptador y usa su comando
locdev stack use auto shop                     # vuelve a la detección
```

Las elecciones quedan en `projects.toml` (`adapter`, `stacks`, `hidden_stacks`). En la aplicación, la sección **Stack** del resumen del sitio tiene el mismo control (**Editar…**).

## Definiciones actualizables

Lo que es cada framework (reglas de detección, origen de la versión, comandos, archivos de configuración, reglas de `.env`, enlaces a la documentación oficial) está en **definiciones**: datos puros, separados del código. LocDev trae un conjunto integrado y puede descargar uno más nuevo sin otra versión de la aplicación:

- el conjunto se publica firmado con minisign, y LocDev solo usa un conjunto cuya firma coincide con la clave pública que trae;
- el formato tiene versión. Un conjunto que requiere capacidades que esta versión no tiene se rechaza, y también los campos desconocidos;
- los comandos solo pueden usar programas de una lista fija (`npm`, `php`, `python3`, `bin/rails`, `cargo`, `dotnet`…), sin metacaracteres de shell, y rutas solo dentro del proyecto. Ni siquiera un conjunto firmado puede ejecutar otra cosa;
- solo una revisión más nueva reemplaza a la actual, y una revisión más antigua que la más nueva ya vista se rechaza;
- `locdev stack definitions --rollback` vuelve al conjunto anterior (o al integrado), y la revisión abandonada se omite en las comprobaciones automáticas;
- la comprobación es diaria (por la cola de jobs), respeta el modo sin conexión y se puede desactivar: `locdev settings set automation.definitions off`.

```bash
locdev stack definitions             # revisión en uso, origen, última comprobación
locdev stack definitions --refresh   # comprueba ahora
```

El estado también aparece en el `doctor` y en **Settings → Frameworks**. La publicación la hace la herramienta `tools/definitions` (build, firma, verificación), que además consulta los registros (npm, Packagist, PyPI, RubyGems) para avisar cuando sale un major nuevo de algún framework. Mientras no se publique ninguna actualización, LocDev usa las definiciones que trae, sin avisos.

## Stacks reconocidas

<!-- generated:frameworks -->

Los servicios que suele usar cada stack (bases de datos, cachés) aparecen en `locdev stack inspect`, y LocDev puede crear instancias de ellos ([Servicios](/docs/services)). Las colas, los programadores y los servidores WebSocket se convierten en [procesos auxiliares](/docs/servers) del sitio.
