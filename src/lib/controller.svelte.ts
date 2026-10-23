import { tick } from 'svelte';
import { app, SORT_LABEL, SORT_ORDER, TITLE_FILTERS, TITLE_ORDERS, TITLE_ORDER_LABEL, TITLES_TABS, validFilter, type MenuItem, type SortMode, type TitleFilter, type View } from './store.svelte';
import { backend, initBackend } from './backend';
import { blip } from './sound';
import type { BackendEvent, Config, Library, MagpieMode, Shot, Title } from './types';
import { fmtDur } from './format';
import { oskCmd, oskKeydown } from './osk.svelte';
import {
  accentCmd, addCmd, addShotsTo, curTitle, exitSel, formCmd, gdirCmd, hiddenCmd, moveCmd, onFilesDropped,
  openAccentPicker, openAdd, openHidden, openShotMenu, patchTitle, shotMenuAct, toggleHideTitle, toggleSel,
} from './ops.svelte';
import { shotLayout, type ShotLayout } from './shotlayout';
import { tgrid, TZOOM_MAX } from './tgrid';
import { EVO_KINDS, PERIOD_KINDS } from './stats';
import { editSgdbKey, openSgdbPicker, openSgdbTitles, sgdbAuto, sgdbCmd, sgdbTitlesCmd, toggleSgdbAuto } from './sgdb.svelte';
import { AUDIO_COMBO_PRESETS, AUDIO_HOTKEY_PRESETS, COMBO_PRESETS, HOTKEY_PRESETS, MOUSE_PRESETS, learnKeydown, learnMouse, onLearnPad, startLearn } from './learn.svelte';
import { READING_ROWS, isReadingId, readingAct, type ReadingId } from './reading.svelte';
import { checkUpdate, updateAct } from './updater.svelte';
import { isRemoteId, remoteAct, type RemoteId } from './remote.svelte';
import { openPhrases, phrasesCmd } from './phrases.svelte';
import { goalMinLeft, goalTrack, localDay, type GoalKey } from './readgoals';
import { listenSaves, openSaves, refreshSavesStatus, savesCmd, updateManifestFromSettings } from './saves.svelte';
import { libraryCmd, loadLibraryInfo, openLibrary } from './library.svelte';
import { addCollCmd, collCmd, collFirst, collFormCmd, collKey, collReveal, collTab, collValid, loadCollections, openAddColl, openCollForm, pickerCmd, resetColl, setCollFocus } from './coleccion.svelte';
import { openRelease, releaseCmd } from './release.svelte';
import { checkStalled, listCmd, listFirst, listHas, listReveal, listTab, onLaunched, openStatusBar, resetList, stalledCmd, statusBarCmd } from './milista.svelte';

export const GRID_VIEW_H = 480;
/** Medidas de la cuadrícula de juegos según el tamaño elegido (portadas por fila, filas, página). */
export const titlesGrid = () => tgrid(app.config.titlesZoom);
/** La lista de juegos tiene contenido (pestaña «Todos»). */
const onAllTab = () => app.titlesTab === 'all';
/** Capturas recientes que enseña la ficha de un juego. */
export const DETAIL_SHOTS = 5;
/** Lista de la clasificación: alto visible y posición/alto de cada fila (las 3 primeras, con hero, son más altas). */
export const RANK_VIEW_H = 384;
export const rankH = (i: number) => (i < 3 ? 76 : 64);
export const rankY = (i: number) => (i < 3 ? i * 84 : 252 + (i - 3) * 72);
/** Entradas del dock de Inicio (Buscar, Favoritas, Estadísticas, Añadir, Ajustes, Salir). */
export const DOCK_N = 6;

const screenEl = () => document.getElementById('screen');

/** La vista por días solo tiene sentido con orden por fecha. */
export const daysOn = () => app.config.shotsView === 'days' && (app.sortMode === 'recent' || app.sortMode === 'oldest');
/** Posiciones de la cuadrícula de capturas actual. */
export const curLayout = (): ShotLayout => shotLayout(app.cur.shots, daysOn());
/** Posición de un título en la lista visible (filtrada); la baldosa «Añadir» va al final. */
export const viewPos = (ti: number) => {
  const n = app.platform?.titles.length ?? 0;
  return ti >= n ? app.tview.length : Math.max(0, app.tview.indexOf(ti));
};
/** Título real en una posición de la lista visible (la última es la baldosa «Añadir»). */
const atPos = (pos: number) => app.tview[pos] ?? app.platform?.titles.length ?? 0;

/* ---------- toast ---------- */
let toastT: ReturnType<typeof setTimeout> | undefined;
export function toast(msg: string, ms = 1600) {
  app.toastMsg = msg;
  app.toastOn = true;
  clearTimeout(toastT);
  if (ms > 0) toastT = setTimeout(() => (app.toastOn = false), ms);
}
export function hideToast() {
  clearTimeout(toastT);
  app.toastOn = false;
}

/* ---------- configuracion ---------- */
/** Cambia y guarda la configuración. Si el núcleo la rechaza (p. ej. un atajo ocupado), avisa, recupera la
 *  que de verdad quedó guardada y devuelve `false`. */
export async function patchConfig(p: Partial<Config>): Promise<boolean> {
  Object.assign(app.config, p);
  try {
    await backend.saveConfig($state.snapshot(app.config) as Config);
    return true;
  } catch (e) {
    toast(typeof e === 'string' ? e : 'No se pudo guardar la configuración', 3600);
    console.error(e);
    try { Object.assign(app.config, await backend.loadConfig()); } catch { /* */ }
    return false;
  }
}

/* ---------- foco ---------- */
function validKey(view: View, key: string | undefined): string | undefined {
  if (!key) return undefined;
  const n = parseInt(key.replace(/^\D+/, ''), 10);
  if (view === 'home') {
    if (key === 'welcome') return app.config.root ? undefined : key;
    if (/^plat\d+$/.test(key)) return app.config.root && n < app.lib.platforms.length ? key : undefined;
    if (/^dock\d+$/.test(key)) return n < DOCK_N ? key : undefined;
    if (/^rec\d+$/.test(key)) return app.config.root && n < app.recent.length ? key : undefined;
    if (key === 'read') return app.readCard ? key : undefined;
  } else if (view === 'titles') {
    // La baldosa «Añadir juego» es la posición `titles.length`.
    if (/^tit\d+$/.test(key)) {
      if (app.titlesTab === 'coll') return undefined;
      if (listTab()) return listHas(n) ? key : undefined;
      return n === (app.platform?.titles.length ?? 0) || app.tview.includes(n) ? key : undefined;
    }
    // «Colecciones»: `cl<estantería>.<portada>` y la baldosa final `cnew`.
    if (/^(cl\d+\.\d+|cnew)$/.test(key)) return app.titlesTab === 'coll' && collValid(key) ? key : undefined;
  } else if (view === 'detail') {
    const t = app.platform?.titles[app.t];
    if (!t) return undefined;
    if (key === 'play') return t.launchable ? key : undefined;
    if (/^ds\d+$/.test(key)) return n < Math.min(DETAIL_SHOTS, t.shots.length) ? key : undefined;
    if (key === 'dall') return t.shots.length ? key : undefined;
    if (key === 'dhrs' || (key === 'dwk' && t.launchable)) return t.plays?.length ? key : undefined;
  } else if (view === 'shots') {
    if (/^shot\d+$/.test(key)) return n < app.cur.shots.length ? key : undefined;
  } else if (view === 'stats') {
    if (app.statsTab === 'rank' && /^sr\d+$/.test(key)) return n < app.rank.rows.length ? key : undefined;
    if (app.statsTab === 'cal' && /^cd\d+$/.test(key)) return app.cal.days[n] && !app.cal.days[n].future ? key : undefined;
  } else if (view === 'evo') {
    if (/^eb\d+$/.test(key)) return n < app.evo.length ? key : undefined;
  }
  return undefined;
}
function defaultKey(view: View): string {
  if (view === 'home') return app.config.root ? validKey('home', 'plat' + Math.min(app.hp, Math.max(0, app.lib.platforms.length - 1))) ?? 'plat0' : 'welcome';
  if (view === 'titles') {
    if (app.titlesTab === 'coll') return collFirst();
    return listTab() ? (listFirst() === undefined ? '' : 'tit' + listFirst()) : 'tit' + atPos(0);
  }
  if (view === 'detail') {
    const t = app.platform?.titles[app.t];
    return t?.launchable ? 'play' : t?.shots.length ? 'ds0' : '';
  }
  if (view === 'shots') return app.cur.shots.length ? 'shot0' : '';
  if (view === 'stats') {
    if (app.statsTab === 'year') return '';
    if (app.statsTab === 'rank') return app.rank.rows.length ? 'sr0' : '';
    const c = app.cal;
    return 'cd' + (c.today >= 0 ? c.today : c.days.length - 1);
  }
  if (view === 'evo') return app.evo.length ? 'eb' + (app.evo.length - 1) : '';
  return '';
}

/** `mouse`: el foco viene del ratón; no se desplaza la cuadrícula para no mover el contenido bajo el cursor. */
export function setFocus(k: string, mouse = false) {
  app.focus = k;
  app.fk[app.view] = k;
  if (app.view === 'home') {
    if (k.startsWith('plat')) app.hp = +k.slice(4);
  } else if (app.view === 'titles') {
    if (k.startsWith('tit')) {
      app.t = +k.slice(3);
      // «Mi lista»: la estantería y la balda se desplazan para que el juego enfocado se vea (con el ratón no, para no mover lo que hay bajo el cursor).
      if (!mouse && app.titlesTab === 'list') listReveal();
    } else if (k.startsWith('cl') || k === 'cnew') {
      setCollFocus(k);
      if (!mouse) collReveal();
    }
  } else if (app.view === 'stats') {
    // La lista se desplaza para que la fila enfocada se vea entera.
    if (!k.startsWith('sr')) return;
    const i = +k.slice(2), y = rankY(i), h = rankH(i);
    if (y < app.goff) app.goff = y;
    else if (y + h > app.goff + RANK_VIEW_H) app.goff = y + h - RANK_VIEW_H;
  } else if (app.view === 'shots') {
    const i = +k.slice(4);
    app.s = i;
    if (mouse) return;
    const L = curLayout();
    if (L.y[i] === undefined) return;
    // En la vista por días, la cabecera del día también tiene que verse en su primera fila.
    const head = L.days ? L.heads.find((h) => h.first <= i && i < h.first + h.n) : undefined;
    const top = head && L.y[i] === L.y[head.first] ? head.y : L.y[i];
    let g = app.goff;
    if (top - g < 14) g = Math.max(0, top - 18);
    if (L.y[i] + L.h - g > GRID_VIEW_H - 18) g = L.y[i] + L.h - GRID_VIEW_H + 24;
    app.goff = Math.max(0, Math.min(g, Math.max(0, L.total - GRID_VIEW_H)));
  }
}

/* ---------- navegacion espacial (pos/move del mockup) ---------- */
function pos(e: HTMLElement, root: HTMLElement) {
  let x = 0, y = 0;
  let n: HTMLElement | null = e;
  while (n && n !== root) {
    x += n.offsetLeft;
    y += n.offsetTop;
    n = n.offsetParent as HTMLElement | null;
  }
  return { x: x + e.offsetWidth / 2, y: y + e.offsetHeight / 2 };
}

