// Colecciones (antes sagas): pestaña «Colecciones» de cada categoría, «Editar colección», selector de juegos y
// «Añadir a una colección». Una colección es de una sola categoría y solo guarda ids de juego: no mueve capturas.
// Usa las funciones del controlador (foco, toast, paneles).
import { app, accentOf } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { confirmTwice } from './confirm';
import { RAINBOW, type Accent } from './icons';
import { hueOf, inkDark } from './oklch';
import { variablesCSS } from './atmos';
import { openOsk } from './osk.svelte';
import { bump, closeOverlay, go, openTitle, openTitlesMenu, playTitle, popOverlay, pushOverlay, setFocus, toast } from './controller.svelte';
import { closeOrConfirm, inkFor } from './ops.svelte';
import { SHELF_CW, SHELF_PITCH } from './milista.svelte';
import { autoHex, coverColors, gameYear, harmonize, railGradient, suggestGroups, suggestName, titleHex, topHues, yearLayout } from './collections';
import type { Collection, Library, Title } from './types';

const norm = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
const byName = new Intl.Collator('es', { numeric: true, sensitivity: 'base' });

/* ------------------------------------------------------------------ */
/* Carga y guardado                                                     */
/* ------------------------------------------------------------------ */

/** Lee las colecciones. Si quedan sagas antiguas sin categoría, con la biblioteca ya cargada se convierten (una sola
 *  vez: el núcleo las reparte por categoría y guarda el resultado). */
export async function loadCollections(lib: Library) {
  try {
    let list = await backend.getCollections();
    if (list.some((c) => !c.platform)) {
      const map: Record<string, string> = {};
      for (const p of lib.platforms) for (const t of p.titles) if (t.id && !t.members) map[t.id] = p.id;
      for (const h of lib.hidden) map[h.id] = h.platform;
      if (Object.keys(map).length) list = await backend.convertCollections(map);
    }
    app.collections = list;
  } catch (e) { console.error(e); }
}

async function saveColls(list: Collection[]): Promise<boolean> {
  try {
    app.collections = await backend.saveCollections(list);
    return true;
  } catch (e) {
    toast(String(e), 3600);
    return false;
  }
}

/* ------------------------------------------------------------------ */
/* Colores y aspecto de la pestaña                                      */
/* ------------------------------------------------------------------ */

/** Color de una colección: el elegido o, en automático, el derivado de los acentos de sus portadas. */
export function collAccent(coll: Collection, games: Title[]): Accent {
  const m = coll.color != null ? RAINBOW[coll.color] : undefined;
  if (m) return m;
  const hex = autoHex(games);
  return hex ? accentOf(hex, inkDark(hex), 'Automático') : app.chrome;
}

/** Color de una estantería de «Colecciones» (el de su colección, con las portadas de sus juegos). */
export const shelfAccent = (sh: { coll: Collection; items: number[] }): Accent =>
  collAccent(sh.coll, sh.items.map((ti) => app.platform!.titles[ti]));

/** Color de la estantería enfocada (el de «Nueva colección», o sin estanterías, es el de la app): acento de las pestañas superiores. */
export function collFocusAccent(): Accent {
  const sh = app.collShelves[app.collAt.c];
  return sh ? shelfAccent(sh) : app.chrome;
}

/** Color del formulario «Nueva colección» / «Editar colección» y de su selector de juegos: el color elegido o el de la app. */
export function formAccent(): Accent {
  const c = app.collForm.color;
  return c !== null && RAINBOW[c] ? RAINBOW[c] : app.chrome;
}

/** Aspecto del rótulo de la estantería según el estilo elegido en Ajustes: Aurora (manchas de color dentro) o multicolor
 *  (degradado). La tinta es siempre la del color de la colección (el rótulo encendido), también para los claros (amarillo). */
export function tabLook(games: Title[], acc: Accent): { style: string; aurora: string[]; ink: string } {
  const ink = inkFor(acc.c);
  if (app.config.collectionsStyle === 'multi') {
    let cols = coverColors(games, 0.6, 0.15, 6);
    if (cols.length === 1) cols = [cols[0], cols[0]];
    if (!cols.length) return { style: '', aurora: [], ink };
    return { style: `background:linear-gradient(90deg,${cols.join(',')})`, aurora: [], ink };
  }
  return { style: '', aurora: coverColors(games, 0.72, 0.16, 5), ink };
}

