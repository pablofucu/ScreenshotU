# Plan: metas diarias «Juegos de PC» y «Switch»

Dos metas nuevas, `pc` y `switch` (ids de plataforma que ya existen, `src/lib/store.svelte.ts:23-24`), que funcionan exactamente igual que `vn`/`jvn`: anillo, racha, mapa, rescate, toasts, menú rápido, aviso sobre el juego y ajustes. El tiempo se cuenta como ahora (`eachSpan`, `src/lib/readgoals.ts:156`: `plays` de los títulos de esa plataforma más la partida en curso). **Las categorías no se tocan**: ni nombres ni cómo se detectan. Solo se leen sus ids.

## Decisiones de diseño (tomadas aquí)

- **Valor por defecto: 60 min para las dos, y 0 = «Desactivada» solo en las metas de juego.** Con 60 la función se ve en cuanto se actualiza la app (el usuario la ha pedido) y coincide con el defecto de `goalVn`. Con 0 se pueden apagar, y entonces la cara «Juego de hoy» desaparece (la ficha queda fija). `vn`/`jvn` siguen en 5..600, sin «desactivada», para no cambiar su comportamiento.
- **Una sola fuente de «metas activas»**: `app.goalKeys` (plataforma presente y meta > 0), en el orden `['vn','jvn','pc','switch']`. La usan la ficha, la racha, el controlador, el contexto del menú rápido y el aviso de meta. Así se quitan los `k === 'vn' ? goalVn : goalJvn` repartidos por el código.
- **Grupos (caras)**: `read = ['vn','jvn']` y `play = ['pc','switch']`. Una cara existe si tiene al menos una meta activa.
- **Ficha de Inicio**: un componente nuevo, `ReadCard.svelte`, que solo es el envoltorio de las dos caras. Monta **dos `ReadRings`** (no se duplica nada) con la prop `keys`. El título de la sección, la línea `rmeta`, los puntitos y el anillo pequeño también salen de `ReadCard`, para que Home no repita el bloque dos veces (hoy está copiado en las líneas 107-111 y 114-118).
- **Puntitos y anillo pequeño, en la línea del título** («Lectura de hoy …», top 378), alineados con el borde derecho de la ficha. Dentro de la ficha, la esquina de arriba a la derecha la ocupa la medalla de la segunda columna (ficha de 150 px de alto, contenido de unos 126 px). Ver «Dudas».
- **Solo LB/RB cambian de cara.** ←/→ no: la ficha es la última de la fila «Seguir jugando», y ← tiene que seguir yendo a los recientes.
- **Rotación**: cada 12 s, con fundido de opacidad (.45 s). Se pausa con `app.focus === 'read'`, se reinicia con cualquier `cmd()` y no rota si una cara está «fijada». Con `prefers-reduced-motion`, cambio sin transición.
- **Urgencia por cara** (para fijarla y para la cara inicial). Nivel = el máximo de sus pistas:
  - 3: `rr.pending(t)` (meta cumplida hoy y racha sin encender);
  - 2: `t.rescue`;
  - 1: `!t.done && t.streak > 0` (racha en peligro hoy);
  - 0: nada.

  Se fija la cara con el nivel **estrictamente mayor**. Si empatan, se rota. Si el usuario cambia a mano a la cara no fijada, al acabar el ciclo vuelve a la fijada.
- **Cara inicial al montar Inicio**: la fijada; si no hay, la del grupo con la partida más reciente (máximo `b` de `plays` de sus plataformas, o `now` si `app.playing` es de ese grupo).
- **Encendido de racha en la cara oculta**: `ReadRings` recibe la prop `active`. Solo programa `light()` mientras está activa; al pasar a activa espera al fundido (≥ 450 ms). Si deja de estar activa antes de que salte el temporizador, se cancela y se quita del `scheduled` para volver a programarse. Como «pendiente de encender» es urgencia 3, la ficha se va a esa cara sola, y la animación se ve.
- **Pantalla de racha**: una pestaña por meta activa (hasta 4). Rings, leyenda y medallas muestran todas las metas activas. Con más de 2 se usa un modo compacto. Las pestañas son clicables con `data-hact="goal:<k>"` (adiós al truco `lb`/`rb` por posición).
- **Textos según el grupo** (`GOAL_TXT[group]`): «leído/lees/novelas/lectura» frente a «jugado/juegas/juegos/juego».

