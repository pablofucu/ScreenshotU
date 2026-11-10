// Nintendo Switch: cuenta, estado de la sincronización y ajuste del mínimo de tiempo (sección «Nintendo» de Ajustes).
// El inicio de sesión se hace en una ventana propia del núcleo; aquí solo se pide y se escucha el resultado.
import { IS_TAURI } from './backend';
import { app } from './store.svelte';
import { blip } from './sound';
import { patchConfig, popOverlay, pushOverlay, rescan, toast } from './controller.svelte';
import type { NxCandidate, NxChoice, NxDoubtItem, NxItem, NxLinkItem, NxNewGroup, NxPreview, NxReport } from './types';

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
let inv: Invoke | null = null;
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) return mock(cmd) as T;
  inv ??= (await import('./ipc')).invoke as Invoke;
  return inv<T>(cmd, args);
}

/** Navegador sin núcleo: cuenta simulada. */
let mockConnected = false;
function mock(cmd: string): unknown {
  if (cmd === 'nintendo_status') return { connected: mockConnected };
  if (cmd === 'nintendo_login_start') { mockConnected = true; setTimeout(() => { nt.busy = ''; nt.connected = true; toast('Cuenta conectada'); }, 400); return null; }
  if (cmd === 'nintendo_logout') { mockConnected = false; return null; }
  if (cmd === 'nintendo_import_preview') return { preview: mockPreview, error: null };
  if (cmd === 'nintendo_import_apply') return { created: [], linked: 1, ignored: 0, skipped: [] } satisfies NxReport;
  return null;
}
const mi = (nxId: string, name: string, minutes: number): NxItem => ({ nxId, name, minutes, lastPlayed: Date.now() - minutes * 60000, imageUrl: null, platform: 'HAC' });
const mockPreview: NxPreview = {
  nuevos: [
    { name: 'Hollow Knight', items: [mi('a1', 'Hollow Knight', 1260)] },
    { name: 'Celeste', items: [mi('a2', 'Celeste', 540)] },
  ],
  enlazados: [{ item: mi('b1', 'Hades', 900), target: { gameId: 'g1', folder: 'Hades', name: 'Hades' } }],
  dudosos: [{ item: mi('c1', 'Pokémon Escarlata', 300), options: [{ gameId: 'g2', folder: 'Pokemon Scarlet', name: 'Pokémon Scarlet' }, { gameId: null, folder: 'Pokemon Violet', name: 'Pokemon Violet' }] }],
  excluidos: [mi('d1', 'Tetris 99', 20), mi('d2', 'Pikmin 3', 35)],
  ignorados: [],
  demos: 2,
  minMinutes: 60,
};

export const nt = $state({
  connected: false,
  /** Hay un inicio de sesión o una descarga en curso. */
  busy: '' as '' | 'login' | 'import',
  /** Juegos pendientes de importar tras la última sincronización (null = aún no se sabe). */
  pending: null as number | null,
  /** Aviso de la última sincronización fallida. */
  error: '' as string,
});

/** Mínimos de tiempo ofrecidos (minutos). Con 0 se ofrece todo lo que tenga tiempo. */
export const MIN_STEPS = [0, 15, 30, 60, 120, 180, 300];
export const minLabel = (m: number) => (m <= 0 ? 'Todos' : m < 60 ? `${m} min` : `${m / 60} h`);

let listening = false;
async function listen() {
  if (listening || !IS_TAURI) return;
  listening = true;
  const { listen } = await import('./ipc');
  void listen<{ ok: boolean; message: string }>('nintendo-login-result', (e) => {
    nt.busy = '';
    nt.connected = e.payload.ok;
    toast(e.payload.message, 3200);
    if (e.payload.ok) void importNow(true);
  });
  void listen<{ changed: number; newTitles: number; expired: boolean; error: string | null }>('nintendo-synced', (e) => {
    nt.pending = e.payload.newTitles;
    nt.error = e.payload.expired ? 'La sesión de Nintendo ha caducado: vuelve a conectar la cuenta.' : e.payload.error ?? '';
    if (e.payload.expired) nt.connected = false;
  });
}

/** Al abrir Ajustes: lee si hay cuenta conectada. */
export async function nintendoInit() {
  await listen();
  try { nt.connected = (await call<{ connected: boolean }>('nintendo_status')).connected; } catch { nt.connected = false; }
}

