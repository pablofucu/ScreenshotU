// Cuadrícula de portadas de la lista de juegos: cuatro tamaños (5, 6, 7 u 8 por fila).
export const TZOOM_MAX = 3;
/** Por nivel: portadas por fila, ancho y alto de la portada, hueco horizontal y vertical. */
const LEVELS = [
  { cols: 5, cw: 160, ch: 214, gap: 44, rg: 16 },
  { cols: 6, cw: 132, ch: 177, gap: 34, rg: 14 },
  { cols: 7, cw: 106, ch: 142, gap: 32, rg: 12 },
  { cols: 8, cw: 92, ch: 123, gap: 28, rg: 12 },
];
/** Borde inferior útil de la cuadrícula (por encima de las pistas de botones). */
const BOTTOM = 656;

export interface TGrid {
  cols: number; rows: number; size: number;
  cw: number; ch: number; gap: number; rg: number;
  /** Primer x de la página y y de la primera fila. */
  gx: number; top: number;
  /** Escala respecto al tamaño base (para bordes y textos). */
  s: number;
}

/** Medidas de la cuadrícula para un nivel. */
export function tgrid(level: number): TGrid {
  const L = LEVELS[Math.max(0, Math.min(TZOOM_MAX, Math.round(level) || 0))];
  const top = 204;
  const rows = Math.max(1, Math.floor((BOTTOM - top + L.rg) / (L.ch + L.rg)));
  return { ...L, rows, size: L.cols * rows, top, gx: (1280 - L.cols * L.cw - (L.cols - 1) * L.gap) / 2, s: L.cw / 160 };
}
