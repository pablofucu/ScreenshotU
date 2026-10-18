// SteamGridDB: descarga automática de los grafismos que faltan (portada, hero, logo) y selector de alternativas
// (desde Ajustes o desde la ficha del juego). Las imágenes se guardan en la caché de la app; la carpeta del juego gana.
import { app } from './store.svelte';
import { backend, IS_TAURI } from './backend';
import { blip } from './sound';
import { openOsk } from './osk.svelte';
import { closeOverlay, markRefresh, patchConfig, popOverlay, pushOverlay, softRefresh, toast } from './controller.svelte';
import { patchTitle } from './ops.svelte';
import { isSagaPlatform } from './sagas';
import type { Title } from './types';

export type ArtKind = 'cover' | 'hero' | 'logo';
export const ART_KINDS: ArtKind[] = ['cover', 'hero', 'logo'];
export const KIND_LABEL: Record<ArtKind, string> = { cover: 'Portada', hero: 'Hero', logo: 'Logo' };
/** Columnas de la cuadrícula del selector por tipo. */
export const KIND_COLS: Record<ArtKind, number> = { cover: 7, hero: 3, logo: 4 };

export interface SgdbGame { id: number; name: string; year: number | null; verified: boolean }
export interface SgdbImage { id: number; url: string; thumb: string; width: number; height: number; mime: string; style: string; author: string }
interface SgdbEntry { gameId: number | null; gameName: string; autoDone: boolean }
interface DoneEvent { images: number; missing: string[]; error: string | null }

/* ---------- acceso al núcleo (o simulado en el navegador) ---------- */
type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
let inv: Invoke | null = null;
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) return mock(cmd, args) as T;
  inv ??= (await import('@tauri-apps/api/core')).invoke as Invoke;
  return inv<T>(cmd, args);
}

const mockArt = (k: ArtKind, i: number) => {
  const [w, h] = k === 'cover' ? [600, 900] : k === 'hero' ? [1920, 620] : [800, 300];
  const hue = (i * 47) % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}"><rect width="100%" height="100%" fill="${k === 'logo' ? 'none' : `hsl(${hue} 55% 45%)`}"/><text x="50%" y="55%" font-size="${h / 5}" text-anchor="middle" fill="hsl(${hue} 80% 85%)" font-family="sans-serif" font-weight="800">${KIND_LABEL[k]} ${i + 1}</text></svg>`;
  return 'data:image/svg+xml,' + encodeURIComponent(svg);
};
const mockStore = new Map<string, SgdbEntry>();
function mock(cmd: string, a: Record<string, unknown> = {}): unknown {
  if (cmd === 'sgdb_search') return [{ id: 1, name: String(a.term), year: 2020, verified: true }, { id: 2, name: `${a.term} (remaster)`, year: 2023, verified: false }];
  if (cmd === 'sgdb_images') {
    const k = a.kind as ArtKind, p = a.page as number;
    if (p > 1) return [];
    return Array.from({ length: 24 }, (_, j) => { const i = p * 24 + j, u = mockArt(k, i); return { id: 1000 + i, url: u, thumb: u, width: 600, height: 900, mime: 'image/png', style: 'alternate', author: 'mock' }; });
  }
  if (cmd === 'sgdb_info') return mockStore.get(a.titleId as string) ?? null;
  if (cmd === 'sgdb_link') { mockStore.set(a.titleId as string, { gameId: a.gameId as number, gameName: a.gameName as string, autoDone: true }); return null; }
  if (cmd === 'sgdb_apply') return (a.image as SgdbImage).url;
  if (cmd === 'sgdb_local') return mockArt(a.kind as ArtKind, 99);
  if (cmd === 'sgdb_auto') return 0;
  return null;
}

/* ---------- estado ---------- */
export interface SgdbPick {
  pi: number;
  ti: number;
  kind: ArtKind;
  game: { id: number; name: string } | null;
  items: SgdbImage[];
  /** Imagen enfocada. */
  i: number;
  page: number;
  more: boolean;
  loading: boolean;
  error: string;
  /** Lista de juegos encontrados (al buscar otro juego) en vez de imágenes. */
  mode: 'images' | 'games';
  games: SgdbGame[];
  gi: number;
  busy: boolean;
}
class SgdbState {
  pick = $state<SgdbPick>({ pi: 0, ti: 0, kind: 'cover', game: null, items: [], i: 0, page: 0, more: false, loading: false, error: '', mode: 'images', games: [], gi: 0, busy: false });
  /** Lista de juegos de Ajustes → «Cambiar grafismos de un juego…». */
  titlesRow = $state(0);
  /** Progreso de la descarga automática ('' = parada). */
  status = $state('');
}
export const sg = new SgdbState();

const hasKey = () => !!app.config.sgdbKey?.trim();
const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : String(e));

