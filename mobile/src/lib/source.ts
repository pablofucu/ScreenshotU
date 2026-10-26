// De dónde saca la app los datos: el visor del PC (API HTTP de `docs/api-movil.md`) o una biblioteca de
// demostración. Mañana un servidor en la nube hablará la misma API: solo cambia la dirección.
import type { Library, Saga } from '@shared/types';

export interface RemoteConfig {
  goalVn: number;
  goalJvn: number;
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

export interface ServerCfg { host: string; port: number; code: string; name?: string }
export const DEFAULT_PORT = 47811;

export class AuthError extends Error {}
export class NetError extends Error {}

/** «192.168.1.20», «192.168.1.20:47811» o «http://pc.local:47811/» → host y puerto. */
export function parseAddress(input: string): { host: string; port: number } | null {
  let s = input.trim().replace(/^https?:\/\//i, '').replace(/\/.*$/, '');
  if (!s) return null;
  let port = DEFAULT_PORT;
  const m = s.match(/^(.*):(\d{1,5})$/);
  if (m && !s.startsWith('[')) { s = m[1]; port = Number(m[2]); }
  if (!/^[\w.-]+$/.test(s) || port < 1 || port > 65535) return null;
  return { host: s, port };
}
export const normCode = (c: string) => c.replace(/[\s-]/g, '').toUpperCase();
export const fmtCode = (c: string) => { const n = normCode(c); return n.length === 8 ? `${n.slice(0, 4)}-${n.slice(4)}` : n; };

const base = (s: ServerCfg) => `http://${s.host}:${s.port}`;

async function get(url: string, init: RequestInit, ms: number): Promise<Response> {
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), ms);
  try {
    return await fetch(url, { ...init, signal: ctl.signal, cache: 'no-store' });
  } catch (e) {
    throw new NetError(ctl.signal.aborted ? 'El PC no responde' : 'No se puede conectar con el PC');
  } finally {
    clearTimeout(t);
  }
}

/** Comprueba que en esa dirección hay un visor (sin código). */
export async function hello(host: string, port: number): Promise<{ name: string; api: number }> {
  const r = await get(`http://${host}:${port}/api/v1/hello`, {}, 5000);
  if (!r.ok) throw new NetError(`Respuesta inesperada (${r.status})`);
  const j = await r.json().catch(() => null);
  if (!j || j.app !== 'visor-capturas') throw new NetError('En esa dirección no hay un visor de capturas');
  if (j.api !== 1) throw new Error('El servidor usa una versión de API no compatible');
  return { name: String(j.name ?? 'PC'), api: Number(j.api ?? 1) };
}

export class RemoteSource implements Source {
  readonly kind = 'remote';
  private etag = '';
  constructor(readonly server: ServerCfg) {}
  get label() { return this.server.name || this.server.host; }
  private q(path: string) { return `p=${encodeURIComponent(path)}&k=${encodeURIComponent(normCode(this.server.code))}`; }

  async snapshot(fresh = false): Promise<Snapshot | null> {
    const headers: Record<string, string> = { Authorization: `Bearer ${normCode(this.server.code)}` };
    if (this.etag && !fresh) headers['If-None-Match'] = this.etag;
    const r = await get(`${base(this.server)}/api/v1/snapshot${fresh ? '?fresh=1' : ''}`, { headers }, 60000);
    if (r.status === 304) return null;
    if (r.status === 401) throw new AuthError('El código no es correcto (míralo en Ajustes del visor del PC)');
    if (r.status === 429) throw new AuthError('Demasiados intentos con un código incorrecto; espera un minuto');
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
        config: { goalVn: cfg.goalVn, goalJvn: cfg.goalJvn, accentMode: cfg.accentMode, accentFixed: cfg.accentFixed },
        library, favorites, sagas, playing: null,
      };
    },
    thumb: (p, s) => b.thumb(p, s),
    preview: async (p) => b.fullUrl(p),
    file: async (p) => b.artUrl(p),
  };
}
