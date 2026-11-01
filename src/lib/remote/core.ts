// Sustituto de `@tauri-apps/api/core` para la UI de escritorio fuera de Tauri (tablet; mañana, nube).
// `invoke` habla con `POST /api/v1/call`; las lecturas se guardan en IndexedDB para seguir funcionando sin el PC
// y los cambios simples hechos sin conexión van a un buzón (outbox) que se envía en cuanto el servidor vuelve.
import { endpoint, type Endpoint } from './config';
import { isLive, markOffline, markOnline, remoteStatus } from './status.svelte';
import { onServerUp, startEventLoop } from './event';

export const OFFLINE_MSG = 'Sin conexión con el PC';

/** Comandos de solo lectura: su último resultado se guarda y se sirve sin conexión. */
const READS = new Set([
  'get_config', 'cached_library', 'scan_library', 'get_favorites', 'get_collections', 'game_stats', 'get_thumb',
  'get_game_achievements', 'get_achievements_summary', 'get_achievements_card_variant',
  'saves_info', 'saves_status', 'readstats_get', 'phrases_list', 'phrases_due_count', 'phrases_media_dir',
  'sgdb_local', 'sgdb_info', 'magpie_status', 'audio_outputs', 'startup_notices', 'library_info',
]);
/** Sin caché propia, se responde con la de otro comando (el escaneo completo cae a la última biblioteca guardada). */
const FALLBACK: Record<string, string> = { scan_library: 'cached_library', cached_library: 'scan_library' };
/** Escrituras de estado seguras para guardar en el buzón y reenviar más tarde, en orden. */
const OUTBOX = new Set([
  'set_favorite', 'set_title_status', 'set_pending_order', 'snooze_stalled', 'save_collections', 'set_hidden',
  'set_accent_override', 'set_sort_pref', 'set_release_date', 'save_config', 'saves_set_note', 'saves_set_pinned',
  'phrases_update', 'phrases_grade',
]);

/* ---------- IndexedDB ---------- */
const DB = 'visor-tablet';
let dbp: Promise<IDBDatabase> | null = null;
function db(): Promise<IDBDatabase> {
  return (dbp ??= new Promise((ok, ko) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => {
      r.result.createObjectStore('reads', { keyPath: 'key' });
      r.result.createObjectStore('outbox', { autoIncrement: true });
    };
    r.onsuccess = () => ok(r.result);
    r.onerror = () => ko(r.error);
  }));
}
function tx<T>(store: string, mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>): Promise<T | undefined> {
  return db().then((d) => new Promise<T | undefined>((ok) => {
    try {
      const r = f(d.transaction(store, mode).objectStore(store));
      r.onsuccess = () => ok(r.result);
      r.onerror = () => ok(undefined);
    } catch { ok(undefined); }
  }), () => undefined);
}

const readKey = (ep: Endpoint, cmd: string, args: unknown) => `${ep.base}|${cmd}|${JSON.stringify(args ?? {})}`;
async function cacheGet<T>(key: string): Promise<{ v: T } | null> {
  const r = (await tx<{ key: string; v: T }>('reads', 'readonly', (s) => s.get(key))) ?? null;
  return r ? { v: r.v } : null;
}
const cachePut = (key: string, v: unknown) => tx('reads', 'readwrite', (s) => s.put({ key, v, at: Date.now() }));

interface OutItem { base: string; cmd: string; args: unknown }
const outAll = () => db().then((d) => new Promise<Array<{ k: IDBValidKey; v: OutItem }>>((ok) => {
  const out: Array<{ k: IDBValidKey; v: OutItem }> = [];
  const c = d.transaction('outbox').objectStore('outbox').openCursor();
  c.onsuccess = () => { const cur = c.result; if (cur) { out.push({ k: cur.key, v: cur.value }); cur.continue(); } else ok(out); };
  c.onerror = () => ok(out);
}), () => []);
async function refreshPending() { remoteStatus.pending = (await outAll()).length; }

/* ---------- red ---------- */
class NetFail extends Error {}

async function http(ep: Endpoint, path: string, init: RequestInit, ms: number): Promise<Response> {
  const ctl = new AbortController();
  const t = setTimeout(() => ctl.abort(), ms);
  try {
    return await fetch(`${ep.base}${path}`, { ...init, signal: ctl.signal, cache: 'no-store' });
  } catch { throw new NetFail(); } finally { clearTimeout(t); }
}

let probing: Promise<boolean> | null = null;
let probedAt = 0;
/** Comprobación corta de alcance (si el servidor ya está vivo no hace falta) (≈3,5 s): evita que cada lectura espere el tiempo de red entero estando el PC apagado. */
function probe(ep: Endpoint): Promise<boolean> {
  if (probing) return probing;
  if (Date.now() - probedAt < 4000) return Promise.resolve(false);
  probing = http(ep, '/api/v1/hello', {}, 3500).then((r) => r.ok, () => false).then((ok) => {
    probedAt = Date.now();
    if (ok) { markOnline(); onUp(); } else markOffline();
    return ok;
  }).finally(() => { probing = null; });
  return probing;
}

