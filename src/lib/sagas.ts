// Antigua categoría virtual «Sagas». El escritorio ya no la usa (ahora hay colecciones dentro de cada categoría, ver
// collections.ts); se conserva solo porque la app del móvil la importa como `@shared/sagas`.
import type { Library, Platform, Saga, Shot, Title } from './types';

export const SAGA_PLATFORM = 'sagas';

const EMPTY: Omit<Title, 'name' | 'cover' | 'shots' | 'id' | 'members'> = {
  hero: null, logo: null, folder: '', dir: '', hasFolder: false, gameId: null, source: null, launchable: false,
  removable: false, lastPlayed: null, playSeconds: 0, launcher: null, sessions: 0, firstPlayed: null,
  week: [0, 0, 0, 0, 0, 0, 0], plays: [], hidden: false, accent: null, accentOverride: null, sortMode: null,
};

export const isSagaPlatform = (p: Platform | undefined) => p?.id === SAGA_PLATFORM;

/** Quita la categoría de sagas (si la hay) y devuelve las demás plataformas. */
export const realPlatforms = (lib: Library) => lib.platforms.filter((p) => p.id !== SAGA_PLATFORM);

/** Biblioteca con la categoría «Sagas» al final. */
export function withSagas(lib: Library, sagas: Saga[]): Library {
  const real = realPlatforms(lib);
  const byId = new Map<string, Title>();
  for (const p of real) for (const t of p.titles) if (!t.hidden) byId.set(t.id, t);
  const titles: Title[] = sagas.map((s) => {
    const members = s.members.map((id) => byId.get(id)).filter((t): t is Title => !!t);
    const shots: Shot[] = members
      .flatMap((t) => t.shots.map((x) => ({ ...x, game: t.name, gid: t.id })))
      .sort((a, b) => b.date - a.date);
    const covers = members.map((t) => t.cover ?? t.shots[0]?.path ?? null);
    return {
      ...EMPTY, id: `saga:${s.id}`, name: s.name, cover: covers.find(Boolean) ?? null, shots,
      members: members.map((t, i) => ({ id: t.id, name: t.name, cover: covers[i] })),
    };
  });
  const plat: Platform = { id: SAGA_PLATFORM, name: 'Sagas', folder: '', accent: 3, icon: 'saga', titles };
  return { ...lib, platforms: [...real, plat] };
}

export { suggestName } from './collections';
