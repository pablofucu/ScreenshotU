import type { AddReport, AudioOutput, Backend, BackupResult, GameStatus, RestorePreview, StartupNotice, BackendEvent, Config, DirScan, HiddenEntry, ImportReport, Library, MagpieMode, MagpieStatus, MoveReport, TrashReport, RemoteInfo, NewGame, Collection, SortMode } from './types';

export const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Cola con concurrencia limitada para pedir miniaturas sin saturar el IPC. */
class Queue {
  private running = 0;
  private waiting: Array<() => void> = [];
  constructor(private max: number) {}
  run<T>(job: () => Promise<T>, signal?: AbortSignal): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      const start = () => {
        if (signal?.aborted) return reject(new DOMException('cancelado', 'AbortError'));
        this.running++;
        job().then(resolve, reject).finally(() => {
          this.running--;
          this.waiting.shift()?.();
        });
      };
      if (this.running < this.max) start();
      else this.waiting.push(start);
    });
  }
}

const IMG_EXT = ['png', 'jpg', 'jpeg', 'webp'];

/* Miniaturas ya resueltas (ruta → URL) y ya decodificadas: lo que ya se cargó una vez se pinta al instante, sin
 * esperar a la promesa ni pasar por el hueco vacío (al cambiar de pestaña, filtro, orden, página o tamaño). */
const thumbUrls = new Map<string, string>();
const decoded = new Set<string>();
const decoding = new Map<string, Promise<void>>();
/** URL de la miniatura si ya se conoce (síncrono). */
export const thumbNow = (path: string) => thumbUrls.get(path);
/** La URL ya se decodificó alguna vez: se puede pintar sin fundido. */
export const thumbReady = (url: string) => decoded.has(url);
/** Carga y decodifica una URL de imagen (una sola vez por URL); nunca falla. */
export function decodeImg(url: string): Promise<void> {
  if (!url || decoded.has(url)) return Promise.resolve();
  let p = decoding.get(url);
  if (!p) {
    const im = new Image();
    im.decoding = 'async';
    im.src = url;
    p = (im.decode ? im.decode() : new Promise<void>((ok) => { im.onload = im.onerror = () => ok(); }))
      .catch(() => {})
      .then(() => { decoded.add(url); decoding.delete(url); });
    decoding.set(url, p);
  }
  return p;
}

