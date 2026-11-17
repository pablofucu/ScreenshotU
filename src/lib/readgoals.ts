// Anillos de Inicio: minutos leídos o jugados cada día en las novelas visuales, las japonesas, los juegos de PC y Switch,
// la meta diaria de cada una, la racha de días cumplidos y el mapa de las últimas semanas.
import type { Config, Platform, Title } from './types';

export type GoalKey = 'vn' | 'jvn' | 'pc' | 'switch';
export const GOAL_KEYS: GoalKey[] = ['vn', 'jvn', 'pc', 'switch'];
export type GoalGroup = 'read' | 'play';
export const GOAL_GROUPS: Record<GoalGroup, GoalKey[]> = { read: ['vn', 'jvn'], play: ['pc', 'switch'] };
export const groupOf = (k: GoalKey): GoalGroup => (k === 'pc' || k === 'switch' ? 'play' : 'read');
const GOAL_CFG = { vn: 'goalVn', jvn: 'goalJvn', pc: 'goalPc', switch: 'goalSwitch' } as const;
export const goalMin = (cfg: Pick<Config, 'goalVn' | 'goalJvn' | 'goalPc' | 'goalSwitch'>, k: GoalKey) => cfg[GOAL_CFG[k]] ?? 0;
export const goalCfgKey = (k: GoalKey) => GOAL_CFG[k];
/** Colores de cada anillo: azul = novelas visuales, rosa = novelas japonesas, verde = juegos de PC, naranja = Switch. */
export const GOAL_COL: Record<GoalKey, { c: string; soft: string; ink: string; edge: string; name: string; full: string; tab: string }> = {
  vn: { c: '#00A0D8', soft: '#cfe6f1', ink: '#00739c', edge: '#0073a0', name: 'novelas', full: 'Novelas visuales', tab: 'Novelas' },
  jvn: { c: '#E5487A', soft: '#f6d3df', ink: '#a82b58', edge: '#b02c5c', name: 'japonés', full: 'Novelas japonesas', tab: 'Japonés' },
  pc: { c: '#1DB37A', soft: '#cdeedd', ink: '#11804f', edge: '#14875a', name: 'PC', full: 'Juegos de PC', tab: 'PC' },
  switch: { c: '#F25C2A', soft: '#fbd9cb', ink: '#b33a12', edge: '#c0431a', name: 'Switch', full: 'Switch', tab: 'Switch' },
};
/** Textos de cada grupo de metas (lectura / juego). */
export const GOAL_TXT: Record<GoalGroup, { card: string; crumb: string; what: string; when: string; none30: string; titles: string; notice1: string; sub1: string }> = {
  read: { card: 'Lectura de hoy', crumb: 'Racha de lectura', what: 'Qué has leído este mes', when: 'A qué hora lees', none30: 'Sin lecturas en los últimos 30 días', titles: 'Novelas', notice1: '¡Meta de lectura cumplida!', sub1: 'Hoy cuenta para tu racha de lectura' },
  play: { card: 'Juego de hoy', crumb: 'Racha de juego', what: 'Qué has jugado este mes', when: 'A qué hora juegas', none30: 'Sin partidas en los últimos 30 días', titles: 'Juegos', notice1: '¡Meta de juego cumplida!', sub1: 'Hoy cuenta para tu racha de juego' },
};
export const MAP_WEEKS = 10;

export interface MapCell {
  /** Parte de la meta cumplida ese día (0..1). */
  v: number;
  /** Minutos leídos / meta, sin tope (1 = meta justa, 2 = el doble…): gradúa el color por encima de la meta. */
  r: number;
  /** El día forma parte de la racha actual. */
  streak: boolean;
  today: boolean;
  future: boolean;
  /** Día libre dentro de la semana móvil (saldado o vigente). */
  free: boolean;
  /** Día libre con deuda aún abierta. */
  owe: boolean;
}

/** Solo las partidas de al menos esto (minutos) cuentan como extra para pagar un día libre. */
export const EXTRA_MIN_PLAY = 10;
/** Máximo de días libres permitidos en cualquier tramo de `FREE_WINDOW` días. */
export const FREE_MAX = 2;
/** Plazo (días) para recuperar lo que faltó en un día libre y ventana de las fichas. */
export const FREE_WINDOW = 7;
/** Cada tantos días seguidos de racha se gana un día libre extra (oculto, no se acumula: como mucho uno guardado). */
export const BONUS_STREAK = 15;

