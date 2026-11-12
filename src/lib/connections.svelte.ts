// Ajustes › Conexiones en el PC principal: este PC (nombre, permitir conexiones), los dispositivos emparejados, el
// asistente «Añadir dispositivo» y lo avanzado (puerto, direcciones, Apollo, cursor). Sustituye a `remote.svelte.ts`
// y a la parte de PC principal de `devices.svelte.ts`. En modo cliente la sección sigue siendo la de `devices.svelte.ts`.
import { app } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { patchConfig, toast } from './controller.svelte';
import { openOsk } from './osk.svelte';
import { clientMode, listen, ready } from './ipc';
import type { ApolloWebInfo, PairedDevice, PairRequestInfo, PairTicketInfo, RemoteInfo, StreamAppStatus } from './types';

export type ConnId =
  | 'connname' | 'connon' | 'connwin'
  | 'conndev' | 'connrename' | 'connrevoke' | 'connadd'
  | 'connport' | 'connaddrs' | 'connapp' | 'conncreds' | 'conncredsclear' | 'conncursor';
/** Filas de la sección en el PC principal, en el orden de la maqueta (Este PC, Dispositivos, Avanzado). */
export const CONN_ROWS: ConnId[] = [
  'connname', 'connon', 'connwin',
  'conndev', 'connrename', 'connrevoke', 'connadd',
  'connport', 'connaddrs', 'connapp', 'conncreds', 'conncredsclear', 'conncursor',
];
export const isConnId = (id: string): id is ConnId => (CONN_ROWS as string[]).includes(id);
/** Filas con ‹ ›: los interruptores y la elección de dispositivo. */
export const CONN_CYCLE = new Set<ConnId>(['connon', 'connwin', 'conndev', 'conncursor']);

/** Pasos del asistente «Añadir dispositivo»: esperando solicitudes o QR con billete. */
export type WizardStep = '' | 'wait' | 'qr';

const cn = $state({
  info: null as RemoteInfo | null,
  list: [] as PairedDevice[],
  sel: 0,
  addrs: [] as string[],
  apollo: null as StreamAppStatus | null,
  apolloBusy: false,
  web: null as ApolloWebInfo | null,
  /** Dos pulsaciones para quitar un dispositivo o las credenciales de Apollo. */
  confirm: '' as '' | 'revoke' | 'creds',
  wizard: '' as WizardStep,
  pending: [] as PairRequestInfo[],
  ticket: null as PairTicketInfo | null,
  ticketError: '',
  now: Date.now(),
});
/** Para la pantalla de Ajustes y el asistente. */
export const connView = cn;

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');
const live = () => cn.list.filter((d) => !d.revoked);
const cur = () => live()[cn.sel] as PairedDevice | undefined;
let confirmT: ReturnType<typeof setTimeout> | undefined;
function armed(what: 'revoke' | 'creds', msg: string): boolean {
  if (cn.confirm === what) { clearTimeout(confirmT); cn.confirm = ''; return true; }
  cn.confirm = what;
  clearTimeout(confirmT);
  confirmT = setTimeout(() => { cn.confirm = ''; }, 3500);
  toast(msg, 3000);
  blip(520);
  return false;
}

/** Al abrir Ajustes y tras cada cambio. */
export async function refreshConnections() {
  await ready;
  if (clientMode) return;
  void listenChanges();
  try { cn.info = await backend.remoteInfo(); } catch { cn.info = null; }
  try { cn.list = await backend.devicesList(); cn.sel = Math.min(cn.sel, Math.max(0, live().length - 1)); } catch { cn.list = []; }
  try { cn.addrs = await backend.hostAddrs(); } catch { cn.addrs = []; }
  try { cn.apollo = await backend.streamAppStatus(); } catch { cn.apollo = null; }
  try { cn.web = await backend.apolloWebState(); } catch { cn.web = null; }
}
let listening = false;
async function listenChanges() {
  if (listening) return;
  listening = true;
  try { await listen('devices-changed', () => { void refreshConnections(); }); } catch { listening = false; }
}

