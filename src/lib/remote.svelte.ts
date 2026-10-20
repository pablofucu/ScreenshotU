// Ajustes de la sección «Móvil»: servidor en la red local para la app Visor de Android (docs/api-movil.md).
import { app } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { patchConfig, toast } from './controller.svelte';
import type { RemoteInfo } from './types';

export type RemoteId = 'remoteon' | 'remoteaddr' | 'remotecode' | 'remotehelp';
export const REMOTE_ROWS: RemoteId[] = ['remoteon', 'remoteaddr', 'remotecode', 'remotehelp'];
export const isRemoteId = (id: string): id is RemoteId => (REMOTE_ROWS as string[]).includes(id);
export const REMOTE_CYCLE = new Set<RemoteId>(['remoteon']);

const HELP = 'Abre la app Visor en el móvil (misma red Wi-Fi) y escribe esta dirección y el código. La primera vez Windows puede pedir permiso en el cortafuegos: permite redes privadas.';

/** Último estado que ha dado el núcleo (se pide al abrir Ajustes y tras cada cambio). */
const rm = $state({ info: null as RemoteInfo | null, busy: false });

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');

function setInfo(i: RemoteInfo) {
  rm.info = i;
  // El código lo genera el núcleo: se copia para que la configuración en memoria no se quede atrás.
  if (i.code !== app.config.remoteToken) app.config.remoteToken = i.code;
}

export async function refreshRemote() {
  try { setInfo(await backend.remoteInfo()); } catch { rm.info = null; }
}

function addrLabel(i: RemoteInfo | null): string {
  if (!i) return '…';
  if (!i.enabled) return 'Desactivado';
  if (!i.running) return i.error ? 'No se pudo abrir el puerto' : 'Parado';
  const ip = i.addresses[0] ?? 'IP del PC';
  return `${ip}:${i.port} · ${i.code}`;
}

export function remoteRow(id: RemoteId): { l: string; v: string; d: string } {
  const i = rm.info;
  switch (id) {
    case 'remoteon':
      return { l: 'Acceso desde el móvil', v: app.config.remoteEnabled ? 'Activado' : 'Desactivado', d: `Deja que la app Visor del móvil vea la biblioteca, las estadísticas y las capturas por la red de casa (solo lectura: el móvil no puede cambiar ni borrar nada). ${HELP}` };
    case 'remoteaddr': {
      let d = 'Dirección (IP del PC y puerto) y código de emparejamiento que hay que escribir en el móvil.';
      if (i?.enabled && i.running && i.addresses.length > 1) d += ` Si no conecta, prueba con otra IP del PC: ${i.addresses.slice(1).map((a) => `${a}:${i.port}`).join(', ')}.`;
      if (i?.enabled && !i.running && i.error) d += ` Error: ${i.error}. Puede que otro programa use el puerto ${i.port}.`;
      return { l: 'Dirección y código', v: addrLabel(i), d };
    }
    case 'remotecode':
      return { l: 'Generar código nuevo', v: '', d: 'Pulsa A para cambiar el código de emparejamiento. Los móviles ya conectados dejarán de tener acceso hasta que escribas el código nuevo.' };
    case 'remotehelp':
      return { l: 'Cómo conectar', v: '', d: HELP };
  }
}

export function remoteAct(id: RemoteId, dir: number) {
  if (id === 'remoteon') {
    void patchConfig({ remoteEnabled: !app.config.remoteEnabled }).then(async () => {
      await refreshRemote();
      if (rm.info?.enabled && !rm.info.running && rm.info.error) toast(rm.info.error, 4200);
    });
    blip(1320);
  } else if (id === 'remotecode' && dir === 0) {
    if (rm.busy) return;
    rm.busy = true;
    backend.remoteNewCode().then(
      (i) => { setInfo(i); toast(`Código nuevo: ${i.code}`, 3200); blip(1320); },
      (e) => toast(errText(e), 4200),
    ).finally(() => { rm.busy = false; });
  }
}