/** Historial ordenado de metas: `[díaDesde, minutos][]`. */
export type GoalHist = [number, number][];

/** Meta vigente en el día local `d` según el historial `hist` (o `goal` si no hay entrada previa). */
export function goalAt(hist: GoalHist | undefined, d: number, goal: number): number {
  if (!hist?.length) return goal;
  let found: number | undefined;
  let bestDay = -Infinity;
  for (const [from, min] of hist) {
    if (from <= d && from >= bestDay) {
      bestDay = from;
      found = min;
    }
  }
  return found ?? goal;
}

/** Historial de metas de la configuración para la clave `k`. */
export function goalHistOf(cfg: Pick<Config, 'goalHist'>, k: GoalKey): GoalHist {
  return cfg.goalHist?.[k] ?? [];
}

/** Día libre usado dentro de la racha: lo que faltó se recupera con exceso en `day+1 .. day+7`. */
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
  /** Día libre extra ganado con la racha: no gasta ficha ni cuenta para el tope de `FREE_MAX`. */
  bonus: boolean;
}

/** Una vela de los últimos 7 días (de hoy-6 a hoy). */
export interface WeekDay {
  day: number;
  goal: number;
  min: number;
  met: boolean;
  today: boolean;
  /** Minutos válidos por encima de la meta. */
  extra: number;
  /** Parte de `extra` que pagó días libres. */
  gave: number;
  free: FreeDay | null;
  /** Día que rompió la racha. */
  broken: boolean;
}

export type StreakState = 'ok' | 'lit' | 'debt' | 'tope' | 'urgent';

export interface GoalTrack {
  key: GoalKey;
  /** Meta de hoy. */
  goal: number;
  /** Meta de mañana (distinta de `goal` si hay un cambio diferido). */
  nextGoal: number;
  /** Minutos de hoy. */
  today: number;
  /** Parte de la meta de hoy cumplida (0..1; 1 en cuanto `done`). */
  frac: number;
  done: boolean;
  streak: number;
  best: number;
  /** Días (locales) de la mejor racha; NaN si aún no hay ninguna. */
  bestFrom: number;
  bestTo: number;
  /** Semanas (lunes a domingo), la última es la actual. */
  weeks: MapCell[][];
  /** Últimos 7 días (de hoy-6 a hoy). */
  week: WeekDay[];
  /** Días libres con deadline ≥ hoy (vigentes o saldados), del más antiguo al más reciente. */
  free: FreeDay[];
  /** Los de `free` sin saldar. */
  open: FreeDay[];
  /** Suma de `open[].owe`. */
  owe: number;
  /** Día libre abierto más antiguo con `left <= 2`. */
  urgent: FreeDay | null;
  /** Fichas libres hoy (0..2). */
  tokens: number;
  /** Día en que vuelve la próxima ficha gastada (`NaN` si no hay). */
  tokenBack: number;
  state: StreakState;
  /** Día libre que se saldó HOY (para el aviso «Día libre recuperado»); null si no. */
  paidDay: number | null;
}

/** Hora local a la que empieza el día de las metas (antes de esa hora todavía cuenta como el día anterior). */
export const DAY_START_HOUR = 7;
const DAY_SHIFT = DAY_START_HOUR * 3600000;

/** Día local (días desde 1970-01-01 en la hora de este equipo, con el día empezando a las 7:00) de una marca real en ms. */
export const localDay = (ms: number) => Math.floor((ms - new Date(ms).getTimezoneOffset() * 60000 - DAY_SHIFT) / 864e5);

/**
 * ¿Un día con `min` minutos cumple la meta de `goal`? Se compara con los minutos REDONDEADOS, como se enseñan
 * (59,85 min se ven como «1 h» y cuentan): unos segundos de menos no deben romper la racha.
 */
export const metGoal = (min: number, goal: number) => goal > 0 && Math.round(min) >= goal;
/** Minutos que faltan para que `metGoal` se cumpla (0 si ya se cumple). */
export const goalMinLeft = (min: number, goal: number) => (metGoal(min, goal) ? 0 : goal - 0.5 - min);
/** Parte de la meta cumplida (0..1): llega a 1 en cuanto `metGoal` se cumple. */
export const goalFrac = (min: number, goal: number) => (goal <= 0 ? 0 : metGoal(min, goal) ? 1 : Math.max(0, Math.min(1, min / goal)));

