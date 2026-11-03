# Plan: rediseño del menú de opciones del juego («Grafismos y opciones»)

Mockups (1280×720, tema v5): `mockups/menu-juego-a.html`, `-b.html`, `-c.html` y `mockups/menu-juego-comparativa.html` (hoy, A, B y C lado a lado; casos .exe / emulador / tablet). Cada página tiene botones para todos sus estados y acepta `#estado` en la URL.

## 1. El problema, en corto

Análisis completo en `docs/informe-menu-juego.md`. Resumen:

- Hasta 17 filas en dos columnas, todas con el mismo peso: jugar, configurar una vez (ejecutable, Magpie, fecha) y peligroso (quitar).
- El título («Grafismos y opciones») y las secciones («Juego y Lanzamiento», «Biblioteca y Estado») no describen lo que contienen.
- Las filas hacen cosas distintas sin avisar (inmediata, abre panel, cicla en el sitio como Magpie, doble pulsación) y los «…» no son coherentes.
- **Duplicidad comprobada con la ficha** (`Detail.svelte:157-212`): la ficha ya tiene el botón **Jugar** (y X), la píldora **Estado** (`dst`) y la píldora **Fecha de lanzamiento** (`drel`), las tres con mando y ratón. Las **colecciones no salen en la ficha**. El menú no tiene botón visible: solo Start o clic derecho (`controller.svelte.ts:1801`, `:1888`).

## 2. Inventario (todas se conservan)

| id | Hoy | Nombre nuevo | Se muestra si |
|---|---|---|---|
| play | Iniciar juego | — (ya en la ficha: botón Jugar y X) | launchable y nada en marcha |
| status | Estado: X | — (ya en la ficha: píldora) | id y no agrupación |
| release | Fecha de lanzamiento… | — (ya en la ficha: píldora) | id y no agrupación |
| addcoll | Añadir a una colección… | **Colecciones…** · valor «En 2» / «Ninguna» | id y no agrupación |
| saves | Partidas guardadas… | Partidas guardadas… | gameId y no agrupación |
| phrases | Frases y repaso… | Frases y repaso… | gameId y categoría novela (jvn/vn) |
| sgdb | Grafismos (SteamGridDB o del PC)… | **Imágenes del juego…** · «Portada, fondo, logo e icono» | siempre |
| accent | Color de acento | **Color del juego…** · muestra del color | siempre |
| folder | Abrir carpeta | **Abrir carpeta de capturas** | hasFolder, no tablet |
| addshots | Añadir capturas | **Añadir capturas…** | root y dir, no tablet |
| magpie | Magpie: X (cicla con A) | **Escalado con Magpie…** · valor = modo; abre lista de 4 modos | .exe manual con gameId |
| setexe | Cambiar ejecutable… | **Ejecutable…** | .exe manual, no tablet |
| setdir | Cambiar carpeta de trabajo… | **Carpeta de trabajo…** | .exe manual, no tablet |
| reload | Recargar grafismos | **Volver a cargar las imágenes** | siempre |
| category | Cambiar de categoría… | **Mover a otra categoría…** · valor = categoría actual | gameId, removable, >1 categoría |
| hide / unhide | Ocultar juego / Mostrar juego | **Ocultar de la biblioteca** / **Volver a mostrar en la biblioteca** | siempre |
| remove | Quitar de la lista | **Quitar de la biblioteca** · «Pide confirmación · las capturas no se borran» | removable (solo añadidos a mano) |

Las categorías no cambian: «Mover a otra categoría…» sigue listando `app.config.platforms` igual que hoy; solo cambia la etiqueta y el sitio.

**Reglas comunes a las tres propuestas** (resuelven los problemas 4-6 del informe):
- «…» = abre un panel o diálogo. Sin puntos = se hace al momento. «›» = entra en un submenú.
- La etiqueta es siempre la acción; el valor actual va a la derecha, en gris (`.srow .sv`, ya existe en `app.css:240`).
- Rojo = pide una segunda pulsación, y va al final, separado.
- Magpie deja de ciclar con A: abre una lista de modos con ✓ (como «Filtrar»).

## 3. Las tres propuestas