async function tauriBackend(): Promise<Backend> {
  const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const { getCurrentWebview } = await import('@tauri-apps/api/webview');
  const q = new Queue(6);
  const cache = thumbUrls;
  let artVer = 0;
  listen<{ src: string; thumb: string }>('thumb-ready', (e) => {
    cache.set(e.payload.src, convertFileSrc(e.payload.thumb));
  });
  return {
    isTauri: true,
    loadConfig: () => invoke<Config>('get_config'),
    saveConfig: (config) => invoke('save_config', { config }),
    async pickFolder(title = 'Carpeta raíz de capturas') {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({ directory: true, multiple: false, title });
      return typeof r === 'string' ? r : null;
    },
    scan: () => invoke<Library>('scan_library'),
    cachedScan: () => invoke<Library | null>('cached_library'),
    async thumb(path, signal) {
      const hit = cache.get(path);
      if (hit) return hit;
      const p = await q.run(() => invoke<string>('get_thumb', { path }), signal);
      const url = convertFileSrc(p);
      cache.set(path, url);
      return url;
    },
    fullUrl: (path) => convertFileSrc(path),
    artUrl: (path) => convertFileSrc(path) + (artVer ? `?v=${artVer}` : ''),
    bustArt() { artVer = Date.now(); },
    prefetch(paths) {
      const todo = paths.filter((p) => !cache.has(p));
      if (todo.length) void invoke('prefetch_thumbs', { paths: todo });
    },
    getFavorites: () => invoke<string[]>('get_favorites'),
    setFavorite: (path, on) => invoke('set_favorite', { path, on }),
    toggleFullscreen: async () => {
      const w = getCurrentWindow();
      await w.setFullscreen(!(await w.isFullscreen()));
    },
    isFullscreen: () => getCurrentWindow().isFullscreen(),
    windowMonitor: () => invoke<string>('window_monitor'),
    nextMonitor: (dir: number) => invoke<string>('next_monitor', { dir }),
    exit: () => invoke('exit_app'),
    learnPad: (on: boolean) => invoke('learn_pad', { on }),
    audioOutputs: () => invoke<AudioOutput[]>('audio_outputs'),
    setAudioOutput: (id: string) => invoke<AudioOutput[]>('set_audio_output', { id }),
    hotkeySuspend: (suspend: boolean) => invoke('hotkey_suspend', { suspend }),
    launch: (id) => invoke('launch_game', { id }),
    goalNotice: (id, at, text) => invoke('goal_notice', { id, at, text }),
    openFolder: (path) => invoke('open_folder', { path }),
    addGame: (game: NewGame) => invoke<AddReport>('add_game', { game }),
    scanGamesDir: (dir: string) => invoke<DirScan>('scan_games_dir', { dir }),
    setHidden: (e: HiddenEntry, hidden: boolean) => invoke('set_hidden', { id: e.id, name: e.name, platform: e.platform, hidden }),
    setAccentOverride: (id: string, color: string | null) => invoke('set_accent_override', { id, color }),
    setSortPref: (id: string, mode: SortMode) => invoke('set_sort_pref', { id, mode }),
    setMagpieMode: (id: string, mode: MagpieMode) => invoke('set_magpie_mode', { id, mode }),
    setTitleStatus: (id: string, status: GameStatus, finishedMs?: number | null, rating?: number | null) => invoke('set_title_status', { id, status, finishedMs: finishedMs ?? null, rating: rating ?? null }),
    setPendingOrder: (ids: string[]) => invoke('set_pending_order', { ids }),
    snoozeStalled: (ids: string[]) => invoke('snooze_stalled', { ids }),
    magpieStatus: () => invoke<MagpieStatus>('magpie_status'),
    remoteInfo: () => invoke<RemoteInfo>('remote_info'),
    remoteNewCode: () => invoke<RemoteInfo>('remote_new_code'),
    async pickSaveZip(defaultName: string) {
      const { save } = await import('@tauri-apps/plugin-dialog');
      return (await save({ title: 'Guardar copia de seguridad', defaultPath: defaultName, filters: [{ name: 'Copia de seguridad (.zip)', extensions: ['zip'] }] })) ?? null;
    },
    async pickZip() {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({ multiple: false, title: 'Elige la copia de seguridad', filters: [{ name: 'Copia de seguridad (.zip)', extensions: ['zip'] }] });
      return typeof r === 'string' ? r : null;
    },
    backupCreate: (dest: string, includeMedia: boolean) => invoke<BackupResult>('backup_create', { dest, includeMedia }),
    restorePrepare: (zip: string) => invoke<RestorePreview>('restore_prepare', { zip }),
    restoreCancel: () => invoke('restore_cancel'),
    restoreApply: () => invoke('restore_apply'),
    startupNotices: () => invoke<StartupNotice[]>('startup_notices'),
    getCollections: () => invoke<Collection[]>('get_collections'),
    saveCollections: (collections: Collection[]) => invoke<Collection[]>('save_collections', { collections }),
    convertCollections: (platforms: Record<string, string>) => invoke<Collection[]>('convert_collections', { platforms }),
    async pickImages() {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({ multiple: true, title: 'Elige capturas', filters: [{ name: 'Imágenes', extensions: IMG_EXT }] });
      return Array.isArray(r) ? r : typeof r === 'string' ? [r] : [];
    },
    async importShots(dir: string, paths: string[], onProgress: (done: number, total: number) => void) {
      const un = await listen<{ done: number; total: number }>('import-progress', (e) => onProgress(e.payload.done, e.payload.total));
      try { return await invoke<ImportReport>('import_shots', { dir, paths }); } finally { un(); }
    },
    moveShots: (paths: string[], targetDir: string) => invoke<MoveReport>('move_shots', { paths, targetDir }),
    trashShots: (paths: string[]) => invoke<TrashReport>('trash_shots', { paths }),
    reveal: (path: string) => invoke('reveal_in_explorer', { path }),
    onFilesDropped(cb: (paths: string[]) => void) {
      void getCurrentWebview().onDragDropEvent((e) => { if (e.payload.type === 'drop') cb(e.payload.paths); });
    },
    removeGame: (id) => invoke('remove_game', { id }),
    moveGamePlatform: (id, platform) => invoke('move_game_platform', { id, platform }),
    async pickFile(kind) {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const r = await open({
        multiple: false,
        title: kind === 'exe' ? 'Elige el ejecutable' : kind === 'image' ? 'Elige la imagen' : 'Elige la ROM o el juego',
        filters: kind === 'exe' ? [{ name: 'Ejecutables', extensions: ['exe', 'bat', 'cmd'] }]
          : kind === 'image' ? [{ name: 'Imágenes', extensions: IMG_EXT }] : undefined,
      });
      return typeof r === 'string' ? r : null;
    },
    onEvent(cb) {
      void listen<{ path: string; name: string }>('capture-saved', (e) => cb({ type: 'capture-saved', name: e.payload.name }));
      void listen<string>('capture-error', (e) => cb({ type: 'capture-error', message: String(e.payload) }));
      void listen<AudioOutput[]>('audio-outputs', (e) => cb({ type: 'audio-outputs', outputs: e.payload }));
      void listen<{ combo: string; connected: boolean; done: boolean }>('learn-pad', (e) => cb({ type: 'learn-pad', ...e.payload }));
      void listen<{ id: string; name: string }>('game-started', (e) => cb({ type: 'game-started', id: e.payload.id, name: e.payload.name }));
      void listen<{ id: string; name: string; seconds: number }>('game-exited', (e) =>
        cb({ type: 'game-exited', id: e.payload.id, name: e.payload.name, seconds: e.payload.seconds }));
      void listen<{ id: string; message: string; error: boolean }>('magpie-status', (e) => cb({ type: 'magpie-status', ...e.payload }));
    },
  };
}

export async function createBackend(): Promise<Backend> {
  if (IS_TAURI) return tauriBackend();
  const { mockBackend } = await import('./mock');
  return mockBackend();
}

export let backend: Backend;
export async function initBackend() {
  const b = await createBackend();
  // Toda miniatura resuelta queda en la caché síncrona (`thumbNow`), también en el modo navegador.
  const raw = b.thumb.bind(b);
  b.thumb = (path, signal) => {
    const hit = thumbUrls.get(path);
    if (hit) return Promise.resolve(hit);
    return raw(path, signal).then((u) => { thumbUrls.set(path, u); return u; });
  };
  backend = b;
  return backend;
}
