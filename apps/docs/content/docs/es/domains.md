# Dominios .test (DNS)

## Cómo funciona

El daemon ejecuta un servidor DNS mínimo en `127.0.0.1:10053`, solo en loopback. Responde `*.test → 127.0.0.1` a cualquier profundidad (`blog.test`, `api.blog.test`, `cliente1.tenants.blog.test`) y rechaza cualquier otra zona. Para que el sistema operativo lo consulte **solo** para `.test`, LocDev usa el mecanismo nativo de cada sistema:

| SO                       | Mecanismo                       | Dónde                                           |
| ------------------------ | ------------------------------- | ----------------------------------------------- |
| Linux (systemd-resolved) | dominio de enrutamiento `~test` | `/etc/systemd/resolved.conf.d/locdev-test.conf` |
| macOS                    | resolvedor por dominio          | `/etc/resolver/test`                            |
| Windows                  | regla NRPT                      | regla con el comentario `Managed by LocDev`     |

Un archivo `hosts` no sirve porque no admite comodines: cada sitio y subdominio nuevo exigiría otra edición como administrador. LocDev nunca lo edita.

DNS y enrutamiento son cosas distintas. El DNS hace que el nombre llegue a esta máquina; el proxy decide a qué sitio y puerto va cada nombre ([Subdominios](/docs/subdomains)). `locdev route <host>` muestra ambos pasos y el certificado de un nombre.

## Activación

```bash
locdev dns install
```

El comando muestra el script exacto y solo lo ejecuta con tu confirmación, mediante `sudo`. En la aplicación, lo mismo está en **Settings → dns**, con el aviso de autorización del sistema.

![Settings → dns: servidor, resolvedor del sistema y conflictos con otras herramientas.](screenshot:settings-dns)

![Nada se ejecuta antes de que veas el script y lo autorices.](screenshot:authorize-dns)

Para deshacerlo:

```bash
locdev dns uninstall
```

La desinstalación solo quita lo que lleva la marca `Managed by LocDev`.

## Sin configurar nada: `.localhost`

Mientras `*.test` no resuelve, las URL de los sitios usan `.localhost`, que Chrome, Firefox, Safari y Edge resuelven a la máquina local por su cuenta. El proxy atiende ambos sufijos: `blog.test` y `blog.localhost` llegan al mismo sitio.

## Conflictos con otras herramientas

Si otra herramienta también reclama `.test` (por ejemplo un `yerd-test.conf` o `valet.conf` en systemd-resolved), LocDev muestra el archivo como conflicto, **pero nunca lo edita**. Con dos servidores para el mismo dominio, las consultas pueden volverse lentas o llegar a la herramienta equivocada; elimina el archivo de la otra si ya no la usas.

## Comprobar que funciona

El `doctor` resuelve un nombre aleatorio (`locdev-probe-<n>.test`) a través del sistema y verifica que haya llegado al servidor de LocDev. Así distingue tres situaciones:

- `*.test → LocDev`: todo bien;
- `*.test → another tool`: resuelve, pero mediante otra herramienta;
- `not resolving`: aún no configurado.

> [!NOTE]
> Vite bloquea los hosts desconocidos por defecto. En los procesos que inicia LocDev, se permiten los nombres del sitio (`blog.test` y `.blog.test`) mediante la variable `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS`, sin tocar `vite.config`. Si ejecutas Vite fuera de LocDev, agrega `server: { allowedHosts: [".test"] }`. `.localhost` siempre está permitido.

## Windows

En Windows la estrategia es NRPT (Name Resolution Policy Table), el enrutamiento por dominio del propio sistema:

- **Una regla para `.test` cubre todos los subdominios**, a cualquier profundidad. Es lo que el archivo `hosts` no puede hacer, y la razón por la que otras herramientas sufren con los comodines en Windows.
- **Solo afecta a `.test`:** el resto del DNS de la máquina sigue igual y no se modifica ningún adaptador de red.
- **La regla no admite puerto**, así que el servidor DNS de LocDev escucha en el puerto 53, que en Windows no requiere administrador. Prueba `127.0.0.1:53` y, si otro programa ya ocupa esa dirección (otro servidor DNS local, por ejemplo), pasa a una dirección de loopback propia, `127.53.0.1:53`. La regla apunta a la dirección que realmente quedó escuchando.
- Si luego ambas difieren, el `doctor` avisa y `locdev dns install` reescribe la regla. Las reglas NRPT de otras herramientas para `.test` aparecen como conflicto y nunca se modifican.
- **Chrome y Edge** detectan las reglas NRPT y, por ellas, dejan de usar su resolvedor propio y pasan por el de Windows. Esto está en el código de Chromium (`net/dns/public/win_dns_system_settings.cc` y `dns_config_service_win.cc`). Si el navegador tiene "DNS seguro" con un proveedor fijo, o si Firefox usa DNS sobre HTTPS, agrega `test` a las excepciones (en Firefox, `network.trr.excluded-domains`).
- **Instalar la regla requiere administrador.** La aplicación y la CLI muestran el script de PowerShell (`Add-DnsClientNrptRule`) antes de ejecutarlo, y Windows pide autorización (UAC).

> [!WARNING]
> El comportamiento en Windows se verificó en el código y en pruebas en Linux (el cambio de dirección funciona igual). Aún no se probó en un Windows real.
