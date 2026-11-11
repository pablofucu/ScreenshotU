# Plan: resumen semanal

## Supuesto confirmado

«Las tres opciones» son tres **fuentes**, cada una ligada a categorías que ya existen y que **no se tocan**:

| Fuente | Categoría(s) (`Platform.id`) | De dónde sale el dato |
|---|---|---|
| 1. Juegos de PC | `pc` | `Title.plays` (tabla `sessions` de visor.db, ya en la biblioteca en memoria) |
| 2. Switch | `switch` | `Title.plays` (emulador lanzado desde el visor) **+** `Title.console.days` (Nintendo, tabla `switch_daily`, últimos 28 días) |
| 3. Novelas | `vn` y `jvn` | Minutos: `Title.plays` de esas categorías (lo mismo que usan los anillos). Estudio: `readstats_get` (caracteres, palabras conocidas por día, racha de lectura) |

Las capturas no son fuente: solo aportan «capturas destacadas de la semana» y el número de capturas, dentro del bloque de cada fuente.

Cada fuente es un bloque propio y los rankings («más jugado», «más leído») se hacen **dentro de cada categoría**. `vn` y `jvn` comparten bloque visual («Novelas»), pero cada una tiene su fila, su meta y su top, sin mezclarse. Solo la cabecera suma totales de todo (tiempo, días activos), como ya hace `yearTotals` en `src/lib/stats.ts:366`. Ninguna lista mezcla títulos de categorías distintas.

## Decisión

- **Todo se calcula en el frontend con una función pura `weekReview()`** en `src/lib/stats.ts`, a partir de `app.lib`, que ya está en memoria. No hay escaneo nuevo, ni tablas nuevas, ni lecturas del HDD (P:). Lo mismo hace hoy el resumen anual (`yearReview`, `stats.ts:384`). La tablet ejecuta la misma interfaz de escritorio (`src/lib/remote/core.ts`), y `mobile/` importa `@shared/stats`, así que funciona en los tres sitios sin tocar la API.
- **Lo único que necesita Rust** es lo que no viaja en la biblioteca: frases añadidas al glosario por día y logros desbloqueados en un rango de fechas. Son dos añadidos pequeños y van en la fase 4.
- **Semana: de lunes 00:00 a domingo 24:00, hora local del equipo.** Es la misma semana que la pestaña «Semana» de Estadísticas (`periodRange('week')`, `stats.ts:62`) y que `readstats::week_start` (`readstats.rs:84`). Excepción: los días de meta cumplida y la racha se toman de `goalTrack` (`readgoals.ts:171`), cuyo día empieza a las 7:00 (`DAY_START_HOUR`). Así el resumen dice lo mismo que los anillos.
- **Sin caché persistente:** calcular sobre 14k capturas y unos miles de partidas cuesta milisegundos. Lo único que se guarda es «semana vista» en `localStorage`, con el mismo patrón que `goalLit` / `markGoalLit` (`readgoals.ts:352-365`).
- **Descartado:** calcular el resumen en Rust a partir de `sessions` + `switch_daily` + `read_days`. Duplicaría en Rust el reparto por días, la unión de partidas (`mergeSpans`) y las metas, que hoy viven en TS, y las cifras acabarían sin cuadrar con Estadísticas y los anillos.

## Contenido del resumen

**Cabecera:** «Semana del 2 al 8 nov». Tiempo total, días con actividad (x/7) y comparación con la semana anterior (▲/▼ en % y en tiempo), con el estilo de `rankPeriod` (`prevMs`).

**Bloque PC (`pc`):**
- Tiempo, partidas y juegos distintos, con ▲/▼ frente a la semana anterior.
- Top 3 por tiempo con mini portada (`MiniCover`).
- La partida más larga (juego, día y duración).
- Día favorito de la semana y barras por día (7 columnas).
- Hitos: el total de un juego cruza 10/25/50/100/200 h durante la semana (horas antes de `from` frente a horas en `to`).
- Juegos empezados esta semana (`firstPlayed` dentro del rango) y terminados (`finishedMs` dentro del rango).
- Logros desbloqueados (fase 4): número, el más raro (`ach_rarity.pct`) y su juego.
- Capturas de la semana de esta categoría: número y destacada (favorita más reciente, si no la más reciente), con el criterio de `yearReview` (`stats.ts:421-436`).

**Bloque Switch (`switch`):**
- Lo mismo que PC, sumando `console.days` por día a las partidas del emulador.
- Etiqueta «x h en la consola», para que se vea qué viene de Nintendo.
- Aviso «datos de Nintendo hasta el <día>» si el último día con datos es anterior al domingo.

