# Plan: interfaz pendiente tras la 2.0.0

Bloques: Estadísticas (Semana, Terminados, Panorama), Mi lista, resumen semanal, Inicio, Colecciones y animaciones.

**Regla de `CLAUDE.md`:** las categorías no se tocan. Todo vive dentro de su categoría, y ninguna lista, carril ni top mezcla títulos de categorías distintas. Solo pueden sumar categorías las cifras de cabecera (tiempo total, días activos), como ya hace `yearTotals`. Un destacado suelto puede mostrar un título de cualquier categoría si lleva su etiqueta, como hace hoy `Year.svelte:17` (`pname`).

**Comprobación común a cada fase:**

1. `npm run check | tail -5` sin errores nuevos.
2. `npm run dev` con `?mock` a 1280×720, usando el teclado como mando: Enter = A, Retroceso = B, m = Start, q/e = LB/RB, z/c = LT/RT, v = View, x/y.
3. Revisar en tema claro y en «Noche ahumada».
4. Commit y push según `CLAUDE.md`, sin `git add -A`: hay maquetas sin seguimiento que no son de la fase.
5. Antes del commit, comprobar los finales de línea. El repositorio mezcla LF y CRLF, y cada fichero debe conservar los que tenía.

**Líneas citadas:** son del 10-nov (commit `0a50e7e`) y sirven solo de orientación. Busca siempre por el nombre de la función o por el texto citado.

---

## 1. Qué falta de verdad (revisado contra `src/` y los commits)

| Maqueta | Estado en el código | Fase |
|---|---|---|
| `mockups-estadisticas/final/` (Semana, Terminados, Panorama) | Nada. Hoy solo existen `rank`, `cal` y `year` (`store.svelte.ts:427`). | E1–E6 |
| `mockups-estadisticas/d-panorama.html` | Nada. Su contenido lo sustituye la maqueta final. Lo que solo sale aquí (vista «Todo» con el río del año, «Lo mejor de cada categoría», «¿Sabías que…?») queda como opcional. | E7 |
| `mockups-mi-lista/index.html` | El tablero tiene 5 franjas, con Terminado incluida (`milista.svelte.ts:143-149`). No hay selector de portadas ni estilos A–D. | L1–L3 |
| `plan-resumen-semanal.md` | Nada implementado (no existe `weekReview`). | R1, R3–R6 (su R2 lo absorbe E2/E3) |
| `mockups-menu-principal/` (retoques A–D) | Ninguno aplicado. Ya hecho por otro lado: «Últimos 7 días» en la cabecera (`Home.svelte:91`) y la pista Start = «Menú». | H1 |
| `mockups-colecciones/` (A–E) | Ninguna. Hoy: una estantería por colección con línea de tiempo (`CollTab.svelte:2-4`). | C1 |
| `mockups-animaciones/` 1, portada que viaja | **Hecho:** `morph` en `openTitle` (`controller.svelte.ts:311-369`), más `intro` escalonado (l.291). | — |
| 2, dirección y memoria | **Hecho:** `go(v, 'fwd'/'back')` (l.333) y foco recordado en `app.fk`. Falta solo el «pulso» al volver. | A2 (menor) |
| 3, foco que se desliza | Falta. El rebote en el borde ya existe: `bump` hace empujón, sonido `blip(220)` y vibración (l.226-237). | A4 |
| 4, mantener para confirmar y pistas vivas | Falta. Solo existe mantener el atajo del menú para forzar el cierre (l.1539). | A2 |
| 5, carga desde el HDD | A medias: hay fundido `thumbIn` de .28 s (`app.css:439-440`). Faltan el esqueleto y el orden escalonado. | A1 |
| 6, números que cuentan | Falta en Estadísticas. Se hace con la infraestructura de chips. | E2 |
| 7, celebraciones | Falta. | A3 |
| 8, fondo que sigue al juego | **Hecho:** atmósfera con fundido de 700 ms (`App.svelte:88-113`). Falta el parallax del hero. | A5 (opcional) |

---

## 2. Decisiones de diseño tomadas en este plan

1. **Un solo cálculo semanal.** `weekReview()` (fase R1 del plan del resumen) es la fuente de la pestaña **Semana** y de los chips semanales de **Panorama**: C (días esta semana), D (esta semana) y la cifra «+12 h esta semana» del chip A.
   - Cambio respecto a `plan-resumen-semanal.md`: los bloques salen **uno por categoría (`pi`)** y no por las fuentes `pc | switch | novels`. La maqueta final pinta una columna por categoría (Juegos PC, Switch, Novelas visuales, Novelas japonesas).
   - Cada bloque lleva `kind: 'play' | 'read'`, que sale de `groupOf` (`readgoals.ts:9`) cuando `p.id` está en `GOAL_KEYS`. Si no está, el bloque es `'play'` y no tiene meta.
   - Así se cierra también la duda 1 de ese plan («Otras» categorías): cada una tendrá su columna.
