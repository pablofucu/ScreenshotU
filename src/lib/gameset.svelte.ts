// «Ajustes del juego»: hoja lateral sobre la ficha con pestañas (Start, Y, clic derecho o Ajustes).
// Cada fila ejecuta la acción a través de `menuAct`; Magpie, «Mover a otra categoría» y «Quitar» se resuelven aquí.
import { app, type GameTab, type MenuItem } from './store.svelte';
import { IS_TABLET } from './backend';
import { blip } from './sound';
import { confirmTwice } from './confirm';
import { ICON } from './icons';
import { canResume, closeGamePanel, closeOverlay, fmtBytes, hideToast, MAGPIE_ORDER, menuAct, pushOverlay, setMagpie, softRefresh, toast } from './controller.svelte';
import { copyCancel, copyGame, copyInfo, installedHere, isClient, type CopyProgress } from './clientgames';
import { listen } from './ipc';
import type { Title } from './types';

export const GAME_TABS: Array<[GameTab, string, string]> = [
  ['fic', 'Ficha', ICON.card],
  ['jug', 'Jugar', ICON.play],
  ['dat', 'Datos', ICON.import],
  ['ges', 'Gestión', ICON.sliders],
];

/** Texto de ayuda cuando no hay una descripción más concreta de la fila. */
export const TAB_HELP: Record<GameTab, string> = {
  fic: 'Estado, colecciones, fecha, imágenes y color de esta ficha.',
  jug: 'Opciones al lanzar el juego desde este equipo.',
  dat: 'Archivos, capturas, partidas y lectura de este juego.',
  ges: 'Cambiar de categoría o administrar la visibilidad en la biblioteca.',
};

/** Filas de una pestaña según el tipo de juego y si es PC o tablet. */
export function gameTabItems(t: Title | undefined, tab: GameTab): MenuItem[] {
  if (!t) return [];
  const isNovel = !!t.gameId && (app.platform?.id === 'jvn' || app.platform?.id === 'vn' || app.platform?.icon === 'vn');
  const manualExe = t.launchable && t.source === 'manual' && !!t.gameId;
  const items: MenuItem[] = [];
  if (tab === 'fic') {
    if (!t.members && t.id) items.push('status', 'addcoll', 'release');
    items.push('sgdb', 'accent');
  } else if (tab === 'jug') {
    if (canResume(t.gameId)) items.push('closegame');
    if (manualExe) items.push('magpie');
    if (manualExe && !IS_TABLET) items.push('setexe', 'setdir');
    if (isClient() && !IS_TABLET && t.id && t.gameId && !t.members && (!installedHere(t) || copying.id === t.id)) items.push('copyhere');
  } else if (tab === 'dat') {
    if (t.hasFolder && !IS_TABLET) items.push('folder');
    if (app.config.root && t.dir && !IS_TABLET) items.push('addshots');
    if (t.gameId && !t.members) items.push('saves');
    if (isNovel) items.push('phrases');
    items.push('reload');
  } else {
    if (t.gameId && t.removable && !t.members && app.config.platforms.length > 1) items.push('category');
    items.push(t.hidden ? 'unhide' : 'hide');
    if (t.removable) items.push('remove');
  }
  return items;
}

/** Solo las pestañas que tienen opciones para este juego. */
export const visibleTabs = (t: Title | undefined) =>
  GAME_TABS.filter(([k]) => gameTabItems(t, k).length > 0);

const curTitle = () => app.platform?.titles[app.gameSet.title];
const rowsOf = () => gameTabItems(curTitle(), app.gameSet.tab);

export function openGameSettings() {
  const t = app.platform?.titles[app.t];
  if (!t) return;
  const tab = visibleTabs(t)[0]?.[0] ?? 'fic';
  app.gameSet = { title: app.t, tab, inRows: true, row: 0, list: '', lrow: 0, confirm: false };
  pushOverlay('gameset');
  blip(1320);
}

export function selectTab(tab: GameTab) {
  const g = app.gameSet;
  if (g.tab === tab) return;
  g.tab = tab; g.inRows = true; g.row = 0; g.list = ''; g.confirm = false;
}

export function stepTab(dir: -1 | 1) {
  const tabs = visibleTabs(curTitle());
  if (tabs.length <= 1) return;
  const cur = tabs.findIndex(([k]) => k === app.gameSet.tab);
  const next = tabs[(cur + dir + tabs.length) % tabs.length]?.[0];
  if (!next || next === app.gameSet.tab) return;
  selectTab(next);
  blip(700);
}

export function cycleMagpie(dir: -1 | 1) {
  const t = curTitle();
  if (!t?.gameId) return;
  const cur = Math.max(0, MAGPIE_ORDER.indexOf(t.magpieMode ?? 'off'));
  const next = MAGPIE_ORDER[(cur + dir + MAGPIE_ORDER.length) % MAGPIE_ORDER.length];
  void setMagpie(t, next);
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
  } else if (item === 'copyhere') {
    if (copying.id) { void copyCancel(); toast('Cancelando la copia…'); blip(520); return; }
    void copyHere(t.id!, t.name);
  } else if (item === 'closegame') {
    void closeGamePanel(g);
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
  const dest = await open({ directory: true, multiple: false, title: `Dónde copiar «${name}» (${fmtBytes(size)})` });
  if (typeof dest !== 'string') return;
  Object.assign(copying, { id, name, done: 0, total: size });
  closeOverlay();
  toast(`Copiando «${name}»… 0 %`, 0);
  const off = await listen<{ id: string; progress: CopyProgress }>('client-copy-progress', (ev) => {
    if (ev.payload.id !== id) return;
    copying.done = ev.payload.progress.done;
    copying.total = ev.payload.progress.total;
    toast(`Copiando «${name}»… ${pct()} % de ${fmtBytes(copying.total)}`, 0);
  });
  try {
    await copyGame(id, dest);
    hideToast();
    toast(`«${name}» ya está instalado en este equipo`, 3000);
    blip(1320);
    await softRefresh();
  } catch (e) {
    const msg = String(e);
    toast(msg.includes('cancelada') ? 'Copia cancelada: lo descargado se reanuda la próxima vez' : msg, 4000);
    blip(220);
  } finally {
    off();
    Object.assign(copying, { id: '', name: '', done: 0, total: 0 });
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
  const rows = rowsOf();
  if (c === 'lb' || c === 'lt') { stepTab(-1); return; }
  if (c === 'rb' || c === 'rt') { stepTab(1); return; }
  if (c === 'up') { g.row = Math.max(0, g.row - 1); g.confirm = false; blip(880); }
  else if (c === 'down') { g.row = Math.min(rows.length - 1, g.row + 1); g.confirm = false; blip(880); }
  else if (c === 'left' || c === 'right') {
    const dir = c === 'left' ? -1 : 1;
    if (rows[g.row] === 'magpie') cycleMagpie(dir);
    else stepTab(dir);
  }
  else if (c === 'a') { if (rows[g.row]) gameSetAct(rows[g.row]); }
  else if (c === 'b') { closeOverlay(); blip(520); }
}
