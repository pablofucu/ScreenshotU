import { GOAL_COL, GOAL_KEYS, type GoalKey } from './readgoals';
// Tono de cada chip de Estadísticas: pareja de colores del arcoíris (índice k) y su variante oscura para texto.
const ARC = ['rojo', 'naranja', 'amarillo', 'verde', 'azul', 'anil', 'violeta'];
const HUE = [20, 52, 88, 146, 232, 268, 308];

/** Variables CSS del tono `k` (0-6, se envuelve): --v1/--v2/--gv para rellenos, --f1/--f2/--gf para texto y --g1/--g2 para
 * el marco de foco (así el cristal enfocado sale del color del recuadro y no del tono de la sección, que podía dar naranja). */
export function toneStyle(k: Tone): string {
  if (typeof k === 'string') {
    const g = GOAL_COL[k as GoalKey];
    return `--v1:${g.c};--v2:${g.c};--g1:${g.c};--g2:${g.edge};--gv:linear-gradient(115deg,${g.c},${g.edge});--f1:${g.ink};--f2:${g.ink};--gf:${g.ink}`;
  }
  const a = ((k % 7) + 7) % 7, b = (a + 1) % 7, f = (h: number) => `oklch(0.52 0.15 ${h})`;
  return `--v1:var(--arc-${ARC[a]});--v2:var(--arc-${ARC[b]});--g1:var(--arc-${ARC[a]});--g2:var(--arc-${ARC[b]});--gv:linear-gradient(115deg,var(--arc-${ARC[a]}),var(--arc-${ARC[b]}));` +
    `--f1:${f(HUE[a])};--f2:${f(HUE[b])};--gf:linear-gradient(115deg,${f(HUE[a])},${f(HUE[b])})`;
}

/** Tono fijo de cada categoría: el mismo color que su racha (Novelas visuales azul, Novelas japonesas rosa,
 * Juegos de PC verde, Switch naranja). Las demás plataformas reparten tonos del arcoíris por su id. */
export type Tone = number | GoalKey;
export function catTone(id: string): Tone {
  if (id in GOAL_COL) return id as GoalKey;
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return [1, 3, 5][h % 3];
}
/** Color de relleno de la categoría (para puntos, barras y carriles). */
export const catColor = (id: string): string => {
  const t = catTone(id);
  return typeof t === 'string' ? GOAL_COL[t].c : `var(--arc-${ARC[t % 7]})`;
};
/** Orden fijo de las categorías: Novelas visuales, Novelas japonesas, Juegos de PC, Switch y luego las demás. */
export const catRank = (id: string): number => { const i = GOAL_KEYS.indexOf(id as GoalKey); return i < 0 ? 99 : i; };

/** Icono de cada categoría (nombre de `CIcon`). */
export const catIcon = (id: string) => (id === 'pc' ? 'pad' : id === 'switch' ? 'switch' : id === 'vn' || id === 'jvn' ? 'book' : 'grid');