const KIND: Record<string, string> = { tablet: 'Tablet', pc: 'PC' };
function ago(ms: number): string {
  if (!ms) return 'nunca';
  const m = Math.max(0, Math.round((Date.now() - ms) / 60000));
  if (m < 1) return 'ahora mismo';
  if (m < 60) return `hace ${m} min`;
  const h = Math.round(m / 60);
  if (h < 24) return `hace ${h} h`;
  const d = Math.round(h / 24);
  return d === 1 ? 'ayer' : `hace ${d} días`;
}
/** Línea de un dispositivo: tipo, versión y último uso. */
export const deviceLine = (d: PairedDevice) => [KIND[d.kind] ?? d.kind, d.appVersion ? `Visor ${d.appVersion}` : '', ago(d.lastSeenMs)].filter(Boolean).join(' · ');
const WEB: Record<string, string> = {
  ok: 'Conectado', badCredentials: 'Usuario o contraseña incorrectos', apolloOff: 'Apollo no responde',
  incompatible: 'Versión de Apollo no compatible', noCredentials: 'Sin guardar',
};
const yn = (b: boolean) => (b ? 'Sí' : 'No');

export function connRow(id: ConnId): { l: string; v: string; d: string } {
  const i = cn.info;
  switch (id) {
    case 'connname':
      return { l: 'Nombre de este PC', v: i?.name || '…', d: 'Cómo lo ven la tablet y los otros PCs al buscarlo en la red. Pulsa A para cambiarlo con el teclado en pantalla.' };
    case 'connon':
      return { l: 'Permitir conexiones', v: yn(app.config.remoteEnabled), d: 'Deja que las tablets y los otros PCs de casa vean la biblioteca, las estadísticas y las capturas (solo lectura: nadie puede cambiar ni borrar nada desde fuera). La primera vez Windows puede pedir permiso en el cortafuegos: permite redes privadas.' };
    case 'connwin':
      return { l: 'Arrancar con Windows', v: yn(app.config.startWithWindows), d: 'Abre el visor en segundo plano al iniciar sesión en Windows (solo el icono junto al reloj, sin ventana) para que la tablet conecte sin abrir nada. Con esto activado, cerrar el visor lo deja en la bandeja; «Salir» del icono lo cierra del todo.' };
    case 'conndev': {
      const l = live();
      const d = cur();
      if (!d) return { l: 'Dispositivos', v: 'Ninguno', d: 'Aquí salen las tablets y los PCs emparejados con este PC. Usa «Añadir dispositivo» para emparejar el primero.' };
      return {
        l: 'Dispositivos',
        v: l.length > 1 ? `${d.name} (${cn.sel + 1}/${l.length})` : d.name,
        d: `${deviceLine(d)}. Streaming ${d.apolloId ? 'emparejado' : 'sin emparejar'}${d.apolloId ? '' : ': ábrelo desde ese dispositivo y el visor lo empareja con Moonlight'}.${l.length > 1 ? ' ‹ › elige otro.' : ''}`,
      };
    }
    case 'connrename':
      return { l: 'Renombrar dispositivo', v: cur() ? '' : '—', d: 'Cambia el nombre con el que sale el dispositivo elegido arriba (también en las estadísticas de partidas).' };
    case 'connrevoke':
      return { l: 'Quitar dispositivo', v: cn.confirm === 'revoke' ? 'Pulsa otra vez' : cur() ? '' : '—', d: 'El dispositivo elegido arriba deja de tener acceso, también al streaming; para volver tendrá que emparejarse otra vez. Sus partidas ya guardadas se quedan.' };
    case 'connadd':
      return { l: 'Añadir dispositivo', v: '', d: 'Empareja una tablet u otro PC: espera su solicitud y apruébala comparando el número de 4 cifras, o muestra un QR de un solo uso si no se ven en la red.' };
    case 'connport':
      return { l: 'Puerto', v: String(i?.port ?? app.config.remotePort), d: `Por el que el visor atiende a los dispositivos. Cámbialo solo si otro programa ya lo usa; los dispositivos emparejados lo aprenden solos.${i?.enabled && !i.running && i.error ? ` Error: ${i.error}.` : ''} Pulsa A para escribir otro (1024-65535).` };
    case 'connaddrs': {
      const n = cn.addrs.length;
      return { l: 'Direcciones detectadas', v: n === 0 ? (i?.enabled ? 'Ninguna' : 'Desactivado') : n === 1 ? cn.addrs[0] : `${n} direcciones`, d: n ? `Se detectan solas y se envían al emparejar, no hay que escribirlas: ${cn.addrs.join(', ')}.` : 'Se detectan solas cuando las conexiones están permitidas y el servidor está en marcha.' };
    }
    case 'connapp': {
      const a = cn.apollo;
      const v = cn.apolloBusy ? 'Creando…' : !a ? '…' : !a.found ? 'Apollo no encontrado' : !a.installed ? 'Crear' : a.outdated ? 'Actualizar' : 'Creada';
      return { l: 'App «Juego actual» de Apollo', v, d: 'Crea en Apollo una app «Juego actual» (hace falta aceptar el permiso de administrador y reinicia su servicio). Con ella, el juego se abre en el PC al conectar la tablet y la sesión de streaming termina sola al cerrarse el juego. Sin ella funciona con «Desktop», pero la sesión no se cierra sola.' };
    }
    case 'conncreds': {
      const w = cn.web;
      return { l: 'Credenciales de Apollo', v: !w ? '…' : w.hasCredentials ? (w.state === 'ok' ? 'Guardadas' : WEB[w.state] ?? 'Guardadas') : 'Sin guardar', d: `El usuario y la contraseña del panel web de Apollo: con ellos el visor empareja el streaming de cada dispositivo sin que nadie escriba el PIN. Se guardan cifradas en este PC. Pulsa A para escribirlos (primero el usuario y después la contraseña).${w?.state === 'incompatible' ? ' Esta versión de Apollo no permite emparejar así.' : ''}` };
    }
    case 'conncredsclear':
      return { l: 'Olvidar credenciales de Apollo', v: cn.confirm === 'creds' ? 'Pulsa otra vez' : cn.web?.hasCredentials ? '' : '—', d: 'Borra el usuario y la contraseña guardados. El streaming de los dispositivos ya emparejados sigue funcionando; para emparejar otros nuevos tendrás que volver a guardarlos.' };
    case 'conncursor':
      return { l: 'Ocultar el cursor en el streaming', v: yn(app.config.streamHideCursor), d: 'Mientras juegas desde la tablet, el cursor del PC se vuelve transparente para que no salga sobre el juego. Desactívalo si usas el modo trackpad de Artemis, donde el puntero tiene que verse.' };
  }
}

