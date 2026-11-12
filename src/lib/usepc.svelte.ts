// Asistente «Usar la biblioteca de otro PC» (este equipo como cliente) y aviso «ya no estás emparejado».
// Flujo: elegir PC (o dirección y billete) → aprobación con 4 cifras en el otro PC → emparejar Moonlight → recargar.
// Se muestra encima de todo (components/UseOtherPc.svelte); el mando lo lleva `useCmd`, enganchado en `cmd`.
import { blip } from './sound';
import { toast } from './controller.svelte';
import { invoke, listen, ready } from './ipc';
import { openOsk } from './osk.svelte';
import type { ClientInfoOut, ClientUnpairedEvent, PairRequestOut, PairWaitOut } from './types';

export interface FoundPc { id: string; name: string; addr: string }
export type UsePhase = '' | 'pick' | 'manual' | 'wait' | 'rejected' | 'expired' | 'stream' | 'streamerr' | 'done' | 'unpaired';

export const up = $state({
  phase: '' as UsePhase,
  found: [] as FoundPc[],
  sel: 0,
  finding: false,
  /** PC elegido (nombre y dirección) y solicitud en curso. */
  host: '',
  addr: '',
  reqId: '',
  digits: '',
  expiresMs: 0,
  now: Date.now(),
  /** Dirección y billete escritos a mano; `field` = 0 dirección, 1 billete, 2 conectar. */
  mAddr: '',
  mTicket: '',
  field: 0,
  busy: false,
  err: '',
  /** Tras cerrar hay que recargar (el aviso de «ya no emparejado» deja la interfaz en modo cliente). */
  reload: false,
});

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');
let session = 0;
let tickT: ReturnType<typeof setInterval> | undefined;

function startTick() {
  clearInterval(tickT);
  up.now = Date.now();
  tickT = setInterval(() => { up.now = Date.now(); }, 500);
}

export function openUsePc() {
  session++;
  up.phase = 'pick';
  up.err = '';
  blip(1320);
  void discover();
}

export function closeUsePc() {
  session++;
  clearInterval(tickT);
  if (up.phase === 'wait' && up.reqId) void invoke('client_pair_cancel', { addr: up.addr, requestId: up.reqId }).catch(() => {});
  const reload = up.reload;
  up.phase = '';
  up.reqId = '';
  up.busy = false;
  if (reload) location.reload();
}

async function discover() {
  if (up.finding) return;
  up.finding = true;
  try {
    up.found = await invoke<FoundPc[]>('client_discover');
    up.sel = 0;
  } catch (e) { toast(errText(e), 4200); }
  up.finding = false;
}

async function request(addr: string, name: string) {
  if (up.busy) return;
  up.busy = true;
  const my = ++session;
  try {
    const r = await invoke<PairRequestOut>('client_pair_request', { addr });
    if (my !== session) { void invoke('client_pair_cancel', { addr, requestId: r.requestId }).catch(() => {}); return; }
    up.addr = addr;
    up.host = r.hostName || name;
    up.reqId = r.requestId;
    up.digits = r.digits;
    up.expiresMs = r.expiresMs || Date.now() + 120000;
    up.phase = 'wait';
    startTick();
    blip(1320);
    up.busy = false;
    void poll(my, addr, r.requestId);
  } catch (e) {
    up.busy = false;
    toast(errText(e), 5000);
  }
}

async function poll(my: number, addr: string, requestId: string) {
  while (my === session) {
    let r: PairWaitOut;
    try { r = await invoke<PairWaitOut>('client_pair_wait', { addr, requestId }); }
    catch (e) {
      if (my !== session) return;
      toast(errText(e), 5000);
      up.phase = 'pick';
      up.reqId = '';
      return;
    }
    if (my !== session) return;
    if (r.status === 'pending') {
      if (Date.now() > up.expiresMs + 5000) { up.phase = 'expired'; up.reqId = ''; return; }
      await new Promise((ok) => setTimeout(ok, 1500));
      continue;
    }
    up.reqId = '';
    if (r.status === 'approved') { up.host = r.info?.hostName || up.host; void streamPair(); return; }
    up.phase = r.status;
    blip(520);
    return;
  }
}