---

## Pasos (la app compila tras cada uno; un commit por paso)

> Antes de cada commit: finales de línea mezclados (ver memoria). Comprobad los CR del fichero antes y después; no normalicéis.
> Comprobación rápida tras cada paso: `npm run check 2>&1 | tail -5`, y en Rust `cargo check -p visor-core 2>&1 | tail -5` (no compartáis `target` con worktrees temporales).

### Paso 1. Config nueva (Rust + TS), sin uso todavía

1. `src-tauri/visor-core/src/config.rs:132-134`: añadir `pub goal_pc: u32, pub goal_switch: u32` (comentario: «metas diarias de juego; 0 = desactivada»).
   - `:249-250` (Default): `goal_pc: 60, goal_switch: 60`.
   - `:292-293`: `if c.goal_pc > 0 { c.goal_pc = c.goal_pc.clamp(5, 600); }`, y lo mismo para `goal_switch`. El struct ya es `#[serde(default)]` (línea 48), así que las configs viejas toman 60.
2. `src-tauri/visor-core/src/remote.rs:56-66`: añadir `goal_pc`, `goal_switch` a `RemoteConfig` y a `from_config`. En el test `:1134`, `goal_pc: 60, goal_switch: 60`.
3. `src-tauri/src/quickmenu.rs:961`: `"goals": { "vn": c.goal_vn, "jvn": c.goal_jvn, "pc": c.goal_pc, "switch": c.goal_switch },`.
4. `src/lib/types.ts:86-87`: `goalPc: number; goalSwitch: number;`.
5. `src/lib/store.svelte.ts:63-64`: `goalPc: 60, goalSwitch: 60`.
6. `src/lib/mock.ts:325`: añadir `goalPc: 60, goalSwitch: 30` tras `goalJvn: 30` (un valor distinto para probar).
7. `mobile/src/lib/source.ts:5` (interfaz `RemoteConfig`) y `mobile/src/lib/store.svelte.ts:32` (`DEFAULT_CFG`): añadir los dos campos.
8. `docs/api-movil.md:44`: añadir `"goalPc": 60, "goalSwitch": 60` al ejemplo.

Comprobar: `cargo test -p visor-core remote 2>&1 | tail -5`, `npm run check`.

### Paso 2. Modelo en `readgoals.ts` y tipos que dependen de `GoalKey`

