# Rachas con días libres (semana móvil)

Maqueta elegida: [maquetas/rachas/semana-7-velas-revisada.html](../maquetas/rachas/semana-7-velas-revisada.html) («Velas revisada», 15-nov). Lo estético está aprobado; este plan fija lo funcional. Sustituye a la hucha de rescate (`Rescue`, `HuchaRescate`, `HuchaMini`).

Decisión del 15-nov (RCM-1): la pantalla de racha antigua (anillos de hoy, medallas, mapa de 6 meses, lo leído este mes) pasa a **Estadísticas › Rachas**. La pantalla de racha que se abre desde Inicio es la nueva de la maqueta.

## Reglas (una por clave de meta: `vn`, `jvn`, `pc`, `switch`; nunca se mezclan)

1. **Meta del día.** Cada día local (empieza a las 7:00, `localDay`) tiene su meta `goalAt(hist, d, goal)`. Se cumple con `metGoal` (como ahora).
2. **Día libre.** Un día pasado (`d < hoy`) sin cumplir es **día libre** si:
   - el día anterior pertenece a la racha (cumplido o día libre vigente), y
   - en `[d-6, d-1]` hay menos de 2 días libres (como mucho **2 días libres en cualquier tramo de 7 días**).
   Si no, ese día **rompe** la racha. Un día con 0 min también puede ser día libre. Los días libres cuentan como días de la racha.
3. **Deuda.** Un día libre debe `need = meta(d) − minutos(d)` (redondeados). Se paga con lo leído **de más** (`max(0, válidos(w) − meta(w))`) en los días `d+1 … d+7` (hoy incluido). `válidos` = solo partidas de `EXTRA_MIN_PLAY` (10) min o más, como la hucha.
4. **Reparto automático.** El exceso de cada día paga primero el día libre más antiguo aún abierto y dentro de plazo. **No se guarda por adelantado:** el exceso de un día solo paga días libres anteriores a él; lo que sobre se pierde.
5. **Plazo.** Si al terminar el día `d+7` sigue debiendo algo, el día `d` pasa a **roto**: la racha se corta ahí (los días cumplidos después forman una racha nueva). Como esto cambia la historia hacia atrás, se calcula por punto fijo: se recalcula con `d` forzado a roto hasta que no aparezca ningún plazo vencido nuevo.
6. **Fichas.** Fichas libres hoy = `2 − (días libres en [hoy-6, hoy-1])`. La ficha gastada el día `d` vuelve el día `d+7`.
7. **Día libre extra por racha (oculto).** Al llegar a 15 días seguidos de racha (cumplidos o libres, 15, 30, 45…) se gana un día libre extra. No se ve en la interfaz, no se acumula (como mucho uno guardado) y el siguiente día libre lo gasta en vez de una ficha: ese día no cuenta para el tope de 2 en 7 días ni gasta ficha, pero su deuda se paga igual. Si la racha se rompe, se pierde. Código: `BONUS_STREAK`, `FreeDay.bonus`.
8. **Meta diferida.** Cambiar la meta en Ajustes vale **desde mañana**: hoy sigue la anterior (no se puede bajar la meta para cumplir hoy). Excepciones: activar (desde 0) o desactivar (a 0) valen al momento.

### Historial de metas

`config.goalHist[key] = [[díaDesde, minutos], …]` ordenado. `goalAt(hist, d, goal)` = último elemento con `díaDesde ≤ d`; si no hay historial o ningún elemento vale, `goal` (la meta actual de la config). Al cambiar en Ajustes de `G0` a `G1` el día `T`: si el historial está vacío se añade `[0, G0]`; se añade (o sustituye) `[T+1, G1]`; se guarda `goalVn = G1`. Con 0 en cualquiera de los dos lados, `[T, G1]` (inmediato).

## API nueva de `src/lib/readgoals.ts`