async function callServer(ep: Endpoint, cmd: string, args: unknown, ms: number): Promise<unknown> {
  const r = await http(ep, '/api/v1/call', {
    method: 'POST',
    headers: { Authorization: `Bearer ${ep.code}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ cmd, args: args ?? {} }),
  }, ms);
  markOnline();
  if (r.status === 401 || r.status === 429) throw 'El código no es correcto (míralo en Ajustes del visor del PC)';
  if (r.status === 403) throw 'No disponible desde la tablet';
  const j = await r.json().catch(() => null);
  if (!r.ok || (j && typeof j === 'object' && 'error' in j && (j as { error: unknown }).error)) {
    throw String((j as { error?: unknown } | null)?.error ?? `Error del PC (${r.status})`);
  }
  return j;
}

/* ---------- invoke ---------- */
let afterLaunch: ((args: unknown) => void | Promise<void>) | null = null;
/** Gancho tras `launch_game` correcto (la tablet abre Moonlight). */
export function setAfterLaunch(fn: ((args: unknown) => void | Promise<void>) | null) { afterLaunch = fn; }

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const ep = endpoint();
  if (!ep) throw 'Sin servidor configurado';
  startEventLoop();
  const read = READS.has(cmd);
  if (isLive() || (await probe(ep))) {
    try {
      const res = await callServer(ep, cmd, args, read ? 60000 : 180000);
      if (read) void cachePut(readKey(ep, cmd, args), res);
      if (cmd === 'launch_game' && afterLaunch) void Promise.resolve(afterLaunch(args)).catch((e) => console.warn(e));
      return res as T;
    } catch (e) {
      if (!(e instanceof NetFail)) throw e;
      markOffline();
    }
  }
  return offline<T>(ep, cmd, args);
}

async function offline<T>(ep: Endpoint, cmd: string, args: unknown): Promise<T> {
  if (READS.has(cmd)) {
    const hit = (await cacheGet<T>(readKey(ep, cmd, args))) ?? (FALLBACK[cmd] ? await cacheGet<T>(readKey(ep, FALLBACK[cmd], args)) : null);
    if (hit) return hit.v;
    throw OFFLINE_MSG;
  }
  if (!OUTBOX.has(cmd)) throw OFFLINE_MSG;
  await tx('outbox', 'readwrite', (s) => s.add({ base: ep.base, cmd, args } satisfies OutItem));
  void refreshPending();
  return (await patchLocal(ep, cmd, (args ?? {}) as Record<string, unknown>)) as T;
}

/** Que la interfaz siga coherente mientras el cambio espera: se retoca la copia guardada de lo que lee. */
async function patchLocal(ep: Endpoint, cmd: string, a: Record<string, unknown>): Promise<unknown> {
  if (cmd === 'set_favorite') {
    const key = readKey(ep, 'get_favorites', {});
    const cur = (await cacheGet<string[]>(key))?.v ?? [];
    const set = new Set(cur);
    if (a.on) set.add(String(a.path)); else set.delete(String(a.path));
    await cachePut(key, [...set]);
  } else if (cmd === 'save_collections') {
    await cachePut(readKey(ep, 'get_collections', {}), a.collections);
    return a.collections;
  } else if (cmd === 'save_config') {
    await cachePut(readKey(ep, 'get_config', {}), a.config);
  }
  return null;
}

/* ---------- reenvío del buzón ---------- */
let flushing = false;
async function flush() {
  if (flushing) return;
  const ep = endpoint();
  if (!ep) return;
  flushing = true;
  try {
    for (const { k, v } of await outAll()) {
      if (v.base !== ep.base) continue;
      try {
        await callServer(ep, v.cmd, v.args, 60000);
      } catch (e) {
        if (e instanceof NetFail) { markOffline(); break; }
        console.warn('Cambio descartado por el PC:', v.cmd, e); // lo rechazó el servidor: no se reintenta
      }
      await tx('outbox', 'readwrite', (s) => s.delete(k));
      await refreshPending();
    }
  } finally {
    flushing = false;
    void refreshPending();
  }
}
function onUp() { void flush(); }
onServerUp(onUp);
void refreshPending().then(() => { const ep = endpoint(); if (remoteStatus.pending && ep) void probe(ep); });

/** Tauri sirve archivos con su protocolo `asset`; aquí cada ruta del PC sale por `/api/v1/asset` (la caché HTTP del WebView los guarda). */
export function convertFileSrc(path: string, _protocol?: string): string {
  const ep = endpoint();
  return ep ? `${ep.base}/api/v1/asset?p=${encodeURIComponent(path)}&k=${encodeURIComponent(ep.code)}` : path;
}