1. `src/lib/readgoals.ts:5-17`:
   ```ts
   export type GoalKey = 'vn' | 'jvn' | 'pc' | 'switch';
   export const GOAL_KEYS: GoalKey[] = ['vn', 'jvn', 'pc', 'switch'];
   export type GoalGroup = 'read' | 'play';
   export const GOAL_GROUPS: Record<GoalGroup, GoalKey[]> = { read: ['vn', 'jvn'], play: ['pc', 'switch'] };
   export const groupOf = (k: GoalKey): GoalGroup => (k === 'pc' || k === 'switch' ? 'play' : 'read');
   const GOAL_CFG = { vn: 'goalVn', jvn: 'goalJvn', pc: 'goalPc', switch: 'goalSwitch' } as const;
   export const goalMin = (cfg: Pick<Config, 'goalVn' | 'goalJvn' | 'goalPc' | 'goalSwitch'>, k: GoalKey) => cfg[GOAL_CFG[k]] ?? 0;
   export const goalCfgKey = (k: GoalKey) => GOAL_CFG[k];
   ```
   - `Config` se importa como tipo desde `./types`.
   - `GOAL_COL` gana el campo `tab` (etiqueta corta de pestaña): vn `'Novelas'`, jvn `'Japonés'`.
     - `pc: { c: '#1DB37A', soft: '#cdeedd', ink: '#11804f', edge: '#14875a', name: 'PC', full: 'Juegos de PC', tab: 'PC' }`
     - `switch: { c: '#F25C2A', soft: '#fbd9cb', ink: '#b33a12', edge: '#c0431a', name: 'Switch', full: 'Switch', tab: 'Switch' }`

     `ink` y `edge` siguen el patrón de los que ya hay: tono del color oscurecido ~30 % para el texto y ~25 % para el borde de la racha. Hay que comprobar el contraste en pantalla.
   - `RESCUE_COL`:
     - `pc: { rc: '#B44FD6', rsoft: '#f1e0f8', rink: '#7a2a99', rmid: '#dfbdeb' }`
     - `switch: { rc: '#8DB92E', rsoft: '#eaf3d3', rink: '#557514', rmid: '#cfe2a3' }`
   - `rescueCls`: `({ vn: 'r-vn', jvn: 'r-jv', pc: 'r-pc', switch: 'r-sw' } as const)[k]`.
   - Textos por grupo, nuevo `GOAL_TXT: Record<GoalGroup, {...}>`:
     - `read`: `{ card: 'Lectura de hoy', crumb: 'Racha de lectura', what: 'Qué has leído este mes', when: 'A qué hora lees', none30: 'Sin lecturas en los últimos 30 días', titles: 'Novelas', notice1: '¡Meta de lectura cumplida!', sub1: 'Hoy cuenta para tu racha de lectura' }`
     - `play`: `{ card: 'Juego de hoy', crumb: 'Racha de juego', what: 'Qué has jugado este mes', when: 'A qué hora juegas', none30: 'Sin partidas en los últimos 30 días', titles: 'Juegos', notice1: '¡Meta de juego cumplida!', sub1: 'Hoy cuenta para tu racha de juego' }`
   - Helper nuevo, cerca de `eachSpan`: `export function lastPlayAt(keys: GoalKey[], platforms: Platform[], live: Live, now: number): number`. Devuelve el máximo `b` de `plays` de los títulos de esas plataformas, o `now` si `live` es de un título suyo; 0 si no hay nada.
   - Actualizar el comentario de cabecera (`:1-2`).
2. `src/app.css:832-833`: añadir
   `.r-pc{--rc:#B44FD6;--rsoft:#f1e0f8;--rink:#7a2a99;--rmid:#dfbdeb}` y `.r-sw{--rc:#8DB92E;--rsoft:#eaf3d3;--rink:#557514;--rmid:#cfe2a3}`.
3. `src/quickmenu/quickmenu.css:1329-1337`: añadir `.r-pc` y `.r-sw` con `--rc` y `--rmid` (mismo formato).
4. `src/quickmenu/data.ts:20` → `key: GoalKey | null;` (importar el tipo). `:25` → `goals: Record<GoalKey, number>;`. Los mocks `:187, :230, :278, :328` → `{ vn: 60, jvn: 45, pc: 60, switch: 30 }`.
5. `src/quickmenu/QuickMenu.svelte:535` y `:672`, y `src/quickmenu/TileTime.svelte:14`: el mismo objeto por defecto con las 4 claves.
6. `mobile/src/components/GoalRings.svelte:10`: el móvil queda fuera del alcance. Para que compile, tipar `COL` como `Partial<Record<GoalKey, …>>` y el array de `:19` como `GoalKey[]` filtrado a `k in COL`. Solo hay que tocarlo si el compilador se queja de `Record<GoalKey>` incompleto, y se queja.

Comprobar: `npm run check` sin errores nuevos. En pantalla no cambia nada.

### Paso 3. Store y controlador: metas activas en todo el flujo

1. `src/lib/store.svelte.ts`:
   - Junto a `readCard` (`:559`): añadir
     - `goalKeys = $derived(GOAL_KEYS.filter((k) => this.lib.platforms.some((p) => p.id === k) && goalMin(this.config, k) > 0));`
     - `goalFaces = $derived((['read', 'play'] as GoalGroup[]).filter((g) => this.goalKeys.some((k) => groupOf(k) === g)));`
   - `readCard` (`:559`) → `!!this.config.root && this.goalKeys.length > 0`.
   - Cerca de `streakKey` (`:432`): `readFace = $state<GoalGroup>('read');` (cara visible de la ficha de Inicio) e `inputAt = $state(0);` (última entrada; reinicia el ciclo de la ficha).
   - `:654` (`streakKey`/`hueOf`) ya es genérico con `GOAL_COL`, no hay que cambiarlo. Solo comprobad que el import de `:8` sigue valiendo.
