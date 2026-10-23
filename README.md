# Visor de capturas

Visor de capturas de pantalla de juegos, **lanzador de juegos** y capturador, para **pantalla grande y mando**, con una interfaz muy inspirada en el menú de la Wii U (marcos de cristal translúcido, fondo con burbujas e iconos fantasma, acentos de arcoíris, pistas de botones A/B/X/Y).

> Estado: **versión preliminar** (visor + lanzador + capturas propias). Ver [Hecho y pendiente](#hecho-y-pendiente) y, sobre todo, [qué está verificado y qué no](#verificación-realizada-en-esta-versión).

Pila: **Tauri 2 + Svelte 5 + TypeScript + Vite** en el frontend y **Rust** en el núcleo. Sin librerías de componentes: el CSS es el del mockup aprobado.

## Qué hace

- Inicio con los iconos de plataforma (Juegos PC, Juegos Switch, Novelas visuales…), la fila **Seguir jugando** (los 3 juegos jugados o capturados más recientemente, con su hero y su logo; `A` abre la ficha y `X` lo lanza), el resumen de los últimos 7 días (horas y capturas) y un dock con texto (Buscar, Favoritas, Añadir juego, Ajustes, Salir).
- Títulos de la plataforma en **cuadrícula paginada de 5×2** o en **carrusel**; se cambia con `Y` o en Ajustes (por defecto, cuadrícula). El hero (o la portada) del juego enfocado tiñe el fondo, la cabecera muestra capturas, horas, última partida y favoritas, y `LT`/`RT` cambian el filtro: *Todos*, *Con lanzador*, *Solo capturas*, *Con favoritas*, *A–Z*. La página se indica como «Página 1 de 4».
- Capturas del juego en cuadrícula de 4 columnas con desplazamiento virtual (solo se pintan las filas visibles), ordenables con `Y` (recientes, antiguas, favoritas primero, por nombre; el orden se recuerda por juego). Vista alternativa **agrupada por días** (botón `View` o Ajustes → Apariencia): 5 columnas, una cabecera por día (con lo jugado ese día si es de la última semana; los días con pocas capturas comparten fila), una línea de tiempo por meses a la derecha y `LT`/`RT` para saltar de día. Solo se agrupa con orden por fecha.
- Visor con anterior/siguiente, tira de miniaturas, píldora de información, interfaz que se oculta sola a los 3 s y precarga de las vecinas.
- Favoritas (`X`) y vista «Favoritas» con todas las marcadas.
- **Estadísticas** (dock de Inicio), a partir de las partidas guardadas en `stats.json`:
  - **Clasificación**: juegos por tiempo jugado en una semana, mes, año o «Siempre» (`LB`/`RB`), con `LT`/`RT` para ir al periodo anterior o siguiente. Arriba, tiempo, partidas, media por partida y días jugados, comparados con el periodo anterior. Los 3 primeros salen en filas con su hero fundido hacia el blanco y su logo encima; el resto, en filas con la mini portada (si falta: hero, última captura o iniciales sobre el acento).
  - **Calendario** (`Y` alterna): mapa de calor de 26 semanas con el tiempo de cada día, racha actual, mejor racha, día favorito y horas por mes. El día enfocado muestra sus partidas (hora, duración y capturas hechas en ellas).
  - **Evolución de un juego**: desde los paneles de horas de la ficha (`A`). Barras de horas y puntos de partidas por días, semanas, meses o años (`LB`/`RB`), con el detalle del periodo enfocado.
- **Color de acento por juego**: por defecto («Del juego (automático)») se extrae de su hero y su portada; se puede cambiar a mano por juego, o usar el arcoíris o un color fijo (ver [Acento por juego](#acento-por-juego)).
- Todo el escenario se diseña a **1280×720** y se escala para caber en la ventana (con bandas negras si la proporción no es 16:9), así que se ve igual en cualquier TV o monitor.
- Funciona sin conexión: la fuente M PLUS Rounded 1c va incluida en `src/assets/fonts`.
- **Lanzador**: los juegos se añaden a mano (un `.exe`, un emulador con su ROM, o una carpeta con muchas subcarpetas de juegos); no hay integración con Steam ni con otras tiendas. Al pulsar `A` sobre un título se abre su **ficha de juego** (botón **Jugar**, horas jugadas, semana, capturas recientes). Ver [Ficha de juego](#ficha-de-juego) y [Lanzador](#lanzador).
- **Capturas propias mientras juegas**: atajo global de teclado (por defecto `F12`) y combinación de mando (por defecto `View + RB`) que guardan una captura PNG del juego en la carpeta correcta de tu biblioteca, con sonido y notificación. Ver [Capturas propias](#capturas-propias-mientras-juegas).
- Los juegos aparecen en la lista de títulos aunque aún no tengan capturas, y los **jugados más recientemente salen primero**. Se guarda la última vez que jugaste y el tiempo total.
- **Añadir juegos sueltos** con un asistente (un `.exe`, un emulador con su ROM, una **carpeta con muchos juegos** —un asistente repasa cada subcarpeta, propone el ejecutable y el nombre (por defecto el de la carpeta)— o solo una carpeta de capturas), con teclado en pantalla para mando. Ver [Añadir juegos](#añadir-juegos).
- **Ocultar juegos** sin borrar nada y **añadir y organizar capturas** (importar copiando, arrastrar y soltar, ordenar, mover entre juegos). Ver [Ocultar juegos](#ocultar-juegos) y [Añadir y organizar capturas](#añadir-y-organizar-capturas).
- **Tus archivos originales no se tocan**: el visor solo lee tus imágenes. Importar capturas siempre *copia* (con un nombre nuevo y sin sobrescribir nada). Lo único que mueve archivos es la acción explícita «Mover a otro juego…», con confirmación y siempre dentro de tu carpeta raíz. Escribe además capturas nuevas (nunca sobrescribe), miniaturas en la caché y sus propios JSON.

## Estructura de carpetas esperada

```
Raíz/
├── PC/
│   ├── Elden Ring/
│   │   ├── hero.png             (opcional; banner 1920×620)
│   │   ├── logo.png             (opcional; PNG/WebP transparente)
│   │   ├── cover.jpg            (opcional; 600×900; si falta se usa la primera captura)
│   │   ├── 2024-03-09 21-05-44.png
│   │   └── ...
│   └── Hades II/
├── Switch/
│   └── Zelda/
└── Novelas visuales/
    └── Steins;Gate/
```

- Se leen `png`, `jpg`, `jpeg` y `webp` (también dentro de subcarpetas del juego).
- **Grafismos** (con los nombres y tamaños estándar de SteamGridDB), en la raíz de la carpeta del juego, sin distinguir mayúsculas; los que cumplen estos nombres **no cuentan como capturas**:

  | Grafismo | Nombres aceptados (por orden de preferencia) | Formatos | Tamaño habitual |
  |---|---|---|---|
  | Hero (banner de la ficha) | `hero`, `library_hero` | png, jpg, jpeg, webp | 1920×620 |
  | Logo (sobre el banner) | `logo`, `library_logo` | png, webp (con transparencia) | cualquiera |
  | Portada | `cover`, `library_600x900`, `grid` | jpg, jpeg, png, webp | 600×900 (2:3) |

  En juegos de **Steam** se usan además, si faltan en la carpeta, `library_hero.jpg` y `logo.png` de la caché local (`appcache\librarycache\<appid>_library_hero.jpg`, `<appid>_logo.png`, o dentro de `<appid>\` y de su subcarpeta hash). El arte de la carpeta gana siempre al de Steam. No se usa ningún icono. Los cambios se ven con *Recargar grafismos* (`Y` en la ficha) o al volver a escanear.
- Los nombres de las carpetas de plataforma son configurables (abajo). Se comparan sin distinguir mayúsculas.
- La fecha de cada captura se saca del nombre del archivo si tiene un patrón conocido (Steam `20240115123456_1.jpg`, Xbox Game Bar `2024-03-09 21-05-44`, NVIDIA `Juego 2025.11.02 - 08.15.30`, Switch `2023103118452300-HASH.jpg`, `AAAA-MM-DD`…); si no, la fecha de modificación. Orden: más reciente primero.
- Si la raíz está vacía o no existe no se crea nada en disco: se muestran igualmente las tres plataformas, vacías.

### Configuración (JSON)

Se guarda en la carpeta de configuración de la aplicación (en Windows, `%APPDATA%\app.visorcapturas.desktop\config.json`) junto con `favorites.json`, `games.json` (juegos y emuladores añadidos a mano) y `stats.json` y, desde esta versión, `hidden.json` (juegos ocultos), `titleprefs.json` (acento manual y orden de capturas por juego). `stats.json` guarda por juego: último inicio, segundos totales y la lista de **partidas** `{start, end}` en ms; el formato antiguo sin `sessions` se sigue leyendo y se amplía en cuanto juegas):

> **Base de datos (`visor.db`).** Los datos de usuario (favoritas, juegos añadidos a mano, estadísticas y partidas, ocultos, preferencias por juego, sagas, SteamGridDB, novelas japonesas, palabras conocidas y glosario de frases) viven en un único `%APPDATA%pp.visorcapturas.desktopisor.db` (SQLite). La primera vez que arranca, importa los JSON antiguos que se citan abajo (`favorites.json`, `games.json`, `stats.json`, `hidden.json`, `titleprefs.json`, `sagas.json`, `sgdb.json`, `novels.json`, `known-words.json` y `phrases/*.json`) sin borrarlos ni modificarlos: quedan como respaldo y la app ya no los lee. `config.json` sigue en JSON, igual que la media de `phrases/media/`.

> **Rutas portables y catálogo de capturas.** Las favoritas se guardan con la ruta *relativa a la carpeta raíz* (`PC/Elden Ring/captura.png`), así que siguen valiendo si la raíz cambia de sitio; las que estaban guardadas con ruta absoluta se convierten solas al arrancar (las de fuera de la raíz se quedan como estaban). Cada escaneo pone al día la tabla `shots` de `visor.db` (ruta relativa, título, tamaño, fechas, dimensiones) y un hilo en segundo plano calcula la huella SHA-256 de las capturas nuevas o modificadas. Es la base para sincronizar la biblioteca con la nube. Las capturas solo se leen.


```json
{
  "root": "D:\\Capturas",
  "titlesView": "grid",
  "shotsView": "grid",
  "accentMode": "art",
  "showHidden": false,
  "accentFixed": 4,
  "sound": false,
  "scanLaunchers": true,
  "captureHotkey": "F12",
  "captureCombo": "back+rb",
  "captureSound": true,
  "platforms": [
    { "id": "pc",     "name": "Juegos PC",        "folder": "PC",               "accent": 4, "icon": "pc" },
    { "id": "switch", "name": "Juegos Switch",    "folder": "Switch",           "accent": 0, "icon": "switch" },
    { "id": "vn",     "name": "Novelas visuales", "folder": "Novelas visuales", "accent": 6, "icon": "vn" }
  ]
}
```

`accentMode` puede ser `art` (acento extraído del arte de cada juego; por defecto), `rainbow` o `fixed` (usa `accentFixed`). `showHidden` muestra los juegos ocultos atenuados. `accent` es el índice del arcoíris (0 rojo, 1 naranja, 2 amarillo, 3 verde, 4 azul Wii U, 5 añil, 6 violeta). `icon` puede ser `pc`, `switch` o `vn`. Las miniaturas (JPEG de 480 px) están en la caché de la aplicación (`%LOCALAPPDATA%\app.visorcapturas.desktop\thumbs`), con nombre derivado de ruta + fecha + tamaño del original, así que se regeneran solas si cambias una imagen. Se pueden borrar sin problema. Los colores extraídos están en `palettes.json` en esa misma caché (clave ruta + fecha de modificación del hero/portada, así que se recalculan solos si cambias la imagen).

> Las favoritas, los juegos manuales, los ocultos, las preferencias por juego y las estadísticas se guardan en **JSON**, no en SQLite. Es suficiente para esta versión; la migración a SQLite queda para más adelante.

## Controles

| Acción | Mando (Xbox) | Teclado | Ratón |
|---|---|---|---|
| Mover el foco / anterior-siguiente en el visor | Cruceta o stick izq. | Flechas | Mover y pasar por encima |
| Abrir / aceptar (en un título: abre su ficha) | A | Intro o Espacio | Clic |
| Volver | B | Esc o Retroceso | Botones de la barra de pistas |
| Página anterior / siguiente (títulos), ±12 (capturas), ±10 (visor) | LB / RB | Q / E | Flechas laterales |
| Marcar favorita (en «Seguir jugando» de Inicio: jugar) | X | X | — |
| Pestaña anterior/siguiente en la lista de juegos (Todos · Mi lista · Colecciones) · día anterior/siguiente (capturas por días) · sección de Ajustes | LT / RT | Z / C | Botones LT/RT |
| Siguiente pestaña en la lista de juegos · cuadrícula de capturas continua o por días | View | V | — |
| Vista de títulos (cuadrícula/carrusel) · orden de capturas (recientes, antiguas, favoritas, nombre) · menú «Grafismos y opciones» en la ficha | Y | Y | — |
| Menú de capturas (mover, mostrar en el Explorador, seleccionar, salida de audio) en la cuadrícula de capturas y el visor | Start (Menú) | M | — |
| Menú de la lista de juegos: filtrar «Todos» (con lanzador, solo capturas, con favoritas), tamaño de las portadas, estado del juego, salida de audio y Ajustes | Start (Menú) | M | — |
| Salida de audio (desplegable de la barra superior): elegir · cambiar · cerrar | ↑↓ · A · B | Flechas · Intro · Esc | Clic en el icono del altavoz |
| Teclado en pantalla: escribir tecla · borrar · espacio · mayúsculas · confirmar | A · B · X · Y · Start | las letras se escriben tal cual · Retroceso · — · — · Tab (Intro = A, Esc = cancelar) | Clic en las teclas |
| Ajustes | Start (Menú; en la lista de juegos, Start › Ajustes) | Esc en Inicio | Icono de engranaje |
| Pantalla completa | — | F11 | — |
| Captura propia (también con el visor minimizado) | `View + RB` (configurable) | `F12` (configurable) | — |

Los ajustes están en secciones (Biblioteca, Apariencia, Capturas propias, Sistema): `LB`/`RB` cambian de sección, arriba/abajo pasan a la siguiente al llegar al borde, izquierda/derecha cambian el valor de la fila (marcado con ‹ ›) y abajo se explica la opción enfocada. Todos los campos de texto usan el teclado en pantalla (estilo Wii U, con ñ, acentos y ¿¡), así que todo se puede hacer solo con el mando; el teclado físico también escribe en él. El ratón solo roba el foco cuando se mueve de verdad (un puntero quieto no deshace lo que haces con el mando). La batería del mando de la barra superior es decorativa: la API de mandos del navegador no expone el nivel; la barra aparece apagada si no hay mando conectado.

## Ficha de juego

`A` sobre un título (cuadrícula o carrusel) abre su ficha, con el diseño «C · Wii U» aprobado:

- **Banner** (el hero) en un marco de cristal grande con el **logo** abajo a la izquierda, y la **portada** como baldosa de cristal propia a su derecha.
- Botón grande **Jugar** (`Con <emulador>` o `Ejecutable propio`). Al pulsarlo lanza el juego con el lanzador existente; mientras corre muestra **Jugando** con un cronómetro de sesión y el icono pulsando. Al terminar la partida (evento `game-exited`) vuelve a **Jugar** y las estadísticas se refrescan.
- Panel de **horas jugadas** con contador de dígitos rodantes (horas y minutos, y el número de sesiones) y panel con las **barras de la última semana** (7 días locales, minutos por día).
- **Capturas**: las 5 más recientes y **Ver todas** (cuadrícula). Una baldosa abre el visor en esa captura. `B` en el visor vuelve a la ficha, `B` en la cuadrícula vuelve a la ficha y `B` en la ficha vuelve a los títulos.
- `Y` abre el menú **Grafismos y opciones**: *Abrir carpeta*, *Recargar grafismos*, *Añadir capturas*, *Color de acento…*, *Ocultar juego* y *Quitar de la lista* (solo juegos añadidos a mano, pide confirmar).
- Sin lanzador (carpetas con capturas que no son un juego detectado): en lugar de Jugar aparece una píldora «Sin lanzador · Añádelo en Ajustes», las horas salen a 0 y no se dibuja la gráfica de la semana.
- Faltas de arte: sin hero se usa una ampliación borrosa y oscurecida de la portada (o de la primera captura); sin logo se escribe el nombre del juego en grande, en blanco; sin portada se usa la primera captura o un recuadro con el nombre.

## Lanzador

Cada juego tiene una **plataforma** y un **nombre de carpeta**. Su carpeta de capturas es `Raíz/<Plataforma>/<Carpeta>/`; por defecto la carpeta es el nombre del juego «saneado» para Windows (`Zelda: Tears` → `Zelda - Tears`, se quitan `<>"/\|?*` y `™ ® ©`). Si ya existe una carpeta equivalente (mismo nombre sin mayúsculas ni signos, por ejemplo `ELDEN RING` para `Elden Ring`) se reutiliza esa. La lista de títulos es la **unión** de las carpetas con capturas y de los juegos detectados. La portada es, por orden: `cover.*` de la carpeta, la portada de la caché local de Steam, y la primera captura; sin nada de eso se dibuja un recuadro con el nombre.

| Origen | Cómo se detecta | Cómo se lanza |
|---|---|---|
| **Juego manual** | Ajustes → *Añadir juego (.exe)*: ejecutable (diálogo de archivo), nombre, argumentos opcionales y plataforma. Se guarda en `games.json`. | El propio `.exe` |
| **Emulador** | Ajustes → *Añadir emulador*: ejecutable del emulador, ROM, nombre, plantilla de argumentos con `{rom}` (por defecto `"{rom}"`; si la plantilla no contiene `{rom}` la ruta se añade al final) y plataforma (por defecto `Switch`). Una entrada por juego. | `emulador.exe <argumentos>` |

Las entradas manuales se pueden quitar desde el menú `Y` de la ficha (**Quitar de la lista**, pide confirmar); eso solo borra la línea de `games.json`, nunca archivos.

**Seguimiento de la partida.** Tras lanzar un juego el visor se **minimiza** (no se cierra) y vigila cada 2 s los procesos con `sysinfo`: el PID del hijo en lanzamientos directos y, para emuladores, cualquier proceso cuyo `.exe` esté dentro de la carpeta de instalación del juego. Se da por terminada la partida tras ~6 s sin procesos; entonces se guarda la partida (`{start, end}` en `stats.json`) y se suma al total, el visor se restaura y avisa («Has jugado 1 h 20 min a …»). No se desactiva ni se inyecta nada en el juego.

**Nota sobre la pantalla de inicio.** Para que quepa la fila «Seguir jugando», los iconos de plataforma miden 190 px y el dock son píldoras con texto en la barra inferior. Además, los jugados más recientemente siguen yendo **primero en la lista de títulos** de cada plataforma.

## Añadir juegos

Desde el dock de Inicio («Añadir juego»), la baldosa **+** al final de la lista de títulos o Ajustes → *Añadir juego*. Un asistente elige el origen:

| Origen | Qué pide | Cómo se lanza |
|---|---|---|
| **Ejecutable (.exe)** | Archivo, nombre, plataforma, argumentos opcionales | El `.exe` |
| **Emulador + ROM** | Emulador, ROM, nombre, plantilla de argumentos, plataforma | `emulador.exe <argumentos>` |
| **Solo capturas** | Nombre y plataforma; crea `Raíz/<Plataforma>/<Nombre>/` | Sin lanzador |

Cada ruta permite elegir opcionalmente **hero**, **logo** y **portada**; se *copian* a la carpeta del juego como `hero.ext`, `logo.png` y `cover.ext` (si ya existe uno de ese tipo no se sobrescribe y se avisa). Los juegos añadidos se guardan en `games.json` (tipos `exe`, `emulator` y `captures`; las entradas de Steam de versiones anteriores se ignoran).

**Teclado en pantalla.** Todos los campos de texto lo abren: distribución en español con `ñ`, vocales acentuadas, `ü`, `¿` y `¡`; `A` escribe, `B` borra, `X` espacio, `Y` mayúsculas (de una pulsación) y `Start` confirma.

## Ocultar juegos

Menú `Y` de la ficha → **Ocultar juego**. Un juego oculto desaparece de las listas, de los contadores y de los recientes (sus archivos no se tocan). Ajustes → **Juegos ocultos** lista los ocultos para volver a mostrarlos, y **Mostrar ocultos** los enseña atenuados con una marca. La lista se guarda en `hidden.json` con un id estable (el del lanzador, o `plataforma/carpeta`).

## Añadir y organizar capturas

- **Añadir capturas**: en el menú `Y` de la ficha se elige «Añadir capturas»; se eligen varios archivos y se *copian* a la carpeta del juego con el nombre `<Carpeta>_AAAA-MM-DD_HH-MM-SS.ext`. La fecha sale, por este orden, de EXIF `DateTimeOriginal`, de la fecha del nombre del archivo, o de la fecha de modificación. Nunca se sobrescribe (si el nombre existe se añade un sufijo). Hay aviso de progreso y la biblioteca se refresca sola. También se puede **arrastrar y soltar** archivos sobre la ventana, en la ficha o la cuadrícula de un juego.
- **Orden**: `Y` recorre recientes, antiguas, favoritas primero y por nombre; se recuerda por juego.
- **Menú de capturas** (`Start`/`M`): *Mover a otro juego…* (elige el destino, pide confirmar con otra `A`; solo dentro de la raíz de la biblioteca, y las favoritas siguen a la captura), *Mostrar en el Explorador* y *Seleccionar* (modo de selección para mover varias a la vez).

## Colecciones

Pestaña «Colecciones» dentro de cada categoría (`LT`/`RT` o `View` pasan de Todos a Mi lista y a Colecciones). Agrupa varios juegos **de esa categoría** (p. ej. Trails, Kingdom Hearts) en una estantería y reúne todas sus capturas en una sola vista. Sustituyen a las antiguas sagas (que eran una categoría aparte): al arrancar con la biblioteca cargada, cada saga pasa a ser colección de la categoría donde tenga más juegos (si mezclaba varias, se divide en una por categoría). Es solo una agrupación virtual (nombre, categoría, color y ids de juego en la base de datos): **no crea, copia ni mueve archivos**; borrar una colección no borra nada.

- **Estantería**: pestaña con el título, «N juegos · N capturas», una línea de tiempo con un punto por juego (año de su primera captura o partida) y el botón «Ver todas las capturas» (`X`); debajo, las portadas por año con el año en el borde. `A` abre el juego; `X` abre las capturas de toda la colección (`LB/RB` filtran por juego); `Y` la edita.
- **Crear y editar**: «Nueva colección» al final (con sugerencias de grupos con nombre en común). «Editar colección»: nombre (teclado en pantalla), juegos (selector a pantalla completa con buscador, `A` marca, `X` marca todos los resultados, `Y` busca, `Start` guarda), color (automático, sacado de las portadas, o uno de los 7), mover arriba/abajo y borrar.
- **Añadir a una colección**: desde el menú de la ficha del juego y, al marcar un juego como Terminado, con `Y` en la barra de estado.
- **Estilo**: en Ajustes › Apariencia, «Estilo de las colecciones»: Aurora (pestaña del color de la colección con manchas difuminadas de las portadas) o Multicolor armonizada (degradado con los colores de las portadas, igualados en luz e intensidad).

## Acento por juego

Con el modo «Del juego (automático)» (por defecto), el acento de cada juego se extrae del hero y la portada (o de la primera captura): se agrupan los colores en el espacio OKLab, se prefieren los vivos, y se normaliza el resultado al mismo estilo que el arcoíris (luminosidad 0,62–0,72, croma ≥ 0,13). El color del texto sobre el acento (claro u oscuro) se elige por contraste. Si no hay arte útil se usa el arcoíris. En el menú `Y` de la ficha, **Color de acento…** permite elegir entre *Automático*, hasta 5 colores extraídos, los 7 del arcoíris o un tono libre con un deslizador, con vista previa en vivo (`B` cancela). Se guarda por juego en `titleprefs.json`; *Automático* lo borra. El color manual gana en cualquier modo. Los modos «Arcoíris» y «Fijo» siguen en Ajustes.

## Capturas propias mientras juegas

Con la aplicación abierta (aunque esté minimizada o detrás del juego) hay dos disparadores:

- **Atajo global de teclado**, por defecto `F12`; se elige en Ajustes entre `F12`, `F10`, `F9`, `Ctrl+F12`, `Alt+F12`, `Ctrl+Shift+S` o desactivado (`tauri-plugin-global-shortcut`). Si otro programa ya usa ese atajo se avisa al cambiarlo.
- **Combinación de mando** leída desde Rust con **XInput** (un hilo a ~60 Hz), porque la webview no recibe mando sin foco. Por defecto `View + RB`; también `View + LB`, `LB + RB`, `View + Menú`, pulsar ambos sticks, o desactivada. Funciona con mandos Xbox/XInput (los de otro tipo deben ir mediante Steam Input o un emulador XInput).

**Qué se captura.** La ventana del juego con la API del sistema **Windows.Graphics.Capture** (crate `windows-capture`): la ventana en primer plano, o, si estás en otra aplicación, la ventana del juego que lanzó el visor. Si la captura de ventana falla o sale **toda negra**, se captura **el monitor entero** como alternativa.

**Dónde se guarda.** `Raíz/<Plataforma>/<Carpeta>/<Carpeta>_AAAA-MM-DD_HH-MM-SS.png` (se crea la carpeta si falta; si ya hay un archivo con ese nombre se añade `_2`, `_3`…; nunca se sobrescribe). El juego es el lanzado desde el visor o, si no hay ninguno, el del proceso en primer plano cuando su `.exe` está dentro de la carpeta de un juego detectado. Si no se puede asignar a ningún juego va a **`Raíz/PC/Sin clasificar/`**. Sin carpeta raíz configurada se avisa y no se guarda nada. Después suena un sonido corto (se desactiva en Ajustes), aparece una notificación de Windows «Captura guardada: <Juego>» y, si el visor está a la vista, la biblioteca se actualiza sola.

**Límites conocidos**

- En **pantalla completa exclusiva** (algunos juegos DirectX antiguos) la captura de ventana puede dar negro: se detecta y se usa el monitor, pero con HDR o varios monitores puede no ser idéntico a lo que ves. Lo más fiable es jugar en *ventana sin bordes*.
- Un **juego iniciado fuera del visor** no tiene proceso rastreado: se captura la ventana en primer plano y se clasifica solo si está en la carpeta de un juego añadido; si no, en «Sin clasificar».
- Una **Switch real no se puede capturar desde el PC**: sus capturas se importan copiando la tarjeta SD o por USB a `Raíz/Switch/<Juego>/` (el visor las lee igual). Con emuladores sí funciona la captura propia.
- **No inyecta nada en el juego** (ni hooks ni overlays): usa únicamente la API de captura del sistema, por lo que es compatible con juegos con anticheat. Tampoco graba el cursor.
- Windows 10 1903 o posterior (requisito de Windows.Graphics.Capture).
- Escribir texto (nombre, argumentos) en los formularios de «Añadir» necesita teclado; con solo mando se usa el nombre sugerido a partir del archivo.

## Compilar en Windows

Requisitos:

1. [Rust](https://rustup.rs) (`rustup`, toolchain estable; en Windows pide las «Build Tools de Visual Studio» con la carga *Desarrollo para el escritorio con C++*).
2. [Node.js 20 o superior](https://nodejs.org).
3. WebView2: ya viene instalado en Windows 10 (actualizado) y 11.

```powershell
npm install
npm run tauri dev      # desarrollo con recarga en caliente (arranca a pantalla completa; F11 para salir)
npm run tauri build    # genera los instaladores
```

Los instaladores quedan en `src-tauri\target\release\bundle\`:

- `nsis\Visor de capturas_0.1.0_x64-setup.exe`
- `msi\Visor de capturas_0.1.0_x64_es-ES.msi`

### Compilar desde GitHub (sin instalar nada)

Sube el proyecto a un repositorio y ejecuta el flujo **Compilar para Windows** (pestaña *Actions* → *Run workflow*; también se lanza al empujar una etiqueta `v*`). Pasa las pruebas del núcleo y deja el instalador como artefacto `visor-capturas-windows`. Ver `.github/workflows/build-windows.yml`.

### Desarrollo en el navegador (sin Tauri)

```bash
npm install
npm run dev                       # o: npm run build && npx vite preview
```

Fuera de Tauri (no existe `window.__TAURI_INTERNALS__`) la interfaz usa una **biblioteca simulada** con las ilustraciones del mockup (33 juegos, ~1000 capturas, paletas de acento derivadas del arte, con juegos «lanzables», sin capturas y de varios orígenes). Parámetros útiles: `?empty=1` (sin carpeta elegida: pantalla de bienvenida), `?nolib=1` (carpeta sin capturas), `?view=carousel`. En el navegador *Jugar* se simula (aparece «Jugando» con su cronómetro; para terminar la partida se llama a `__visorEmit({type:'game-exited', id:'mock:pc:Elden Ring', name:'Elden Ring', seconds:5400})`, que además suma la sesión a las estadísticas simuladas). El arte de las fichas (hero, logo, portada) se genera con canvas, sin imágenes de juegos reales; hay títulos sin lanzador (`Hi-Fi Rush`, `Sekiro`), sin hero (`Hollow Knight`, `Hi-Fi Rush`, `Clannad`), sin logo (`Hades II`, `Hi-Fi Rush`) y sin capturas (`Dave the Diver`). Añadir, ocultar y quitar juegos, importar y mover capturas, orden y acento por juego funcionan en memoria (ocultos y preferencias en `localStorage`; hay tres juegos instalados simulados para el origen «Juego instalado»; `__visorDrop(['a.jpg','b.png'])` simula soltar archivos en la ventana). Además y desde la consola se puede simular un evento con `__visorEmit({type:'capture-saved', name:'Elden Ring'})`. Otras comprobaciones: `npm run check` (svelte-check/tsc) y, en `src-tauri`, `cargo test -p visor-core`.

## Arquitectura

```
src/                      Frontend Svelte 5 + TS
  lib/store.svelte.ts     estado global (runas), acentos, título actual
  lib/controller.svelte.ts  foco espacial (pos/move), navegación, teclado, mando, ajustes
  lib/backend.ts          capa Tauri (invoke, asset://, diálogo) con cola de miniaturas
  lib/mock.ts             biblioteca simulada para el navegador (juegos lanzables, sin capturas…)
  lib/osk.svelte.ts       teclado en pantalla (filas, mayúsculas, teclado físico)
  lib/ops.svelte.ts       asistente de alta, ocultos, importar/mover capturas, selector de acento
  lib/oklch.ts            réplica en TS de la matemática de color (deslizador de tono, simulación)
  screens/                Home, Titles (cuadrícula+carrusel), Detail (ficha de juego), Shots, Viewer, Settings,
                          ActionMenu, GameForm, AddChooser, AvailList, HiddenList, MoveTo, AccentPicker; components/Osk y Picker
  app.css                 CSS portado del mockup + añadidos
src-tauri/
  visor-core/             lógica pura SIN Tauri y con pruebas: configuración, escaneo (walkdir),
                          fechas por nombre, miniaturas (image, rayon), favoritas, analizador VDF (vdf.rs),
                          grafismos hero/logo/cover (art.rs), detección de juegos/vinculación/partidas y
                          agregado semanal (games.rs), nombres de archivo,
                          paleta/acento OKLab con caché (palette.rs), ocultos y preferencias (userdata.rs),
                          importar/mover capturas con EXIF y protección de la raíz (importer.rs),
                          PNG, detección de negro, combinaciones de mando y sonido (capture.rs)
  src/lib.rs              comandos Tauri (capa fina sobre visor-core), plugins y arranque
  src/launcher.rs         lanzar juegos y vigilar el proceso (sysinfo); minimiza/restaura el visor
  src/snap.rs             orquesta una captura: juego, PNG, sonido, notificación, evento
  src/platform.rs         Windows.Graphics.Capture y sonido (cfg(windows)); stubs en otros sistemas
  src/pad.rs              hilo XInput a ~60 Hz (cfg(windows)); vacío en otros sistemas
  tauri.conf.json         ventana sin bordes a pantalla completa, protocolo asset con ámbito
```

Comandos Rust: `get_config`, `save_config`, `scan_library`, `get_thumb`, `prefetch_thumbs` (evento `thumb-ready`), `get_favorites`, `set_favorite`, `launch_game`, `open_folder`, `add_game` (exe, emulador, captures), `scan_games_dir`, `remove_game`, `set_hidden`, `set_accent_override`, `set_sort_pref`, `import_shots`, `move_shots`, `reveal_in_explorer`, `capture_now`, `exit_app`. Eventos hacia la interfaz: `game-started`, `game-exited`, `capture-saved`, `capture-error`, `import-progress`. Las imágenes se sirven con el protocolo `asset:` de Tauri (`convertFileSrc`); el ámbito se amplía en tiempo de ejecución con la raíz que elijas.

## Hecho y pendiente

**Hecho (preliminar)**

- Visor: escaneo de la biblioteca, miniaturas en segundo plano con caché, inicio, títulos (cuadrícula y carrusel), capturas, visor, favoritas persistentes, orden de capturas, ajustes, configuración persistente, pantalla completa, mando/teclado/ratón, estados vacíos.
- Lanzador: juegos y emuladores manuales, asistente de carpeta de juegos, ficha de juego, tiempo de juego y último juego, portadas de la caché de Steam, vinculación juego ↔ carpeta de capturas.
- Añadir juegos sueltos (5 orígenes) con teclado en pantalla, ocultar juegos, importar capturas con fecha EXIF, arrastrar y soltar, orden por juego, mover entre juegos (uno o varios) y acento por juego (automático o manual).
- Capturas propias: atajo global, combinación XInput, Windows.Graphics.Capture con alternativa a monitor, guardado ordenado, sonido y notificación, actualización de la biblioteca.

**Pendiente**

| Elemento |
|---|
| Vigilancia del sistema de archivos (`notify`) para refrescar sin pulsar «Volver a escanear» |
| Búsqueda real (ahora el icono muestra «próximamente») |
| Importación automática desde las carpetas de Steam/Xbox Game Bar/NVIDIA (hoy se eligen los archivos a mano) |
| Fila «Jugado recientemente» en la pantalla de inicio (no cabe sin rehacer la maqueta) |
| Bandeja del sistema (ahora el visor se minimiza mientras juegas) |
| Otros lanzadores (GOG Galaxy, Xbox/Game Pass, EA, Ubisoft) |
| Descargar grafismos desde SteamGridDB; reemplazar grafismos ya existentes desde la interfaz (al añadir un juego se copian, pero no se sobrescriben) |
| Editar o cambiar de plataforma un juego manual desde la interfaz (hoy: quitarlo y añadirlo de nuevo) |
| Soporte JXR / HDR; SQLite en lugar de JSON |
| Fuente: solo se incluyen los subconjuntos latino y latino extendido; los kana/kanji caen a la fuente del sistema |

## Verificación realizada en esta versión

**Comprobado automáticamente (en Linux, sin Windows):**

- `npm run build` y `npm run check` limpios.
- `cd src-tauri && cargo test -p visor-core`: 82 pruebas (las nuevas de esta versión: extracción de paleta y normalización OKLCH, tinta por contraste, caché por ruta+fecha, ocultos y preferencias, id estable, orden de capturas, importación con fecha EXIF (JPEG sintético), nombres sin sobrescribir, protección de la raíz incluido un enlace simbólico, mover capturas, tipos de juego manual `steam`/`captures`/`pick`, ajuste `accentMode`; las anteriores: búsqueda de hero/logo/cover y nombres SteamGridDB, hero/logo en la caché de Steam en los tres diseños, prioridad carpeta > Steam, agregado semanal con cruce de medianoche y zonas horarias, migración de `stats.json`), incluidas el analizador VDF, `appmanifest`, manifiestos de Epic, detección de Steam con varias bibliotecas y portadas, vinculación juego ↔ carpeta (unión de listas, reutilización de carpeta, orden por último jugado), argumentos de emulador, nombre de archivo de captura, guardado PNG sin sobrescribir, detección de fotograma negro y disparo de la combinación de mando.
- `cargo check` de la aplicación Tauri completa, en Linux y con `--target x86_64-pc-windows-msvc` (esto último compila de verdad el código de Windows: captura, XInput, sonido, registro).
- Recorrido con Playwright sobre la biblioteca simulada: inicio → títulos → ficha de juego → *Jugar* (simulado, cronómetro) → fin de partida simulado → baldosa de captura → visor → atrás → *Ver todas* → atrás, fichas sin lanzador / sin hero / sin logo / sin capturas, menú `Y` (recargar, quitar de la lista), ajustes con las filas nuevas, añadir juego y emulador, quitar de la lista, aviso de captura guardada; sin errores de consola.
- Recorrido de las funciones nuevas con Playwright, solo con teclado, sobre la biblioteca simulada (39 comprobaciones, 0 errores de consola): añadir `.exe` escribiendo «Piñata Ñu» con el teclado en pantalla, baldosa +, emulador, juego instalado, Steam por URL con teclado físico, solo capturas, ocultar/mostrar (también atenuados y desde Ajustes), añadir capturas, arrastrar y soltar, los 4 órdenes recordados por juego, mover una y varias capturas con confirmación, acento (paleta, tono libre, automático, cancelar con `B`) y modos de acento de Ajustes.

**NO comprobado (no hay Windows ni mando en el entorno de desarrollo):** los diálogos de archivo de Tauri y `onDragDropEvent` al soltar archivos reales; la lectura EXIF de fotos reales (solo hay prueba con un JPEG sintético); `explorer /select`; servir con el protocolo `asset:` el arte recién copiado; la detección real de Steam/Epic con los juegos añadidos como «instalado»; servir hero y logo reales con el protocolo `asset:` (incluido el parámetro `?v=` que se añade al recargar grafismos y la autorización en tiempo de ejecución de la carpeta `appcache\librarycache` de Steam); ejecutar la aplicación empaquetada; la detección real de Steam/Epic en un PC con ellos instalados; lanzar juegos y la vigilancia de procesos; el atajo global; la lectura XInput con un mando real; Windows.Graphics.Capture sobre un juego real (ventana, negro en pantalla completa exclusiva, alternativa a monitor); el sonido y las notificaciones de Windows. Todo eso compila y está escrito siguiendo las APIs, pero **hay que probarlo en Windows** antes de fiarse.

## Licencias

La fuente M PLUS Rounded 1c se distribuye bajo SIL Open Font License 1.1.
