// Acciones de usuario añadidas sobre el controlador: asistente «Añadir juego», juegos ocultos, importar y mover
// capturas, selección múltiple y selector de color de acento. Usa las funciones del controlador (toast, refrescos...).
import { app, type FormKind, type MenuItem, type MoveTarget } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { openOsk } from './osk.svelte';
import { RAINBOW } from './icons';
import { accentForHue, hueOf, inkDark } from './oklch';
import {
  back, closeOverlay, go, markRefresh, popOverlay, pushOverlay, setFocus, softRefresh, toast, hideToast,
} from './controller.svelte';
import type { HiddenEntry, NewGame, Saga, Shot, Title } from './types';
import { realPlatforms, SAGA_PLATFORM, suggestName, withSagas } from './sagas';

export const curTitle = (): Title | undefined => app.platform?.titles[app.t];
const isImagePath = (p: string) => /\.(png|jpe?g|webp)$/i.test(p);
const baseName = (p: string) => (p.split(/[\\/]/).pop() ?? '').replace(/\.[^.]+$/, '').replace(/[_]+/g, ' ').trim();
const shortFile = (p: string) => p.split(/[\\/]/).pop() ?? p;

/** Sustituye un título de la biblioteca en memoria (sin volver a escanear). */
export function patchTitle(pi: number, ti: number, patch: Partial<Title>) {
  app.lib = {
    ...app.lib,
    platforms: app.lib.platforms.map((p, i) => (i !== pi ? p : { ...p, titles: p.titles.map((t, j) => (j === ti ? { ...t, ...patch } : t)) })),
  };
}

/* ------------------------------------------------------------------ */
/* Asistente «Añadir juego»                                             */
/* ------------------------------------------------------------------ */
export const ADD_ITEMS = ['exe', 'emu', 'dir', 'captures'] as const;
export type AddItem = (typeof ADD_ITEMS)[number];
let addPlatformHint: string | undefined;

/** `platformId`: plataforma sugerida (la de la lista desde la que se abre). */
export function openAdd(platformId?: string) {
  addPlatformHint = platformId;
  app.addRow = 0;
  pushOverlay('add');
  blip(1320);
}

export function addAct(i: number) {
  const it = ADD_ITEMS[i];
  if (it === 'dir') void openGameDir();
  else openForm(it === 'emu' ? 'emu' : it === 'captures' ? 'captures' : 'exe');
}

export function addCmd(c: string) {
  const n = ADD_ITEMS.length;
  if (c === 'up') { app.addRow = Math.max(0, app.addRow - 1); blip(880); }
  else if (c === 'down') { app.addRow = Math.min(n - 1, app.addRow + 1); blip(880); }
  else if (c === 'a') addAct(app.addRow);
  else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}

/* ---- formulario ---- */
export type FormRow = 'exe' | 'rom' | 'name' | 'args' | 'platform' | 'hero' | 'logo' | 'cover' | 'save';
const ART: FormRow[] = ['hero', 'logo', 'cover'];
export const isArtRow = (r: FormRow) => ART.includes(r);
export function formRows(kind: FormKind = app.form.kind): FormRow[] {
  if (kind === 'emu') return ['exe', 'rom', 'name', 'args', 'platform', ...ART, 'save'];
  if (kind === 'captures') return ['name', 'platform', ...ART, 'save'];
  return ['exe', 'name', 'args', 'platform', ...ART, 'save'];
}

export function openForm(kind: FormKind) {
  const plats = app.config.platforms;
  const hint = addPlatformHint && plats.some((p) => p.id === addPlatformHint) ? addPlatformHint : undefined;
  // Un emulador es de Switch salvo que se añada desde la lista de otra plataforma que no sea la de PC.
  const want = kind === 'emu' ? (hint && hint !== 'pc' ? hint : 'switch') : (hint ?? 'pc');
  app.form = {
    kind, exe: '', rom: '', name: '', args: kind === 'emu' ? '"{rom}"' : '',
    platform: plats.find((p) => p.id === want)?.id ?? plats[0]?.id ?? 'pc', nameTouched: false, hero: '', logo: '', cover: '',
  };
  app.formRow = 0;
  pushOverlay('form');
  blip(1320);
}

