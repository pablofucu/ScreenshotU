// Partidas guardadas de un juego, a pantalla completa: lista de copias por días a la izquierda; a la derecha la copia
// elegida (miniatura, restaurar, fijar, menú) y la comparación de sus archivos con los de ahora. «Dónde guarda»
// (ubicaciones detectadas con Ludusavi o por el motor, o añadidas a mano) es un cajón que se abre desde la cabecera.
import { IS_TAURI, backend } from './backend';
import { blip } from './sound';
import { fmtBytes, popOverlay, pushOverlay, toast } from './controller.svelte';
import { fmtAgo } from './format';
import { openOsk } from './osk.svelte';

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
export interface SaveCopy { file: string; createdMs: number; kind: 'auto' | 'manual' | 'antes'; size: number; files: number; note: string; pinned: boolean }
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
  /** Copias automáticas que se conservan además de las fijadas. */
  keep: number;
}
export interface SavesStatus { games: number; fetchedMs: number; defaultDir: string; busy: boolean }

/** Estado de un archivo de la copia frente a lo que hay ahora. */
export type FileState = 'same' | 'changed' | 'onlyCopy' | 'newNow';
export interface CopyFile { entry: string; loc: number; name: string; size: number; copyMs: number; nowSize: number; nowMs: number; state: FileState; registry: boolean }
export interface CopyFilesInfo { locs: Array<{ path: string; kind: 'path' | 'registry'; place: string }>; files: CopyFile[] }
export type FileFilter = 'all' | FileState;

/** Filas del cajón «Dónde guarda». */
export type SaveRow =
  | { k: 'download' }
  | { k: 'loc'; i: number }
  | { k: 'adddir' }
  | { k: 'addfile' };

/** Dónde está el foco: lista de copias, tabla de archivos, menú ⋯ o cajón de ubicaciones. */
export type SavesFocus = 'list' | 'table' | 'menu' | 'locs';
export type MenuId = 'note' | 'pin' | 'export' | 'open' | 'del';

export const sv = $state({
  info: null as SavesInfo | null,
  loading: false,
  busy: '',
  status: null as SavesStatus | null,
  focus: 'list' as SavesFocus,
  /** Copia elegida (índice en `info.copies`). */
  sel: 0,
  cmp: null as CopyFilesInfo | null,
  cmpLoading: false,
  filter: 'all' as FileFilter,
  fileRow: 0,
  menuRow: 0,
  /** Posición del menú con el clic derecho (null = junto al botón ⋯). */
  menuXY: null as [number, number] | null,
  /** Fila del cajón «Dónde guarda». */
  row: 0,
  /** Acción esperando la segunda pulsación: 'all' (restaurar la copia), 'del' o el nombre de un archivo. */
  confirm: '',
});

