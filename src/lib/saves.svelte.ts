// Partidas guardadas de un juego, en dos pestañas: «Copias» (copiar y restaurar, lo habitual) y «Dónde guarda»
// (ubicaciones detectadas con Ludusavi o por el motor, o añadidas a mano).
import { IS_TAURI, backend } from './backend';
import { blip } from './sound';
import { fmtBytes, popOverlay, pushOverlay, toast } from './controller.svelte';
import { fmtAgo } from './format';

export interface SaveLoc {
  path: string;
  kind: 'path' | 'registry';
  origin: 'ludusavi' | 'engine' | 'manual';
  label: string;
  /** «AppData › RenPy › DDLC-1454445547». */
  place: string;
  enabled: boolean;
  exists: boolean;
  files: number;
  size: number;
  modifiedMs: number;
}
export interface SaveCopy { file: string; createdMs: number; kind: 'auto' | 'manual' | 'antes'; size: number; files: number }
export interface SavesInfo {
  gameId: string;
  name: string;
  matched: string | null;
  engine: string | null;
  locs: SaveLoc[];
  copies: SaveCopy[];
  dir: string;
  manifestGames: number;
  hasExe: boolean;
}
export interface SavesStatus { games: number; fetchedMs: number; defaultDir: string; busy: boolean }

export type SavesTab = 'copies' | 'locs';
export const TABS: Array<[SavesTab, string]> = [['copies', 'Copias'], ['locs', 'Dónde guarda']];

export type SaveRow =
  | { k: 'backup' }
  | { k: 'copy'; i: number }
  | { k: 'download' }
  | { k: 'loc'; i: number }
  | { k: 'adddir' }
  | { k: 'addfile' };

export const sv = $state({
  info: null as SavesInfo | null,
  loading: false,
  busy: '',
  tab: 'copies' as SavesTab,
  row: 0,
  /** Copia esperando la segunda A para restaurarla. */
  confirm: -1,
  status: null as SavesStatus | null,
});