2. **La pestaña «Resumen semanal» del plan del resumen (su fase 2, `Week.svelte`) no se hace.** La sustituye la pestaña **Semana** de la maqueta final. Su fase 3 (aviso en Inicio) abre `statsTab = 'week'`.
3. **El Calendario no se toca:** ni `calendar()` (`stats.ts:210`) ni su rama en `Stats.svelte`. El mapa de calor de Panorama (chip I) usa una función nueva por categoría y no reutiliza `calendar()`.
4. **Botones:** se mantiene el contrato actual de Estadísticas.
   - Y (o clic en el segmentado) cambia de pestaña.
   - LB/RB cambian de periodo (semana, año…).
   - LT/RT cambian de categoría en Panorama, como ya hace la Clasificación (`trigger`, l.1034-1035).
   - La maqueta usa LB/RB para cambiar de pestaña. Esto queda en las decisiones abiertas (§7.1).
5. **Desplazamiento:** el panel de chips se mueve con `app.goff`, como la Clasificación (`controller.svelte.ts:179-182`).
   - Se calcula con el rectángulo del chip enfocado, sin alturas fijas.
   - Lleva la barra lateral y el aviso «N chips más abajo» de la maqueta (`final/index.html:661-676`).
6. **Acento:**
   - Los gráficos usan `--v1/--v2/--gv` y las cifras `--ink` o `--f1`, nunca `--v1` (guía §3.2 y §11). Un tono del arcoíris por chip, en orden de lectura (`final/index.html:700-703`).
   - En Semana y Terminados, cada categoría lleva su color fijo (función `tono`, l.573-575).
   - En Panorama manda el acento de la categoría elegida.
   - Lo que está «Terminado» va en `--done` (violeta), siempre con su rótulo.

---

## 3. Fases

### Estadísticas y resumen semanal

#### R1. Cálculo semanal compartido (M) · depende de: nada

Es la fase 1 de `docs/plan-resumen-semanal.md`, con estos cambios:

- `src/lib/stats.ts`, tras `yearReview` (~l.384-445):
  - `export interface WeekBlock { pi; name; kind: 'play'|'read'; ms; prevMs; n; games; perDay: number[7]; top: Array<{ti, ms}> (máx. 3); longest; milestones; started; finished; shots; shotPick; consoleMs?; consoleUntil?; goal?: {met, of: 7, perDay: number[7] (fracción 0..1), streak, rescued} }`.
  - `export interface WeekReview { range; prev; ms; prevMs; activeDays; blocks: WeekBlock[]; highlights: Highlight[] }`.
  - `highlights`: hitos, rachas, juegos terminados y récords de partida. Cada uno lleva su `pi` para la etiqueta (alimentan el chip Q).
  - `export const MILESTONES_H = [10, 25, 50, 100, 200]` y `export function nextMilestone(t: Title)`. Esta última la usa también Panorama (chip H).
  - Helper `shotInRange(s, from, to)` con el criterio de fecha local de `shotYear` (l.352).
- El resto (Switch con `console.days`, metas con `goalTrack`, comparación con la semana anterior) va como dice el plan del resumen.
- `tests/weekreview.test.ts` (nuevo): los casos del plan del resumen más uno nuevo, «una categoría sin meta da `goal` indefinido». **Apto para `agy`** si se quieren muchos casos; revisar su diff.
- **Comprobación:** `node --test tests/weekreview.test.ts tests/readgoals.test.ts` en verde. Con la semana pasada, el tiempo del bloque de `pc` coincide con Clasificación → Semana pasada.

#### E1. Pestañas nuevas y su estado (S) · depende de: nada (puede ir en paralelo con R1)

- `src/lib/store.svelte.ts:427`:
  - `statsTab: 'week' | 'fin' | 'pano' | 'rank' | 'cal' | 'year'`.
  - Estado nuevo: `weekOff = $state(0)`, `finYear = $state(año actual)`, `panoPi = $state(-1)` (−1 = la primera con datos) y `panoPer = $state<'all' | number | 'd30'>('all')`.
  - Valor inicial `'week'`, si se confirma la decisión §7.2.
