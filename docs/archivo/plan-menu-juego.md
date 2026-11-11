# Plan: rediseño del menú de opciones del juego («Grafismos y opciones»)

> **Decisión del usuario: propuesta C** (ficha + «Ajustes del juego» con 4 pestañas), con la píldora Colecciones y el botón Ajustes visible. El resto de decisiones abiertas se resuelve con las recomendaciones del §8. Los pasos a ejecutar están en el **§9 Implementación de C**.

Mockups (1280×720, tema v5): `mockups/menu-juego-a.html`, `-b.html`, `-c.html` y `mockups/menu-juego-comparativa.html` (hoy, A, B y C lado a lado; casos .exe / emulador / tablet). Cada página tiene botones para todos sus estados y acepta `#estado` en la URL. **La referencia para implementar es `menu-juego-c.html`.**

## 1. El problema, en corto

Análisis completo en `docs/informe-menu-juego.md`. Resumen:

- Hasta 17 filas en dos columnas, todas con el mismo peso: jugar, configurar una vez (ejecutable, Magpie, fecha) y peligroso (quitar).
- El título («Grafismos y opciones») y las secciones («Juego y Lanzamiento», «Biblioteca y Estado») no describen lo que contienen.
- Las filas hacen cosas distintas sin avisar (inmediata, abre panel, cicla en el sitio como Magpie, doble pulsación) y los «…» no son coherentes.
- **Duplicidad comprobada con la ficha** (`Detail.svelte:157-212`): la ficha ya tiene el botón **Jugar** (y X), la píldora **Estado** (`dst`) y la píldora **Fecha de lanzamiento** (`drel`), las tres con mando y ratón. Las **colecciones no salen en la ficha**. El menú no tiene botón visible: solo Start, Y o clic derecho (`controller.svelte.ts:1790`, `:1801`, `:1888`).

## 2. Inventario (todas se conservan)

| id | Hoy | Nombre nuevo | Se muestra si |
|---|---|---|---|
| play | Iniciar juego | — (ya en la ficha: botón Jugar y X) | launchable y nada en marcha |
| status | Estado: X | **Estado…** · valor = estado | id y no agrupación |
| release | Fecha de lanzamiento… | **Fecha de lanzamiento…** · valor = fecha | id y no agrupación |
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

**Reglas comunes** (resuelven los problemas 4-6 del informe):
- «…» = abre un panel o diálogo. Sin puntos = se hace al momento.
- La etiqueta es siempre la acción; el valor actual va a la derecha, en gris.
- Rojo = pide una segunda pulsación, y va al final, separado.
- Magpie deja de ciclar con A: abre una lista de modos con ✓.

## 3. Las tres propuestas

### A · Menú corto con lo de siempre + «Más opciones» (`menu-juego-a.html`)
Nivel 1 de 8 filas como mucho (Colecciones, Guardados, Frases │ Imágenes, Color │ Carpeta, Añadir capturas │ Más opciones ›); nivel 2 con Al jugar / Si algo se ve mal / Biblioteca. Coste bajo. *Descartada por el usuario.*

### B · Accesos rápidos arriba + lista por temas debajo (`menu-juego-b.html`)
5 fichas grandes (Colecciones, Imágenes, Color, Guardados, Frases) + dos columnas compactas. Navegación 2D dentro del menú; coste medio. *Descartada.*

### C · Lo de cada día en la ficha + «Ajustes del juego» con pestañas (`menu-juego-c.html`) — **elegida**
- **Ficha:** Jugar, Estado y Fecha (ya están) + píldora nueva **Colecciones** + botón **Ajustes** visible.
- **Start / Y / clic derecho / botón Ajustes** abren el panel «Ajustes del juego»: pestañas a la izquierda (patrón `.scats` de Ajustes), filas a la derecha con valor actual.
- **Pestañas fijas, siempre las 4 y en este orden:**
  - **Aspecto:** Imágenes del juego…, Color del juego…, Volver a cargar las imágenes.
  - **Al jugar:** Escalado con Magpie…, Ejecutable…, Carpeta de trabajo….
  - **Capturas y datos:** Abrir carpeta de capturas, Añadir capturas…, Partidas guardadas…, Frases y repaso….
  - **Biblioteca:** Colecciones…, Estado…, Fecha de lanzamiento…, (hueco) Mover a otra categoría…, Ocultar de la biblioteca, Quitar de la biblioteca.
- Jugar no entra en el panel (está en la ficha).

## 4. Recomendación original y decisión