function cycleIn<T>(list: T[], cur: T, d: number): T {
  const i = Math.max(0, list.indexOf(cur));
  return list[(i + d + list.length) % list.length];
}

export async function formAct(row: number, dir: number) {
  const id = formRows()[row];
  const f = app.form;
  if (id === 'exe' && dir === 0) {
    const r = await backend.pickFile('exe');
    if (r) { f.exe = r; if (!f.nameTouched && f.kind === 'exe') f.name = baseName(r); }
  } else if (id === 'rom' && dir === 0) {
    const r = await backend.pickFile('rom');
    if (r) { f.rom = r; if (!f.nameTouched) f.name = baseName(r); }
  } else if (id === 'name' && dir === 0) {
    openOsk({ label: 'Nombre del juego', value: f.name, capitalize: true, hint: 'El nombre decide la carpeta de capturas', onDone: (v) => { f.name = v.trim(); f.nameTouched = true; } });
  } else if (id === 'args' && dir === 0) {
    openOsk({ label: 'Argumentos', value: f.args, maxLen: 200, onDone: (v) => { f.args = v; } });
  } else if (id === 'platform') {
    f.platform = cycleIn(app.config.platforms.map((p) => p.id), f.platform, dir || 1);
    blip(880);
  } else if ((id === 'hero' || id === 'logo' || id === 'cover') && dir === 0) {
    const r = await backend.pickFile('image');
    if (r) f[id] = r;
  } else if (id === 'save' && dir === 0) {
    await saveForm();
  }
}

/** X sobre una fila: quita el valor. */
function clearRow(row: number) {
  const f = app.form;
  const id = formRows()[row];
  if (isArtRow(id)) f[id as 'hero' | 'logo' | 'cover'] = '';
  else if (id === 'exe') f.exe = '';
  else if (id === 'rom') f.rom = '';
  else if (id === 'name') { f.name = ''; f.nameTouched = false; }
  else if (id === 'args') f.args = '';
  blip(520);
}

async function saveForm() {
  const f = app.form;
  if ((f.kind === 'exe' || f.kind === 'emu') && !f.exe) { toast(f.kind === 'emu' ? 'Elige primero el emulador' : 'Elige primero el ejecutable'); return; }
  if (f.kind === 'emu' && !f.rom) { toast('Elige la ROM o el juego'); return; }
  if (!f.name.trim()) { toast('Escribe un nombre'); return; }
  if ((f.hero || f.logo || f.cover || f.kind === 'captures') && !app.config.root) { toast('Elige primero la carpeta de la biblioteca', 3200); return; }
  const kind: NewGame['kind'] = f.kind === 'emu' ? 'emulator' : f.kind;
  try {
    const rep = await backend.addGame({
      kind, name: f.name.trim(), exe: f.exe, args: f.args, rom: f.rom, platform: f.platform,
      hero: f.hero, logo: f.logo, cover: f.cover,
    });
    app.overlay = ''; app.overlayStack = [];
    toast(`«${f.name.trim()}» añadido${rep.notes.length ? ' · ' + rep.notes[0] : ''}`, rep.notes.length ? 4200 : 1800);
    await refreshAfterAdd(f.platform, f.name.trim());
  } catch (e) {
    toast(String(e), 3600);
  }
}

/** Reescanea y, si se está en una lista de títulos, deja el foco en el juego recién añadido. */
async function refreshAfterAdd(platformId: string, name: string) {
  markRefresh();
  await softRefresh();
  if (app.view !== 'titles') return;
  const pi = app.lib.platforms.findIndex((p) => p.id === platformId);
  if (pi < 0) return;
  const ti = app.lib.platforms[pi].titles.findIndex((t) => t.name.toLowerCase() === name.toLowerCase());
  if (ti >= 0) { app.p = pi; setFocus('tit' + ti); }
}