- `src/lib/controller.svelte.ts`:
  - l.782-790, `toggleStatsTab`: ciclo según el orden de §7.2. Al cambiar de pestaña, `app.goff = 0`, como ahora.
  - l.793, `setStatsTab`: igual.
  - l.140-156, `defaultKey('stats')`: `'wk0'`, `'fn0'` y `'pn0'` (el primer chip de cada pestaña).
  - l.130-132, `validKey`: admitir `/^(wk|fn|pn)\d+$/` si el elemento existe en el DOM. Basta con `screenEl()?.querySelector('[data-k="…"]')`, como en otras vistas.
  - l.879-905, `statsKind` / `statsShift`: ramas para `week` (`weekOff`, límite `<= 0` y primera partida), `fin` (`finYear` dentro de los años con terminados) y `pano` (ciclo de `panoPer`).
  - l.1034-1036, `trigger`: en `pano`, LT/RT recorren las categorías con datos (copiar `cycleStatsPlat`).
  - l.2104, rueda: en `week`/`fin`/`pano`, vertical (`move(0, d)`).
- `src/screens/Stats.svelte`:
  - l.14, `VIEW_LABEL`: `week: 'Semana'`, `fin: 'Terminados'`, `pano: 'Panorama'`.
  - l.93-97: el segmentado recorre la lista de pestañas en el orden de §7.2.
  - l.67-92: cabecera de cada pestaña nueva con el formato de la maqueta: `#scope`, la píldora de categoría con ‹ › (solo en Panorama), y `#per`, el periodo con ‹ ›.
  - Cuerpo provisional: un aviso «En preparación» por pestaña. **No se publica versión** hasta que E2 esté hecha.
- **Comprobación:** Y y el clic recorren las 6 pestañas. Clasificación, Calendario y Resumen anual se comportan igual que antes, incluidos LB/RB, LT/RT y View.

#### E2. Infraestructura de chips y Semana, parte 1 (M) · depende de: R1, E1

- **`src/chips.css` (nuevo)**, importado en `src/main.ts:2` tras `app.css`:
  - Se copian de `docs/mockups-estadisticas/final/index.html:29-324` los bloques del chip base (`.st`, `.face`, `.x-k`, `.x-v`, `.x-s`) y de los chips A, B, C, D, F, G, H, I, J, K, L, M, N, R, más `.cols`, `.lanes`, `.yg`, `.det` y `.delta`.
  - Hay que quitar lo que es solo de la maqueta: `.tool`, `.name`, la letra `em` de `.x-k` y `#stage.m`.
  - La regla `.st:not(.run) *{animation:none!important}` se conserva: es la que repite la animación al enfocar.
  - **Apto para `agy`** (copia larga y mecánica). Revisar el diff contra la maqueta.
- **`src/lib/countup.ts` (nuevo):**
  - Acción Svelte `use:countUp={valor}` que sube el número en 800 ms con easeOutCubic (demo 6 de animaciones).
  - Sin animación si se cumple `calm()` (`controller.svelte.ts:278`; exportarlo) o si `!app.anim`.
  - Se anima una sola vez por visita a la pestaña: un `Set` en memoria de claves `pestaña:chip`, que se vacía al salir de `stats`.
- **`src/components/StatChip.svelte` (nuevo):**
  - Props: `k` (la `data-k`), `icon`, `label`, `wide?`, `tone?` (índice del arcoíris), `act?` (`data-act`, p. ej. `sr:pi:ti`).
  - Pinta `.glass.st.fx`, con `class:f` y `class:run`. `run` se reinicia al ganar el foco, como `lanza()` de la maqueta (l.624).
  - Slot por defecto para el cuerpo del chip.
- **`src/components/ChipPane.svelte` (nuevo):**
  - Contenedor con `transform: translateY(-goff)`, la barra lateral y el aviso «N chips más abajo» (lógica de `foco()`, l.661-676).
  - Se apoya en un ajuste de `goff` nuevo en `controller.svelte.ts:179-182`: si la pestaña es `week`/`fin`/`pano`, mide el chip enfocado y su título de sección `.sh` y ajusta `app.goff` (la regla de 260 px de la maqueta, l.667-669).
- **`src/screens/StatsWeek.svelte` (nuevo):**
  - Sección «La semana» con G (tiempo total frente a la anterior), R (siete anillos), F (racha) y K (abanico de capturas).
  - Datos: `weekReview(app.lib, app.weekOff, app.favs, app.playing, metas, app.now)` en un `$derived`.
  - R y F siguen la decisión §7.3.
  - K sigue la decisión §7.4: las capturas se toman de `shotPick` de cada bloque y nunca forman una lista mezclada.
