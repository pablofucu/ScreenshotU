// De dónde saca la app los datos: el visor del PC (API HTTP de `docs/referencia/api-movil.md`) o una biblioteca de
// demostración. Mañana un servidor en la nube hablará la misma API: solo cambia la dirección.
import type { Library, Saga } from '@shared/types';
import { APP_API, DEFAULT_PORT, orderAddrs, splitAddr, type ServerCfg } from './pairing';

export interface RemoteConfig {
  goalVn: number;
  goalJvn: number;
  goalPc: number;
  goalSwitch: number;
  accentMode: 'art' | 'rainbow' | 'fixed';
  accentFixed: number;
}
export interface Playing { id: string; name: string; since: number }
export interface Snapshot {
  api: number;
  generatedAt: number;
  name: string;
  config: RemoteConfig;
  library: Library;
  favorites: string[];
  sagas: Saga[];
  playing: Playing | null;
}

export interface Source {
  readonly kind: 'remote' | 'demo';
  /** Nombre del servidor o «Demostración». */
  readonly label: string;
  /** null si no ha cambiado desde la última vez (ETag). */
  snapshot(fresh?: boolean): Promise<Snapshot | null>;
  /** Miniatura (480 px). */
  thumb(path: string, signal?: AbortSignal): Promise<string>;
  /** Imagen para el visor (hasta 1920 px). */
  preview(path: string, signal?: AbortSignal): Promise<string>;
  /** Archivo original (grafismos con transparencia, «Ver original»). */
  file(path: string): Promise<string>;
}

export { DEFAULT_PORT, type ServerCfg };

export class AuthError extends Error {}
export class NetError extends Error {}
/** Versiones incompatibles: `who` es quién está desactualizado (el visor del PC o esta app). */
export class UpdateError extends Error {
  constructor(public who: 'pc' | 'app', msg?: string) {
    super(msg ?? (who === 'pc' ? 'Actualiza el visor del PC: es anterior al emparejamiento nuevo' : 'Actualiza la app: este PC usa el emparejamiento nuevo'));
  }
}

/** «192.168.1.20», «192.168.1.20:47811» o «http://pc.local:47811/» → host y puerto. */
export const parseAddress = splitAddr;

const base = (s: Pick<ServerCfg, 'host' | 'port'>) => `http://${s.host}:${s.port}`;
export const unpairedMsg = (name?: string) => `Este dispositivo ya no está emparejado con ${name || 'el PC'}`;

async function get(url: string, init: RequestInit, ms: number, outer?: AbortSignal): Promise<Response> {
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), ms);
  const onOuter = () => ctl.abort();
  outer?.addEventListener('abort', onOuter);
  try {
    return await fetch(url, { ...init, headers: { 'X-Visor-Api': String(APP_API), ...(init.headers as Record<string, string> | undefined) }, signal: ctl.signal, cache: 'no-store' });
  } catch (e) {
    throw new NetError(outer?.aborted ? 'Cancelado' : ctl.signal.aborted ? 'El PC no responde' : 'No se puede conectar con el PC');
  } finally {
    clearTimeout(t);
    outer?.removeEventListener('abort', onOuter);
  }
}
const errOf = async (r: Response) => ((await r.json().catch(() => null)) as { error?: string } | null)?.error ?? `Error del PC (${r.status})`;
const json = { 'Content-Type': 'application/json' };

export interface Hello { name: string; api: number; id: string; features: string[]; addrs: string[]; version: string }

/** Comprueba que en esa dirección hay un visor (sin token). */
export async function hello(host: string, port: number, ms = 5000): Promise<Hello> {
  const r = await get(`http://${host}:${port}/api/v1/hello`, {}, ms);
  if (!r.ok) throw new NetError(`Respuesta inesperada (${r.status})`);
  const j = await r.json().catch(() => null);
  if (!j || j.app !== 'visor-capturas') throw new NetError('En esa dirección no hay un visor de capturas');
  if (j.api !== 1) throw new Error('El servidor usa una versión de API no compatible');
  const strs = (v: unknown) => (Array.isArray(v) ? (v as unknown[]).filter((x): x is string => typeof x === 'string') : []);
  return { name: String(j.name ?? 'PC'), api: Number(j.api ?? 1), id: String(j.id ?? ''), features: strs(j.features), addrs: orderAddrs(strs(j.addrs)), version: String(j.version ?? '') };
}

