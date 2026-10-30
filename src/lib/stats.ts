// Estadísticas de juego a partir de las partidas (`Title.plays`, `[inicio, fin]` en ms reales).
// Todo se agrupa por días locales; una partida que cruza la medianoche reparte su tiempo entre los dos días
// y cuenta como partida del periodo en el que empezó.
import type { Library, Title } from './types';

const MONTHS = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const MONTHS_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
export const WDAYS = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];
const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

/* ---------- fechas locales ---------- */
export const dayStart = (ms: number) => { const d = new Date(ms); d.setHours(0, 0, 0, 0); return d.getTime(); };
/** Suma días de calendario (correcto con cambios de hora). */
export const addDays = (ms: number, n: number) => { const d = new Date(ms); d.setDate(d.getDate() + n); return d.getTime(); };
/** Lunes de la semana. */
export const weekStart = (ms: number) => { const d = new Date(dayStart(ms)); d.setDate(d.getDate() - ((d.getDay() + 6) % 7)); return d.getTime(); };
const sameYear = (a: number, b: number) => new Date(a).getFullYear() === new Date(b).getFullYear();
/** «5 oct», o «5 oct 2025» si no es de este año. */
const dm = (ms: number, now: number) => { const d = new Date(ms); return `${d.getDate()} ${MONTHS[d.getMonth()]}${sameYear(ms, now) ? '' : ' ' + d.getFullYear()}`; };
/** «sábado 10 de octubre» (con el año si no es este). */
export const fmtLongDay = (ms: number, now = Date.now()) => {
  const d = new Date(ms);
  const s = `${WDAYS[d.getDay()]} ${d.getDate()} de ${MONTHS_L[d.getMonth()]}`;
  return cap(sameYear(ms, now) ? s : `${s} de ${d.getFullYear()}`);
};
export const fmtClock = (ms: number) => { const d = new Date(ms); return `${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`; };
export const fmtDate = (ms: number) => { const d = new Date(ms); return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')}/${d.getFullYear()}`; };
/** Duración compacta para cifras grandes: «10 h 20», «45 min», «0 min». */
export const fmtHM = (ms: number) => {
  const m = Math.round(ms / 60000);
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')}`;
};

type Play = [number, number];
const overlap = (p: Play, from: number, to: number) => Math.max(0, Math.min(p[1], to) - Math.max(p[0], from));
/** Días locales (inicio) que toca una partida dentro de [from, to). */
function eachDay(p: Play, from: number, to: number, f: (day: number, ms: number) => void) {
  const a = Math.max(p[0], from), b = Math.min(p[1], to);
  for (let d = dayStart(a); d < b; d = addDays(d, 1)) {
    const ms = overlap(p, d, addDays(d, 1));
    if (ms > 0) f(d, ms);
  }
}

/** Juegos reales (sin ocultos) con su posición en la biblioteca. */
export interface GameRef { pi: number; ti: number; t: Title }
export function games(lib: Library): GameRef[] {
  const out: GameRef[] = [];
  lib.platforms.forEach((p, pi) => {
    p.titles.forEach((t, ti) => { if (!t.hidden && (t.plays?.length || t.playSeconds > 0)) out.push({ pi, ti, t }); });
  });
  return out;
}

/* ---------- periodos ---------- */
export type PeriodKind = 'week' | 'month' | 'year' | 'all';
/** Periodos que ofrece la clasificación (X los recorre). */
export const STATS_KINDS: PeriodKind[] = ['month', 'year', 'all'];
export const PERIOD_KINDS: Array<[PeriodKind, string]> = [['week', 'Semana'], ['month', 'Mes'], ['year', 'Año'], ['all', 'Siempre']];
export interface Range { from: number; to: number; label: string }

export function periodRange(kind: PeriodKind, off: number, now = Date.now()): Range {
  const d = new Date(now);
  if (kind === 'week') {
    const from = addDays(weekStart(now), 7 * off), to = addDays(from, 7);
    const label = off === 0 ? 'Esta semana' : off === -1 ? 'Semana pasada' : `${dm(from, now)} – ${dm(addDays(to, -1), now)}`;
    return { from, to, label };
  }
  if (kind === 'month') {
    const from = new Date(d.getFullYear(), d.getMonth() + off, 1).getTime();
    const to = new Date(d.getFullYear(), d.getMonth() + off + 1, 1).getTime();
    const f = new Date(from);
    return { from, to, label: `${cap(MONTHS_L[f.getMonth()])} ${f.getFullYear()}` };
  }
  if (kind === 'year') {
    const y = d.getFullYear() + off;
    return { from: new Date(y, 0, 1).getTime(), to: new Date(y + 1, 0, 1).getTime(), label: String(y) };
  }
  return { from: -Infinity, to: Infinity, label: 'Siempre' };
}
/** Nombre del periodo anterior para las comparaciones: «la semana pasada», «septiembre»… */
export function prevName(kind: PeriodKind, off: number, now = Date.now()) {
  if (kind === 'week') return off === 0 ? 'la semana pasada' : 'la semana anterior';
  if (kind === 'month') return MONTHS_L[new Date(periodRange('month', off - 1, now).from).getMonth()];
  if (kind === 'year') return periodRange('year', off - 1, now).label;
  return '';
}

export interface RankRow extends GameRef { ms: number; n: number; prevMs: number }
export interface PeriodStats {
  range: Range;
  rows: RankRow[];
  ms: number; n: number; days: number;
  prevMs: number; prevN: number;
  /** Hay datos antes de este periodo (para dejar retroceder). */
  older: boolean;
}

/** Índices de las categorías con algún juego jugado (las únicas entre las que se reparte la clasificación). */
export function playedPlatforms(lib: Library): number[] {
  return [...new Set(games(lib).map((g) => g.pi))].sort((a, b) => a - b);
}

/** Clasificación de UNA categoría (`pi`): nunca mezcla títulos de plataformas distintas. */
export function rankPeriod(lib: Library, kind: PeriodKind, off: number, now = Date.now(), pi = -1): PeriodStats {
  const range = periodRange(kind, off, now);
  const prev = kind === 'all' ? null : periodRange(kind, off - 1, now);
  const days = new Set<number>();
  const rows: RankRow[] = [];
  let first = Infinity, prevAll = 0, prevN = 0;
  for (const g of games(lib)) {
    if (g.pi !== pi) continue;
    const plays = (g.t.plays ?? []) as Play[];
    let ms = 0, n = 0, prevMs = 0;
    for (const p of plays) {
      first = Math.min(first, p[0]);
      ms += overlap(p, range.from, range.to);
      if (p[0] >= range.from && p[0] < range.to) n++;
      if (prev) {
        prevMs += overlap(p, prev.from, prev.to);
        if (p[0] >= prev.from && p[0] < prev.to) prevN++;
      }
      eachDay(p, Math.max(range.from, p[0]), Math.min(range.to, p[1]), (d) => days.add(d));
    }
    // «Siempre» incluye el tiempo del formato antiguo, guardado sin partidas.
    if (kind === 'all') { ms = Math.max(ms, g.t.playSeconds * 1000); n = Math.max(n, g.t.sessions); }
    prevAll += prevMs;
    if (ms > 0 || n > 0) rows.push({ ...g, ms, n, prevMs });
  }
  rows.sort((a, b) => b.ms - a.ms || b.n - a.n || a.t.name.localeCompare(b.t.name));
  return {
    range, rows,
    ms: rows.reduce((a, r) => a + r.ms, 0), n: rows.reduce((a, r) => a + r.n, 0), days: days.size,
    prevMs: prevAll, prevN,
    older: first < range.from && kind !== 'all',
  };
}

/* ---------- estadísticas de juego (por título y por categoría) ---------- */
export interface TitleStats {
  ms: number; sessions: number; avg: number; days: number;
  longest: { ms: number; at: number } | null;
  last: { ms: number; at: number } | null;
  first: number | null;
}

/** Cifras de un juego a partir de sus partidas (el tiempo del formato antiguo, sin partidas, también cuenta). */
export function titleStats(t: Title): TitleStats {
  const plays = (t.plays ?? []) as Play[];
  const days = new Set<number>();
  let sum = 0, longest: TitleStats['longest'] = null;
  for (const p of plays) {
    const ms = p[1] - p[0];
    sum += ms;
    if (!longest || ms > longest.ms) longest = { ms, at: p[0] };
    eachDay(p, -Infinity, Infinity, (d) => days.add(d));
  }
  const ms = Math.max(sum, t.playSeconds * 1000), sessions = Math.max(plays.length, t.sessions);
  const l = plays.at(-1);
  return {
    ms, sessions, avg: sessions ? ms / sessions : 0, days: days.size, longest,
    last: l ? { ms: l[1] - l[0], at: l[0] } : null,
    first: plays[0]?.[0] ?? t.firstPlayed ?? null,
  };
}

export type GMetric = 'time' | 'sessions' | 'longest' | 'avg';
export const GMETRICS: Array<[GMetric, string]> = [['time', 'Más jugados'], ['sessions', 'Más partidas'], ['longest', 'Sesión récord'], ['avg', 'Partida media']];
export interface GRow extends GameRef { v: number; ts: TitleStats }
export interface CategoryStats { rows: GRow[]; ms: number; sessions: number; games: number; longest: { ms: number; t: Title } | null }

/** Clasificación de UNA categoría por la métrica elegida: nunca mezcla títulos de categorías distintas. */
export function categoryStats(lib: Library, pi: number, metric: GMetric): CategoryStats {
  const rows: GRow[] = [];
  let ms = 0, sessions = 0, longest: CategoryStats['longest'] = null;
  for (const g of games(lib)) {
    if (g.pi !== pi) continue;
    const ts = titleStats(g.t);
    ms += ts.ms; sessions += ts.sessions;
    if (ts.longest && (!longest || ts.longest.ms > longest.ms)) longest = { ms: ts.longest.ms, t: g.t };
    const v = metric === 'time' ? ts.ms : metric === 'sessions' ? ts.sessions : metric === 'longest' ? (ts.longest?.ms ?? 0) : ts.avg;
    if (v > 0) rows.push({ ...g, v, ts });
  }
  rows.sort((a, b) => b.v - a.v || b.ts.ms - a.ts.ms || a.t.name.localeCompare(b.t.name));
  return { rows, ms, sessions, games: games(lib).filter((g) => g.pi === pi).length, longest };
}

/* ---------- calendario ---------- */
export const CAL_WEEKS = 26;
export interface CalDay { start: number; ms: number; future: boolean }
export interface Calendar {
  days: CalDay[];
  /** Índice de hoy en `days` (-1 si la ventana es anterior). */
  today: number;
  months: Array<{ label: string; ms: number }>;
  played: number; elapsed: number;
  curStreak: number;
  best: { n: number; from: number; to: number };
  fav: { wd: number; avg: number } | null;
  older: boolean;
}

/** Minutos por día local de todas las partidas. */
function dayTotals(lib: Library): Map<number, number> {
  const m = new Map<number, number>();
  for (const g of games(lib)) for (const p of (g.t.plays ?? []) as Play[]) eachDay(p, -Infinity, Infinity, (d, ms) => m.set(d, (m.get(d) ?? 0) + ms));
  return m;
}

export function calendar(lib: Library, off: number, now = Date.now()): Calendar {
  const tot = dayTotals(lib);
  const today = dayStart(now);
  const lastWeek = addDays(weekStart(now), 7 * CAL_WEEKS * off);
  const first = addDays(lastWeek, -7 * (CAL_WEEKS - 1));
  const days: CalDay[] = [];
  for (let i = 0; i < CAL_WEEKS * 7; i++) {
    const start = addDays(first, i);
    days.push({ start, ms: tot.get(start) ?? 0, future: start > today });
  }
  // Racha actual (global): días seguidos con partida hasta hoy (o hasta ayer si hoy aún no has jugado).
  let cur = 0;
  for (let d = tot.has(today) ? today : addDays(today, -1); tot.has(d); d = addDays(d, -1)) cur++;
  // Mejor racha, meses y día favorito dentro de la ventana.
  const best = { n: 0, from: 0, to: 0 };
  let run = 0;
  const wsum = Array(7).fill(0), wcnt = Array(7).fill(0);
  const months: Array<{ label: string; ms: number; key: number }> = [];
  let played = 0, elapsed = 0;
  for (const d of days) {
    if (d.future) break;
    elapsed++;
    const wd = new Date(d.start).getDay();
    wcnt[wd]++; wsum[wd] += d.ms;
    if (d.ms > 0) {
      played++; run++;
      if (run > best.n) { best.n = run; best.to = d.start; best.from = addDays(d.start, -(run - 1)); }
    } else run = 0;
    const dt = new Date(d.start), key = dt.getFullYear() * 12 + dt.getMonth();
    if (months.at(-1)?.key !== key) months.push({ key, ms: 0, label: MONTHS[dt.getMonth()] + (sameYear(d.start, now) ? '' : ` ${String(dt.getFullYear()).slice(2)}`) });
    months.at(-1)!.ms += d.ms;
  }
  let fav: Calendar['fav'] = null;
  for (let wd = 0; wd < 7; wd++) {
    const avg = wcnt[wd] ? wsum[wd] / wcnt[wd] : 0;
    if (avg > 0 && (!fav || avg > fav.avg)) fav = { wd, avg };
  }
  const firstPlay = Math.min(...[...tot.keys()]);
  return {
    days, today: days.findIndex((d) => d.start === today), months: months.map(({ label, ms }) => ({ label, ms })),
    played, elapsed, curStreak: cur, best, fav, older: firstPlay < first,
  };
}

export interface DayPlay extends GameRef { start: number; end: number; ms: number; shots: number }
/** Partidas que tocan un día, con las capturas hechas durante cada una. */
export function dayPlays(lib: Library, day: number): DayPlay[] {
  const to = addDays(day, 1);
  const out: DayPlay[] = [];
  for (const g of games(lib)) {
    for (const p of (g.t.plays ?? []) as Play[]) {
      const ms = overlap(p, day, to);
      if (ms <= 0) continue;
      // Las fechas de las capturas son la hora local codificada como UTC.
      const a = p[0] - new Date(p[0]).getTimezoneOffset() * 60000, b = p[1] - new Date(p[1]).getTimezoneOffset() * 60000;
      const shots = g.t.shots.reduce((n, s) => n + (s.date >= a && s.date <= b ? 1 : 0), 0);
      out.push({ ...g, start: p[0], end: p[1], ms, shots });
    }
  }
  return out.sort((a, b) => a.start - b.start);
}

/* ---------- evolución de un juego ---------- */
export type EvoKind = 'day' | 'week' | 'month' | 'year';
export const EVO_KINDS: Array<[EvoKind, string]> = [['day', 'Días'], ['week', 'Semanas'], ['month', 'Meses'], ['year', 'Años']];
export interface Bucket {
  from: number; to: number;
  /** Etiqueta bajo la barra y nombre largo del periodo. */
  label: string; long: string;
  ms: number; n: number; days: number;
  longest: number; longestAt: number;
}

function bucketRanges(kind: EvoKind, t: Title, now: number): Array<Omit<Bucket, 'ms' | 'n' | 'days' | 'longest' | 'longestAt'>> {
  const out = [];
  if (kind === 'day') {
    for (let i = 13; i >= 0; i--) {
      const from = addDays(dayStart(now), -i), d = new Date(from);
      out.push({ from, to: addDays(from, 1), label: `${WDAYS[d.getDay()].slice(0, 2)} ${d.getDate()}`, long: fmtLongDay(from, now) });
    }
  } else if (kind === 'week') {
    for (let i = 11; i >= 0; i--) {
      const from = addDays(weekStart(now), -7 * i), to = addDays(from, 7);
      out.push({ from, to, label: dm(from, now).replace(/ \d{4}$/, ''), long: i === 0 ? 'Esta semana' : `Semana del ${dm(from, now)} al ${dm(addDays(to, -1), now)}` });
    }
  } else if (kind === 'month') {
    const d = new Date(now);
    for (let i = 11; i >= 0; i--) {
      const from = new Date(d.getFullYear(), d.getMonth() - i, 1).getTime(), f = new Date(from);
      out.push({ from, to: new Date(f.getFullYear(), f.getMonth() + 1, 1).getTime(), label: MONTHS[f.getMonth()], long: `${cap(MONTHS_L[f.getMonth()])} ${f.getFullYear()}` });
    }
  } else {
    const y1 = new Date(now).getFullYear();
    const firstPlay = Math.min(now, ...(t.plays ?? []).map((p) => p[0]));
    const y0 = Math.max(y1 - 9, new Date(firstPlay).getFullYear());
    for (let y = y0; y <= y1; y++) out.push({ from: new Date(y, 0, 1).getTime(), to: new Date(y + 1, 0, 1).getTime(), label: String(y), long: String(y) });
  }
  return out;
}

export function evoBuckets(t: Title, kind: EvoKind, now = Date.now()): Bucket[] {
  return bucketRanges(kind, t, now).map((r) => {
    let ms = 0, n = 0, longest = 0, longestAt = 0;
    const days = new Set<number>();
    for (const p of (t.plays ?? []) as Play[]) {
      ms += overlap(p, r.from, r.to);
      eachDay(p, Math.max(r.from, p[0]), Math.min(r.to, p[1]), (d) => days.add(d));
      if (p[0] >= r.from && p[0] < r.to) {
        n++;
        if (p[1] - p[0] > longest) { longest = p[1] - p[0]; longestAt = p[0]; }
      }
    }
    return { ...r, ms, n, days: days.size, longest, longestAt };
  });
}

/* ---------- resumen anual ---------- */
export const MONTH_ABBR = MONTHS;
export const MONTH_NAMES = MONTHS_L;
export interface YearGame extends GameRef { ms: number; n: number }
export interface YearShot { pi: number; ti: number; t: Title; path: string; date: number; fav: boolean }
export interface YearTotals { year: number; ms: number; n: number; days: number; shots: number }
export interface YearReview extends YearTotals {
  games: YearGame[];
  /** Milisegundos por mes (0 = enero). */
  months: number[];
  bestMonth: number;
  streak: { n: number; from: number; to: number };
  /** Tiempo por día de la semana (0 = domingo) y el favorito (el de más tiempo total). */
  wdMs: number[];
  favWd: { wd: number; ms: number; days: number } | null;
  longest: (GameRef & { start: number; end: number; ms: number }) | null;
  /** Capturas del año por juego, de más a menos. */
  shotGames: Array<GameRef & { n: number }>;
  /** Favorita del año (la más reciente de las marcadas) o, si no hay, la más reciente. */
  shot: YearShot | null;
  /** Días del año transcurridos hasta hoy (o todos si ya terminó). */
  elapsed: number;
  prev: YearTotals | null;
}

/** Año local de una captura (su fecha es la hora local codificada como UTC). */
const shotYear = (date: number) => new Date(date).getUTCFullYear();

/** Años con alguna partida, de menor a mayor. */
export function playYears(lib: Library): number[] {
  const ys = new Set<number>();
  for (const g of games(lib)) {
    for (const p of (g.t.plays ?? []) as Play[]) {
      const a = new Date(p[0]).getFullYear(), b = new Date(p[1]).getFullYear();
      for (let y = a; y <= b; y++) ys.add(y);
    }
  }
  return [...ys].sort((a, b) => a - b);
}

function yearTotals(lib: Library, year: number): YearTotals {
  const from = new Date(year, 0, 1).getTime(), to = new Date(year + 1, 0, 1).getTime();
  const days = new Set<number>();
  let ms = 0, n = 0, shots = 0;
  lib.platforms.forEach((pl) => {
    for (const t of pl.titles) {
      if (t.hidden) continue;
      for (const p of (t.plays ?? []) as Play[]) {
        ms += overlap(p, from, to);
        if (p[0] >= from && p[0] < to) n++;
        eachDay(p, from, to, (d) => days.add(d));
      }
      for (const s of t.shots) if (shotYear(s.date) === year) shots++;
    }
  });
  return { year, ms, n, days: days.size, shots };
}

export function yearReview(lib: Library, year: number, favs: { has(p: string): boolean }, now = Date.now()): YearReview {
  const from = new Date(year, 0, 1).getTime(), to = new Date(year + 1, 0, 1).getTime();
  const tot = yearTotals(lib, year);
  const rows: YearGame[] = [];
  const months = Array(12).fill(0), wdMs = Array(7).fill(0);
  const dayset = new Map<number, number>();
  let longest: YearReview['longest'] = null;
  for (const g of games(lib)) {
    let ms = 0, n = 0;
    for (const p of (g.t.plays ?? []) as Play[]) {
      ms += overlap(p, from, to);
      if (p[0] >= from && p[0] < to) {
        n++;
        if (!longest || p[1] - p[0] > longest.ms) longest = { ...g, start: p[0], end: p[1], ms: p[1] - p[0] };
      }
      eachDay(p, from, to, (d, m) => {
        dayset.set(d, (dayset.get(d) ?? 0) + m);
        months[new Date(d).getMonth()] += m;
        wdMs[new Date(d).getDay()] += m;
      });
    }
    if (ms > 0 || n > 0) rows.push({ ...g, ms, n });
  }
  rows.sort((a, b) => b.ms - a.ms || b.n - a.n || a.t.name.localeCompare(b.t.name));
  // Mejor racha de días seguidos dentro del año.
  const streak = { n: 0, from: 0, to: 0 };
  let run = 0, last = -1;
  for (const d of [...dayset.keys()].sort((a, b) => a - b)) {
    run = last >= 0 && addDays(last, 1) === d ? run + 1 : 1;
    last = d;
    if (run > streak.n) { streak.n = run; streak.to = d; streak.from = addDays(d, -(run - 1)); }
  }
  let favWd: YearReview['favWd'] = null;
  for (let wd = 0; wd < 7; wd++) if (wdMs[wd] > 0 && (!favWd || wdMs[wd] > favWd.ms)) favWd = { wd, ms: wdMs[wd], days: 0 };
  if (favWd) for (const d of dayset.keys()) if (new Date(d).getDay() === favWd.wd) favWd.days++;
  const bestMonth = months.some((m) => m > 0) ? months.indexOf(Math.max(...months)) : -1;
  // Capturas del año.
  const shotGames: YearReview['shotGames'] = [];
  let shot: YearShot | null = null, shotFav = false;
  lib.platforms.forEach((pl, pi) => {
    pl.titles.forEach((t, ti) => {
      if (t.hidden) return;
      let n = 0;
      for (const s of t.shots) {
        if (shotYear(s.date) !== year) continue;
        n++;
        const fav = favs.has(s.path);
        if (!shot || (fav && !shotFav) || (fav === shotFav && s.date > shot.date)) { shot = { pi, ti, t, path: s.path, date: s.date, fav }; shotFav = fav; }
      }
      if (n) shotGames.push({ pi, ti, t, n });
    });
  });
  shotGames.sort((a, b) => b.n - a.n || a.t.name.localeCompare(b.t.name));
  const ys = playYears(lib), py = ys.filter((y) => y < year).at(-1);
  const today = dayStart(now);
  const elapsed = Math.max(1, Math.round((Math.min(to, addDays(today, 1)) - from) / 864e5));
  return {
    ...tot, games: rows, months, bestMonth, streak, wdMs, favWd, longest, shotGames, shot, elapsed,
    prev: py === undefined ? null : yearTotals(lib, py),
  };
}

/** Iniciales para la miniatura de respaldo: «Hollow Knight» → «HK». */
export const initials = (name: string) =>
  name.split(/[\s:\-–—_]+/).filter((w) => /^[\p{L}\p{N}]/u.test(w)).slice(0, 2).map((w) => w[0].toUpperCase()).join('') || '?';
