// Las sagas son una categoría virtual («Sagas») que se añade a la biblioteca en memoria: cada saga es un
// título cuyas capturas son la unión de las de sus juegos. No se escribe nada en las carpetas de capturas.
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

/** Prefijo común de los nombres (sin números ni signos finales): sugerencia de nombre de saga. */
export function suggestName(names: string[]): string {
  if (names.length < 2) return '';
  let p = names[0];
  for (const n of names.slice(1)) {
    let i = 0;
    while (i < p.length && i < n.length && p[i].toLowerCase() === n[i].toLowerCase()) i++;
    p = p.slice(0, i);
  }
  p = p.replace(/[\s:\-–—_.,0-9IVXivx]+$/, '').trim();
  return p.length >= 3 ? p : '';
}
