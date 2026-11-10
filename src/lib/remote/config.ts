// Dónde está el servidor al que habla la UI de escritorio cuando corre fuera de Tauri (tablet hoy, nube mañana).
// La dirección base se pide siempre a `endpoint()`: para un servidor en la nube basta con llamar a `setEndpointProvider`.
// El servidor guardado es el mismo que usa la pantalla de conexión de la app móvil (clave de localStorage compartida).

export interface ServerCfg { host: string; port: number; code: string; name?: string; /** Token del dispositivo (sin normalizar). */ token?: string }
export interface Endpoint { base: string; code: string; name: string }

const LS_SERVER = 'visor.m.server';

/** Código de emparejamiento normalizado: sin espacios ni guiones y en mayúsculas. */
export const normCode = (c: string) => c.replace(/[\s-]/g, '').toUpperCase();

/** Servidor guardado por la pantalla de conexión; null si no hay (el modo demo no sirve para la UI de escritorio). */
export function loadServer(): ServerCfg | null {
  try {
    const s = window.localStorage.getItem(LS_SERVER);
    if (!s || s === '"demo"') return null;
    const c = JSON.parse(s) as ServerCfg;
    return c && c.host && c.port && c.code ? c : null;
  } catch { return null; }
}

/** Olvida el servidor (Desconectar). */
export function clearServer() {
  try { window.localStorage.removeItem(LS_SERVER); } catch { /* sin almacenamiento */ }
}

let provider: () => Endpoint | null = () => {
  const s = loadServer();
  return s ? { base: `http://${s.host}:${s.port}`, code: s.token || normCode(s.code), name: s.name || s.host } : null;
};
/** Cambia de dónde sale la dirección (p. ej. un servidor en la nube con otro esquema de acceso). */
export function setEndpointProvider(fn: () => Endpoint | null) { provider = fn; }
/** Servidor actual o null si todavía no hay ninguno. */
export const endpoint = () => provider();