### A · Menú corto con lo de siempre + «Más opciones» (`menu-juego-a.html`)
- **Idea:** el menú de Start enseña en una columna lo que se usa a menudo (8 filas como mucho) y lo de configurar una vez o lo peligroso va detrás de «Más opciones ›». Jugar, Estado y Fecha salen del menú (ya están en la ficha).
- **Nivel 1 «Opciones del juego»** (tres bloques sin título separados por una línea + cierre): Colecciones… · Partidas guardadas… · Frases y repaso… │ Imágenes del juego… · Color del juego… │ Abrir carpeta de capturas · Añadir capturas… │ Más opciones ›
- **Nivel 2 «Más opciones»:** AL JUGAR (Escalado con Magpie…, Ejecutable…, Carpeta de trabajo…) · SI ALGO SE VE MAL (Volver a cargar las imágenes) · BIBLIOTECA (Mover a otra categoría…, Ocultar de la biblioteca, Quitar de la biblioteca).
- **Nivel 3:** solo las listas de Magpie (4 modos) y de categorías (ya existe).
- **Mando:** ↑/↓ y A, como hoy. B en nivel 2/3 vuelve al nivel anterior con el foco en la fila de origen; B en nivel 1 y Start cierran. Ratón: la ruta «Opciones › Más opciones» de la cabecera es pulsable para volver.
- **.exe con todo (17):** 8 filas + 7 en «Más». **Emulador:** 7 + 2 (Volver a cargar, Ocultar). **Tablet:** 6 + 5. **Solo capturas (sin lanzador):** Imágenes, Color, Carpeta, Añadir + «Más» con 2.
- **Pros:** una columna y filas de 54-58 px siempre (adiós a `tight/squeeze` y a las 2 columnas); orden estable; lo peligroso nunca está a un A despistado; coste bajo; reutiliza el mecanismo de submenú que ya usa «Cambiar de categoría».
- **Contras:** Magpie/ejecutable quedan a dos pulsaciones; quien use mucho Magpie lo notará.
- **Coste:** bajo. `store.svelte.ts`, `controller.svelte.ts` (`menuItemsFor`, `MENU_GROUPS`, `menuLayout`, `menuAct`, `menuCmd`, `cycleMagpie`), `ActionMenu.svelte`, `README.md`.

### B · Accesos rápidos arriba + lista por temas debajo (`menu-juego-b.html`)
- **Idea:** una sola pantalla, sin submenús. Arriba 4-5 fichas grandes de posición fija (Colecciones, Imágenes, Color, Guardados, Frases). Debajo, dos columnas compactas: AL JUGAR + CAPTURAS | BIBLIOTECA + SI ALGO SE VE MAL.
- **Mando:** ←/→ entre fichas; ↓ baja a la columna de debajo; ↑ desde la primera fila vuelve a las fichas; ←/→ en la lista cambia de columna. Magpie abre una lista flotante (←/→ quedan para cambiar de columna).
- **.exe con todo:** 5 fichas + 9 filas, todo a la vista. **Emulador:** 4 fichas + 4 filas (panel más estrecho). **Tablet:** 5 fichas + 1 fila a la izquierda y 4 a la derecha (descompensado).
- **Pros:** lo cotidiano se reconoce de un vistazo; nada escondido; muy «consola».
- **Contras:** sigue habiendo 14 cosas a la vista; navegación 2D nueva dentro del menú (más código y más casos raros); el panel cambia mucho de forma entre juegos y en tablet.
- **Coste:** medio. Lo de A + una rejilla de fichas y navegación por zonas en `menuCmd`, y CSS nuevo para las fichas en `ActionMenu.svelte`/`app.css`.