El planificador recomendó A por coste; el usuario eligió C por tener categorías limpias y estables y por llevar lo cotidiano a la ficha. Se implementa C.

## 5-7. (Pasos, comprobación y riesgos de A: sustituidos por el §9.)

## 8. Decisiones (resueltas con la recomendación)

1. Propuesta: **C**.
2. Jugar no va en el panel; Estado y Fecha sí (además de en la ficha).
3. Nombres: «Ajustes del juego», «Imágenes del juego», «Color del juego», pestañas «Aspecto / Al jugar / Capturas y datos / Biblioteca».
4. Abrir carpeta y Añadir capturas: pestaña «Capturas y datos».
5. «Volver a cargar las imágenes»: se queda como fila en Aspecto (no se fusiona con el selector).
6. Píldora Colecciones y botón Ajustes: **sí**.
7. **Sigue abierto (preguntar al usuario):** qué borra «Quitar de la biblioteca» además de la entrada en la base (`lib.rs:739`, `manual_remove`): ¿horas jugadas, estado? Mientras tanto, la subetiqueta dice solo «Pide confirmación · las capturas no se borran».
8. Sin nombre del .exe a la derecha de «Ejecutable…»: el `Title` del frontend no trae la ruta (`types.ts`). En el mockup sale, pero no se implementa.

## 9. Implementación de C — para Sonnet

**Todo lo hace Sonnet; nada es masivo ni repetitivo, así que no se delega en `agy`.** Orden recomendado: pasos 1→8, un commit al final (o uno tras el paso 6 y otro tras el 8).

### Idea técnica
- Overlay nuevo `'gameset'` con su propio estado `app.gameSet` y su componente `src/screens/GameSettings.svelte`.
- Cada fila ejecuta la acción existente **a través de `menuAct`**, que ya sabe hacerlo todo. Para eso, antes de llamarla, se rellena `app.menu` con `{ kind: 'detail', title: app.t, … }` (`menuAct` lee `app.menu.title`, `app.menu.kind` y `app.menu.confirm`).
- Excepciones que no pasan por `menuAct`: `magpie` y `category`, que se convierten en listas dentro del propio panel, y `remove`, que se confirma dentro del panel.
- Los `closeOverlay()` que hace `menuAct` cierran el overlay de arriba (ahora `'gameset'`), igual que hoy cerraban `'menu'`. Los paneles que se apilan sin cerrar (sgdb, accent, saves, status, release) vuelven al panel al cerrarse, igual que hoy volvían al menú.

### Paso 1 — `src/lib/store.svelte.ts`
- `:77` (`Overlay`): añadir `'gameset'`.
- Junto a `MenuState` (`:104-111`), nueva interfaz:
  ```ts
  export type GameTab = 'asp' | 'jug' | 'dat' | 'bib';
  export interface GameSetState {
    title: number;              // índice del juego en la categoría (app.t)
    tab: GameTab;
    inRows: boolean;            // false = foco en las pestañas; true = en las filas
    row: number;
    list: '' | 'magpie' | 'category';  // lista de opciones abierta dentro del panel
    lrow: number;               // fila dentro de esa lista
    confirm: boolean;           // «Quitar»: pulsa A otra vez
  }
  ```
- En la clase del estado, junto a `menu` (`:314`): `gameSet = $state<GameSetState>({ title: 0, tab: 'asp', inRows: false, row: 0, list: '', lrow: 0, confirm: false });`.

### Paso 2 — `src/lib/gameset.svelte.ts` (nuevo, lógica sin UI)
- `GAME_TABS: Array<[GameTab, string, string /*icono*/]>` = `[['asp','Aspecto',ICON.image], ['jug','Al jugar',ICON.monitor], ['dat','Capturas y datos',ICON.import], ['bib','Biblioteca',ICON.saga]]`.
- `gameTabItems(t: Title, tab: GameTab): MenuItem[]` con las **mismas condiciones** que `menuItemsFor` (`controller.svelte.ts:346-363`, tabla del §2):
  - `asp`: `sgdb`, `accent`, `reload`.
  - `jug`: `magpie` (`launchable && source==='manual' && gameId`); `setexe` y `setdir` (además `!IS_TABLET`).
  - `dat`: `folder`, `addshots`, `saves`, `phrases`.
  - `bib`: `addcoll`, `status`, `release`, `category`, `hide|unhide`, `remove`.
