# Plan: revisión 9-nov, fases 2 y 3

Fuente: `docs/informe-revision-2026-11-09.md`. Regla de `CLAUDE.md`: las categorías no se tocan; todo vive dentro de cada categoría y nunca se mezclan títulos de categorías distintas (el buscador global agrupa por categoría, una fila cada una).

**Requisito previo:** la fase 1 (fallos 1, 2, 3, 4, 7, 9, 13) ya está integrada. Los números de línea son del 9-nov con la fase 1 a medio aplicar, así que pueden haberse movido unas líneas: **busca siempre por el nombre de la función o por el texto citado** y usa la línea solo como orientación.

**Comprobación común a todas las tareas:** `npm run check | tail -5` sin errores nuevos, y una prueba en `npm run dev` con `?mock` a 1280×720 usando el teclado como mando (Enter = A, Retroceso = B, m = Start, q/e = LB/RB, z/c = LT/RT, v = View, x/y). Revisa también el tema «Noche ahumada». Al acabar, commit y push según `CLAUDE.md`, sin `git add -A` (hay ficheros sin seguimiento que no son tuyos). Antes del commit, comprueba los finales de línea: el repositorio mezcla LF y CRLF, y cada fichero debe conservar los que tenía.

## Decisiones tomadas (para que el usuario las vea)

1. **Inicio, Start:** la pista pasa a «Menú», pero Start sigue abriendo los Ajustes globales, que hacen de menú de Inicio.
2. **Mi lista, X:** pasa a «Jugar», como dice el contrato C. «Fijar o soltar» se va al menú Start (elemento nuevo `pin`). Y pasa a llamarse «Estado» (antes «Opciones»): sigue abriendo la barra de estado.
3. **Colecciones, X:** solo «Jugar» (el juego enfocado, si se puede lanzar). «Ver capturas de la colección» pasa al menú Start (elemento nuevo `collshots`).
4. **Ficha:**
   - Con el foco en «Jugar», A ya dice «Jugar» y la pista X no se muestra.
   - Fuera de Jugar, X = Jugar.
   - Y solo actúa sobre los logros; antes, en el resto de sitios, abría los Ajustes del juego, igual que Start.
5. **Capturas y visor:** X sigue siendo «Favorita», porque ahí no hay nada que jugar.
6. **Estadísticas:**
   - «Periodo» (Mes / Año / Siempre) pasa de X a View.
   - Y sigue cambiando de vista, y ahora también se puede pulsar en el segmentado.
7. **Fallo 6:** no se cambia el cálculo. Se muestra el ámbito: «Juegos PC · Siempre» en la Clasificación, y «Todas las categorías · 2026» y «Todas las categorías · Últimos 6 meses» en el Resumen anual y el Calendario.
8. **Visor:**
   - Se abre en modo desnudo (solo la imagen).
   - Cualquier botón o el stick enseñan durante 2 s la información abajo a la izquierda y las pistas abajo a la derecha.
   - View pasa al «Marco» (el modo de ahora).
   - Y = «Detalles»: añade la resolución a la píldora y la deja fija.
   - B sale del visor directamente, sin pasar primero por el marco.
9. **Mi lista, carriles:**
   - Siempre se dibujan los 5 carriles. Los vacíos se pliegan a una línea de 30 px («Pendiente · 0») y no se pueden enfocar.
   - El alto que sobra se reparte en filas extra para los carriles que lo necesitan.
   - Si un carril no cabe ni así, se desplaza en vertical por dentro.
10. **Avatar (fallo 11):** pasa a ser la racha (llama y días, el máximo de las metas activas) y abre la Racha. Solo es interactivo en Inicio. Sin metas, se queda como un adorno pequeño sin cristal.
11. **⏻ (fallo 12):** el botón de arriba pasa a ser una luna («Suspender el PC») y ⏻ queda solo para «Salir».
12. **«Juego de hoy» (fallo 10):** se quita la rotación automática de la tarjeta; solo cambia con LB/RB o con un clic. En la fase 3 (D1) la tarjeta pasa a mostrar una sola categoría.
13. **D1, dock de 4 elementos** (Buscar, Favoritas, Estadísticas, Añadir juego):
    - «Ajustes» queda en Start.
    - «Salir» desaparece del dock: queda la fila que ya existe en Ajustes, «Salir de la aplicación», y cerrar la ventana sigue respetando la bandeja.
14. **D1, tarjeta de metas:**
    - Muestra una sola categoría con meta («Juego de hoy · PC»), y LB/RB recorren las categorías.
    - Al abrirla, sale la Racha de esa categoría.
    - Al entrar, se muestra la más urgente o, si no hay ninguna urgente, la última jugada.
    - «Últimos 7 días» pasa a la cabecera «Seguir jugando · Últimos 7 días: …».
