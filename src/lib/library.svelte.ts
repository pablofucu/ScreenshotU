// Biblioteca: una sola carpeta (elegida por el usuario) con capturas, partidas, grafismos, glosario, base y ajustes.
// El asistente sale solo al arrancar si aún no hay biblioteca (o no se encuentra) y desde Ajustes › Biblioteca para
// moverla. Copia sin borrar el origen y reinicia el visor al terminar.
import { IS_TAURI } from './backend';
import { blip } from './sound';
import { fmtBytes, popOverlay, pushOverlay, toast } from './controller.svelte';
import { app } from './store.svelte';
import { backend } from './backend';

export type LibMode = 'library' | 'legacy' | 'missing';
export interface LibraryInfo { mode: LibMode; dir: string; suggested: string; busy: boolean }
export interface PlanItem { key: string; label: string; from: string; files: number; bytes: number }
export interface LibraryPlan {
  dest: string;
  kind: 'new' | 'existing' | 'resume';
  items: PlanItem[];
  files: number;
  bytes: number;
  free: number | null;
  fits: boolean;
}
export interface CopyProgress { label: string; doneBytes: number; totalBytes: number; doneFiles: number; totalFiles: number }

export type LibStep = 'choose' | 'copying' | 'done';
export type LibRow = 'dest' | 'go' | 'retry' | 'cancel';

export const lb = $state({
  info: null as LibraryInfo | null,
  step: 'choose' as LibStep,
  dest: '',
  plan: null as LibraryPlan | null,
  /** Por qué no vale la carpeta elegida. */
  err: '',
  checking: false,
  row: 0,
  progress: null as CopyProgress | null,
});

/** Sin biblioteca (o sin encontrarla) el asistente no se puede cerrar. */
export const libForced = () => !!lb.info && lb.info.mode !== 'library';

/* ---------- acceso al núcleo (o simulado en el navegador) ---------- */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) {
    if (cmd === 'library_info') return { mode: 'library', dir: 'D:/Visor de capturas', suggested: 'C:/Users/yo/Documents/Visor de capturas', busy: false } as T;
    if (cmd === 'library_plan') return {
      dest: String(args?.dest), kind: 'new', files: 1204, bytes: 3.2e9, free: 120e9, fits: true,
      items: [
        { key: 'db', label: 'Base de datos y ajustes', from: 'C:/…/visor', files: 2, bytes: 4e6 },
        { key: 'captures', label: 'Capturas', from: 'D:/Capturas', files: 1180, bytes: 3.1e9 },
        { key: 'saves', label: 'Copias de partidas', from: 'C:/…/partidas', files: 22, bytes: 90e6 },
      ],
    } as T;
    if (cmd === 'library_copy' || cmd === 'library_use') return null as T;
    throw 'Solo funciona en la aplicación de escritorio (simulado)';
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export function libRows(): LibRow[] {
  if (lb.step !== 'choose') return [];
  const r: LibRow[] = ['dest', 'go'];
  if (lb.info?.mode === 'missing') r.push('retry');
  if (!libForced()) r.push('cancel');
  return r;
}

export function goLabel(): string {
  if (lb.checking) return 'Comprobando…';
  if (!lb.plan) return lb.err ? 'Elige otra carpeta' : 'Elige una carpeta';
  if (lb.plan.kind === 'existing') return 'Usar esta biblioteca';
  if (!lb.plan.fits) return 'No cabe en ese disco';
  return lb.plan.kind === 'resume' ? 'Retomar la copia' : `Copiar ${fmtBytes(lb.plan.bytes)} y empezar`;
}

/** Comprueba la carpeta. Si es una carpeta con cosas (p. ej. un disco entero), propone una subcarpeta nueva dentro. */
async function check(dest: string, nest = true) {
  lb.dest = dest;
  lb.plan = null;
  lb.err = '';
  if (!dest) return;
  lb.checking = true;
  try {
    lb.plan = await call<LibraryPlan>('library_plan', { dest });
  } catch (e) {
    const msg = String(e);
    if (nest && /vacía/.test(msg)) { lb.checking = false; await check(dest.replace(/[\\/]+$/, '') + '\\Visor de capturas', false); return; }
    lb.err = msg;
  } finally {
    lb.checking = false;
  }
}

export async function loadLibraryInfo() {
  try { lb.info = await call<LibraryInfo>('library_info'); } catch { lb.info = null; }
  return lb.info;
}

/** Abre el asistente. Al arrancar sin biblioteca propone `Documentos\Visor de capturas`; al moverla, nada. */
export async function openLibrary() {
  await loadLibraryInfo();
  lb.step = 'choose';
  lb.progress = null;
  lb.row = 0;
  lb.plan = null;
  lb.err = '';
  lb.dest = '';
  if (app.overlay !== 'library') pushOverlay('library');
  if (lb.info?.mode === 'legacy') void check(lb.info.suggested);
  blip(1320);
}

async function pickDest() {
  const r = await backend.pickFolder(lb.info?.mode === 'missing' ? 'Carpeta de tu biblioteca (la que tiene visor.db)' : 'Carpeta para la biblioteca');
  if (r) { await check(r); lb.row = 1; blip(880); }
}

async function go() {
  const p = lb.plan;
  if (!p || lb.checking) { void pickDest(); return; }
  if (p.kind === 'existing') {
    try { await call('library_use', { dest: p.dest }); } catch (e) { toast(String(e), 4200); return; }
    lb.step = 'done';
    restartSoon();
    return;
  }
  if (!p.fits) { toast(`No hay sitio: hacen falta ${fmtBytes(p.bytes)}${p.free != null ? ` y quedan ${fmtBytes(p.free)}` : ''}`, 4200); return; }
  lb.step = 'copying';
  lb.progress = { label: 'Preparando…', doneBytes: 0, totalBytes: p.bytes, doneFiles: 0, totalFiles: p.files };
  let un: (() => void) | null = null;
  if (IS_TAURI) {
    const { listen } = await import('@tauri-apps/api/event');
    un = await listen<CopyProgress>('library-progress', (e) => { lb.progress = e.payload; });
  }
  try {
    await call('library_copy', { dest: p.dest });
    lb.step = 'done';
    blip(1320);
    restartSoon();
  } catch (e) {
    lb.step = 'choose';
    lb.row = 1;
    toast(`No se pudo copiar: ${e}. Lo que ya está copiado se aprovecha si lo intentas otra vez.`, 8000);
    void check(p.dest, false);
  } finally {
    un?.();
  }
}

function restartSoon() {
  setTimeout(() => { void call('library_restart').catch((e) => toast(String(e), 4200)); }, 1600);
}

export function libraryAct(r: LibRow | undefined) {
  if (r === 'dest') void pickDest();
  else if (r === 'go') void go();
  else if (r === 'retry') void call('library_restart').catch((e) => toast(String(e), 4200));
  else if (r === 'cancel') closeLibrary();
}

function closeLibrary() {
  if (libForced() || lb.step === 'copying') { blip(260); return; }
  popOverlay();
  blip(520);
}

export function libraryCmd(c: string) {
  if (lb.step !== 'choose') return;
  const rows = libRows();
  if (c === 'up') { lb.row = Math.max(0, lb.row - 1); blip(880); }
  else if (c === 'down') { lb.row = Math.min(rows.length - 1, lb.row + 1); blip(880); }
  else if (c === 'a') libraryAct(rows[lb.row]);
  else if (c === 'b' || c === 'menu') closeLibrary();
}