/** Variables de acento completo (`--g1 --g2 --f1 --f2 --gf --v1 --v2 --gv --on`…) de un juego de la categoría abierta. */
export function gameVars(ti: number): string {
  const { h1, h2 } = app.titlePair(app.p, ti);
  return Object.entries(variablesCSS(h1, h2)).map(([k, v]) => `${k}:${v}`).join(';');
}

/** Variables de color (`--g1 --g2 --f1 --f2 --gf --on`…) de una estantería: foco de las portadas, anillo y rótulo. Salen de
 *  los 2 tonos más repetidos entre las portadas de la colección; si tiene un color elegido a mano, gana ese. */
export function shelfVars(sh: { coll: Collection; items: number[] }, acc: Accent = shelfAccent(sh)): string {
  let h1 = hueOf(acc.c), h2 = (h1 + 35) % 360;
  if (sh.coll.color == null || !RAINBOW[sh.coll.color]) {
    const top = topHues(sh.items.map((ti) => app.platform!.titles[ti]));
    if (top) [h1, h2] = top;
  }
  return Object.entries(variablesCSS(h1, h2)).map(([k, v]) => `${k}:${v}`).join(';');
}

/** Capturas del juego enfocado que enseña la ficha de «Colecciones». */
export const COLL_GAME_SHOTS = 3;
const shotRng = $state({ seed: Math.floor(Math.random() * 1e9) || 1 });
/** Semilla actual para muestrear capturas aleatorias de la colección. */
export const collShotSeed = () => shotRng.seed;
/** Cambia la semilla para mostrar otro lote de capturas aleatorias de la colección. */
export function shuffleCollShots() {
  shotRng.seed = ((shotRng.seed * 1664525 + 1013904223) >>> 0) || 1;
  blip(880);
}

/** Línea de tiempo de una estantería: posición de cada portada (por año), si lleva marca de año, su color y el degradado del raíl. */
export function collLayout(sh: { items: number[] }) {
  const titles = app.platform?.titles ?? [];
  if (!sh.items.length) return { xs: [0], marks: [false], totalW: SHELF_CW, tints: [null] as Array<string | null>, rail: null as string | null };
  const L = yearLayout(sh.items.map((ti) => gameYear(titles[ti])), SHELF_PITCH, SHELF_CW);
  const tints = sh.items.map((ti) => { const h = titleHex(titles[ti]); return h ? harmonize(h, 0.72, 0.16) : null; });
  return { ...L, tints, rail: railGradient(L.xs, SHELF_CW, tints, L.totalW) };
}

/* ------------------------------------------------------------------ */
/* Navegación por las estanterías                                       */
/* ------------------------------------------------------------------ */

/** La pestaña «Colecciones» de una categoría está a la vista (sin paneles encima). */
export const collTab = () => app.view === 'titles' && app.titlesTab === 'coll';
/** Portadas de una estantería: una estantería sin juegos enseña un hueco para añadirlos. */
const cols = (c: number) => Math.max(1, app.collShelves[c]?.items.length ?? 1);
/** Clave de foco de la posición actual (la última estantería es «Nueva colección»). */
export const collKey = (at = app.collAt) => (at.c >= app.collShelves.length ? 'cnew' : `cl${at.c}.${at.i}`);
export function collValid(key: string): boolean {
  if (key === 'cnew') return true;
  const m = /^cl(\d+)\.(\d+)$/.exec(key);
  return !!m && +m[1] < app.collShelves.length && +m[2] < cols(+m[1]);
}
export const collFirst = () => (app.collShelves.length ? 'cl0.0' : 'cnew');
/** Fija la posición a partir de una clave de foco (la llama `setFocus`). */
export function setCollFocus(key: string) {
  if (key === 'cnew') { app.collAt = { c: app.collShelves.length, i: 0 }; return; }
  const m = /^cl(\d+)\.(\d+)$/.exec(key);
  if (m) app.collAt = { c: +m[1], i: +m[2] };
}

/** Portadas por fila de la colección a la vista: la pantalla las reparte para que quepan y lo apunta aquí,
 *  así arriba y abajo saltan de fila en fila. */
const grid = $state({ perRow: 1 });
export function setCollPerRow(n: number) { if (n !== grid.perRow) grid.perRow = Math.max(1, n); }