let discardAt = 0;
/** Cierra un panel con datos sin guardar solo si se pulsa B dos veces seguidas. */
function closeOrConfirm(dirty: boolean) {
  if (dirty && performance.now() - discardAt > 3000) {
    discardAt = performance.now();
    toast('Pulsa B otra vez para descartar los cambios');
    return;
  }
  discardAt = 0;
  popOverlay();
  blip(520);
}

export function formCmd(c: string) {
  const n = formRows().length;
  if (c === 'up') { app.formRow = Math.max(0, app.formRow - 1); blip(880); }
  else if (c === 'down') { app.formRow = Math.min(n - 1, app.formRow + 1); blip(880); }
  else if (c === 'left') void formAct(app.formRow, -1);
  else if (c === 'right') void formAct(app.formRow, 1);
  else if (c === 'a') void formAct(app.formRow, 0);
  else if (c === 'x') clearRow(app.formRow);
  else if (c === 'b' || c === 'menu') {
    const f = app.form;
    closeOrConfirm(!!(f.exe || f.rom || f.args || f.hero || f.logo || f.cover || f.nameTouched));
  }
}

/* ---- carpeta con muchos juegos: asistente por subcarpetas ---- */
export type GdirRow = 'exe' | 'name' | 'platform' | 'add' | 'skip' | 'finish';
export const GDIR_ROWS: GdirRow[] = ['exe', 'name', 'platform', 'add', 'skip', 'finish'];

/** Ejecutable elegido de una subcarpeta ('' si no hay). */
export const gdirExe = (it: { exes: string[]; sel: number; custom: string }) => (it.sel >= 0 ? it.exes[it.sel] ?? '' : it.custom);

export async function openGameDir() {
  const dir = await backend.pickFolder('Carpeta con tus juegos (una subcarpeta por juego)');
  if (!dir) return;
  app.gdir = { loading: true, dir, items: [], i: 0, row: 0, added: 0, known: 0, empty: 0 };
  pushOverlay('gdir');
  blip(1320);
  try {
    const r = await backend.scanGamesDir(dir);
    if (!r.candidates.length) {
      popOverlay();
      toast(r.known ? 'Todos los juegos de esa carpeta ya estaban añadidos' : 'No he encontrado ejecutables en las subcarpetas', 3600);
      return;
    }
    const plat = app.config.platforms.find((p) => p.id === 'pc')?.id ?? app.config.platforms[0]?.id ?? 'pc';
    app.gdir = {
      loading: false, dir, i: 0, row: 0, added: 0, known: r.known, empty: r.empty,
      items: r.candidates.map((c) => ({ folder: c.folder, exes: c.exes, sel: 0, custom: '', name: c.name, platform: plat })),
    };
  } catch (e) {
    popOverlay();
    toast(String(e), 3600);
  }
}

/** Termina el asistente: cierra y refresca la biblioteca si se añadió algo. */
async function finishGameDir() {
  const n = app.gdir.added;
  app.overlay = ''; app.overlayStack = [];
  if (!n) { toast('No se añadió ningún juego'); return; }
  toast(`${n} ${n === 1 ? 'juego añadido' : 'juegos añadidos'}`, 2400);
  markRefresh();
  await softRefresh();
}

async function gdirAdd() {
  const g = app.gdir;
  const it = g.items[g.i];
  const exe = gdirExe(it);
  if (!exe) { toast('Elige el ejecutable'); return; }
  if (!it.name.trim()) { toast('Escribe un nombre'); return; }
  try {
    await backend.addGame({ kind: 'exe', name: it.name.trim(), exe, args: '', rom: '', platform: it.platform, hero: '', logo: '', cover: '' });
    g.added++;
    await gdirNext();
  } catch (e) {
    toast(String(e), 3600);
  }
}

async function gdirNext() {
  const g = app.gdir;
  if (g.i + 1 >= g.items.length) { await finishGameDir(); return; }
  const prev = g.items[g.i].platform;
  g.i++;
  g.items[g.i].platform = prev; // se arrastra la última plataforma elegida
  g.row = 0;
  blip(1320);
}