### C · Lo de cada día en la ficha + «Ajustes del juego» con pestañas (`menu-juego-c.html`)
- **Idea:** separar «usar» de «configurar». La ficha gana una píldora **Colecciones** (junto a Estado y Fecha) y un botón **Ajustes** visible. Start abre un panel grande «Ajustes del juego», con el patrón de la pantalla de Ajustes (`.scats`): 4 pestañas fijas — Aspecto · Al jugar · Capturas y datos · Biblioteca — con todas las funciones (Estado y Fecha también, por completitud).
- **Mando:** ↑/↓ en las pestañas (la derecha se actualiza al momento), → o A entra, ← o B vuelve a las pestañas, B en las pestañas cierra. Las pestañas vacías (Al jugar en un emulador) se ven apagadas y se saltan: no se mueven de sitio.
- **.exe con todo:** 3 + 3 + 4 + 6 filas. **Emulador:** Al jugar vacía. **Tablet:** Al jugar con solo Magpie; Capturas y datos con 2.
- **Pros:** categorías limpias y estables; la ficha se vuelve el sitio de lo cotidiano; se puede descubrir con ratón.
- **Contras:** el más caro; un componente nuevo; más cosas en la ficha (un elemento más que enfocar); pestañas de 1 fila en tablet.
- **Coste:** alto. `Detail.svelte` (píldora y botón, `validKey` en `controller.svelte.ts:110-119`), componente nuevo `GameSettings.svelte` con su estado y su navegación, overlay nuevo en `store.svelte.ts:77`, enrutado de mando.

## 4. Recomendación: A

A arregla lo que se queja el usuario (demasiadas opciones, categorías difusas, poco predecible) con el menor cambio: el menú baja de 17 a 8 filas como mucho y pasa a una sola columna con orden fijo; las secciones tienen nombres que dicen lo que hay; las reglas «… / › / valor a la derecha / rojo» hacen predecible cada fila. Se apoya en lo que ya existe (la ficha ya tiene Jugar, Estado y Fecha; el submenú de categorías ya reemplaza los `items` en el sitio), no toca los menús de capturas ni de la lista y no mete navegación nueva. B mantiene demasiado a la vista y obliga a una navegación 2D; C es la más ordenada, pero cuesta un componente nuevo y cambia la ficha. Las dos mejoras de C que salen baratas (píldora Colecciones y botón visible) quedan como pasos opcionales al final, para hacerlos si el usuario quiere.

## 5. Pasos de implementación (A) — para Sonnet

Nada de esto es masivo: no delegar en `agy`.

1. **`src/lib/store.svelte.ts:72-76` (`MenuItem`)**: añadir `'more'` y `` `mag:${string}` ``.
   **`:104-111` (`MenuState`)**: añadir `sub?: '' | 'more' | 'magpie' | 'category'` (opcional, para no tocar los `app.menu = {…}` de capturas y lista).
2. **`src/lib/controller.svelte.ts:345-367`**: cambiar el comentario y sustituir `menuItemsFor` por dos funciones (mismas condiciones que hoy, ver la tabla del §2):
   - `menuMainFor(t)`: `addcoll`, `saves`, `phrases`, `sgdb`, `accent`, `folder`, `addshots`, `more` (en este orden; `more` siempre, porque `reload` y `hide` siempre existen). **Sin** `play`, `status`, `release`.
   - `menuMoreFor(t)`: `magpie`, `setexe`, `setdir`, `reload`, `category`, `hide|unhide`, `remove`.
   - Ojo, `openTitlesMenu` (`:1018`) no se toca: sigue con su lista propia.
3. **`:370-374` (`MENU_GROUPS`)** → dos tablas:
   - `MAIN_GROUPS = [['', ['addcoll','saves','phrases']], ['', ['sgdb','accent']], ['', ['folder','addshots']], ['', ['more']]]` (grupos sin título = separador).
   - `MORE_GROUPS = [['Al jugar', ['magpie','setexe','setdir']], ['Si algo se ve mal', ['reload']], ['Biblioteca', ['category','hide','unhide','remove']]]`.
   **`menuLayout` (`:378-390`)**: si `kind === 'detail'` y `sub` es `''` o `'more'`, devolver **una sola columna** `[groups]` con la tabla que toque (quitar el reparto en 2 columnas). Para `'magpie'`/`'category'` y los otros menús, lista plana como hoy.
4. **`openDetailMenu` (`:407-413`)**: `items: menuMainFor(t)`, `sub: ''`.
5. **`menuAct` (`:448-519`)**:
   - `'more'`: `app.menu.items = menuMoreFor(t); app.menu.sub = 'more'; row = 0; confirm = false; blip(1320)`.
   - `'magpie'`: en vez de `cycleMagpie`, abrir la lista: `items = MAGPIE_ORDER.map(m => `mag:${m}`)`, `sub = 'magpie'`, `row` = índice del modo actual.
   - `'mag:…'`: guardar el modo (reutilizar el cuerpo de `cycleMagpie`, `:544-553`, convertido en `setMagpie(t, mode)`: `backend.setMagpieMode`, `t.magpieMode`, `app.magpieTick++`, el mismo toast la primera vez) y volver a `'more'` con el foco en `magpie`. Borrar `cycleMagpie` si nadie más lo usa (grep: hoy solo `:465`).
   - `'category'`: lo de hoy + `app.menu.sub = 'category'`.
   - `:513`: toast «Imágenes recargadas».
