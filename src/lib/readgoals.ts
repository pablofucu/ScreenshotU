// Anillos de lectura de Inicio: minutos leídos cada día en las novelas visuales y en las japonesas,
// la meta diaria de cada una, la racha de días cumplidos y el mapa de las últimas semanas.
import type { Platform, Title } from './types';

export type GoalKey = 'vn' | 'jvn';
/** Colores de cada anillo: azul = novelas visuales, rosa = novelas japonesas. */
export const GOAL_COL: Record<GoalKey, { c: string; soft: string; ink: string; edge: string; name: string; full: string }> = {
  vn: { c: '#00A0D8', soft: '#cfe6f1', ink: '#00739c', edge: '#0073a0', name: 'novelas', full: 'Novelas visuales' },
  jvn: { c: '#E5487A', soft: '#f6d3df', ink: '#a82b58', edge: '#b02c5c', name: 'japonés', full: 'Novelas japonesas' },
};
/** Colores del rescate de racha de cada categoría (nunca el de la categoría): ámbar para novelas, violeta para japonés. */
export const RESCUE_COL: Record<GoalKey, { rc: string; rsoft: string; rink: string; rmid: string }> = {
  vn: { rc: '#E9A100', rsoft: '#fcefc7', rink: '#8a5d00', rmid: '#f1dca0' },
  jvn: { rc: '#6E56CF', rsoft: '#e8e3fb', rink: '#4a3aa0', rmid: '#cdc3f2' },
};
/** Clase global (`.r-vn` / `.r-jv`, en app.css) que da las variables --rc, --rsoft, --rink y --rmid del rescate. */
export const rescueCls = (k: GoalKey) => (k === 'vn' ? 'r-vn' : 'r-jv');
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
  /** Día fallado y recuperado con un rescate pagado (parche del color del rescate). */
  rescued: boolean;
}

/** Solo las partidas de al menos esto (minutos) cuentan como extra para pagar un rescate. */
export const RESCUE_MIN_PLAY = 10;
/** Un día fallado es rescatable si tiene al menos esta parte de la meta, tras esta racha previa mínima (días seguidos). */
export const RESCUE_MIN_FRAC = 0.5;
export const RESCUE_PRIOR_DAYS = 3;
/** Días de la ventana tras el día fallado; máximo de rescates pagados en cualquier periodo móvil de `RESCUE_PERIOD` días. */
export const RESCUE_WINDOW = 3;
export const RESCUE_MAX = 2;
export const RESCUE_PERIOD = 30;
/** Un segundo rescate dentro del periodo exige este múltiplo del déficit. */
export const RESCUE_REPEAT_MULT = 1.5;

