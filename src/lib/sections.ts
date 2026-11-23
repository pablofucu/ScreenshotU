import { RAINBOW } from './icons.ts';
import { hash } from './rng.ts';

/** Identificadores de los tres espacios fijos a la derecha de las categorías en Inicio. */
export const SPACE_IDS = ['album', 'stats', 'logros'] as const;
export type SpaceId = (typeof SPACE_IDS)[number];

/** Orden visual de las categorías estándar en Inicio (coincide con el arranque del arcoíris). */
export const HOME_CAT_ORDER: Record<string, number> = {
  switch: 0,
  pc: 1,
  vn: 2,
  jvn: 3,
};

/** Ranura fija en el arcoíris de la guía (`RAINBOW` 0..6: Rojo → Violeta) para cada categoría y espacio. */
export const SECTION_RAINBOW: Record<string, number> = {
  switch: 0, // Rojo
  pc: 1,     // Naranja
  vn: 2,     // Amarillo
  jvn: 3,    // Verde
  album: 4,  // Azul Wii U
  stats: 5,  // Añil
  logros: 6, // Violeta
};

/** Índice (0..6 en `RAINBOW`) asignado de forma fija a una categoría o espacio; si es carpeta nueva, hash determinista. */
export function sectionRainbowIdx(id: string): number {
  if (Object.prototype.hasOwnProperty.call(SECTION_RAINBOW, id)) {
    return SECTION_RAINBOW[id];
  }
  return hash(id) % RAINBOW.length;
}

/** Posición normalizada (0..1) en el arcoíris de una categoría o espacio. */
export function sectionRainbowT(id: string): number {
  return sectionRainbowIdx(id) / (RAINBOW.length - 1);
}

/** Color hexadecimal fijo de la sección en `RAINBOW`. */
export function sectionColor(id: string): string {
  return RAINBOW[sectionRainbowIdx(id)].c;
}

/** Devuelve las plataformas en su orden visual de Inicio conservando el índice real `i` en `platforms`. */
export function orderedPlatforms<T extends { id: string }>(platforms: readonly T[]): Array<{ p: T; i: number }> {
  return platforms
    .map((p, i) => ({ p, i }))
    .sort((a, b) => (HOME_CAT_ORDER[a.p.id] ?? 99) - (HOME_CAT_ORDER[b.p.id] ?? 99) || a.i - b.i);
}

/** Índice real de la primera categoría visible en Inicio (`switch` si existe; si no `0`). */
export function firstHomePlat<T extends { id: string }>(platforms: readonly T[]): number {
  return orderedPlatforms(platforms)[0]?.i ?? 0;
}