6. **`menuCmd` (`:555-570`)**: nueva `menuBack()` exportada. Si `kind === 'detail'` y `sub` es `'magpie'` o `'category'` → `items = menuMoreFor(t)`, `sub = 'more'`, foco en la fila de origen (`magpie`/`category`); si `sub === 'more'` → `items = menuMainFor(t)`, `sub = ''`, foco en `more`; si no, cerrar como hoy. En `:569`: `'b'` llama a `menuBack()` (con `blip(520)`); `'menu'` (Start) sigue cerrando del todo.
7. **`src/screens/ActionMenu.svelte`**:
   - **Etiquetas (`:43-80`)**: las de la tabla del §2. Se aplican también al menú de la lista (`openTitlesMenu` comparte ids: `sgdb`, `addcoll`, `accent`, `hide`, `remove`, `folder`, `status`) — es deseable por coherencia; `status` se queda como está. Nuevas: `'more'` → «Más opciones»; `` `mag:${m}` `` → `MAGPIE_LABEL[m]`.
   - **Línea pequeña (`<small>`, como Magpie hoy en `:116`)**: `sgdb` «Portada, fondo, logo e icono»; `magpie` = `magpieSummary` si lo hay; `remove` «Pide confirmación · las capturas no se borran» (salvo cuando `confirm`); `more` = nombres de las secciones de `MORE_GROUPS` que tengan filas, unidos por « · ».
   - **Valor a la derecha** (`<span class="sv">`): `addcoll` → «En N» / «Ninguna» (N = `platformCollections().filter(c => c.members.includes(t.id)).length`, de `coleccion.svelte.ts:459`; solo cuenta colecciones de la categoría actual); `accent` → un círculo de 18 px con `var(--acc)`; `magpie` → `MAGPIE_LABEL[modo]`; `category` → `app.platform?.name`; `more` → `ICON.chevR`.
   - **Iconos (`:87-90`)**: `more` → `ICON.chevR` (o tres puntos si se añade a `icons.ts`); `mag:` → `ICON.check` en el modo actual, vacío en el resto (como `flt:`); `magpie` → `ICON.image` sigue valiendo (o `ICON.monitor`); `setexe` → `ICON.monitor`; `remove` → `ICON.trash`.
   - **Título (`:105`)** en `detail`: `''` → «Opciones del juego», `'more'` → «Más opciones», `'magpie'` → «Escalado con Magpie», `'category'` → «Mover a otra categoría». Encima del `h2`, en los subniveles, una ruta pequeña pulsable («Opciones» / «Opciones › Más opciones») que llama a `menuBack()`.
   - **Cabeceras y separadores (`:110-111`)**: grupo con título → `<h3 class="mhead">` como hoy; grupo sin título que no sea el primero → `<div class="msep">` (1 px, `var(--line)`, margen 5px 12px).
   - **Alturas (`:27-41`)**: añadir `SEP = 11` y sumar `seps * SEP` en `H` y en el cálculo de `pitch` del caso agrupado. Comprobar que con 8 filas + 3 separadores el paso queda en 54-58 px; las clases `tight/tighter/squeeze` siguen sirviendo a los otros menús.
   - **Hints (`:126`)**: B = «Volver» si `kind === 'detail' && app.menu.sub`, si no «Cerrar».
   - Quitar `class:two` / `.menupanel.two` solo si ya no lo usa nadie (en `detail` ya no habrá 2 columnas).
8. **`README.md:107` y `:126`**: cambiar «Grafismos y opciones» por «Opciones del juego» y la lista de acciones por la nueva.
9. Commit y push (CLAUDE.md). Ojo con los finales de línea mezclados LF/CRLF: comprobar los CR antes del commit.