2. `src/lib/controller.svelte.ts`:
   - `:639-645`: borrar `GOAL_KEYS` y `goalOf`. `goalKeys = () => app.goalKeys`. `goalOf = (k) => goalMin(app.config, k)`. `goalKeyOf` busca en `app.goalKeys`, de modo que un juego de PC con meta 0 no manda `key` al menú rápido ni programa aviso.
   - `:703`: `const text = n > 1 ? '¡Meta cumplida!' : GOAL_TXT[groupOf(k)].notice1;`. `:680`, comentario: «Al empezar un juego con meta».
   - `:590-591`: `openStreak(app.goalKeys.find((k) => groupOf(k) === app.readFace));`.
   - `:648-662` (`openStreak`/`streakSwitch`): ya van por índice sobre `goalKeys()` y valen con 4. Añadir `export function pickStreak(k: GoalKey)` (si está en `goalKeys()` y no es el actual: `app.streakKey = k; blip(700)`).
   - `cmd()` (`:1755`): justo tras `wake();`, poner `app.inputAt = Date.now();`. Antes del `switch`, añadir
     ```ts
     if (c.startsWith('goal:')) { pickStreak(c.slice(5) as GoalKey); return; }
     if (c.startsWith('face:')) { setReadFace(c.slice(5) as GoalGroup); return; }
     ```
   - `page()` (`:905`): primera línea, `if (app.view === 'home') { if (app.focus === 'read') stepReadFace(d); return; }`.
   - Funciones nuevas junto a la racha:
     - `setReadFace(g)`: si `g` está en `app.goalFaces` y no es la actual, `app.readFace = g; blip(700)`.
     - `stepReadFace(d)`: si `goalFaces.length < 2`, `bump(screenEl()?.querySelector('.fx.f'), d, 0)`; si no, la otra cara.
3. `src/rec/main.ts:19-22`: `subtitle(title, days, key)` usa `GOAL_TXT[groupOf(key)].sub1` en el caso `days === 1`. `:37` ya cae en `GOAL_COL[key]`, que ahora tiene `pc`/`switch`. Pasar `key` en la llamada a `subtitle`. Comentario de `:2`: «meta diaria cumplida».
4. `src/quickmenu/TileTime.svelte:44`: `isReading` → `const hasGoal = $derived(!!context?.key && context.key in GOAL_COL);` (renombrar en `:49, :55, :130-132, :142, :144, :160, :285`). `:130-131`: sustituir los dos `class:` por `class={hasGoal ? rescueCls(goalKey) : ''}` dentro del `class` existente (importar `rescueCls`). `:142` «Tiempo y racha» vale para los dos grupos. Comentario de `:2`.
5. `src/lib/reading.svelte.ts` (Ajustes):
   - `:15`: añadir `'goalpc' | 'goalswitch'`. `:19`: `'goalvn', 'goaljvn', 'goalpc', 'goalswitch',`. `:26`: añadirlas a `READING_CYCLE`.
   - `:29-34`: `nextGoal(cur, dir, allowOff = false)` usa `allowOff ? [0, ...GOALS] : GOALS`. `goalLabel(0)` → `'Desactivada'`.
   - `:70-73`: dos `case` nuevos.
     - `goalpc`: `l: 'Meta diaria · juegos de PC'`, `d: 'Minutos al día de juegos de PC para cerrar el anillo verde de «Juego de hoy» en Inicio. Cada día que lo cierras suma a la racha. Desactivada: sin anillo. ‹ › la cambia.'`
     - `goalswitch`: igual, con «Switch» y «anillo naranja».
   - `:165-169`: generalizar con un mapa `{ goalvn: 'vn', goaljvn: 'jvn', goalpc: 'pc', goalswitch: 'switch' }` → `goalCfgKey(k)`, y `allowOff = groupOf(k) === 'play'`.
   - En la tablet salen solas: `TABLET_HIDDEN_READING` no las incluye (`controller.svelte.ts:1258`).

