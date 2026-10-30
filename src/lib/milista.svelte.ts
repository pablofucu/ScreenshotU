// «Mi lista»: estanterías de la categoría por estado, barra rápida de estado, juegos parados y orden de pendientes.
// Usa las funciones del controlador (foco, toast, paneles).
import { app, lastUse, STATUS_LABEL, STATUS_ORDER, type ListShelf } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { openAddColl } from './coleccion.svelte';
import { bump, closeOverlay, openTitle, playTitle, popOverlay, pushOverlay, setFocus, toast } from './controller.svelte';
import { fmtShort, nGames } from './format';
import type { GameStatus, Title } from './types';

/** Portadas visibles por estantería y paso (portada + hueco) en píxeles. */
export const LIST_VIS = 9;
export const SHELF_CW = 104;
export const SHELF_CH = 139;
export const SHELF_PITCH = 120;
/** Alto de una estantería (pestaña + balda) y hueco hasta la siguiente. */
export const SHELF_STEP = 241;
/** Estanterías que caben a la vez (la enfocada siempre se ve). */
export const SHELF_ROWS = 2;
const MONTHS = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const DAY = 864e5;

/* ------------------------------------------------------------------ */
/* Navegación por las estanterías                                       */
/* ------------------------------------------------------------------ */

export interface MonthGroup {
  m: number;
  label: string;
  x0: number;
  x1: number;
  has: boolean;
}

export interface ShelfPositions {
  xs: number[];
  groups: MonthGroup[];
  dividers: number[];
  totalW: number;
}

export const MONTH_NAMES = [
  'enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio',
  'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre',
];
const GAP_MONTH = 28;
export const SHELF_VIEW_W = 1097;

/** Calcula la posición horizontal de cada portada con separación extra de ~28px al cambiar de mes y los divisores centrados. */
export function computeMonthPositions(items: number[], titles: Title[], focusIdx = -1): ShelfPositions {
  const xs: number[] = [];
  const groups: MonthGroup[] = [];
  const dividers: number[] = [];
  let x = 0;
  let prevM = -1;

  for (let k = 0; k < items.length; k++) {
    const t = titles[items[k]];
    const m = t?.finishedMs ? new Date(t.finishedMs).getMonth() : 0;
    if (k > 0 && m !== prevM) {
      x += GAP_MONTH;
      const prevRight = xs[k - 1] + SHELF_CW;
      const nextLeft = x;
      dividers.push((prevRight + nextLeft) / 2);
    }
    if (k === 0 || m !== prevM) {
      groups.push({
        m,
        label: MONTH_NAMES[m],
        x0: x,
        x1: x + SHELF_CW,
        has: k === focusIdx,
      });
    } else {
      groups[groups.length - 1].x1 = x + SHELF_CW;
      if (k === focusIdx) groups[groups.length - 1].has = true;
    }
    xs.push(x);
    x += SHELF_PITCH;
    prevM = m;
  }

  const totalW = xs.length ? xs[xs.length - 1] + SHELF_CW : 0;
  return { xs, groups, dividers, totalW };
}

/** Desplazamiento horizontal en píxeles para estanterías con huecos variables (pestaña «Terminados»). */
export function revealShelfPx(
  i: number,
  xs: number[],
  from: number,
  s: number,
  top: number,
): { from: number; top: number } {
  const totalW = xs.length ? xs[xs.length - 1] + SHELF_CW : 0;
  const maxScroll = Math.max(0, totalW - SHELF_VIEW_W);
  let sx = from;
  if (i > 0) {
    if (xs[i - 1] < sx) sx = xs[i - 1];
  } else if (i === 0) {
    if (xs[0] < sx) sx = 0;
  }
  if (i < xs.length - 1) {
    if (xs[i + 1] + SHELF_CW > sx + SHELF_VIEW_W) sx = xs[i + 1] + SHELF_CW - SHELF_VIEW_W;
  } else if (i === xs.length - 1) {
    if (xs[i] + SHELF_CW > sx + SHELF_VIEW_W) sx = xs[i] + SHELF_CW - SHELF_VIEW_W;
  }
  sx = Math.max(0, Math.min(sx, maxScroll));

  let t = top;
  if (s < t) t = s;
  else if (s > t + SHELF_ROWS - 1) t = s - SHELF_ROWS + 1;
  return { from: sx, top: t };
}

