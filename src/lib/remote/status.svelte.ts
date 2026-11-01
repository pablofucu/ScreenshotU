// Estado de la conexión con el servidor, reactivo para la UI (pastilla «Sin conexión», Ajustes).
const LS_SYNC = 'visor.t.lastSync';

function loadSync(): number | null {
  try { const n = Number(window.localStorage.getItem(LS_SYNC)); return n > 0 ? n : null; } catch { return null; }
}

export const remoteStatus = $state({
  /** false en cuanto una petición no llega al servidor; vuelve a true con la primera que sí. */
  online: true,
  /** Última vez (ms) que se leyeron datos del servidor. */
  lastSync: loadSync() as number | null,
  /** Cambios hechos sin conexión que esperan para enviarse. */
  pending: 0,
});

/** Ha habido contacto con el servidor y no se ha perdido desde entonces. */
let live = false;
export const isLive = () => live;

export function markOnline() {
  live = true;
  remoteStatus.online = true;
  remoteStatus.lastSync = Date.now();
  try { window.localStorage.setItem(LS_SYNC, String(remoteStatus.lastSync)); } catch { /* sin almacenamiento */ }
}
export function markOffline() { live = false; remoteStatus.online = false; }

/** «HH:MM» de la última sincronización (vacío si nunca). */
export function lastSyncLabel(): string {
  const t = remoteStatus.lastSync;
  if (!t) return '';
  const d = new Date(t);
  const hm = d.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' });
  return d.toDateString() === new Date().toDateString() ? hm : `${d.toLocaleDateString('es-ES', { day: 'numeric', month: 'short' })} ${hm}`;
}