Comprobar:
- `npm run dev` con el mock: Ajustes → Biblioteca muestra 4 filas de meta, y ‹ › en PC llega a «Desactivada».
- La pantalla de racha ya enseña 4 pistas, con la maqueta aún sin ajustar (se arregla en el paso 5).
- Menú rápido (mock `data.ts`) con `key: 'pc'`: baldosa verde con racha.

### Paso 4. `createReadRings` y `ReadRings` parametrizados

1. `src/lib/readrings.svelte.ts`:
   - `ReadRingsOpts` (`:12-22`): añadir `keys?: () => GoalKey[]` (por defecto `() => app.goalKeys`) y `strokeFor?: (n: number) => { stroke: number; gap: number }` (por defecto, los fijos).
   - `:42`: `const goalOf = (k: GoalKey) => goalMin(app.config, k);`.
   - `:43-47`: `tracks = $derived((o.keys?.() ?? app.goalKeys).filter((k) => app.goalKeys.includes(k)).map((k) => goalTrack(k, goalOf(k), app.lib.platforms, app.playing, now, o.weeks)))`.
   - `:48-51`: `rings` usa `const { stroke, gap } = o.strokeFor?.(tracks.length) ?? o;`. Exponer `get stroke()` para que la plantilla use el mismo grosor.
   - Cabecera `:1-4`: actualizar.
2. `src/components/ReadRings.svelte`:
   - Props: `let { keys, active = true }: { keys: GoalKey[]; active?: boolean } = $props();`. `createReadRings({ keys: () => keys, … })`.
   - Efecto `:21-30`: `if (!active) return;` (leyendo `active` dentro del efecto para que se vuelva a ejecutar). `wait = Math.max(active cambió hace poco ? 450 : 150, 1500 - (Date.now() - rr.mountedAt))`. Guardar el id de cada temporizador. Cuando `active` pase a false, cancelar los pendientes y sacarlos de `scheduled` (con un `$effect` que devuelva la limpieza).
   - `light()` (`:33`): además de `root?.isConnected`, salir si `!active`.
   - Cabecera `:2-4`: «Cara de la ficha de Inicio: anillos de las metas `keys`».
3. `src/screens/Home.svelte:109` y `:116`: de momento, `<ReadRings keys={GOAL_GROUPS.read} />` (importar). Así Inicio queda igual que hoy.

Comprobar: Inicio igual que antes. `npm run check`.

### Paso 5. Pantalla de racha con una pestaña por meta

`src/screens/Streak.svelte`:
1. `:23`: `createReadRings({ weeks: WEEKS, size: S, stroke: SW, gap: 1.18, empty: 'var(--face)', strokeFor: (n) => (n > 2 ? { stroke: 10, gap: 1.25 } : { stroke: SW, gap: 1.18 }) })`. En `:157-158`, `stroke-width={rr.stroke}`.
2. `const T = $derived(GOAL_TXT[groupOf(sel?.key ?? 'vn')]);` y `const many = $derived(tracks.length > 2);`.
3. Textos:
   - `:146` crumb → `T.crumb`.
   - `:242` → `T.what`.
   - `:254` → `Este mes aún no has ${groupOf(sel.key) === 'play' ? 'jugado a' : 'leído'} ${GOAL_COL[sel.key].full.toLowerCase()}.` Sale «novelas visuales / novelas japonesas / juegos de pc / switch»; para pc y switch conviene un texto propio («Este mes aún no has jugado a juegos de PC», «…a la Switch»).
   - `:260` → `T.when`. `:265` → `T.none30`. `:274` → `T.titles`.
   - `:286` hints → `['LB/RB', 'Cambiar meta', 'lb']`. `:288` → «No hay metas diarias en la biblioteca.».
   - Comentario de cabecera `:2-4`.