- `Stats.svelte`: rama `{:else if app.statsTab === 'week'}<StatsWeek />` con sus pistas: «LB/RB Semana», «Y Pestaña», «B Volver».
- **Comprobación:**
  - Las flechas recorren los chips y la rueda baja.
  - Los contadores suben una vez; al volver a enfocar un chip, se repite su animación pero no la cuenta.
  - Con movimiento reducido no hay animaciones.
  - G coincide con la suma de Clasificación → Semana de cada categoría.

#### E3. Semana, parte 2: por categoría y destacados (M) · depende de: E2

- `StatsWeek.svelte`, sección «Por categoría» (`.cols`, maqueta l.579-592):
  - Una columna por `WeekBlock`, en el orden de la biblioteca, con el color fijo de la categoría (`tono`).
  - Columnas `'play'`: barras por día (`bars`, chip E). Columnas `'read'`: siete anillos de meta (`rings`).
  - Cada columna lleva su top (máx. 3, con `MiniCover`) y sus etiquetas: «X terminado» en `--done`, hito, capturas, «Meta n de 7» y «x h en la consola» en Switch.
  - Aviso «Datos de Nintendo hasta el …» si `consoleUntil` < domingo.
  - A sobre una portada del top abre la ficha (`data-act="sr:pi:ti"`, que ya maneja `activate`, l.608-611).
- Chip **Q** (destacados que rotan cada 3 s) al pie, con `highlights` y la etiqueta de categoría en cada uno. CSS de `chips/index.html` (~l.380-386).
- Novelas japonesas: caracteres, velocidad y `newKnown` con `readstats_get(null)` en un `$effect`, como en el plan del resumen (fase 2, paso 5).
- **Comprobación:**
  - Ninguna columna muestra títulos de otra categoría.
  - Los días de meta de `vn`/`jvn` coinciden con la pantalla Racha.
  - Los minutos de Switch = Clasificación + `console.days`.

#### R3. Aviso de semana nueva en Inicio (S) · depende de: E2 (mejor tras E3)

- Es la fase 3 de `plan-resumen-semanal.md`, sin cambios. A abre `app.view='stats'; app.statsTab='week'; app.weekOff=-1`.
- **No se hace en paralelo con H1:** las dos tocan `Home.svelte`.

#### E4. Terminados (M) · depende de: E2

- `src/lib/stats.ts`: `export function finishedYear(lib, year)`, que devuelve:
  - `lanes`: un carril por categoría con terminados, en el orden de la biblioteca, con `months: Array<GameRef & {finishedMs, ms}>[12]`.
  - `count`, `prevCount`, `last` (el más reciente, con su `pi`) y `years` (los años con algún terminado, para LB/RB).
  - Usa `t.status === 'finished' && t.finishedMs && !t.hidden`, el mismo criterio que `Year.svelte:26`.
  - Añadir un test pequeño a `tests/weekreview.test.ts` o un fichero propio.
- `src/screens/StatsFin.svelte` (nuevo):
  - M (estantería) **agrupada por categoría** (§7.5), con el desglose por color.
  - N: medalla del último terminado, con la etiqueta de su categoría.
  - G: comparación con el año anterior.
  - Debajo, «Mes a mes»: la rejilla `.yg` con un carril por categoría y la línea `.det` del juego enfocado.
  - Las portadas `.cvf` son `.fx` con `data-k="fn…"` y `data-act="sr:pi:ti"`. A = «Abrir juego».
- Periodo: los años de `years`. «Todo» no se hace (§7.6).
- **Comprobación:**
  - El total del año coincide con «n terminados este año» del Resumen anual (`Year.svelte:87`).
  - Las flechas recorren las portadas del carril, y ↑/↓ cambia de carril.

#### E5. Panorama, parte 1: cifras y récords (M) · depende de: E2

- `src/lib/stats.ts`, funciones puras de **una** categoría `pi`:
  - `catPeriod(lib, pi, per)`: tiempo, juegos y partida más larga en el periodo. Reutiliza `overlap` y `rankPeriod(…, pi)`.
  - `nextMilestone` por título (de R1): elige el juego de la categoría más cerca de su próximo umbral.
- `src/screens/StatsPano.svelte` (nuevo):
  - «Lo esencial»: A (odómetro del tiempo del periodo, con «+x h esta semana · n juegos» de `weekReview`), B, C (días con actividad esta semana, del bloque de `weekReview`) y D (esta semana frente a la anterior, con la línea de los últimos 8 periodos semanales).
  - B (anillo de la meta de hoy) solo existe si la categoría tiene meta: `goalMin(app.config, key) > 0` con `key = p.id` en `GOAL_KEYS`. Si no, no se pinta.
  - «Récords e hitos»: L (partida más larga del periodo, con portada y día; A abre el juego) y H (próximo hito).
  - El acento es el de la categoría (el `--acc`/`--accv` de la plataforma, como en el resto de pantallas de la categoría).
  - La pista de abajo dice «Solo títulos de {categoría}».
