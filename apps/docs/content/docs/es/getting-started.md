# Inicio rápido

## 1. Inicia el daemon

La aplicación lo hace al abrirse. Desde la terminal:

```bash
locdev start
locdev status
```

```text
● daemon running  pid 41628 · up 3s
  version   0.6.0 (protocol 2)
  projects  0 registered, 0 running
  proxy     127.0.0.1:80 · *.localhost
  dns       127.0.0.1:10053 · *.test not resolving
  https     127.0.0.1:443 · CA not trusted (locdev tls trust)
```

## 2. Registra un proyecto

Dentro de la carpeta del proyecto:

```bash
locdev add . --start
```

LocDev detecta el tipo de proyecto, sugiere el comando del servidor de desarrollo, elige un puerto libre (desde el 8100) e inicia el sitio. En la aplicación, usa `Ctrl` `N` o el botón **+**.

![Agregar sitio: solo la carpeta es obligatoria.](screenshot:add-site)

## 3. Ábrelo en el navegador

Justo después de instalar, el sitio ya responde en `http://nombre.localhost`. Los navegadores resuelven `*.localhost` por sí mismos, sin configurar nada.

Para usar `nombre.test` con HTTPS sin avisos, configura dos cosas, una sola vez:

```bash
locdev dns install   # *.test resuelve hacia LocDev
locdev tls trust     # el sistema y los navegadores confían en la CA local
```

Ambos comandos muestran exactamente lo que se ejecutará y piden confirmación antes. Consulta [Proxy y DNS](/docs/domains) y [HTTPS](/docs/https).

## 4. Revisa el entorno

```bash
locdev doctor
```

`doctor` señala puertos ocupados, un resolvedor sin configurar, una CA no confiable y herramientas que faltan, cada uno con el comando que lo resuelve.

## Próximos pasos

Sigue uno de los tutoriales de principio a fin:

- [Un sitio Laravel desde cero](/docs/tutorial-laravel): PHP descargado por LocDev, proyecto nuevo, HTTPS y Mailbox.
- [Un proyecto Node.js que ya tienes](/docs/tutorial-node): la carpeta, su versión de Node.js y un túnel para mostrarlo.
- [Versiones por proyecto y en la terminal](/docs/tutorial-versions): instalar, fijar y usar la predeterminada en la terminal.