15. **D2, buscador:**
    - Reutiliza la regla de búsqueda del Catálogo (`app.catalogSearch`, store.svelte.ts:553-556), que se extrae a una función compartida, pero con una consulta propia: compartir `app.catalogSearch` dejaría filtrado el Catálogo de otra categoría al volver.
    - Los resultados salen en una fila por categoría, en el orden de la biblioteca, sin ocultos y con un máximo de 12 por fila.
16. **D3:** el lector (`reader.html`) queda fuera; su tema oscuro es otra ventana y tiene prioridad baja. La luz blanca del botón de suspender armado (`#fff` sobre rojo) se queda como está.

---

## Fase 2

### Paralelismo

```
T0 (controller.svelte.ts y lógica .ts) ─┐
T1 pistas y títulos   (Titles, Detail, GameSettings)
T2 visor             (Viewer.svelte y el bloque del visor de app.css)
T3 Estadísticas      (Stats.svelte)
T4 Mi lista          (MiListaDual.svelte)
T5 Inicio y barra    (Home.svelte, ReadCard.svelte, TopBar.svelte)
```

- T0 es la única tarea que toca `controller.svelte.ts`, `store.svelte.ts`, `milista.svelte.ts`, `coleccion.svelte.ts` y `ActionMenu.svelte`.
- T1 a T5 no comparten ficheros entre sí ni con T0, y usan los nombres que T0 define más abajo (contrato fijo). Por eso pueden lanzarse las seis a la vez.
- Si se trabaja con ramas, integra T0 primero. Hasta que esté, `svelte-check` puede quejarse de `app.vinfoMore` en T2.
- T0 es trabajo de un solo implementador: son cambios pequeños en muchos puntos, y no conviene pasarlo a agy.

### T0. Controlador y lógica (contrato de botones, visor, estadísticas, avatar)

Ficheros: `src/lib/controller.svelte.ts`, `src/lib/store.svelte.ts`, `src/lib/milista.svelte.ts`, `src/lib/coleccion.svelte.ts` y `src/screens/ActionMenu.svelte`.

**Contrato que usan las demás tareas (respétalo al pie de la letra):**
- `app.vinfoMore: boolean` (store).
- Las órdenes de `cmd()`: `'stab:rank' | 'stab:cal' | 'stab:year'`.
- La acción `data-act="avatar"` con la clave de foco `avatar` en Inicio.
- Los elementos de menú `'pin'` y `'collshots'`.
- `export const BAND_DEF` en milista.

1. **store.svelte.ts**
   - En `export type MenuItem` (alrededor de la línea 85), añade `'pin' | 'collshots'`.
   - Junto a `vbare = $state(false)` (alrededor de la 395), añade `vinfoMore = $state(false)`.
2. **cmd(), caso `'x'`** (alrededor de las líneas 1819-1827):
   - Quita la rama `app.view === 'stats' && app.statsTab === 'rank'`.
   - La rama de `titles` ya la dejó la fase 1 como Jugar; no la toques.
3. **cmd(), caso `'view'`** (alrededor de la 1832): añade `else if (app.view === 'stats' && app.statsTab === 'rank') cycleStatsKind();`.
4. **cmd(), caso `'y'`:**
   - En `detail`: cuando `focus !== 'dach'`, haz `bump(screenEl()?.querySelector('.fx.f'), 0, 0)` en lugar de `openGameSettings()`.
   - Añade `else if (app.view === 'viewer') { app.vinfoMore = !app.vinfoMore; wake(); blip(700); }`.
5. **cmd(), antes del `switch`:** añade `if (c.startsWith('stab:')) { setStatsTab(c.slice(5) as typeof app.statsTab); return; }`.
6. **Estadísticas:**
   - Nueva función `export function setStatsTab(t)` junto a `toggleStatsTab` (alrededor de la 767). Hace lo mismo que esta, pero con un destino fijo, y no hace nada si ya es la pestaña actual.
   - **`openStats()`** (alrededor de la 742): tras reiniciar, abre en el último periodo con partidas:
     ```ts
     for (let k = 0; k < 240 && !app.rank.rows.length && app.rank.older; k++) app.statsOff--;
     ```
     `app.rank` es `$derived` y se recalcula al leerlo; el tope evita un bucle infinito.