export async function gdirAct(row: number, dir: number) {
  const g = app.gdir;
  const it = g.items[g.i];
  if (g.loading || !it) return;
  const id = GDIR_ROWS[row];
  if (id === 'exe') {
    if (dir === 0) {
      const r = await backend.pickFile('exe');
      if (r) { it.custom = r; it.sel = -1; }
    } else {
      // Entre los encontrados y, al final, el elegido a mano (si lo hay).
      const n = it.exes.length + (it.custom ? 1 : 0);
      const cur = it.sel >= 0 ? it.sel : it.exes.length;
      const nx = (cur + dir + n) % n;
      it.sel = nx < it.exes.length ? nx : -1;
      blip(880);
    }
  } else if (id === 'name' && dir === 0) {
    openOsk({ label: 'Nombre del juego', value: it.name, capitalize: true, hint: 'El nombre decide la carpeta de capturas', onDone: (v) => { it.name = v.trim() || it.folder; } });
  } else if (id === 'platform') {
    it.platform = cycleIn(app.config.platforms.map((p) => p.id), it.platform, dir || 1);
    blip(880);
  } else if (id === 'add' && dir === 0) await gdirAdd();
  else if (id === 'skip' && dir === 0) await gdirNext();
  else if (id === 'finish' && dir === 0) await finishGameDir();
}

export function gdirCmd(c: string) {
  const g = app.gdir;
  if (c === 'up') { g.row = Math.max(0, g.row - 1); blip(880); }
  else if (c === 'down') { g.row = Math.min(GDIR_ROWS.length - 1, g.row + 1); blip(880); }
  else if (c === 'left') void gdirAct(g.row, -1);
  else if (c === 'right') void gdirAct(g.row, 1);
  else if (c === 'a') void gdirAct(g.row, 0);
  else if (c === 'x') void gdirAct(GDIR_ROWS.indexOf('skip'), 0);
  else if (c === 'b' || c === 'menu') { if (g.added) void finishGameDir(); else popOverlay(); blip(520); }
}

/* ------------------------------------------------------------------ */
/* Juegos ocultos                                                       */
/* ------------------------------------------------------------------ */
export function openHidden() {
  app.hiddenRow = 0;
  pushOverlay('hidden');
  blip(1320);
}

export async function unhideEntry(e: HiddenEntry) {
  try {
    await backend.setHidden(e, false);
    app.lib = { ...app.lib, hidden: app.lib.hidden.filter((h) => h.id !== e.id) };
    app.hiddenRow = Math.max(0, Math.min(app.hiddenRow, app.lib.hidden.length - 1));
    markRefresh();
    toast(`«${e.name}» vuelve a mostrarse`);
  } catch (err) { toast(String(err), 3200); }
}

export function hiddenCmd(c: string) {
  const n = app.lib.hidden.length;
  if (c === 'up') { app.hiddenRow = Math.max(0, app.hiddenRow - 1); blip(880); }
  else if (c === 'down') { app.hiddenRow = Math.min(Math.max(0, n - 1), app.hiddenRow + 1); blip(880); }
  else if (c === 'a') { const e = app.lib.hidden[app.hiddenRow]; if (e) void unhideEntry(e); }
  else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}

/** Oculta o vuelve a mostrar el juego abierto (menú de la ficha). */
export async function toggleHideTitle(t: Title, hide: boolean) {
  const pi = app.p;
  try {
    await backend.setHidden({ id: t.id, name: t.name, platform: app.platform.id }, !!hide);
  } catch (e) { toast(String(e), 3200); return; }
  closeOverlay();
  if (!hide) {
    patchTitle(pi, app.t, { hidden: false });
    toast(`«${t.name}» vuelve a mostrarse`);
    markRefresh();
    return;
  }
  if (app.config.showHidden) {
    patchTitle(pi, app.t, { hidden: true });
    toast(`«${t.name}» oculto (se ve atenuado porque «Mostrar ocultos» está activo)`, 3600);
    markRefresh();
    return;
  }
  toast(`«${t.name}» oculto · Ajustes › Juegos ocultos para mostrarlo`, 3800);
  app.fk.titles = 'tit' + Math.max(0, app.t - 1);
  app.t = Math.max(0, app.t - 1);
  go('titles', 'back');
  markRefresh();
  await softRefresh();
}