/* ---------- acceso al núcleo (o simulado en el navegador) ---------- */
const MOCK_FILES: Array<[string, number, number]> = [
  ['persistent', 0, 8200], ['1-1-LT1.save', 0, 61000], ['1-2-LT1.save', 0, 60000], ['auto-1-LT1.save', 0, 59000],
  ['quick-1-LT1.save', 0, 61000], ['slot1.sav', 1, 24000], ['slot2.sav', 1, 24000],
];
let mockInfo: SavesInfo | null = null;
const MOCK_INFO = (): SavesInfo => (mockInfo ??= {
  gameId: 'mock', name: 'Juego de ejemplo', matched: 'Juego de ejemplo (carpeta del juego)', engine: "Ren'Py",
  locs: [
    { path: 'C:/Users/yo/AppData/Roaming/RenPy/Ejemplo-1234567', kind: 'path', origin: 'engine', label: "Ren'Py", place: 'AppData › RenPy › Ejemplo-1234567', enabled: true, exists: true, files: 14, size: 812_000, modifiedMs: Date.now() - 36e5 },
    { path: 'D:/Juegos/Ejemplo/game/saves', kind: 'path', origin: 'ludusavi', label: 'Ludusavi · partidas', place: 'Carpeta del juego › game › saves', enabled: true, exists: true, files: 6, size: 240_000, modifiedMs: Date.now() - 864e5 },
    { path: 'HKEY_CURRENT_USER\\Software\\Estudio\\Ejemplo', kind: 'registry', origin: 'ludusavi', label: 'Ludusavi · configuración', place: 'Registro › Estudio › Ejemplo', enabled: false, exists: true, files: 0, size: 0, modifiedMs: 0 },
  ],
  copies: [
    { file: 'a.zip', createdMs: Date.now() - 2 * 36e5, kind: 'auto', size: 380_000, files: 7, note: '', pinned: false },
    { file: 'a2.zip', createdMs: Date.now() - 5 * 36e5, kind: 'manual', size: 372_000, files: 7, note: 'Antes de elegir el poema', pinned: true },
    { file: 'b.zip', createdMs: Date.now() - 26 * 36e5, kind: 'auto', size: 375_000, files: 7, note: '', pinned: false },
    { file: 'b2.zip', createdMs: Date.now() - 27 * 36e5, kind: 'antes', size: 370_000, files: 7, note: '', pinned: false },
    { file: 'c.zip', createdMs: Date.now() - 3 * 864e5, kind: 'manual', size: 350_000, files: 6, note: 'Ruta de Yuri completada', pinned: true },
    { file: 'd.zip', createdMs: Date.now() - 6 * 864e5, kind: 'auto', size: 310_000, files: 6, note: '', pinned: false },
  ],
  dir: 'C:/Users/yo/AppData/Roaming/visor/partidas/Juego de ejemplo [abc123]', manifestGames: 22479, hasExe: true, keep: 10,
});
function mockFiles(file: string): CopyFilesInfo {
  const n = MOCK_INFO().copies.findIndex((c) => c.file === file);
  const files: CopyFile[] = MOCK_FILES.map(([name, loc, size], j) => {
    const state: FileState = (j + n) % 4 === 0 ? 'changed' : (j + n) % 7 === 3 ? 'onlyCopy' : 'same';
    return { entry: `e${loc}/${name}`, loc, name, size, copyMs: Date.now() - (n + 1) * 864e5, nowSize: state === 'onlyCopy' ? 0 : size + (state === 'changed' ? 300 : 0), nowMs: state === 'onlyCopy' ? 0 : Date.now() - 36e5, state, registry: false };
  });
  if (n % 2 === 1) files.push({ entry: 'e0/auto-3-LT1.save', loc: 0, name: 'auto-3-LT1.save', size: 0, copyMs: 0, nowSize: 60000, nowMs: Date.now() - 36e5, state: 'newNow', registry: false });
  return { locs: MOCK_INFO().locs.slice(0, 2).map((l) => ({ path: l.path, kind: l.kind, place: l.place })), files };
}
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) {
    const m = MOCK_INFO();
    const c = m.copies.find((x) => x.file === args?.file);
    if (cmd === 'saves_info') return structuredClone(m) as T;
    if (cmd === 'saves_status') return { games: 22479, fetchedMs: Date.now() - 864e5, defaultDir: 'C:/…/partidas', busy: false } as T;
    if (cmd === 'saves_backup') {
      const n = { file: `m${Date.now()}.zip`, createdMs: Date.now(), kind: 'manual' as const, size: 381_000, files: 7, note: '', pinned: false };
      m.copies.unshift(n);
      return n as T;
    }
    if (cmd === 'saves_restore') return { files: 7, registry: 0, before: null } as T;
    if (cmd === 'saves_restore_file') return { files: 1, registry: 0, before: null } as T;
    if (cmd === 'saves_copy_files') return mockFiles(String(args?.file)) as T;
    if (cmd === 'saves_set_note' && c) c.note = String(args?.note ?? '').trim();
    if (cmd === 'saves_set_pinned' && c) c.pinned = !!args?.pinned;
    if (cmd === 'saves_delete_copy') m.copies = m.copies.filter((x) => x.file !== args?.file);
    return null as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** Filas del cajón «Dónde guarda». */
export function savesRows(): SaveRow[] {
  const i = sv.info;
  if (!i) return [];
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
/** «14:03». */
export const hhmm = (ms: number) => { const d = new Date(ms); return `${z(d.getHours())}:${z(d.getMinutes())}`; };
/** «vie 17 oct» (con el año si no es el actual). */
export function dayShort(ms: number): string {
  const d = new Date(ms);
  return `${WD[d.getDay()]} ${d.getDate()} ${MON[d.getMonth()]}${d.getFullYear() !== new Date().getFullYear() ? ' ' + d.getFullYear() : ''}`;
}
/** Cabecera de un día: «Hoy · mar 20 oct», «Ayer · lun 19 oct», «vie 17 oct». */
export function dayLabel(ms: number): string {
  const d = new Date(ms).toDateString(), now = new Date();
  const rel = d === now.toDateString() ? 'Hoy · ' : d === new Date(now.getTime() - 864e5).toDateString() ? 'Ayer · ' : '';
  return rel + dayShort(ms);
}
export const KIND_LABEL: Record<SaveCopy['kind'], string> = { auto: 'Al cerrar el juego', manual: 'Copia manual', antes: 'Antes de restaurar' };
export const KIND_SHORT: Record<SaveCopy['kind'], string> = { auto: 'Al cerrar', manual: 'Manual', antes: 'Antes de restaurar' };
export const STATE_LABEL: Record<FileState, string> = { same: 'Igual', changed: 'Cambia', onlyCopy: 'Solo en la copia', newNow: 'Nuevo ahora' };
/** Origen corto de una ubicación: «Ren'Py», «Ludusavi», «A mano». */
export const originLabel = (l: SaveLoc) => (l.origin === 'manual' ? 'A mano' : l.label.split(' · ')[0]);

/** Resumen de las ubicaciones: de dónde salen las partidas, cuánto ocupan y cuándo cambiaron. */
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

/** Línea de ayuda de la fila enfocada del cajón. */
export function rowHelp(r: SaveRow | undefined): string {
  const i = sv.info;
  if (!i || !r) return '';
  switch (r.k) {
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

/** Entradas del menú ⋯ de la copia elegida. */
export function menuItems(): Array<{ id: MenuId; label: string }> {
  const c = sv.info?.copies[sv.sel];
  if (!c) return [];
  return [
    { id: 'note', label: c.note ? 'Cambiar la nota…' : 'Escribir nota…' },
    { id: 'pin', label: c.pinned ? 'Quitar de fijadas' : 'Fijar' },
    { id: 'export', label: 'Exportar .zip…' },
    { id: 'open', label: 'Abrir en el explorador' },
    { id: 'del', label: sv.confirm === 'del' ? '¿Seguro? Otra vez' : 'Borrar copia' },
  ];
}

/** Archivos de la tabla con el filtro aplicado. */
export function visibleFiles(): CopyFile[] {
  const f = sv.cmp?.files ?? [];
  return sv.filter === 'all' ? f : f.filter((x) => x.state === sv.filter);
}

/* ---------- acciones ---------- */
let cmpTimer: ReturnType<typeof setTimeout> | undefined;
let cmpSeq = 0;
/** Carga la comparación de la copia elegida (con un respiro por si se recorre la lista deprisa). */
function loadCmp() {
  clearTimeout(cmpTimer);
  const c = sv.info?.copies[sv.sel];
  const seq = ++cmpSeq;
  sv.cmp = null;
  if (!c) { sv.cmpLoading = false; return; }
  sv.cmpLoading = true;
  cmpTimer = setTimeout(async () => {
    try {
      const r = await call<CopyFilesInfo>('saves_copy_files', { id: sv.info!.gameId, file: c.file });
      if (seq === cmpSeq) sv.cmp = r;
    } catch (e) {
      if (seq === cmpSeq) toast(String(e), 3200);
    }
    if (seq === cmpSeq) sv.cmpLoading = false;
  }, 140);
}

let confirmTimer: ReturnType<typeof setTimeout> | undefined;
/** Primera pulsación de una acción de dos: se arma y caduca a los 3,5 s. */
function arm(key: string) {
  sv.confirm = key;
  blip(520);
  clearTimeout(confirmTimer);
  confirmTimer = setTimeout(() => { if (sv.confirm === key) sv.confirm = ''; }, 3500);
}
function disarm() { sv.confirm = ''; clearTimeout(confirmTimer); }

export async function reloadSaves() {
  if (!sv.info) return;
  const keep = sv.info.copies[sv.sel]?.file;
  try {
    sv.info = await call<SavesInfo>('saves_info', { id: sv.info.gameId });
  } catch (e) { toast(String(e), 3200); }
  const at = sv.info.copies.findIndex((c) => c.file === keep);
  const changed = at < 0 || at !== sv.sel;
  sv.sel = Math.max(0, Math.min(at >= 0 ? at : sv.sel, sv.info.copies.length - 1));
  sv.row = Math.max(0, Math.min(sv.row, savesRows().length - 1));
  if (changed) loadCmp();
}

/** Recarga solo la comparación (tras restaurar, cambian los archivos de ahora). */
async function refreshAll() {
  await reloadSaves();
  loadCmp();
}

export async function openSaves(gameId: string, name: string) {
  sv.info = { gameId, name, matched: null, engine: null, locs: [], copies: [], dir: '', manifestGames: 1, hasExe: false, keep: 10 };
  sv.focus = 'list'; sv.sel = 0; sv.row = 0; sv.fileRow = 0; sv.filter = 'all'; sv.menuXY = null; sv.cmp = null;
  sv.busy = ''; sv.loading = true; disarm();
  pushOverlay('saves');
  blip(1320);
  try {
    sv.info = await call<SavesInfo>('saves_info', { id: gameId });
    // Sin ubicaciones no hay nada que copiar: se empieza por «Dónde guarda».
    if (!sv.info.locs.length) sv.focus = 'locs';
    // La primera vez, la base de Ludusavi se descarga sola.
    if (!sv.info.manifestGames && sv.info.hasExe) void downloadManifest();
    loadCmp();
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

/** Fila del cajón «Dónde guarda». */
async function act(r: SaveRow) {
  const i = sv.info;
  if (!i || sv.loading) return;
  try {
    if (r.k === 'download') await downloadManifest();
    else if (r.k === 'loc') {
      const l = i.locs[r.i];
      await call('saves_set_enabled', { id: i.gameId, path: l.path, kind: l.kind, enabled: !l.enabled });
      l.enabled = !l.enabled; // al momento; la recarga lo confirma
      blip(1320);
      await reloadSaves();
    } else {
      const p = r.k === 'adddir' ? await backend.pickFolder('Carpeta de partidas') : await pickFile();
      if (!p) return;
      await call('saves_add', { id: i.gameId, path: p });
      await reloadSaves();
      toast('Ubicación añadida');
    }
  } catch (e) { toast(String(e), 4200); }
}

/** Copia manual ahora. */
export async function backupNow() {
  const i = sv.info;
  if (!i || sv.loading || sv.busy) return;
  sv.busy = 'backup';
  try {
    const c = await call<SaveCopy>('saves_backup', { id: i.gameId });
    toast(`Copia guardada (${c.files} archivos, ${fmtBytes(c.size)})`, 3200);
    sv.sel = 0; sv.fileRow = 0; sv.filter = 'all';
    await refreshAll();
  } catch (e) { toast(String(e), 4200); }
  sv.busy = '';
}

/** Restaurar la copia elegida: dos pulsaciones. */
export async function restoreCopy() {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c || sv.busy || sv.loading) return;
  if (sv.confirm !== 'all') { arm('all'); return; }
  disarm();
  sv.busy = 'restore';
  toast('Restaurando…', 0);
  try {
    const rep = await call<{ files: number; registry: number }>('saves_restore', { id: i.gameId, file: c.file });
    toast(`Restaurados ${rep.files} archivos${rep.registry ? ` y ${rep.registry} claves del registro` : ''}. Lo que había quedó en otra copia`, 4200);
  } catch (e) { toast(String(e), 4200); }
  sv.busy = '';
  await refreshAll();
}

/** Restaurar un solo archivo de la copia: dos pulsaciones. */
export async function restoreFile(f: CopyFile) {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c || sv.busy || sv.loading) return;
  if (f.state === 'newNow') { toast('Ese archivo no está en la copia'); return; }
  if (sv.confirm !== f.entry) { arm(f.entry); return; }
  disarm();
  sv.busy = 'restore';
  toast('Restaurando…', 0);
  try {
    await call('saves_restore_file', { id: i.gameId, file: c.file, entry: f.entry });
    toast(`Restaurado ${f.name}. Lo que había quedó en otra copia`, 4200);
  } catch (e) { toast(String(e), 4200); }
  sv.busy = '';
  await refreshAll();
}

export async function togglePin() {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c) return;
  try {
    await call('saves_set_pinned', { id: i.gameId, file: c.file, pinned: !c.pinned });
    toast(c.pinned ? 'Copia sin fijar' : 'Copia fijada: no se borrará al rotar', 2400);
    blip(1320);
    await reloadSaves();
  } catch (e) { toast(String(e), 3200); }
}

function editNote() {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c) return;
  const file = c.file;
  openOsk({
    label: 'Nota de la copia', value: c.note, maxLen: 80, capitalize: true, hint: 'Para reconocerla luego («antes del final», «ruta de Yuri»)',
    onDone: async (v) => {
      try { await call('saves_set_note', { id: i.gameId, file, note: v }); await reloadSaves(); } catch (e) { toast(String(e), 3200); }
    },
  });
}

async function exportCopy() {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c) return;
  if (!IS_TAURI) { toast('Exportar .zip (simulado en el navegador)'); return; }
  try {
    const base = c.file.split(/[\\/]/).pop() ?? 'copia.zip';
    const dest = await backend.pickSaveZip(base);
    if (!dest) return;
    await call('saves_export_copy', { id: i.gameId, file: c.file, dest });
    toast('Copia exportada');
  } catch (e) { toast(String(e), 3200); }
}

async function openCopy() {
  const c = sv.info?.copies[sv.sel];
  if (!c) return;
  try {
    if (IS_TAURI) await backend.reveal(c.file);
    else toast('Abrir en el explorador (simulado en el navegador)');
  } catch (e) { toast(String(e), 3200); }
}

async function deleteCopy() {
  const i = sv.info, c = i?.copies[sv.sel];
  if (!i || !c) return;
  if (sv.confirm !== 'del') { arm('del'); return; }
  disarm();
  closeMenu();
  try {
    await call('saves_delete_copy', { id: i.gameId, file: c.file });
    toast('Copia borrada');
    sv.sel = Math.max(0, sv.sel - (sv.sel >= i.copies.length - 1 ? 1 : 0));
    await refreshAll();
  } catch (e) { toast(String(e), 3200); }
}

/** Ejecuta una entrada del menú ⋯. */
export function runMenu(id: MenuId) {
  if (id === 'del') { void deleteCopy(); return; }
  closeMenu();
  if (id === 'note') editNote();
  else if (id === 'pin') void togglePin();
  else if (id === 'export') void exportCopy();
  else if (id === 'open') void openCopy();
}

export function openMenu(xy: [number, number] | null = null) {
  if (!sv.info?.copies[sv.sel]) return;
  disarm();
  sv.menuXY = xy; sv.menuRow = 0; sv.focus = 'menu';
  blip(880);
}
export function closeMenu() {
  if (sv.focus === 'menu') sv.focus = 'list';
  if (sv.confirm === 'del') disarm();
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

/** Y en el cajón: abre la ubicación enfocada (o la carpeta de copias). */
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

/** Elige una copia de la lista. */
export function selectCopy(n: number) {
  const max = (sv.info?.copies.length ?? 1) - 1;
  n = Math.max(0, Math.min(max, n));
  if (n === sv.sel) return;
  sv.sel = n; sv.fileRow = 0; sv.filter = 'all';
  disarm();
  loadCmp();
}

export function setFilter(f: FileFilter) {
  sv.filter = f; sv.fileRow = 0; disarm();
}

export function pickRow(n: number) { sv.row = n; }

export function openLocs() {
  if (sv.focus === 'locs') return;
  disarm();
  sv.focus = 'locs'; sv.row = 0;
  blip(880);
}
export function closeLocs() { if (sv.focus === 'locs') sv.focus = 'list'; }

export function focusTable() {
  if (!visibleFiles().length) { blip(220); return; }
  sv.focus = 'table'; sv.fileRow = Math.min(sv.fileRow, visibleFiles().length - 1);
  disarm();
}

/** Clic en una fila del cajón. */
export function locsClick(n: number) { sv.row = n; const r = savesRows()[n]; if (r) void act(r); }

export function savesCmd(c: string) {
  const i = sv.info;
  const copy = i?.copies[sv.sel];
  if (sv.focus === 'locs') {
    const rows = savesRows();
    const r = rows[sv.row];
    if (c === 'up') { sv.row = Math.max(0, sv.row - 1); blip(880); }
    else if (c === 'down') { sv.row = Math.min(rows.length - 1, sv.row + 1); blip(880); }
    else if (c === 'a' && r) void act(r);
    else if (c === 'x' && r) void removeManual(r);
    else if (c === 'y') void reveal(r);
    else if (c === 'b' || c === 'lb' || c === 'left') { closeLocs(); blip(520); }
    return;
  }
  if (sv.focus === 'menu') {
    const items = menuItems();
    if (c === 'up') { sv.menuRow = Math.max(0, sv.menuRow - 1); disarm(); blip(880); }
    else if (c === 'down') { sv.menuRow = Math.min(items.length - 1, sv.menuRow + 1); disarm(); blip(880); }
    else if (c === 'a' && items[sv.menuRow]) runMenu(items[sv.menuRow].id);
    else if (c === 'b' || c === 'y' || c === 'left') { closeMenu(); blip(520); }
    return;
  }
  if (sv.focus === 'table') {
    const rows = visibleFiles();
    const f = rows[sv.fileRow];
    if (c === 'up') { sv.fileRow = Math.max(0, sv.fileRow - 1); disarm(); blip(880); }
    else if (c === 'down') { sv.fileRow = Math.min(rows.length - 1, sv.fileRow + 1); disarm(); blip(880); }
    else if (c === 'left' || c === 'b') { sv.focus = 'list'; disarm(); blip(520); }
    else if (c === 'lb') setFilter((['all', 'changed', 'onlyCopy', 'newNow', 'same'] as FileFilter[])[(['all', 'changed', 'onlyCopy', 'newNow', 'same'].indexOf(sv.filter) + 4) % 5]);
    else if (c === 'rb') setFilter((['all', 'changed', 'onlyCopy', 'newNow', 'same'] as FileFilter[])[(['all', 'changed', 'onlyCopy', 'newNow', 'same'].indexOf(sv.filter) + 1) % 5]);
    else if (c === 'a' && f) void restoreFile(f);
    else if (c === 'x') void togglePin();
    else if (c === 'y') openMenu();
    return;
  }
  // Lista de copias
  if (c === 'up') { selectCopy(sv.sel - 1); blip(880); }
  else if (c === 'down') { selectCopy(sv.sel + 1); blip(880); }
  else if (c === 'right') focusTable();
  else if (c === 'a') { if (copy) void restoreCopy(); else void backupNow(); }
  else if (c === 'x') void togglePin();
  else if (c === 'y') openMenu();
  else if (c === 'lb' || c === 'lt') openLocs();
  else if (c === 'rb' || c === 'rt') void backupNow();
  else if (c === 'b') { disarm(); popOverlay(); blip(520); }
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