export function nintendoStateLabel(): string {
  if (nt.busy === 'login') return 'Conectando…';
  if (!nt.connected) return 'Sin conectar';
  if (nt.error) return 'Conectada · con avisos';
  return nt.pending ? `Conectada · ${nt.pending} nuevos` : 'Conectada';
}

/** A sobre «Cuenta»: conecta (abre la ventana de inicio de sesión) o desconecta. */
export async function nintendoToggleAccount() {
  if (nt.busy) return;
  blip(1320);
  if (nt.connected) {
    try { await call('nintendo_logout'); nt.connected = false; nt.pending = null; nt.error = ''; toast('Cuenta desconectada'); }
    catch (e) { toast(String(e), 3200); }
    return;
  }
  await listen();
  nt.busy = 'login';
  try { await call('nintendo_login_start'); }
  catch (e) { nt.busy = ''; toast(String(e), 3200); }
}

/** ‹ › cambian el mínimo de tiempo para importar; A pasa al siguiente. */
export function nintendoCycleMin(dir: number) {
  const cur = app.config.nintendoMinMinutes ?? 60;
  const i = MIN_STEPS.indexOf(cur);
  const n = MIN_STEPS.length;
  const next = MIN_STEPS[i < 0 ? 3 : (i + (dir || 1) + n) % n];
  void patchConfig({ nintendoMinMinutes: next });
  blip(880);
}

/** Baja el historial y avisa de cuántos juegos hay por importar. */
export async function importNow(quiet = false) {
  if (nt.busy) return;
  if (!nt.connected) { toast('Conecta primero la cuenta de Nintendo'); return; }
  nt.busy = 'import';
  try {
    const r = await call<{ preview: { nuevos: unknown[]; dudosos: unknown[] }; error: string | null }>('nintendo_import_preview', { minMinutes: null, refresh: true });
    nt.pending = r.preview.nuevos.length + r.preview.dudosos.length;
    nt.error = r.error ?? '';
    if (r.error) toast(r.error, 4200);
    else if (!quiet || nt.pending) toast(nt.pending ? `${nt.pending} juegos de Nintendo por importar` : 'No hay juegos nuevos que importar', 3200);
  } catch (e) { toast(String(e), 4200); }
  finally { nt.busy = ''; }
}

/* ------------------------------------------------------------------ */
/* Pantalla de importación: vista previa, ajustes a mano y aplicar     */
/* ------------------------------------------------------------------ */
export type ImportRow =
  | { kind: 'new'; group: NxNewGroup; on: boolean }
  | { kind: 'link'; it: NxLinkItem; on: boolean }
  /** `pick`: -1 = decidir luego; 0..n-1 = enlazar con esa ficha; n = crear ficha nueva; n+1 = ignorar. */
  | { kind: 'doubt'; it: NxDoubtItem; pick: number }
  | { kind: 'ex'; it: NxItem; on: boolean };

export const ni = $state({ rows: [] as ImportRow[], row: 0, minMinutes: 60, demos: 0, ignored: 0, busy: false });

export const fmtMin = (m: number) => (m < 60 ? `${m} min` : `${Math.round(m / 6) / 10} h`.replace('.', ','));

/** Descarga el historial y abre la pantalla de importación con lo que hay. */
export async function openNintendoImport() {
  if (nt.busy) return;
  if (!nt.connected) { toast('Conecta primero la cuenta de Nintendo'); return; }
  nt.busy = 'import';
  try {
    const r = await call<{ preview: NxPreview; error: string | null }>('nintendo_import_preview', { minMinutes: null, refresh: true });
    nt.error = r.error ?? '';
    if (r.error) toast(r.error, 4200);
    const p = r.preview;
    nt.pending = p.nuevos.length + p.dudosos.length;
    ni.rows = [
      ...p.nuevos.map((group): ImportRow => ({ kind: 'new', group, on: true })),
      ...p.enlazados.map((it): ImportRow => ({ kind: 'link', it, on: true })),
      ...p.dudosos.map((it): ImportRow => ({ kind: 'doubt', it, pick: -1 })),
      ...p.excluidos.map((it): ImportRow => ({ kind: 'ex', it, on: false })),
    ];
    ni.row = 0; ni.minMinutes = p.minMinutes; ni.demos = p.demos; ni.ignored = p.ignorados.length;
    if (!ni.rows.length) { toast('No hay juegos de Nintendo que importar', 3200); return; }
    pushOverlay('ntimport');
    blip(1320);
  } catch (e) { toast(String(e), 4200); }
  finally { nt.busy = ''; }
}