/* ------------------------------------------------------------------ */
/* Añadir capturas                                                      */
/* ------------------------------------------------------------------ */
let importing = false;

export async function importInto(t: Title, paths: string[]) {
  if (importing) { toast('Ya se está importando…'); return; }
  if (!t.dir || !app.config.root) { toast('Elige primero la carpeta de la biblioteca', 3200); return; }
  importing = true;
  toast(`Importando 0/${paths.length}…`, 0);
  try {
    const rep = await backend.importShots(t.dir, paths, (d, n) => toast(`Importando ${d}/${n}…`, 0));
    const n = rep.copied.length;
    const sk = rep.skipped.length;
    toast(
      n
        ? `${n} ${n === 1 ? 'captura añadida' : 'capturas añadidas'} a «${t.name}»${sk ? ` · ${sk} omitida${sk === 1 ? '' : 's'}` : ''}`
        : (rep.skipped[0] ?? 'No se añadió ninguna captura'),
      3400,
    );
    if (n) { markRefresh(); await softRefresh(); } else hideToastLater();
  } catch (e) {
    toast(String(e), 3600);
  } finally {
    importing = false;
  }
}
const hideToastLater = () => setTimeout(hideToast, 3400);

export async function addShotsTo(t: Title | undefined) {
  if (!t) return;
  const paths = await backend.pickImages();
  if (paths.length) await importInto(t, paths);
}

/** Archivos soltados sobre la ventana: se añaden al juego abierto. */
export function onFilesDropped(paths: string[]) {
  if (app.osk.open || app.overlay) return;
  const imgs = paths.filter(isImagePath);
  if (!imgs.length) { toast('Solo se pueden añadir imágenes (png, jpg, jpeg o webp)', 3000); return; }
  const t = curTitle();
  if (t?.members) { toast('Abre un juego (no una saga) para soltar capturas encima', 3000); return; }
  if (!t || app.favView || !['detail', 'shots', 'viewer'].includes(app.view)) { toast('Abre un juego para soltar capturas encima', 3000); return; }
  void importInto(t, imgs);
}

/* ------------------------------------------------------------------ */
/* Selección y menú de capturas                                         */
/* ------------------------------------------------------------------ */
export function toggleSel(path: string) {
  if (app.sel.has(path)) app.sel.delete(path); else app.sel.add(path);
  blip(app.sel.has(path) ? 1100 : 700);
}
export function exitSel() {
  app.selMode = false;
  app.sel.clear();
}
function enterSel() {
  app.selMode = true;
  app.sel.clear();
  toast('Selección: A marca cada captura · Start abre el menú', 3000);
}

export function shotMenuItems(): MenuItem[] {
  const it: MenuItem[] = [];
  if (app.view === 'shots') {
    if (!app.selMode) it.push('select', 'move');
    else { it.push('selall'); if (app.sel.size) it.push('move'); it.push('unselect'); }
  } else it.push('move');
  it.push('reveal');
  if (!app.favView) it.push('addshots');
  return it;
}

export function openShotMenu() {
  if (!app.cur.shots.length && !app.platform?.titles[app.t]) return;
  app.menu = { kind: 'shots', title: app.t, row: 0, items: shotMenuItems(), confirm: false };
  pushOverlay('menu');
  blip(1320);
}

/** Acciones del menú de capturas (las de la ficha están en el controlador). */
export async function shotMenuAct(item: MenuItem) {
  const focused = app.cur.shots[app.s];
  if (item === 'select') { closeOverlay(); enterSel(); }
  else if (item === 'unselect') { closeOverlay(); exitSel(); }
  else if (item === 'selall') { closeOverlay(); for (const s of app.cur.shots) app.sel.add(s.path); toast(`${app.sel.size} capturas marcadas`); }
  else if (item === 'move') {
    const paths = app.selMode && app.sel.size ? [...app.sel] : focused ? [focused.path] : [];
    if (paths.length) openMoveTo(paths);
  } else if (item === 'reveal') {
    closeOverlay();
    if (!focused) return;
    try { await backend.reveal(focused.path); if (!backend.isTauri) toast('Mostrar en el Explorador (simulado en el navegador)'); }
    catch (e) { toast(String(e), 3200); }
  } else if (item === 'addshots') {
    closeOverlay();
    await addShotsTo(curTitle());
  }
}