/** `hello` y, además, comprueba que el visor sabe emparejar con el sistema nuevo (si no, UpdateError('pc')). */
export async function probe(host: string, port: number, ms = 5000): Promise<Hello> {
  const h = await hello(host, port, ms);
  if (!h.features.includes('pair2')) throw new UpdateError('pc');
  return h;
}

/** Nombre con el que sale este dispositivo en el PC (se puede cambiar allí, en Ajustes → Equipos). */
const deviceName = () => (/Mobile/i.test(navigator.userAgent) ? 'Móvil' : 'Tablet');
const identity = () => ({ name: deviceName(), kind: 'tablet', appVersion: '', api: APP_API });

export interface Paired { token: string; deviceId: string; hostId: string; hostName: string; addrs: string[] }
const toPaired = (j: Record<string, unknown>): Paired => ({
  token: String(j.token ?? ''), deviceId: String(j.deviceId ?? ''), hostId: String(j.hostId ?? ''), hostName: String(j.hostName ?? ''),
  addrs: Array.isArray(j.addrs) ? (j.addrs as unknown[]).filter((x): x is string => typeof x === 'string') : [],
});

/** Pide entrar al PC: devuelve el número de 4 cifras que se enseña y el identificador de la solicitud. */
export async function requestPair(host: string, port: number): Promise<{ requestId: string; digits: string }> {
  const r = await get(`http://${host}:${port}/api/v1/pair/request`, { method: 'POST', headers: json, body: JSON.stringify(identity()) }, 8000);
  if (r.status === 426) throw new UpdateError('app');
  if (!r.ok) throw new NetError(await errOf(r));
  const j = await r.json();
  return { requestId: String(j.requestId), digits: String(j.digits) };
}

export type WaitResult = { status: 'approved'; paired: Paired } | { status: 'rejected' | 'expired' };

/** Espera la respuesta del PC (long-poll de ~25 s, se repite mientras esté pendiente). `signal` la cancela. */
export async function waitPair(host: string, port: number, requestId: string, signal: AbortSignal): Promise<WaitResult> {
  let fails = 0;
  while (!signal.aborted) {
    try {
      const r = await get(`http://${host}:${port}/api/v1/pair/wait?id=${encodeURIComponent(requestId)}`, {}, 35000, signal);
      if (r.status === 426) throw new UpdateError('app');
      if (!r.ok) throw new NetError(await errOf(r));
      const j = await r.json();
      fails = 0;
      if (j.status === 'approved') return { status: 'approved', paired: toPaired(j) };
      if (j.status === 'rejected' || j.status === 'expired') return { status: j.status };
    } catch (e) {
      if (signal.aborted || e instanceof UpdateError || ++fails >= 3) throw e;
      await new Promise((ok) => setTimeout(ok, 1000));
    }
  }
  throw new NetError('Cancelado');
}

/** Retira la solicitud (si el PC no contesta, caduca sola). */
export async function cancelPair(host: string, port: number, requestId: string) {
  try { await get(`http://${host}:${port}/api/v1/pair/cancel?id=${encodeURIComponent(requestId)}`, { method: 'POST', headers: json, body: '{}' }, 4000); } catch { /* apagado */ }
}

/** Entra con el billete del QR (sin tener que pulsar nada en el PC). */
export async function pairWithTicket(host: string, port: number, ticket: string): Promise<Paired> {
  const r = await get(`http://${host}:${port}/api/v1/pair/ticket`, { method: 'POST', headers: json, body: JSON.stringify({ ticket, ...identity() }) }, 8000);
  if (r.status === 426) throw new UpdateError('app');
  if (r.status === 401) throw new AuthError('El billete no es válido o ha caducado. Genera otro en el PC.');
  if (!r.ok) throw new NetError(await errOf(r));
  return toPaired(await r.json());
}