/** Un grafismo está guardado en la caché de SteamGridDB (y no en la carpeta del juego). */
export const isSgdbPath = (p: string | null | undefined) => !!p && (/[\\/]sgdb[\\/]/.test(p) || p.startsWith('data:'));
/** Id de imagen de SteamGridDB de un grafismo en uso (por el nombre `cover-<id>.ext`). */
export function usedImageId(t: Title | undefined, k: ArtKind): number | null {
  const p = t?.[k];
  if (!isSgdbPath(p)) return null;
  const m = /(?:cover|hero|logo)-(\d+)\.\w+$/.exec(p!);
  return m ? +m[1] : null;
}
const missingKinds = (t: Title) => ART_KINDS.filter((k) => !t[k]);

/* ---------- descarga automática ---------- */
let listening = false;
let autoFailed = false;
let loudRun = false;

async function listen() {
  if (listening || !IS_TAURI) return;
  listening = true;
  const { listen } = await import('@tauri-apps/api/event');
  void listen<{ done: number; total: number; name: string }>('sgdb-progress', (e) => {
    sg.status = `${e.payload.done + 1}/${e.payload.total} · ${e.payload.name}`;
    if (loudRun) toast(`SteamGridDB ${sg.status}`, 0);
  });
  void listen<DoneEvent>('sgdb-done', (e) => {
    const d = e.payload;
    sg.status = '';
    if (d.error) {
      autoFailed = true;
      toast(d.error, 4200);
    } else if (d.images) {
      toast(`SteamGridDB: ${d.images} ${d.images === 1 ? 'grafismo nuevo' : 'grafismos nuevos'}`, 2600);
    } else if (loudRun) {
      toast(d.missing.length ? `SteamGridDB no encontró: ${d.missing.slice(0, 3).join(', ')}${d.missing.length > 3 ? '…' : ''}` : 'SteamGridDB: no había nada nuevo', 3200);
    }
    loudRun = false;
    if (d.images) void softRefresh();
  });
}

/** Tras escanear: pide los grafismos que faltan (cada título solo se revisa una vez, salvo `force`). */
export async function sgdbAuto(force = false) {
  if (!hasKey() || (!force && (!app.config.sgdbAuto || autoFailed))) return;
  if (sg.status) { if (force) toast('Ya se están buscando grafismos…'); return; }
  await listen();
  const titles = app.lib.platforms
    .filter((p) => !isSagaPlatform(p))
    .flatMap((p) => p.titles)
    .map((t) => ({ id: t.id, name: t.name, missing: missingKinds(t) }))
    .filter((t) => t.missing.length);
  try {
    loudRun = force;
    const n = await call<number>('sgdb_auto', { titles, force });
    if (n) sg.status = `0/${n}`;
    else { loudRun = false; if (force) toast('Todos los juegos tienen ya sus grafismos'); }
    if (n && force) toast(`Buscando grafismos de ${n} ${n === 1 ? 'juego' : 'juegos'} en SteamGridDB…`, 0);
  } catch (e) {
    loudRun = false;
    if (force) toast(errText(e), 3600);
  }
}

/* ---------- Ajustes ---------- */
export function sgdbKeyLabel() {
  const k = app.config.sgdbKey?.trim() ?? '';
  return k ? `••••${k.slice(-4)}` : 'Sin clave';
}

export function editSgdbKey() {
  openOsk({
    label: 'Clave de API de SteamGridDB',
    hint: 'steamgriddb.com → Preferencias → API. Puedes pegarla con Ctrl+V.',
    value: app.config.sgdbKey ?? '',
    maxLen: 64,
    onDone: (v) => {
      const key = v.replace(/\s+/g, '');
      autoFailed = false;
      void patchConfig({ sgdbKey: key }).then(() => {
        toast(key ? 'Clave guardada' : 'Clave quitada');
        if (key) void sgdbAuto();
      });
    },
  });
}

export function toggleSgdbAuto() {
  autoFailed = false;
  void patchConfig({ sgdbAuto: !app.config.sgdbAuto }).then(() => { if (app.config.sgdbAuto) void sgdbAuto(); });
  blip(1320);
}

/** Juegos que se pueden personalizar (los de las categorías reales). */
export function sgdbTitleRefs(): Array<{ pi: number; ti: number }> {
  const out: Array<{ pi: number; ti: number }> = [];
  app.lib.platforms.forEach((p, pi) => { if (!isSagaPlatform(p)) p.titles.forEach((_, ti) => out.push({ pi, ti })); });
  return out;
}

export function openSgdbTitles() {
  if (!sgdbTitleRefs().length) { toast('Aún no hay juegos en la biblioteca'); return; }
  sg.titlesRow = 0;
  pushOverlay('sgdbtitles');
  blip(1320);
}

