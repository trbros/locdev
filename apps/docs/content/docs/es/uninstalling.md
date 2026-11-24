# Desinstalar LocDev

Decide si quieres quitar un proyecto registrado, el inicio automático o toda la instalación. Desinstalar el app no es lo mismo que borrar el código de tus proyectos. Haz un respaldo antes de modificar datos locales.

## Quitar solo un proyecto

Sustituye `acme` por el nombre registrado:

```bash
locdev down acme
locdev remove acme
```

`remove` elimina el registro, no los archivos del proyecto. Si ese era el objetivo, no necesitas desinstalar el app ni quitar DNS y certificados que otros proyectos utilizan.

## Preparar la eliminación completa

Mantén la CLI disponible y el daemon en ejecución durante la preparación: los comandos DNS y TLS consultan su estado. Cierra el escritorio para que no vuelva a abrir el daemon. Detén los proyectos activos con `locdev down` y sus nombres.

Si el inicio de sesión automático está habilitado, quítalo antes de borrar los ejecutables:

```bash
locdev startup status
locdev startup uninstall
```

Esto elimina el mecanismo de inicio, no los binarios. Consulta [Inicio automático](/docs/service) para detalles por sistema.

## Revertir DNS y confianza TLS

Los cambios afectan al entorno LocDev del usuario y a otros proyectos locales. Úsalos solo si quieres retirar esa integración del sistema. Revisa los planes y las solicitudes de autorización; no uses confirmación automática.

Con el daemon todavía disponible:

```bash
locdev dns uninstall
locdev tls untrust
```

La primera operación revierte la integración del resolvedor `.test`; la segunda retira la confianza en la CA local de los almacenes gestionados por el app. No borran proyectos. Si alguna falla, revisa el estado y consulta [Solución de problemas](/docs/troubleshooting) antes de continuar.

## Detener y quitar binarios

Tras la preparación:

```bash
locdev stop
```

Esto detiene el daemon y los proyectos restantes. Usa el desinstalador o gestor correspondiente a la instalación original. Para una CLI extraída manualmente, elimina solo `locdev` y `locdevd` de la carpeta elegida y revisa la entrada del `PATH`.

## Decidir qué datos conservar

Configuración, registro de proyectos, logs, runtimes gestionados y datos de la CA pueden quedar separados de los binarios. Consulta las rutas reales en [Configuración](/docs/configuration), incluido un posible `LOCDEV_HOME`. Haz respaldo y revisa cada directorio antes de borrarlo manualmente. No elimines proyectos, instalaciones externas de PHP/Node ni directorios compartidos.

Vuelve a abrir el navegador tras retirar la confianza TLS y revisa las preferencias de inicio del sistema. Para reinstalar, sigue [Instalación](/docs/installation); no supongas que los datos de otra versión sean compatibles.
