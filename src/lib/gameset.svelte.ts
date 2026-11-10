// ﾂｫAjustes del juegoﾂｻ: panel con pestaﾃｱas que se abre desde la ficha (Start, Y, clic derecho o el botﾃｳn Ajustes).
// Cada fila ejecuta la acciﾃｳn de siempre a travﾃｩs de `menuAct`; Magpie, ﾂｫMover a otra categorﾃｭaﾂｻ y ﾂｫQuitarﾂｻ se resuelven aquﾃｭ dentro.
import { app, type GameTab, type MenuItem } from './store.svelte';
import { IS_TABLET } from './backend';
import { blip } from './sound';
import { confirmTwice } from './confirm';
import { ICON } from './icons';
import { closeOverlay, fmtBytes, hideToast, MAGPIE_ORDER, menuAct, pushOverlay, setMagpie, softRefresh, toast } from './controller.svelte';
import { copyCancel, copyGame, copyInfo, installedHere, isClient, type CopyProgress } from './clientgames';
import { listen } from './ipc';
import type { Title } from './types';

export const GAME_TABS: Array<[GameTab, string, string]> = [
  ['asp', 'Aspecto', ICON.image],
  ['jug', 'Al jugar', ICON.play],
  ['dat', 'Capturas y datos', ICON.import],
  ['bib', 'Biblioteca', ICON.saga],
];

/** Texto de ayuda bajo el tﾃｭtulo de cada pestaﾃｱa. */
export const TAB_HELP: Record<GameTab, string> = {
  asp: 'Cﾃｳmo se ve el juego en la biblioteca.',
  jug: 'Se aplica la prﾃｳxima vez que lo lances desde aquﾃｭ.',
  dat: 'Archivos del juego en este PC.',
  bib: 'Dﾃｳnde aparece y cﾃｳmo se organiza, sin salir de su categorﾃｭa.',
};

/** Filas de una pestaﾃｱa, con las mismas condiciones que tenﾃｭa el menﾃｺ de la ficha. */
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
    if (isClient() && !IS_TABLET && t.id && t.gameId && !t.members && (!installedHere(t) || copying.id === t.id)) items.push('copyhere');
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

/** Rellena `app.menu` como lo hacﾃｭa la ficha y delega en `menuAct`. */
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
  } else if (item === 'copyhere') {
    if (copying.id) { void copyCancel(); toast('Cancelando la copia窶ｦ'); blip(520); return; }
    void copyHere(t.id!, t.name);
  } else if (item === 'remove') {
    if (!confirmTwice(g)) return;
    viaMenu('remove', true);
  } else {
    viaMenu(item);
  }
}

/** Copia en curso de un juego del PC principal a este equipo (fase 3b): una a la vez. */
export const copying = $state({ id: '', name: '', done: 0, total: 0 });

const pct = () => (copying.total ? Math.floor((copying.done / copying.total) * 100) : 0);

async function copyHere(id: string, name: string) {
  let size = 0;
  try {
    size = (await copyInfo(id)).total;
  } catch (e) {
    toast(String(e), 4000); blip(220); return;
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const dest = await open({ directory: true, multiple: false, title: `Dﾃｳnde copiar ﾂｫ${name}ﾂｻ (${fmtBytes(size)})` });
  if (typeof dest !== 'string') return;
  Object.assign(copying, { id, name, done: 0, total: size });
  closeOverlay();
  toast(`Copiando ﾂｫ${name}ﾂｻ窶ｦ 0 %`, 0);
  const off = await listen<{ id: string; progress: CopyProgress }>('client-copy-progress', (ev) => {
    if (ev.payload.id !== id) return;
    copying.done = ev.payload.progress.done;
    copying.total = ev.payload.progress.total;
    toast(`Copiando ﾂｫ${name}ﾂｻ窶ｦ ${pct()} % de ${fmtBytes(copying.total)}`, 0);
  });
  try {
    await copyGame(id, dest);
    hideToast();
    toast(`ﾂｫ${name}ﾂｻ ya estﾃ｡ instalado en este equipo`, 3000);
    blip(1320);
    await softRefresh();
  } catch (e) {
    const msg = String(e);
    toast(msg.includes('cancelada') ? 'Copia cancelada: lo descargado se reanuda la prﾃｳxima vez' : msg, 4000);
    blip(220);
  } finally {
    off();
    Object.assign(copying, { id: '', name: '', done: 0, total: 0 });
  }
}

/** Otras categorﾃｭas a las que se puede mover el juego. */
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