export function sgdbTitlesCmd(c: string) {
  const n = sgdbTitleRefs().length;
  if (c === 'up') { sg.titlesRow = Math.max(0, sg.titlesRow - 1); blip(880); }
  else if (c === 'down') { sg.titlesRow = Math.min(n - 1, sg.titlesRow + 1); blip(880); }
  else if (c === 'lb') { sg.titlesRow = Math.max(0, sg.titlesRow - 8); blip(700); }
  else if (c === 'rb') { sg.titlesRow = Math.min(n - 1, sg.titlesRow + 8); blip(700); }
  else if (c === 'a') { const r = sgdbTitleRefs()[sg.titlesRow]; if (r) void openSgdbPicker(r.pi, r.ti); }
  else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}

/* ---------- selector de grafismos ---------- */
export const pickTitle = (): Title | undefined => app.lib.platforms[sg.pick.pi]?.titles[sg.pick.ti];

export async function openSgdbPicker(pi: number, ti: number, kind?: ArtKind) {
  const t = app.lib.platforms[pi]?.titles[ti];
  if (!t) return;
  sg.pick = {
    pi, ti, kind: kind ?? missingKinds(t)[0] ?? 'cover', game: null, items: [], i: 0, page: 0, more: false,
    loading: true, error: '', mode: 'images', games: [], gi: 0, busy: false,
  };
  pushOverlay('sgdb');
  blip(1320);
  if (!hasKey()) {
    // Sin clave solo se pueden elegir imágenes del PC.
    sg.pick.loading = false;
    sg.pick.error = 'Sin clave de API de SteamGridDB (Ajustes → SteamGridDB).\nPulsa View para elegir una imagen del PC.';
    return;
  }
  try {
    const e = await call<SgdbEntry | null>('sgdb_info', { titleId: t.id });
    if (e?.gameId) { sg.pick.game = { id: e.gameId, name: e.gameName }; await loadImages(true); return; }
    const hits = await call<SgdbGame[]>('sgdb_search', { term: t.name });
    if (hits.length) { await chooseGame(hits[0], false); return; }
    sg.pick.loading = false;
    sg.pick.error = `No se encontró «${t.name}» en SteamGridDB. Pulsa Y para buscarlo con otro nombre.`;
  } catch (e) {
    sg.pick.loading = false;
    sg.pick.error = errText(e);
  }
}

let loadSeq = 0;
async function loadImages(reset: boolean) {
  const p = sg.pick;
  if (!p.game) return;
  const seq = ++loadSeq;
  if (reset) { p.items = []; p.i = 0; p.page = 0; p.more = false; }
  p.loading = true; p.error = '';
  const page = reset ? 0 : p.page + 1;
  try {
    const list = await call<SgdbImage[]>('sgdb_images', { gameId: p.game.id, kind: p.kind, page });
    if (seq !== loadSeq) return;
    const seen = new Set(p.items.map((x) => x.id));
    p.items = [...p.items, ...list.filter((x) => !seen.has(x.id))];
    p.page = page;
    p.more = list.length > 0;
    if (reset) {
      // Empieza en la imagen en uso, si está en la primera página.
      const used = usedImageId(pickTitle(), p.kind);
      const k = used === null ? -1 : p.items.findIndex((x) => x.id === used);
      p.i = Math.max(0, k);
    }
  } catch (e) {
    if (seq === loadSeq) p.error = errText(e);
  } finally {
    if (seq === loadSeq) p.loading = false;
  }
}

async function chooseGame(g: SgdbGame, user: boolean) {
  const t = pickTitle();
  if (!t) return;
  sg.pick.game = { id: g.id, name: g.name };
  sg.pick.mode = 'images';
  try { await call('sgdb_link', { titleId: t.id, gameId: g.id, gameName: g.name }); } catch (e) { console.error(e); }
  if (user) blip(1320);
  await loadImages(true);
}

function searchOther() {
  const t = pickTitle();
  openOsk({
    label: 'Buscar en SteamGridDB',
    value: sg.pick.game?.name ?? t?.name ?? '',
    hint: 'Escribe el nombre del juego tal como se conoce (en inglés suele encontrarse mejor).',
    capitalize: true,
    onDone: async (v) => {
      const term = v.trim();
      if (!term) return;
      const p = sg.pick;
      p.loading = true; p.error = ''; p.mode = 'games'; p.games = []; p.gi = 0;
      try {
        p.games = await call<SgdbGame[]>('sgdb_search', { term });
        if (!p.games.length) p.error = `Sin resultados para «${term}»`;
      } catch (e) { p.error = errText(e); }
      finally { p.loading = false; }
    },
  });
}

export function setKind(k: ArtKind) {
  if (sg.pick.kind === k && sg.pick.mode === 'images') return;
  sg.pick.kind = k;
  sg.pick.mode = 'images';
  blip(700);
  void loadImages(true);
}