- `openGameSettings()`: si no hay juego, nada. Si hay, `app.gameSet = { title: app.t, tab: primera pestaña con filas, inRows: false, row: 0, list: '', lrow: 0, confirm: false }`, `pushOverlay('gameset')`, `blip(1320)`.
- `gameSetAct(item)`:
  - `magpie` → `list = 'magpie'`, `lrow` = índice del modo actual en `MAGPIE_ORDER`.
  - `category` → `list = 'category'`, `lrow = 0`.
  - `remove` → primera A: `confirm = true`, `blip(520)`. Segunda A: seguir con la rama `remove` de abajo.
  - Resto (y `remove` confirmado): `app.menu = { kind: 'detail', title: app.gameSet.title, row: 0, items: [], confirm: item === 'remove' }; void menuAct(item);`.
- `gameSetListAct()`:
  - `magpie`: `setMagpie(t, MAGPIE_ORDER[lrow])` y `list = ''`.
  - `category`: rellenar `app.menu` como arriba y `void menuAct(`cat:${id}`)`. La rama `cat:` de `menuAct` (`:481-494`) ya cierra el overlay, mueve el juego y vuelve a la lista de juegos.
- `gameSetCmd(c)`, la navegación de mando:
  - **Lista abierta** (`list !== ''`): ↑/↓ mueve `lrow`; A llama a `gameSetListAct()`; B o ← → `list = ''`.
  - **Foco en las pestañas** (`!inRows`): ↑/↓ va a la pestaña anterior/siguiente **saltando las vacías** (`gameTabItems(...).length === 0`), sin dar la vuelta; → o A → `inRows = true`, `row = 0` (solo si la pestaña tiene filas); B → `closeOverlay()`.
  - **Foco en las filas**: ↑/↓ mueve `row` dentro de la pestaña; A → `gameSetAct(item)`; ← o B → `inRows = false`.
  - Cualquier movimiento pone `confirm = false`.
  - Start (`'menu'`) cierra siempre.
  - Sonidos como `menuCmd`: `blip(880)` al mover y `blip(520)` al volver o cerrar.

### Paso 3 — `src/lib/controller.svelte.ts`
- **Magpie** (`:540-553`): exportar `MAGPIE_ORDER` y convertir `cycleMagpie(t)` en `export async function setMagpie(t: Title, mode: MagpieMode)`, con el mismo cuerpo pero `next = mode`. La línea `:465` (`'magpie'` en `menuAct`) ya no se alcanza desde la ficha; quitarla.
- **`overlayCmd`** (`:1544-1565`): añadir `case 'gameset': gameSetCmd(c); return;`.
- **Abrir el panel en lugar del menú**: en `:1790` (Y), `:1801` (Start), `:1888` (clic derecho) y en `runAct` `'options'` (`:642`), cambiar `openDetailMenu()` por `openGameSettings()`.
- **Código del menú de la ficha que queda muerto**: borrar `openDetailMenu` (`:407-413`), `menuItemsFor` (`:346-367`) y `MENU_GROUPS` (`:370-374`). `menuLayout` (`:378-390`) queda como lista plana: `return [[{ title: '', items: app.menu.items }]]`. Antes de borrar, comprobar con grep que nadie más usa esas funciones (hoy solo las líneas citadas).
- En `menuAct`, el toast de `:513` pasa a «Imágenes recargadas».
- **Ficha** — `validKey` (`:110-119`): añadir `if (key === 'dcol') return t.id && !t.members ? key : undefined;` y `if (key === 'dopt') return key;`.
- **Ficha** — `runAct` (`:583+`): `'addcoll'` → `openAddColl(app.t)` (de `coleccion.svelte.ts:462`). El botón Ajustes usa `data-act="options"`, que ya existe.

### Paso 4 — `src/screens/GameSettings.svelte` (nuevo)
- **Modelo visual:** la pantalla de Ajustes (`Settings.svelte`; CSS `.scats/.scat` en `app.css:532-537` y `.setpanel` en `:235-237`) y `mockups/menu-juego-c.html`.
- **Panel:** `.glass` de 980×590 en `left:150px; top:56px`, dentro de `.scrim`. El clic en el fondo hace `closeOverlay()`, como `ActionMenu.svelte:102`.
- **Columna izquierda** (`.scats`, 260 px): `h2` «Ajustes del juego», `p` con el nombre del juego y las 4 `.scat`.
  - Pestaña activa: clase `on`. Pestaña activa con el foco en las pestañas: además `f` (relleno `--gf`, como `.srow.f`).
  - Pestaña vacía: clase `dis` (`opacity:.45`, sin hover) con «—» a la derecha; las demás, el número de filas.
