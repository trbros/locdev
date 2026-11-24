# Solución de problemas

Elige la siguiente acción según el fallo observado. La [referencia de Doctor](/docs/doctor) explica cada comprobación. Los comandos usan `acme` como ejemplo: sustitúyelo por un proyecto registrado. El sitio no ejecuta estos comandos.

## Empieza por las evidencias

```bash
locdev version
locdev status
locdev doctor
locdev list
```

Anota versión, sistema operativo, arquitectura y primera comprobación fallida. Identifica la capa afectada antes de reinstalar todo el entorno.

## El daemon no responde

```bash
locdev start
locdev status
```

Comprueba que `locdev` y `locdevd` pertenecen a la misma instalación. Revisa los logs del daemon en el directorio de datos indicado en [Configuración](/docs/configuration). Para versiones incompatibles, consulta [Actualización](/docs/updating). Iniciar el daemon también puede iniciar proyectos con `auto_start` habilitado.

## Un proyecto no se inicia

```bash
locdev logs acme
locdev domain acme
```

Revisa directorio registrado, comando efectivo, runtime y dependencias. Instálalas con el gestor y lockfile del proyecto, fuera del sitio. Tras corregir la causa, reinicia solo ese proyecto:

```bash
locdev reload acme
```

## Puertos y proxy

Doctor distingue el puerto del proyecto de los puertos del proxy. Si 80/443 están ocupados, la configuración predeterminada permite usar 7080/7443 como alternativa. Usa el puerto indicado por el app; una URL sin puerto explícito puede apuntar a otro servidor.

Revisa [Configuración](/docs/configuration) antes de cambiar puertos. Detén otro servidor solo si sabes a quién pertenece; no interrumpas procesos desconocidos ni apliques privilegios automáticamente.

## El dominio .test no resuelve

```bash
locdev dns status
```

Si otra herramienta controla el resolvedor, investiga el conflicto primero. Solo si quieres dirigir `.test` a LocDev, ejecuta y revisa el cambio propuesto:

```bash
locdev dns install
```

Consulta [Dominios](/docs/domains) para `.localhost` y detalles por sistema.

## HTTPS muestra un aviso de certificado

```bash
locdev tls status
```

Revisa la CA local y el almacén de certificados del navegador. Si la CA esperada aún no es de confianza, consulta [HTTPS](/docs/https) y los cambios propuestos por:

```bash
locdev tls trust
```

Vuelve a abrir el navegador. No ignores un certificado distinto del esperado.

## El servidor de desarrollo rechaza el host

En los procesos que inicia LocDev, Vite ya recibe los nombres del sitio mediante la variable `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS` (consulta [Servidores de desarrollo](/docs/servers)). Si Vite corre fuera de LocDev, agrega un dominio específico que controlas, como `acme.test`, a `server.allowedHosts`. No uses `true` para aceptar cualquier host. Consulta las [opciones oficiales de Vite](https://vite.dev/config/server-options.html#server-allowedhosts) y adapta la configuración al framework instalado.

## Cuándo pedir ayuda

Usa [Soporte](/support) con pasos de reproducción y logs sin contraseñas, tokens, archivos `.env` ni datos personales. Para posibles vulnerabilidades, utiliza el [canal privado](/security), no una issue pública.
