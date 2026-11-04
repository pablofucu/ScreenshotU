// Ajustes de la sección «Móvil»: servidor en la red local para la app Visor de Android (docs/api-movil.md).
import { app } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { patchConfig, toast } from './controller.svelte';
import type { RemoteInfo, StreamAppStatus } from './types';

export type RemoteId = 'remoteon' | 'remoteqr' | 'remoteaddr' | 'remotecode' | 'remotewin' | 'remotecursor' | 'remotestream' | 'remotehelp';
export const REMOTE_ROWS: RemoteId[] = ['remoteon', 'remoteqr', 'remoteaddr', 'remotecode', 'remotewin', 'remotecursor', 'remotestream', 'remotehelp'];
export const isRemoteId = (id: string): id is RemoteId => (REMOTE_ROWS as string[]).includes(id);
export const REMOTE_CYCLE = new Set<RemoteId>(['remoteon', 'remotewin', 'remotecursor']);

const HELP = 'Abre la app Visor en el móvil (misma red Wi-Fi) y escribe esta dirección y el código. La primera vez Windows puede pedir permiso en el cortafuegos: permite redes privadas.';

/** Último estado que ha dado el núcleo (se pide al abrir Ajustes y tras cada cambio). */
const rm = $state({ info: null as RemoteInfo | null, busy: false, qrOpen: false, apollo: null as StreamAppStatus | null, apolloBusy: false });
/** Para la pantalla de Ajustes: el QR de emparejamiento abierto y el último estado del servidor. */
export const remoteView = rm;

const errText = (e: unknown) => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Error desconocido');

function setInfo(i: RemoteInfo) {
  rm.info = i;
  // El código lo genera el núcleo: se copia para que la configuración en memoria no se quede atrás.
  if (i.code !== app.config.remoteToken) app.config.remoteToken = i.code;
}

export async function refreshRemote() {
  try { setInfo(await backend.remoteInfo()); } catch { rm.info = null; }
  try { rm.apollo = await backend.streamAppStatus(); } catch { rm.apollo = null; }
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
    case 'remoteqr':
      return { l: 'Conectar la tablet con QR', v: rm.qrOpen ? 'Abierto' : 'Mostrar', d: 'Pulsa A para ver un código QR. En la tablet, abre el Visor y elige «Escanear QR»: se rellenan la dirección y el código solos. El PC queda guardado por su identificador, así que si cambia de IP la tablet lo vuelve a encontrar sin preguntar.' };
    case 'remotewin':
      return { l: 'Arrancar con Windows', v: app.config.startWithWindows ? 'Sí' : 'No', d: 'Abre el visor en segundo plano al iniciar sesión en Windows (solo el icono junto al reloj, sin ventana) para que la tablet conecte sin abrir nada. Con esto activado, cerrar el visor lo deja en la bandeja; «Salir» del icono lo cierra del todo.' };
    case 'remotecursor':
      return { l: 'Ocultar el cursor en el streaming', v: app.config.streamHideCursor ? 'Sí' : 'No', d: 'Mientras juegas desde la tablet, el cursor del PC se vuelve transparente para que no salga sobre el juego. Desactívalo si usas el modo trackpad de Artemis, donde el puntero tiene que verse.' };
    case 'remotestream': {
      const a = rm.apollo;
      const v = rm.apolloBusy ? 'Creando…' : !a ? '…' : !a.found ? 'Apollo no encontrado' : !a.installed ? 'Crear' : a.outdated ? 'Actualizar' : 'Creada';
      return { l: 'Juego actual en Apollo', v, d: 'Crea en Apollo una app «Juego actual» (hace falta aceptar el permiso de administrador y reinicia su servicio). Con ella, el juego se abre en el PC al conectar la tablet y la sesión de streaming termina sola al cerrarse el juego. Sin ella funciona con «Desktop», pero la sesión no se cierra sola.' };
    }
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
  if (id !== 'remoteqr') rm.qrOpen = false;
  if (id === 'remoteqr') {
    if (!rm.info?.enabled) { toast('Activa antes el acceso desde el móvil', 3200); return; }
    if (!rm.info.qr) { toast('El servidor no está en marcha o no se encontró la IP del PC', 3600); return; }
    rm.qrOpen = !rm.qrOpen;
    blip(1320);
  } else if (id === 'remotewin') {
    void patchConfig({ startWithWindows: !app.config.startWithWindows });
    blip(1320);
  } else if (id === 'remotestream') {
    if (rm.apolloBusy) return;
    if (!rm.apollo?.found) { toast('No se encontró Apollo ni Sunshine en este PC', 3600); return; }
    rm.apolloBusy = true;
    backend.streamAppInstall()
      .then((s) => { rm.apollo = s; toast('«Juego actual» creada en Apollo', 3200); })
      .catch((e) => toast(errText(e), 4200))
      .finally(() => { rm.apolloBusy = false; });
    blip(1320);
  } else if (id === 'remotecursor') {
    void patchConfig({ streamHideCursor: !app.config.streamHideCursor });
    blip(1320);
  } else if (id === 'remoteon') {
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