**Bloque Novelas (`vn` y `jvn`, una fila cada una):**
- Minutos de la semana y meta cumplida x/7 días (de `goalTrack(...).weeks`: la columna de la semana es la última, o la penúltima si ya es lunes y se mira la semana pasada).
- Racha actual al cerrar la semana y rescates usados (`MapCell.rescued`).
- Novela más leída de la categoría.
- Solo en `jvn`, sacado de `readstats_get(null).days`: caracteres leídos, velocidad media (car./h = chars/activeMs), palabras nuevas conocidas (`newKnown`) y, en la fase 4, frases añadidas al glosario.

**Pie:** «Lo mejor de la semana». Rotan 3 destacados de cualquier bloque: hito, racha, juego terminado, logro raro, récord de partida. Cada destacado lleva la etiqueta de su categoría, como hace ya `Year.svelte:17` (`pname`).

## Formato y entradas

1. **Pantalla propia:** cuarta pestaña de Estadísticas, «Resumen semanal», entre Calendario y Resumen anual. LB/RB (o ←/→) cambian de semana.
2. **Aviso de semana nueva:** la primera vez que se abre la app a partir del lunes, si la semana pasada tuvo actividad y no se ha visto, aparece en Inicio una tarjeta-aviso con tres cifras (una por fuente). A abre la pestaña y B la descarta. Se marca vista en `localStorage` por equipo, así que el PC y la tablet la ven una vez cada uno.
3. **Tarjeta compartible (PNG):** fase 5, opcional. Imagen de 1200×675 dibujada en `<canvas>` con las cifras, el top de cada fuente y la captura destacada (desde la miniatura ya cacheada, nunca el original en P:). Se guarda con un comando Rust en una carpeta del usuario.
4. **Notificación nativa de Windows el lunes:** fase 6, opcional. Ver la decisión abierta 3.

## Pasos

### Fase 1: cálculo puro (tamaño M)

1. **`src/lib/stats.ts`, tras `yearReview` (~l.444):**
   - Añadir `export interface WeekReview` y `export function weekReview(lib, off, favs, live, goals: {vn, jvn}, now = Date.now())`.
   - Rango: `periodRange('week', off, now)`. La semana anterior es `off - 1`.
   - Por bloque (`'pc' | 'switch' | 'novels'`), filtrar `lib.platforms` por `p.id`. Para cada título no oculto:
     - `overlap(p, from, to)` para el tiempo.
     - `eachDay` para las barras por día.
     - El conteo de partidas como en `rankPeriod` (`stats.ts:111-129`).
   - Switch: además, sumar `t.console?.days` cuyos `[dayStartMs, min]` caigan en `[from, to)`. Esas entradas ya empiezan a medianoche local. Guardar `consoleMs` aparte.
   - Hitos: `hoursBefore` = `playSeconds*1000` menos lo jugado desde `from`. El umbral se cruza si `hoursBefore < H <= hoursBefore + msSemana`. Umbrales en una constante `MILESTONES_H = [10, 25, 50, 100, 200]`.
   - Capturas: el mismo criterio de fecha local que `shotYear` (`stats.ts:352`). `s.date` es la hora local codificada como UTC, así que hay que comparar con `from - new Date(from).getTimezoneOffset()*60000`. Hacer un helper `shotInRange`.
   - Novelas: llamar a `goalTrack('vn'|'jvn', goal, lib.platforms, live, now, nWeeks)` (de `readgoals.ts`), con `nWeeks` = `1 - off + 1`, y tomar la columna de la semana.
   - Devolver también `prev` con los totales de cada bloque para las ▲/▼.
   - Una categoría que no exista en la biblioteca da un bloque `null` y no se pinta.
2. **`tests/weekreview.test.ts` (nuevo, con `node --test` como `tests/readgoals.test.ts`):**
   - Partida que cruza la medianoche del domingo.
   - Cambio de hora (último domingo de octubre).
   - Días de consola de Switch.
   - Hito cruzado.
   - Capturas en el borde lunes 00:00.
   - Juegos ocultos excluidos.
   - Ningún top mezcla `pi`.
   - Apto para `agy` si se quieren muchos casos. Revisar su diff.

### Fase 2: pestaña «Resumen semanal» (tamaño M)