/** Momento real (ms) en que empieza el día local `d` (a las 7:00). */
export const dayStart = (d: number) => d * 864e5 + DAY_SHIFT + new Date(d * 864e5 + DAY_SHIFT + 12 * 3600000).getTimezoneOffset() * 60000;

/** Reparte una partida `[a, b)` entre los días locales que toca (las que cruzan las 7:00 cuentan en los dos). */
function addSpan(mins: Map<number, number>, a: number, b: number) {
  while (a < b) {
    const d = localDay(a);
    const end = Math.min(b, (d + 1) * 864e5 + DAY_SHIFT + new Date(a).getTimezoneOffset() * 60000);
    if (end <= a) break;
    mins.set(d, (mins.get(d) ?? 0) + (end - a) / 60000);
    a = end;
  }
}

type Live = { id: string; since: number } | null;

/** Dos partidas del mismo juego separadas por menos de esto (ms) son una sola; el hueco cuenta como tiempo jugado. */
export const MERGE_GAP_MS = 120000;

/**
 * Ordena por inicio las partidas `[inicio, fin)` de un juego y une las que se solapan o dejan menos de `MERGE_GAP_MS`
 * entre una y otra (reinicios del juego, cortes del vigilante…): el tiempo no se duplica y el hueco cuenta.
 */
export function mergeSpans(spans: Array<[number, number]>, gap = MERGE_GAP_MS): Array<[number, number]> {
  const sorted = spans.filter(([a, b]) => Number.isFinite(a) && Number.isFinite(b) && b >= a).sort((p, q) => p[0] - q[0] || p[1] - q[1]);
  const out: Array<[number, number]> = [];
  for (const [a, b] of sorted) {
    const last = out[out.length - 1];
    if (last && a - last[1] < gap) last[1] = Math.max(last[1], b);
    else out.push([a, b]);
  }
  return out;
}

/** Recorre las partidas `[inicio, fin)` de los juegos de la categoría (la partida en curso, hasta `now`), ya unidas. */
function eachSpan(key: GoalKey, platforms: Platform[], live: Live, now: number, f: (t: Title, a: number, b: number) => void) {
  for (const p of platforms) {
    if (p.id !== key) continue;
    for (const t of p.titles) {
      const spans: Array<[number, number]> = (t.plays ?? []).map(([a, b]) => [a, b]);
      if (live && t.gameId === live.id) spans.push([live.since, Math.max(now, live.since)]);
      for (const [a, b] of mergeSpans(spans)) f(t, a, b);
    }
  }
}

/** Última vez que se jugó a algo de las plataformas `keys`: el mayor fin de partida, o `now` si la partida en curso es de ellas; 0 si nada. */
export function lastPlayAt(keys: GoalKey[], platforms: Platform[], live: Live, now: number): number {
  let best = 0;
  for (const p of platforms) {
    if (!(keys as string[]).includes(p.id)) continue;
    for (const t of p.titles) {
      if (live && t.gameId === live.id) return now;
      for (const pl of t.plays ?? []) if (pl[1] > best) best = pl[1];
    }
  }
  return best;
}

/**
 * Calcula un anillo a partir de las partidas de los juegos de la categoría `key`.
 * `live` es la partida en curso (si es de esa categoría cuenta hasta `now`). `nWeeks`: columnas del mapa.
 * `hist`: historial de metas `[díaDesde, minutos][]` para evaluar cada día con la meta que tenía entonces.
 */
