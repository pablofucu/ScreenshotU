// Acceso al backend: Tauri real o, fuera de Tauri, el simulado de mock.ts (modo demo).
export const IN_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface HookApi {
  call<T = void>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  on<T>(event: string, cb: (payload: T) => void): Promise<() => void>;
}

async function tauriApi(): Promise<HookApi> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  return {
    call: <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args),
    on: async <T>(event: string, cb: (p: T) => void) => listen<T>(event, e => cb(e.payload)),
  };
}

export async function getApi(): Promise<HookApi> {
  if (IN_TAURI) return tauriApi();
  const { mockApi } = await import('./mock');
  return mockApi();
}