4. Pestañas `:210-216`: `data-hact={t.key === sel.key ? '' : 'goal:' + t.key}` y etiqueta `{many ? GOAL_COL[t.key].tab : GOAL_COL[t.key].full}`. Con 4 pestañas, quitar el `<small>meta … al día</small>` de `:209` si `many` (la meta ya sale en la leyenda) para que quepa en 768 px.
5. Modo compacto (`class:many` en `.streak`, `:149`):
   - Leyenda (`:164-174`): con `many`, nombre y `b / meta` en la misma línea, barra de 5 px y `sln2` a 12 px. Gap de 8 px. Tiene que caber en 252 px de alto (4 × ~56 + 3 × 8).
   - Medallas (`:179`): ya usan `repeat(tracks.length,1fr)`. Con `many`: `.smed` con padding `0 12px`, gap 8, llama y `.snum` más pequeños, y `.ssub` con elipsis. Usar `c.tab` en `.ssub` si no cabe.
   - CSS en `:300-340`.
6. Hucha (`:36-39`, `:198-205`): `resc` ordenado con `sel` primero y luego por `closeAt`, y `.slice(0, 2)`. `huch = resc.length > 1 ? 'duo' : sel?.rescue ? 'wide' : ''`. Con 2 metas, el comportamiento es el de hoy. Con 3 o más rescates a la vez, el de `sel` siempre se ve y los demás al cambiar de pestaña.
7. `src/components/HuchaRescate.svelte:26`: lo mismo que en el punto 4, con `data-hact={k === r.key ? '' : 'goal:' + k}` y etiqueta corta si `tabs.length > 2`.
8. Toast «¡Hucha llena!» (`:50-59`) y `light()` (`:117-143`): ya son genéricos por clave, no hay que cambiarlos.

Comprobar en el mock: abrir la racha. Hay 4 pestañas y LB/RB las recorren con bump en los extremos. El clic en una pestaña no contigua salta directamente. 4 anillos concéntricos sin solaparse. Leyenda y medallas sin desbordar. Con 2 metas (PC a 0 en Ajustes) se ve exactamente como hoy.

### Paso 6. Ficha de dos caras en Inicio (`ReadCard.svelte`)

1. `src/lib/readfmt.ts`: mover aquí `metaGoal` (`Home.svelte:27`) como `export const fmtGoal`. Home sigue importándolo si lo usa en otro sitio; si no, se borra de Home.
2. Nuevo `src/components/ReadCard.svelte`. Props: `{ left: number; w: number }`. Contenido:
   - `faces = app.goalFaces`. `keysOf = (g) => GOAL_GROUPS[g].filter((k) => app.goalKeys.includes(k))`.
   - Dos instancias pequeñas `rrR = createReadRings({ keys: () => keysOf('read'), size: 22, stroke: 3.5, gap: 1.3, empty: '' })` y `rrP` (igual con `'play'`). Dan las pistas para la urgencia y la geometría del anillo pequeño.
   - `level(rr)`: el máximo, según la regla de arriba (3 pending, 2 rescue, 1 en peligro). `pinned = $derived(nivel de una cara > nivel de la otra ? esa cara : null)` (solo con 2 caras).
   - Al montar (`onMount`): si `app.readFace` no está en `faces`, o al entrar en Inicio, `app.readFace = pinned ?? (lastPlayAt(keysOf('play'), …) > lastPlayAt(keysOf('read'), …) ? 'play' : 'read')`, restringido a `faces`.
   - `$effect`: si `pinned && pinned !== app.readFace && app.focus !== 'read'`, `app.readFace = pinned`. Si `faces` cambia y `readFace` ya no está, `faces[0]`.
   - Ciclo:
     - Constante `ROT = 12000`; `let cycle = $state(0)`; `let left = ROT, t0 = 0, timer`.
     - `paused = $derived(faces.length < 2 || app.focus === 'read' || app.view !== 'home' || pinned === app.readFace)`.
     - Efecto sobre `app.inputAt` y `app.readFace`: `left = ROT; cycle++;` y se reprograma.
     - Efecto sobre `paused`: si pasa a pausado, `clearTimeout` y `left -= Date.now() - t0`; si se reanuda, `t0 = Date.now(); timer = setTimeout(flip, left)`.
     - `flip()`: `app.readFace = pinned ?? otra`. Ojo: el `$effect` de `readFace` ya reinicia el ciclo. `onDestroy` limpia.
     - Cuidado con efectos que leen y escriben el mismo `$state` (`left`/`t0` son variables normales, no estado).
   - Plantilla, tres hermanos con posición absoluta, como ahora en Home:
     - `<div class="hsect" style="left:{left}px">{GOAL_TXT[app.readFace].card}</div>`
     - Indicador (solo si `faces.length > 1`), en `top:378px; right:{1280 - left - w}px`, de unos 24 px de alto:
       - Primero el anillo pequeño de la otra cara: SVG de 22 px con `rings` de `rrR`/`rrP`, arcos `soft` + `c` como en `ReadRings.svelte:55-60`. `data-hact="face:<otra>"`. Clicable.
       - Después dos puntitos: el inactivo, de 7 px; el activo, una pastilla de 20 px con `<b>` de relleno dentro de `{#key cycle}`. `animation: rfill {ROT}ms linear forwards; animation-play-state: {paused ? 'paused' : 'running'}`. Si la cara está fijada, relleno completo y sin animación. Color: `--acc` del primer anillo de la cara.
     - `<div class="glass rcard rread fx" class:f={app.focus === 'read'} data-k="read" data-act="read" style="left:{left}px;width:{w}px;--acc:var(--acc)"><div class="face">`. Dentro, `{#each faces as g (g)}<div class="rface" class:on={g === app.readFace}><ReadRings keys={keysOf(g)} active={g === app.readFace} /></div>{/each}`.
     - `<div class="rmeta" style="left:{left + 4}px;width:{w - 4}px">Meta diaria<span>&nbsp;· {keysOf(app.readFace).map((k) => `${fmtGoal(goalMin(app.config, k))} ${GOAL_COL[k].name}`).join(' · ')}</span></div>`. Sale «1 h novelas · 30 min japonés» o «1 h PC · 1 h Switch», como ahora.
   - CSS:
     - `.rface{position:absolute;inset:0;opacity:0;pointer-events:none;transition:opacity .45s ease}`
     - `.rface.on{opacity:1;pointer-events:auto}`
     - `@media (prefers-reduced-motion: reduce){.rface{transition:none}}`
     - y los estilos del indicador.