- **Comprobación:**
  - A con «Siempre» = Clasificación → Siempre de esa categoría.
  - LT/RT cambia de categoría y todo se recalcula, sin restos de la anterior.

#### E6. Panorama, parte 2: capturas, reloj y mapa de calor (M) · depende de: E5

- `src/lib/stats.ts`:
  - `hourHistogram(lib, pi, range)`: 24 cubos con el reparto de `overlap` por hora local.
  - `dayHeat(lib, pi, year)`: ms por día del año, con `eachDay`.
  - `calendar()` no se toca (§2.3).
- `StatsPano.svelte`:
  - K: capturas de la categoría en el periodo, con un abanico de 3 (la favorita más reciente primero).
  - J: reloj de 24 h con la franja más jugada.
  - I: «El año, día a día», el mapa de calor en ola de la categoría.
  - Miniaturas siempre desde la caché (`Thumb`), nunca el original de P:.
- Rendimiento: medir `hourHistogram` + `dayHeat` con la biblioteca real (`console.time`), con un objetivo de menos de 20 ms. Si se pasa, memorizar por `pi` + `lib`.
- **Comprobación:** la suma de los 24 cubos de J = tiempo de A en el mismo periodo.

#### E7. Panorama extra de `d-panorama.html` (S–M, opcional) · depende de: E6

- Vista «Todo» en el ámbito: «El año en un río» (una franja por categoría) y «Lo mejor de cada categoría» (un récord por categoría, con su etiqueta).
- «¿Sabías que…?»: datos curiosos que rotan.
- Solo si el usuario lo pide (§7.7).

#### R4. Datos que faltan, en Rust (S–M) · depende de: E3

- Es la fase 4 de `plan-resumen-semanal.md`, sin cambios: `phrases` en `DayPoint` y `week_achievements`.
- En E3 se añaden la etiqueta «n logros · el más raro» por columna y el destacado «logro raro» en Q.
- **Comprobación:** las del plan del resumen (`cargo test` en `visor-core` y la llamada desde la tablet).

#### R5 y R6. Tarjeta PNG y notificación del lunes (opcionales)

Son las fases 5 y 6 de `plan-resumen-semanal.md`. Van al final y solo si el usuario las confirma (decisiones 3 y 4 de ese plan).

### Mi lista

#### L1. Sin la franja Terminado (S) · depende de: E4 (para que los terminados sigan teniendo un sitio en la misma versión)

- `src/lib/milista.svelte.ts`:
  - l.123: `BandKind` sin `'done'`.
  - l.143-149: `BAND_DEF` sin Terminado.
  - l.156-188: `buildBoard` deja fuera `finished` (`if (st === 'finished') return;`), quita `by.done`, su orden y `yearStart`, y `compact` pasa a ser siempre `false`.
  - Mantener `year`/`yearStart` en `BoardItem` **solo si** los usa la pestaña `'fin'` (l.289). Comprobarlo con grep antes de quitarlos.
- `src/screens/MiListaDual.svelte`:
  - l.14 y l.21-22: quitar los casos `done`.
  - l.85-88, l.107 y l.117: quitar las ramas `band.key === 'done'`.
  - l.204-205: quitar `.st-done`.
  - `rowsOf` (l.18-39) reparte ahora el alto entre 4 carriles.
- `src/lib/mock.ts:261`: ajustar el comentario.
- **Comprobación:**
  - Con el mock: 4 franjas. Una vacía se pliega a 30 px.
  - ↑/↓ no cae en franjas vacías.
  - Los terminados se ven en Estadísticas → Terminados y en el Catálogo.

#### L2. Selector de portadas: hoy, grandes y máximas (M) · depende de: L1

- Ajuste nuevo `listCovers: 'normal' | 'big' | 'max'`:
  - `src-tauri/visor-core/src/config.rs`: campo, valor por defecto `"normal"` (junto a l.167/272) y validación (como l.316-317). Ampliar el test de l.431-434.
  - `src/lib/types.ts`: `Config`.
  - `src/lib/mock.ts:321`: valor por defecto.
  - `src/screens/Settings.svelte`: fila «Portadas de Mi lista», junto a las de la lista.
  - `controller.svelte.ts` ~l.1633: rama `listcovers` con `cycleIn`, igual que `liststalledact`.
  - Opcional: en el menú Start de la lista (`listopts`, l.1113), junto a `tsize`.