7. **Visor:**
   - En `go()` (alrededor de la 336), cambia `app.vbare = false;` por `app.vbare = v === 'viewer'; app.vinfoMore = false;`.
   - En `back()` (alrededor de la 866), borra la línea `if (app.vbare) { app.vbare = false; ... }`: B sale del visor directamente.
   - En `wake()` (alrededor de la 1226), cambia `3200` por `2000`.
   - En `onScreenClick` (alrededor de la 1931, «`!app.vbare &&`»), quita la condición `!app.vbare`: en modo desnudo, el clic a izquierda o derecha también pasa de captura.
8. **Colecciones** (`coleccion.svelte.ts`, `collCmd`, líneas 181-204):
   - `case 'x'`: `const ti = sh?.items[at.i]; const t = ti !== undefined ? app.platform?.titles[ti] : undefined; if (t?.launchable) void playTitle(t); else bump(flash(), 0, 0);`. Importa `playTitle` del controlador.
   - Nuevo `case 'menu'`: `if (sh?.items.length) app.t = sh.items[at.i]; openTitlesMenu(); return true;`. Ahora mismo, Start en Colecciones usa un `app.t` viejo.
9. **openTitlesMenu()** (alrededor de la 1016):
   - Tras `items.push('listopts')`, añade `if (app.titlesTab === 'coll' && app.collShelves[app.collAt.c]) items.push('collshots');`.
   - Dentro de `if (t)`, añade `if (app.titlesTab === 'list' && t.id && t.status === 'playing') items.push('pin');`.
   - En `menuAct()` (alrededor de la 415), antes de `if (app.menu.kind === 'shots')`, añade:
     - `if (item === 'collshots') { closeOverlay(); openCollShots(app.collAt.c); return; }`
     - `if (item === 'pin') { closeOverlay(); togglePin(); return; }`
10. **milista.svelte.ts:**
    - Exporta `togglePin` (línea 207) y `BAND_DEF` (línea 143).
    - En `listCmd`, `case 'x'` (líneas 380-385): siempre `playTitle` si `t.launchable && listHas(app.t)`; si no, `bump`. Quita la rama `onBoard() → togglePin`.
    - **`boardMove`, parte vertical** (líneas 237-250): en lugar de saltar de franja, busca entre todos los `#screen .board .it` el elemento de la fila visual más cercana en la dirección `dy`:
      - Candidatos: centro y mayor (o menor) que el centro y actual más 20 px.
      - Entre ellos, se queda con el de menor |Δy| y, a igual fila, con el de menor |Δx|.
      - Si no hay ninguno, `bump`.
      - Así se navega por las filas de un carril de varias filas y entre carriles.
11. **ActionMenu.svelte:**
    - En `label()` (líneas 43-80): `case 'pin': return app.config.pinned?.includes(t?.id ?? '') ? 'Soltar de «En marcha»' : 'Fijar en «En marcha»';` y `case 'collshots': return 'Ver capturas de la colección';`.
    - En el mapa de `icon` (línea 89): `pin: ICON.check, collshots: ICON.image`.
12. **Avatar (fallo 11):**
    - En `validKey`, rama `home` (alrededor de la 95): `if (key === 'avatar') return app.readCard ? key : undefined;`.
    - En `runAct` (alrededor de la 591): `else if (a === 'avatar') openStreak();`.

**Comprueba:**
- En Colecciones, X lanza el juego, y Start muestra «Ver capturas de la colección» del juego enfocado.
- En Mi lista, X juega y Start › Fijar funciona.
- En Estadísticas, «v» cambia el periodo y la Clasificación no se abre vacía si hay meses anteriores con partidas.
- En el visor, B sale directamente, «y» alterna los detalles y el clic lateral pasa de captura.

### T1. Pistas y títulos (Start = «Menú»; ficha; Colecciones; Mi lista)

Ficheros: `src/screens/Titles.svelte`, `src/screens/Detail.svelte` y `src/screens/GameSettings.svelte`.

1. **Titles.svelte**, pistas de Mi lista con títulos (líneas 333-340): `['X', 'Jugar', 'x']` solo si `cur?.launchable` (usa el título `app.platform?.titles[app.t]`) y `['Y', 'Estado', 'y']`.
2. **Titles.svelte**, Colecciones (líneas 343-349): cambia `['X', 'Jugar / Capturas', 'x']` por `['X', 'Jugar', 'x']`, solo si el título enfocado se puede lanzar: `app.platform?.titles[app.collShelves[app.collAt.c]?.items[app.collAt.i]]?.launchable`. Mantén `['Y', 'Editar', 'y']`.
3. **Detail.svelte**, pistas (líneas 260-266):
   - Cambia `['Start', 'Opciones', 'menu']` por `['Start', 'Menú', 'menu']`.
   - La X solo si `T.launchable && !playing && app.focus !== 'play'`.
   - Pon `B` y `Start` en el mismo orden que en el resto de pantallas: Start antes que B.