export async function applyImage(i = sg.pick.i) {
  const p = sg.pick, t = pickTitle(), img = p.items[i];
  if (!t || !img || p.busy) return;
  p.busy = true;
  toast('Descargando…', 0);
  try {
    const path = await call<string>('sgdb_apply', { titleId: t.id, kind: p.kind, image: $state.snapshot(img) });
    const own = t[p.kind] && !isSgdbPath(t[p.kind]);
    if (!own) patchTitle(p.pi, p.ti, { [p.kind]: path });
    markRefresh();
    toast(own ? `Guardado, pero la carpeta del juego tiene su propio ${KIND_LABEL[p.kind].toLowerCase()} y tiene prioridad` : `${KIND_LABEL[p.kind]} cambiad${p.kind === 'cover' ? 'a' : 'o'}`, own ? 4200 : 1800);
    blip(1320);
  } catch (e) {
    toast(errText(e), 3600);
  } finally { p.busy = false; }
}

/** Elige una imagen del PC como grafismo del tipo actual (funciona sin clave y sin conexión). */
async function pickLocal() {
  const p = sg.pick, t = pickTitle();
  if (!t || p.busy) return;
  const kind = p.kind;
  const file = await backend.pickFile('image');
  if (!file) return;
  p.busy = true;
  try {
    const path = await call<string>('sgdb_local', { titleId: t.id, kind, path: file });
    const own = t[kind] && !isSgdbPath(t[kind]);
    if (!own) patchTitle(p.pi, p.ti, { [kind]: path });
    markRefresh();
    toast(own ? `Guardado, pero la carpeta del juego tiene su propio ${KIND_LABEL[kind].toLowerCase()} y tiene prioridad` : `${KIND_LABEL[kind]} cambiad${kind === 'cover' ? 'a' : 'o'}`, own ? 4200 : 1800);
    blip(1320);
  } catch (e) {
    toast(errText(e), 3600);
  } finally { p.busy = false; }
}

async function clearImage() {
  const p = sg.pick, t = pickTitle();
  if (!t) return;
  if (!isSgdbPath(t[p.kind])) { toast(t[p.kind] ? 'Ese grafismo es de la carpeta del juego: no se quita desde aquí' : 'No hay nada que quitar', 2600); blip(220); return; }
  try {
    await call('sgdb_clear', { titleId: t.id, kind: p.kind });
    patchTitle(p.pi, p.ti, { [p.kind]: null });
    markRefresh();
    toast(`${KIND_LABEL[p.kind]} quitad${p.kind === 'cover' ? 'a' : 'o'}`);
    blip(520);
  } catch (e) { toast(errText(e), 3200); }
}

export function sgdbFocus(i: number) {
  const p = sg.pick;
  p.i = Math.max(0, Math.min(i, p.items.length - 1));
  // Cerca del final se pide la página siguiente.
  if (p.more && !p.loading && p.i >= p.items.length - KIND_COLS[p.kind] * 2) void loadImages(false);
}

export function closeSgdbPicker() {
  popOverlay();
  blip(520);
}

export function sgdbCmd(c: string) {
  const p = sg.pick;
  if (c === 'lb' || c === 'rb' || c === 'lt' || c === 'rt') {
    const n = ART_KINDS.length;
    setKind(ART_KINDS[(ART_KINDS.indexOf(p.kind) + (c === 'lb' || c === 'lt' ? -1 : 1) + n) % n]);
    return;
  }
  if (c === 'view') { void pickLocal(); return; }
  if (c === 'y') { if (hasKey()) searchOther(); else blip(220); return; }
  if (p.mode === 'games') {
    if (c === 'up') { p.gi = Math.max(0, p.gi - 1); blip(880); }
    else if (c === 'down') { p.gi = Math.min(p.games.length - 1, p.gi + 1); blip(880); }
    else if (c === 'a') { const g = p.games[p.gi]; if (g) void chooseGame(g, true); }
    else if (c === 'b') { p.mode = 'images'; p.error = ''; blip(520); }
    else if (c === 'menu') closeOverlay();
    return;
  }
  const cols = KIND_COLS[p.kind], n = p.items.length;
  const step = (d: number) => {
    const ni = p.i + d;
    if (!n || ni < 0 || ni >= n) { blip(220); return; }
    sgdbFocus(ni); blip(880);
  };
  if (c === 'left') step(-1);
  else if (c === 'right') step(1);
  else if (c === 'up') step(-cols);
  else if (c === 'down') { if (p.i + cols >= n && Math.floor(p.i / cols) < Math.floor((n - 1) / cols)) { sgdbFocus(n - 1); blip(880); } else step(cols); }
  else if (c === 'a') void applyImage();
  else if (c === 'x') void clearImage();
  else if (c === 'b') closeSgdbPicker();
  else if (c === 'menu') closeOverlay();
}