/** Partida (unida) dentro de la ventana de un rescate. */
export interface RescuePlay {
  /** Día local de la partida (o del trozo de partida si cruza las 7:00). */
  day: number;
  name: string;
  /** Minutos de la partida ese día. */
  min: number;
  /** Cuenta para pagar el rescate (la partida entera dura `RESCUE_MIN_PLAY` min o más). */
  counts: boolean;
  /** Inicio real (ms), para ordenar. */
  at: number;
}
/** Un rescate pendiente: el día fallado `day` se recupera leyendo de más en los 3 días siguientes. */
export interface Rescue {
  key: GoalKey;
  /** Día fallado (D). */
  day: number;
  goal: number;
  /** Minutos leídos ese día (redondeados) y lo que faltó. */
  played: number;
  deficit: number;
  /** Extra que hay que reunir (el déficit, o ×1,5 si es el segundo rescate en 30 días). */
  need: number;
  repeat: boolean;
  /** Extra reunido (limitado a `need`), repartido en lo de días anteriores a hoy y lo de hoy. */
  got: number;
  gotBefore: number;
  gotToday: number;
  /** Rescates (incluido este) en el periodo de 30 días, de un máximo de `RESCUE_MAX`. */
  used: number;
  /** Ventana: primer y último día (D+1 a D+3). */
  from: number;
  to: number;
  /** Hoy es el último día de la ventana. */
  last: boolean;
  /** Momento real (ms) en que cierra la ventana: las 7:00 del día siguiente a `to`. */
  closeAt: number;
  /** Minutos que haría falta leer el próximo día (su meta más lo que falte); en el último día, lo que falta hoy. */
  next: number;
  /** Partidas de la ventana hasta hoy, de la más antigua a la más reciente. */
  plays: RescuePlay[];
}
export interface GoalTrack {
  key: GoalKey;
  goal: number;
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
  /** Rescate pendiente de esta categoría (la racha se ve «pendiente de recuperar»); null si no hay. */
  rescue: Rescue | null;
  /** Día fallado cuyo rescate se acaba de pagar hoy (para el aviso «¡Hucha llena!»); null si no. */
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
export const goalFrac = (min: number, goal: number) => (metGoal(min, goal) ? 1 : Math.max(0, Math.min(1, min / goal)));

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

/**
 * Calcula un anillo a partir de las partidas de los juegos de la categoría `key`.
 * `live` es la partida en curso (si es de esa categoría cuenta hasta `now`). `nWeeks`: columnas del mapa.
 */
export function goalTrack(key: GoalKey, goal: number, platforms: Platform[], live: Live, now: number, nWeeks = MAP_WEEKS): GoalTrack {
  const mins = new Map<number, number>();
  // Para el rescate: minutos de cada día solo con partidas de `RESCUE_MIN_PLAY` min o más, y las partidas por trozo de día.
  const valid = new Map<number, number>();
  const pieces: RescuePlay[] = [];
  eachSpan(key, platforms, live, now, (t, a, b) => {
    addSpan(mins, a, b);
    const per = new Map<number, number>();
    addSpan(per, a, b);
    const counts = (b - a) / 60000 >= RESCUE_MIN_PLAY;
    for (const [d, v] of per) {
      if (counts) valid.set(d, (valid.get(d) ?? 0) + v);
      pieces.push({ day: d, name: t.name, min: v, counts, at: a });
    }
  });
  const met = (d: number) => metGoal(mins.get(d) ?? 0, goal);
  const today = localDay(now);

  // Rescates (retroactivo, sobre todo el historial): un día fallado D con 3 días cumplidos antes, al menos la mitad
  // de la meta, y sin 2 rescates pagados en los 30 días anteriores, se recupera leyendo de más en D+1..D+3.
  const repaired = new Set<number>();
  const paidDays: number[] = [];
  let rescue: Rescue | null = null;
  let paidDay: number | null = null;
  if (goal > 0) {
    const cands = [...mins.keys()].filter((d) => d < today && !met(d) && Math.round(mins.get(d)!) >= goal * RESCUE_MIN_FRAC
      &&Array.from({ length: RESCUE_PRIOR_DAYS }, (_, i) => met(d - 1 - i)).every(Boolean)).sort((p, q) => p - q);
    for (const D of cands) {
      const recent = paidDays.filter((p) => D - p <= RESCUE_PERIOD).length;
      if (recent >= RESCUE_MAX) continue;
      const played = Math.round(mins.get(D)!);
      const deficit = goal - played;
      const repeat = recent >= 1;
      const need = Math.ceil(deficit * (repeat ? RESCUE_REPEAT_MULT : 1) - 1e-9);
      let sum = 0, before = 0, tod = 0, state: 'pending' | 'paid' | 'failed' = 'pending', payDay = NaN;
      for (let w = D + 1; w <= D + RESCUE_WINDOW && w <= today; w++) {
        if (w < today && !met(w)) { state = 'failed'; break; }
        const ex = Math.max(0, (valid.get(w) ?? 0) - goal);
        sum += ex;
        if (w === today) tod = ex; else before += ex;
        if (Math.round(sum) >= need) { state = 'paid'; payDay = w; break; }
      }
      if (state === 'pending' && today > D + RESCUE_WINDOW) state = 'failed';
      if (state === 'paid') {
        paidDays.push(D);
        repaired.add(D);
        if (payDay === today) paidDay = D;
      } else if (state === 'pending') {
        const got = Math.min(need, Math.round(sum));
        const gotBefore = Math.min(got, Math.round(before));
        const to = D + RESCUE_WINDOW;
        rescue = {
          key, day: D, goal, played, deficit, need, repeat, got, gotBefore, gotToday: got - gotBefore,
          used: recent + 1, from: D + 1, to, last: today >= to, closeAt: dayStart(to + 1),
          next: today >= to ? need - got : goal + need - got,
          plays: pieces.filter((p) => p.day > D && p.day <= Math.min(to, today)).sort((p, q) => p.at - q.at),
        };
      }
    }
  }
  // El día fallado cuyo rescate sigue abierto cuenta provisionalmente: la racha se ve «pendiente», no rota.
  const kept = (d: number) => met(d) || repaired.has(d) || d === rescue?.day;
  const done = met(today);

  // Racha: días seguidos cumplidos hasta hoy (o hasta ayer mientras hoy siga abierto).
  let streak = 0;
  for (let d = done ? today : today - 1; kept(d); d--) streak++;
  const streakFrom = (done ? today : today - 1) - streak + 1;

  let best = streak, bestTo = streak ? streakFrom + streak - 1 : NaN, run = 0, prev = NaN;
  for (const d of [...mins.keys()].filter(kept).sort((a, b) => a - b)) {
    run = d === prev + 1 ? run + 1 : 1;
    prev = d;
    if (run > best) { best = run; bestTo = d; }
  }

  // Mapa: nWeeks columnas de lunes a domingo; la última contiene hoy. 1970-01-01 fue jueves.
  const wd = (today + 3) % 7;
  const first = today - wd - (nWeeks - 1) * 7;
  const weeks: MapCell[][] = [];
  for (let w = 0; w < nWeeks; w++) {
    const col: MapCell[] = [];
    for (let i = 0; i < 7; i++) {
      const d = first + w * 7 + i;
      col.push({ v: goalFrac(mins.get(d) ?? 0, goal), r: goal > 0 ? (mins.get(d) ?? 0) / goal : 0, streak: streak > 0 && d >= streakFrom && d <= streakFrom + streak - 1, today: d === today, future: d > today, rescued: repaired.has(d) });
    }
    weeks.push(col);
  }
  return { key, goal, today: mins.get(today) ?? 0, frac: goalFrac(mins.get(today) ?? 0, goal), done, streak, best, bestFrom: bestTo - best + 1, bestTo, weeks, rescue, paidDay };
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

/* El aviso «¡Hucha llena!» de un rescate pagado se da una sola vez: se guarda el día fallado ya avisado por categoría. */
const RESCUE_KEY = (k: GoalKey) => `visor.rescueSeen.${k}`;
export function rescueSeen(k: GoalKey, day: number) {
  try { return Number(localStorage.getItem(RESCUE_KEY(k)) ?? 0) === day; } catch { return false; }
}
export function markRescueSeen(k: GoalKey, day: number) {
  try { localStorage.setItem(RESCUE_KEY(k), String(day)); } catch { /* sin almacenamiento */ }
}
