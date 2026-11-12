// Lógica pura del emparejamiento (sin red ni Capacitor, para poder probarla con `node --test`).

/** Revisión de API que habla esta app; el PC rechaza con 426 las menores que su `apiMin`. */
export const APP_API = 3;
export const DEFAULT_PORT = 47811;

export interface ServerCfg {
  host: string;
  port: number;
  /** Token propio de este dispositivo (lo da el PC al aprobar la solicitud o al leer el QR). */
  token: string;
  name?: string;
  /** Identificador fijo del PC (para volver a encontrarlo si cambia de IP). */
  id?: string;
  /** Direcciones `ip:puerto` con las que llegar al PC (casa primero, Tailscale después). */
  addrs?: string[];
}

/** «192.168.1.20:47811» → host y puerto (sin puerto, el de siempre). */
export function splitAddr(a: string): { host: string; port: number } | null {
  const s = a.trim().replace(/^https?:\/\//i, '').replace(/\/.*$/, '');
  const m = s.match(/^(.*):(\d{1,5})$/);
  const host = m ? m[1] : s;
  const port = m ? Number(m[2]) : DEFAULT_PORT;
  if (!/^[\w.-]+$/.test(host) || !(port >= 1 && port <= 65535)) return null;
  return { host, port };
}

/** Sin repetidos y en el mismo orden que dio el PC; `first` (la que funciona ahora) pasa al principio. */
export function orderAddrs(addrs: readonly string[], first?: string): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const a of [...(first ? [first] : []), ...addrs]) {
    const k = a.trim();
    if (k && !seen.has(k)) { seen.add(k); out.push(k); }
  }
  return out;
}

/** Datos que lleva el QR del visor (v2): `{v:2, id, name, addrs[], port, ticket}`. */
export interface QrV2 { id: string; name: string; addrs: string[]; port: number; ticket: string }

export function parseQrV2(text: string): QrV2 | null {
  let j: unknown;
  try { j = JSON.parse(text.trim()); } catch { return null; }
  if (!j || typeof j !== 'object') return null;
  const o = j as Record<string, unknown>;
  if (o.v !== 2 || typeof o.ticket !== 'string' || !o.ticket || !Array.isArray(o.addrs)) return null;
  const port = Number(o.port) || DEFAULT_PORT;
  const addrs = orderAddrs(
    o.addrs.filter((a): a is string => typeof a === 'string').map((a) => (/:\d{1,5}$/.test(a.trim()) ? a : `${a.trim()}:${port}`)),
  ).filter((a) => splitAddr(a));
  if (!addrs.length) return null;
  return { id: typeof o.id === 'string' ? o.id : '', name: typeof o.name === 'string' ? o.name : '', addrs, port, ticket: o.ticket };
}

/**
 * Lo guardado en el dispositivo → `ServerCfg`. Las versiones anteriores guardaban el token en `code`: se sigue
 * leyendo para que lo ya emparejado entre sin volver a emparejar. Un código corto de 8 caracteres sin token
 * (emparejamiento antiguo por código) ya no vale: devuelve null y la app abre el asistente.
 */
export function migrateServer(raw: unknown): ServerCfg | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  const str = (v: unknown) => (typeof v === 'string' ? v : '');
  const legacy = str(o.code);
  const token = str(o.token) || (legacy.replace(/[\s-]/g, '').length > 8 ? legacy : '');
  const host = str(o.host), port = Number(o.port);
  if (!token || !host || !(port >= 1 && port <= 65535)) return null;
  const addrs = Array.isArray(o.addrs) ? orderAddrs(o.addrs.filter((a): a is string => typeof a === 'string')) : undefined;
  return { host, port, token, name: str(o.name) || undefined, id: str(o.id) || undefined, addrs: addrs?.length ? addrs : undefined };
}

/** Lo que se guarda en disco: `code` repite el token porque la interfaz de escritorio de la tablet aún lo lee así. */
export const toStored = (s: ServerCfg) => ({ ...s, code: s.token });

/** `art://<host>:<puerto>?pin=…&passphrase=…` con lo que da el PC en `/stream/pair`. */
export function artemisUrl(host: string, o: { pin: string; passphrase: string; port?: number }): string {
  return `art://${host}:${o.port || 47989}?pin=${encodeURIComponent(o.pin)}&passphrase=${encodeURIComponent(o.passphrase)}`;
}