```ts
export const EXTRA_MIN_PLAY = 10;      // antes RESCUE_MIN_PLAY
export const FREE_MAX = 2;             // días libres en 7 días
export const FREE_WINDOW = 7;          // plazo y ventana
export type GoalHist = [number, number][];
export function goalAt(hist: GoalHist | undefined, d: number, goal: number): number;
export function goalHistOf(cfg: Pick<Config, 'goalHist'>, k: GoalKey): GoalHist;

export interface FreeDay {
  day: number;        // el día libre
  goal: number;       // meta de ese día
  played: number;     // minutos (redondeados)
  need: number;       // lo que faltó
  paid: number;       // recuperado (≤ need)
  owe: number;        // need − paid
  done: boolean;      // owe ≤ 0
  deadline: number;   // último día para recuperarlo (day + 7) = día en que vuelve la ficha
  left: number;       // días que quedan contando hoy: deadline − hoy + 1 (2 = «hoy y mañana»)
  closeAt: number;    // ms: dayStart(deadline + 1)
  payDay: number;     // día en que quedó saldado (NaN si no)
}
export interface WeekDay {   // una vela: los 7 últimos días, de hoy-6 a hoy
  day: number; goal: number; min: number;
  met: boolean; today: boolean;
  extra: number;      // minutos por encima de la meta que cuentan (válidos)
  gave: number;       // parte de `extra` que pagó días libres
  free: FreeDay | null;     // si es día libre
  broken: boolean;    // día que rompió la racha
}
export type StreakState = 'ok' | 'lit' | 'debt' | 'tope' | 'urgent';
// ok: al día sin cumplir hoy · lit: hoy cumplido y nada pendiente · debt: algo por recuperar
// tope: 0 fichas libres hoy · urgent: un día libre abierto con left ≤ 2  (prioridad urgent > tope > debt > lit/ok)

export interface GoalTrack {
  key; goal /* meta de hoy */; nextGoal /* meta de mañana (≠ goal si hay cambio diferido) */;
  today; frac; done; streak; best; bestFrom; bestTo; weeks;
  week: WeekDay[];          // 7 elementos
  free: FreeDay[];          // días libres con deadline ≥ hoy (vigentes o saldados), del más antiguo al más reciente
  open: FreeDay[];          // los de `free` sin saldar
  owe: number;              // suma de `open[].owe`
  urgent: FreeDay | null;   // el abierto más antiguo con left ≤ 2
  tokens: number;           // fichas libres hoy (0..2)
  tokenBack: number;        // día en que vuelve la próxima ficha gastada (NaN si no hay)
  state: StreakState;
  paidDay: number | null;   // día libre que se saldó HOY (para el aviso «Día libre recuperado»)
}
// MapCell: `rescued` desaparece; añadir `free: boolean` (día libre, saldado o vigente) y `owe: boolean` (aún debe).
export function goalTrack(key, goal, platforms, live, now, nWeeks = MAP_WEEKS, hist: GoalHist = []): GoalTrack;
```

`Rescue`, `RescuePlay`, `RESCUE_*`, `rescueSeen/markRescueSeen` y `rescueCls` se quitan; `readfmt.ts` cambia sus textos a días libres (`freeDayName(d, long)`, `freeLeft(f)` → «hoy», «hoy y mañana», «quedan N días», `freeClose(f)`). `readDetail` se mantiene.

## Reparto (agentes en paralelo; cada uno toca solo sus ficheros)

| Agente | Ficheros | Qué |
|---|---|---|
| A · Lógica | `src/lib/readgoals.ts`, `src/lib/readfmt.ts`, `src/lib/stats.ts`, `tests/readgoals.test.ts`, `src/lib/reading.svelte.ts`, `src-tauri/visor-core/src/config.rs`, `src-tauri/visor-core/src/remote.rs` (solo `RemoteConfig`) | Reglas, API, pruebas; Ajustes con meta diferida («desde mañana»); `goal_hist` en la config de Rust (serde `default`, camelCase) y en `RemoteConfig` |
| B · Pantalla de racha | `src/screens/Streak.svelte` (reescrita), `src/components/racha/*` (nuevos, salvo `VelasMini`) | La «Pantalla de racha» de la maqueta |
| C · Estadísticas › Rachas | `src/screens/stats/Rachas.svelte` (nuevo, a partir del `Streak.svelte` de HEAD), `src/screens/Stats.svelte`, `src/lib/store.svelte.ts`, `src/lib/controller.svelte.ts` | Pestaña «Rachas» con la pantalla antigua sin hucha; navegación; Y en la pantalla de racha abre esta pestaña |
| D · Inicio, overlay y móvil | `src/components/ReadRings.svelte`, `src/components/ReadCard.svelte`, `src/components/racha/VelasMini.svelte`, `src/lib/readrings.svelte.ts`, `src/quickmenu/TileTime.svelte`, `src/quickmenu/data.ts`, `src-tauri/src/quickmenu.rs`, `src/rec/main.ts`, `src-tauri/src/overlay.rs`, `mobile/src/**` | Tarjeta de Inicio de la maqueta; baldosa y hoja de racha del menú sobre el juego; aviso sobre el juego; anillos del móvil |

Después (sesión principal): avisos de `scheduleGoalNotice` en el controlador, borrar `HuchaRescate`/`HuchaMini`, `npm run check`, pruebas y repaso del diff.

## Avisos sobre el juego (overlay)

- Meta de hoy sin cumplir: aviso al cumplirla (como ahora). Si hay deuda abierta, el subtítulo dice cuánto falta para el día libre (`Lo que leas de más paga el día libre · faltan 25 min`).
- Meta cumplida y deuda abierta: aviso cuando lo leído de más salde **todos** los días libres abiertos: «¡Día libre recuperado!» (sustituye a «¡Racha recuperada!»).
- El texto del aviso viaja como `clave|días|titular|subtítulo` (el cuarto campo es opcional; si falta, el de siempre).