export function connAct(id: ConnId, dir: number) {
  if (id !== 'connrevoke' && id !== 'conncredsclear') cn.confirm = '';
  switch (id) {
    case 'connname':
      if (dir !== 0) return;
      openOsk({
        label: 'Nombre de este PC', hint: 'Vacío = el nombre de Windows', value: cn.info?.name ?? '', maxLen: 60, capitalize: true,
        onDone: (v) => {
          backend.setDeviceName(v)
            .then((r) => { cn.info = r; blip(1320); })
            .catch((e) => toast(errText(e), 4200));
        },
      });
      return;
    case 'connon':
      void patchConfig({ remoteEnabled: !app.config.remoteEnabled }).then(async () => {
        await refreshConnections();
        if (cn.info?.enabled && !cn.info.running && cn.info.error) toast(cn.info.error, 4200);
      });
      blip(1320);
      return;
    case 'connwin':
      void patchConfig({ startWithWindows: !app.config.startWithWindows });
      blip(1320);
      return;
    case 'conndev': {
      const n = live().length;
      if (n > 1) { cn.sel = (cn.sel + (dir || 1) + n) % n; blip(1320); }
      return;
    }
    case 'connrename': {
      const d = cur();
      if (!d || dir !== 0) return;
      openOsk({
        label: 'Nombre del dispositivo', value: d.name, maxLen: 60, capitalize: true,
        onDone: (v) => {
          if (!v.trim()) return;
          backend.deviceRename(d.id, v.trim()).then(() => refreshConnections()).catch((e) => toast(errText(e), 4200));
        },
      });
      return;
    }
    case 'connrevoke': {
      const d = cur();
      if (!d || dir !== 0) return;
      if (!armed('revoke', `Pulsa otra vez para quitar ${d.name}`)) return;
      backend.deviceRevoke(d.id)
        .then(() => { toast(`${d.name} ya no tiene acceso`, 3000); return refreshConnections(); })
        .catch((e) => toast(errText(e), 4200));
      return;
    }
    case 'connadd':
      if (dir === 0) openWizard();
      return;
    case 'connport':
      if (dir !== 0) return;
      openOsk({
        label: 'Puerto', hint: 'Entre 1024 y 65535', value: String(cn.info?.port ?? app.config.remotePort), maxLen: 5,
        onDone: (v) => {
          const n = Number.parseInt(v, 10);
          if (!Number.isInteger(n) || n < 1024 || n > 65535) { toast('El puerto va de 1024 a 65535', 3200); return; }
          void patchConfig({ remotePort: n }).then(async () => {
            await refreshConnections();
            if (cn.info?.enabled && !cn.info.running && cn.info.error) toast(cn.info.error, 4200);
          });
        },
      });
      return;
    case 'connaddrs':
      return;
    case 'connapp': {
      if (dir !== 0 || cn.apolloBusy) return;
      if (!cn.apollo?.found) { toast('No se encontró Apollo ni Sunshine en este PC', 3600); return; }
      cn.apolloBusy = true;
      backend.streamAppInstall()
        .then((s) => { cn.apollo = s; toast('«Juego actual» creada en Apollo', 3200); })
        .catch((e) => toast(errText(e), 4200))
        .finally(() => { cn.apolloBusy = false; });
      blip(1320);
      return;
    }
    case 'conncreds':
      if (dir !== 0) return;
      openOsk({
        label: 'Usuario de Apollo', hint: 'El del panel web de Apollo (paso 1 de 2)', value: '', maxLen: 80,
        onDone: (user) => {
          if (!user.trim()) return;
          openOsk({
            label: 'Contraseña de Apollo', hint: 'Se guarda cifrada en este PC (paso 2 de 2)', value: '', maxLen: 120,
            onDone: (pass) => {
              if (!pass) return;
              backend.apolloCredsSet(user.trim(), pass)
                .then((w) => { cn.web = w; toast(w.state === 'ok' ? 'Credenciales de Apollo guardadas' : WEB[w.state] ?? 'Guardadas', 3600); })
                .catch((e) => toast(errText(e), 4200));
            },
          });
        },
      });
      return;
    case 'conncredsclear':
      if (dir !== 0 || !cn.web?.hasCredentials) return;
      if (!armed('creds', 'Pulsa otra vez para olvidar las credenciales de Apollo')) return;
      backend.apolloCredsClear()
        .then((w) => { cn.web = w; toast('Credenciales de Apollo borradas', 3000); })
        .catch((e) => toast(errText(e), 4200));
      return;
    case 'conncursor':
      void patchConfig({ streamHideCursor: !app.config.streamHideCursor });
      blip(1320);
      return;
  }
}

