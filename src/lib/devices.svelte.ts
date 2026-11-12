// Ajustes › Conexiones, parte de «otro PC». En el PC principal (provisional hasta el asistente de la fase 3): conectarse
// a otro PC para ser su cliente. En un PC cliente: con qué PC está conectado, Moonlight y desconectarse. Los dispositivos
// emparejados y el servidor del PC principal están en `connections.svelte.ts`.
import { blip } from './sound';
import { toast } from './controller.svelte';
import { openOsk } from './osk.svelte';
import { clientMode, invoke, ready } from './ipc';

export type DevicesId =
  | 'pcfind' | 'pcaddr' | 'pcname' | 'pcpair'
  | 'pcmode' | 'pcaddrs' | 'pcmoon' | 'pcwake' | 'pcunpair';
const ALL: DevicesId[] = ['pcfind', 'pcaddr', 'pcname', 'pcpair', 'pcmode', 'pcaddrs', 'pcmoon', 'pcwake', 'pcunpair'];
export const isDevicesId = (id: string): id is DevicesId => (ALL as string[]).includes(id);
export const DEVICES_CYCLE = new Set<DevicesId>(['pcfind']);

/** Filas según el modo de este equipo (en modo cliente los dispositivos se gestionan en el PC principal). */
export function devicesRows(): DevicesId[] {
  return clientMode
    ? ['pcmode', 'pcaddrs', 'pcmoon', 'pcwake', 'pcunpair']
    : ['pcfind', 'pcaddr', 'pcname', 'pcpair'];
}

interface Found { id: string; name: string; addr: string }
interface ClientInfo {
  mode: 'host' | 'client'; hostId: string; hostName: string; hostAddr: string; hostAddrs: string[]; hostMac: string;
  deviceId: string; deviceName: string; moonlightPath: string; moonlightFound: string; suggestedName: string;
}