/** La pestaña «Mi lista» o «Terminados» de una categoría normal está a la vista (sin paneles encima). */
export const listTab = () => app.view === 'titles' && (app.titlesTab === 'list' || app.titlesTab === 'fin');
/** Algún estante tiene este título (el foco de «Mi lista» solo cae en juegos que se ven). */
export const listHas = (ti: number) => app.shelves.some((s) => s.items.includes(ti));
/** Primer juego de «Mi lista» (foco por defecto). */
export const listFirst = (): number | undefined => app.shelves[0]?.items[0];

/** Estantería y posición del juego enfocado. */
function place(ti = app.t): { s: number; i: number } | null {
  const sh = app.shelves;
  for (let s = 0; s < sh.length; s++) {
    const i = sh[s].items.indexOf(ti);
    if (i >= 0) return { s, i };
  }
  return null;
}

/** Qué estantería se ve (`top`) y desde qué portada (`from`) para que la portada `i` de la estantería `s` (de `n`
 *  portadas) quede a la vista, con un vecino a cada lado si se puede. Lo comparten «Mi lista» y «Colecciones». */
export function revealShelf(i: number, n: number, from: number, s: number, top: number): { from: number; top: number } {
  let f = from;
  if (i < f + 1) f = Math.max(0, i - 1);
  else if (i > f + LIST_VIS - 2) f = Math.min(Math.max(0, n - LIST_VIS), i - LIST_VIS + 2);
  f = Math.max(0, Math.min(f, Math.max(0, n - LIST_VIS)));
  let t = top;
  if (s < t) t = s;
  else if (s > t + SHELF_ROWS - 1) t = s - SHELF_ROWS + 1;
  return { from: f, top: t };
}

/** Desplaza lo justo para que el juego enfocado se vea, con un vecino a cada lado si se puede. */
export function listReveal() {
  const at = place();
  if (!at) return;
  const sh = app.shelves[at.s];
  if (!sh) return;
  const f0 = app.listFrom[sh.key] ?? 0;
  let r: { from: number; top: number };
  if (app.titlesTab === 'fin' && sh.year > 0) {
    const titles = app.platform?.titles ?? [];
    const pos = computeMonthPositions(sh.items, titles, at.i);
    r = revealShelfPx(at.i, pos.xs, f0, at.s, app.listTop);
  } else {
    r = revealShelf(at.i, sh.items.length, f0, at.s, app.listTop);
  }
  if (r.from !== f0) app.listFrom = { ...app.listFrom, [sh.key]: r.from };
  if (r.top !== app.listTop) app.listTop = r.top;
}

/** Vuelve a empezar al entrar en una categoría. */
export function resetList() {
  app.listFrom = {};
  app.listTop = 0;
}

function listMove(dx: number, dy: number) {
  const at = place();
  const sh = app.shelves;
  if (!at) {
    const f = listFirst();
    if (f !== undefined) setFocus('tit' + f);
    return;
  }
  let s = at.s, i = at.i;
  // Diseño dual: arriba «Jugando» (izquierda) y «En pausa» (derecha); abajo «Pendientes» a todo el ancho.
  // Se navega por tipo de estantería, porque las vacías no existen y los índices cambian.
  const of = (k: string) => sh.findIndex((x) => x.kind === k);
  const play = of('playing'), drop = of('dropped'), pend = of('pending');
  const kind = sh[s].kind;
  const top = [play, drop].filter((x) => x >= 0);
  if (dx && (kind === 'playing' || kind === 'dropped')) {
    const k = top.indexOf(s);
    if (i + dx >= 0 && i + dx < sh[s].items.length) i += dx;
    else if (top[k + dx] !== undefined) { s = top[k + dx]; i = dx > 0 ? 0 : sh[s].items.length - 1; }
  } else if (dx) {
    i += dx;
  } else if (dy > 0 && kind !== 'pending') {
    if (pend >= 0) { s = pend; i = Math.min(i * 2, sh[s].items.length - 1); }
    else { bump(document.querySelector('#screen .fx.f'), dx, dy); return; }
  } else if (dy < 0 && kind === 'pending') {
    if (!top.length) { bump(document.querySelector('#screen .fx.f'), dx, dy); return; }
    s = i < 3 || top.length === 1 ? top[0] : top[1];
    i = Math.min(s === top[0] ? i : Math.max(0, i - 3), sh[s].items.length - 1);
  } else {
    bump(document.querySelector('#screen .fx.f'), dx, dy);
    return;
  }
  if (!sh[s] || i < 0 || i >= sh[s].items.length) {
    bump(document.querySelector('#screen .fx.f'), dx, dy);
    return;
  }
  setFocus('tit' + sh[s].items[i]);
  blip(880);
}