export function bump(e: Element | null | undefined, dx: number, dy: number) {
  if (!e) return;
  try {
    (e as HTMLElement).animate(
      { translate: ['0 0', `${dx * 8}px ${dy * 8}px`, '0 0'], offset: [0, 0.4, 1] },
      { duration: 180, easing: 'ease-out' },
    );
  } catch {
    /* sin animaciones web */
  }
  blip(220);
}

export function move(dx: number, dy: number) {
  if (app.view === 'viewer') {
    if (app.vz.k > 1) { panView(dx, dy); return; }
    if (dx) stepViewer(dx);
    return;
  }
  if (app.view === 'stats' && app.statsTab === 'year') {
    if (dx) yearStep(dx); else bump(screenEl()?.querySelector('.ycard'), 0, dy);
    return;
  }
  const root = screenEl();
  if (!root) return;
  const cur = root.querySelector<HTMLElement>('.fx.f');
  const all = [...root.querySelectorAll<HTMLElement>('.fx')];
  if (!cur) {
    if (all[0]) setFocus(all[0].dataset.k!);
    return;
  }
  const c = pos(cur, root);
  let best: HTMLElement | null = null, bs = 1e9;
  for (const e of all) {
    if (e === cur) continue;
    const p = pos(e, root), vx = p.x - c.x, vy = p.y - c.y;
    const prim = dx ? vx * dx : vy * dy;
    if (prim <= 6) continue;
    const s = prim + Math.abs(dx ? vy : vx) * 2.4;
    if (s < bs) { bs = s; best = e; }
  }
  if (best) { setFocus(best.dataset.k!); blip(880); } else bump(cur, dx, dy);
}

/* ---------- animaciones ---------- */
const calm = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

/** «Pulsacion» de la tarjeta al confirmar con A o clic. */
function press(e: Element | null | undefined) {
  if (!e || calm()) return;
  try {
    (e as HTMLElement).animate({ scale: [1, 0.92, 1] }, { duration: 170, easing: 'cubic-bezier(.3,.7,.4,1)' });
  } catch {
    /* sin animaciones web */
  }
}

/** Entrada escalonada de las tarjetas visibles al montar una pantalla (accion de #screen). */
export function intro(root: HTMLElement) {
  if (!app.anim || calm()) return;
  const sr = root.getBoundingClientRect();
  const els = [...root.querySelectorAll<HTMLElement>('.glass.fx')]
    .map((e) => ({ e, r: e.getBoundingClientRect() }))
    .filter(({ r }) => r.bottom > sr.top && r.top < sr.bottom && r.right > sr.left && r.left < sr.right)
    .sort((a, b) => a.r.top - b.r.top || a.r.left - b.r.left);
  els.forEach(({ e }, k) => {
    try {
      e.animate(
        [{ opacity: 0, translate: '0 16px', scale: 0.9 }, { opacity: 1, translate: '0 0', scale: 1 }],
        { duration: 380, delay: 60 + Math.min(k, 12) * 28, easing: 'cubic-bezier(.34,1.4,.64,1)', fill: 'backwards' },
      );
    } catch {
      /* sin animaciones web */
    }
  });
}

/** Transicion compartida: el elemento `from` se transforma en el que devuelve `to` tras `update`. */
function morph(from: HTMLElement | null | undefined, update: () => void, to: () => HTMLElement | null | undefined) {
  const doc = document as Document & { startViewTransition?: (cb: () => Promise<void>) => { finished: Promise<void> } };
  if (!from || !doc.startViewTransition || calm()) { update(); return; }
  let dest: HTMLElement | null | undefined;
  from.style.viewTransitionName = 'shot';
  const vt = doc.startViewTransition(async () => {
    from.style.viewTransitionName = '';
    update();
    await tick();
    dest = to();
    if (dest) dest.style.viewTransitionName = 'shot';
    // En el visor, esperar (poco) a que la imagen a tamano completo este pintada.
    if (app.view === 'viewer') {
      const t0 = performance.now();
      while (!screenEl()?.querySelector('.vframe img') && performance.now() - t0 < 250) await new Promise((r) => setTimeout(r, 16));
    }
  });
  vt.finished.finally(() => { if (dest) dest.style.viewTransitionName = ''; });
}

/* ---------- pantallas ---------- */
let goT = 0;
export function go(v: View, anim: 'fwd' | 'back' | '' = '') {
  goT = performance.now();
  app.audioMenu = false;
  app.view = v;
  app.anim = anim;
  app.animKey++;
  app.goff = 0;
  resetZoom();
  app.vbare = false;
  if (v !== 'shots' && v !== 'viewer') { exitSel(); app.collOpen = null; }
  if (v === 'viewer') {
    app.vdir = 0;
    wake();
  } else {
    setFocus(validKey(v, app.fk[v]) ?? defaultKey(v));
  }
  if (v === 'shots') backend.prefetch(app.cur.shots.slice(0, 40).map((s) => s.path));
  if (v !== 'viewer') flushRefresh();
}

/* ---------- ficha del juego y su menu «Grafismos y opciones» ---------- */
export function menuItemsFor(t: Title): MenuItem[] {
  const items: MenuItem[] = [];
  if (t.hasFolder) items.push('folder');
  items.push('reload');
  items.push('sgdb');
  if (t.gameId && app.platform?.id === 'jvn') items.push('phrases');
  if (app.config.root && t.dir) items.push('addshots');
  items.push('accent');
  if (!t.members && t.id) items.push('status', 'release', 'addcoll');
  if (t.gameId && !t.members) items.push('saves');
  // Escalado con Magpie: solo juegos .exe (no emuladores).
  if (t.launchable && t.source === 'manual' && t.gameId) items.push('magpie');
  if (t.gameId && t.removable && !t.members && app.config.platforms.length > 1) items.push('category');
  items.push(t.hidden ? 'unhide' : 'hide');
  if (t.removable) items.push('remove');
  return items;
}

/** Abre la ficha del título i de la plataforma actual. */
export function openTitle(i: number) {
  const t = app.platform?.titles[i];
  if (!t) return;
  app.detailFrom = 'titles';
  app.t = i; app.s = 0; app.favView = false; app.sortMode = t.sortMode ?? app.config.shotsOrder;
  delete app.fk.detail;
  go('detail', 'fwd');
  backend.prefetch(t.shots.slice(0, DETAIL_SHOTS + 1).map((s) => s.path));
}

export function openDetailMenu() {
  const t = app.platform?.titles[app.t];
  if (!t) return;
  app.menu = { kind: 'detail', title: app.t, row: 0, items: menuItemsFor(t), confirm: false };
  pushOverlay('menu');
  blip(1320);
}

/* Paneles apilados: B vuelve al panel que abrió el actual. */
export function pushOverlay(o: typeof app.overlay) {
  if (app.overlay) app.overlayStack.push(app.overlay);
  app.overlay = o;
}
export function popOverlay() {
  app.overlay = app.overlayStack.pop() ?? '';
  if (!app.overlay) flushRefresh();
}
export function closeOverlay() {
  app.overlay = '';
  app.overlayStack = [];
  flushRefresh();
}
/** Hay que reescanear en cuanto se cierren los paneles. */
export function markRefresh() { pendingRefresh = true; }

export async function playTitle(t: Title) {
  if (!t.gameId) return;
  if (app.playing) {
    toast(app.playing.id === t.gameId ? `Ya estás jugando a ${t.name}` : 'Ya hay un juego en marcha', 2200);
    return;
  }
  try {
    await backend.launch(t.gameId);
    if (!app.playing) app.playing = { id: t.gameId, since: Date.now() };
    onLaunched(t);
    toast(backend.isTauri ? `Iniciando ${t.name}…` : `Jugar: ${t.name} (simulado en el navegador)`, 2400);
  } catch (e) {
    toast(String(e), 3600);
  }
}

export async function menuAct(item: MenuItem) {
  if (item === 'audio') { closeOverlay(); void openAudioMenu(); return; }
  if (app.menu.kind === 'titles') { titlesMenuAct(item); return; }
  if (app.menu.kind === 'shots') { await shotMenuAct(item); return; }
  const t = app.platform?.titles[app.menu.title];
  if (!t) { closeOverlay(); return; }
  if (item === 'hide' || item === 'unhide') { await toggleHideTitle(t, item === 'hide'); return; }
  if (item === 'accent') { openAccentPicker(); return; }
  if (item === 'magpie') { void cycleMagpie(t); return; }
  if (item === 'status') { openStatusBar(app.menu.title); return; }
  if (item === 'release') { openRelease(app.menu.title); return; }
  if (item === 'addcoll') { const ti = app.menu.title; closeOverlay(); openAddColl(ti); return; }
  if (item === 'sgdb') { void openSgdbPicker(app.p, app.menu.title); return; }
  if (item === 'phrases') { closeOverlay(); void openPhrases(t.gameId, t.name); return; }
  if (item === 'saves' && t.gameId) { void openSaves(t.gameId, t.name); return; }
  if (item === 'addshots') { closeOverlay(); await addShotsTo(t); return; }
  if (item === 'category') {
    // El menú pasa a listar las demás categorías: pulsar A en una mueve el juego (y su carpeta de capturas).
    app.menu.items = app.config.platforms.filter((p) => p.id !== app.platform?.id).map((p): MenuItem => `cat:${p.id}`);
    app.menu.row = 0; app.menu.confirm = false;
    blip(1320);
    return;
  }
  if (item.startsWith('cat:')) {
    const to = item.slice(4);
    const name = app.config.platforms.find((p) => p.id === to)?.name ?? to;
    closeOverlay();
    try {
      await backend.moveGamePlatform(t.gameId!, to);
      toast(`«${t.name}» movido a ${name}`);
      app.fk.titles = 'tit' + Math.max(0, app.t - 1);
      app.t = Math.max(0, app.t - 1);
      go('titles', 'back');
      await softRefresh();
    } catch (e) { toast(String(e), 3600); }
    return;
  }
  if (item === 'remove') {
    if (!app.menu.confirm) { app.menu.confirm = true; blip(520); return; }
    app.overlay = ''; app.overlayStack = [];
    try {
      await backend.removeGame(t.gameId!);
      toast(`«${t.name}» quitado de la lista`);
      app.fk.titles = 'tit' + Math.max(0, app.t - 1);
      app.t = Math.max(0, app.t - 1);
      go('titles', 'back');
      await softRefresh();
    } catch (e) { toast(String(e), 3200); }
    return;
  }
  closeOverlay();
  if (item === 'reload') {
    backend.bustArt();
    app.artVer++;
    await softRefresh();
    toast('Grafismos recargados');
  } else if (item === 'folder') {
    try { await backend.openFolder(t.dir); if (!backend.isTauri) toast('Abrir carpeta (simulado en el navegador)'); }
    catch (e) { toast(String(e), 3200); }
  }
  blip(1320);
}

export const MAGPIE_LABEL: Record<MagpieMode, string> = { off: 'Desactivado', auto: 'Automático', '2x': 'Forzar 2x', '4x': 'Forzar 4x' };
const MAGPIE_ORDER: MagpieMode[] = ['off', 'auto', '2x', '4x'];

