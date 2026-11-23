export const PENTATONIC = [0, 2, 4, 7, 9, 12, 14] as const;

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));
const round1 = (v: number) => {
  const r = Math.round(v * 10) / 10;
  return r === 0 ? 0 : r;
};

/** Frecuencia cristalina al mover el foco: sube suavemente por la escala pentatónica en ráfagas rápidas. */
export function moveTone({ dx = 0, dy = 0, streak = 0 }: { dx?: number; dy?: number; streak?: number } = {}): number {
  const idx = clamp(Math.floor(streak), 0, PENTATONIC.length - 1);
  let semis = streak > 0 ? PENTATONIC[idx] * 0.55 : 0;
  if (dx > 0 || dy > 0) semis += 0.5;
  else if (dx < 0 || dy < 0) semis -= 0.7;
  return Math.round(784 * 2 ** (semis / 12));
}

/** Paneo estéreo sutil en [-0.28, 0.28] según la posición horizontal del foco y la dirección. */
export function panFor(dx = 0, normX = 0.5): number {
  const nx = clamp(Number.isFinite(normX) ? normX : 0.5, 0, 1);
  const raw = (nx - 0.5) * 0.36 + Math.sign(dx) * 0.12;
  const p = clamp(raw, -0.28, 0.28);
  const r = Math.round(p * 1000) / 1000;
  return r === 0 ? 0 : r;
}

/** Desplazamiento de paralaje para las capas de fondo (.bg-ghosts y .bg-bokeh). */
export function parallaxFor(normX: number, normY: number, scrollY = 0): { gx: number; gy: number; bx: number; by: number } {
  const nx = clamp(Number.isFinite(normX) ? normX : 0.5, 0, 1) - 0.5;
  const ny = clamp(Number.isFinite(normY) ? normY : 0.5, 0, 1) - 0.5;
  const s = clamp((Number.isFinite(scrollY) ? scrollY : 0) / 900, -1, 1);
  return {
    gx: round1(-nx * 24),
    gy: round1(-ny * 14 - s * 6),
    bx: round1(-nx * 52),
    by: round1(-ny * 28 - s * 14),
  };
}
