// Nintendo Switch: cuenta, estado de la sincronización y ajuste del mínimo de tiempo (sección «Nintendo» de Ajustes).
// El inicio de sesión se hace en una ventana propia del núcleo; aquí solo se pide y se escucha el resultado.
import { IS_TAURI } from './backend';
import { app } from './store.svelte';
import { blip } from './sound';
import { patchConfig, toast } from './controller.svelte';

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
let inv: Invoke | null = null;
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) return mock(cmd) as T;
  inv ??= (await import('@tauri-apps/api/core')).invoke as Invoke;
  return inv<T>(cmd, args);
}

/** Navegador sin núcleo: cuenta simulada. */
let mockConnected = false;
function mock(cmd: string): unknown {
  if (cmd === 'nintendo_status') return { connected: mockConnected };
  if (cmd === 'nintendo_login_start') { mockConnected = true; setTimeout(() => { nt.busy = ''; nt.connected = true; toast('Cuenta conectada'); }, 400); return null; }
  if (cmd === 'nintendo_logout') { mockConnected = false; return null; }
  if (cmd === 'nintendo_import_preview') return { preview: { nuevos: [{}, {}], dudosos: [] }, error: null };
  return null;
}

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
  const { listen } = await import('@tauri-apps/api/event');
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