- **Columna derecha:** `h3` con el nombre de la pestaña y una línea de ayuda:
  - Aspecto: «Cómo se ve el juego en la biblioteca.»
  - Al jugar: «Se aplica la próxima vez que lo lances desde aquí.»
  - Capturas y datos: «Archivos del juego en este PC.»
  - Biblioteca: «Dónde aparece y cómo se organiza, sin salir de su categoría.»
- **Filas:** `button.srow` de 58 px con `.sl` (etiqueta + `<small>`) y `.sv` (valor), con las etiquetas y subetiquetas de la tabla del §2. Valores:
  - `addcoll`: «En N» / «Ninguna» (N = `platformCollections().filter(c => c.members.includes(t.id)).length`, de `coleccion.svelte.ts:459`; solo cuenta colecciones de la categoría actual).
  - `accent`: un círculo de 18 px con `var(--acc)`.
  - `magpie`: `MAGPIE_LABEL[modo]`, con `magpieSummary` en el `<small>` (lo mismo que `ActionMenu.svelte:82-86`).
  - `category`: `app.platform?.name`.
  - `status`: `STATUS_LABEL[t.status ?? 'none']`.
  - `release`: `fmtRelease(t.release)` o vacío.
- **Filas especiales:** en Biblioteca, un margen de 14 px antes de `category`. `remove` lleva la clase `danger` y, con `confirm`, el texto «Pulsa A otra vez para quitar».
- **Pestaña vacía** (no debería verse, porque se salta): texto «Este juego lo abre su emulador; aquí no hay nada que ajustar.» (Al jugar) o genérico.
- **Lista abierta** (`list`): sustituye las filas de la pestaña.
  - Magpie: 4 filas `MAGPIE_LABEL` con `ICON.check` en la actual y subetiqueta «Mide la ventana al lanzar y elige» en Automático.
  - Category: «Mover a {nombre}» por cada `app.config.platforms` distinta de la actual.
  - Encima, «‹ Escalado con Magpie» / «‹ Mover a otra categoría», pulsable para volver.
- **Ratón:** hover con `makeMoved()` (como `ActionMenu.svelte:96-99`). Pasar por una pestaña la selecciona (`tab`, `inRows=false`). Pasar por una fila pone `inRows=true` y `row`. Clic en una fila → `gameSetAct`.
- **Hints:**
  - Foco en las pestañas: `[['A','Entrar'],['B','Cerrar']]`.
  - Foco en las filas: `[['A','Elegir'],['B','Pestañas']]`.
  - Con `confirm`: A = «Confirmar».
- **Montarlo:** `src/App.svelte:181`, junto a `ActionMenu`: `{#if app.overlay === 'gameset'}<GameSettings />{/if}` (e importarlo).

### Paso 5 — `src/screens/Detail.svelte`
- En `.dchips` (`:157-201`), tras la píldora de fecha, nueva píldora Colecciones (solo si `T.id && !T.members`):
  - clases `relpill pill fx`, `class:f={app.focus === 'dcol'}`, `data-k="dcol"`, `data-act="addcoll"`, `onclick={() => openAddColl(app.t)}`;
  - icono `ICON.saga`; texto `<b>En {n} colecciones</b>`, o `<b>Añadir a colección</b>` si n = 0 (mismo cálculo de n que en el paso 4).
- Botón Ajustes: `relpill pill fx`, `data-k="dopt"`, `data-act="options"`, icono `ICON.gear`, texto «Ajustes», `onclick={openGameSettings}`. Va con `position:absolute; top:108px; right:310px` (lado derecho del banner, fuera de `.dchips`, z-index 6).
- La navegación espacial ya usa la posición de los `data-k`, así que no hace falta grafo. Comprobar a mano que ↑/↓/←/→ llega y sale de `dcol` y `dopt`.
- Hints (`:261`): A = «Colecciones» si `focus === 'dcol'`, «Ajustes del juego» si `focus === 'dopt'`; la última pista pasa de `['Start','Opciones','menu']` a `['Start','Ajustes del juego','menu']`.
- El estilo `.relpill` (`:274-277`) sirve para los dos; añadir el foco `.relpill.f` si no existe (ver cómo se enfoca hoy `drel`).

