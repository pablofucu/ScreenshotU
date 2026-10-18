// Posiciones de las miniaturas de la cuadrícula de capturas. Hay dos variantes:
// - continua: 4 columnas, como siempre;
// - por días: 5 columnas más pequeñas con una cabecera por cada día (solo con orden por fecha).
import type { Shot } from './types';
import { dayKey } from './format';

export interface DayHead {
  /** Posición vertical de la cabecera. */
  y: number;
  /** Fecha de la primera captura del día. */
  date: number;
  /** Índice de la primera captura del día y número de capturas. */
  first: number;
  n: number;
  /** Posición horizontal y columnas que ocupa (los días cortos comparten fila). */
  x: number;
  cols: number;
}
export interface ShotLayout {
  days: boolean;
  x: number[];
  y: number[];
  w: number;
  h: number;
  /** Altura de una fila (para la rueda del ratón). */
  rowStep: number;
  total: number;
  heads: DayHead[];
}

const GRID = { cols: 4, x0: 68, dx: 290, y0: 18, dy: 190, w: 266, h: 162 };
const DAYS = { cols: 5, x0: 68, dx: 224, y0: 8, dy: 134, w: 206, h: 116, head: 40, gap: 18 };

let memo: { shots: Shot[]; days: boolean; out: ShotLayout } | null = null;

export function shotLayout(shots: Shot[], days: boolean): ShotLayout {
  if (memo && memo.shots === shots && memo.days === days) return memo.out;
  const x: number[] = [], y: number[] = [];
  let out: ShotLayout;
  if (!days) {
    shots.forEach((_, i) => {
      x.push(GRID.x0 + (i % GRID.cols) * GRID.dx);
      y.push(GRID.y0 + Math.floor(i / GRID.cols) * GRID.dy);
    });
    out = { days, x, y, w: GRID.w, h: GRID.h, rowStep: GRID.dy, total: Math.ceil(shots.length / GRID.cols) * GRID.dy + 20, heads: [] };
  } else {
    // Los días que caben en lo que queda de una fila de un solo día la comparten; los largos empiezan línea.
    const heads: DayHead[] = [];
    let lineY = DAYS.y0, used = 0, i = 0;
    const closeLine = (rows: number) => { lineY += DAYS.head + rows * DAYS.dy + DAYS.gap; used = 0; };
    while (i < shots.length) {
      const k = dayKey(shots[i].date);
      let j = i;
      while (j < shots.length && dayKey(shots[j].date) === k) j++;
      const n = j - i;
      if (used > 0 && n > DAYS.cols - used) closeLine(1);
      const c0 = used;
      heads.push({ y: lineY, date: shots[i].date, first: i, n, x: DAYS.x0 + c0 * DAYS.dx, cols: Math.min(n, DAYS.cols - c0) });
      for (let m = 0; m < n; m++) {
        x.push(DAYS.x0 + (c0 + (m % DAYS.cols)) * DAYS.dx);
        y.push(lineY + DAYS.head + Math.floor(m / DAYS.cols) * DAYS.dy);
      }
      if (c0 === 0 && n >= DAYS.cols) closeLine(Math.ceil(n / DAYS.cols));
      else { used = c0 + n; if (used === DAYS.cols) closeLine(1); }
      i = j;
    }
    if (used > 0) closeLine(1);
    const cy = lineY;
    out = { days, x, y, w: DAYS.w, h: DAYS.h, rowStep: DAYS.dy, total: cy + 10, heads };
  }
  memo = { shots, days, out };
  return out;
}
