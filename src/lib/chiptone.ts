// Tono de cada chip de Estadísticas: pareja de colores del arcoíris (índice k) y su variante oscura para texto.
const ARC = ['rojo', 'naranja', 'amarillo', 'verde', 'azul', 'anil', 'violeta'];
const HUE = [20, 52, 88, 146, 232, 268, 308];

/** Variables CSS del tono `k` (0-6, se envuelve): --v1/--v2/--gv para rellenos y --f1/--f2/--gf para texto. */
export function toneStyle(k: number): string {
  const a = ((k % 7) + 7) % 7, b = (a + 1) % 7, f = (h: number) => `oklch(0.52 0.15 ${h})`;
  return `--v1:var(--arc-${ARC[a]});--v2:var(--arc-${ARC[b]});--gv:linear-gradient(115deg,var(--arc-${ARC[a]}),var(--arc-${ARC[b]}));` +
    `--f1:${f(HUE[a])};--f2:${f(HUE[b])};--gf:linear-gradient(115deg,${f(HUE[a])},${f(HUE[b])})`;
}

/** Tono fijo de cada categoría: no cambia entre semanas ni entre pestañas. Las demás plataformas reparten por su id. */
// Los cinco colores de los estados de Mi lista, bien separados: Switch rojo, PC azul, Novelas japonesas verde,
// Novelas visuales violeta; las demás reparten naranja, amarillo y añil por su id.
const CAT_TONE: Record<string, number> = { pc: 4, switch: 0, vn: 6, jvn: 3 };
export function catTone(id: string): number {
  if (id in CAT_TONE) return CAT_TONE[id];
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return [1, 2, 5][h % 3];
}

/** Icono de cada categoría (nombre de `CIcon`). */
export const catIcon = (id: string) => (id === 'pc' ? 'pad' : id === 'switch' ? 'switch' : id === 'vn' || id === 'jvn' ? 'book' : 'grid');