3. `src/screens/Home.svelte`:
   - `:107-111` → `{#if app.readCard}<ReadCard left={rx} w={RW} />{/if}`.
   - `:114-118` → `{#if app.readCard}<ReadCard left={1280 - 96 - RW} w={RW} />{/if}`.
   - Quitar el import de `ReadRings` y añadir el de `ReadCard`.
   - `:133` Hints: con `app.focus === 'read' && app.goalFaces.length > 1`, añadir `['LB/RB', 'Cambiar', 'lb']`.
   - `:34`, comentario: «la ficha de metas».
4. `src/lib/controller.svelte.ts:100`: `'read'` sigue igual (`app.readCard`).

Comprobar en el mock:
- La ficha alterna sola cada 12 s con fundido, y la barra del puntito se llena en 12 s.
- Al enfocarla, se para. Con LB/RB cambia de cara. Al pulsar cualquier tecla, la barra vuelve a empezar.
- El anillo pequeño enseña la otra cara. Al pulsarlo, cambia.
- A abre la racha con la primera meta de la cara visible.
- Meta PC = Desactivada y Switch = Desactivada: ficha fija, sin puntitos ni anillo pequeño, igual que hoy.
- Sin plataformas vn/jvn (mock editado): solo «Juego de hoy».
- Forzar una meta cumplida sin encender en PC (`localStorage.removeItem('visor.goalLit.pc')` con PC cumplida en el mock): Inicio arranca en «Juego de hoy», no rota, y la racha se enciende visible.

### Paso 7 (opcional, pequeño). Resumen de partida en la tablet

`mobile/src/tablet.ts:74-78`: `if (GOAL_KEYS.includes(s.platform) && goalMin(a.config, s.platform) > 0)`, con `const g = goalMin(a.config, s.platform)`, importando de `@desk/lib/readgoals`. Así la tostada de fin de partida muestra también la meta de PC y de Switch.

---

## Sitios que hoy asumen solo dos claves