4. **GameSettings.svelte**:
   - En la línea 76, cambia `<h2>Ajustes</h2>` por `<h2>Ajustes del juego</h2>`. Comprueba que cabe en los 290 px del panel; si no, baja un poco el tamaño de letra solo en este `h2`.
   - En el estilo `.gs-tab em` (dentro de `<style>`), añade `margin-left:auto; padding-left:10px` para que «Capturas y datos» no quede pegado al contador (aspecto, «datos3»).

**Comprueba** que en la ficha nunca salen a la vez «A Jugar» y «X Jugar», y que en Catálogo, Mi lista, Colecciones, ficha y Capturas, Start se llama «Menú».

### T2. Visor (diseño D, fallo 5)

Ficheros: `src/screens/Viewer.svelte` y `src/app.css`, **solo** el bloque del visor (líneas 174-212 y 258: `.vw`, `.vframe`, `.vinfo`, `.vstrip`, `.vw .hints`, `.vw.bare`).

1. **Viewer.svelte**, píldora (líneas 88-92):
   - Añade `class:more={app.vinfoMore}`.
   - La resolución solo con `{#if app.vinfoMore}`.
2. **Viewer.svelte**, pistas (líneas 102-104), sin zoom:
   ```
   [['LB/RB','±10','rb'], ['X','Favorita','x'], ['Y', app.vinfoMore ? 'Menos datos' : 'Detalles','y'], ['View', app.vbare ? 'Marco' : 'Solo imagen','view'], ['Start','Menú','menu'], ['B','Volver','b']]
   ```
   «Stick D Zoom» desaparece de aquí, pero sigue en las pistas con zoom.
3. **app.css**, todo abajo:
   - `.vframe`: `top:14px` (se queda con el mismo alto).
   - `.vinfo`: `top:auto; bottom:104px; max-width:520px`.
   - `.vw .hints`: `top:auto; bottom:104px`.
   - Queda una fila 578-616 entre el marco y la tira, que empieza en 638.
4. **app.css**, modo desnudo:
   - En la regla `.vw.bare .vbg,.vw.bare .vinfo,.vw.bare .vstrip,.vw.bare .varrow,.vw.bare .hints{display:none}`, quita `.vinfo` y `.hints`.
   - Añade `.vw.bare .vinfo,.vw.bare .hints{bottom:16px}`.
5. **app.css**, ocultar con inactividad: `.vw.idle .vinfo` pasa a `.vw.idle .vinfo:not(.more)`. Las pistas siguen ocultándose.

**Comprueba:**
- La captura se abre a pantalla completa.
- Al pulsar cualquier tecla, la información (n/N, juego, fecha) y las pistas salen abajo y desaparecen a los 2 s, sin solaparse a 1280 px con títulos largos.
- «v» muestra el marco con la tira y las flechas, y la información no tapa la imagen.
- «y» deja fija la píldora con la resolución.
- La animación de entrada desde la miniatura (`morph` a `.vframe`) sigue funcionando.

### T3. Estadísticas (fallo 6, diseño F, §2.9)

Fichero: `src/screens/Stats.svelte`.

1. **Segmentado visible:** cambia el bloque `.vsw` (líneas 93-96) por tres pastillas «Clasificación · Calendario · Resumen anual»:
   - Cada una con `data-hact="stab:rank|cal|year"`.
   - La activa, con la clase `gem`.
   - Quita los puntos (`.dots`).
   - Ajusta el CSS de `.vsw` en el `<style>` del fichero.
2. **Ámbito visible:**
   - Clasificación (línea 69): el chip `.cat` muestra `{pname(app.statsPlat)} · {KIND_LABEL[app.statsKind]}`; si no hay categoría, «Sin partidas».
   - Resumen anual (línea 79): `Todas las categorías` (el año ya sale al lado).
   - Calendario (línea 86): `Todas las categorías`.
3. **Pistas:**
   - Clasificación (línea 163): `['A','Ver juego','a']` solo si `rows.length`; cambia `['X','Periodo','x']` por `['View','Periodo','view']`.
   - En la línea 71, el chip del periodo pasa de `data-hact="x"` a `data-hact="view"`.
4. Si la Clasificación sigue vacía (sin partidas en toda la categoría), el texto de `.stnone` (línea 130) no cambia.

**Comprueba:**
- Con el mock, Resumen 2026 indica «Todas las categorías» y la Clasificación indica «Juegos PC · Siempre», así que ya no parece una contradicción.
- El clic en cada pastilla cambia de vista.
- No sale «A Ver juego» sin filas.

