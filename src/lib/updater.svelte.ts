// Actualizaciones automáticas (plugin updater de Tauri): busca al arrancar y, a petición, descarga e instala.
import { blip } from './sound';
import { toast } from './controller.svelte';

type Found = { version: string; body?: string; downloadAndInstall: (cb?: (e: { event: string; data?: { contentLength?: number; chunkLength?: number } }) => void) => Promise<void> };

/** idle: sin comprobar · checking · none: al día · available: hay versión nueva · installing · error. */
export const up = $state({ phase: 'idle' as 'idle' | 'checking' | 'none' | 'available' | 'installing' | 'error', version: '', percent: 0, error: '' });
let found: Found | null = null;

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');

/** `silent`: en la comprobación del arranque no se avisa de que está al día ni de los fallos (sin red, etc.). */
export async function checkUpdate(silent = false) {
  if (up.phase === 'checking' || up.phase === 'installing') return;
  up.phase = 'checking';
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    const u = (await check()) as Found | null;
    if (u) {
      found = u;
      up.version = u.version;
      up.phase = 'available';
      toast(`Hay una versión nueva (${u.version}). Ajustes › Sistema › Actualizaciones.`, 5200);
    } else {
      found = null;
      up.phase = 'none';
      if (!silent) toast('Ya tienes la última versión', 2600);
    }
  } catch (e) {
    found = null;
    up.error = errText(e);
    up.phase = silent ? 'idle' : 'error';
    if (!silent) toast(`No se pudo buscar actualizaciones: ${up.error}`, 4200);
  }
}

export async function installUpdate() {
  if (!found || up.phase === 'installing') return;
  up.phase = 'installing';
  up.percent = 0;
  let total = 0, got = 0;
  try {
    await found.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data?.contentLength ?? 0;
      else if (e.event === 'Progress') { got += e.data?.chunkLength ?? 0; if (total) up.percent = Math.min(99, Math.round((got / total) * 100)); }
    });
    // En Windows el instalador cierra y reabre el visor por su cuenta.
  } catch (e) {
    up.error = errText(e);
    up.phase = 'error';
    toast(`No se pudo instalar la actualización: ${up.error}`, 4200);
  }
}

export function updateRow(): { l: string; v: string; d: string } {
  const d = 'Busca versiones nuevas en GitHub al arrancar. Pulsa A para buscar ahora; si hay una, pulsa A otra vez para instalarla (el visor se cierra y se vuelve a abrir solo). Tus datos no se tocan.';
  switch (up.phase) {
    case 'checking': return { l: 'Actualizaciones', v: 'Buscando…', d };
    case 'none': return { l: 'Actualizaciones', v: 'Estás al día', d };
    case 'available': return { l: 'Actualizaciones', v: `Instalar ${up.version}`, d };
    case 'installing': return { l: 'Actualizaciones', v: up.percent ? `Descargando ${up.percent} %` : 'Descargando…', d };
    case 'error': return { l: 'Actualizaciones', v: 'Error · A para reintentar', d: `${d} Último error: ${up.error}` };
    default: return { l: 'Actualizaciones', v: 'Buscar ahora', d };
  }
}

export function updateAct(dir: number) {
  if (dir !== 0) return;
  blip(1320);
  if (up.phase === 'available') void installUpdate();
  else void checkUpdate(false);
}