- `MiListaDual.svelte`:
  - Clases `pg`/`pm` en `.board` según el ajuste, con el CSS de `mockups-mi-lista/index.html:156-170`, adaptado a 4 franjas y sin `.done`.
  - `rowsOf`: con `big`/`max`, cada franja con juegos ocupa una fila de alto variable (`flex:1`) y las vacías se quedan en 30 px. El cálculo de filas extra solo vale para `normal`.
- «Máximas» sube las pestañas a la barra superior (`.sx` en `top:16px`, maqueta l.167). Las pestañas las pinta `src/screens/Titles.svelte`. Añadir allí una clase condicionada a `app.titlesTab === 'list' && app.config.listCovers === 'max'` y comprobar que no pisa la miga ni la hora.
- **Comprobación:**
  - Los tres modos a 1280×720 y en la tablet: deslizar por los carriles sigue funcionando (commit `0620008`).
  - La portada enfocada se mantiene a la vista (`$effect`, l.51-64).

#### L3. Estilo elegido (M) · depende de: L1. Va después de L2 o antes, pero no en paralelo, porque las dos tocan `MiListaDual.svelte`

- Aplicar la variante que elija el usuario (§7.8). La maqueta propone A, «Chips», por defecto.
- Se copia su CSS (bloques `.v-a`, `.v-b`, `.v-c` o `.v-d` de la maqueta, ~l.40-155) al `<style>` de `MiListaDual.svelte`, sustituyendo el estilo actual del lomo.
- Los colores vienen de `BAND_DEF` (`--c`). Los gráficos usan `--v1` y el texto `--ink` (guía §3.2).
- Si el usuario quiere elegir el estilo en Ajustes y no fijar uno, es otro ajuste más, como en L2. Duplica las pruebas visuales; no lo recomiendo.
- **Comprobación:** los 7 tonos del arcoíris, en claro y oscuro. Ningún texto sobre `--v1`. Foco visible en los cuatro carriles.

### Inicio, Colecciones y animaciones

#### H1. Retoque del menú principal (S) · depende de: la elección §7.9. No en paralelo con R3 (`Home.svelte`)

- Pasar a CSS real el bloque de la variante elegida (`mockups-menu-principal/index.html:42-114`). Sitios:
  - `src/app.css`: `.ghost` l.24, `.bokeh` l.26, `.hsect` l.451, `.hdock` l.469, `.plat` y `.plabel`.
  - `src/screens/Home.svelte`: l.91-126.
- La nota «“Últimos 7 días” como etiqueta» de A/B/C ya está resuelta (`Home.svelte:91`): no se rehace.
- Solo cambia el acabado: ni posiciones ni categorías.

#### C1. Colecciones (S–L según la elección) · depende de: la elección §7.10

- Las maquetas A–E no tienen ganadora. Tamaños, según su propia tabla (`mockups-colecciones/index.html:28-33`): A pequeño, B/C/E medio, D grande.
- Toca `src/screens/CollTab.svelte` y `src/lib/coleccion.svelte.ts` (foco y navegación).
- Las colecciones siguen dentro de su categoría.
- Escribir el plan detallado cuando haya elección. No se planifica a ciegas.

#### A1. Miniaturas del HDD: esqueleto y fundido en orden (S) · depende de: nada

- `src/components/Thumb.svelte:13` y l.33, más `app.css:439-440`.
- Mientras `src` no está, se pinta un hueco con el brillo de carga.
- Al llegar, la miniatura hace un fundido de 300 ms con un retraso de `índice × 30 ms`, calculado sobre el orden de la rejilla y no sobre el de llegada.
- Sin animación si `calm()`.

#### A2. Mantener para confirmar, pistas vivas y pulso al volver (S–M) · depende de: nada

- Mantener 600 ms con un anillo que se llena para las acciones serias que hoy piden dos pulsaciones. Antes, listar con grep dónde está hoy la doble pulsación: cerrar el juego en `src/quickmenu/TileExit.svelte`, suspender en `StatusBar` y borrar.
- `src/components/Hints.svelte`: la pista se hunde y brilla al pulsar su botón.
- Al volver con `go(…, 'back')`, un pulso en el elemento que recupera el foco.

#### A3. Celebraciones con medida (M) · depende de: nada

- Sello «Terminado» con rebote, confeti arcoíris corto y tres notas (`src/lib/sound.ts`) al guardar Terminado (`milista.svelte.ts:540-553`).
- Logro raro: tarjeta que baja con un brillo.
- Solo para lo que pasa poco. Respetar `calm()` y `app.config.sound`.

#### A4. Foco que se desliza (M, opcional) · depende de: nada

- Hoy el foco es la clase `.f` de cada tarjeta. Un resaltado que se desliza obliga a añadir un elemento único que siga al enfocado por pantalla, empezando por los carriles de Mi lista.
- Riesgo de parpadeo y de coste en la tablet. Hacer primero un spike en Mi lista.