### T4. Mi lista: carriles adaptativos (fallo 8, diseño E)

Fichero: `src/screens/MiListaDual.svelte`. Depende de que T0 exporte `BAND_DEF` y haga la navegación vertical espacial.

1. **Carriles:** importa `BAND_DEF` y construye
   ```ts
   lanes = BAND_DEF.map(d => bands.find(b => b.key === d.key) ?? { ...d, compact: d.key === 'done', items: [] })
   ```
2. **Filas por carril** (`$derived`):
   - Alto útil: `H = 564` (de `.board`, top 112 a bottom 44).
   - Carril vacío: 30 px. Hueco entre carriles: 6 px.
   - Una fila mide 100 px más 8 px de hueco.
   - Juegos por fila: 5 en las franjas normales (ancho de cada juego, unos 199 px, sobre un ancho útil de 1138 px) y 6 en la compacta `done` (unos 167 px). Si el separador de año de `done` complica la cuenta, trátalo como si ocupara un juego.
   - Reparto: empieza con 1 fila por carril no vacío y da cada fila sobrante al carril con más `ceil(n/porFila) - filas`. En caso de empate, gana el que va antes en `BAND_DEF`.
   - El alto del carril es `filas*100 + (filas-1)*8 + 8`, aplicado con `style="height:…px"` en `section.band`.
3. **CSS:**
   - `.band`: quita `height:108px`.
   - `.its`: `flex-wrap:wrap; align-content:flex-start; padding:4px 0; overflow:hidden`.
   - Carril vacío (`.band.empty`): 30 px, sin `.vt` vertical, con el texto en línea «{label} · 0» a 13 px en `--ink-3`.
4. **Desplazamiento:** el `$effect` de las líneas 22-34 también desplaza en vertical (`scrollTop`) dentro de `.its` cuando la portada enfocada queda fuera.
5. **Avisos:** «Se aparca en N d» (líneas 84-87) se acorta a «Aparca en {it.warn} d» y `.wtxt` admite salto de línea (`flex-wrap:wrap`). Así no se trunca.
6. **Rueda:** el `onwheel` (línea 41) se queda en horizontal solo para carriles de una fila. En los de varias filas, la rueda baja y sube (`scrollTop`).

**Comprueba:**
- Con el mock, «A medias» enseña todos sus juegos (Outer Wilds entero).
- Los carriles vacíos son una línea.
- Las flechas arriba y abajo recorren las filas dentro de un carril y luego pasan al siguiente.
- No se mezcla nada entre categorías (`buildBoard` ya solo mira `app.platform`).

### T5. Inicio y barra superior (Start = «Menú», fallos 10, 11 y 12)

Ficheros: `src/screens/Home.svelte`, `src/components/ReadCard.svelte` y `src/components/TopBar.svelte`.

1. **Home.svelte:128:** cambia `['Start', 'Ajustes', 'menu']` por `['Start', 'Menú', 'menu']`.
2. **Fallo 10, ReadCard.svelte:**
   - Quita la rotación: `ROT`, `cycle`, `remain`, `t0`, `timer`, `paused`, `flip`, `arm`, los dos `$effect` del ciclo (líneas 48-76) y el `onDestroy`.
   - Actualiza el comentario de las líneas 2-3.
   - Mantén la cara inicial (líneas 36-46).
   - Comprueba que, al pulsar la tarjeta, se abre la racha del grupo que se ve (`runAct 'read'` usa `app.readFace`). Si la cara cambia en el `mousedown` por el `$effect` de la línea 44 (la cara fijada solo se aplica sin foco), aplica la cara fijada solo al montar.
3. **Fallo 12, TopBar.svelte:71-73:** el botón de suspender pasa a ser una luna:
   ```html
   <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linejoin="round"><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/></svg>
   ```
   El `title` y el `aria-label` («Suspender el PC») no cambian.
4. **Fallo 11, TopBar.svelte:56:**
   - Si `pad && app.readCard`, el avatar pasa a ser `class="glass avatar fx" class:f={app.focus === 'avatar'} data-k="avatar" data-act="avatar"`, sin `aria-hidden` y con `aria-label="Racha"`.
   - Dentro va una llama SVG pequeña y el número `best`, con `const rr = pad ? createReadRings({ size: 22, stroke: 3, gap: 1.3, empty: '' }) : null` y `best = Math.max(0, ...rr.tracks.map(t => t.streak))`.
   - `createReadRings` hay que llamarlo durante la inicialización. Si no puede ser condicional, llámalo siempre con `keys: () => pad ? app.goalKeys : []`.
   - En los demás casos (otras pantallas o sin metas), un avatar más pequeño y sin cristal: `class="avatar mini"`, con un CSS local que reduzca la opacidad y quite la sombra.

