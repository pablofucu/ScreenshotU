// Colecciones: funciones puras para ordenar sus juegos por año, sacar sus colores, sugerir grupos y montar la vista de
// capturas combinada. No tocan el estado de la app (eso está en coleccion.svelte.ts) ni los archivos de capturas.
import type { Collection, Platform, Shot, Title } from './types';
import { hexToRgb, lchToRgb, rgbToHex, rgbToLch } from './oklch';

const EMPTY: Omit<Title, 'name' | 'cover' | 'shots' | 'id' | 'members'> = {
  hero: null, logo: null, folder: '', dir: '', hasFolder: false, gameId: null, source: null, launchable: false,
  removable: false, lastPlayed: null, playSeconds: 0, launcher: null, sessions: 0, firstPlayed: null,
  week: [0, 0, 0, 0, 0, 0, 0], plays: [], hidden: false, accent: null, accentOverride: null, sortMode: null,
};

/** Instante de la primera captura o la primera partida de un juego (null = ninguno de los dos). */
export function firstMs(t: Title): number | null {
  let m = t.firstPlayed ?? Infinity;
  for (const s of t.shots) if (s.date < m) m = s.date;
  return Number.isFinite(m) ? m : null;
}
export const gameYear = (t: Title): number | null => {
  const ms = firstMs(t);
  return ms === null ? null : new Date(ms).getFullYear();
};

/** Índices (en `p.titles`) de los juegos visibles de la colección, por año de menor a mayor (los sin año al final) y,
 *  a igualdad, en el orden en que están en la colección. Los juegos que ya no están en la categoría no se muestran. */
export function collItems(c: Collection, p: Platform): number[] {
  const at = new Map<string, number>();
  p.titles.forEach((t, i) => { if (!t.hidden && !t.members && t.id) at.set(t.id, i); });
  const out: Array<{ i: number; y: number; k: number }> = [];
  c.members.forEach((id, k) => {
    const i = at.get(id);
    if (i !== undefined) out.push({ i, y: gameYear(p.titles[i]) ?? 1e9, k });
  });
  return out.sort((a, b) => a.y - b.y || a.k - b.k).map((o) => o.i);
}

/** Vista de capturas de una colección: un título virtual cuyas capturas son la unión de las de sus juegos. */
export function collectionTitle(c: Collection, titles: Title[]): Title {
  const byId = new Map<string, Title>();
  for (const t of titles) if (!t.hidden && t.id) byId.set(t.id, t);
  const members = c.members.map((id) => byId.get(id)).filter((t): t is Title => !!t);
  const shots: Shot[] = members
    .flatMap((t) => t.shots.map((x) => ({ ...x, game: t.name, gid: t.id })))
    .sort((a, b) => b.date - a.date);
  const covers = members.map((t) => t.cover ?? t.shots[0]?.path ?? null);
  return {
    ...EMPTY, id: `coll:${c.id}`, name: c.name, cover: covers.find(Boolean) ?? null, shots,
    members: members.map((t, i) => ({ id: t.id, name: t.name, cover: covers[i] })),
  };
}

/** Color de acento (`#rrggbb`) de un juego: el manual o el sacado de su arte. */
export const titleHex = (t: Title): string | null => t.accentOverride ?? t.accent?.accent.color ?? null;

/** Color automático de una colección: el tono que más se repite entre los acentos de sus portadas (en cubos de 30°),
 *  con el acento más vivo de ese cubo. null = ninguna portada tiene color. */
export function autoHex(titles: Title[]): string | null {
  const bins = new Map<number, { w: number; hex: string; c: number }>();
  for (const t of titles) {
    const hex = titleHex(t);
    const rgb = hex ? hexToRgb(hex) : null;
    if (!hex || !rgb) continue;
    const [, c, h] = rgbToLch(rgb);
    if (c < 0.04) continue;
    const b = Math.floor((h % 360) / 30);
    const cur = bins.get(b);
    if (!cur) bins.set(b, { w: 1, hex, c });
    else { cur.w++; if (c > cur.c) { cur.hex = hex; cur.c = c; } }
  }
  let best: { w: number; hex: string } | null = null;
  for (const v of bins.values()) if (!best || v.w > best.w) best = v;
  return best ? best.hex : null;
}

/** Mismo tono que `hex`, pero con luz e intensidad fijas (OKLCH): así combinan aunque las portadas sean muy distintas. */
export function harmonize(hex: string, L: number, C: number): string {
  const rgb = hexToRgb(hex);
  if (!rgb) return hex;
  const [, , h] = rgbToLch(rgb);
  return rgbToHex(lchToRgb(L, C, h));
}

/** Colores armonizados de las portadas de una colección (en el orden de los juegos, sin repetir tonos casi iguales). */
export function coverColors(titles: Title[], L: number, C: number, max: number): string[] {
  const out: string[] = [];
  const hues: number[] = [];
  for (const t of titles) {
    const hex = titleHex(t);
    const rgb = hex ? hexToRgb(hex) : null;
    if (!hex || !rgb) continue;
    const [, c, h] = rgbToLch(rgb);
    if (c < 0.03) continue;
    if (hues.some((x) => Math.min(Math.abs(x - h), 360 - Math.abs(x - h)) < 14)) continue;
    hues.push(h);
    out.push(harmonize(hex, L, C));
    if (out.length >= max) break;
  }
  return out;
}

/** Prefijo común de los nombres (sin números ni signos finales): sugerencia de nombre de colección. */
export function suggestName(names: string[]): string {
  if (names.length < 2) return '';
  let p = names[0];
  for (const n of names.slice(1)) {
    let i = 0;
    while (i < p.length && i < n.length && p[i].toLowerCase() === n[i].toLowerCase()) i++;
    p = p.slice(0, i);
  }
  p = p.replace(/[\s:\-–—_.,0-9IVXivx]+$/, '').trim();
  return p.length >= 3 ? p : '';
}

const ARTICLES = new Set(['the', 'a', 'an', 'el', 'la', 'los', 'las', 'un', 'una', 'de', 'of']);
const wordsOf = (name: string) => name.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);

/** Grupos de juegos con un comienzo en común (Trails…, Kingdom Hearts…) entre los `titles` que se pasan. */
export function suggestGroups(titles: Title[]): Array<{ name: string; ids: string[] }> {
  const groups = new Map<string, Title[]>();
  for (const t of titles) {
    const w = wordsOf(t.name).filter((x, i, a) => !(i === 0 && ARTICLES.has(x) && a.length > 1));
    const key = w[0];
    if (!key || key.length < 3) continue;
    (groups.get(key) ?? groups.set(key, []).get(key)!).push(t);
  }
  const out: Array<{ name: string; ids: string[] }> = [];
  for (const [key, list] of groups) {
    if (list.length < 2) continue;
    const name = suggestName(list.map((t) => t.name)) || key.charAt(0).toUpperCase() + key.slice(1);
    out.push({ name, ids: list.map((t) => t.id) });
  }
  return out.sort((a, b) => b.ids.length - a.ids.length || a.name.localeCompare(b.name, 'es')).slice(0, 8);
}