#### A5. Parallax leve del hero (S, opcional)

- `src/components/HeroStrip.svelte`. Desactivado en la tablet y con `calm()`.

---

## 4. Orden recomendado y motivo

1. **R1 + E1** (en paralelo): son la base. R1 es pura y se prueba sola; E1 es solo navegación.
2. **E2 → E3:** la pestaña Semana es lo más pedido y estrena la infraestructura de chips que reutilizan E4–E6.
3. **E4 → L1:** Terminados primero, para que quitar la franja de Mi lista no deje los terminados sin sitio dentro de la misma versión.
4. **R3:** el aviso del lunes necesita la pestaña Semana terminada.
5. **L2, L3:** Mi lista; independiente de Estadísticas salvo por L1.
6. **E5 → E6:** Panorama es lo más grande y lo que menos urge.
7. **R4:** completa Semana con logros y frases.
8. **H1, A1, A2:** retoques pequeños sin dependencias. Sirven de relleno entre las fases grandes.
9. **C1, A3, A4, E7, R5, R6:** necesitan una decisión o son opcionales.

## 5. Paralelismo

- **Pistas independientes:**
  - Estadísticas: `stats.ts`, `Stats*.svelte`, `chips.css`.
  - Mi lista: `milista.svelte.ts`, `MiListaDual.svelte`, `config.rs`.
  - Inicio: `Home.svelte`.
  - Miniaturas: `Thumb.svelte`.
- **Ficheros calientes:** `controller.svelte.ts`, `store.svelte.ts` y `app.css`. Dos fases que los toquen no van a la vez, salvo en ramas distintas con un rebase cuidadoso. Si se usan worktrees temporales, que no compartan el `target` de cargo (L2 compila Rust).
- **Se pueden hacer a la vez:**
  - R1 con E1.
  - L2 o L3 con cualquier fase de Estadísticas posterior a E2.
  - A1 y H1 con todo, salvo H1 con R3.
- **Para `agy`:** los tests de R1 y E4, y el traslado del CSS de chips de E2. Nada más: el resto son tareas pequeñas o de criterio.

## 6. Versiones objetivo

| Versión | Contenido | Por qué |
|---|---|---|
| **2.1.0** | R1, E1, E2, E3, E4, L1, R3 | Estadísticas nuevas usables (Semana y Terminados), Mi lista sin Terminado y el aviso del lunes. Es la entrega coherente mínima. |
| **2.2.0** | L2, L3, E5, E6, R4, H1, A1, A2 | Panorama completo, portadas y estilo de Mi lista, logros de la semana y retoques. |
| **2.3.0** | C1, A3, más los opcionales confirmados (A4, A5, E7, R5, R6) | Necesitan elección o son de baja prioridad. |

La subida de versión se hace como en `0a50e7e` («chore: subir version a 2.0.0»): mismos ficheros y mismo mensaje.

## 6bis. Decisiones del usuario (2026-11-11)

1. Botones de Estadísticas: se mantiene el contrato actual (Y pestaña, LB/RB periodo, LT/RT categoría).
2. Pestañas: siguen Clasificación y Resumen anual. Orden recomendado: Semana · Terminados · Panorama · Clasificación · Calendario · Resumen anual.
3. Chips R y F de Semana: salen de las metas de lectura de **Novelas visuales** (`vn`), no de toda la biblioteca.
4. Mi lista: el estilo es el que ya hay en `MiListaDual.svelte` en GitHub (maqueta C ajustada, 5 carriles adaptativos). **L3 no se hace**; L2 (portadas grandes y máximas) sigue.
5. Chip K (abanico de capturas): sí, una destacada por categoría (máx. 3).
6. Chip M (estantería de terminados): agrupada por categoría, cada grupo con su color.
7. Terminados → «Todo»: **no se hace**, solo años. Apuntado como idea futura.
8. Extras de `d-panorama` (vista «Todo», río del año, «¿Sabías que…?»): sí, E7 pasa a la 2.2.0.
9. Menú principal: pendiente de elegir entre A (Pulido), B (Barra inferior), C (Estante) y D (Foco claro).
10. Colecciones: aparcado (C1 fuera de plan por ahora).

## 7. Decisiones abiertas para el usuario