/** LB/RB en «Mi lista»: salta varias portadas. */
function listShift(d: number) {
  const at = place();
  const sh = app.shelves;
  if (!at) return;
  const shelf = sh[at.s];
  const ni = Math.max(0, Math.min(shelf.items.length - 1, at.i + d * (LIST_VIS - 1)));
  if (ni === at.i) { bump(document.querySelector('#screen .fx.f'), d, 0); return; }
  setFocus('tit' + shelf.items[ni]);
  blip(700);
}

/** Mando y teclado en «Mi lista». Devuelve true si el botón se ha atendido aquí. */
export function listCmd(c: string): boolean {
  switch (c) {
    case 'left': listMove(-1, 0); return true;
    case 'right': listMove(1, 0); return true;
    case 'up': listMove(0, -1); return true;
    case 'down': listMove(0, 1); return true;
    case 'lb': listShift(-1); return true;
    case 'rb': listShift(1); return true;
    case 'a':
      if (listHas(app.t)) openTitle(app.t); else bump(document.querySelector('#screen .fx.f'), 0, 0);
      return true;
    case 'x': {
      const t = app.platform?.titles[app.t];
      if (t && listHas(app.t) && t.launchable) void playTitle(t); else bump(document.querySelector('#screen .fx.f'), 0, 0);
      return true;
    }
    case 'y':
      if (listHas(app.t)) openStatusBar(app.t); else bump(document.querySelector('#screen .fx.f'), 0, 0);
      return true;
  }
  return false;
}

