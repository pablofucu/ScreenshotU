import type { AddReport, AudioOutput, Backend, BackupResult, GameStatus, RestorePreview, StartupNotice, BackendEvent, Config, DirScan, HiddenEntry, ImportReport, Library, MagpieMode, MagpieStatus, MoveReport, RemoteInfo, NewGame, Saga, SortMode } from './types';

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

async function tauriBackend(): Promise<Backend> {
  const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const { getCurrentWebview } = await import('@tauri-apps/api/webview');
  const q = new Queue(6);
  const cache = new Map<string, string>();
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
    setTitleStatus: (id: string, status: GameStatus) => invoke('set_title_status', { id, status }),
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
    getSagas: () => invoke<Saga[]>('get_sagas'),
    saveSagas: (sagas: Saga[]) => invoke<Saga[]>('save_sagas', { sagas }),
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
  backend = await createBackend();
  return backend;
}