/* ------------------------------------------------------------------ */
/* Mover capturas a otro juego                                          */
/* ------------------------------------------------------------------ */
export function openMoveTo(paths: string[]) {
  const cur = curTitle();
  const exclude = !app.favView && cur ? cur.dir : '';
  const targets: MoveTarget[] = [];
  const order = [...app.lib.platforms.keys()].sort((a, b) => (a === app.p ? -1 : b === app.p ? 1 : a - b));
  for (const pi of order) {
    const p = app.lib.platforms[pi];
    for (const t of p.titles) {
      if (!t.dir || t.dir === exclude || t.hidden) continue;
      targets.push({ id: t.id, name: t.name, platform: p.id, platformName: p.name, dir: t.dir });
    }
  }
  if (!targets.length) { toast('No hay otro juego al que mover'); return; }
  app.moveTo = { paths, targets, row: 0, confirm: false };
  pushOverlay('moveto');
  blip(1320);
}

async function doMove() {
  const m = app.moveTo;
  const tg = m.targets[m.row];
  if (!tg) return;
  const fromViewer = app.view === 'viewer';
  try {
    const rep = await backend.moveShots(m.paths, tg.dir);
    for (const [o, n] of rep.moved) if (app.favs.has(o)) { app.favs.delete(o); app.favs.add(n); }
    app.overlay = ''; app.overlayStack = [];
    exitSel();
    const n = rep.moved.length;
    toast(
      n
        ? `${n} ${n === 1 ? 'captura movida' : 'capturas movidas'} a «${tg.name}»${rep.failed.length ? ` · ${rep.failed.length} con error` : ''}`
        : (rep.failed[0] ?? 'No se movió nada'),
      3600,
    );
    if (n) {
      if (fromViewer) back();
      markRefresh();
      await softRefresh();
    }
  } catch (e) {
    toast(String(e), 3600);
  }
}

export function moveCmd(c: string) {
  const m = app.moveTo;
  const n = m.targets.length;
  if (c === 'up') { m.row = Math.max(0, m.row - 1); m.confirm = false; blip(880); }
  else if (c === 'down') { m.row = Math.min(n - 1, m.row + 1); m.confirm = false; blip(880); }
  else if (c === 'lb') { m.row = Math.max(0, m.row - 6); m.confirm = false; blip(700); }
  else if (c === 'rb') { m.row = Math.min(n - 1, m.row + 6); m.confirm = false; blip(700); }
  else if (c === 'a') {
    if (!m.confirm) { m.confirm = true; blip(520); } else void doMove();
  } else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}

/* ------------------------------------------------------------------ */
/* Color de acento de un juego                                          */
/* ------------------------------------------------------------------ */
/** Filas disponibles del selector: 0 automático, 1 paleta del arte (si hay), 2 arcoíris, 3 tono. */
export function accentRows(): number[] {
  return app.accentPick.palette.length ? [0, 1, 2, 3] : [0, 2, 3];
}
export function accentRowLen(row: number): number {
  return row === 1 ? app.accentPick.palette.length : row === 2 ? RAINBOW.length : 1;
}
export function accentPickColor(): string | null {
  const a = app.accentPick;
  if (a.row === 0) return null;
  if (a.row === 1) return a.palette[a.col]?.color ?? null;
  if (a.row === 2) return RAINBOW[a.col].c;
  return accentForHue(a.hue).color;
}
function preview() { app.accentPreview = app.accentPick.row === 0 ? 'auto' : accentPickColor(); }

