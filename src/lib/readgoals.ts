// Anillos de lectura de Inicio: minutos leídos cada día en las novelas visuales y en las japonesas,
// la meta diaria de cada una, la racha de días cumplidos y el mapa de las últimas semanas.
import type { Platform } from './types';

export type GoalKey = 'vn' | 'jvn';
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

/**
 * Calcula un anillo a partir de las partidas de los juegos de la categoría `key`.
 * `live` es la partida en curso (si es de esa categoría cuenta hasta `now`).
 */
export function goalTrack(key: GoalKey, goal: number, platforms: Platform[], live: { id: string; since: number } | null, now: number): GoalTrack {
  const mins = new Map<number, number>();
  for (const p of platforms) {
    if (p.id !== key) continue;
    for (const t of p.titles) {
      for (const [a, b] of t.plays ?? []) addSpan(mins, a, b);
      if (live && t.gameId === live.id) addSpan(mins, live.since, now);
    }
  }
  const met = (d: number) => (mins.get(d) ?? 0) >= goal;
  const today = localDay(now);
  const done = met(today);

  // Racha: días seguidos cumplidos hasta hoy (o hasta ayer mientras hoy siga abierto).
  let streak = 0;
  for (let d = done ? today : today - 1; met(d); d--) streak++;
  const streakFrom = (done ? today : today - 1) - streak + 1;

  let best = streak, run = 0, prev = NaN;
  for (const d of [...mins.keys()].filter(met).sort((a, b) => a - b)) {
    run = d === prev + 1 ? run + 1 : 1;
    prev = d;
    if (run > best) best = run;
  }

  // Mapa: MAP_WEEKS columnas de lunes a domingo; la última contiene hoy. 1970-01-01 fue jueves.
  const wd = (today + 3) % 7;
  const first = today - wd - (MAP_WEEKS - 1) * 7;
  const weeks: MapCell[][] = [];
  for (let w = 0; w < MAP_WEEKS; w++) {
    const col: MapCell[] = [];
    for (let i = 0; i < 7; i++) {
      const d = first + w * 7 + i;
      col.push({ v: Math.min(1, (mins.get(d) ?? 0) / goal), streak: streak > 0 && d >= streakFrom && d <= streakFrom + streak - 1, today: d === today, future: d > today });
    }
    weeks.push(col);
  }
  return { key, goal, today: mins.get(today) ?? 0, done, streak, best, weeks };
}