| Sitio | Qué asume | Paso |
|---|---|---|
| `src/lib/readgoals.ts:5, 7-15, 17` | tipo, colores, `rescueCls` ternario | 2 |
| `src/lib/readrings.svelte.ts:42, 44` | `goalOf` ternario, `['vn','jvn']` | 4 |
| `src/lib/controller.svelte.ts:639-640` | `GOAL_KEYS`, `goalOf` ternario | 3 |
| `src/lib/controller.svelte.ts:703` | texto «de lectura» | 3 |
| `src/lib/store.svelte.ts:559` | `readCard` solo vn/jvn | 3 |
| `src/lib/reading.svelte.ts:15, 19, 26, 70-73, 165-166` | filas y acción de meta | 3 |
| `src/screens/Home.svelte:108-110, 115-117` | título y `rmeta` de lectura | 6 |
| `src/screens/Streak.svelte:213, 254, 286` y textos | pestañas `lb`/`rb` por posición, «novelas», «lees» | 5 |
| `src/components/HuchaRescate.svelte:26` | pestañas `lb`/`rb` por posición | 5 |
| `src/quickmenu/data.ts:20, 25` y mocks | tipo de `key` y `goals` | 2 |
| `src/quickmenu/TileTime.svelte:14, 44, 130-131` | `isReading`, clases `r-vn`/`r-jv` | 2-3 |
| `src/quickmenu/QuickMenu.svelte:535, 672` | `goals` por defecto | 2 |
| `src/rec/main.ts:21` | «racha de lectura» | 3 |
| `src/app.css:832-833`, `src/quickmenu/quickmenu.css:1329-1337` | solo `.r-vn`/`.r-jv` | 2 |
| `src-tauri/src/quickmenu.rs:961` | `goals` con 2 claves | 1 |
| `src-tauri/visor-core/src/config.rs:133-134, 249-250, 292-293`, `remote.rs:57-65, 1134` | config | 1 |
| `mobile/src/components/GoalRings.svelte:10, 19-21`, `mobile/src/tablet.ts:74-76` | móvil y tablet | 2 / 7 |

Sin cambios, porque ya son genéricos por clave: `goalLit`/`markGoalLit`/`rescueSeen` (localStorage por clave), `store.svelte.ts:654` (`hueOf(GOAL_COL[streakKey])`), `lib.rs:414` y `overlay.rs:51` (`key` pasa tal cual), `HuchaMini.svelte` (usa la clase global).

**No tocar**: `Detail.svelte:121` y `gameset.svelte.ts:28` (`isNovel`/`isJvn` son funciones de novela, no de meta), y la detección de plataformas.

Nada de esto es masivo: no compensa delegar en `agy`.

## Riesgos y dudas abiertas

- **Dónde va el indicador**: lo pongo en la línea del título, encima de la esquina derecha de la ficha, porque dentro la tapa la medalla de la segunda columna. Si el usuario lo quiere dentro de la ficha, hay que reducir el ancho de `.rr-side` o el padding de arriba en las dos caras. Conviene confirmarlo con una captura.
- **Volver de la racha**: según la regla, la cara inicial se recalcula (pendiente, o lo último jugado). Puede chocar si el usuario venía de mirar la pestaña de PC. Propuesta (sin aplicar): si al montar `app.focus === 'read'` (se vuelve de la racha), usar `groupOf(app.streakKey)`.
- **Coste**: hay 2 instancias pequeñas más de `createReadRings` en `ReadCard`, así que `goalTrack` se calcula dos veces por cara cada 20 s. PC puede tener muchas partidas. Es lineal, debería ir bien; si se nota, que `ReadRings` acepte las pistas por prop.
- **«Racha en peligro» casi siempre empata** (las dos caras con racha y sin cumplir): entonces se rota. Solo se fija por diferencia de nivel. Si el usuario prefiere fijar siempre que haya algo, basta cambiar `>` por «nivel > 0 y la otra no».
- **`ink`/`edge` de pc y switch** son una propuesta: hay que mirar el contraste del naranja (`#b33a12` sobre `#fbd9cb`) en la pantalla real.
- `docs/plan-resumen-semanal.md:95` pasa `{vn, jvn}` a `weekReview`. Si ese plan se ejecuta después, debería usar `goalMin(app.config, k)` con las 4 claves.
