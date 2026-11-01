// Sustituto de `@tauri-apps/api/event`: los eventos del núcleo llegan del servidor por long-poll.
// Un único bucle atiende a todos los oyentes; también sirve de latido para saber si hay conexión.
import { endpoint } from './config';
import { markOffline, markOnline } from './status.svelte';

export interface Event<T> { event: string; id: number; payload: T }
export type EventCallback<T> = (event: Event<T>) => void;
export type UnlistenFn = () => void;

const listeners = new Map<string, Set<EventCallback<any>>>();
let nextId = 1;

function dispatch(name: string, payload: unknown) {
  const set = listeners.get(name);
  if (!set) return;
  for (const cb of [...set]) {
    try { cb({ event: name, id: nextId++, payload }); } catch (e) { console.error(e); }
  }
}

export async function listen<T>(name: string, cb: EventCallback<T>): Promise<UnlistenFn> {
  let set = listeners.get(name);
  if (!set) listeners.set(name, (set = new Set()));
  set.add(cb);
  startEventLoop();
  return () => { listeners.get(name)?.delete(cb); };
}

export async function once<T>(name: string, cb: EventCallback<T>): Promise<UnlistenFn> {
  let un: UnlistenFn = () => {};
  un = await listen<T>(name, (e) => { un(); cb(e); });
  return un;
}

/** Evento local (solo en esta pantalla). */
export async function emit(name: string, payload?: unknown): Promise<void> { dispatch(name, payload); }

/* ---------- bucle de long-poll ---------- */
let running = false;
let seq = 0;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
/** Otros módulos (el outbox) se enteran de que el servidor ha vuelto. */
const upHooks = new Set<() => void>();
export const onServerUp = (fn: () => void) => { upHooks.add(fn); };

export function startEventLoop() {
  if (running) return;
  running = true;
  void loop();
}

async function loop() {
  let wasUp = true;
  for (;;) {
    const ep = endpoint();
    if (!ep) { await sleep(2000); continue; }
    const ctl = new AbortController();
    const kill = setTimeout(() => ctl.abort(), seq ? 40000 : 6000);
    try {
      const r = await fetch(`${ep.base}/api/v1/events?since=${seq}&wait=${seq ? 25 : 0}`, {
        headers: { Authorization: `Bearer ${ep.code}` }, signal: ctl.signal, cache: 'no-store',
      });
      if (!r.ok) throw new Error(String(r.status));
      const j = (await r.json()) as { seq: number; events?: Array<{ seq: number; name: string; payload: unknown }> };
      markOnline();
      if (!wasUp) { wasUp = true; upHooks.forEach((f) => f()); }
      // Si el servidor se reinició su contador vuelve atrás: se empieza de nuevo desde el actual.
      if (j.seq < seq) seq = j.seq;
      else for (const ev of j.events ?? []) { if (ev.seq > seq) dispatch(ev.name, ev.payload); }
      seq = Math.max(seq, j.seq);
    } catch {
      markOffline();
      wasUp = false;
      await sleep(4000);
    } finally {
      clearTimeout(kill);
    }
  }
}