export function goalTrack(key: GoalKey, goal: number, platforms: Platform[], live: Live, now: number, nWeeks = MAP_WEEKS, hist: GoalHist = []): GoalTrack {
  const mins = new Map<number, number>();
  // Minutos de cada día solo con partidas de `EXTRA_MIN_PLAY` min o más (para pagar días libres).
  const valid = new Map<number, number>();
  eachSpan(key, platforms, live, now, (_t, a, b) => {
    addSpan(mins, a, b);
    const per = new Map<number, number>();
    addSpan(per, a, b);
    if ((b - a) / 60000 >= EXTRA_MIN_PLAY) {
      for (const [d, v] of per) valid.set(d, (valid.get(d) ?? 0) + v);
    }
  });
  const today = localDay(now);
  const goalOf = (d: number) => goalAt(hist, d, goal);
  const metAt = (d: number) => metGoal(mins.get(d) ?? 0, goalOf(d));
  const goalToday = goalOf(today);
  const nextGoal = goalOf(today + 1);
  const todayMin = mins.get(today) ?? 0;
  const done = metGoal(todayMin, goalToday);

  // Reglas 2–5: días libres (máx. 2 en 7 días), deuda y reparto al más antiguo, y punto fijo para plazos vencidos.
  const firstDay = mins.size ? Math.min(...mins.keys()) : today;
  const startDay = Math.min(firstDay, today);
  const forcedBroken = new Set<number>();
  let freeAll: FreeDay[] = [];
  let freeMap = new Map<number, FreeDay>();
  let kept = new Set<number>();
  let brokenSet = new Set<number>();
  let extraMap = new Map<number, number>();
  let gaveMap = new Map<number, number>();
  let bonusHeld = false;

  while (true) {
    freeAll = [];
    freeMap = new Map<number, FreeDay>();
    kept = new Set<number>();
    brokenSet = new Set<number>();
    extraMap = new Map<number, number>();
    gaveMap = new Map<number, number>();
    // Día libre extra oculto: se gana al llegar a `BONUS_STREAK` días seguidos de racha (cumplidos o libres),
    // no se acumula (como mucho uno guardado) y el siguiente día libre lo gasta en vez de una ficha. Se pierde con la racha.
    bonusHeld = false;
    let runLen = 0;

    for (let d = startDay; d < today; d++) {
      const g = goalOf(d);
      const m = mins.get(d) ?? 0;
      if (g <= 0) {
        if (kept.has(d - 1)) brokenSet.add(d);
        runLen = 0;
        bonusHeld = false;
        continue;
      }
      if (metGoal(m, g)) {
        kept.add(d);
        if (++runLen % BONUS_STREAK === 0) bonusHeld = true;
        continue;
      }
      let winFree = 0;
      for (let k = d - FREE_WINDOW + 1; k <= d - 1; k++) {
        if (freeMap.has(k) && !freeMap.get(k)!.bonus) winFree++;
      }
      const useBonus = bonusHeld && kept.has(d - 1) && !forcedBroken.has(d);
      if (!forcedBroken.has(d) && kept.has(d - 1) && (useBonus || winFree < FREE_MAX)) {
        const played = Math.round(m);
        const need = g - played;
        const deadline = d + FREE_WINDOW;
        const f: FreeDay = {
          day: d,
          goal: g,
          played,
          need,
          paid: 0,
          owe: need,
          done: false,
          deadline,
          left: deadline - today + 1,
          closeAt: dayStart(deadline + 1),
          payDay: NaN,
          bonus: useBonus,
        };
        freeAll.push(f);
        freeMap.set(d, f);
        kept.add(d);
        if (useBonus) bonusHeld = false;
        if (++runLen % BONUS_STREAK === 0) bonusHeld = true;
      } else {
        if (kept.has(d - 1) || forcedBroken.has(d)) brokenSet.add(d);
        runLen = 0;
        bonusHeld = false;
      }
    }

    // Reparto del exceso (d+1..d+7, sin guardar por adelantado ni cruzar un día roto).
    let runStart = startDay;
    for (let w = startDay; w <= today; w++) {
      const gw = goalOf(w);
      const ex = gw > 0 ? Math.max(0, Math.round(valid.get(w) ?? 0) - gw) : 0;
      extraMap.set(w, ex);
      if (ex > 0 && !brokenSet.has(w)) {
        let rem = ex;
        let gave = 0;
        for (const f of freeAll) {
          if (f.done || f.day < runStart || w <= f.day || w > f.deadline) continue;
          const p = Math.min(rem, f.owe);
          if (p > 0) {
            f.paid += p;
            f.owe -= p;
            rem -= p;
            gave += p;
            if (f.owe <= 0) {
              f.done = true;
              f.payDay = w;
            }
          }
          if (rem <= 0) break;
        }
        if (gave > 0) gaveMap.set(w, gave);
      }
      if (w < today && !kept.has(w)) runStart = w + 1;
    }

    let added = false;
    for (const f of freeAll) {
      if (!f.done && today > f.deadline) {
        forcedBroken.add(f.day);
        added = true;
      }
    }
    if (!added) break;
  }

  // Racha actual: días seguidos cumplidos o libres hasta hoy (o hasta ayer mientras hoy siga abierto).
  const keptCurrent = (d: number) => (d === today ? done : d < today && kept.has(d));
  let streak = 0;
  for (let d = done ? today : today - 1; keptCurrent(d); d--) streak++;
  const streakFrom = (done ? today : today - 1) - streak + 1;
  const activeFrom = streak > 0 ? streakFrom : today + 1;

  let best = streak;
  let bestTo = streak ? streakFrom + streak - 1 : NaN;
  let run = 0;
  for (let d = startDay; d <= today; d++) {
    const k = d === today ? done : metAt(d) || (freeMap.has(d) && (freeMap.get(d)!.done || d >= activeFrom));
    if (k) {
      run++;
      if (run > best) {
        best = run;
        bestTo = d;
      }
    } else {
      run = 0;
    }
  }

  const week: WeekDay[] = [];
  for (let d = today - 6; d <= today; d++) {
    const g = goalOf(d);
    const m = mins.get(d) ?? 0;
    week.push({
      day: d,
      goal: g,
      min: m,
      met: metGoal(m, g),
      today: d === today,
      extra: extraMap.get(d) ?? (g > 0 ? Math.max(0, Math.round(valid.get(d) ?? 0) - g) : 0),
      gave: gaveMap.get(d) ?? 0,
      free: freeMap.get(d) ?? null,
      broken: brokenSet.has(d),
    });
  }

  const free = freeAll.filter((f) => f.deadline >= today && (f.done || f.day >= activeFrom));
  const open = free.filter((f) => !f.done);
  const owe = open.reduce((a, f) => a + f.owe, 0);
  const urgent = open.find((f) => f.left <= 2) ?? null;
  const spent = freeAll.filter((f) => !f.bonus && f.day >= today - 6 && f.day <= today - 1);
  const tokens = Math.max(0, FREE_MAX - spent.length);
  const tokenBack = spent.length ? spent[0].deadline : NaN;
  const state: StreakState = urgent ? 'urgent' : tokens === 0 && !bonusHeld ? 'tope' : open.length ? 'debt' : done ? 'lit' : 'ok';
  let paidDay: number | null = null;
  for (const f of free) if (f.payDay === today) paidDay = f.day;

  const openSet = new Set(open.map((f) => f.day));
  const freeCellSet = new Set(freeAll.filter((f) => f.done || f.day >= activeFrom).map((f) => f.day));

  // Mapa: nWeeks columnas de lunes a domingo; la última contiene hoy. 1970-01-01 fue jueves.
  const wd = (today + 3) % 7;
  const first = today - wd - (nWeeks - 1) * 7;
  const weeks: MapCell[][] = [];
  for (let w = 0; w < nWeeks; w++) {
    const col: MapCell[] = [];
    for (let i = 0; i < 7; i++) {
      const d = first + w * 7 + i;
      const g = goalOf(d);
      const m = mins.get(d) ?? 0;
      col.push({
        v: goalFrac(m, g),
        r: g > 0 ? m / g : 0,
        streak: streak > 0 && d >= streakFrom && d <= streakFrom + streak - 1,
        today: d === today,
        future: d > today,
        free: freeCellSet.has(d),
        owe: openSet.has(d),
      });
    }
    weeks.push(col);
  }
  return {
    key,
    goal: goalToday,
    nextGoal,
    today: todayMin,
    frac: goalFrac(todayMin, goalToday),
    done,
    streak,
    best,
    bestFrom: bestTo - best + 1,
    bestTo,
    weeks,
    week,
    free,
    open,
    owe,
    urgent,
    tokens,
    tokenBack,
    state,
    paidDay,
  };
}