/** Pide al PC un PIN de un solo uso para emparejar Artemis (el PC lo registra en Apollo). */
export async function streamPair(s: Pick<ServerCfg, 'host' | 'port' | 'token'>): Promise<{ pin: string; passphrase: string; port: number }> {
  const r = await get(`${base(s)}/api/v1/stream/pair`, { method: 'POST', headers: { ...json, Authorization: `Bearer ${s.token}` }, body: '{}' }, 15000);
  if (r.status === 401) throw new AuthError(unpairedMsg());
  if (r.status === 426) throw new UpdateError('app');
  if (!r.ok) throw new NetError(await errOf(r));
  const j = await r.json().catch(() => null);
  if (!j || typeof j.pin !== 'string' || typeof j.passphrase !== 'string') throw new NetError('El PC no ha dado el PIN del streaming');
  return { pin: j.pin, passphrase: j.passphrase, port: Number(j.port) || 47989 };
}

/** Avisa al PC de que este dispositivo se desconecta (si no contesta, se puede quitar allí). */
export async function unpairDevice(s: ServerCfg) {
  try { await get(`${base(s)}/api/v1/unpair`, { method: 'POST', headers: { Authorization: `Bearer ${s.token}` } }, 4000); } catch { /* apagado */ }
}

export class RemoteSource implements Source {
  readonly kind = 'remote';
  private etag = '';
  /** `server` cambia si el PC cambia de dirección. */
  constructor(public server: ServerCfg) {}
  get label() { return this.server.name || this.server.host; }
  private q(path: string) { return `p=${encodeURIComponent(path)}&k=${encodeURIComponent(this.server.token)}`; }

  async snapshot(fresh = false): Promise<Snapshot | null> {
    const headers: Record<string, string> = { Authorization: `Bearer ${this.server.token}` };
    if (this.etag && !fresh) headers['If-None-Match'] = this.etag;
    const r = await get(`${base(this.server)}/api/v1/snapshot${fresh ? '?fresh=1' : ''}`, { headers }, 60000);
    if (r.status === 304) return null;
    if (r.status === 401) throw new AuthError(unpairedMsg(this.server.name));
    if (r.status === 426) throw new UpdateError('app');
    if (r.status === 429) throw new NetError('Demasiados intentos fallidos; espera un minuto');
    if (!r.ok) {
      const j = await r.json().catch(() => null);
      throw new NetError(j?.error ?? `Error del PC (${r.status})`);
    }
    const snap = (await r.json()) as Snapshot;
    this.etag = r.headers.get('ETag') ?? '';
    return snap;
  }
  setEtag(e: string) { this.etag = e; }
  getEtag() { return this.etag; }
  /** Acción del PC (`launch`, `capture`): POST con el token; lanza el error con el motivo que da el visor. */
  async post(action: 'launch' | 'capture', body: object = {}) {
    const headers = { Authorization: `Bearer ${this.server.token}`, 'Content-Type': 'application/json' };
    const r = await get(`${base(this.server)}/api/v1/${action}`, { method: 'POST', headers, body: JSON.stringify(body) }, 15000);
    if (r.status === 401) throw new AuthError(unpairedMsg(this.server.name));
    if (r.status === 426) throw new UpdateError('app');
    if (!r.ok) {
      const j = await r.json().catch(() => null);
      throw new NetError(j?.error ?? `Error del PC (${r.status})`);
    }
  }
  async thumb(path: string) { return `${base(this.server)}/api/v1/thumb?${this.q(path)}`; }
  async preview(path: string) { return `${base(this.server)}/api/v1/preview?${this.q(path)}`; }
  async file(path: string) { return `${base(this.server)}/api/v1/file?${this.q(path)}`; }
}

/** Biblioteca simulada del escritorio (arte dibujado en el propio móvil). */
export async function demoSource(): Promise<Source> {
  const { mockBackend } = await import('@shared/mock');
  const b = await mockBackend();
  return {
    kind: 'demo',
    label: 'Demostración',
    async snapshot() {
      const [library, cfg, favorites] = await Promise.all([b.scan(), b.loadConfig(), b.getFavorites()]);
      const sagas: Saga[] = [];
      return {
        api: 1, generatedAt: Date.now(), name: 'Demostración',
        config: { goalVn: cfg.goalVn, goalJvn: cfg.goalJvn, goalPc: cfg.goalPc, goalSwitch: cfg.goalSwitch, accentMode: cfg.accentMode, accentFixed: cfg.accentFixed },
        library, favorites, sagas, playing: null,
      };
    },
    thumb: (p, s) => b.thumb(p, s),
    preview: async (p) => b.fullUrl(p),
    file: async (p) => b.artUrl(p),
  };
}
