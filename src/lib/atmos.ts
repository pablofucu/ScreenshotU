import { lchToRgb, luminance, rgbToHex, hueOf, oklch } from './oklch';
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

/** Degradado del arcoíris completo (o del tramo a..b) en la dirección dada. */
export const rbGrad = (dir: string, a = 0, b = 1): string =>
  `linear-gradient(${dir},${[0, 1, 2, 3, 4, 5, 6].map((i) => oklch(0.74, 0.14, RH(a + ((b - a) * i) / 6))).join(',')})`;

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
  // Pasos cortos (de un juego al siguiente del catálogo por color) siempre por el camino corto, aunque crucen el rojo.
  if (inRB(a) && inRB(b) && hueDist(a, b) > 60) return (a + (b - a) * p + 360) % 360;
  const d = ((b - a + 540) % 360) - 180;
  return (a + d * p + 360) % 360;
}

export const bl = (h: number): string => oklch(0.80, 0.12, h);

/** Claridad y croma (OKLCH) de los siete colores del arcoíris, por tono. */
const VIVO: [number, number, number][] = [
  [19.3, 0.616, 0.21], [51.6, 0.705, 0.175], [85.4, 0.813, 0.166], [145.2, 0.683, 0.182],
  [232.4, 0.663, 0.136], [267.4, 0.547, 0.195], [306.7, 0.577, 0.21],
];
/** Acento vivo de un tono: el color puro del arcoíris (el amarillo es amarillo), interpolando
 * claridad y croma entre los vecinos. Solo para gráficos y adornos, nunca bajo texto. */
export function vivo(h: number): string {
  const hn = ((h % 360) + 360) % 360;
  for (let i = 0; i < VIVO.length; i++) {
    const a = VIVO[i], b = VIVO[(i + 1) % VIVO.length];
    const hb = b[0] < a[0] ? b[0] + 360 : b[0];
    const hx = hn < a[0] ? hn + 360 : hn;
    if (hx >= a[0] && hx <= hb) {
      const f = (hx - a[0]) / (hb - a[0]);
      return oklch(a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f, hn);
    }
  }
  return oklch(0.7, 0.18, hn);
}

export interface AtmosVariables {
  '--g1': string;
  '--g2': string;
  '--grad': string;
  '--f1': string;
  '--f2': string;
  '--gf': string;
  '--v1': string;
  '--v2': string;
  '--gv': string;
  '--on': string;
  '--b1': string;
  '--b2': string;
  '--b3': string;
  [key: string]: string;
}

/** Variables CSS para el tema claro a partir de dos tonos h1 y h2.
 * El texto sobre el acento siempre es blanco puro (#ffffff) para contraste uniforme. */
export function variablesCSS(h1: number, h2: number): AtmosVariables {
  const on = '#ffffff';
  const lumOn = 1.0;

  const relleno = (h: number) => {
    let l = 0.66;
    const hNorm = ((h % 360) + 360) % 360;
    while (l > 0.2 && l < 0.97) {
      const rgb = lchToRgb(l, 0.15, hNorm);
      const la = luminance(rgb);
      const c = (Math.max(la, lumOn) + 0.05) / (Math.min(la, lumOn) + 0.05);
      if (c >= 4.5) return rgbToHex(rgb);
      l -= 0.01;
    }
    return rgbToHex(lchToRgb(l, 0.15, hNorm));
  };

  const f1 = relleno(h1);
  const f2 = relleno(h2);
  const v1 = vivo(h1);
  const v2 = vivo(h2);
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
    '--v1': v1,
    '--v2': v2,
    '--gv': `linear-gradient(115deg,${v1},${v2})`,
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