/** Fecha de calendario (en UTC) de un día local: `getUTCDate()`, `getUTCMonth()`… dan el día de este equipo. */
export const dayDate = (d: number) => new Date(d * 864e5);

export interface ReadDetail {
  /** Hora (ms reales) a la que se cumplió hoy la meta; null si aún no. */
  doneAt: number | null;
  /** Mes en curso: días cumplidos y días transcurridos. */
  month: { met: number; elapsed: number };
  /** Minutos de media al día en los últimos 30 días (hoy incluido). */
  avg30: number;
  /** Juegos del mes en curso por minutos, de más a menos. */
  games: Array<{ t: Title; min: number }>;
  /** Minutos por hora del día en los últimos 30 días; el índice 0 es la hora a la que empieza el día (7:00). */
  hours: number[];
  /** Año en curso: minutos, días cumplidos, novelas distintas y partidas. */
  year: { min: number; met: number; titles: number; sessions: number };
}

/** Cifras de la pantalla de racha para una categoría. */
export function readDetail(key: GoalKey, goal: number, platforms: Platform[], live: Live, now: number): ReadDetail {
  const today = localDay(now);
  const td = dayDate(today);
  const y = td.getUTCFullYear();
  const monthFirst = today - (td.getUTCDate() - 1);
  const yearFirst = Math.round(Date.UTC(y, 0, 1) / 864e5);
  const from30 = today - 29;

  const mins = new Map<number, number>();
  const games = new Map<Title, number>();
  const yearTitles = new Set<Title>();
  const hours: number[] = Array(24).fill(0);
  const todaySpans: Array<[number, number]> = [];
  let sessions = 0;
  eachSpan(key, platforms, live, now, (t, a, b) => {
    if (b <= a) return;
    const per = new Map<number, number>();
    addSpan(per, a, b);
    for (const [d, v] of per) {
      mins.set(d, (mins.get(d) ?? 0) + v);
      if (d >= monthFirst) games.set(t, (games.get(t) ?? 0) + v);
      if (d >= yearFirst) yearTitles.add(t);
    }
    if (localDay(b - 1) >= yearFirst) sessions++;
    if (localDay(b - 1) === today || localDay(a) === today) todaySpans.push([a, b]);
    // Horas del día: se reparte la partida por horas de reloj.
    if (localDay(b - 1) >= from30) {
      for (let x = Math.max(a, (from30 * 864e5) + DAY_SHIFT + new Date(a).getTimezoneOffset() * 60000); x < b;) {
        const h = new Date(x);
        const next = new Date(h); next.setMinutes(60, 0, 0);
        const end = Math.min(b, next.getTime());
        hours[(h.getHours() - DAY_START_HOUR + 24) % 24] += (end - x) / 60000;
        x = end;
      }
    }
  });

  // Hora a la que se llegó a la meta hoy: se suman las partidas de hoy en orden.
  let doneAt: number | null = null;
  if (metGoal(mins.get(today) ?? 0, goal)) {
    let acc = 0;
    for (const [a, b] of todaySpans.sort((p, q) => p[0] - q[0])) {
      const one = new Map<number, number>();
      addSpan(one, a, b);
      const v = one.get(today) ?? 0;
      if (metGoal(acc + v, goal)) {
        const start = localDay(a) === today ? a : b - v * 60000;
        doneAt = Math.min(b, start + Math.max(0, goalMinLeft(acc, goal)) * 60000);
        break;
      }
      acc += v;
    }
  }

  let met = 0, yMin = 0, yMet = 0, sum30 = 0;
  for (const [d, v] of mins) {
    if (d >= monthFirst && d <= today && metGoal(v, goal)) met++;
    if (d >= yearFirst && d <= today) { yMin += v; if (metGoal(v, goal)) yMet++; }
    if (d >= from30 && d <= today) sum30 += v;
  }
  return {
    doneAt,
    month: { met, elapsed: today - monthFirst + 1 },
    avg30: sum30 / 30,
    games: [...games].map(([t, min]) => ({ t, min })).filter((g) => g.min >= 1).sort((p, q) => q.min - p.min),
    hours,
    year: { min: yMin, met: yMet, titles: yearTitles.size, sessions },
  };
}

/* La racha cumplida se «enciende» con una animación una vez al día por anillo (en Inicio o en la pantalla de racha). */
const LIT_KEY = (k: GoalKey) => `visor.goalLit.${k}`;
/** La racha de hoy de este anillo ya se encendió. */
export function goalLit(k: GoalKey, day: number) {
  try { return Number(localStorage.getItem(LIT_KEY(k)) ?? 0) === day; } catch { return false; }
}
export function markGoalLit(k: GoalKey, day: number) {
  try { localStorage.setItem(LIT_KEY(k), String(day)); } catch { /* sin almacenamiento */ }
}