/* ---------- acceso al núcleo (o simulado en el navegador) ---------- */
const MOCK_INFO = (): SavesInfo => ({
  gameId: 'mock', name: 'Juego de ejemplo', matched: 'Juego de ejemplo (carpeta del juego)', engine: "Ren'Py",
  locs: [
    { path: 'C:/Users/yo/AppData/Roaming/RenPy/Ejemplo-1234567', kind: 'path', origin: 'engine', label: "Ren'Py", place: 'AppData › RenPy › Ejemplo-1234567', enabled: true, exists: true, files: 14, size: 812_000, modifiedMs: Date.now() - 36e5 },
    { path: 'D:/Juegos/Ejemplo/game/saves', kind: 'path', origin: 'ludusavi', label: 'Ludusavi · partidas', place: 'Carpeta del juego › game › saves', enabled: true, exists: true, files: 6, size: 240_000, modifiedMs: Date.now() - 864e5 },
    { path: 'HKEY_CURRENT_USER\\Software\\Estudio\\Ejemplo', kind: 'registry', origin: 'ludusavi', label: 'Ludusavi · configuración', place: 'Registro › Estudio › Ejemplo', enabled: false, exists: true, files: 0, size: 0, modifiedMs: 0 },
  ],
  copies: [
    { file: 'a.zip', createdMs: Date.now() - 2 * 36e5, kind: 'auto', size: 380_000, files: 20 },
    { file: 'b.zip', createdMs: Date.now() - 26 * 36e5, kind: 'auto', size: 375_000, files: 20 },
    { file: 'c.zip', createdMs: Date.now() - 3 * 864e5, kind: 'manual', size: 350_000, files: 19 },
  ],
  dir: 'C:/Users/yo/AppData/Roaming/visor/partidas/Juego de ejemplo [abc123]', manifestGames: 22479, hasExe: true,
});
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) {
    if (cmd === 'saves_info') return MOCK_INFO() as T;
    if (cmd === 'saves_status') return { games: 22479, fetchedMs: Date.now() - 864e5, defaultDir: 'C:/…/partidas', busy: false } as T;
    if (cmd === 'saves_backup') return { file: 'd.zip', createdMs: Date.now(), kind: 'manual', size: 1, files: 1 } as T;
    if (cmd === 'saves_restore') return { files: 20, registry: 0, before: null } as T;
    return null as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** Filas de la pestaña abierta. */
export function savesRows(): SaveRow[] {
  const i = sv.info;
  if (!i) return [];
  if (sv.tab === 'copies') return [{ k: 'backup' }, ...i.copies.map((_, n) => ({ k: 'copy' as const, i: n }))];
  const out: SaveRow[] = [];
  if (!i.manifestGames && i.hasExe) out.push({ k: 'download' });
  i.locs.forEach((_, n) => out.push({ k: 'loc', i: n }));
  out.push({ k: 'adddir' }, { k: 'addfile' });
  return out;
}

/* ---------- textos ---------- */
const WD = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const z = (n: number) => String(n).padStart(2, '0');
/** «Hoy, 14:03», «Ayer, 23:31», «vie 17 oct, 01:31». */
export function when(ms: number): string {
  const d = new Date(ms), now = new Date();
  const ymd = (x: Date) => x.toDateString();
  const day = ymd(d) === ymd(now) ? 'Hoy' : ymd(d) === ymd(new Date(now.getTime() - 864e5)) ? 'Ayer'
    : `${WD[d.getDay()]} ${d.getDate()} ${MON[d.getMonth()]}${d.getFullYear() !== now.getFullYear() ? ' ' + d.getFullYear() : ''}`;
  return `${day}, ${z(d.getHours())}:${z(d.getMinutes())}`;
}
export const KIND_LABEL: Record<SaveCopy['kind'], string> = { auto: 'Al cerrar el juego', manual: 'Copia manual', antes: 'Antes de restaurar' };
/** Origen corto de una ubicación: «Ren'Py», «Ludusavi», «A mano». */
export const originLabel = (l: SaveLoc) => (l.origin === 'manual' ? 'A mano' : l.label.split(' · ')[0]);

/** Resumen bajo el título: de dónde salen las partidas, cuánto ocupan y cuándo cambiaron. */
export function savesSummary(): string {
  const i = sv.info;
  if (!i || sv.loading) return 'Buscando dónde guarda sus partidas…';
  const on = i.locs.filter((l) => l.enabled && l.exists);
  if (!on.length) return i.locs.length ? 'Ninguna ubicación activa: actívalas en «Dónde guarda».' : 'No se sabe dónde guarda sus partidas: añádelas en «Dónde guarda».';
  const src = i.engine ?? (i.matched ? 'Ludusavi' : 'Rutas a mano');
  const size = on.reduce((a, l) => a + l.size, 0);
  const mod = Math.max(...on.map((l) => l.modifiedMs));
  return [src, `${on.length} ${on.length === 1 ? 'ubicación' : 'ubicaciones'}`, fmtBytes(size), mod ? `cambió ${fmtAgo(mod).toLowerCase()}` : '']
    .filter(Boolean).join(' · ');
}

/** Línea de ayuda de la fila enfocada. */
export function rowHelp(r: SaveRow | undefined): string {
  const i = sv.info;
  if (!i || !r) return '';
  switch (r.k) {
    case 'backup': return `Guarda ahora un .zip con las partidas. Al cerrar el juego se hace solo si cambiaron (Ajustes › Partidas). Copias en ${i.dir}`;
    case 'copy': {
      const c = i.copies[r.i];
      return sv.confirm === r.i
        ? 'Pulsa A otra vez: se sobrescriben las partidas actuales con las de esta copia. Lo que hay ahora se guarda antes en otra copia.'
        : `${KIND_LABEL[c.kind]} · ${c.files} ${c.files === 1 ? 'archivo' : 'archivos'} · ${fmtBytes(c.size)}. A para restaurarla.`;
    }
    case 'download': return 'Base libre con dónde guarda sus partidas cada juego (más de 20.000). Se descarga una vez (18 MB) y se actualiza desde Ajustes › Partidas.';
    case 'loc': {
      const l = i.locs[r.i];
      const extra = l.kind === 'path' && l.exists ? ` · ${l.files} ${l.files === 1 ? 'archivo' : 'archivos'}${l.modifiedMs ? `, cambió ${fmtAgo(l.modifiedMs).toLowerCase()}` : ''}` : l.exists ? '' : ' · ahora no existe';
      return `${l.path}${extra} · ${l.label}`;
    }
    case 'adddir': return 'Si sus partidas están en otro sitio, elige la carpeta.';
    case 'addfile': return 'Para juegos que guardan en un único archivo.';
  }
}

/* ---------- acciones ---------- */
export async function reloadSaves() {
  if (!sv.info) return;
  try {
    sv.info = await call<SavesInfo>('saves_info', { id: sv.info.gameId });
  } catch (e) { toast(String(e), 3200); }
  sv.row = Math.max(0, Math.min(sv.row, savesRows().length - 1));
}

export async function openSaves(gameId: string, name: string) {
  sv.info = { gameId, name, matched: null, engine: null, locs: [], copies: [], dir: '', manifestGames: 1, hasExe: false };
  sv.tab = 'copies'; sv.row = 0; sv.confirm = -1; sv.busy = ''; sv.loading = true;
  pushOverlay('saves');
  blip(1320);
  try {
    sv.info = await call<SavesInfo>('saves_info', { id: gameId });
    // Sin ubicaciones no hay nada que copiar: se empieza por «Dónde guarda».
    if (!sv.info.locs.length) sv.tab = 'locs';
    // La primera vez, la base de Ludusavi se descarga sola.
    if (!sv.info.manifestGames && sv.info.hasExe) void downloadManifest();
  } catch (e) { toast(String(e), 3200); popOverlay(); }
  sv.loading = false;
}

export async function downloadManifest() {
  if (sv.busy) return;
  sv.busy = 'download';
  toast('Descargando la base de Ludusavi…', 0);
  try {
    sv.status = await call<SavesStatus>('saves_update_manifest');
    toast(`Base de Ludusavi lista: ${sv.status.games.toLocaleString('es')} juegos`, 2600);
  } catch (e) { toast(String(e), 4200); }
  sv.busy = '';
  await reloadSaves();
}

async function pickFile(): Promise<string | null> {
  if (!IS_TAURI) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const r = await open({ multiple: false, directory: false, title: 'Archivo de partida' });
  return typeof r === 'string' ? r : null;
}

async function act(r: SaveRow) {
  const i = sv.info;
  if (!i || sv.loading) return;
  if (r.k !== 'copy') sv.confirm = -1;
  try {
    if (r.k === 'download') await downloadManifest();
    else if (r.k === 'loc') {
      const l = i.locs[r.i];
      await call('saves_set_enabled', { id: i.gameId, path: l.path, kind: l.kind, enabled: !l.enabled });
      l.enabled = !l.enabled; // al momento; la recarga lo confirma
      blip(1320);
      await reloadSaves();
    } else if (r.k === 'backup') {
      if (sv.busy) return;
      sv.busy = 'backup';
      try {
        const c = await call<SaveCopy>('saves_backup', { id: i.gameId });
        toast(`Copia guardada (${c.files} archivos, ${fmtBytes(c.size)})`, 3200);
      } finally { sv.busy = ''; }
      await reloadSaves();
    } else if (r.k === 'adddir' || r.k === 'addfile') {
      const p = r.k === 'adddir' ? await backend.pickFolder('Carpeta de partidas') : await pickFile();
      if (!p) return;
      await call('saves_add', { id: i.gameId, path: p });
      await reloadSaves();
      toast('Ubicación añadida');
    } else if (r.k === 'copy') {
      if (sv.busy) return;
      if (sv.confirm !== r.i) { sv.confirm = r.i; blip(520); return; }
      sv.confirm = -1;
      sv.busy = 'restore';
      toast('Restaurando…', 0);
      try {
        const rep = await call<{ files: number; registry: number }>('saves_restore', { id: i.gameId, file: i.copies[r.i].file });
        toast(`Restaurados ${rep.files} archivos${rep.registry ? ` y ${rep.registry} claves del registro` : ''}. Lo que había quedó en otra copia`, 4200);
      } finally { sv.busy = ''; }
      await reloadSaves();
    }
  } catch (e) { toast(String(e), 4200); }
}

/** X: quita una ubicación añadida a mano. */
async function removeManual(r: SaveRow) {
  const i = sv.info;
  if (!i || r.k !== 'loc' || i.locs[r.i].origin !== 'manual') return;
  try {
    await call('saves_remove', { id: i.gameId, path: i.locs[r.i].path });
    toast('Ubicación quitada');
    await reloadSaves();
  } catch (e) { toast(String(e), 3200); }
}

/** Y: en «Dónde guarda» abre la ubicación enfocada; en «Copias», la carpeta de copias. */
async function reveal(r: SaveRow | undefined) {
  const i = sv.info;
  if (!i) return;
  try {
    if (r?.k === 'loc') {
      const l = i.locs[r.i];
      if (l.kind === 'registry') { toast('Es una clave del registro: ábrela con regedit', 2600); return; }
      // Con comodines se abre la parte fija de la ruta.
      const fixed = l.path.split(/[\\/]/).filter((c, n, a) => !a.slice(0, n + 1).some((x) => /[*?]/.test(x))).join('\\');
      await backend.reveal(fixed);
    } else if (IS_TAURI) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('saves_open_dir', { id: i.gameId });
    } else toast('Abrir carpeta (simulado en el navegador)');
  } catch (e) { toast(String(e), 3200); }
}