**Comprueba:**
- La tarjeta de metas ya no cambia sola.
- Clic, o A, sobre el avatar en Inicio abre la Racha.
- El icono de arriba es una luna y el del dock «Salir» sigue siendo ⏻.

---

## Fase 3

### Paralelismo

```
P0 (controller y store para D1 y D2) ─┬─ D1 Inicio (Home.svelte, ReadCard.svelte)
                                      └─ D2 buscador (search.svelte.ts nuevo, Search.svelte nuevo, App.svelte)
D3a app.css   ┐
D3b components├─ en paralelo con todo lo anterior: no comparten ficheros
D3c screens   ┘
```

- P0 es la única tarea de la fase que toca `controller.svelte.ts` y `store.svelte.ts`. Igual que en la fase 2, D1 y D2 pueden empezar a la vez con el contrato de abajo.
- D3 no toca `Home.svelte`, `ReadCard.svelte`, `TopBar.svelte` ni ningún fichero de D1 o D2.
- D3b y D3c usan los tokens que define D3a, con los nombres fijados más abajo.

### P0. Controlador y store (D1 y D2)

Ficheros: `src/lib/controller.svelte.ts` y `src/lib/store.svelte.ts`.

**Contrato:**
- `Overlay` incluye `'search'`.
- `app.homeGoal: GoalKey | ''`.
- La función `titleMatches(name: string, q: string): boolean` exportada desde el store.
- `openSearch()` y `searchCmd(c)` viven en `src/lib/search.svelte.ts`, que escribe D2. P0 los importa.
- La orden `'hgoal:<k>'`.

1. **store.svelte.ts:**
   - `export const titleMatches = (name: string, q: string) => name.toLowerCase().includes(q.toLowerCase().trim());`. Úsala en `tview` (líneas 553-556) sin cambiar su comportamiento.
   - Añade `'search'` al tipo `Overlay` (línea 90).
   - Añade `homeGoal = $state<GoalKey | ''>('')` junto a `readFace` (línea 437). `readFace` y `goalFaces` se pueden borrar cuando D1 deje de usarlos: búscalos con grep al final.
2. **Dock** (`DOCK_N`, línea 46): pasa a `4` y actualiza el comentario de la línea 45.
   - `dock()` (línea 630): 0 → `openSearch()`, 1 → `openFavs()`, 2 → `openStats()`, 3 → `openAdd()`. Borra las ramas 4 y 5. `closeApp` sigue usándose desde el tray y Ajustes; si se queda sin uso, bórralo.
   - `validKey`, el dock (línea 99): `n >= 0 && n < DOCK_N && !(IS_TABLET && n === 3)`.
   - `defaultKey('home')` (línea 136): en la tablet sin raíz, `'dock1'` sigue siendo válido.
3. **Overlay:**
   - En `overlayCmd` (línea 1571), añade `case 'search': searchCmd(c); return;`.
   - En `onKeyDown` (línea 1885), amplía el selector `.closest('.tsearch')` a `.closest('.tsearch, .gsearch')`.
   - Además, si el foco está en `.gsearch input` y la tecla es `ArrowDown`, haz `blur()`, `preventDefault()` y `cmd('down')`: así se baja a los resultados.
4. **Meta de Inicio:**
   - `runAct 'read'` (línea 591): `openStreak(app.homeGoal || undefined)`.
   - `stepReadFace(d)` (línea 668): recorre `app.goalKeys` con `homeGoal`, con vuelta, y hace `bump` si hay menos de 2.
   - `setReadFace` pasa a `setHomeGoal(k: GoalKey)` y `cmd` atiende `'hgoal:'` en lugar de `'face:'` (línea 1807).
   - Borra lo que quede sin uso.
5. **Home**, `page()` (línea 923): `if (app.focus === 'read') stepReadFace(d)` se queda igual.

### D1. Inicio más limpio (diseño A)

Ficheros: `src/screens/Home.svelte` y `src/components/ReadCard.svelte`.

1. **Home.svelte, `DOCK`** (líneas 11-18): Buscar (sin `hide`), Favoritas, Estadísticas y Añadir juego. En la línea 120, el filtro de la tablet queda en `i === 3`.
2. **Home.svelte, «Últimos 7 días»:**
   - Borra el `.hweek` flotante (líneas 95-97) y su CSS.
   - La cabecera de la línea 94 pasa a `Seguir jugando{#if week.min > 0 || week.shots > 0}<small> · Últimos 7 días: <b>…</b> · <b>…</b></small>{/if}`, con el mismo texto que antes.
