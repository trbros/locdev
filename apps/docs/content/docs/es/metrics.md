# Métricas de solicitudes

Cada solicitud a un sitio a través del proxy de LocDev (`<sitio>.test`, sus subdominios, HTTP y HTTPS) se cuenta. La pestaña **Métricas** del sitio (o `locdev metrics <sitio>`) muestra, actualizado cada 3 segundos:

- solicitudes y errores 5xx;
- latencia (promedio, p50, p95, p99 y la más lenta) como el tiempo hasta las cabeceras de la respuesta, en las últimas 1.000 solicitudes;
- solicitudes por minuto en la última hora;
- códigos de estado por clase, más las solicitudes que el servidor del sitio no respondió;
- las rutas más visitadas (método y ruta, sin la query) con su tiempo promedio y errores;
- las últimas 100 solicitudes.

## Qué se cuenta y qué no

Los números viven solo en la memoria del daemon: nunca se escriben en disco ni se envían a ningún lado. Reiniciar el daemon, o **Reiniciar** (`locdev metrics <sitio> --reset`), empieza de cero. Las solicitudes directas a `127.0.0.1:<puerto>` no pasan por el proxy y no se cuentan. Un upgrade de WebSocket cuenta como una solicitud (101).

La memoria tiene límite por sitio: 1.000 latencias, 60 minutos, 100 muestras y 300 rutas distintas (las demás rutas siguen contando en los totales).

```bash
locdev metrics
locdev metrics tienda --json
locdev metrics tienda --reset
```

Las respuestas 5xx y un servidor inaccesible también aparecen en el [Debug Center](/docs/debug-center).
