import { contrast, hueOf, oklch } from './oklch';
import { RAINBOW } from './icons';
import { hash } from './rng';
import type { Title } from './types';

export const INK = '#1f2530';

/** Tonos del arcoíris en orden monótono creciente (sumando 360 si decrece). */
export const RAINBOW_H: number[] = (() => {
  let prev = -1;
  return RAINBOW.map((r) => {
    let h = hueOf(r.c);
    while (h < prev) h += 360;
    prev = h;
    return h;
  });
})();

/** Interpolación lineal a lo largo del arcoíris ordenado para t ∈ [0, 1] (mod 360). */
export function RH(t: number): number {
  const clamped = Math.max(0, Math.min(1, t)) * (RAINBOW_H.length - 1);
  const i = Math.min(RAINBOW_H.length - 2, Math.floor(clamped));
  const f = clamped - i;
  return (RAINBOW_H[i] + (RAINBOW_H[i + 1] - RAINBOW_H[i]) * f) % 360;
}

export const inRB = (h: number): boolean => h >= RAINBOW_H[0] - 0.5 && h <= RAINBOW_H[RAINBOW_H.length - 1] + 0.5;

/** Pareja de tonos para una posición t ∈ [0, 1]. */
export function pair(t: number, d = 1 / 6): { h1: number; h2: number } {
  const h1 = RH(t);
  const h2 = RH(t + d <= 1 ? t + d : t - d);
  return { h1, h2 };
}

export const hueDist = (a: number, b: number): number => {
  const d = Math.abs(a - b) % 360;
  return d > 180 ? 360 - d : d;
};

/** Distancia angular más corta o deslizamiento a lo largo del arcoíris. */
export function lerpH(a: number, b: number, p: number): number {
  if (inRB(a) && inRB(b)) return (a + (b - a) * p + 360) % 360;
  const d = ((b - a + 540) % 360) - 180;
  return (a + d * p + 360) % 360;
}

export const bl = (h: number): string => oklch(0.80, 0.12, h);

export interface AtmosVariables {
  '--g1': string;
  '--g2': string;
  '--grad': string;
  '--f1': string;
  '--f2': string;
  '--gf': string;
  '--on': string;
  '--b1': string;
  '--b2': string;
  '--b3': string;
  [key: string]: string;
}

/** Variables CSS para el tema claro a partir de dos tonos h1 y h2. */
export function variablesCSS(h1: number, h2: number): AtmosVariables {
  const isYellowish = (h: number) => hueDist(h, 100) <= 20;
  const on = isYellowish(h1) || isYellowish(h2) ? INK : '#ffffff';

  const relleno = (h: number) => {
    let l = 0.66;
    const ok = () => contrast(oklch(l, 0.15, h), on) >= 4.5;
    while (!ok() && l > 0.2 && l < 0.97) {
      l += on === INK ? 0.01 : -0.01;
    }
    return oklch(l, 0.15, h);
  };

  const f1 = relleno(h1);
  const f2 = relleno(h2);
  const g1 = oklch(0.80, 0.12, h1);
  const g2 = oklch(0.80, 0.12, h2);
  const mid = lerpH(h1, h2, 0.5);
  const b1 = bl(h1);
  const b2 = bl(h2);
  const b3 = bl(mid);

  return {
    '--g1': g1,
    '--g2': g2,
    '--grad': `linear-gradient(115deg,${g1},${g2})`,
    '--f1': f1,
    '--f2': f2,
    '--gf': `linear-gradient(115deg,${f1},${f2})`,
    '--on': on,
    '--b1': b1,
    '--b2': b2,
    '--b3': b3,
  };
}

/** Pareja de tonos para un título específico. */
export function gamePair(t: Title, accentPreview?: string | null): { h1: number; h2: number } {
  let overrideColor: string | null = null;
  if (accentPreview) {
    if (accentPreview !== 'auto') overrideColor = accentPreview;
  } else if (t.accentOverride) {
    overrideColor = t.accentOverride;
  }

  if (overrideColor) {
    const h1 = hueOf(overrideColor);
    return { h1, h2: (h1 + 35) % 360 };
  }

  if (t.accent) {
    const h1 = hueOf(t.accent.accent.color);
    let h2: number | null = null;
    if (t.accent.palette) {
      for (const sw of t.accent.palette) {
        const hb = hueOf(sw.color);
        let d = ((hb - h1 + 540) % 360) - 180;
        if (Math.abs(d) >= 35) {
          if (Math.abs(d) > 60) d = Math.sign(d) * 60;
          h2 = (h1 + d + 360) % 360;
          break;
        }
      }
    }
    if (h2 === null) {
      h2 = (h1 + 35) % 360;
    }
    return { h1, h2 };
  }

  const k = ((hash(t.name) % 7) + 7) % 7;
  return pair(k / 6);
}