/** Vuelve a empezar al entrar en una categoría. */
export function resetColl() {
  app.collAt = { c: 0, i: 0 };
  app.collTop = 0;
  app.collFrom = {};
  shotRng.seed = Math.floor(Math.random() * 1e9) || 1;
}

const flash = () => document.querySelector('#screen .fx.f');

/** Izquierda/derecha recorren las portadas en orden; arriba/abajo saltan de fila (a la última portada si la fila
 *  de abajo es más corta). En «Nueva colección», arriba vuelve a la última colección. */
function collMove(dx: number, dy: number) {
  const nsh = app.collShelves.length;
  const { c, i } = app.collAt;
  if (c >= nsh) {
    if (dy < 0 && nsh) { collShift(-1); return; }
    bump(flash(), dx, dy); return;
  }
  const n = cols(c);
  const pr = Math.max(1, Math.min(grid.perRow, n));
  let ni = i;
  if (dx) ni = i + dx;
  if (dy > 0) ni = Math.floor(i / pr) < Math.floor((n - 1) / pr) ? Math.min(n - 1, i + pr) : -1;
  if (dy < 0) ni = i - pr;
  if (ni < 0 || ni >= n) { bump(flash(), dx, dy); return; }
  setFocus(collKey({ c, i: ni }));
  blip(880);
}

/** LB/RB: colección anterior o siguiente (después de la última está «Nueva colección»). */
function collShift(d: number) {
  const c = app.collAt.c + d;
  if (c < 0 || c > app.collShelves.length) { bump(flash(), d, 0); return; }
  setFocus(collKey({ c, i: 0 }));
  blip(700);
}

/** Mando y teclado en «Colecciones». Devuelve true si el botón se ha atendido aquí. */
export function collCmd(c: string): boolean {
  const at = app.collAt;
  const sh = app.collShelves[at.c];
  switch (c) {
    case 'left': collMove(-1, 0); return true;
    case 'right': collMove(1, 0); return true;
    case 'up': collMove(0, -1); return true;
    case 'down': collMove(0, 1); return true;
    case 'lb': collShift(-1); return true;
    case 'rb': collShift(1); return true;
    case 'a':
      if (!sh) openCollForm();
      else if (!sh.items.length) openCollForm(sh.coll.id);
      else openTitle(sh.items[at.i]);
      return true;
    case 'x': {
      const ti = sh?.items[at.i];
      const t = ti !== undefined ? app.platform?.titles[ti] : undefined;
      if (t?.launchable) void playTitle(t); else bump(flash(), 0, 0);
      return true;
    }
    case 'menu':
      if (sh?.items.length) app.t = sh.items[at.i];
      openTitlesMenu();
      return true;
    case 'y':
      if (sh) openCollForm(sh.coll.id); else bump(flash(), 0, 0);
      return true;
    case 'view': {
      const ti = sh?.items[at.i];
      const n = ti !== undefined ? (app.platform?.titles[ti]?.shots.length ?? 0) : 0;
      if (n > COLL_GAME_SHOTS) shuffleCollShots(); else bump(flash(), 0, 0);
      return true;
    }
  }
  return false;
}

/** Abre la vista de capturas de toda la colección `k` (o directamente el visor si se pasa `shotPath`). Con `ti`,
 *  la vista sale filtrada a ese juego (se puede volver a «Todos» desde la propia vista). */
export function openCollShots(k: number, shotPath?: string, ti?: number) {
  const sh = app.collShelves[k];
  if (!sh) return;
  const titles = app.platform!.titles;
  if (ti !== undefined ? !titles[ti]?.shots.length : !sh.items.some((i) => titles[i].shots.length)) {
    toast(ti !== undefined ? 'Este juego aún no tiene capturas' : 'Esta colección aún no tiene capturas');
    return;
  }
  app.collAt = { c: k, i: Math.min(app.collAt.i, cols(k) - 1) };
  app.collOpen = sh.coll.id;
  app.favView = false; app.sagaFilter = -1; app.sortMode = app.config.shotsOrder;
  if (ti !== undefined) app.sagaFilter = Math.max(-1, app.collTitle?.members?.findIndex((m) => m.id === titles[ti].id) ?? -1);
  if (shotPath) {
    const idx = app.cur.shots.findIndex((s) => s.path === shotPath);
    app.s = idx >= 0 ? idx : 0;
    app.fk.shots = 'shot' + app.s;
    app.viewerFrom = 'shots';
    go('viewer', 'fwd');
    return;
  }
  app.s = 0;
  app.fk.shots = 'shot0';
  go('shots', 'fwd');
}