3. **`src/lib/store.svelte.ts:420`:** `statsTab` admite `'week'`. Añadir `weekOff = $state(0)` (0 = esta semana; el aviso abre con -1).
4. **`src/lib/controller.svelte.ts`:**
   - l.737-743: el ciclo pasa a ser rank → cal → week → year.
   - l.146-147: foco por defecto `''` para `'week'`, como `'year'`.
   - l.777 y l.788: con `'week'`, ←/→ y LB/RB llaman a un `weekShift(d)` nuevo, limitado a `off <= 0` y a no ir más atrás que la primera partida.
   - l.1756/1767: no tocar.
5. **`src/screens/Week.svelte` (nuevo):**
   - Recibe `weekReview(app.lib, app.weekOff, app.favs, app.playing, {vn: app.config.goalVn, jvn: app.config.goalJvn}, app.now)`.
   - Diseño: cabecera más tres columnas o tarjetas (PC / Switch / Novelas), con el estilo de tarjetas `.rbox` de `Year.svelte`.
   - Reutiliza `MiniCover`, `fmtHM` y los colores `GOAL_COL` para `vn`/`jvn`.
   - El estudio japonés se pide con `readstats_get(null)` (ya existe; `backend` / `invoke`) en un `$effect` al montar.
   - Pedir antes un mockup HTML en `docs/archivo/maquetas/design/resumen-semanal-propuestas.html`, siguiendo la costumbre del proyecto, si el usuario quiere elegir el diseño.
6. **`src/screens/Stats.svelte`:**
   - l.14: `VIEW_LABEL` con `week: 'Resumen semanal'`.
   - l.68-77: una rama `{:else if app.statsTab === 'week'}<Week />`.
   - l.95: añadir `'week'` a los puntos.
   - l.103-106: las pistas (`Hints`) de esa pestaña: «LB/RB Semana», «Y Pestaña».

### Fase 3: aviso de semana nueva en Inicio (tamaño S)

7. **`src/lib/readgoals.ts` (junto a l.352-365) o `stats.ts`:** `weekSeen(weekFrom)` / `markWeekSeen(weekFrom)` con `localStorage` (clave `visor.weekSeen`).
8. **`src/screens/Home.svelte`:**
   - Una tarjeta-aviso superpuesta (no una fila nueva; el layout es de posiciones fijas, l.101-117) cuando se cumpla todo esto:
     - Es la semana nueva.
     - `!weekSeen(lastWeek)`.
     - La semana pasada tuvo actividad.
   - La tarjeta muestra «Tu semana: PC 8 h 20 · Switch 3 h · Novelas 5/7 metas».
9. **`src/lib/controller.svelte.ts`:**
   - Al abrir el aviso, A hace `app.view='stats'; app.statsTab='week'; app.weekOff=-1` y marca la semana vista.
   - B solo la marca vista.

### Fase 4: datos que faltan, en Rust (tamaño S-M)

10. **`src-tauri/visor-core/src/jp/readstats.rs`:**
    - `DayPoint` (l.219): añadir `phrases: u32`.
    - Rellenarlo en `compute` (l.300-302 acumula `per_day`; ampliar la tupla con `r.phrases`).
    - Ajustar el test `agregados` (l.512).
    - Frontend: añadir `phrases` en `src/lib/novel-types.ts:54`.
11. **Logros de la semana.** Nuevo `#[tauri::command] week_achievements(from_ms, to_ms) -> Vec<WeekAch>` en `src-tauri/src/achievements.rs`:
    - Recorre `state.games`. Para cada juego obtiene `app_id` con `resolve_game_info` (l.27), cacheando el mapa en memoria durante la sesión.
    - Consulta `achievement_unlocks` con `unlocked_at BETWEEN from/1000 AND to/1000` (**está en segundos**: ver `achievements/mod.rs:231` y l.524), con JOIN a `achievements` (nombre, icono) y LEFT JOIN a `ach_rarity` (pct).
    - Solo lee la BD; no llama a `sync_achievements_from_disk`.
    - Devuelve `{gameId, displayName, icon, unlockedAt, pct}`. El frontend lo reparte a su categoría por `gameId`.
12. **Registrar el comando** en:
    - `src-tauri/src/lib.rs`, `invoke_handler` (~l.1446).
    - `src-tauri/src/remote_call.rs`, `dispatch` (junto a l.156): `"week_achievements" => val(...)`.
    - `src/lib/remote/core.ts:11-16`, conjunto `READS`, para que la tablet lo tenga sin conexión.
    - `src/lib/backend.ts` (~l.185): el envoltorio.
    - `src/lib/mock.ts`: un valor de prueba.

### Fase 5: tarjeta compartible (tamaño M, opcional)

