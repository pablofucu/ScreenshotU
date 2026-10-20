// Persistencia local del móvil: el servidor elegido y sus preferencias en localStorage, y la última
// instantánea en IndexedDB (puede ocupar varios MB) para abrir al instante y funcionar sin conexión.
import type { ServerCfg, Snapshot } from './source';

const LS_SERVER = 'visor.m.server';
const LS_PREFS = 'visor.m.prefs';

export interface Prefs {
  /** Capturas en cuadrícula continua o agrupadas por días. */
  shotsView: 'days' | 'grid';
  /** null = el que use el PC. */
  accentMode: 'art' | 'rainbow' | 'fixed' | null;
  accentFixed: number;
  /** Visor: cargar el original (más pesado) en vez de la versión reducida. */
  originals: boolean;
}
export const DEFAULT_PREFS: Prefs = { shotsView: 'days', accentMode: null, accentFixed: 4, originals: false };

const ls = () => { try { return window.localStorage; } catch { return null; } };

export function loadServer(): ServerCfg | 'demo' | null {
  try {
    const s = ls()?.getItem(LS_SERVER);
    if (!s) return null;
    return s === '"demo"' ? 'demo' : (JSON.parse(s) as ServerCfg);
  } catch { return null; }
}
export function saveServer(s: ServerCfg | 'demo' | null) {
  try { if (s === null) ls()?.removeItem(LS_SERVER); else ls()?.setItem(LS_SERVER, JSON.stringify(s)); } catch { /* sin almacenamiento */ }
}
export function loadPrefs(): Prefs {
  try { return { ...DEFAULT_PREFS, ...JSON.parse(ls()?.getItem(LS_PREFS) ?? '{}') }; } catch { return { ...DEFAULT_PREFS }; }
}
export function savePrefs(p: Prefs) {
  try { ls()?.setItem(LS_PREFS, JSON.stringify(p)); } catch { /* sin almacenamiento */ }
}

/* ---------- IndexedDB ---------- */
interface Cached { key: string; snap: Snapshot; etag: string; savedAt: number }
const DB = 'visor-movil', STORE = 'snap';

function open(): Promise<IDBDatabase> {
  return new Promise((ok, ko) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'key' });
    r.onsuccess = () => ok(r.result);
    r.onerror = () => ko(r.error);
  });
}

/** Clave por servidor: no mezclar la biblioteca de un PC con la de otro. */
export const serverKey = (s: ServerCfg) => `${s.host}:${s.port}`;

export async function readSnap(key: string): Promise<Cached | null> {
  try {
    const db = await open();
    return await new Promise((ok) => {
      const r = db.transaction(STORE).objectStore(STORE).get(key);
      r.onsuccess = () => ok((r.result as Cached) ?? null);
      r.onerror = () => ok(null);
    });
  } catch { return null; }
}
export async function writeSnap(key: string, snap: Snapshot, etag: string) {
  try {
    const db = await open();
    await new Promise<void>((ok) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put({ key, snap, etag, savedAt: Date.now() } satisfies Cached);
      tx.oncomplete = () => ok();
      tx.onerror = () => ok();
    });
  } catch { /* sin almacenamiento: solo se pierde el arranque instantáneo */ }
}
export async function clearSnaps() {
  try {
    const db = await open();
    db.transaction(STORE, 'readwrite').objectStore(STORE).clear();
  } catch { /* nada que borrar */ }
}