/* ------------------------------------------------------------------ */
/* Nueva colección / Editar colección                                   */
/* ------------------------------------------------------------------ */
export type CollRow = 'name' | 'games' | 'color' | 'up' | 'down' | 'del' | 'save';

/** Juegos visibles de una categoría (los que se pueden meter en una colección). */
export const platformGames = (platformId: string): Title[] =>
  (app.lib.platforms.find((p) => p.id === platformId)?.titles ?? []).filter((t) => !t.hidden && !t.members && t.id);

export const collFormRows = (): CollRow[] =>
  app.collForm.id ? ['name', 'games', 'color', 'up', 'down', 'del', 'save'] : ['name', 'games', 'color', 'save'];

/** Abre «Editar colección» (con `editId`) o «Nueva colección». `preset`: juegos ya elegidos (desde «Añadir a una colección»). */
export function openCollForm(editId?: string, preset?: string[]) {
  const c = editId ? app.collections.find((x) => x.id === editId) : undefined;
  const platform = c?.platform ?? app.platform?.id ?? '';
  const games = platformGames(platform);
  const known = new Set(games.map((t) => t.id));
  let sugg: Array<{ name: string; ids: string[] }> = [];
  if (!c && !preset) {
    const taken = new Set(app.collections.filter((x) => x.platform === platform).flatMap((x) => x.members));
    sugg = suggestGroups(games.filter((t) => !taken.has(t.id)));
  }
  const members = c ? c.members.filter((id) => known.has(id)) : (preset ?? []).filter((id) => known.has(id));
  app.collForm = {
    id: c?.id ?? null, platform, name: c?.name ?? '', nameTouched: !!c, members,
    keep: c ? c.members.filter((id) => !known.has(id)) : [], color: c?.color ?? null,
    step: sugg.length ? 'suggest' : 'form', sugg, row: 0, confirm: false, dirty: false,
  };
  if (!c && preset) autoName();
  pushOverlay('collform');
  blip(1320);
}

function autoName() {
  const f = app.collForm;
  if (f.nameTouched) return;
  const by = new Map(platformGames(f.platform).map((t) => [t.id, t.name] as const));
  f.name = suggestName(f.members.map((id) => by.get(id) ?? '').filter(Boolean));
}

export async function saveCollForm() {
  const f = app.collForm;
  const name = f.name.trim();
  if (!name) { toast('Escribe un nombre para la colección'); f.row = 0; return; }
  if (!f.members.length) { toast('Elige al menos un juego'); f.row = 1; return; }
  const id = f.id ?? `c${Date.now().toString(36)}`;
  const entry: Collection = { id, name, members: [...f.members, ...f.keep], platform: f.platform, color: f.color };
  const list = f.id ? app.collections.map((c) => (c.id === f.id ? entry : c)) : [...app.collections, entry];
  if (!(await saveColls(list))) return;
  closeOverlay();
  toast(f.id ? `Colección «${name}» actualizada` : `Colección «${name}» creada`);
  blip(1320);
  // Si se está viendo la pestaña, el foco va a la colección guardada.
  const k = app.collShelves.findIndex((s) => s.coll.id === id);
  if (app.view === 'titles' && app.titlesTab === 'coll' && k >= 0) { app.collAt = { c: k, i: 0 }; setFocus(collKey()); }
}

async function deleteColl() {
  const f = app.collForm;
  if (!f.id) return;
  if (!confirmTwice(f)) return;
  if (!(await saveColls(app.collections.filter((c) => c.id !== f.id)))) return;
  closeOverlay();
  toast('Colección borrada · tus capturas no se han tocado');
  if (app.view === 'titles' && app.titlesTab === 'coll') {
    app.collAt = { c: Math.min(app.collAt.c, app.collShelves.length), i: 0 };
    setFocus(collKey());
  }
}