1. **Botones en Estadísticas:** ¿LB/RB cambian de pestaña, como en la maqueta, o se mantiene lo de hoy (Y cambia de pestaña, LB/RB de periodo, LT/RT de categoría)? *Recomiendo mantener lo de hoy: es el contrato C ya publicado.*
2. **Pestañas y orden:** la maqueta solo enseña Semana · Terminados · Panorama, y el Calendario se queda.
   - ¿Siguen la Clasificación y el Resumen anual?
   - *Recomiendo* Semana · Terminados · Panorama · Clasificación · Calendario · Resumen anual, con Semana al entrar.
   - Si Panorama sustituye a la Clasificación, se quitaría en otra fase.
3. **Chips R y F de Semana (toda la biblioteca):** las metas son de cada categoría. *Recomiendo:* R = día cumplido si se cumplió al menos una meta activa; F = la racha más larga de las metas activas, igual que el avatar. El detalle de cada meta va en su columna.
4. **Chip K de Semana:** ¿abanico con capturas de categorías distintas? *Recomiendo* una destacada por categoría (máx. 3), sin nombres de juego en el chip.
5. **Chip M (estantería de terminados):** en la maqueta mezcla portadas de todas las categorías por fecha, y eso choca con la regla de `CLAUDE.md`. *Recomiendo* agruparlas por categoría, cada grupo con su color.
6. **Terminados → «Todo»** (el periodo que sale en la maqueta sin diseño): *recomiendo* no hacerlo por ahora y dejar solo los años.
7. **Extras de `d-panorama.html`** (vista «Todo», río del año, «¿Sabías que…?»): ¿se quieren? *Recomiendo* decidirlo tras usar E6.
8. **Mi lista:**
   - ¿Qué estilo, A, B, C o D? ¿Uno fijo o elegible en Ajustes? *Recomiendo* A fijo.
   - El brief dice «Jugando y Abandonados», pero la maqueta y el código usan En marcha / A medias / Pendiente / Aparcado, y Aparcado junta los abandonados (`dropped`) con los «jugando» parados más de 90 días. ¿Se cambian los nombres, o se separa una franja «Abandonado» propia? No lo he planificado hasta saberlo.
9. **Menú principal:** ¿A, B, C o D? *Recomiendo* A (el más conservador).
10. **Colecciones:** ¿qué propuesta, o qué combinación (p. ej. A como vista general y C al abrir)?

## 8. Riesgos

- **Rendimiento en la tablet:** decenas de chips animados a la vez. En la tablet, solo se anima el chip enfocado y los contadores se ponen directamente en su valor, como ya se hace con la atmósfera (`App.svelte:90-91`).
- **Dos criterios de día** (medianoche para los totales, 7:00 para las metas): ya está asumido en el plan del resumen. Se ve en R/F frente a G.
- **`console.days` solo cubre 28 días:** al ir más de 3 semanas atrás, la columna de Switch dice «sin datos de consola», no 0.
- **Quitar Terminado de Mi lista antes de E4** dejaría los terminados accesibles solo por el Catálogo. Por eso L1 depende de E4.
- **«Máximas» mueve las pestañas de `Titles.svelte`:** puede chocar con la miga y la hora a 1280×720 y en la tablet. Hay que revisarlo a mano.

## 9. Resumen

| Fase | Tamaño | Depende de | Versión |
|---|---|---|---|
| R1 Cálculo semanal | M | — | 2.1.0 |
| E1 Pestañas y estado | S | — | 2.1.0 |
| E2 Chips y Semana (1) | M | R1, E1 | 2.1.0 |
| E3 Semana (2): por categoría y Q | M | E2 | 2.1.0 |
| E4 Terminados | M | E2 | 2.1.0 |
| L1 Mi lista sin Terminado | S | E4 | 2.1.0 |
| R3 Aviso en Inicio | S | E2 (mejor E3) | 2.1.0 |
| L2 Portadas grandes y máximas | M | L1 | 2.2.0 |
| L3 Estilo de Mi lista | M | L1, §7.8 | 2.2.0 |
| E5 Panorama (1) | M | E2 | 2.2.0 |
| E6 Panorama (2) | M | E5 | 2.2.0 |
| R4 Logros y frases (Rust) | S–M | E3 | 2.2.0 |
| H1 Menú principal | S | §7.9; no con R3 | 2.2.0 |
| A1 Esqueleto de miniaturas | S | — | 2.2.0 |
| A2 Mantener y pistas vivas | S–M | — | 2.2.0 |
| C1 Colecciones | S–L | §7.10 | 2.3.0 |
| A3 Celebraciones | M | — | 2.3.0 |
| A4 Foco deslizante | M | spike | opcional |
| A5 Parallax del hero | S | — | opcional |
| E7 Extras de Panorama | S–M | E6, §7.7 | opcional |
| R5 Tarjeta PNG | M | E3 | opcional |
| R6 Notificación del lunes | S | R3 | opcional |
