// Anillos de lectura de Inicio: minutos leídos cada día en las novelas visuales y en las japonesas,
// la meta diaria de cada una, la racha de días cumplidos y el mapa de las últimas semanas.
import type { Platform, Title } from './types';

export type GoalKey = 'vn' | 'jvn';
/** Colores de cada anillo: azul = novelas visuales, rosa = novelas japonesas. */
export const GOAL_COL: Record<GoalKey, { c: string; soft: string; ink: string; edge: string; name: string; full: string }> = {
  vn: { c: '#00A0D8', soft: '#cfe6f1', ink: '#00739c', edge: '#0073a0', name: 'novelas', full: 'Novelas visuales' },
  jvn: { c: '#E5487A', soft: '#f6d3df', ink: '#a82b58', edge: '#b02c5c', name: 'japonés', full: 'Novelas japonesas' },
};
export const MAP_WEEKS = 10;

export interface MapCell {
  /** Parte de la meta cumplida ese día (0..1). */
  v: number;
  /** El día forma parte de la racha actual. */
  streak: boolean;
  today: boolean;
  future: boolean;
}
export interface GoalTrack {
  key: GoalKey;
  goal: number;
  /** Minutos de hoy. */
  today: number;
  done: boolean;
  streak: number;
  best: number;
  /** Días (locales) de la mejor racha; NaN si aún no hay ninguna. */
  bestFrom: number;
  bestTo: number;
  /** Semanas (lunes a domingo), la última es la actual. */
  weeks: MapCell[][];
}

/** Hora local a la que empieza el día de las metas (antes de esa hora todavía cuenta como el día anterior). */
export const DAY_START_HOUR = 7;
const DAY_SHIFT = DAY_START_HOUR * 3600000;

/** Día local (días desde 1970-01-01 en la hora de este equipo, con el día empezando a las 7:00) de una marca real en ms. */
export const localDay = (ms: number) => Math.floor((ms - new Date(ms).getTimezoneOffset() * 60000 - DAY_SHIFT) / 864e5);

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

/** Recorre las partidas `[inicio, fin)` de los juegos de la categoría (la partida en curso, hasta `now`). */
function eachSpan(key: GoalKey, platforms: Platform[], live: Live, now: number, f: (t: Title, a: number, b: number) => void) {
  for (const p of platforms) {
    if (p.id !== key) continue;
    for (const t of p.titles) {
      for (const [a, b] of t.plays ?? []) f(t, a, b);
      if (live && t.gameId === live.id) f(t, live.since, now);
    }
  }
}

/**
 * Calcula un anillo a partir de las partidas de los juegos de la categoría `key`.
 * `live` es la partida en curso (si es de esa categoría cuenta hasta `now`). `nWeeks`: columnas del mapa.
 */
export function goalTrack(key: GoalKey, goal: number, platforms: Platform[], live: Live, now: number, nWeeks = MAP_WEEKS): GoalTrack {
  const mins = new Map<number, number>();
  eachSpan(key, platforms, live, now, (_, a, b) => addSpan(mins, a, b));
  const met = (d: number) => (mins.get(d) ?? 0) >= goal;
  const today = localDay(now);
  const done = met(today);

  // Racha: días seguidos cumplidos hasta hoy (o hasta ayer mientras hoy siga abierto).
  let streak = 0;
  for (let d = done ? today : today - 1; met(d); d--) streak++;
  const streakFrom = (done ? today : today - 1) - streak + 1;

  let best = streak, bestTo = streak ? streakFrom + streak - 1 : NaN, run = 0, prev = NaN;
  for (const d of [...mins.keys()].filter(met).sort((a, b) => a - b)) {
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
      col.push({ v: Math.min(1, (mins.get(d) ?? 0) / goal), streak: streak > 0 && d >= streakFrom && d <= streakFrom + streak - 1, today: d === today, future: d > today });
    }
    weeks.push(col);
  }
  return { key, goal, today: mins.get(today) ?? 0, done, streak, best, bestFrom: bestTo - best + 1, bestTo, weeks };
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
  if ((mins.get(today) ?? 0) >= goal) {
    let acc = 0;
    for (const [a, b] of todaySpans.sort((p, q) => p[0] - q[0])) {
      const one = new Map<number, number>();
      addSpan(one, a, b);
      const v = one.get(today) ?? 0;
      if (acc + v >= goal) {
        const start = localDay(a) === today ? a : b - v * 60000;
        doneAt = start + (goal - acc) * 60000;
        break;
      }
      acc += v;
    }
  }

  let met = 0, yMin = 0, yMet = 0, sum30 = 0;
  for (const [d, v] of mins) {
    if (d >= monthFirst && d <= today && v >= goal) met++;
    if (d >= yearFirst && d <= today) { yMin += v; if (v >= goal) yMet++; }
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