export function pickRow(n: number) {
  if (sv.row !== n) sv.confirm = -1;
  sv.row = n;
}

export function setTab(t: SavesTab) {
  if (sv.tab === t) return;
  sv.tab = t; sv.row = 0; sv.confirm = -1;
  blip(880);
}

export function savesCmd(c: string) {
  const rows = savesRows();
  const r = rows[sv.row];
  if (c === 'up') { pickRow(Math.max(0, sv.row - 1)); blip(880); }
  else if (c === 'down') { pickRow(Math.min(rows.length - 1, sv.row + 1)); blip(880); }
  else if (c === 'lb' || c === 'lt' || c === 'left') setTab('copies');
  else if (c === 'rb' || c === 'rt' || c === 'right') setTab('locs');
  else if (c === 'a' && r) void act(r);
  else if (c === 'x' && r) void removeManual(r);
  else if (c === 'y') void reveal(r);
  else if (c === 'b') { sv.confirm = -1; popOverlay(); blip(520); }
}

/* ---------- Ajustes › Partidas ---------- */
export async function refreshSavesStatus() {
  try { sv.status = await call<SavesStatus>('saves_status'); } catch { sv.status = null; }
}
export function ludusaviLabel(): string {
  const s = sv.status;
  if (sv.busy === 'download') return 'Descargando…';
  if (!s || !s.games) return 'Sin descargar';
  return `${s.games.toLocaleString('es')} juegos · ${fmtAgo(s.fetchedMs).toLowerCase()}`;
}
export async function updateManifestFromSettings() {
  if (sv.busy) return;
  sv.busy = 'download';
  toast('Actualizando la base de Ludusavi…', 0);
  try {
    sv.status = await call<SavesStatus>('saves_update_manifest');
    toast(`Base de Ludusavi al día: ${sv.status.games.toLocaleString('es')} juegos`, 2600);
  } catch (e) { toast(String(e), 4200); }
  sv.busy = '';
}

/** Avisos de la copia automática al cerrar un juego. */
export async function listenSaves() {
  if (!IS_TAURI) return;
  const { listen } = await import('@tauri-apps/api/event');
  // Llega justo después del aviso de «Has jugado…»: se deja leer ese primero.
  void listen<{ message: string; error: boolean }>('saves-copied', (e) =>
    setTimeout(() => toast(e.payload.message, e.payload.error ? 5200 : 2600), 3400));
}
