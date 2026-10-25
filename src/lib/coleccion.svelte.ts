// Colecciones (antes sagas): pestaña «Colecciones» de cada categoría, «Editar colección», selector de juegos y
// «Añadir a una colección». Una colección es de una sola categoría y solo guarda ids de juego: no mueve capturas.
// Usa las funciones del controlador (foco, toast, paneles).
import { app, accentOf } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { RAINBOW, type Accent } from './icons';
import { inkDark } from './oklch';
import { openOsk } from './osk.svelte';
import { bump, closeOverlay, go, openTitle, popOverlay, pushOverlay, setFocus, toast } from './controller.svelte';
import { closeOrConfirm, inkFor } from './ops.svelte';
import { LIST_VIS, revealShelf } from './milista.svelte';
import { autoHex, coverColors, suggestGroups, suggestName } from './collections';
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

/** Desplaza lo justo para que la portada enfocada se vea, con un vecino a cada lado si se puede. */
export function collReveal() {
  const { c, i } = app.collAt;
  const sh = app.collShelves[c];
  const f0 = sh ? (app.collFrom[sh.coll.id] ?? 0) : 0;
  const r = revealShelf(i, cols(c), f0, c, app.collTop);
  if (sh && r.from !== f0) app.collFrom = { ...app.collFrom, [sh.coll.id]: r.from };
  if (r.top !== app.collTop) app.collTop = r.top;
}

/** Vuelve a empezar al entrar en una categoría. */
export function resetColl() {
  app.collAt = { c: 0, i: 0 };
  app.collTop = 0;
  app.collFrom = {};
}

const flash = () => document.querySelector('#screen .fx.f');

function collMove(dx: number, dy: number) {
  const nsh = app.collShelves.length;
  let { c, i } = app.collAt;
  if (dx) {
    if (c >= nsh) { bump(flash(), dx, 0); return; }
    i += dx;
    if (i < 0 || i >= cols(c)) { bump(flash(), dx, 0); return; }
  }
  if (dy) {
    c += dy;
    if (c < 0 || c > nsh) { bump(flash(), 0, dy); return; }
    i = c >= nsh ? 0 : Math.min(i, cols(c) - 1);
  }
  setFocus(collKey({ c, i }));
  blip(880);
}

/** LB/RB: salta varias portadas de la estantería. */
function collShift(d: number) {
  const { c, i } = app.collAt;
  if (c >= app.collShelves.length) { bump(flash(), d, 0); return; }
  const ni = Math.max(0, Math.min(cols(c) - 1, i + d * (LIST_VIS - 1)));
  if (ni === i) { bump(flash(), d, 0); return; }
  setFocus(collKey({ c, i: ni }));
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
    case 'x':
      if (sh) openCollShots(at.c); else bump(flash(), 0, 0);
      return true;
    case 'y':
      if (sh) openCollForm(sh.coll.id); else bump(flash(), 0, 0);
      return true;
  }
  return false;
}

/** Abre la vista de capturas de toda la colección `k` (LB/RB filtran por juego). */
export function openCollShots(k: number) {
  const sh = app.collShelves[k];
  if (!sh) return;
  if (!sh.items.some((i) => app.platform!.titles[i].shots.length)) { toast('Esta colección aún no tiene capturas'); return; }
  app.collAt = { c: k, i: Math.min(app.collAt.i, cols(k) - 1) };
  app.collOpen = sh.coll.id;
  app.s = 0; app.favView = false; app.sagaFilter = -1; app.sortMode = app.config.shotsOrder;
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
  if (!f.confirm) { f.confirm = true; blip(520); return; }
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
    if (k >= 0) { app.collAt = { c: k, i: Math.min(app.collAt.i, cols(k) - 1) }; collReveal(); }
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