const optLabel = (o: NxCandidate) => o.name || o.folder;

function importChoices(): NxChoice[] {
  const out: NxChoice[] = [];
  const link = (nxId: string, t: NxCandidate): NxChoice => ({ action: 'link', nxId, gameId: t.gameId, folder: t.gameId ? null : t.folder });
  for (const r of ni.rows) {
    if (r.kind === 'new' && r.on) out.push({ action: 'create', nxIds: r.group.items.map((i) => i.nxId), name: r.group.name });
    else if (r.kind === 'link' && r.on) out.push(link(r.it.item.nxId, r.it.target));
    else if (r.kind === 'ex' && r.on) out.push({ action: 'create', nxIds: [r.it.nxId], name: r.it.name });
    else if (r.kind === 'doubt' && r.pick >= 0) {
      const n = r.it.options.length;
      if (r.pick < n) out.push(link(r.it.item.nxId, r.it.options[r.pick]));
      else if (r.pick === n) out.push({ action: 'create', nxIds: [r.it.item.nxId], name: r.it.item.name });
      else out.push({ action: 'ignore', nxId: r.it.item.nxId });
    }
  }
  return out;
}

/** Texto de cada fila de la pantalla; la fila 0 es siempre «Importar». */
export function importLabels(): Array<{ l: string; v: string }> {
  const out = ni.rows.map((r): { l: string; v: string } => {
    if (r.kind === 'new') return { l: `Nuevo · ${r.group.name}`, v: `${fmtMin(r.group.items.reduce((a, i) => a + i.minutes, 0))} · ${r.on ? 'Crear ficha' : 'No'}` };
    if (r.kind === 'link') return { l: `Enlazar · ${r.it.item.name}`, v: r.on ? `→ ${optLabel(r.it.target)}` : 'No' };
    if (r.kind === 'ex') return { l: `Bajo el mínimo · ${r.it.name}`, v: `${fmtMin(r.it.minutes)} · ${r.on ? 'Añadir' : 'No'}` };
    const n = r.it.options.length;
    const v = r.pick < 0 ? 'Decidir luego' : r.pick < n ? `→ ${optLabel(r.it.options[r.pick])}` : r.pick === n ? 'Crear ficha' : 'Ignorar';
    return { l: `Dudoso · ${r.it.item.name}`, v: `‹ ${v} ›` };
  });
  return [{ l: `Importar ${importChoices().length} cambios`, v: '' }, ...out];
}

async function applyImport() {
  const choices = importChoices();
  if (!choices.length) { toast('No hay nada seleccionado'); return; }
  if (ni.busy) return;
  ni.busy = true;
  try {
    const r = await call<NxReport>('nintendo_import_apply', { choices });
    const parts = [r.created.length && `${r.created.length} fichas nuevas`, r.linked && `${r.linked} enlazadas`, r.ignored && `${r.ignored} ignoradas`].filter(Boolean);
    toast(parts.length ? `Importado: ${parts.join(', ')}` : 'No se importó nada', 3600);
    if (r.skipped.length) console.warn('[nintendo] saltados', r.skipped);
    nt.pending = null;
    popOverlay();
    await rescan();
  } catch (e) { toast(String(e), 4200); }
  finally { ni.busy = false; }
}

export function nintendoImportCmd(c: string) {
  const n = ni.rows.length + 1;
  if (c === 'up') { ni.row = Math.max(0, ni.row - 1); blip(880); return; }
  if (c === 'down') { ni.row = Math.min(n - 1, ni.row + 1); blip(880); return; }
  if (c === 'b' || c === 'menu') { popOverlay(); blip(520); return; }
  if (ni.row === 0) { if (c === 'a') void applyImport(); return; }
  const r = ni.rows[ni.row - 1];
  if (!r) return;
  if (r.kind === 'doubt') {
    if (c === 'a' || c === 'right' || c === 'left') {
      const span = r.it.options.length + 3; // de -1 a n+1
      const d = c === 'left' ? -1 : 1;
      r.pick = ((r.pick + 1 + d + span) % span) - 1;
      blip(880);
    }
  } else if (c === 'a' || c === 'left' || c === 'right') { r.on = !r.on; blip(1320); }
}
