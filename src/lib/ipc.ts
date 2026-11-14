// Puerta única de la interfaz hacia el núcleo. En el PC principal (y en la tablet, donde `@tauri-apps/api/*` ya son
// los sustitutos de `remote/`) es Tauri tal cual. En un PC «cliente» (emparejado desde Ajustes → Este equipo), lo
// propio de este equipo (ventana, atajos, menú rápido, `client_*`) va a Tauri y todo lo demás al PC principal, por
// la misma vía que usa la tablet (`remote/core.ts`).
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';

type Invoke = <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
interface Mod {
  invoke: Invoke;
  convertFileSrc: (path: string, protocol?: string) => string;
  listen: <T>(name: string, cb: EventCallback<T>) => Promise<UnlistenFn>;
}

/** Siempre en este equipo, también en modo cliente. */
const LOCAL = new Set([
  'window_monitor', 'next_monitor', 'exit_app', 'close_main', 'hotkey_suspend', 'learn_key', 'learn_pad',
  'open_folder', 'reveal_in_explorer', 'startup_notices',
]);
const isLocal = (cmd: string) => LOCAL.has(cmd) || cmd.startsWith('client_') || cmd.startsWith('quickmenu_');

interface Endpoint { base: string; token: string; hostId: string }
export interface ClientMode { host: string; hostName: string }

let local: Mod | null = null;
let remote: Mod | null = null;
/** Datos del PC principal cuando este equipo es cliente (null = este equipo tiene la biblioteca). */
export let clientMode: ClientMode | null = null;

async function detect() {
  const [core, ev] = await Promise.all([import('@tauri-apps/api/core'), import('@tauri-apps/api/event')]);
  local = { invoke: core.invoke, convertFileSrc: core.convertFileSrc, listen: ev.listen };
  // Fuera de Tauri (tablet, navegador) no hay modo cliente: esos módulos ya son los de `remote/` o no se usan.
  if (!('__TAURI_INTERNALS__' in window)) return;
  let ep: Endpoint | null = null;
  let info: { hostName?: string } = {};
  try {
    ep = await core.invoke<Endpoint | null>('client_endpoint');
    if (ep) info = await core.invoke<{ hostName: string }>('client_info');
  } catch (e) { console.warn('client_endpoint', e); }
  if (!ep) return;
  // Con varias direcciones del PC principal (casa y Tailscale) puede cambiar la que contesta.
  setInterval(() => {
    core.invoke<Endpoint | null>('client_endpoint').then((e) => { if (e) ep = e; }).catch(() => {});
  }, 60_000);
  const [{ setEndpointProvider }, rc, rev, sc, ctrl, store] = await Promise.all([
    import('./remote/config'), import('./remote/core'), import('./remote/event'),
    import('./remote/session_close'), import('./controller.svelte'), import('./store.svelte'),
  ]);
  const name = info.hostName || ep.base.replace(/^https?:\/\//, '');
  setEndpointProvider(() => ({ base: ep!.base, code: ep!.token, name }));
  rc.setClientPc(name);
  let streamApp = '';
  let localPlayingId: string | null = null;
  // «Jugar» desde aquí: el PC principal lanza el juego y este equipo abre Moonlight con el stream.
  rc.setAfterLaunch(async (_args, res) => {
    const nextApp = (res as { app?: string | null } | null)?.app || '';
    streamApp = nextApp;
    try {
      await core.invoke('client_open_stream', { app: nextApp });
    } catch (e) {
      streamApp = '';
      void rc.invoke('stream_cancel').catch(() => {});
      throw e;
    }
  });
  remote = { invoke: rc.invoke, convertFileSrc: rc.convertFileSrc, listen: rev.listen };
  clientMode = { host: ep.base, hostName: name };
  const syncRemoteSession = async () => {
    try {
      const r = await fetch(`${ep!.base}/api/v1/session`, { headers: { Authorization: `Bearer ${ep!.token}` } });
      if (!r.ok) return;
      const u = sc.remotePlayingFromSession(await r.json());
      if (u.app !== null) streamApp = u.app;
      if (!localPlayingId) {
        if (u.active) {
          store.app.playing = { id: u.id, since: u.since };
        } else if (store.app.playing) {
          store.app.playing = null;
        }
      }
    } catch { /* sin conexión */ }
  };
  void local!.listen<{ id?: string }>('game-started', (ev) => { localPlayingId = ev.payload?.id || '__local__'; });
  void local!.listen('game-exited', () => { localPlayingId = null; });
  void rev.listen('game-exited', () => { streamApp = ''; });
  ctrl.setResumeHandler({
    can: () => !localPlayingId,
    resume: async () => { await core.invoke('client_open_stream', { app: streamApp }); },
    close: async (force) => sc.parseCloseStatus(await rc.invoke('close_game', { force })),
  });
  void syncRemoteSession();
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') void syncRemoteSession(); });
  }
}

/** Se resuelve cuando ya se sabe si este equipo es cliente (hay que esperarlo antes de usar `convertFileSrc`). */
export const ready: Promise<void> = (typeof window === 'undefined' ? Promise.resolve() : detect()).catch((e) => {
  console.error('ipc', e);
});

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await ready;
  return (remote && !isLocal(cmd) ? remote : local!).invoke<T>(cmd, args);
}

/** URL de un archivo (en modo cliente, del PC principal). Usar tras `await ready`. */
export function convertFileSrc(path: string, protocol?: string): string {
  return (remote ?? local)?.convertFileSrc(path, protocol) ?? path;
}

/** En modo cliente escucha los eventos de los dos lados (los del juego vienen del PC principal). */
export async function listen<T>(name: string, cb: EventCallback<T>): Promise<UnlistenFn> {
  await ready;
  const offs = await Promise.all([local!.listen<T>(name, cb), ...(remote ? [remote.listen<T>(name, cb)] : [])]);
  return () => { for (const off of offs) off(); };
}