/** Sube (-1) o baja (+1) la colección entre las de su categoría. */
async function moveColl(d: number) {
  const f = app.collForm;
  const list = [...app.collections];
  const at = list.findIndex((c) => c.id === f.id);
  let to = at + d;
  while (to >= 0 && to < list.length && list[to].platform !== f.platform) to += d;
  if (at < 0 || to < 0 || to >= list.length) { toast(d < 0 ? 'Ya es la primera' : 'Ya es la última'); return; }
  [list[at], list[to]] = [list[to], list[at]];
  if (!(await saveColls(list))) return;
  if (app.view === 'titles' && app.titlesTab === 'coll') {
    const k = app.collShelves.findIndex((s) => s.coll.id === f.id);
    if (k >= 0) { app.collAt = { c: k, i: Math.min(app.collAt.i, cols(k) - 1) }; }
  }
  blip(1100);
}

const COLORS = [null, 0, 1, 2, 3, 4, 5, 6] as const;
export function setFormColor(color: number | null) {
  const f = app.collForm;
  if (f.color === color) return;
  f.color = color;
  f.dirty = true;
  blip(880);
}

export function collFormAct(row: number) {
  const f = app.collForm;
  const r = collFormRows()[row];
  f.row = row;
  if (r !== 'del') f.confirm = false;
  if (r === 'name') {
    openOsk({ label: 'Nombre de la colección', value: f.name, capitalize: true, onDone: (v) => { f.name = v.trim(); f.nameTouched = true; f.dirty = true; } });
  } else if (r === 'games') openPicker();
  else if (r === 'color') {
    const i = COLORS.indexOf(f.color as never);
    setFormColor(COLORS[(i + 1) % COLORS.length]);
  } else if (r === 'up') void moveColl(-1);
  else if (r === 'down') void moveColl(1);
  else if (r === 'del') void deleteColl();
  else if (r === 'save') void saveCollForm();
}

/** A en una sugerencia (o «En blanco») al crear. */
export function collSuggestPick(i: number) {
  const f = app.collForm;
  const s = f.sugg[i];
  if (s) { f.name = s.name; f.nameTouched = true; f.members = [...s.ids]; f.dirty = true; }
  f.step = 'form';
  f.row = 0;
  blip(1100);
}

export function collFormCmd(c: string) {
  const f = app.collForm;
  if (f.step === 'suggest') {
    const n = f.sugg.length;
    if (c === 'up') { f.row = Math.max(0, f.row - 1); blip(880); }
    else if (c === 'down') { f.row = Math.min(n, f.row + 1); blip(880); }
    else if (c === 'a') collSuggestPick(f.row < n ? f.row : -1);
    else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
    return;
  }
  const rows = collFormRows();
  if (c === 'up') { f.row = Math.max(0, f.row - 1); f.confirm = false; blip(880); }
  else if (c === 'down') { f.row = Math.min(rows.length - 1, f.row + 1); f.confirm = false; blip(880); }
  else if ((c === 'left' || c === 'right') && rows[f.row] === 'color') {
    const i = COLORS.indexOf(f.color as never) + (c === 'left' ? -1 : 1);
    if (i < 0 || i >= COLORS.length) { blip(220); return; }
    setFormColor(COLORS[i]);
  } else if (c === 'a') collFormAct(f.row);
  else if (c === 'menu') void saveCollForm();
  else if (c === 'b') closeOrConfirm(f.dirty);
}

/* ------------------------------------------------------------------ */
/* Selector de juegos                                                   */
/* ------------------------------------------------------------------ */
export const PICK_COLS = 9;
/** Filas de portadas que caben a la vez. */
export const PICK_ROWS = 3;

export function openPicker() {
  const f = app.collForm;
  app.picker = { q: '', ids: [...f.members], filter: 'all', at: 0, start: f.members.join('|') };
  pushOverlay('picker');
  blip(1320);
}

/** Juegos que se ven ahora: los de la categoría que casan con la búsqueda (y, en «Seleccionados», solo los marcados). */
export function pickerResults(): Title[] {
  const p = app.picker;
  const q = norm(p.q.trim());
  const sel = new Set(p.ids);
  return platformGames(app.collForm.platform)
    .filter((t) => (p.filter === 'all' || sel.has(t.id)) && (!q || norm(t.name).includes(q)))
    .sort((a, b) => byName.compare(a.name, b.name));
}

function pickToggle(t: Title) {
  const p = app.picker;
  const at = p.ids.indexOf(t.id);
  if (at >= 0) p.ids.splice(at, 1); else p.ids.push(t.id);
  // En «Seleccionados», desmarcar quita el juego de la lista: el foco no se queda fuera de ella.
  p.at = Math.max(0, Math.min(p.at, pickerResults().length - 1));
  blip(at >= 0 ? 700 : 1100);
}

