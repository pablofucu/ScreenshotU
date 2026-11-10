// Ajustes de la sección «Equipos». En el PC principal: su nombre, los dispositivos emparejados (tablets y otros PCs)
// y conectarse a otro PC para ser su cliente. En un PC cliente: con qué PC está conectado, Moonlight y desconectarse.
import { blip } from './sound';
import { toast } from './controller.svelte';
import { openOsk } from './osk.svelte';
import { clientMode, invoke, ready } from './ipc';
import type { RemoteInfo } from './types';

export type DevicesId =
  | 'devname' | 'devlist' | 'devrename' | 'devrevoke'
  | 'pcfind' | 'pcaddr' | 'pcname' | 'pcpair'
  | 'pcmode' | 'pcaddrs' | 'pcmoon' | 'pcwake' | 'pcunpair';
const ALL: DevicesId[] = ['devname', 'devlist', 'devrename', 'devrevoke', 'pcfind', 'pcaddr', 'pcname', 'pcpair', 'pcmode', 'pcaddrs', 'pcmoon', 'pcwake', 'pcunpair'];
export const isDevicesId = (id: string): id is DevicesId => (ALL as string[]).includes(id);
export const DEVICES_CYCLE = new Set<DevicesId>(['devlist', 'pcfind']);

/** Filas según el modo de este equipo (en modo cliente los dispositivos se gestionan en el PC principal). */
export function devicesRows(): DevicesId[] {
  return clientMode
    ? ['pcmode', 'pcaddrs', 'pcmoon', 'pcwake', 'pcunpair']
    : ['devname', 'devlist', 'devrename', 'devrevoke', 'pcfind', 'pcaddr', 'pcname', 'pcpair'];
}

interface Device { id: string; name: string; kind: string; createdMs: number; lastSeenMs: number; appVersion: string; api: number; revoked: boolean }
interface Found { id: string; name: string; addr: string }
interface ClientInfo {
  mode: 'host' | 'client'; hostId: string; hostName: string; hostAddr: string; hostAddrs: string[]; hostMac: string;
  deviceId: string; deviceName: string; moonlightPath: string; moonlightFound: string; suggestedName: string;
}

const dv = $state({
  name: '',
  list: [] as Device[],
  sel: 0,
  found: [] as Found[],
  foundSel: -1,
  finding: false,
  addr: '',
  pcName: '',
  busy: false,
  info: null as ClientInfo | null,
  /** Dos pulsaciones para quitar un dispositivo o desconectarse. */
  confirm: '' as '' | 'revoke' | 'unpair',
});
/** Para la barra superior: este equipo es cliente de otro PC (botón de encenderlo). */
export const pcView = $state({ client: false });
void ready.then(() => { pcView.client = !!clientMode; });

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');
const live = () => dv.list.filter((d) => !d.revoked);
const cur = () => live()[dv.sel] as Device | undefined;
let confirmT: ReturnType<typeof setTimeout> | undefined;
function armed(what: 'revoke' | 'unpair', msg: string): boolean {
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
  void listenChanges();
  try { dv.info = await invoke<ClientInfo>('client_info'); } catch { dv.info = null; }
  if (dv.info && !dv.pcName) dv.pcName = dv.info.suggestedName;
  if (clientMode) return;
  try { dv.name = (await invoke<RemoteInfo & { name?: string }>('remote_info')).name ?? ''; } catch { /* sin servidor */ }
  try {
    dv.list = await invoke<Device[]>('devices_list');
    dv.sel = Math.min(dv.sel, Math.max(0, live().length - 1));
  } catch { dv.list = []; }
}
let listening = false;
async function listenChanges() {
  if (listening) return;
  listening = true;
  const { listen } = await import('./ipc');
  await listen('devices-changed', () => { void refreshDevices(); });
}

const KIND: Record<string, string> = { tablet: 'Tablet', pc: 'PC' };
function when(ms: number): string {
  if (!ms) return 'nunca';
  return new Date(ms).toLocaleString('es-ES', { dateStyle: 'short', timeStyle: 'short' });
}

export function devicesRow(id: DevicesId): { l: string; v: string; d: string } {
  const i = dv.info;
  switch (id) {
    case 'devname':
      return { l: 'Nombre de este equipo', v: dv.name || '…', d: 'Cómo lo ven la tablet y los otros PCs al buscarlo en la red. Pulsa A para cambiarlo.' };
    case 'devlist': {
      const l = live();
      const d = cur();
      if (!d) return { l: 'Dispositivos conectados', v: 'Ninguno', d: 'Aquí salen las tablets y los PCs que se han conectado a este equipo con el código de «Dirección y código».' };
      return {
        l: 'Dispositivos conectados',
        v: l.length > 1 ? `${d.name} (${dv.sel + 1}/${l.length})` : d.name,
        d: `${KIND[d.kind] ?? d.kind} · último uso ${when(d.lastSeenMs)}${d.appVersion ? ` · versión ${d.appVersion}` : ''}. ‹ › elige otro.`,
      };
    }
    case 'devrename':
      return { l: 'Renombrar dispositivo', v: cur() ? '' : '—', d: 'Cambia el nombre con el que sale el dispositivo elegido arriba (también en las estadísticas de partidas).' };
    case 'devrevoke':
      return { l: 'Quitar dispositivo', v: dv.confirm === 'revoke' ? 'Pulsa otra vez' : cur() ? '' : '—', d: 'El dispositivo elegido arriba deja de tener acceso; para volver tendrá que conectarse otra vez con el código. Sus partidas ya guardadas se quedan.' };
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
      invoke('client_pair', { addr: dv.addr, code, deviceName: dv.pcName })
        .then(() => { toast('Conectado: recargando…', 2000); setTimeout(() => location.reload(), 600); })
        .catch((e) => { toast(errText(e), 5000); dv.busy = false; });
    },
  });
}

export function devicesAct(id: DevicesId, dir: number) {
  if (id !== 'devrevoke' && id !== 'pcunpair') dv.confirm = '';
  switch (id) {
    case 'devname':
      if (dir !== 0) return;
      openOsk({
        label: 'Nombre de este equipo', value: dv.name, maxLen: 60, capitalize: true,
        onDone: (v) => {
          invoke<RemoteInfo & { name?: string }>('set_device_name', { name: v })
            .then((r) => { dv.name = r.name ?? v; blip(1320); })
            .catch((e) => toast(errText(e), 4200));
        },
      });
      return;
    case 'devlist': {
      const n = live().length;
      if (n > 1) { dv.sel = (dv.sel + (dir || 1) + n) % n; blip(1320); }
      return;
    }
    case 'devrename': {
      const d = cur();
      if (!d || dir !== 0) return;
      openOsk({
        label: 'Nombre del dispositivo', value: d.name, maxLen: 60, capitalize: true,
        onDone: (v) => {
          if (!v.trim()) return;
          invoke('device_rename', { id: d.id, name: v.trim() }).then(() => refreshDevices()).catch((e) => toast(errText(e), 4200));
        },
      });
      return;
    }
    case 'devrevoke': {
      const d = cur();
      if (!d || dir !== 0) return;
      if (!armed('revoke', `Pulsa otra vez para quitar ${d.name}`)) return;
      invoke('device_revoke', { id: d.id })
        .then(() => { toast(`${d.name} ya no tiene acceso`, 3000); return refreshDevices(); })
        .catch((e) => toast(errText(e), 4200));
      return;
    }
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