### Paso 6 — `src/screens/ActionMenu.svelte`
- Ya no se abre con `kind: 'detail'`. Los menús de capturas (`kind: 'shots'`) y de la lista (`kind: 'titles'`, `openTitlesMenu` `controller.svelte.ts:1018`) **no cambian de forma**.
- `:105`: quitar la rama `'Grafismos y opciones'`. El último caso queda como el título que use el menú de la lista o de las capturas: `kind === 'detail'` ya no existe.
- `menuLayout` es ahora plana, así que `grouped` siempre es `false` y desaparecen las dos columnas. Se puede dejar el código de grupos o limpiarlo (`HH`, `colHeads`, `.menupanel.two`, `.mhead`), siempre que el menú de la lista siga igual.
- Etiquetas compartidas con el menú de la lista (`sgdb`, `addcoll`, `accent`, `reload`, `hide`, `unhide`, `remove`, `folder`): ponerles los nombres nuevos del §2 por coherencia. `status` se queda como «Estado: X» en ese menú. Opcional: que `GameSettings` reutilice una función `menuLabel(it)` exportada desde aquí o desde `gameset.svelte.ts`, en lugar de duplicar el `switch`.

### Paso 7 — Tablet
- No hace falta nada aparte: `gameTabItems` aplica `!IS_TABLET` a `folder`, `addshots`, `setexe` y `setdir`.
- Resultado: Al jugar queda solo con Magpie; Capturas y datos con Partidas guardadas y Frases (o vacía y apagada si no es novela ni tiene gameId).
- El botón Ajustes y la píldora Colecciones también salen en la tablet.

### Paso 8 — Documentación y subida
- `README.md:107` y `:126`: «Grafismos y opciones» → «Ajustes del juego» (Start, Y o el botón Ajustes de la ficha) y la lista de pestañas.
- Commit y push (CLAUDE.md). Ojo con los finales de línea mezclados LF/CRLF: contar los CR de los ficheros tocados antes del commit.

### Comprobación de C
- `npm run check` (svelte-check) sin errores nuevos; no hace falta compilar Tauri.
- **Navegador** (`npm run dev`, datos de `src/lib/mock.ts`), juego .exe añadido a mano en una categoría de novelas:
  - **Ficha:** se ven las píldoras Estado, Fecha y Colecciones y el botón Ajustes. Con la cruceta se llega a las dos nuevas. A en Colecciones abre «Añadir a una colección» y al volver el número se actualiza. A en Ajustes abre el panel.
  - **Abrir el panel:** Start, Y y clic derecho también lo abren.
  - **Pestañas:** foco en la pestaña Aspecto; ↑/↓ recorre las 4 y la derecha cambia al momento; → o A entra; ← o B vuelve a las pestañas; B otra vez cierra; Start cierra desde cualquier sitio.
  - **Acciones:** cada fila hace lo mismo que hacía en el menú viejo. Imágenes, Color, Guardados, Frases, Estado y Fecha se abren encima y al cerrarlos se vuelve al panel. Carpeta, Añadir capturas, Ejecutable y Carpeta de trabajo cierran el panel, como antes.
  - **Magpie:** A abre los 4 modos con ✓; elegir uno lo guarda y el valor de la derecha cambia; B vuelve sin cambiar.
  - **Mover a otra categoría:** lista con las categorías de siempre; elegir una mueve el juego y vuelve a la lista de juegos.
  - **Quitar:** primera A → «Pulsa A otra vez para quitar»; moverse lo cancela; segunda A lo quita.
  - **Ocultar / Volver a mostrar:** funciona y la etiqueta cambia.
- **Juego de emulador:** Al jugar apagada con «—» y saltada al navegar. Título solo de capturas: Biblioteca sin Colecciones, Estado ni Fecha.
- **Tablet** (`IS_TABLET`): sin Abrir carpeta, Añadir capturas, Ejecutable ni Carpeta de trabajo; las pestañas vacías, saltadas.
- **Sin regresiones:** el menú de capturas (Start en capturas o visor) y el de la lista de juegos se ven y funcionan igual (salvo los textos renombrados).
- **Ratón:** hover mueve el foco en pestañas y filas; clic en una fila la ejecuta; clic fuera cierra.

### Riesgos de C
- **`menuAct` depende de `app.menu`:** si alguna rama lee `app.menu.row` o `app.menu.items` (hoy `category` y `remove`), está cubierta por las excepciones del paso 2. Revisar `:448-519` al implementar por si hay otra.
- **Pila de overlays:** comprobar que `closeOverlay()` saca solo el de arriba (sgdb, status…) y vuelve a `'gameset'`, como hoy volvía a `'menu'`.
- **Hábito:** «Iniciar juego» ya no está en el menú; queda el botón Jugar y X en la ficha.