/** Recorre Desactivado → Automático → 2x → 4x. Se aplica (y Magpie se reinicia si hace falta) al lanzar el juego. */
async function cycleMagpie(t: Title) {
  const next = cycleIn(MAGPIE_ORDER, t.magpieMode ?? 'off', 1);
  try {
    await backend.setMagpieMode(t.gameId!, next);
    t.magpieMode = next;
    app.magpieTick++;
    blip(1320);
    if (next !== 'off' && !t.magpieSummary) toast('Al lanzarlo se mide la ventana y Magpie se configura solo (pedirá permiso una vez)', 3600);
  } catch (e) { toast(String(e), 3200); }
}

function menuCmd(c: string) {
  const n = app.menu.items.length;
  if (c === 'up') { app.menu.row = Math.max(0, app.menu.row - 1); app.menu.confirm = false; blip(880); }
  else if (c === 'down') { app.menu.row = Math.min(n - 1, app.menu.row + 1); app.menu.confirm = false; blip(880); }
  else if (c === 'a') void menuAct(app.menu.items[app.menu.row]);
  else if (c === 'b' || c === 'menu') { closeOverlay(); blip(520); }
}

/** Confirma la tarjeta: se «hunde» y la accion se lanza cuando la pulsacion ya se ha visto. */
let pressing = false;
export function activate(act: string | undefined) {
  if (!act || pressing) return;
  const el = screenEl()?.querySelector<HTMLElement>(`[data-act="${act}"]`);
  press(el);
  if (app.selMode || calm()) { runAct(act, el); return; }
  pressing = true;
  setTimeout(() => { pressing = false; runAct(act, el); }, 110);
}

function runAct(act: string, el: HTMLElement | null | undefined) {
  const [a, b, c] = act.split(':');
  if (a === 'plat') {
    app.p = +b; app.hp = +b; app.t = 0; app.fk.titles = 'tit0'; app.titleFilter = 'all'; app.titlesTab = 'all'; resetList(); resetColl();
    go('titles', 'fwd');
  } else if (a === 'tit') {
    if (+b >= (app.platform?.titles.length ?? 0)) openAdd(app.platform?.id);
    else openTitle(+b);
  } else if (a === 'cgame') {
    // Portada de una colección (o el hueco de una colección sin juegos, que abre su edición).
    const sh = app.collShelves[+b];
    const ti = sh?.items[+c];
    if (ti !== undefined) openTitle(ti); else if (sh) openCollForm(sh.coll.id);
  } else if (a === 'cnew') {
    openCollForm();
  } else if (a === 'play') {
    const t = app.platform?.titles[app.t];
    if (t) void playTitle(t);
  } else if (a === 'dshot') {
    // La fila de la ficha es siempre «más recientes primero»; el visor sigue el orden de la lista.
    const ds = app.platform?.titles[app.t]?.shots[+b];
    app.s = Math.max(0, ds ? app.cur.shots.findIndex((s) => s.path === ds.path) : +b); app.viewerFrom = 'detail';
    morph(el, () => go('viewer', ''), () => screenEl()?.querySelector('.vframe'));
  } else if (a === 'dall') {
    app.s = 0; app.fk.shots = 'shot0';
    go('shots', 'fwd');
  } else if (a === 'shot') {
    app.s = +b;
    if (app.selMode) { toggleSel(app.cur.shots[+b].path); return; }
    app.viewerFrom = 'shots';
    morph(el, () => go('viewer', ''), () => screenEl()?.querySelector('.vframe'));
  } else if (a === 'welcome') {
    void openLibrary();
  } else if (a === 'dock') {
    dock(+b);
  } else if (a === 'rec') {
    openRecent(+b);
  } else if (a === 'read') {
    openStreak();
  } else if (a === 'sr') {
    // Juego de la clasificación: su ficha (B vuelve a las estadísticas).
    app.p = +b; app.titleFilter = 'all'; app.titlesTab = 'all'; resetList(); resetColl();
    openTitle(+c);
    app.detailFrom = 'stats';
  } else if (a === 'evo') {
    delete app.fk.evo;
    go('evo', 'fwd');
  } else if (a === 'cd' || a === 'evo-bar') {
    return;
  }
  blip(1320);
}

/** «Seguir jugando»: abre la ficha del juego en su plataforma (B vuelve a la lista de esa plataforma). */
function openRecent(i: number) {
  const r = app.recent[i];
  if (!r) return;
  app.p = r.pi; app.hp = r.pi; app.titleFilter = 'all'; app.titlesTab = 'all'; resetList(); resetColl();
  openTitle(r.ti);
}

function dock(i: number) {
  if (i === 0) toast('Buscar: próximamente');
  else if (i === 1) openFavs();
  else if (i === 2) openStats();
  else if (i === 3) openAdd();
  else if (i === 4) openSettings();
  else if (i === 5) void quit();
}

/* ---------- racha de lectura ---------- */
const GOAL_KEYS: GoalKey[] = ['vn', 'jvn'];
const goalOf = (k: GoalKey) => (k === 'vn' ? app.config.goalVn : app.config.goalJvn);
const goalKeys = () => GOAL_KEYS.filter((k) => app.lib.platforms.some((p) => p.id === k));
/** Anillo (categoría de novelas) de un juego, si es de una. */
function goalKeyOf(gameId: string): GoalKey | undefined {
  return GOAL_KEYS.find((k) => app.lib.platforms.some((p) => p.id === k && p.titles.some((t) => t.gameId === gameId)));
}

/** Pantalla «Racha de lectura» (B vuelve a la pantalla desde la que se abrió). */
export function openStreak(k?: GoalKey) {
  const keys = goalKeys();
  if (!keys.length) return;
  app.streakKey = k && keys.includes(k) ? k : keys.includes(app.streakKey) ? app.streakKey : keys[0];
  app.streakFrom = app.view === 'streak' || app.view === 'viewer' ? 'home' : app.view;
  go('streak', 'fwd');
}
/** LB/RB en la racha: el otro anillo. */
function streakSwitch(d: number) {
  const keys = goalKeys();
  const i = keys.indexOf(app.streakKey) + d;
  if (i < 0 || i >= keys.length) { bump(screenEl()?.querySelector('.smap'), d, 0); return; }
  app.streakKey = keys[i];
  blip(700);
}

/** Al empezar una novela: pide al núcleo un aviso sobre el juego para cuando se llegue a la meta de hoy. */
function scheduleGoalNotice(gameId: string) {
  const k = goalKeyOf(gameId);
  if (!k) return;
  const now = Date.now(), goal = goalOf(k);
  const t = goalTrack(k, goal, app.lib.platforms, null, now);
  if (t.done) return;
  // Con la partida que empieza ya contada: si la anterior de este juego terminó hace menos de 2 min, se unen y el
  // hueco cuenta (como en el anillo); la meta se da por cumplida al llegar a `goal - 0,5` min (minutos redondeados).
  const t2 = goalTrack(k, goal, app.lib.platforms, { id: gameId, since: now }, now);
  // Unos segundos de margen: el núcleo empieza a contar cuando ve el proceso del juego.
  const at = now + goalMinLeft(t2.today, goal) * 60000 + 15000;
  if (localDay(at) !== localDay(now)) return;
  const n = t.streak + 1;
  const text = n > 1 ? `¡Meta cumplida! Racha de ${n} días` : '¡Meta de lectura cumplida!';
  backend.goalNotice(gameId, at, text).catch((e) => console.error(e));
}

export function openStats() {
  app.statsOff = 0; app.calOff = 0; app.yearSel = null; app.yearCard = 0;
  delete app.fk.stats;
  go('stats', 'fwd');
}

export const YEAR_CARDS = 8;
/** Resumen anual: tarjeta anterior/siguiente (izquierda/derecha, A avanza). */
function yearStep(d: number) {
  const n = app.yearCard + d;
  if (n < 0 || n >= YEAR_CARDS) { bump(screenEl()?.querySelector('.ycard'), d, 0); return; }
  app.yearDir = d > 0 ? 1 : -1;
  app.yearCard = n;
  blip(880);
}
/** LT/RT en el resumen anual: año anterior/siguiente (solo años con partidas). */
function yearShift(d: number) {
  const i = app.years.indexOf(app.yearNow) + d;
  if (i < 0 || i >= app.years.length) { bump(screenEl()?.querySelector('.ycard'), d, 0); return; }
  app.yearSel = app.years[i];
  app.yearCard = 0; app.yearDir = d > 0 ? 1 : -1;
  blip(700);
}

/** Y recorre Clasificación > Calendario > Resumen anual. */
function toggleStatsTab() {
  app.statsTab = app.statsTab === 'rank' ? 'cal' : app.statsTab === 'cal' ? 'year' : 'rank';
  if (app.statsTab === 'year') { app.yearCard = 0; app.yearSel = null; }
  app.goff = 0;
  delete app.fk.stats;
  setFocus(defaultKey('stats'));
  blip(700);
}