3. **Home.svelte, `meta()`** (líneas 39-43): separa la fecha del total: `when` = «Jugado {fmtAgo}», `rest` = «{fmtDur} en total» (§2.8).
4. **Home.svelte, pistas** (línea 128): `LB/RB` dice «Categoría» y aparece cuando `app.focus === 'read' && app.goalKeys.length > 1`.
5. **ReadCard.svelte**, de una cara por grupo a una cara por categoría:
   - `k = app.homeGoal`.
   - Título: `GOAL_TXT[groupOf(k)].card + ' · ' + GOAL_COL[k].tab`.
   - Anillo: `<ReadRings keys={[k]} active />`.
   - La línea `rmeta` muestra la meta solo de esa categoría.
   - El selector pequeño (líneas 83-100) pasa a ser un punto por categoría de `app.goalKeys`, con `data-hact="hgoal:{k}"`.
   - Valor inicial al montar, si `homeGoal` no está en `goalKeys`: la categoría de más nivel según `level()`, calculado ahora por categoría con un `createReadRings` de `keys: () => app.goalKeys`. Si no hay ninguna urgente, la de `lastPlayAt([k], …)` más reciente.
   - Nunca se mezclan categorías dentro del anillo.

**Comprueba:**
- El dock tiene 4 elementos y «Buscar» abre el buscador.
- La tarjeta no rota.
- LB/RB pasan PC → Switch → Novelas…
- A abre la racha de la categoría que se ve.
- La tablet no muestra «Añadir juego».

### D2. Buscador global (diseño B)

Ficheros nuevos: `src/lib/search.svelte.ts` y `src/screens/Search.svelte`. Fichero que se edita: `src/App.svelte` (junto a la línea 202, `{#if app.overlay === 'search'}<Search />{/if}`).

Sigue el patrón de `src/lib/gameset.svelte.ts` (`openGameSettings` / `gameSetCmd`) y de `GameSettings.svelte`.

1. **search.svelte.ts:**
   - `export const srch = $state({ q: '', r: -1, c: 0 })`. `r = -1` es el campo de texto.
   - `export const searchGroups()`: por cada `app.lib.platforms[pi]`, en su orden, los `ti` con `!t.hidden && titleMatches(t.name, srch.q)`, con un máximo de 12. Devuelve `{ pi, name, items: ti[] }` solo de las categorías con resultados, y nada si `q.trim()` está vacío. Una fila por categoría; nunca se mezclan.
   - `openSearch()`: reinicia `srch`, `pushOverlay('search')` y `blip(1320)`.
2. **`searchCmd(c)`:**
   - Arriba y abajo cambian de fila (de -1 a la última); izquierda y derecha, de columna, con `c` limitado a la fila. Fuera de rango, `bump`.
   - A sobre el campo: `openOsk({ label: 'Buscar', value: srch.q, onChange: v => (srch.q = v), onDone: v => (srch.q = v) })`.
   - A sobre un resultado: `closeOverlay(); app.p = pi; app.hp = pi; openTitle(ti, 'home')`. Comprueba que B desde la ficha vuelve a Inicio.
   - X: `playTitle(t)` si `t.launchable`; si no, `bump`.
   - B o Start: `closeOverlay()`.
   - Al cambiar `q`, reinicia `r` y `c`.
3. **Search.svelte:**
   - Capa a pantalla completa (`.scrim`).
   - Arriba, un campo `<div class="gsearch"><input bind:value={srch.q} autofocus …></div>`, que se escribe con el teclado físico. Copia el estilo de `.tsearch` de `Titles.svelte` (líneas 202-218).
   - Debajo, una fila por categoría: el nombre de la categoría y sus portadas (`Thumb` o `MiniCover`, como en Stats), con el foco `class:f`.
   - Sin resultados: «Ningún juego coincide con «…»».
   - Pistas: `[['A', r < 0 ? 'Escribir' : 'Abrir','a'], ['X','Jugar','x'] si se puede lanzar, ['B','Cerrar','b']]`.
   - Clic en una portada: el mismo efecto que A, llamando a las funciones de `search.svelte.ts`.
   - Usa solo tokens (`--panel`, `--glass-chip`, `--ink*`), sin blancos fijos, para que funcione en los dos temas.

**Comprueba:**
- Al escribir «el» salen filas separadas por categoría, sin ocultos.
- A abre la ficha en su categoría y X juega.
- Funciona con el teclado en pantalla (A sobre el campo) y con el teclado físico.
- Al cerrar, el Catálogo no queda filtrado.