export function openAccentPicker() {
  const t = curTitle();
  if (!t) return;
  const palette = t.accent?.palette ?? [];
  const ov = t.accentOverride;
  const hue = ov ? hueOf(ov) : t.accent ? hueOf(t.accent.accent.color) : 220;
  let row = 0, col = 0;
  if (ov) {
    const ip = palette.findIndex((s) => s.color.toLowerCase() === ov.toLowerCase());
    const ir = RAINBOW.findIndex((s) => s.c.toLowerCase() === ov.toLowerCase());
    if (ip >= 0) { row = 1; col = ip; } else if (ir >= 0) { row = 2; col = ir; } else row = 3;
  }
  app.accentPick = { title: app.t, row, col, hue, palette };
  pushOverlay('accent');
  preview();
  blip(1320);
}

export function accentFocus(row: number, col = 0) {
  const a = app.accentPick;
  a.row = row; a.col = Math.min(col, accentRowLen(row) - 1);
  preview();
}

export function cancelAccent() {
  app.accentPreview = null;
  popOverlay();
  blip(520);
}

export async function applyAccent() {
  const t = curTitle();
  const color = accentPickColor();
  if (!t) return;
  try {
    await backend.setAccentOverride(t.id, color);
  } catch (e) { toast(String(e), 3200); return; }
  patchTitle(app.p, app.t, { accentOverride: color });
  app.accentPreview = null;
  closeOverlay();
  toast(color ? 'Color de acento fijado' : 'Color de acento: automático');
  blip(1320);
}

export function accentCmd(c: string) {
  const a = app.accentPick;
  const rows = accentRows();
  if (c === 'up' || c === 'down') {
    const i = rows.indexOf(a.row) + (c === 'up' ? -1 : 1);
    if (i < 0 || i >= rows.length) { blip(220); return; }
    accentFocus(rows[i], a.col);
    blip(880);
  } else if (c === 'left' || c === 'right') {
    const d = c === 'left' ? -1 : 1;
    if (a.row === 3) { a.hue = (a.hue + d * 4 + 360) % 360; preview(); }
    else if (a.row === 0) blip(220);
    else {
      const n = accentRowLen(a.row);
      const nc = a.col + d;
      if (nc < 0 || nc >= n) { blip(220); return; }
      a.col = nc; preview(); blip(880);
    }
  } else if (c === 'a') void applyAccent();
  else if (c === 'b' || c === 'menu') cancelAccent();
}

/* ------------------------------------------------------------------ */
/* Sagas (agrupaciones virtuales de juegos)                             */
/* ------------------------------------------------------------------ */
export interface SagaCandidate { id: string; name: string; platform: string }

/** Juegos que se pueden meter en una saga: todos los visibles de las categorías reales. */
export function sagaCandidates(): SagaCandidate[] {
  return realPlatforms(app.lib).flatMap((p) => p.titles.filter((t) => !t.hidden).map((t) => ({ id: t.id, name: t.name, platform: p.name })));
}

/** Aplica la lista de sagas a la biblioteca en memoria (sin reescanear). */
export function applySagas(list: Saga[]) {
  app.sagas = list;
  app.lib = withSagas(app.lib, list);
}

export function openSagaForm(editId?: string) {
  const s = editId ? app.sagas.find((x) => x.id === editId) : undefined;
  app.sagaForm = { id: s?.id ?? null, name: s?.name ?? '', nameTouched: !!s, members: [...(s?.members ?? [])], row: 0 };
  pushOverlay('saga');
  blip(1320);
}

function autoName() {
  const f = app.sagaForm;
  if (f.nameTouched) return;
  const names = sagaCandidates().filter((c) => f.members.includes(c.id)).map((c) => c.name);
  f.name = suggestName(names);
}

