// Réplica en TypeScript de las matemáticas de color de `visor_core::palette` (OKLab/OKLCH).
// El núcleo en Rust es la fuente de verdad para los acentos extraídos del arte; esto sirve para el selector
// de tono personalizado, para decidir el texto sobre un color elegido a mano y para la paleta simulada del navegador.
export const L_MIN = 0.62, L_MAX = 0.72, C_MIN = 0.13, C_MAX = 0.2, WHITE_MIN_CONTRAST = 2.55;

const toLin = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const fromLin = (c: number) => (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055);

export function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const v = parseInt(m[1], 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}
export const rgbToHex = (r: number[]) => '#' + r.map((x) => x.toString(16).padStart(2, '0')).join('');

export function rgbToLch([r8, g8, b8]: number[]): [number, number, number] {
  const [r, g, b] = [toLin(r8 / 255), toLin(g8 / 255), toLin(b8 / 255)];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  const h = (Math.atan2(bb, a) * 180) / Math.PI;
  return [L, Math.hypot(a, bb), h < 0 ? h + 360 : h];
}

function lchToLin(L: number, C: number, hDeg: number): [number, number, number] {
  const h = (hDeg * Math.PI) / 180;
  const a = C * Math.cos(h), b = C * Math.sin(h);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
}
const inGamut = (v: number[]) => v.every((c) => c >= -0.0005 && c <= 1.0005);

export function maxChroma(L: number, h: number): number {
  let lo = 0, hi = 0.4;
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    if (inGamut(lchToLin(L, mid, h))) lo = mid; else hi = mid;
  }
  return lo;
}
export function lchToRgb(L: number, C: number, h: number): number[] {
  const c = Math.max(0, Math.min(C, maxChroma(L, h)));
  return lchToLin(L, c, h).map((v) => Math.round(fromLin(Math.max(0, Math.min(1, v))) * 255));
}

const luminance = (rgb: number[]) => 0.2126 * toLin(rgb[0] / 255) + 0.7152 * toLin(rgb[1] / 255) + 0.0722 * toLin(rgb[2] / 255);
export const contrastWithWhite = (rgb: number[]) => 1.05 / (luminance(rgb) + 0.05);
/** Texto oscuro sobre este color (poco contraste con el blanco). */
export const needsDarkInk = (rgb: number[]) => contrastWithWhite(rgb) < WHITE_MIN_CONTRAST;
export function inkDark(hex: string): boolean {
  const rgb = hexToRgb(hex);
  return rgb ? needsDarkInk(rgb) : false;
}

/** Lleva un color OKLCH al «peso» de los acentos del arcoíris (igual que `palette::normalize_lch`). */
export function normalizeLch(L: number, C: number, h: number): number[] {
  const l0 = Math.min(L_MAX, Math.max(L_MIN, L));
  const want = Math.min(C_MAX, Math.max(C_MIN, C));
  let best: [number, number] | null = null;
  let fallback: [number, number] = [0, l0];
  for (let i = 0; i <= 10; i++) {
    const ls = L_MIN + ((L_MAX - L_MIN) * i) / 10;
    const mc = maxChroma(ls, h);
    if (mc >= want - 0.002) {
      const d = Math.abs(ls - l0);
      if (!best || d < best[0]) best = [d, ls];
    }
    if (mc > fallback[0]) fallback = [mc, ls];
  }
  const lf = best ? best[1] : fallback[1];
  return lchToRgb(lf, Math.min(want, maxChroma(lf, h)), h);
}
export function normalizeHex(hex: string): { color: string; dark: boolean } | null {
  const rgb = hexToRgb(hex);
  if (!rgb) return null;
  const [l, c, h] = rgbToLch(rgb);
  const out = normalizeLch(l, c, h);
  return { color: rgbToHex(out), dark: needsDarkInk(out) };
}
/** Acento para un tono (selector personalizado). */
export function accentForHue(h: number): { color: string; dark: boolean } {
  const out = normalizeLch(0.68, 0.16, ((h % 360) + 360) % 360);
  return { color: rgbToHex(out), dark: needsDarkInk(out) };
}
export const hueOf = (hex: string): number => { const rgb = hexToRgb(hex); return rgb ? rgbToLch(rgb)[2] : 220; };

export function oklch(L: number, C: number, h: number): string {
  return rgbToHex(lchToRgb(L, C, ((h % 360) + 360) % 360));
}
export function contrast(hexA: string, hexB: string): number {
  const rgbA = hexToRgb(hexA);
  const rgbB = hexToRgb(hexB);
  if (!rgbA || !rgbB) return 1;
  const la = luminance(rgbA);
  const lb = luminance(rgbB);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}
