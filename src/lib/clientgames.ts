// PC «cliente» (multi-PC, fase 2): juegos instalados en este equipo. La biblioteca viene del PC principal; lo que está
// instalado aquí se guarda con el id del título del host, así que casan por `Title.id`. Un título instalado aquí se
// juega aquí (las partidas se suben solas al host); el resto se juega por streaming desde el PC principal.
import { clientMode, invoke, ready } from './ipc';
import type { Library, Title } from './types';

export interface LocalGame { id: string; name: string; platform: string; kind: string; exe: string; rom: string }
export interface Candidate { id: string; name: string; reason: string; score: number }
export interface LocalAdd {
  /** Título del host elegido; null = que el host lo cree en `platform` (nunca se crean categorías). */
  gameId: string | null;
  platform: string;
  name: string;
  kind: 'exe' | 'emulator';
  exe: string;
  args: string;
  rom: string;
  workdir?: string;
}

let local = new Map<string, LocalGame>();

export const isClient = () => !!clientMode;
export const installedHere = (t: Pick<Title, 'id'>) => !!clientMode && local.has(t.id);

export async function refreshLocal(): Promise<void> {
  await ready;
  if (!clientMode) return;
  try {
    const list = await invoke<LocalGame[]>('client_local_games');
    local = new Map(list.map((g) => [g.id, g]));
  } catch (e) {
    console.warn('client_local_games', e);
  }
}

/** Marca como jugables los títulos instalados en este equipo (aunque en el host solo sean «capturas»). */
export async function markLocal(lib: Library | null): Promise<Library | null> {
  if (!lib || !clientMode) return lib;
  await refreshLocal();
  for (const p of lib.platforms) for (const t of p.titles) {
    if (!local.has(t.id)) continue;
    t.launchable = true;
    t.removable = true;
    if (!t.gameId) t.gameId = t.id;
  }
  return lib;
}

const base = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? '';
const parent = (p: string) => { const s = p.split(/[\\/]/).filter(Boolean); return s.length > 1 ? s[s.length - 2] : ''; };

/** Títulos del host (de esa categoría) que pueden ser el juego que se añade. */
export function matchTitle(platform: string, name: string, main: string): Promise<Candidate[]> {
  return invoke<Candidate[]>('match_title', { platform, name, exeName: base(main), folderName: parent(main) });
}

export async function addLocal(game: LocalAdd): Promise<string> {
  const id = await invoke<string>('client_add_game', { game: { workdir: '', ...game } });
  await refreshLocal();
  return id;
}

export async function removeLocal(id: string): Promise<void> {
  await invoke('client_remove_game', { id });
  local.delete(id);
}

export const launchLocal = (id: string) => invoke('client_launch', { id });
export const syncNow = () => invoke<number>('client_sync').catch(() => 0);
