// Buscador global: un campo arriba y, debajo, una fila de portadas por categoría (nunca se mezclan).
import { app, titleMatches } from './store.svelte';
import { blip } from './sound';
import { bump, closeOverlay, openTitle, playTitle, pushOverlay } from './controller.svelte';
import { openOsk } from './osk.svelte';

/** `r = -1` es el campo de texto; `r >= 0` es la fila de resultados (categoría) y `c` la portada. */
export const srch = $state({ q: '', r: -1, c: 0 });

export const SEARCH_MAX = 12;

export interface SearchGroup { pi: number; name: string; items: number[] }

export function searchGroups(): SearchGroup[] {
  if (!srch.q.trim()) return [];
  const out: SearchGroup[] = [];
  app.lib.platforms.forEach((p, pi) => {
    const items: number[] = [];
    for (let ti = 0; ti < p.titles.length && items.length < SEARCH_MAX; ti++) {
      const t = p.titles[ti];
      if (!t.hidden && titleMatches(t.name, srch.q)) items.push(ti);
    }
    if (items.length) out.push({ pi, name: p.name, items });
  });
  return out;
}

export function openSearch() {
  srch.q = ''; srch.r = -1; srch.c = 0;
  pushOverlay('search');
  blip(1320);
}

/** Cambia el texto y devuelve el foco al campo. */
export function setSearchQ(v: string) {
  if (v === srch.q) return;
  srch.q = v; srch.r = -1; srch.c = 0;
}

const hitEl = () => document.querySelector('.srch .srow-g.f') ?? document.querySelector('.srch .gsearch');

export function openSearchResult(pi: number, ti: number) {
  closeOverlay();
  app.p = pi; app.hp = pi;
  openTitle(ti, 'home');
}

export function playSearchResult(pi: number, ti: number) {
  const t = app.lib.platforms[pi]?.titles[ti];
  if (t?.launchable) { void playTitle(t); blip(1320); } else bump(hitEl(), 0, 0);
}

export function searchCmd(c: string) {
  if (c === 'b' || c === 'menu') { closeOverlay(); blip(520); return; }
  const groups = searchGroups();
  const last = groups.length - 1;
  if (c === 'up') {
    if (srch.r < 0) { bump(hitEl(), 0, -1); return; }
    srch.r = srch.r - 1; srch.c = Math.min(srch.c, Math.max(0, (groups[srch.r]?.items.length ?? 1) - 1)); blip(880);
  } else if (c === 'down') {
    if (srch.r >= last) { bump(hitEl(), 0, 1); return; }
    srch.r++; srch.c = Math.min(srch.c, groups[srch.r].items.length - 1); blip(880);
  } else if (c === 'left' || c === 'right') {
    const d = c === 'left' ? -1 : 1;
    const n = srch.r < 0 ? 0 : (groups[srch.r]?.items.length ?? 0);
    const k = srch.c + d;
    if (srch.r < 0 || k < 0 || k >= n) { bump(hitEl(), d, 0); return; }
    srch.c = k; blip(880);
  } else if (c === 'a') {
    if (srch.r < 0) {
      openOsk({ label: 'Buscar', value: srch.q, onChange: setSearchQ, onDone: setSearchQ });
      return;
    }
    const g = groups[srch.r];
    if (g) openSearchResult(g.pi, g.items[srch.c]);
  } else if (c === 'x') {
    const g = srch.r >= 0 ? groups[srch.r] : undefined;
    if (g) playSearchResult(g.pi, g.items[srch.c]); else bump(hitEl(), 0, 0);
  }
}
