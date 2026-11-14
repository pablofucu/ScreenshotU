export type CloseStatus = 'none' | 'asked' | 'forced';

/** Interpreta la respuesta de cierre remoto (`none` si ya no había juego). */
export function parseCloseStatus(raw: unknown): CloseStatus {
  const c = (raw as { closed?: unknown } | null | undefined)?.closed;
  return c === 'asked' ? 'asked' : c === 'forced' ? 'forced' : 'none';
}

export type RemoteSessionSnapshot = {
  state?: string;
  game?: { id?: string } | null;
  app?: string | null;
  elapsedSeconds?: number;
};

/** Calcula el estado de partida remota a partir de `/api/v1/session`. */
export function remotePlayingFromSession(j: RemoteSessionSnapshot, now = Date.now()):
  | { active: true; id: string; since: number; app: string | null }
  | { active: false; app: string | null } {
  const app = j.state === 'idle' ? '' : (j.app || null);
  if ((j.state === 'playing' || j.state === 'starting') && j.game?.id) {
    const sec = Math.max(0, Number(j.elapsedSeconds) || 0);
    return { active: true, id: j.game.id, since: now - sec * 1000, app };
  }
  return { active: false, app };
}