export async function saveSagaForm() {
  const f = app.sagaForm;
  const name = f.name.trim();
  if (!name) { toast('Escribe un nombre para la saga'); app.sagaForm.row = 0; return; }
  if (f.members.length < 2) { toast('Elige al menos dos juegos'); return; }
  const id = f.id ?? `s${Date.now().toString(36)}`;
  const entry: Saga = { id, name, members: [...f.members] };
  const list = f.id ? app.sagas.map((s) => (s.id === f.id ? entry : s)) : [...app.sagas, entry];
  try {
    const saved = await backend.saveSagas(list);
    applySagas(saved);
  } catch (e) { toast(String(e), 3600); return; }
  app.overlay = ''; app.overlayStack = [];
  toast(f.id ? `Saga «${name}» actualizada` : `Saga «${name}» creada`);
  const pi = app.lib.platforms.findIndex((p) => p.id === SAGA_PLATFORM);
  const ti = app.lib.platforms[pi]?.titles.findIndex((t) => t.id === `saga:${id}`) ?? -1;
  if (app.view === 'titles' && pi === app.p && ti >= 0) setFocus('tit' + ti);
  else if (app.view === 'shots' || app.view === 'detail') { app.sagaFilter = -1; app.s = 0; setFocus('shot0'); }
}

export async function deleteSaga(sagaId: string) {
  const list = app.sagas.filter((s) => s.id !== sagaId);
  try { applySagas(await backend.saveSagas(list)); } catch (e) { toast(String(e), 3600); return; }
  toast('Saga disuelta · tus capturas no se han tocado');
}

/** Abre la saga `i` de la categoría Sagas directamente en la vista unificada. */
export function openSaga(i: number) {
  const t = app.platform?.titles[i];
  if (!t) return;
  app.t = i; app.s = 0; app.favView = false; app.sagaFilter = -1; app.sortMode = 'recent';
  app.fk.shots = 'shot0';
  go('shots', 'fwd');
}

export function openSagaMenu() {
  const t = app.platform?.titles[app.t];
  if (!t?.members) return;
  app.menu = { kind: 'detail', title: app.t, row: 0, items: ['editsaga', 'delsaga'], confirm: false };
  pushOverlay('menu');
  blip(1320);
}

export async function sagaMenuAct(item: MenuItem) {
  const t = app.platform?.titles[app.menu.title];
  if (!t) { closeOverlay(); return; }
  const id = t.id.replace(/^saga:/, '');
  if (item === 'editsaga') { app.overlay = ''; app.overlayStack = []; openSagaForm(id); return; }
  if (!app.menu.confirm) { app.menu.confirm = true; blip(520); return; }
  closeOverlay();
  await deleteSaga(id);
  app.t = Math.max(0, app.t - 1);
  app.fk.titles = 'tit' + app.t;
  go('titles', 'back');
}

export function sagaToggle(i: number) {
  const c = sagaCandidates()[i];
  if (!c) return;
  const m = app.sagaForm.members;
  const at = m.indexOf(c.id);
  if (at >= 0) m.splice(at, 1); else m.push(c.id);
  autoName();
  blip(at >= 0 ? 700 : 1100);
}

export function sagaAct(row: number) {
  const n = sagaCandidates().length;
  if (row === 0) {
    openOsk({ label: 'Nombre de la saga', value: app.sagaForm.name, capitalize: true, onDone: (v) => { app.sagaForm.name = v.trim(); app.sagaForm.nameTouched = true; } });
  } else if (row <= n) sagaToggle(row - 1);
  else void saveSagaForm();
}

export function sagaCmd(c: string) {
  const f = app.sagaForm;
  const last = sagaCandidates().length + 1;
  if (c === 'up') { f.row = Math.max(0, f.row - 1); blip(880); }
  else if (c === 'down') { f.row = Math.min(last, f.row + 1); blip(880); }
  else if (c === 'lb') { f.row = Math.max(0, f.row - 6); blip(700); }
  else if (c === 'rb') { f.row = Math.min(last, f.row + 6); blip(700); }
  else if (c === 'a') sagaAct(f.row);
  else if (c === 'b' || c === 'menu') closeOrConfirm(f.nameTouched || f.members.length > 0);
}

/** Texto de tinta para un color elegido (se usa en el selector). */
export const inkFor = (hex: string) => (inkDark(hex) ? '#3a2f00' : '#fff');
export const fileLabel = (p: string) => (p ? shortFile(p) : '');
export type { Shot };