/** X: marca todos los resultados de la búsqueda (si ya estaban todos marcados, los desmarca). */
function pickAll() {
  const p = app.picker;
  const res = pickerResults();
  if (!res.length) { blip(220); return; }
  const all = res.every((t) => p.ids.includes(t.id));
  if (all) { const gone = new Set(res.map((t) => t.id)); p.ids = p.ids.filter((id) => !gone.has(id)); }
  else for (const t of res) if (!p.ids.includes(t.id)) p.ids.push(t.id);
  toast(all ? `${res.length} desmarcados` : `${res.length} marcados`, 1200);
  blip(1100);
}

export function pickerSetFilter(f: 'all' | 'sel') {
  if (app.picker.filter === f) return;
  app.picker.filter = f;
  app.picker.at = 0;
  blip(700);
}

export function pickerSearch() {
  const p = app.picker;
  const prev = p.q;
  openOsk({ label: 'Buscar juego', hint: 'La lista se filtra mientras escribes', value: prev, onChange: (v) => { p.q = v; p.at = 0; }, onDone: (v) => { p.q = v; p.at = 0; }, onCancel: () => { p.q = prev; } });
}

export function pickerSave() {
  const f = app.collForm;
  const p = app.picker;
  if (p.ids.join('|') !== p.start) { f.members = [...p.ids]; f.dirty = true; autoName(); }
  popOverlay();
  blip(1320);
}

export function pickerAct(i: number) {
  const t = pickerResults()[i];
  if (!t) return;
  app.picker.at = i;
  pickToggle(t);
}

export function pickerCmd(c: string) {
  const p = app.picker;
  const n = pickerResults().length;
  const move = (to: number) => { const a = Math.max(0, Math.min(n - 1, to)); if (a === p.at) blip(220); else { p.at = a; blip(880); } };
  if (c === 'left') move(p.at - 1);
  else if (c === 'right') move(p.at + 1);
  else if (c === 'up') move(p.at - PICK_COLS);
  else if (c === 'down') move(p.at + PICK_COLS);
  else if (c === 'lb') move(p.at - PICK_COLS * PICK_ROWS);
  else if (c === 'rb') move(p.at + PICK_COLS * PICK_ROWS);
  else if (c === 'a') pickerAct(p.at);
  else if (c === 'x') pickAll();
  else if (c === 'y') pickerSearch();
  else if (c === 'lt' || c === 'rt') { pickerSetFilter(p.filter === 'all' ? 'sel' : 'all'); }
  else if (c === 'menu') pickerSave();
  else if (c === 'b') closeOrConfirm(p.ids.join('|') !== p.start);
}

/* ------------------------------------------------------------------ */
/* Añadir a una colección                                               */
/* ------------------------------------------------------------------ */

/** Colecciones de la categoría abierta (para «Añadir a una colección»). */
export const platformCollections = () => app.collections.filter((c) => c.platform === app.platform?.id);

/** Abre «Añadir a una colección» para el juego `ti` de la categoría abierta. */
export function openAddColl(ti: number) {
  const t = app.platform?.titles[ti];
  if (!t || t.members || !t.id) { toast('Este elemento no se puede añadir a una colección'); return; }
  app.addColl = { ti, row: 0 };
  pushOverlay('addcoll');
  blip(1320);
}

export async function addCollAct(row: number) {
  const t = app.platform?.titles[app.addColl.ti];
  const list = platformCollections();
  if (!t) { closeOverlay(); return; }
  if (row >= list.length) { closeOverlay(); openCollForm(undefined, [t.id]); return; }
  const c = list[row];
  if (c.members.includes(t.id)) { toast(`«${t.name}» ya está en «${c.name}»`); return; }
  if (!(await saveColls(app.collections.map((x) => (x.id === c.id ? { ...x, members: [...x.members, t.id] } : x))))) return;
  closeOverlay();
  toast(`«${t.name}» añadido a «${c.name}»`);
  blip(1320);
}

export function addCollCmd(c: string) {
  const a = app.addColl;
  const last = platformCollections().length;
  if (c === 'up') { a.row = Math.max(0, a.row - 1); blip(880); }
  else if (c === 'down') { a.row = Math.min(last, a.row + 1); blip(880); }
  else if (c === 'a') void addCollAct(a.row);
  else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}
