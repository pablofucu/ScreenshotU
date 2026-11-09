// «Ajustes del juego»: panel con pestañas que se abre desde la ficha (Start, Y, clic derecho o el botón Ajustes).
// Cada fila ejecuta la acción de siempre a través de `menuAct`; Magpie, «Mover a otra categoría» y «Quitar» se resuelven aquí dentro.
import { app, type GameTab, type MenuItem } from './store.svelte';
import { IS_TABLET } from './backend';
import { blip } from './sound';
import { confirmTwice } from './confirm';
import { ICON } from './icons';
import { closeOverlay, MAGPIE_ORDER, menuAct, pushOverlay, setMagpie } from './controller.svelte';
import type { Title } from './types';

export const GAME_TABS: Array<[GameTab, string, string]> = [
  ['asp', 'Aspecto', ICON.image],
  ['jug', 'Al jugar', ICON.play],
  ['dat', 'Capturas y datos', ICON.import],
  ['bib', 'Biblioteca', ICON.saga],
];

/** Texto de ayuda bajo el título de cada pestaña. */
export const TAB_HELP: Record<GameTab, string> = {
  asp: 'Cómo se ve el juego en la biblioteca.',
  jug: 'Se aplica la próxima vez que lo lances desde aquí.',
  dat: 'Archivos del juego en este PC.',
  bib: 'Dónde aparece y cómo se organiza, sin salir de su categoría.',
};

/** Filas de una pestaña, con las mismas condiciones que tenía el menú de la ficha. */
export function gameTabItems(t: Title | undefined, tab: GameTab): MenuItem[] {
  if (!t) return [];
  const isNovel = !!t.gameId && (app.platform?.id === 'jvn' || app.platform?.id === 'vn' || app.platform?.icon === 'vn');
  const manualExe = t.launchable && t.source === 'manual' && !!t.gameId;
  const items: MenuItem[] = [];
  if (tab === 'asp') {
    items.push('sgdb', 'accent', 'reload');
  } else if (tab === 'jug') {
    if (manualExe) items.push('magpie');
    if (manualExe && !IS_TABLET) items.push('setexe', 'setdir');
  } else if (tab === 'dat') {
    if (t.hasFolder && !IS_TABLET) items.push('folder');
    if (app.config.root && t.dir && !IS_TABLET) items.push('addshots');
    if (t.gameId && !t.members) items.push('saves');
    if (isNovel) items.push('phrases');
  } else {
    if (!t.members && t.id) items.push('addcoll', 'status', 'release');
    if (t.gameId && t.removable && !t.members && app.config.platforms.length > 1) items.push('category');
    items.push(t.hidden ? 'unhide' : 'hide');
    if (t.removable) items.push('remove');
  }
  return items;
}

const curTitle = () => app.platform?.titles[app.gameSet.title];
const rowsOf = () => gameTabItems(curTitle(), app.gameSet.tab);

export function openGameSettings() {
  const t = app.platform?.titles[app.t];
  if (!t) return;
  const tab = GAME_TABS.find(([k]) => gameTabItems(t, k).length)?.[0] ?? 'asp';
  app.gameSet = { title: app.t, tab, inRows: false, row: 0, list: '', lrow: 0, confirm: false };
  pushOverlay('gameset');
  blip(1320);
}

export function selectTab(tab: GameTab) {
  const g = app.gameSet;
  if (g.tab === tab) return;
  g.tab = tab; g.row = 0; g.list = ''; g.confirm = false;
}

/** Rellena `app.menu` como lo hacía la ficha y delega en `menuAct`. */
function viaMenu(item: MenuItem, confirm = false) {
  app.menu = { kind: 'detail', title: app.gameSet.title, row: 0, items: [], confirm };
  void menuAct(item);
}

export function gameSetAct(item: MenuItem) {
  const g = app.gameSet;
  const t = curTitle();
  if (!t) return;
  if (item === 'magpie') {
    g.list = 'magpie';
    g.lrow = Math.max(0, MAGPIE_ORDER.indexOf(t.magpieMode ?? 'off'));
    blip(1320);
  } else if (item === 'category') {
    g.list = 'category'; g.lrow = 0;
    blip(1320);
  } else if (item === 'remove') {
    if (!confirmTwice(g)) return;
    viaMenu('remove', true);
  } else {
    viaMenu(item);
  }
}

/** Otras categorías a las que se puede mover el juego. */
export const otherPlatforms = () => app.config.platforms.filter((p) => p.id !== app.platform?.id);

export function gameSetListAct(i: number) {
  const g = app.gameSet;
  const t = curTitle();
  if (!t) return;
  if (g.list === 'magpie') {
    void setMagpie(t, MAGPIE_ORDER[i]);
    g.list = '';
  } else if (g.list === 'category') {
    const p = otherPlatforms()[i];
    if (p) viaMenu(`cat:${p.id}`);
  }
}

export function gameSetCmd(c: string) {
  const g = app.gameSet;
  if (c === 'menu') { closeOverlay(); blip(520); return; }
  if (g.list) {
    const n = g.list === 'magpie' ? MAGPIE_ORDER.length : otherPlatforms().length;
    if (c === 'up') { g.lrow = Math.max(0, g.lrow - 1); blip(880); }
    else if (c === 'down') { g.lrow = Math.min(n - 1, g.lrow + 1); blip(880); }
    else if (c === 'a') gameSetListAct(g.lrow);
    else if (c === 'b' || c === 'left') { g.list = ''; blip(520); }
    return;
  }
  const t = curTitle();
  if (!g.inRows) {
    if (c === 'up' || c === 'down') {
      const d = c === 'up' ? -1 : 1;
      let k = GAME_TABS.findIndex(([x]) => x === g.tab) + d;
      while (k >= 0 && k < GAME_TABS.length && !gameTabItems(t, GAME_TABS[k][0]).length) k += d;
      if (k >= 0 && k < GAME_TABS.length) selectTab(GAME_TABS[k][0]);
      blip(880);
    } else if ((c === 'right' || c === 'a') && rowsOf().length) {
      g.inRows = true; g.row = 0; g.confirm = false; blip(1320);
    } else if (c === 'b') { closeOverlay(); blip(520); }
    return;
  }
  const rows = rowsOf();
  if (c === 'up') { g.row = Math.max(0, g.row - 1); g.confirm = false; blip(880); }
  else if (c === 'down') { g.row = Math.min(rows.length - 1, g.row + 1); g.confirm = false; blip(880); }
  else if (c === 'a') { if (rows[g.row]) gameSetAct(rows[g.row]); }
  else if (c === 'b' || c === 'left') { g.inRows = false; g.confirm = false; blip(520); }
}