/** LB/RB: tipo de periodo (o escala de la evolución); en el calendario, igual que LT/RT. */
function statsKind(d: number) {
  if (app.view === 'stats' && app.statsTab === 'year') { yearShift(d); return; }
  if (app.view === 'evo') {
    const i = EVO_KINDS.findIndex(([k]) => k === app.evoKind) + d;
    if (i < 0 || i >= EVO_KINDS.length) { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
    app.evoKind = EVO_KINDS[i][0];
    setFocus(defaultKey('evo'));
  } else if (app.statsTab === 'rank') {
    const i = PERIOD_KINDS.findIndex(([k]) => k === app.statsKind) + d;
    if (i < 0 || i >= PERIOD_KINDS.length) { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
    app.statsKind = PERIOD_KINDS[i][0];
    app.statsOff = 0; app.goff = 0;
    setFocus(defaultKey('stats'));
  } else { statsShift(d); return; }
  blip(700);
}
/** LT/RT: periodo anterior/siguiente (en el calendario, 26 semanas). */
function statsShift(d: number) {
  if (app.view === 'stats' && app.statsTab === 'year') { yearShift(d); return; }
  if (app.view === 'evo') { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
  if (app.statsTab === 'rank') {
    const ok = d < 0 ? app.rank.older : app.statsOff < 0;
    if (!ok || app.statsKind === 'all') { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
    app.statsOff += d; app.goff = 0;
    setFocus(defaultKey('stats'));
  } else {
    const ok = d < 0 ? app.cal.older : app.calOff < 0;
    if (!ok) { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
    // Se conserva la casilla (mismo día de la semana); si cae en el futuro, la semana anterior.
    const k = app.focus.startsWith('cd') ? +app.focus.slice(2) : 0;
    app.calOff += d;
    const days = app.cal.days;
    let nk = Math.min(k, days.length - 1);
    while (nk > 0 && days[nk].future) nk -= 7;
    setFocus('cd' + Math.max(0, nk));
  }
  blip(700);
}

function collectFavs(): Shot[] {
  const all = new Map<string, Shot>();
  for (const p of app.lib.platforms) for (const t of p.titles) for (const s of t.shots) if (app.favs.has(s.path)) all.set(s.path, s);
  return [...all.values()].sort((a, b) => b.date - a.date);
}

export function openFavs() {
  if (app.favs.size === 0) {
    toast('Aún no hay favoritas: pulsa X sobre una captura');
    return;
  }
  const list = collectFavs();
  if (!list.length) {
    toast('Tus favoritas ya no están en la biblioteca');
    return;
  }
  app.favSnap = list;
  app.favView = true; app.sortMode = app.config.shotsOrder; app.s = 0; app.fk.shots = 'shot0';
  go('shots', 'fwd');
}

export function back() {
  if (app.view === 'shots' && app.selMode) { exitSel(); toast('Selección terminada'); blip(520); return; }
  if (app.view === 'viewer') {
    if (app.vbare) { app.vbare = false; blip(520); return; }
    if (app.vz.k > 1) { resetZoom(); blip(520); return; }
    const frame = screenEl()?.querySelector<HTMLElement>('.vframe');
    if (app.viewerFrom === 'detail') {
      const di = app.platform?.titles[app.t]?.shots.findIndex((s) => s.path === app.cur.shots[app.s]?.path) ?? app.s;
      app.fk.detail = di >= 0 && di < DETAIL_SHOTS ? 'ds' + di : 'dall';
      const k = app.fk.detail;
      morph(frame, () => go('detail', ''), () => screenEl()?.querySelector(`[data-k="${k}"]`));
    } else {
      app.fk.shots = 'shot' + app.s;
      const k = app.fk.shots;
      morph(frame, () => go('shots', ''), () => screenEl()?.querySelector(`[data-k="${k}"]`));
    }
  } else if (app.view === 'shots') {
    if (app.favView) {
      app.favView = false;
      app.fk.home = 'dock1';
      go('home', 'back');
    } else if (app.collOpen) {
      // De las capturas de una colección se vuelve a su estantería (el foco de la pestaña se conserva).
      app.collOpen = null;
      go('titles', 'back');
    } else {
      app.fk.detail = 'dall';
      go('detail', 'back');
    }
  } else if (app.view === 'detail') {
    if (app.detailFrom === 'stats') { go('stats', 'back'); blip(520); return; }
    if (app.titlesTab !== 'coll') app.fk.titles = 'tit' + app.t;
    go('titles', 'back');
  } else if (app.view === 'evo') {
    app.fk.detail = 'dhrs';
    go('detail', 'back');
  } else if (app.view === 'stats') {
    app.fk.home = 'dock2';
    go('home', 'back');
  } else if (app.view === 'streak') {
    if (app.streakFrom === 'home') app.fk.home = 'read';
    go(app.streakFrom, 'back');
  } else if (app.view === 'titles') {
    app.fk.home = 'plat' + app.p;
    go('home', 'back');
  } else {
    bump(screenEl()?.querySelector('.fx.f'), 0, 0);
    return;
  }
  blip(520);
}

export function page(d: number) {
  if (app.view === 'viewer') { stepViewer(d * 10); return; }
  if (app.view === 'streak') { streakSwitch(d); return; }
  if (app.view === 'stats' || app.view === 'evo') { statsKind(d); return; }
  if (app.view === 'titles') {
    const n = app.tview.length;
    if (!n || !onAllTab()) return;
    const grid = app.config.titlesView === 'grid';
    const size = titlesGrid().size;
    const cur = viewPos(app.t);
    const np = Math.max(0, Math.min(n - 1, cur + d * (grid ? size : 5)));
    if (np === cur || (grid && Math.floor(np / size) === Math.floor(cur / size))) {
      bump(screenEl()?.querySelector('.fx.f'), d, 0);
      return;
    }
    setFocus('tit' + atPos(np)); blip(700);
    return;
  }
  if (app.view === 'shots' && !app.favView && app.collTitle) {
    // En una colección LB/RB cambian el filtro por juego (-1 = todos).
    const members = app.collTitle.members ?? [];
    const m = members.length;
    const f = ((app.sagaFilter + 1 + d + (m + 1)) % (m + 1)) - 1;
    app.sagaFilter = f; app.s = 0; app.goff = 0;
    setFocus(app.cur.shots.length ? 'shot0' : '');
    toast(f < 0 ? 'Todos los juegos' : members[f].name);
    blip(700);
    return;
  }
  if (app.view === 'shots') {
    const n = app.cur.shots.length;
    if (n) { setFocus('shot' + Math.max(0, Math.min(n - 1, app.s + d * 12))); blip(700); }
    return;
  }
  move(d, 0);
}

/** LT/RT: en la lista de juegos, pestaña anterior/siguiente; en la vista de capturas por días, día anterior/siguiente. */
export function trigger(d: number) {
  if (app.view === 'stats' || app.view === 'evo') { statsShift(d); return; }
  if (app.view === 'titles') { cycleTitlesTab(d); return; }
  if (app.view === 'shots' && daysOn()) {
    const heads = curLayout().heads;
    const k = heads.findIndex((h) => h.first <= app.s && app.s < h.first + h.n);
    // LT en mitad de un día vuelve primero a su principio.
    const nk = d < 0 && heads[k] && app.s > heads[k].first ? k : k + d;
    if (nk < 0 || nk >= heads.length) { bump(screenEl()?.querySelector('.fx.f'), d, 0); return; }
    setFocus('shot' + heads[nk].first);
    blip(700);
    return;
  }
  bump(screenEl()?.querySelector('.fx.f'), d, 0);
}

/** Botón View: alterna la cuadrícula de capturas entre continua y agrupada por días. */
export function toggleShotsView() {
  const v = app.config.shotsView === 'days' ? 'grid' : 'days';
  void patchConfig({ shotsView: v });
  if (app.view === 'shots') {
    app.goff = 0;
    setFocus('shot' + app.s);
    toast(v === 'grid' ? 'Vista: cuadrícula continua' : daysOn() ? 'Vista: agrupada por días' : 'Agrupar por días necesita orden por fecha (Y)', 2200);
  } else {
    toast('Capturas: ' + (v === 'grid' ? 'cuadrícula continua' : 'agrupadas por días'));
  }
  blip(700);
}

/** X en la lista de juegos: recorre los órdenes (se recuerda por categoría) conservando el juego enfocado. */
function cycleTitleOrder() {
  const p = app.platform;
  if (!p || !onAllTab()) { bump(screenEl()?.querySelector('.fx.f'), 0, 0); return; }
  const next = TITLE_ORDERS[(TITLE_ORDERS.indexOf(app.titleOrder) + 1) % TITLE_ORDERS.length];
  const keep = app.t;
  void patchConfig({ titlesOrder: { ...app.config.titlesOrder, [p.id]: next } });
  if (p.titles.length) setFocus('tit' + (app.tview.includes(keep) ? keep : atPos(0)));
  toast('Orden: ' + TITLE_ORDER_LABEL[next]);
  blip(700);
}

/** Filtro de la pestaña «Todos» (Start › Filtrar). Se conserva el juego enfocado si sigue visible. */
export function setTitleFilter(f: TitleFilter) {
  const keep = app.t;
  app.titleFilter = validFilter(f);
  if (app.view === 'titles' && onAllTab()) setFocus('tit' + (app.tview.includes(keep) ? keep : atPos(0)));
  const label = TITLE_FILTERS.find(([k]) => k === app.titleFilter)?.[1] ?? '';
  toast(app.titleFilter === 'all' ? 'Sin filtro' : app.tview.length ? `${label}: ${app.tview.length}` : `${label}: ningún juego`);
  blip(700);
}

/** Start en la lista de juegos: filtrar, estado del juego enfocado, salida de audio y Ajustes. */
export function openTitlesMenu() {
  const items: MenuItem[] = [];
  const t = app.platform?.titles[app.t];
  if (onAllTab() && app.platform?.titles.length) {
    items.push('filter');
    if (app.config.titlesView === 'grid') items.push('tsize');
    if (t?.id && !t.members) items.push('status');
  }
  if (app.audioOutputs.length) items.push('audio');
  items.push('settings');
  app.menu = { kind: 'titles', title: app.t, row: 0, items, confirm: false };
  pushOverlay('menu');
  blip(1320);
}
function titlesMenuAct(item: MenuItem) {
  if (item === 'filter') {
    // El menú pasa a listar los filtros, con el activo enfocado.
    app.menu.items = TITLE_FILTERS.map(([f]): MenuItem => `flt:${f}`);
    app.menu.row = Math.max(0, TITLE_FILTERS.findIndex(([f]) => f === app.titleFilter));
    blip(1320);
  } else if (item.startsWith('flt:')) {
    closeOverlay();
    setTitleFilter(item.slice(4) as TitleFilter);
  } else if (item === 'tsize') {
    // A recorre los tamaños (de más grandes a más pequeñas y vuelta); el menú sigue abierto para ver el cambio.
    void patchConfig({ titlesZoom: (app.config.titlesZoom + 1) % (TZOOM_MAX + 1) });
    const keep = app.t;
    setFocus('tit' + (app.tview.includes(keep) ? keep : atPos(0)));
    blip(880);
  } else if (item === 'status') {
    openStatusBar(app.menu.title);
  } else if (item === 'settings') {
    closeOverlay();
    openSettings();
  }
}

/** View / LT / RT en la lista de juegos: pasa de pestaña (Todos · Mi lista · Colecciones). */
function cycleTitlesTab(d = 1) {
  const n = TITLES_TABS.length;
  const i = TITLES_TABS.findIndex(([t]) => t === app.titlesTab);
  const [tab] = TITLES_TABS[(((i + d) % n) + n) % n];
  app.titlesTab = tab;
  if (tab === 'coll') {
    // Se vuelve a la colección en la que se estaba (se reinicia al entrar en la categoría).
    const k = collKey();
    if (!collValid(k)) resetColl();
    setFocus(validKey('titles', k) ?? defaultKey('titles'));
  } else {
    // «Mi lista» conserva el juego enfocado si está en alguna estantería.
    if (tab === 'list') resetList();
    setFocus(validKey('titles', 'tit' + app.t) ?? defaultKey('titles'));
  }
  // Sin aviso: la barra de pestañas ya enseña dónde se está.
  blip(700);
}

/** Tamaño de las portadas: `d` > 0 = más grandes, < 0 = más pequeñas (5, 6, 7 u 8 por fila); se guarda en la config. */
export function zoomTitles(d: number, fresh = true) {
  if (app.view !== 'titles' || app.overlay || app.osk.open || app.config.titlesView !== 'grid' || !onAllTab()) return;
  const z = Math.max(0, Math.min(TZOOM_MAX, app.config.titlesZoom - Math.sign(d)));
  if (z === app.config.titlesZoom) {
    if (fresh) toast(d > 0 ? 'Portadas al tamaño máximo' : 'Portadas al tamaño mínimo');
    return;
  }
  void patchConfig({ titlesZoom: z });
  toast(`Portadas: ${tgrid(z).cols} por fila`);
  blip(700);
}

export function toggleFav() {
  if (app.view !== 'shots' && app.view !== 'viewer') return;
  const shot = app.cur.shots[app.s];
  if (!shot) return;
  const on = !app.favs.has(shot.path);
  if (on) app.favs.add(shot.path); else app.favs.delete(shot.path);
  void backend.setFavorite(shot.path, on);
  toast(on ? 'Añadida a favoritas' : 'Quitada de favoritas');
  blip(1100);
}

function cycleSort() {
  const path = app.cur.shots[app.s]?.path;
  app.sortMode = SORT_ORDER[(SORT_ORDER.indexOf(app.sortMode) + 1) % SORT_ORDER.length];
  const idx = Math.max(0, app.cur.shots.findIndex((s) => s.path === path));
  app.goff = 0;
  setFocus('shot' + idx);
  toast('Orden: ' + SORT_LABEL[app.sortMode]);
  blip(700);
  // Se recuerda por juego (no en la vista de favoritas, que mezcla varios).
  const t = app.platform?.titles[app.t];
  if (!app.favView && !app.collTitle && t && !t.members) {
    patchTitle(app.p, app.t, { sortMode: app.sortMode });
    backend.setSortPref(t.id, app.sortMode).catch((e) => console.error(e));
  }
}

export function toggleTitlesView() {
  const v = app.config.titlesView === 'grid' ? 'carousel' : 'grid';
  void patchConfig({ titlesView: v });
  toast('Vista: ' + (v === 'grid' ? 'cuadrícula' : 'carrusel'));
  blip(700);
}

/* ---------- visor ---------- */
export function stepViewer(d: number) {
  const n = app.cur.shots.length;
  const ni = Math.max(0, Math.min(n - 1, app.s + d));
  if (ni === app.s) {
    bump(document.querySelector('.vframe'), Math.sign(d), 0);
    return;
  }
  app.vdir = Math.sign(d) || 1;
  app.s = ni;
  blip(880);
  wake();
}
/* Zoom del visor: `app.vz` = aumento y desplazamiento (px del escenario) de la imagen respecto al centro del marco. */
const ZOOM_MAX = 8;
export function resetZoom() {
  if (app.vz.k !== 1 || app.vz.x || app.vz.y) app.vz = { k: 1, x: 0, y: 0 };
}
/** Mantiene la imagen ampliada dentro del marco (sin enseñar fondo si ya lo cubre). */
function clampPan() {
  const z = app.vz;
  const box = screenEl()?.querySelector<HTMLElement>('.vzoom');
  const im = box?.querySelector('img');
  if (!box || !im || !im.naturalWidth) return;
  const fw = box.offsetWidth, fh = box.offsetHeight;
  const s = Math.min(fw / im.naturalWidth, fh / im.naturalHeight);
  const mx = Math.max(0, (im.naturalWidth * s * z.k - fw) / 2), my = Math.max(0, (im.naturalHeight * s * z.k - fh) / 2);
  z.x = Math.max(-mx, Math.min(mx, z.x));
  z.y = Math.max(-my, Math.min(my, z.y));
}
/** Multiplica el aumento por `f` manteniendo fijo el punto (px, py) respecto al centro del marco. */
export function zoomView(f: number, px = 0, py = 0) {
  const z = app.vz;
  const k = Math.max(1, Math.min(ZOOM_MAX, z.k * f));
  if (k === z.k) return;
  const r = k / z.k;
  z.x = px - (px - z.x) * r;
  z.y = py - (py - z.y) * r;
  z.k = k;
  if (k < 1.005) { resetZoom(); return; }
  clampPan();
}
/** Mueve la imagen ampliada; la dirección es hacia dónde se «mira». */
function panView(dx: number, dy: number) {
  const z = app.vz;
  const x0 = z.x, y0 = z.y;
  z.x -= dx * 90;
  z.y -= dy * 90;
  clampPan();
  if (z.x === x0 && z.y === y0) bump(document.querySelector('.vframe'), dx, dy);
  else blip(880);
}
/** Botón View en el visor: alterna «solo imagen». */
export function toggleBare() {
  app.vbare = !app.vbare;
  blip(700);
}

export function jumpViewer(i: number) {
  const d = i - app.s;
  if (!d) return;
  app.vdir = Math.sign(d);
  app.s = i;
}

let idleT: ReturnType<typeof setTimeout> | undefined;
export function wake() {
  app.idle = false;
  clearTimeout(idleT);
  if (app.view === 'viewer') idleT = setTimeout(() => (app.idle = true), 3200);
}

/* ---------- ajustes ---------- */
export type SettingId =
  | 'root' | 'rescan' | 'addgame' | 'hidden' | 'showhidden' | 'view' | 'shotsview' | 'shotsorder' | 'collstyle' | 'accent' | 'sound' | 'fullscreen' | 'monitor' | 'oncursor'
  | 'audioout' | 'audiocombo' | 'audiohotkey' | 'hotkey' | 'combo' | 'mouse' | 'magpie' | 'magpieexe' | 'capsound' | 'cappeek' | 'caponly' | 'caprumble' | 'quit'
  | 'sgdbkey' | 'sgdbauto' | 'sgdbnow' | 'sgdbgame'
  | 'backup' | 'backupmedia' | 'restore' | 'autobackup'
  | 'savesauto' | 'ludusavi' | 'update'
  | 'liststalled' | 'liststalleddays' | 'liststalledact' | 'listautoplay'
  | ReadingId | RemoteId;
/** Secciones de Ajustes (LB/RB las recorren). `color` es el índice del arcoíris. */
export const SETTING_SECTIONS: Array<{ name: string; icon: string; color: number; rows: SettingId[] }> = [
  { name: 'Biblioteca', icon: 'folder', color: 0, rows: ['root', 'rescan', 'addgame', 'hidden', 'showhidden'] },
  { name: 'Apariencia', icon: 'palette', color: 1, rows: ['accent', 'view', 'shotsview', 'shotsorder', 'collstyle'] },
  { name: 'SteamGridDB', icon: 'image', color: 2, rows: ['sgdbkey', 'sgdbauto', 'sgdbnow', 'sgdbgame'] },
  { name: 'Capturas propias', icon: 'camera', color: 3, rows: ['combo', 'mouse', 'hotkey', 'capsound', 'cappeek', 'caprumble', 'caponly'] },
  { name: 'Audio', icon: 'speaker', color: 4, rows: ['audioout', 'audiocombo', 'audiohotkey'] },
  { name: 'Mi lista', icon: 'checks', color: 3, rows: ['liststalled', 'liststalleddays', 'liststalledact', 'listautoplay'] },
  { name: 'Lectura', icon: 'jvn', color: 5, rows: READING_ROWS },
  { name: 'Magpie', icon: 'image', color: 6, rows: ['magpieexe', 'magpie'] },
  { name: 'Copias', icon: 'import', color: 0, rows: ['backup', 'backupmedia', 'restore', 'autobackup'] },
  { name: 'Partidas', icon: 'pad', color: 1, rows: ['savesauto', 'ludusavi'] },
  // Móvil: fuera de la 1.0 (código en remote.svelte.ts; ver REMOTE_AVAILABLE en src-tauri/src/remote.rs).
  { name: 'Sistema', icon: 'gear', color: 2, rows: ['sound', 'fullscreen', 'monitor', 'oncursor', 'update', 'quit'] },
];
const secRows = () => SETTING_SECTIONS[app.setSec]?.rows ?? [];


async function refreshMagpieStatus() {
  try { app.magpieStatus = await backend.magpieStatus(); } catch { app.magpieStatus = null; }
}

/** A: elegir Magpie.exe a mano; izquierda/derecha: volver a buscarlo solo. */
async function chooseMagpieExe(dir: number) {
  if (dir !== 0) {
    if (app.config.magpieExe) await patchConfig({ magpieExe: '' });
  } else {
    const r = await backend.pickFile('exe');
    if (!r) return;
    if (!/magpie\.exe$/i.test(r)) { toast('Elige el archivo Magpie.exe', 2600); return; }
    await patchConfig({ magpieExe: r });
  }
  await refreshMagpieStatus();
  blip(1320);
}

/* ---------- copias de seguridad ---------- */
export const fmtBytes = (n: number) => (n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : n >= 1e6 ? `${(n / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1e3))} KB`);

/** Descarta una restauración preparada que no se llegó a confirmar. */
export function dropRestore() {
  if (!app.restoreReady) return;
  app.restoreReady = '';
  void backend.restoreCancel().catch(() => {});
}

/** Crea un .zip con la base de datos, la configuración y (según la fila de al lado) los medios del glosario. */
async function createBackup() {
  if (app.backupBusy) return;
  const d = new Date();
  const name = `visor-copia-${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}.zip`;
  let dest: string | null = null;
  try { dest = await backend.pickSaveZip(name); } catch (e) { toast(String(e), 3600); return; }
  if (!dest) return;
  app.backupBusy = true;
  toast('Creando copia de seguridad…', 0);
  try {
    const r = await backend.backupCreate(dest, app.backupMedia);
    toast(`Copia guardada (${fmtBytes(r.size)}${r.mediaFiles ? `, con ${r.mediaFiles} archivos de medios` : ''})`, 4200);
    blip(1320);
  } catch (e) {
    toast(`No se pudo crear la copia: ${e}`, 5200);
  } finally {
    app.backupBusy = false;
  }
}

/** Restaurar: A elige el .zip y lo valida; A otra vez (confirmación) reinicia y lo aplica al arrancar. */
async function restoreBackup() {
  if (app.restoreReady) {
    toast('Reiniciando para restaurar…', 0);
    try { await backend.restoreApply(); } catch (e) { app.restoreReady = ''; toast(String(e), 4200); }
    return;
  }
  let zip: string | null = null;
  try { zip = await backend.pickZip(); } catch (e) { toast(String(e), 3600); return; }
  if (!zip) return;
  toast('Comprobando la copia…', 0);
  try {
    const r = await backend.restorePrepare(zip);
    app.restoreReady = `${fmtBytes(r.size)}${r.hasConfig ? ' + ajustes' : ''}${r.mediaFiles ? ` + ${r.mediaFiles} medios` : ''}`;
    toast('Copia válida. Pulsa A otra vez para restaurarla y reiniciar', 4200);
    blip(520);
  } catch (e) {
    toast(`No se puede usar esa copia: ${e}`, 5200);
  }
}

/** Monitor del visor: se lee al abrir Ajustes; A / ‹ › lo pasan al siguiente o al anterior. */
async function refreshMonitor(dir = 0) {
  try { app.monitorLabel = dir ? await backend.nextMonitor(dir) : await backend.windowMonitor(); } catch { app.monitorLabel = ''; }
}

/* ---------- salida de audio ---------- */
export async function refreshAudio() {
  try { app.audioOutputs = await backend.audioOutputs(); } catch { app.audioOutputs = []; }
}

/** Pasa a la salida `dir` posiciones más allá (dando la vuelta) o, con `id`, a esa. */
export async function switchAudio(dir: number, id?: string) {
  if (!id) {
    await refreshAudio();
    const outs = app.audioOutputs;
    if (outs.length < 2) { toast(outs.length ? 'Solo hay una salida de audio' : 'No se encontraron salidas de audio', 2200); return; }
    const cur = Math.max(0, outs.findIndex((o) => o.default));
    id = outs[(cur + dir + outs.length) % outs.length].id;
  }
  try {
    app.audioOutputs = await backend.setAudioOutput(id);
    const now = app.audioOutputs.find((o) => o.default);
    if (now) toast(`Audio: ${now.short}`, 1800);
    blip(1320);
  } catch (e) {
    toast(String(e), 3600);
  }
}

/* Desplegable de la barra superior: con el mando arriba/abajo eligen, A cambia, B (o Start) cierra. Mientras está
 * abierto se queda con todas las pulsaciones. */
export async function openAudioMenu() {
  await refreshAudio();
  if (!app.audioOutputs.length) { toast('No se encontraron salidas de audio', 2200); return; }
  app.audioRow = Math.max(0, app.audioOutputs.findIndex((o) => o.default));
  app.audioMenu = true;
  blip(1320);
}
export function closeAudioMenu() {
  if (!app.audioMenu) return;
  app.audioMenu = false;
  blip(520);
}
function audioCmd(c: string) {
  const n = app.audioOutputs.length;
  if (!n) { closeAudioMenu(); return; }
  if (c === 'up' || c === 'down') {
    const r = Math.max(0, Math.min(n - 1, app.audioRow + (c === 'up' ? -1 : 1)));
    if (r === app.audioRow) bump(document.querySelector('.audmenu'), 0, c === 'up' ? -1 : 1);
    else { app.audioRow = r; blip(880); }
  } else if (c === 'a') {
    const o = app.audioOutputs[Math.min(app.audioRow, n - 1)];
    app.audioMenu = false;
    if (o && !o.default) void switchAudio(0, o.id);
    else blip(520);
  } else if (c === 'b' || c === 'menu') closeAudioMenu();
}

export function openSettings() {
  void loadLibraryInfo();
  void refreshMagpieStatus();
  void refreshSavesStatus();
  void refreshMonitor();
  void refreshAudio();
  app.overlayStack = [];
  app.overlay = 'settings';
  app.setRow = 0;
  app.setSec = 0;
  blip(1320);
}
export function closeSettings() {
  dropRestore();
  app.overlay = '';
  flushRefresh();
  blip(520);
}
/** Estados de acento: 0 = del juego (automático), 1 = arcoíris, 2..8 = color fijo. */
export function accentState(): number {
  const m = app.config.accentMode;
  return m === 'art' ? 0 : m === 'rainbow' ? 1 : app.config.accentFixed + 2;
}
function cycleAccent(d: number) {
  const n = (accentState() + d + 9) % 9;
  void patchConfig(n === 0 ? { accentMode: 'art' } : n === 1 ? { accentMode: 'rainbow' } : { accentMode: 'fixed', accentFixed: n - 2 });
  blip(880);
}
function cycleIn<T>(list: T[], cur: T, d: number): T {
  const i = Math.max(0, list.indexOf(cur));
  return list[(i + d + list.length) % list.length];
}
export async function toggleFullscreen() {
  try {
    await backend.toggleFullscreen();
    app.fullscreen = await backend.isFullscreen();
  } catch (e) {
    console.error(e);
  }
}
async function quit() {
  if (backend.isTauri) await backend.exit();
  else toast('Salir solo funciona en la aplicación de escritorio');
}

export function setSection(i: number) {
  dropRestore();
  const n = SETTING_SECTIONS.length;
  app.setSec = ((i % n) + n) % n;
  app.setRow = 0;
}

export function settingsAct(id: SettingId | undefined, dir: number) {
  // dir: 0 = A / clic, -1 / +1 = izquierda / derecha
  if (id !== 'restore') dropRestore();
  if (id === 'root' && dir === 0) void openLibrary();
  else if (id === 'rescan' && dir === 0) void rescan();
  else if (id === 'addgame' && dir === 0) openAdd();
  else if (id === 'hidden' && dir === 0) openHidden();
  else if (id === 'showhidden') {
    void patchConfig({ showHidden: !app.config.showHidden }).then(() => { markRefresh(); });
    blip(1320);
  }
  else if (id === 'view') toggleTitlesView();
  else if (id === 'shotsview') toggleShotsView();
  else if (id === 'shotsorder') { void patchConfig({ shotsOrder: app.config.shotsOrder === 'oldest' ? 'recent' : 'oldest' }); blip(1320); }
  else if (id === 'collstyle') { void patchConfig({ collectionsStyle: app.config.collectionsStyle === 'aurora' ? 'multi' : 'aurora' }); blip(1320); }
  else if (id === 'accent') cycleAccent(dir || 1);
  else if (id === 'sound') { void patchConfig({ sound: !app.config.sound }); blip(1320); }
  else if (id === 'fullscreen') void toggleFullscreen();
  else if (id === 'monitor') { void refreshMonitor(dir || 1); blip(880); }
  else if (id === 'oncursor') { void patchConfig({ openOnCursor: !app.config.openOnCursor }); blip(1320); }
  // Botones de captura: A / clic espera el input; izquierda/derecha recorren unos cuantos de serie.
  else if (id === 'audioout') void switchAudio(dir || 1);
  else if (id === 'audiocombo') { if (dir === 0) startLearn('pad', 'audio'); else void patchConfig({ audioCombo: cycleIn(AUDIO_COMBO_PRESETS, app.config.audioCombo, dir) }); }
  else if (id === 'audiohotkey') { if (dir === 0) startLearn('key', 'audio'); else void patchConfig({ audioHotkey: cycleIn(AUDIO_HOTKEY_PRESETS, app.config.audioHotkey, dir) }); }
  else if (id === 'hotkey') { if (dir === 0) startLearn('key'); else void patchConfig({ captureHotkey: cycleIn(HOTKEY_PRESETS, app.config.captureHotkey, dir) }); }
  else if (id === 'combo') { if (dir === 0) startLearn('pad'); else void patchConfig({ captureCombo: cycleIn(COMBO_PRESETS, app.config.captureCombo, dir) }); }
  else if (id === 'mouse') { if (dir === 0) startLearn('mouse'); else void patchConfig({ captureMouse: cycleIn(MOUSE_PRESETS, app.config.captureMouse, dir) }); }
  else if (id === 'magpie') { void patchConfig({ captureMagpie: app.config.captureMagpie === 'source' ? 'scaled' : 'source' }); blip(1320); }
  else if (id === 'magpieexe') void chooseMagpieExe(dir);
  else if (id === 'capsound') { void patchConfig({ captureSound: !app.config.captureSound }); blip(1320); }
  else if (id === 'cappeek') { void patchConfig({ capturePeek: !app.config.capturePeek }); blip(1320); }
  else if (id === 'caponly') { void patchConfig({ captureOnlyGames: !app.config.captureOnlyGames }); blip(1320); }
  else if (id === 'caprumble') { void patchConfig({ captureRumble: !app.config.captureRumble }); blip(1320); }
  else if (id === 'update') updateAct(dir);
  else if (id === 'quit' && dir === 0) void quit();
  else if (id === 'backup' && dir === 0) void createBackup();
  else if (id === 'backupmedia') { app.backupMedia = !app.backupMedia; blip(1320); }
  else if (id === 'restore' && dir === 0) void restoreBackup();
  else if (id === 'autobackup') { void patchConfig({ autoBackup: !app.config.autoBackup }); blip(1320); }
  else if (id === 'savesauto') { void patchConfig({ savesAuto: !app.config.savesAuto }); blip(1320); }
  else if (id === 'liststalled') { void patchConfig({ listStalled: !app.config.listStalled }); blip(1320); }
  else if (id === 'liststalleddays') {
    // ‹ › de uno en uno (mantener pulsado acelera); A suma una semana y da la vuelta al llegar a 180.
    const d = app.config.listStalledDays;
    void patchConfig({ listStalledDays: dir ? Math.max(7, Math.min(180, d + dir)) : d >= 180 ? 7 : Math.min(180, d + 7) });
    blip(880);
  }
  else if (id === 'liststalledact') { void patchConfig({ listStalledAction: cycleIn(['ask', 'pending', 'dropped'] as Config['listStalledAction'][], app.config.listStalledAction, dir || 1) }); blip(1320); }
  else if (id === 'listautoplay') { void patchConfig({ listAutoPlay: !app.config.listAutoPlay }); blip(1320); }
  else if (id === 'ludusavi' && dir === 0) void updateManifestFromSettings();
  else if (id === 'sgdbkey' && dir === 0) editSgdbKey();
  else if (id === 'sgdbauto') toggleSgdbAuto();
  else if (id === 'sgdbnow' && dir === 0) void sgdbAuto(true);
  else if (id === 'sgdbgame' && dir === 0) openSgdbTitles();
  else if (id && isReadingId(id)) readingAct(id, dir);
  else if (id && isRemoteId(id)) remoteAct(id, dir);
}

function overlayCmd(c: string) {
  switch (app.overlay) {
    case 'menu': menuCmd(c); return;
    case 'form': formCmd(c); return;
    case 'add': addCmd(c); return;
    case 'gdir': gdirCmd(c); return;
    case 'hidden': hiddenCmd(c); return;
    case 'moveto': moveCmd(c); return;
    case 'accent': accentCmd(c); return;
    case 'collform': collFormCmd(c); return;
    case 'picker': pickerCmd(c); return;
    case 'addcoll': addCollCmd(c); return;
    case 'sgdb': sgdbCmd(c); return;
    case 'phrases': phrasesCmd(c); return;
    case 'sgdbtitles': sgdbTitlesCmd(c); return;
    case 'saves': savesCmd(c); return;
    case 'library': libraryCmd(c); return;
    case 'status': statusBarCmd(c); return;
    case 'release': releaseCmd(c); return;
    case 'stalled': stalledCmd(c); return;
  }
  // Arriba/abajo pasan de una sección a la siguiente al llegar al borde; LB/RB (o LT/RT) saltan de sección.
  const rows = secRows();
  if (c === 'up' || c === 'down') dropRestore();
  if (c === 'up') {
    if (app.setRow > 0) app.setRow--;
    else if (app.setSec > 0) { setSection(app.setSec - 1); app.setRow = secRows().length - 1; }
  } else if (c === 'down') {
    if (app.setRow < rows.length - 1) app.setRow++;
    else if (app.setSec < SETTING_SECTIONS.length - 1) setSection(app.setSec + 1);
  }
  else if (c === 'lb' || c === 'lt') { setSection(app.setSec - 1); blip(700); }
  else if (c === 'rb' || c === 'rt') { setSection(app.setSec + 1); blip(700); }
  else if (c === 'left') settingsAct(rows[app.setRow], -1);
  else if (c === 'right') settingsAct(rows[app.setRow], 1);
  else if (c === 'a') settingsAct(rows[app.setRow], 0);
  else if (c === 'b' || c === 'menu') closeSettings();
  if (c === 'up' || c === 'down') blip(880);
}

/* ---------- biblioteca ---------- */
/** Escanea y carga las colecciones (convirtiendo las sagas antiguas la primera vez). */
async function scanLib() {
  const lib = await backend.scan();
  await loadCollections(lib);
  return lib;
}

/** Antes de la biblioteca (o sin encontrarla): el asistente sale solo al arrancar y no se puede cerrar. */
async function libraryGate() {
  const info = await loadLibraryInfo();
  if (info && info.mode !== 'library') await openLibrary();
}

export async function rescan() {
  app.scanning = true;
  toast('Buscando capturas…', 0);
  try {
    app.lib = await scanLib();
    const titles = app.lib.platforms.reduce((n, p) => n + p.titles.length, 0);
    toast(app.config.root ? `${titles} títulos encontrados` : 'Elige una carpeta para empezar');
    // calienta las portadas
    const covers = app.lib.platforms.flatMap((p) => p.titles.map((t) => t.cover ?? t.shots[0]?.path)).filter(Boolean) as string[];
    backend.prefetch(covers.slice(0, 300));
  } catch (e) {
    console.error(e);
    toast('Error al escanear la biblioteca');
  } finally {
    app.scanning = false;
  }
  app.fk = {};
  app.favView = false;
  app.p = 0; app.t = 0; app.s = 0;
  app.view = 'home';
  app.animKey++;
  setFocus(defaultKey('home'));
  void sgdbAuto();
}

/** Reescanea sin perder el sitio: mantiene plataforma, título y captura seleccionados. */
let pendingRefresh = false;
export function flushRefresh() {
  if (pendingRefresh && !app.overlay && app.view !== 'viewer') void softRefresh();
}
const titleKey = (t: Title | undefined) => (t ? (t.gameId ?? t.dir ?? t.name) : '');

export async function softRefresh() {
  if (app.scanning) return;
  if (app.overlay || app.view === 'viewer') { pendingRefresh = true; return; }
  pendingRefresh = false;
  const pid = app.platform?.id;
  const tkey = titleKey(app.platform?.titles[app.t]);
  const spath = app.cur.shots[app.s]?.path;
  let lib;
  try { lib = await scanLib(); } catch (e) { console.error(e); return; }
  app.lib = lib;
  if (app.favView) app.favSnap = collectFavs();
  const pi = lib.platforms.findIndex((p) => p.id === pid);
  app.p = pi < 0 ? 0 : pi;
  const titles = lib.platforms[app.p]?.titles ?? [];
  const ti = titles.findIndex((t) => titleKey(t) === tkey);
  app.t = ti < 0 ? Math.min(app.t, Math.max(0, titles.length - 1)) : ti;
  if (spath) {
    const si = app.cur.shots.findIndex((s) => s.path === spath);
    app.s = si < 0 ? Math.min(app.s, Math.max(0, app.cur.shots.length - 1)) : si;
  }
  if (app.view === 'titles') setFocus(validKey('titles', app.titlesTab === 'coll' ? app.focus : 'tit' + app.t) ?? defaultKey('titles'));
  else if (app.view === 'shots') setFocus(validKey('shots', 'shot' + app.s) ?? defaultKey('shots'));
  else if (app.view === 'detail') setFocus(validKey('detail', app.focus) ?? defaultKey('detail'));
  else if (app.view === 'home') setFocus(validKey('home', app.focus) ?? defaultKey('home'));
  else if (app.view === 'stats' || app.view === 'evo') setFocus(validKey(app.view, app.focus) ?? defaultKey(app.view));
  const covers = lib.platforms.flatMap((p) => p.titles.map((t) => t.cover ?? t.shots[0]?.path)).filter(Boolean) as string[];
  backend.prefetch(covers.slice(0, 300));
  // Juegos nuevos (añadidos o con su primera captura): sus grafismos de SteamGridDB.
  void sgdbAuto();
}

function onBackendEvent(e: BackendEvent) {
  if (e.type === 'capture-saved') {
    toast(`Captura guardada: ${e.name}`, 2200);
    void softRefresh();
  } else if (e.type === 'capture-error') {
    toast(e.message, 3600);
  } else if (e.type === 'audio-outputs') {
    app.audioOutputs = e.outputs;
  } else if (e.type === 'learn-pad') {
    onLearnPad(e.combo, e.connected, e.done);
  } else if (e.type === 'game-started') {
    if (app.playing?.id !== e.id) app.playing = { id: e.id, since: Date.now() };
    scheduleGoalNotice(e.id);
  } else if (e.type === 'magpie-status') {
    toast(e.message, e.error ? 4200 : 3200);
    if (!e.error) {
      const summary = e.message.replace(/^Magpie: /, '');
      for (const p of app.lib.platforms) for (const t of p.titles) if (t.gameId === e.id) t.magpieSummary = summary;
      app.magpieTick++;
    }
  } else if (e.type === 'game-exited') {
    app.playing = null;
    const t = e.seconds >= 60 ? app.lib.platforms.flatMap((p) => p.titles).find((t) => t.gameId === e.id) : undefined;
    app.gain = t ? { id: e.id, seconds: e.seconds, secs: t.playSeconds, week: [...t.week], day: new Date().toDateString(), at: Date.now() } : null;
    toast(e.seconds >= 60 ? `Has jugado ${fmtDur(e.seconds)} a ${e.name}` : `${e.name} se ha cerrado`, 3200);
    void softRefresh();
  }
}

export async function boot() {
  await initBackend();
  backend.onEvent(onBackendEvent);
  void refreshAudio();
  backend.onFilesDropped(onFilesDropped);
  void listenSaves();
  const cfg = await backend.loadConfig();
  Object.assign(app.config, cfg);
  try {
    for (const f of await backend.getFavorites()) app.favs.add(f);
  } catch { /* sin favoritas */ }
  try { app.fullscreen = await backend.isFullscreen(); } catch { /* */ }
  // Con la biblioteca del último escaneo se pinta ya, y el escaneo real (lento en disco duro) va detrás sin bloquear.
  let cached: Library | null = null;
  let fresh: Promise<void> = Promise.resolve();
  try { cached = (await backend.cachedScan?.()) ?? null; } catch (e) { console.error(e); }
  if (cached) {
    await loadCollections(cached);
    app.lib = cached;
    app.view = 'home';
    setFocus(defaultKey('home'));
    app.ready = true;
    fresh = softRefresh().catch(() => {});
  } else {
    await rescan();
    app.ready = true;
  }
  void libraryGate();
  void announceStartup().then(async (shown) => {
    if (!shown) await announceDue();
    // Con la biblioteca de la caché, los estados pueden estar desfasados: se espera al escaneo real. Nunca pisa los avisos de arriba.
    await fresh;
    void checkStalled();
  });
  // Comprobación silenciosa de actualizaciones unos segundos después de arrancar (solo en la app, no en el navegador).
  if (backend.isTauri) setTimeout(() => void checkUpdate(true), 8000);
  setInterval(() => (app.now = new Date()), 20000);
}

/* ---------- entrada ---------- */
export function cmd(c: string) {
  // Mientras Ajustes espera un botón de captura, el mando y el teclado no navegan.
  if (app.learn) return;
  wake();
  if (app.audioMenu) { audioCmd(c); return; }
  if (app.osk.open) { oskCmd(c); return; }
  if (app.overlay) { overlayCmd(c); return; }
  if (listTab() && listCmd(c)) return;
  if (collTab() && collCmd(c)) return;
  switch (c) {
    case 'left': move(-1, 0); break;
    case 'right': move(1, 0); break;
    case 'up': move(0, -1); break;
    case 'down': move(0, 1); break;
    case 'a':
      if (app.view === 'stats' && app.statsTab === 'year') yearStep(1);
      else if (app.view !== 'viewer') activate(screenEl()?.querySelector<HTMLElement>('.fx.f')?.dataset.act); break;
    case 'b': back(); break;
    case 'lb': page(-1); break;
    case 'rb': page(1); break;
    case 'x':
      if (app.view === 'home' && app.focus.startsWith('rec')) {
        const r = app.recent[+app.focus.slice(3)];
        const t = r && app.lib.platforms[r.pi]?.titles[r.ti];
        if (t?.launchable) void playTitle(t); else bump(screenEl()?.querySelector('.fx.f'), 0, 0);
      } else if (app.view === 'titles') cycleTitleOrder();
      else toggleFav();
      break;
    case 'lt': trigger(-1); break;
    case 'rt': trigger(1); break;
    // Chip del filtro activo en la barra de pestañas (clic).
    case 'nofilter': if (app.view === 'titles' && app.titleFilter !== 'all') setTitleFilter('all'); break;
    case 'view':
      if (app.view === 'shots') toggleShotsView();
      else if (app.view === 'viewer') toggleBare();
      else if (app.view === 'titles') cycleTitlesTab();
      break;
    case 'y':
      if (app.view === 'titles') toggleTitlesView();
      else if (app.view === 'detail') openDetailMenu();
      else if (app.view === 'shots') cycleSort();
      else if (app.view === 'stats') toggleStatsTab();
      break;
    case 'menu':
      if (app.view === 'shots' || app.view === 'viewer') openShotMenu();
      else if (app.view === 'titles') openTitlesMenu();
      else openSettings();
      break;
  }
}

const KEYS: Record<string, string> = {
  ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down',
  Enter: 'a', ' ': 'a', Backspace: 'b', m: 'menu', M: 'menu',
  q: 'lb', Q: 'lb', e: 'rb', E: 'rb', x: 'x', X: 'x', y: 'y', Y: 'y',
  z: 'lt', Z: 'lt', c: 'rt', C: 'rt', v: 'view', V: 'view',
};

export function onKeyDown(e: KeyboardEvent) {
  if (learnKeydown(e)) return;
  if (e.key === 'F11') { e.preventDefault(); void toggleFullscreen(); return; }
  if (oskKeydown(e)) return;
  if (e.ctrlKey || e.altKey || e.metaKey) return;
  const tag = (e.target as HTMLElement | null)?.tagName;
  if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
  if (app.view === 'viewer' && !app.overlay && !app.audioMenu && (e.key === '+' || e.key === '=' || e.key === '-')) {
    e.preventDefault();
    zoomView(e.key === '-' ? 0.8 : 1.25);
    return;
  }
  if (app.view === 'titles' && !app.overlay && !app.audioMenu && (e.key === '+' || e.key === '=' || e.key === '-')) {
    e.preventDefault();
    zoomTitles(e.key === '-' ? -1 : 1);
    return;
  }
  let m = KEYS[e.key];
  if (e.key === 'Escape') m = app.view === 'home' && !app.overlay ? 'menu' : 'b';
  if (!m) return;
  e.preventDefault();
  cmd(m);
}

let lastXY = '';
export function onScreenMove(e: MouseEvent) {
  if (app.view === 'viewer') wake();
  const xy = e.clientX + ',' + e.clientY;
  if (xy === lastXY) return; // el ratón solo roba el foco si se mueve de verdad
  lastXY = xy;
  if (app.audioMenu) return;
  // En el carrusel el foco centra la portada: con el ratón sería un baile bajo el cursor (se navega con la rueda).
  if (app.view === 'titles' && app.config.titlesView !== 'grid') return;
  const f = (e.target as HTMLElement).closest<HTMLElement>('.fx');
  if (f && !f.classList.contains('f')) setFocus(f.dataset.k!, true);
}

/** Zona del visor (0 izquierda, 1 centro, 2 derecha) bajo el cursor. */
function viewerZone(e: MouseEvent) {
  const r = screenEl()?.getBoundingClientRect();
  if (!r || !r.width) return 1;
  const x = (e.clientX - r.left) / r.width;
  return x < 0.3 ? 0 : x > 0.7 ? 2 : 1;
}

export function onScreenClick(e: MouseEvent) {
  const t = e.target as HTMLElement;
  // Con el desplegable de audio abierto, un clic fuera solo lo cierra (no activa lo que haya debajo).
  if (app.audioMenu) { if (!t.closest('.aud')) closeAudioMenu(); return; }
  const f = t.closest<HTMLElement>('.fx');
  if (f) { setFocus(f.dataset.k!); activate(f.dataset.act); return; }
  const h = t.closest<HTMLElement>('[data-hact]');
  if (h?.dataset.hact) { cmd(h.dataset.hact); return; }
  const th = t.closest<HTMLElement>('.vthumb');
  if (th) { jumpViewer(+th.dataset.vi!); wake(); return; }
  // Visor: clic en el tercio izquierdo/derecho = anterior/siguiente sin ir a las flechas.
  if (app.view === 'viewer' && app.vz.k === 1 && !app.vbare && performance.now() - goT > 400) { const z = viewerZone(e); if (z !== 1) stepViewer(z === 0 ? -1 : 1); }
  wake();
}

/** Doble clic en el centro del visor = favorita. */
export function onScreenDblClick(e: MouseEvent) {
  if (app.overlay || app.osk.open || app.audioMenu || app.view !== 'viewer' || performance.now() - goT < 400) return;
  if ((e.target as HTMLElement).closest('.vthumb,.hint,[data-hact]')) return;
  if (viewerZone(e) === 1) toggleFav();
}

/** Clic derecho: opciones del elemento (capturas/ficha) o, si no hay, volver/cerrar. */
function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  if (app.osk.open || app.overlay || app.audioMenu) { cmd('b'); return; }
  const f = (e.target as HTMLElement).closest<HTMLElement>('.fx');
  if (f && app.view === 'shots') { setFocus(f.dataset.k!, true); cmd('menu'); return; }
  if (f && app.view === 'detail' && f.dataset.k !== 'play') { setFocus(f.dataset.k!); openDetailMenu(); return; }
  cmd('b');
}

/** Botones laterales del ratón: atrás = volver, adelante = siguiente captura en el visor. */
function onMouseButton(e: MouseEvent) {
  if (e.button !== 3 && e.button !== 4) return;
  e.preventDefault();
  if (e.type !== 'mouseup') return;
  if (e.button === 3) cmd('b');
  else if (app.view === 'viewer' && !app.overlay && !app.osk.open) stepViewer(1);
}

/** Rueda: recorre carruseles, páginas, capturas y listas sin llevar el ratón a los bordes. */
let wheelAcc = 0, wheelT = 0;
let zoomAcc = 0, zoomT = 0;
function onWheel(e: WheelEvent) {
  // Ctrl+rueda en la lista de juegos: tamaño de las portadas (arriba = más grandes).
  if (e.ctrlKey && app.view === 'titles' && !app.overlay && !app.osk.open && !app.audioMenu) {
    e.preventDefault();
    const now = performance.now();
    if (now - zoomT > 250) zoomAcc = 0;
    zoomT = now;
    zoomAcc += e.deltaMode === 1 ? e.deltaY * 40 : e.deltaY;
    if (Math.abs(zoomAcc) < 40) return;
    const d = -Math.sign(zoomAcc);
    zoomAcc = 0;
    zoomTitles(d);
    return;
  }
  if (e.ctrlKey || app.osk.open) return;
  e.preventDefault();
  if (app.audioMenu) { const d = Math.sign(e.deltaY); if (d) cmd(d > 0 ? 'down' : 'up'); return; }
  if (app.view === 'viewer' && !app.overlay) {
    // Acerca/aleja hacia el puntero.
    const fr = screenEl()?.querySelector<HTMLElement>('.vframe');
    if (!fr) return;
    wake();
    const r = fr.getBoundingClientRect(), s = r.width / fr.offsetWidth || 1;
    const dy = e.deltaMode === 1 ? e.deltaY * 40 : e.deltaY;
    zoomView(Math.exp(-dy * 0.002), (e.clientX - r.left - r.width / 2) / s, (e.clientY - r.top - r.height / 2) / s);
    return;
  }
  const now = performance.now();
  if (now - wheelT > 250) wheelAcc = 0;
  wheelT = now;
  const dy = e.deltaMode === 1 ? e.deltaY * 40 : e.deltaY;
  const dx = e.deltaMode === 1 ? e.deltaX * 40 : e.deltaX;
  wheelAcc += Math.abs(dx) > Math.abs(dy) ? dx : dy;
  if (Math.abs(wheelAcc) < 40) return;
  const d = Math.sign(wheelAcc);
  wheelAcc = 0;
  if (app.overlay) { cmd(d > 0 ? 'down' : 'up'); return; }
  wake();
  switch (app.view) {
    case 'home': case 'detail': case 'evo': move(d, 0); break;
    case 'stats': if (app.statsTab === 'rank') move(0, d); else move(d, 0); break;
    case 'titles': {
      if (listTab()) { listCmd(d > 0 ? 'down' : 'up'); break; }
      if (collTab()) { collCmd(d > 0 ? 'down' : 'up'); break; }
      if (app.config.titlesView === 'grid') { page(d); break; }
      const np = viewPos(app.t) + d;
      if (np >= 0 && np <= app.tview.length) { setFocus('tit' + atPos(np)); blip(880); }
      break;
    }
    case 'shots': {
      // Desplaza la cuadrícula una fila; el foco no se toca (lo recoge el ratón al moverse).
      const L = curLayout();
      app.goff = Math.max(0, Math.min(app.goff + d * L.rowStep, Math.max(0, L.total - GRID_VIEW_H)));
      break;
    }
  }
}

export function startMouse() {
  // En fase de captura: al asignar el botón del ratón, ese clic no debe hacer nada más.
  for (const t of ['mousedown', 'mouseup', 'auxclick'] as const) window.addEventListener(t, (e) => { learnMouse(e); }, true);
  window.addEventListener('contextmenu', onContextMenu);
  window.addEventListener('mouseup', onMouseButton);
  window.addEventListener('mousedown', onMouseButton);
  window.addEventListener('auxclick', (e) => { if (e.button === 3 || e.button === 4) e.preventDefault(); });
  window.addEventListener('wheel', onWheel, { passive: false });
}

/* ---------- mando ---------- */
const held: Record<string, { t0: number; last: number }> = {};
const REPEAT = ['up', 'down', 'left', 'right', 'lb', 'rb', 'lt', 'rt'];
/** Repetición al mantener pulsado: espera 300 ms, repite cada 85 ms y acelera hasta 30 ms tras ~2 s más. */
const repeatEvery = (heldMs: number) => Math.max(30, 85 - Math.max(0, heldMs - 300) / 36);

const PAD_KEYS = ['a', 'b', 'x', 'y', 'lb', 'rb', 'lt', 'rt', 'view', 'menu', 'up', 'down', 'left', 'right'] as const;

/** Estado combinado de todos los mandos conectados: vale cualquiera, aunque se desconecte uno o entre otro. */
function padState(pads: Gamepad[]): Record<string, boolean> {
  const st: Record<string, boolean> = {};
  for (const p of pads) {
    const B = (i: number) => !!p.buttons[i]?.pressed, ax = p.axes || [];
    const s: Record<string, boolean> = {
      a: B(0), b: B(1), x: B(2), y: B(3), lb: B(4), rb: B(5), lt: B(6), rt: B(7), view: B(8), menu: B(9),
      up: B(12) || ax[1] < -0.55, down: B(13) || ax[1] > 0.55, left: B(14) || ax[0] < -0.55, right: B(15) || ax[0] > 0.55,
    };
    for (const k of PAD_KEYS) st[k] = st[k] || s[k];
  }
  return st;
}

/** Eje vertical del stick derecho de los mandos conectados (el de mayor recorrido), con zona muerta; -1 = arriba. */
function rightStickY(pads: Gamepad[]): number {
  let v = 0;
  for (const p of pads) {
    const a = p.axes?.[3] ?? 0;
    if (Math.abs(a) > Math.abs(v)) v = a;
  }
  const DEAD = 0.3;
  return Math.abs(v) < DEAD ? 0 : Math.sign(v) * ((Math.abs(v) - DEAD) / (1 - DEAD));
}

let lastPoll = 0;
/** Instante del último paso de tamaño con el stick derecho (0 = suelto). */
let stickZ = 0;
function poll(t: number) {
  const dt = Math.min(64, t - lastPoll);
  lastPoll = t;
  // Se programa el siguiente fotograma antes de nada: un error en un comando no debe parar el mando para siempre.
  requestAnimationFrame(poll);
  let pads: Gamepad[] = [];
  try {
    pads = navigator.getGamepads ? [...navigator.getGamepads()].filter((g): g is Gamepad => !!g && g.connected) : [];
  } catch { pads = []; }
  app.padConnected = pads.length > 0;
  if (!pads.length) {
    // Sin mandos se olvida lo pulsado: al volver uno, sus botones cuentan como pulsaciones nuevas.
    for (const k in held) delete held[k];
    return;
  }
  const st = padState(pads);
  // Visor: el stick derecho acerca (arriba) o aleja (abajo) de forma continua.
  if (app.view === 'viewer' && !app.overlay && !app.osk.open && !app.learn && !app.audioMenu) {
    const ay = rightStickY(pads);
    if (ay) zoomView(Math.exp(-ay * dt * 0.0022));
  }
  // Lista de juegos: el stick derecho cambia el tamaño de las portadas por pasos (arriba = más grandes), con repetición lenta.
  if (app.view === 'titles' && !app.overlay && !app.osk.open && !app.learn && !app.audioMenu) {
    const ay = rightStickY(pads);
    if (!ay) stickZ = 0;
    else if (!stickZ || t - stickZ > 480) { const first = !stickZ; stickZ = t; zoomTitles(-Math.sign(ay), first); }
  } else stickZ = 0;
  for (const k of PAD_KEYS) {
    if (st[k]) {
      try {
        if (!held[k]) { held[k] = { t0: t, last: t }; cmd(k); }
        else if (REPEAT.includes(k) && t - held[k].t0 > 300 && t - held[k].last > repeatEvery(t - held[k].t0)) { held[k].last = t; cmd(k); }
      } catch (e) { console.error(e); }
    } else delete held[k];
  }
}

export function startGamepad() {
  requestAnimationFrame(poll);
  window.addEventListener('gamepadconnected', () => toast('Mando conectado'));
  window.addEventListener('gamepaddisconnected', () => toast('Mando desconectado'));
  // Al volver a la ventana se reescanea: así una portada nueva o cambiada a mano actualiza su acento automático.
  window.addEventListener('focus', () => { if (!app.scanning) { pendingRefresh = true; flushRefresh(); } });
}


/** Avisos del núcleo al arrancar (base de datos que no se pudo abrir, restauración aplicada). true = se mostró alguno. */
async function announceStartup(): Promise<boolean> {
  try {
    const list = await backend.startupNotices();
    if (!list.length) return false;
    const err = list.find((n) => n.kind === 'error');
    // Un error de la base (nada se guardaría) se deja bastante tiempo; se toca el aviso para quitarlo.
    toast((err ?? list[0]).message, err ? 20000 : 6000);
    if (err) console.error(err.message);
    return true;
  } catch { return false; }
}

/** Al arrancar: si hay frases pendientes de repaso, un aviso (se repasan desde Ajustes › Lectura o la ficha del juego). */
async function announceDue() {
  if (!backend.isTauri) return;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const n = await invoke<number>('phrases_due_count', { gameId: null });
    if (n > 0) toast(`Tienes ${n} ${n === 1 ? 'frase' : 'frases'} para repasar (Ajustes › Lectura)`, 4200);
  } catch { /* sin glosario todavia */ }
}