/* ------------------------------------------------------------------ */
/* Textos de las estanterías                                            */
/* ------------------------------------------------------------------ */
const fmtHours = (sec: number) => {
  if (sec < 60) return '';
  if (sec < 3600) return `${Math.floor(sec / 60)} min`;
  const h = sec / 3600;
  return `${h < 10 ? h.toFixed(1).replace('.', ',') : Math.round(h)} h`;
};
const shortDate = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MONTHS[d.getMonth()]}`; };

/** Línea «N juegos» y dato útil de la pestaña de una estantería. */
export function shelfSub(sh: ListShelf, titles: Title[]): string {
  const base = nGames(sh.items.length);
  if (sh.kind === 'playing') {
    const h = fmtHours(sh.items.reduce((a, i) => a + titles[i].playSeconds, 0));
    return h ? `${base} · ${h} jugadas` : base;
  }
  if (sh.kind === 'finished') {
    const rated = sh.items.map((i) => titles[i].rating ?? 0).filter((r) => r > 0);
    if (rated.length) return `${base} · nota media ${(rated.reduce((a, r) => a + r, 0) / rated.length).toFixed(1).replace('.', ',')}`;
  }
  return base;
}

/** Rótulo de una estantería de «Terminados»: juego enfocado o resumen de juegos y horas (sin notas). */
export function finShelfSub(sh: ListShelf, isFocused: boolean, curTitle: Title | null | undefined, titles: Title[]): string {
  if (isFocused && curTitle) {
    if (curTitle.finishedMs) {
      const d = new Date(curTitle.finishedMs);
      return `${curTitle.name} · ${d.getDate()} de ${MONTH_NAMES[d.getMonth()]}`;
    }
    return curTitle.name;
  }
  const base = nGames(sh.items.length);
  const totalSec = sh.items.reduce((a, i) => a + (titles[i]?.playSeconds ?? 0), 0);
  const h = fmtHours(totalSec);
  return h ? `${base} · ${h} jugadas` : base;
}

/** Etiqueta bajo la portada: horas (jugando), posición (pendientes), fecha y estrellas (terminados) o la última actividad. */
export function itemLabel(sh: ListShelf, t: Title, k: number): { label: string; stars: number } {
  if (sh.kind === 'playing') return { label: fmtHours(t.playSeconds), stars: 0 };
  if (sh.kind === 'pending') return { label: '#' + (k + 1), stars: 0 };
  if (sh.kind === 'finished') return { label: t.finishedMs ? shortDate(t.finishedMs) : '', stars: t.rating ?? 0 };
  if (sh.kind === 'dropped') {
    return { label: t.lastPlayed ? shortDate(t.lastPlayed) : t.shots[0] ? fmtShort(t.shots[0].date) : '', stars: 0 };
  }
  return { label: '', stars: 0 };
}

/* ------------------------------------------------------------------ */
/* Cambiar el estado de un juego                                        */
/* ------------------------------------------------------------------ */

/** Cambia títulos de la biblioteca en memoria (sin volver a escanear): `fn` devuelve los campos a cambiar o null. */
function patchTitles(fn: (t: Title) => Partial<Title> | null) {
  app.lib = {
    ...app.lib,
    platforms: app.lib.platforms.map((p) => {
      let changed = false;
      const titles = p.titles.map((t) => {
        const d = t.id && !t.members ? fn(t) : null;
        if (!d) return t;
        changed = true;
        return { ...t, ...d };
      });
      return changed ? { ...p, titles } : p;
    }),
  };
}

/** Fija el estado de un juego (con fecha y nota si es «terminado»). false = no se pudo guardar. */
export async function setStatus(t: Title, status: GameStatus, opts: { finishedMs?: number | null; rating?: number } = {}): Promise<boolean> {
  const id = t.id;
  const was = t.status ?? 'none';
  const rating = status === 'finished' && opts.rating && opts.rating >= 1 && opts.rating <= 5 ? opts.rating : null;
  try {
    await backend.setTitleStatus(id, status, opts.finishedMs ?? null, rating ?? 0);
  } catch (e) {
    toast(String(e), 3200);
    return false;
  }
  const now = Date.now();
  const next = app.lib.platforms.reduce((m, p) => Math.max(m, ...p.titles.map((x) => x.listPos ?? 0)), 0) + 1;
  patchTitles((x) => x.id !== id ? null : {
    status: status === 'none' ? null : status,
    finishedMs: status === 'finished' ? (opts.finishedMs ?? (was === 'finished' && x.finishedMs ? x.finishedMs : now)) : null,
    rating,
    listPos: status === 'pending' ? (x.listPos ?? next) : null,
    statusMs: was === status ? x.statusMs : now,
    snoozeMs: null,
  });
  return true;
}

/** En «Mi lista», el juego cambia de estantería al cambiar de estado: la vista lo sigue para que no quede fuera. */
function followFocus() {
  if (listTab() && listHas(app.t)) listReveal();
}

/** Al lanzar un juego desde el visor: un pendiente pasa a «Jugando» (si el ajuste está activado). */
export function onLaunched(t: Title) {
  if (app.config.listAutoPlay && t.status === 'pending') void setStatus(t, 'playing');
}

/* ------------------------------------------------------------------ */
/* Barra rápida de estado                                               */
/* ------------------------------------------------------------------ */
const dim = (y: number, m: number) => new Date(y, m, 0).getDate();

/** Abre la barra de estado del título `ti` de la categoría actual. */
export function openStatusBar(ti: number) {
  const t = app.platform?.titles[ti];
  if (!t || t.members || !t.id) { toast('Este elemento no tiene estado'); return; }
  const when = new Date(t.status === 'finished' && t.finishedMs ? t.finishedMs : Date.now());
  app.statusBar = {
    title: ti, row: Math.max(0, STATUS_ORDER.indexOf((t.status ?? 'none') as GameStatus)), step: 'pick', field: 0,
    y: when.getFullYear(), m: when.getMonth() + 1, d: when.getDate(), rating: t.status === 'finished' ? (t.rating ?? 0) : 0,
  };
  pushOverlay('status');
  blip(1320);
}

/** Sube (+1) o baja (-1) el valor del campo elegido del paso «Terminado». La fecha no pasa de hoy. */
function stepField(dir: number) {
  const sb = app.statusBar;
  const today = new Date();
  if (sb.field === 0) {
    const n = dim(sb.y, sb.m);
    sb.d = ((sb.d - 1 + dir + n) % n) + 1;
  } else if (sb.field === 1) {
    sb.m = ((sb.m - 1 + dir + 12) % 12) + 1;
    sb.d = Math.min(sb.d, dim(sb.y, sb.m));
  } else if (sb.field === 2) {
    sb.y = Math.max(1990, Math.min(today.getFullYear(), sb.y + dir));
    sb.d = Math.min(sb.d, dim(sb.y, sb.m));
  } else {
    sb.rating = Math.max(0, Math.min(5, sb.rating + dir));
  }
  if (new Date(sb.y, sb.m - 1, sb.d) > today) { sb.y = today.getFullYear(); sb.m = today.getMonth() + 1; sb.d = today.getDate(); }
  blip(880);
}

/** Guarda «Terminado» con fecha y nota. `thenColl`: después abre «Añadir a una colección» para ese juego. */
async function statusBarSave(thenColl = false) {
  const sb = app.statusBar;
  const t = app.platform?.titles[sb.title];
  if (!t) { closeOverlay(); return; }
  const now = new Date();
  const isToday = sb.y === now.getFullYear() && sb.m - 1 === now.getMonth() && sb.d === now.getDate();
  const old = t.status === 'finished' && t.finishedMs ? new Date(t.finishedMs) : null;
  const same = !!old && old.getFullYear() === sb.y && old.getMonth() + 1 === sb.m && old.getDate() === sb.d;
  const ms = same ? t.finishedMs! : isToday ? Date.now() : new Date(sb.y, sb.m - 1, sb.d, 12).getTime();
  if (await setStatus(t, 'finished', { finishedMs: ms, rating: sb.rating })) {
    closeOverlay();
    followFocus();
    toast(`«${t.name}»: Terminado${sb.rating ? ' · ' + '★'.repeat(sb.rating) : ''}`);
    blip(1320);
    if (thenColl) openAddColl(sb.title);
  }
}

/** Confirma el estado elegido: «Terminado» pide antes fecha y nota; el resto se guarda ya. */
export async function statusBarPick() {
  const sb = app.statusBar;
  const t = app.platform?.titles[sb.title];
  if (!t) { closeOverlay(); return; }
  const st = STATUS_ORDER[sb.row];
  if (st === 'finished') { sb.step = 'finish'; sb.field = 0; blip(1100); return; }
  if (await setStatus(t, st)) {
    closeOverlay();
    followFocus();
    toast(`«${t.name}»: ${STATUS_LABEL[st]}`);
    blip(1320);
  }
}

export function statusBarCmd(c: string) {
  const sb = app.statusBar;
  if (sb.step === 'pick') {
    if (c === 'left') { sb.row = Math.max(0, sb.row - 1); blip(880); }
    else if (c === 'right') { sb.row = Math.min(STATUS_ORDER.length - 1, sb.row + 1); blip(880); }
    else if (c === 'a') void statusBarPick();
    else if (c === 'b' || c === 'menu' || c === 'y') { popOverlay(); blip(520); }
    return;
  }
  if (c === 'left') { sb.field = Math.max(0, sb.field - 1); blip(880); }
  else if (c === 'right') { sb.field = Math.min(3, sb.field + 1); blip(880); }
  else if (c === 'up') stepField(1);
  else if (c === 'down') stepField(-1);
  else if (c === 'a') void statusBarSave();
  else if (c === 'y') void statusBarSave(true);
  else if (c === 'b' || c === 'menu') { sb.step = 'pick'; blip(520); }
}

/* ------------------------------------------------------------------ */
/* Juegos parados                                                       */
/* ------------------------------------------------------------------ */

/** Juegos en «Jugando» cuyo último «jugar» (última partida o, si no hay, la captura más reciente) es anterior a N días. */
export function stalledGames(): Array<{ t: Title; days: number }> {
  const now = Date.now();
  const cutoff = now - app.config.listStalledDays * DAY;
  const out: Array<{ t: Title; days: number }> = [];
  for (const p of app.lib.platforms) {
    for (const t of p.titles) {
      if (t.status !== 'playing' || t.hidden || t.members || !t.id) continue;
      const use = lastUse(t);
      // Desde que se marcó «Jugando» también cuenta: marcarlo hoy no lo deja parado al instante.
      const ref = Math.max(use, t.statusMs ?? 0);
      if (!ref || ref >= cutoff) continue;
      // «Dejarlo» vale hasta que se vuelva a jugar.
      if (t.snoozeMs && use <= t.snoozeMs) continue;
      out.push({ t, days: Math.floor((now - ref) / DAY) });
    }
  }
  return out.sort((a, b) => b.days - a.days);
}

const STALLED_CHOICES: Array<GameStatus | 'keep'> = ['pending', 'dropped', 'keep'];
export const STALLED_LABEL = ['Pendiente', 'Abandonado', 'Dejarlo'];

/** Al arrancar: cambia solos los juegos parados o, con «Preguntar», abre una sola tarjeta con todos. */
export async function checkStalled() {
  if (!app.config.listStalled) return;
  const list = stalledGames();
  if (!list.length) return;
  const act = app.config.listStalledAction;
  if (act !== 'ask') {
    let n = 0;
    for (const { t } of list) if (await setStatus(t, act)) n++;
    if (n) toast(`${n === 1 ? '1 juego parado pasa' : `${n} juegos parados pasan`} a ${STATUS_LABEL[act]}`, 5200);
    return;
  }
  // Sin molestar: la tarjeta espera a que la app esté en Inicio y sin paneles abiertos (unos segundos como mucho).
  for (let k = 0; k < 20; k++) {
    if (app.view === 'home' && !app.overlay && !app.osk.open && !app.playing && !app.learn) break;
    await new Promise((r) => setTimeout(r, 3000));
    if (k === 19) return;
  }
  const again = stalledGames();
  if (!again.length) return;
  app.stalled = { rows: again.map(({ t, days }) => ({ id: t.id, name: t.name, days, choice: 2 })), row: 0 };
  pushOverlay('stalled');
}

async function stalledApply() {
  const rows = app.stalled.rows;
  const byId = new Map<string, Title>();
  for (const p of app.lib.platforms) for (const t of p.titles) if (t.id && !t.members) byId.set(t.id, t);
  let moved = 0;
  const keep: string[] = [];
  for (const r of rows) {
    const t = byId.get(r.id);
    const ch = STALLED_CHOICES[r.choice];
    if (!t) continue;
    if (ch === 'keep') keep.push(r.id);
    else if (await setStatus(t, ch)) moved++;
  }
  if (keep.length) {
    try {
      await backend.snoozeStalled(keep);
      const ids = new Set(keep);
      const now = Date.now();
      patchTitles((t) => (ids.has(t.id) ? { snoozeMs: now } : null));
    } catch (e) { toast(String(e), 3200); }
  }
  closeOverlay();
  toast(moved ? `${moved} ${moved === 1 ? 'juego cambiado' : 'juegos cambiados'}${keep.length ? `; ${keep.length} se quedan como están` : ''}` : 'Todo se queda como está', 3200);
  blip(1320);
}

export function stalledCmd(c: string) {
  const s = app.stalled;
  const n = s.rows.length;
  if (c === 'up') { s.row = Math.max(0, s.row - 1); blip(880); }
  else if (c === 'down') { s.row = Math.min(n, s.row + 1); blip(880); }
  else if (c === 'left' && s.rows[s.row]) { s.rows[s.row].choice = Math.max(0, s.rows[s.row].choice - 1); blip(880); }
  else if (c === 'right' && s.rows[s.row]) { s.rows[s.row].choice = Math.min(2, s.rows[s.row].choice + 1); blip(880); }
  else if (c === 'a') {
    if (s.row >= n) void stalledApply();
    else { s.row = Math.min(n, s.row + 1); blip(880); }
  } else if (c === 'b' || c === 'menu') { closeOverlay(); blip(520); }
}
export { stalledApply };