### D3. Resto del tema oscuro (diseño G)

**Tokens nuevos** (los define D3a; D3b y D3c los usan con estos nombres):

| Token | Claro | Oscuro | Para qué |
|---|---|---|---|
| `--hl` | `rgba(255,255,255,1)` | `rgba(255,255,255,.10)` | Brillo interior superior (`inset 0 1.5px 0`) |
| `--hl-ring` | `rgba(255,255,255,.55)` | `rgba(255,255,255,.06)` | Aro interior (`inset 0 0 0 1px`) |
| `--hl-bot` | `rgba(255,255,255,.95)` | `rgba(255,255,255,.04)` | Brillo interior inferior (`inset 0 -10px 14px -8px`) |
| `--tint-base` | `#fff` | `#262a33` | Base de los `color-mix(… white)` y de los fondos pastel |

**Regla de conversión:**
- Los fondos blancos translúcidos pasan al token existente más cercano:
  - `.80` → `--glass-chip`.
  - `.75` → `--panel`.
  - `.72` → `--bub-bg` o `--glass-top`.
  - `.62` → `--tile-bg`.
  - `.55` → `--glass-in` o `--soft-top`.
- Los brillos `inset` pasan a `--hl`, `--hl-ring` y `--hl-bot`.
- `white` dentro de `color-mix` pasa a `var(--tint-base)`.
- El texto blanco sobre un relleno de color (`color:#fff` o `white`) pasa a `var(--on)` o se queda igual. No lo conviertas a `--ink`.

D3a, D3b y D3c son trabajo repetitivo, aptos para **agy**, siempre que quien delega revise el diff y lo pruebe en los dos temas.

- **D3a, `src/app.css` (agy):**
  - Añade los 4 tokens en `#stage` (líneas 18-29) y en `#stage[data-theme=dark]` (líneas 31-39).
  - Convierte los blancos que quedan: `grep -n "rgba(255,255,255" src/app.css`, que hoy da 48, menos los `--*` de las líneas 20-24, que ya son tokens.
  - **No toques** el bloque del visor (líneas 174-212, de T2), que es negro a propósito.
- **D3b, `src/components` (agy):** AchievementCard.svelte (13 + 9 `#fff`), Shelf.svelte (14 + 9), ResumePill.svelte (2 + 1) y HuchaRescate.svelte (1). Lista con `grep -c "rgba(255, \?255, \?255\|#fff\b\|white\b"`.
- **D3c, `src/screens` (agy):**
  - Ficheros: MiListaDual.svelte (11; líneas 139-141, 155-157 y 208-210: los `inset` de `.band`, `.st-run` e `.it`), StatusBar.svelte (6 + 3), Saves.svelte (3), ReleasePicker.svelte (2), Streak.svelte (2), GamePicker.svelte (1), Detail.svelte (1), Titles.svelte (5 `#fff`) y AchievementsModal.svelte (2).
  - **Racha:** las tarjetas pastel usan `GOAL_COL[k].soft` (readgoals.ts:15-18). En Streak.svelte, donde se pinte `soft` como fondo, usa `color-mix(in srgb, <soft> 35%, var(--tint-base))` para que en oscuro quede oscuro. No cambies `readgoals.ts`.
  - D3c depende de que T4 esté integrado, porque los dos tocan MiListaDual.svelte.

**Comprueba D3** en «Noche ahumada» y en el tema claro, pantalla por pantalla: Inicio, Catálogo, Mi lista, Colecciones, ficha, logros, barra de estado, partidas guardadas, fecha de lanzamiento y Racha. En el tema claro no debe cambiar nada visible; compara capturas antes y después. Al final, `grep "rgba(255,255,255" src/screens src/components` debe dar solo usos justificados (brillos ya convertidos en token o texto sobre color).

## Riesgos

- **controller.svelte.ts** se está editando en la fase 1 mientras escribo esto. Las líneas se han movido unas 4 entre dos lecturas. Ancla siempre en el nombre de la función.
- **El visor en modo desnudo por defecto** cambia la animación `morph` de la miniatura al marco: ahora el destino es la pantalla completa. Si queda brusca, que el marco aparezca primero y el modo desnudo se active al terminar `morph`.
- **La navegación espacial de Mi lista** (T0, punto 10) depende de las posiciones del DOM. Con el desplazamiento vertical dentro de un carril, los elementos ocultos siguen teniendo sus coordenadas reales, lo cual es correcto, pero el `$effect` de T4 debe desplazar después de `setFocus`.
- **TopBar en Inicio** calcula las rachas (`goalTrack`) en cada render. Solo se hace con `pad`, es decir, en Inicio.
