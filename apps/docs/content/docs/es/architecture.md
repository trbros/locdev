# Arquitectura

```text
          ┌────────────┐     ┌──────────────┐
          │ app (Tauri)│     │  locdev CLI  │
          └─────┬──────┘     └──────┬───────┘
                │ IPC local (JSON)   │
                └─────────┬──────────┘
                    ┌─────▼──────┐
                    │  locdevd   │  sin privilegios
                    └─┬───┬───┬──┘
        ┌─────────────┘   │   └──────────────┐
  ┌─────▼──────┐   ┌──────▼──────┐   ┌───────▼──────┐
  │ supervisor │   │ proxy HTTP/S│   │  DNS *.test  │
  │ procesos   │   │ SNI + CA    │   │ 127.0.0.1    │
  └────────────┘   └─────────────┘   └──────────────┘
```

## Crates

| Crate               | Responsabilidad                                                                                                  |
| ------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `locdev-core`       | Modelos, errores, rutas, detección de proyectos, análisis de hosts y el protocolo IPC.                           |
| `locdev-ipc`        | Transporte local, cliente y lanzador del daemon (compartido por la CLI y la aplicación).                         |
| `locdev-daemon`     | `locdevd`: registro, supervisor, proxy, DNS, TLS y doctor.                                                       |
| `locdev-proxy`      | Proxy inverso por hostname, con WebSocket y HTTPS.                                                               |
| `locdev-dns`        | Servidor DNS autoritativo mínimo, escrito a mano.                                                                |
| `locdev-tls`        | CA local, emisión de certificados y verificación de confianza.                                                   |
| `locdev-platform`   | Integración con el SO: resolvedor, almacén de confianza, inicio de sesión y elevación.                           |
| `locdev-runtimes`   | Versiones de Node.js, PHP y gestores: catálogo, descarga verificada, extracción segura, detección y shims.       |
| `locdev-frameworks` | Definiciones de frameworks (datos firmados) y el motor de detección.                                             |
| `locdev-env`        | Variables compartidas: lectura de `.env`, reglas por framework, resolución, sincronización y el almacén cifrado. |
| `locdev-services`   | Bases de datos y cachés: instalaciones, instancias y copias en frío.                                             |
| `locdev-db`         | Database Manager: drivers de PostgreSQL, MySQL/MariaDB, SQLite y Redis/Valkey.                                   |
| `locdev-mail`       | Mailbox: SMTP solo de captura, almacenamiento y análisis de mensajes.                                            |
| `locdev-email`      | Email Studio: bloques entre comentarios, descubrimiento, vista previa y generador.                               |
| `locdev-config`     | Config Files: catálogo de opciones por versión, edición de JSON y JS/TS sin ejecutar, reglas.                    |
| `locdev-deploy`     | Asistente de deploy: perfil de la stack, plataformas con fuentes, preparación y archivos generados.              |
| `locdev-deps`       | Dependencias: lectura local, registros npm/Packagist, OSV y comandos de actualización.                           |
| `locdev-git`        | El `git` del sistema sin shell: status, diff, commit, ramas, stash, remotos, reset/revert/restore y copias.      |
| `locdev-github`     | Token en el llavero, cliente de la API (solo `api.github.com`) y device flow.                                    |
| `locdev-actions`    | GitHub Actions builder: plantillas, YAML ↔ modelo, validación y escritura segura.                                |
| `apps/locdev`       | La CLI.                                                                                                          |
| `apps/desktop`      | La aplicación Tauri + React (cliente ligero del daemon).                                                         |

Todo el código Rust compila con `unsafe_code = "forbid"`.

## Protocolo IPC

Los mensajes son JSON con un prefijo de tamaño de 4 bytes, sobre un socket Unix (`0600`, en un directorio `0700` verificado) o un named pipe por usuario que rechaza clientes remotos. Cada mensaje lleva la versión del protocolo:

```json
{ "protocol": 2, "body": { "type": "start_project", "name": "blog" } }
```

Una versión incompatible se convierte en un error claro (`INCOMPATIBLE_PROTOCOL`), sin tumbar el daemon. La aplicación y la CLI envían exactamente las mismas peticiones.

## Operaciones privilegiadas

El daemon nunca corre como administrador. Cuando algo requiere privilegios (resolvedor, almacén de confianza), `locdev-platform` genera un **plan**: un script legible, con un paso como administrador y, opcionalmente, otro como usuario. La CLI y la aplicación muestran el plan y solo lo ejecutan tras la confirmación, mediante el aviso del propio SO (`sudo`, polkit, el diálogo de macOS o UAC). La aplicación recalcula el plan en el lado Rust, así que lo que se ejecuta es lo que se mostró.