13. **Spike primero:** comprobar si `<canvas>` queda contaminado (tainted) al dibujar imágenes del protocolo de assets de Tauri (`convertFileSrc`). Si queda contaminado, cargar las miniaturas como `data:` mediante un comando Rust que lea el fichero de `thumbs_dir()` (solo esa carpeta y la de portadas SGDB).
14. **`src/lib/weekcard.ts` (nuevo):** dibuja la imagen de 1200×675 a partir de `WeekReview` y devuelve `Blob`. Nuevo comando `save_week_card(png_b64, name)` en `src-tauri/src/lib.rs`, que escribe en `<Imágenes>/Visor/resumen-AAAA-Www.png` y abre la carpeta. Entra en `remote_call` como no permitido (es una ruta del PC).

### Fase 6: notificación de Windows el lunes (tamaño S, opcional)

15. **`src-tauri/src/lib.rs`, en el arranque (setup):** si es lunes o más tarde y `meta.week_notified` < lunes actual, lanzar un toast con `tauri_plugin_notification` (ya en uso en `src-tauri/src/snap.rs:58`): «Tu resumen semanal está listo». Sin cifras, para no duplicar cálculos en Rust. Al pulsarlo se abre la pestaña. Se guarda la marca en la tabla `meta`.

## Comprobación

- `node --test tests/weekreview.test.ts tests/readgoals.test.ts` en verde.
- `cargo test` en `src-tauri/visor-core` (readstats) en verde.
- En la app, con la semana pasada (`weekOff=-1`):
  - El tiempo de PC coincide con Estadísticas → Clasificación → «Semana pasada», categoría `pc`.
  - Los días de meta de `vn`/`jvn` coinciden con el mapa de la pantalla Racha.
  - Los minutos de Switch = Clasificación + suma de `console.days` de esa semana.
- Abrir la app un lunes (o cambiar la fecha del equipo): el aviso sale una vez y no vuelve tras A o B.
- En la tablet: la pestaña se ve igual y `week_achievements` responde por `/api/v1/call`.
- Medir `weekReview` con la biblioteca real (`console.time`): debe tardar menos de 20 ms.

## Riesgos

- **Datos de Nintendo con retraso:** `switch_daily` se rellena al sincronizar. El lunes, el domingo puede faltar. Por eso el aviso «datos hasta el <día>»; si no, la semana sale corta sin que se note.
- **`console.days` solo cubre 28 días** (`nintendo/merge.rs:22`). Navegar más de 3 semanas atrás pierde el tiempo de consola. Mostrarlo como «sin datos de consola» y no como 0, o limitar el retroceso del bloque Switch.
- **Dos criterios de día:** medianoche para los totales y 7:00 para metas y racha. Una sesión de las 2:00 del lunes cuenta en la semana nueva para el tiempo y en la anterior para la meta. Es lo mismo que pasa hoy entre Estadísticas y los anillos; documentarlo en un tooltip o aceptarlo.
- **Logros:** `resolve_game_info` lee la carpeta de instalación de cada juego (no P:). Con muchos juegos, la primera llamada puede tardar. Va cacheado y en `spawn_blocking`.
- **Hitos con tiempo antiguo** (`playSeconds` sin partidas): el cálculo «horas antes de la semana» lo resta bien, pero conviene un test.

## Decisiones abiertas para el usuario

1. **Categorías que no son PC, Switch ni novelas** (si las hay: otras consolas o emuladores con categoría propia): ¿bloque «Otras» con una fila por categoría, o fuera? *Recomiendo una fila por categoría bajo «Otras», sin top compartido.*
2. **Dónde vive:** ¿pestaña de Estadísticas + aviso en Inicio, o también una tarjeta fija en Inicio (propuesta 6 de `docs/archivo/maquetas/design/estadisticas-propuestas.html`)? *Recomiendo pestaña + aviso; Inicio ya está lleno (recientes + anillos).*
3. **Notificación nativa de Windows el lunes:** ¿sí o no? Solo sirve si la app arranca con Windows o está en la bandeja. *Recomiendo dejarla para después (fase 6) y empezar con el aviso dentro de la app.*
4. **Tarjeta PNG compartible:** ¿se quiere? Es la fase más incierta (canvas contaminado). *Recomiendo hacerla después de usar el resumen un par de semanas.*
5. **Semana con lectura a las 7:00:** ¿se acepta la doble frontera (totales a medianoche, metas a las 7:00) o se pasa todo el resumen a las 7:00? *Recomiendo aceptarla, para que las cifras cuadren con Estadísticas y con los anillos, cada uno en lo suyo.*