const dv = $state({
  found: [] as Found[],
  foundSel: -1,
  finding: false,
  addr: '',
  pcName: '',
  busy: false,
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
  if (dv.info && !dv.pcName) dv.pcName = dv.info.suggestedName;
}
export function devicesRow(id: DevicesId): { l: string; v: string; d: string } {
  const i = dv.info;
  switch (id) {
    case 'pcfind': {
      const f = dv.found[dv.foundSel];
      const v = dv.finding ? 'Buscando…' : f ? (dv.found.length > 1 ? `${f.name} (${dv.foundSel + 1}/${dv.found.length})` : f.name) : 'Buscar';
      return { l: 'Conectarse a otro PC', v, d: 'Usa la biblioteca de otro PC de casa desde este equipo: los juegos se abren allí y se juegan aquí con Moonlight. Pulsa A para buscar PCs con el visor en la red; ‹ › elige uno.' };
    }
    case 'pcaddr':
      return { l: 'Dirección del PC principal', v: dv.addr || 'Escribir', d: 'IP y puerto que salen en «Dirección y código» de ese PC (p. ej. 192.168.1.20:8787). Se rellena sola al elegirlo arriba.' };
    case 'pcname':
      return { l: 'Nombre de este equipo allí', v: dv.pcName || '…', d: 'Con este nombre sale este equipo en los dispositivos y en las estadísticas del PC principal.' };
    case 'pcpair':
      return { l: 'Conectar', v: dv.busy ? 'Conectando…' : '', d: 'Pulsa A y escribe el código de «Dirección y código» del PC principal. La biblioteca de este equipo no se toca: al desconectarte vuelve tal cual.' };
    case 'pcmode':
      return { l: 'Conectado a', v: i?.hostName || clientMode?.hostName || '…', d: `Este equipo usa la biblioteca de ese PC${i?.hostAddr ? ` (${i.hostAddr})` : ''}. Los ajustes de la biblioteca se cambian allí.` };
    case 'pcaddrs': {
      const n = i?.hostAddrs?.length ?? 0;
      return { l: 'Direcciones del PC principal', v: n > 1 ? `${n} direcciones` : n ? '1 dirección' : '…', d: `Se prueban en orden y se usa la primera que conteste: la de casa primero y después, para jugar fuera, la de Tailscale (p. ej. 100.64.1.2:8787). Pulsa A para cambiarlas, separadas por comas.${i?.hostAddrs?.length ? ` Ahora: ${i.hostAddrs.join(', ')}.` : ''}` };
    }
    case 'pcmoon':
      return { l: 'Moonlight', v: i?.moonlightFound ? (i.moonlightPath ? 'Elegido' : 'Encontrado') : 'No encontrado', d: `Con él se juega aquí lo que se abre en el PC principal${i?.moonlightFound ? ` (${i.moonlightFound})` : ''}. Pulsa A para elegir Moonlight.exe; ‹ vuelve a buscarlo solo.` };
    case 'pcwake':
      return { l: 'Encender el PC principal', v: '', d: 'Lo enciende por la red (Wake-on-LAN) si está suspendido o apagado y su placa lo permite.' };
    case 'pcunpair':
      return { l: 'Desconectar', v: dv.confirm === 'unpair' ? 'Pulsa otra vez' : '', d: 'Este equipo vuelve a usar su propia biblioteca. En el PC principal deja de salir como dispositivo.' };
  }
}

async function pair() {
  if (dv.busy) return;
  if (!dv.addr.trim()) { toast('Busca el PC o escribe su dirección', 3000); return; }
  if (!dv.pcName.trim()) { toast('Ponle un nombre a este equipo', 3000); return; }
  openOsk({
    label: 'Código del PC principal',
    hint: 'El de Ajustes → Mando → Dirección y código en ese PC',
    value: '',
    maxLen: 32,
    onDone: (code) => {
      if (!code.trim()) return;
      dv.busy = true;
      invoke('client_pair_ticket', { addr: dv.addr, ticket: code })
        .then(() => { toast('Conectado: recargando…', 2000); setTimeout(() => location.reload(), 600); })
        .catch((e) => { toast(errText(e), 5000); dv.busy = false; });
    },
  });
}

export function devicesAct(id: DevicesId, dir: number) {
  if (id !== 'pcunpair') dv.confirm = '';
  switch (id) {
    case 'pcfind': {
      if (dir !== 0 && dv.found.length > 1) {
        dv.foundSel = (dv.foundSel + dir + dv.found.length) % dv.found.length;
        dv.addr = dv.found[dv.foundSel].addr;
        blip(1320);
        return;
      }
      if (dv.finding) return;
      dv.finding = true;
      invoke<Found[]>('client_discover')
        .then((f) => {
          dv.found = f;
          dv.foundSel = f.length ? 0 : -1;
          if (f.length) dv.addr = f[0].addr;
          else toast('No se encontró ningún PC con el visor: escribe su dirección abajo', 4000);
        })
        .catch((e) => toast(errText(e), 4200))
        .finally(() => { dv.finding = false; });
      blip(1320);
      return;
    }
    case 'pcaddr':
      if (dir !== 0) return;
      openOsk({ label: 'Dirección del PC principal', hint: 'IP:puerto, p. ej. 192.168.1.20:8787', value: dv.addr, maxLen: 64, onDone: (v) => { dv.addr = v.trim(); } });
      return;
    case 'pcname':
      if (dir !== 0) return;
      openOsk({ label: 'Nombre de este equipo', value: dv.pcName, maxLen: 60, capitalize: true, onDone: (v) => { dv.pcName = v.trim(); } });
      return;
    case 'pcpair':
      if (dir === 0) void pair();
      return;
    case 'pcmode':
      return;
    case 'pcaddrs':
      if (dir !== 0) return;
      openOsk({
        label: 'Direcciones del PC principal',
        hint: 'IP:puerto separadas por comas; la de casa primero',
        value: (dv.info?.hostAddrs ?? []).join(', '),
        maxLen: 200,
        onDone: (v) => {
          invoke<ClientInfo>('client_set_addrs', { addrs: [v] })
            .then((r) => { dv.info = r; blip(1320); })
            .catch((e) => toast(errText(e), 4200));
        },
      });
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
