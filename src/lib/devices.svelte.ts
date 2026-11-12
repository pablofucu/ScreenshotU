// Ajustes › Conexiones, parte de «otro PC». En el PC principal: una fila que abre el asistente «Usar la biblioteca de otro PC»
// (`usepc.svelte.ts`). En un PC cliente: con qué PC está conectado, Moonlight, encenderlo y desconectarse. Los dispositivos
// emparejados y el servidor del PC principal están en `connections.svelte.ts`.
import { blip } from './sound';
import { toast } from './controller.svelte';
import { openUsePc } from './usepc.svelte';
import { clientMode, invoke, ready } from './ipc';
import type { ClientInfoOut } from './types';

export type DevicesId = 'pcuse' | 'pcmode' | 'pcmoon' | 'pcwake' | 'pcunpair';
const ALL: DevicesId[] = ['pcuse', 'pcmode', 'pcmoon', 'pcwake', 'pcunpair'];
export const isDevicesId = (id: string): id is DevicesId => (ALL as string[]).includes(id);
export const DEVICES_CYCLE = new Set<DevicesId>();

/** Filas según el modo de este equipo (en modo cliente los dispositivos se gestionan en el PC principal). */
export function devicesRows(): DevicesId[] {
  return clientMode
    ? ['pcmode', 'pcmoon', 'pcwake', 'pcunpair']
    : ['pcuse'];
}

type ClientInfo = ClientInfoOut;

const dv = $state({
  info: null as ClientInfo | null,
  /** Dos pulsaciones para desconectarse. */
  confirm: '' as '' | 'unpair',
});
/** Para la barra superior: este equipo es cliente de otro PC (botón de encenderlo). */
export const pcView = $state({ client: false });
void ready.then(() => { pcView.client = !!clientMode; });

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');
let confirmT: ReturnType<typeof setTimeout> | undefined;
function armed(what: 'unpair', msg: string): boolean {
  if (dv.confirm === what) { clearTimeout(confirmT); dv.confirm = ''; return true; }
  dv.confirm = what;
  clearTimeout(confirmT);
  confirmT = setTimeout(() => { dv.confirm = ''; }, 3500);
  toast(msg, 3000);
  blip(520);
  return false;
}

/** Al abrir Ajustes. */
export async function refreshDevices() {
  await ready;
  try { dv.info = await invoke<ClientInfo>('client_info'); } catch { dv.info = null; }
}
export function devicesRow(id: DevicesId): { l: string; v: string; d: string } {
  const i = dv.info;
  switch (id) {
    case 'pcuse':
      return { l: 'Usar la biblioteca de otro PC', v: '', d: 'Usa la biblioteca de otro PC de casa desde este equipo: los juegos se abren allí y se juegan aquí con Moonlight. Pulsa A para empezar; el otro PC tendrá que aprobarte con un número de 4 cifras. La biblioteca de este equipo no se toca.' };
    case 'pcmode':
      return { l: 'Conectado a', v: i?.hostName || clientMode?.hostName || '…', d: `Este equipo usa la biblioteca de ese PC${i?.hostAddr ? ` (${i.hostAddr})` : ''}. Los ajustes de la biblioteca se cambian allí.` };
    case 'pcmoon':
      return { l: 'Moonlight', v: i?.moonlightFound ? (i.moonlightPath ? 'Elegido' : 'Encontrado') : 'No encontrado', d: `Con él se juega aquí lo que se abre en el PC principal${i?.moonlightFound ? ` (${i.moonlightFound})` : ''}. Pulsa A para elegir Moonlight.exe; ‹ vuelve a buscarlo solo.` };
    case 'pcwake':
      return { l: 'Encender el PC principal', v: '', d: 'Lo enciende por la red (Wake-on-LAN) si está suspendido o apagado y su placa lo permite.' };
    case 'pcunpair':
      return { l: 'Desconectar', v: dv.confirm === 'unpair' ? 'Pulsa otra vez' : '', d: 'Este equipo vuelve a usar su propia biblioteca. En el PC principal deja de salir como dispositivo.' };
  }
}

export function devicesAct(id: DevicesId, dir: number) {
  if (id !== 'pcunpair') dv.confirm = '';
  switch (id) {
    case 'pcuse':
      if (dir === 0) openUsePc();
      return;
    case 'pcmode':
      return;
    case 'pcmoon':
      void pickMoonlight(dir);
      return;
    case 'pcwake':
      if (dir !== 0) return;
      invoke('client_wake').then(() => toast('Encendiendo el PC principal…', 3000)).catch((e) => toast(errText(e), 4200));
      return;
    case 'pcunpair':
      if (dir !== 0) return;
      if (!armed('unpair', 'Pulsa otra vez para desconectar este equipo')) return;
      invoke('client_unpair')
        .then(() => { toast('Desconectado: recargando…', 2000); setTimeout(() => location.reload(), 600); })
        .catch((e) => toast(errText(e), 4200));
      return;
  }
}

async function pickMoonlight(dir: number) {
  let path = '';
  if (dir === 0) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const p = await open({ title: 'Elige Moonlight.exe', filters: [{ name: 'Moonlight', extensions: ['exe'] }] });
    if (typeof p !== 'string') return;
    path = p;
  } else if (dir > 0) return;
  try { dv.info = await invoke<ClientInfo>('client_set_moonlight', { path }); blip(1320); } catch (e) { toast(errText(e), 4200); }
}