async function connectManual() {
  if (up.busy) return;
  if (!up.mAddr.trim()) { toast('Escribe la dirección del PC', 3000); return; }
  if (!up.mTicket.trim()) { toast('Escribe el billete', 3000); return; }
  up.busy = true;
  try {
    const i = await invoke<ClientInfoOut>('client_pair_ticket', { addr: up.mAddr.trim(), ticket: up.mTicket.trim() });
    up.host = i.hostName || up.mAddr;
    up.busy = false;
    void streamPair();
  } catch (e) {
    up.busy = false;
    toast(errText(e), 5000);
  }
}

async function streamPair() {
  up.phase = 'stream';
  up.err = '';
  up.busy = true;
  try {
    await invoke('client_stream_pair');
    up.busy = false;
    finish();
  } catch (e) {
    up.busy = false;
    up.err = errText(e);
    up.phase = 'streamerr';
    blip(520);
  }
}

function finish() {
  up.phase = 'done';
  up.reload = true;
  blip(1320);
  toast(`Conectado a ${up.host}: recargando…`, 2000);
  const my = session;
  setTimeout(() => { if (my === session && up.phase === 'done') location.reload(); }, 1500);
}

function editField(f: 0 | 1) {
  openOsk(f === 0
    ? { label: 'Dirección del PC', hint: 'IP:puerto, p. ej. 192.168.1.20:8787', value: up.mAddr, maxLen: 64, onDone: (v) => { up.mAddr = v.trim(); } }
    : { label: 'Billete', hint: '8 letras y cifras, p. ej. ABCD-EFGH', value: up.mTicket, maxLen: 16, onDone: (v) => { up.mTicket = v.trim().toUpperCase(); } });
}

/** Mando y teclado mientras el asistente está abierto. Devuelve true si se ha llevado la orden. */
export function useCmd(c: string): boolean {
  const p = up.phase;
  if (!p) return false;
  const back = c === 'b' || c === 'menu';
  switch (p) {
    case 'pick':
      if (c === 'up' || c === 'down') {
        if (up.found.length) up.sel = (up.sel + (c === 'down' ? 1 : -1) + up.found.length) % up.found.length;
        blip(1320);
      } else if (c === 'a') {
        const f = up.found[up.sel];
        if (f) void request(f.addr, f.name);
      } else if (c === 'x') { up.phase = 'manual'; up.field = 0; blip(1320); }
      else if (c === 'y') { blip(1320); void discover(); }
      else if (back) closeUsePc();
      break;
    case 'manual':
      if (c === 'up') up.field = Math.max(0, up.field - 1);
      else if (c === 'down') up.field = Math.min(2, up.field + 1);
      else if (c === 'a') { if (up.field === 2) void connectManual(); else editField(up.field as 0 | 1); }
      else if (back) { up.phase = 'pick'; blip(520); }
      break;
    case 'wait':
      if (back) closeUsePc();
      break;
    case 'rejected': case 'expired':
      if (c === 'a') openUsePc();
      else if (back) closeUsePc();
      break;
    case 'stream':
      if (back) { toast('Un momento: se está emparejando el streaming', 2500); }
      break;
    case 'streamerr':
      if (c === 'a') void streamPair();
      else if (back) { finish(); }
      break;
    case 'done':
      if (c === 'a') location.reload();
      break;
    case 'unpaired':
      if (c === 'a') { up.reload = true; openUsePc(); }
      else if (back) { up.reload = true; closeUsePc(); }
      break;
  }
  return true;
}

let started = false;
/** Solo en el escritorio: el PC principal ha quitado a este equipo de sus dispositivos. */
export async function startUsePc() {
  if (started) return;
  started = true;
  await ready;
  if (!('__TAURI_INTERNALS__' in window)) return;
  try {
    await listen<ClientUnpairedEvent>('client-unpaired', (e) => {
      if (up.phase === 'pick' || up.phase === 'wait' || up.phase === 'manual') return;
      up.host = e.payload?.hostName || 'el PC principal';
      up.phase = 'unpaired';
      up.reload = true;
      blip(520);
    });
  } catch { /* sin Tauri: no hay eventos */ }
}