**Opcionales (mejoras de C, baratas; solo si el usuario las aprueba):**
- O1. Píldora **Colecciones** en `.dchips` (`Detail.svelte:157-201`): `data-k="dcol"`, `data-act="addcoll"`, texto «En N colecciones» / «Añadir a colección»; `validKey` (`controller.svelte.ts:110-119`): `dcol` si `t.id && !t.members`; `runAct` (`:583+`): `'addcoll'` → `openAddColl(app.t)`; hint A «Colecciones». La fila «Colecciones…» del menú se quedaría igualmente.
- O2. Botón visible **Opciones** en la ficha (`data-act="options"`, que ya existe en `runAct` `:642`), para que el ratón lo descubra.

## 6. Comprobación

- `npm run check` (svelte-check) sin errores nuevos; no hace falta compilar Tauri.
- En el navegador (datos de prueba, `src/lib/mock.ts`), ficha de un juego .exe añadido a mano en una categoría de novelas: Start → 8 filas en una columna, sin Jugar/Estado/Fecha; «Más opciones» → 3 secciones; B vuelve con el foco en «Más opciones»; B otra vez cierra; Start cierra desde cualquier nivel.
- Magpie: A abre los 4 modos con ✓ en el actual; elegir otro lo guarda, vuelve a «Más opciones» y el valor de la derecha cambia.
- Mover a otra categoría: lista igual que hoy; B vuelve a «Más opciones»; elegir una mueve el juego como hasta ahora.
- Quitar: primera A → «Pulsa A otra vez para quitar»; mover el foco lo cancela; segunda A quita.
- Juego de emulador: 7 filas + «Más» con 2. Título solo de capturas: sin Colecciones, Guardados ni Frases.
- Tablet (`IS_TABLET`): sin Abrir carpeta, Añadir capturas, Ejecutable ni Carpeta de trabajo.
- Menús de capturas y de la lista (Start en la lista de juegos): misma forma que antes, solo con las etiquetas nuevas.
- Ratón: pasar por encima mueve el foco; clic en la ruta vuelve; clic fuera cierra.

## 7. Riesgos

- **Hábito:** quien usaba Start → «Iniciar juego» o «Estado» en el menú tendrá que usar la ficha (X o el botón Jugar; la píldora de Estado). La pista de la ficha ya dice «X Jugar».
- **Agrupaciones (`t.members`):** la píldora de Estado se ve pero avisa «Este elemento no tiene estado» (`milista.svelte.ts:507`); ya pasa hoy y el menú tampoco lo ofrecía. Sin cambio.
- Al renombrar etiquetas compartidas, el menú de la lista también cambia de texto (intencionado).

## 8. Decisiones abiertas (para el usuario)

1. **¿A, B o C?** (recomendado A).
2. **¿Quitar del menú Jugar, Estado y Fecha** porque ya están en la ficha, o dejarlos también en el nivel 1? (recomendado quitarlos; si se quedan, el nivel 1 pasa a 10-11 filas).
3. **Nombres:** «Imágenes del juego» (o «Portada y fondos»), «Color del juego» (o «Color de acento»), «Más opciones» (o «Avanzado»), secciones «Al jugar / Si algo se ve mal / Biblioteca».
4. **Abrir carpeta y Añadir capturas:** ¿en el nivel 1 (propuesto) o en «Más opciones»? Si van a «Más», el nivel 1 queda en 6.
5. **«Volver a cargar las imágenes»:** ¿se queda en «Más opciones» o se mete en el selector de imágenes (`sgdb.svelte.ts`) como un botón? Lo segundo fusionaría dos funciones y quitaría una fila, pero toca otro panel.
6. **Opcionales O1 (píldora Colecciones en la ficha) y O2 (botón Opciones visible).**
7. **Comprobar con el usuario** qué borra «Quitar de la biblioteca»: el backend (`lib.rs:739`, `manual_remove`) borra el juego añadido a mano de la base; la subetiqueta propuesta dice «las capturas no se borran». Falta confirmar si se pierden también las horas jugadas o el estado: si es así, hay que decirlo en la subetiqueta.
8. El valor de «Ejecutable…» (nombre del .exe a la derecha, como en el mockup C) necesitaría que el `Title` del frontend trajera la ruta del ejecutable, y hoy no la trae (`types.ts`): se deja fuera salvo que el usuario lo quiera.
