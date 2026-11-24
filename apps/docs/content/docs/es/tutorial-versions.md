# Tutorial: versiones de PHP y Node.js por proyecto y en la terminal

Cada proyecto puede ejecutarse con su propia versión, y la terminal puede usar la versión predeterminada que elijas en la app. En este tutorial instalas dos versiones, fijas una en un sitio y empiezas a usar la predeterminada en la terminal.

## 1. Instala las versiones

```bash
locdev runtime available php        # la más nueva de cada línea
locdev runtime install php 8.4
locdev runtime install php 8.3
locdev runtime install node lts
```

En la app, **Runtimes → PHP** (o **Node.js**) **→ Instalar versión…**. Las versiones que ya están en la máquina (paquetes del sistema, Homebrew, nvm, fnm) también aparecen y se pueden usar; LocDev nunca las modifica.

## 2. Fija una versión en un sitio

```bash
locdev edit tienda --php 8.3
locdev reload tienda
locdev info tienda                   # php 8.3.x (projects.toml 8.3)
```

Una versión fijada puede ser la línea (`8.3`, que sigue la versión más nueva instalada de ella) o una versión exacta (`8.3.37`). Sin ella, LocDev usa el archivo de versión del proyecto (`.php-version`, `.nvmrc`…), luego el rango que declara el proyecto (`require.php`, `engines.node`) y, por último, la versión predeterminada.

## 3. Elige la versión predeterminada

```bash
locdev runtime default php 8.4
locdev runtime default node 22
```

La predeterminada vale para los sitios sin versión fijada. En la app, está en el menú **⋯** de cada versión.

## 4. Usa la predeterminada en la terminal

Activa **Usar las versiones predeterminadas en la terminal** en **Runtimes → Resumen**, o:

```bash
locdev runtime terminal on
```

![Runtimes: las versiones predeterminadas y la terminal.](screenshot:runtimes-terminal)

Abre una terminal nueva: `php -v`, `node -v`, `npm` y `composer` pasan a ser las versiones predeterminadas. Cambiar la predeterminada en la app cambia lo que usa la terminal, sin reabrir nada. Para volver a como estaba:

```bash
locdev runtime terminal off
```

En Linux y macOS LocDev añade un bloque marcado a `~/.bashrc` y `~/.zshrc` (y un archivo propio en fish); al desactivarlo se quita solo ese bloque. En Windows, la carpeta va al principio del `Path` del usuario.

## 5. Actualiza y elimina con seguridad

```bash
locdev runtime update php 8.4        # instala el parche más nuevo de la línea
locdev runtime uninstall php 8.3.37
```

La versión antigua sigue instalada después de actualizar, para no romper los sitios que fijaron la versión exacta. Eliminar se rechaza mientras un sitio en ejecución usa la versión.