/* ---------- asistente «Añadir dispositivo» ---------- */

let pollT: ReturnType<typeof setInterval> | undefined;
let tickT: ReturnType<typeof setInterval> | undefined;

async function pollPending() {
  try { cn.pending = await backend.pairPending(); } catch { cn.pending = []; }
}

export function openWizard() {
  if (!app.config.remoteEnabled) { toast('Activa antes «Permitir conexiones»', 3200); return; }
  cn.wizard = 'wait';
  cn.ticket = null;
  cn.ticketError = '';
  cn.now = Date.now();
  void pollPending();
  clearInterval(pollT); clearInterval(tickT);
  pollT = setInterval(() => { void pollPending(); }, 1500);
  tickT = setInterval(() => { cn.now = Date.now(); }, 500);
  blip(1320);
}

export function closeWizard() {
  clearInterval(pollT); clearInterval(tickT);
  pollT = tickT = undefined;
  if (cn.ticket) void backend.pairTicketCancel().catch(() => {});
  cn.wizard = '';
  cn.ticket = null;
  cn.pending = [];
  void refreshConnections();
}

/** Billete nuevo (sustituye al anterior); se pide al entrar en «Mostrar QR» y con «Billete nuevo». */
export async function newTicket() {
  cn.wizard = 'qr';
  cn.ticketError = '';
  try {
    cn.ticket = await backend.pairTicketNew();
    cn.now = Date.now();
    blip(1320);
  } catch (e) {
    cn.ticket = null;
    cn.ticketError = errText(e);
  }
}

/** Vuelve a «esperando solicitudes» anulando el billete. */
export function backToWait() {
  if (cn.ticket) void backend.pairTicketCancel().catch(() => {});
  cn.ticket = null;
  cn.ticketError = '';
  cn.wizard = 'wait';
  blip(520);
}

/** Mando y teclado mientras el asistente está abierto. Devuelve true si se ha llevado la orden. */
export function wizardCmd(c: string): boolean {
  if (!cn.wizard) return false;
  if (cn.wizard === 'wait') {
    if (c === 'a') void newTicket();
    else if (c === 'b' || c === 'menu') { closeWizard(); blip(520); }
  } else {
    if (c === 'y') void newTicket();
    else if (c === 'a') backToWait();
    else if (c === 'b' || c === 'menu') backToWait();
  }
  return true;
}
