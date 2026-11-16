// Contadores que suben de 0 al valor (800 ms, easeOutCubic) una sola vez por visita a cada chip.
import { calm } from './controller.svelte';

const seen = new Set<string>();
/** Olvida lo ya contado (al salir de la pantalla de Estadísticas). */
export const resetCounts = () => seen.clear();

/** Acción: `use:countUp={{ to: 24, key: 'week:G' }}`. Con movimiento reducido pone el valor directo. */
export function countUp(node: HTMLElement, p: { to: number; key: string }) {
  let raf = 0;
  const set = (v: number) => { node.textContent = String(Math.round(v)); };
  const go = (q: { to: number; key: string }) => {
    cancelAnimationFrame(raf);
    if (calm() || seen.has(q.key) || !(q.to > 0)) { set(q.to); seen.add(q.key); return; }
    seen.add(q.key);
    const t0 = performance.now(), D = 800;
    const step = (t: number) => {
      const k = Math.min(1, (t - t0) / D);
      set(q.to * (1 - Math.pow(1 - k, 3)));
      if (k < 1) raf = requestAnimationFrame(step);
    };
    set(0);
    raf = requestAnimationFrame(step);
  };
  go(p);
  return { update: (q: { to: number; key: string }) => { if (q.to !== p.to) { p = q; set(q.to); } }, destroy: () => cancelAnimationFrame(raf) };
}
