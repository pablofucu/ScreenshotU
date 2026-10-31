import { SvelteSet } from 'svelte/reactivity';
import { untrack } from 'svelte';
import { RAINBOW, type Accent } from './icons';
import type { AudioOutput, Config, GameStatsDb, GameStatus, Library, MagpieStatus, Collection, Shot, SortMode as SortModeT, Swatch, Title, TitleOrder } from './types';
import { hexToRgb, hueOf, inkDark, rgbToLch } from './oklch';
import { gamePair, pair } from './atmos';
import { collItems, collectionTitle } from './collections';
import { GOAL_COL, type GoalKey } from './readgoals';
import { calendar, categoryStats, evoBuckets, playedPlatforms, playYears, rankPeriod, titleStats, yearReview, type EvoKind, type GMetric, type PeriodKind } from './stats';

export type View = 'home' | 'titles' | 'detail' | 'shots' | 'viewer' | 'stats' | 'evo' | 'streak' | 'gstats';
export type SortMode = SortModeT;

export const DEFAULT_CONFIG: Config = {
  root: null,
  titlesView: 'grid',
  shotsView: 'grid',
  shotsOrder: 'recent',
  accentMode: 'art',
  accentFixed: 4,
  sound: false,
  platforms: [
    { id: 'pc', name: 'Juegos PC', folder: 'PC', accent: 4, icon: 'pc' },
    { id: 'switch', name: 'Juegos Switch', folder: 'Switch', accent: 0, icon: 'switch' },
    { id: 'vn', name: 'Novelas visuales', folder: 'Novelas visuales', accent: 6, icon: 'vn' },
    { id: 'jvn', name: 'Novelas japonesas', folder: 'Novelas japonesas', accent: 5, icon: 'jvn' },
  ],
  captureHotkey: 'F12',
  captureCombo: 'back+rb',
  captureMouse: '',
  captureMagpie: 'scaled',
  magpieExe: '',
  captureSound: true,
  capturePeek: true,
  captureOnlyGames: true,
  captureRumble: true,
  showHidden: false,
  sgdbKey: '',
  sgdbAuto: true,
  deepseekKey: '',
  deepseekModel: 'deepseek-chat',
  ankiUrl: 'http://127.0.0.1:8765',
  ankiDeck: 'Japonés::Novelas',
  ankiModel: '',
  ankiStyle: 'limpio',
  ankiAutostart: true,
  ankiExe: '',
  ankiMap: { word: 'Word', reading: 'WordReading', meaning: 'PrimaryDefinition', sentence: 'Sentence', translation: 'SentenceTranslation', shot: 'Picture', audio: 'SentenceAudio' },
  recordHotkey: 'F9',
  audioCombo: 'start+rb',
  audioHotkey: '',
  autoBackup: true,
  openOnCursor: false,
  savesAuto: true,
  savesDir: '',
  goalVn: 60,
  goalJvn: 30,
  remoteEnabled: false,
  remotePort: 47811,
  remoteToken: '',
  titlesOrder: {},
  titlesZoom: 0,
  collectionsStyle: 'aurora',
  listStalled: true,
  listStalledDays: 30,
  listStalledAction: 'ask',
  listAutoPlay: true,
  inputMouse: true,
  inputKeys: false,
};

export type MenuItem =
  | 'folder' | 'reload' | 'remove' | 'hide' | 'unhide' | 'addshots' | 'accent'
  | 'select' | 'unselect' | 'selall' | 'move' | 'reveal' | 'addcoll' | 'sgdb' | 'magpie' | 'phrases' | 'status' | 'release' | 'saves' | 'category' | 'delshot' | 'delsel' | 'setexe' | 'setdir' | `cat:${string}`
  /* Menú de Start de la lista de juegos (y «Salida de audio», también en el de capturas). */
  | 'filter' | `flt:${string}` | 'tsize' | 'audio' | 'settings' | 'gstats' | 'play' | 'listopts' | 'copy';
export type Overlay = '' | 'settings' | 'menu' | 'form' | 'add' | 'gdir' | 'hidden' | 'moveto' | 'accent' | 'collform' | 'picker' | 'addcoll' | 'sgdb' | 'sgdbtitles' | 'phrases' | 'saves' | 'library' | 'status' | 'release' | 'stalled' | 'achievements';
/** «Nueva colección» / «Editar colección». */
export interface CollFormState {
  /** null = colección nueva. */
  id: string | null;
  platform: string;
  name: string;
  nameTouched: boolean;
  /** Ids de juego elegidos, en orden. */
  members: string[];
  /** Ids que ya no están en la biblioteca: se conservan al guardar aunque no se vean. */
  keep: string[];
  /** null = color automático; si no, índice en `RAINBOW`. */
  color: number | null;
  /** Al crear, primero se puede partir de una sugerencia. */
  step: 'suggest' | 'form';
  sugg: Array<{ name: string; ids: string[] }>;
  /** Fila del panel (la lista depende de si se crea o se edita). */
  row: number;
  /** «Borrar»: pulsar A otra vez para confirmar. */
  confirm: boolean;
  dirty: boolean;
}
/** Selector de juegos de una colección: búsqueda, juegos marcados (ids) y filtro «Todos / Seleccionados». */
export interface PickerState { q: string; ids: string[]; filter: 'all' | 'sel'; at: number; start: string }
/** «Añadir a una colección»: juego (índice en la categoría) y fila elegida. */
export interface AddCollState { ti: number; row: number }
export interface MenuState {
  /** detail = ficha del juego; shots = cuadrícula o visor de capturas; titles = lista de juegos (Start). */
  kind: 'detail' | 'shots' | 'titles';
  title: number;
  row: number;
  items: MenuItem[];
  confirm: boolean;
}
/** Qué se está asignando en Ajustes «esperando el input». */
export type LearnKind = 'pad' | 'mouse' | 'key';
export interface LearnState {
  kind: LearnKind;
  /** Que atajo se asigna con la tecla: la captura de pantalla o la grabacion de audio. */
  target: 'capture' | 'record' | 'audio';
  /** Combinación del mando pulsada hasta ahora. */
  combo: string;
  /** Hay un mando XInput conectado. */
  connected: boolean;
  /** Segundos que quedan. */
  left: number;
}
export type FormKind = 'exe' | 'emu' | 'captures';
export interface FormState {
  kind: FormKind;
  exe: string;
  rom: string;
  name: string;
  args: string;
  platform: string;
  nameTouched: boolean;
  hero: string;
  logo: string;
  cover: string;
}
/** Una subcarpeta del directorio de juegos, con lo que el usuario va decidiendo en el asistente. */
export interface GameDirItem {
  folder: string;
  exes: string[];
  /** Ejecutable elegido: índice en `exes`, o -1 si se eligió otro a mano (`custom`). */
  sel: number;
  custom: string;
  name: string;
  platform: string;
}
export interface GameDirState {
  loading: boolean;
  dir: string;
  items: GameDirItem[];
  /** Carpeta que se está revisando. */
  i: number;
  row: number;
  added: number;
  known: number;
  empty: number;
}
export interface OskState {
  open: boolean;
  label: string;
  value: string;
  shift: boolean;
  /** Tecla enfocada: fila y posición en la fila. */
  r: number;
  c: number;
  hint: string;
}
export interface MoveTarget { id: string; name: string; platform: string; platformName: string; dir: string }
export interface MoveState { paths: string[]; targets: MoveTarget[]; row: number; confirm: boolean }
export interface AccentPickState {
  title: number;
  /** 0 automático · 1 paleta del arte · 2 arcoíris · 3 tono personalizado */
  row: number;
  col: number;
  hue: number;
  palette: Swatch[];
}

export interface CurTitle {
  name: string;
  cover: string | null;
  shots: Shot[];
  isFavs: boolean;
}

export const SORT_LABEL: Record<SortMode, string> = {
  recent: 'Más recientes primero',
  oldest: 'Más antiguas primero',
  favs: 'Favoritas primero',
  name: 'Por nombre',
};
export const SORT_ORDER: SortMode[] = ['recent', 'oldest', 'favs', 'name'];

/** Filtros de la pestaña «Todos» (Start › Filtrar). Los de estado viven en «Mi lista». */
export type TitleFilter = 'all' | 'launch' | 'shots' | 'favs';
export const TITLE_FILTERS: Array<[TitleFilter, string]> = [
  ['all', 'Sin filtro'], ['launch', 'Con lanzador'], ['shots', 'Solo capturas'], ['favs', 'Con favoritas'],
];
/** Un filtro guardado de una versión anterior (o desconocido) cuenta como «Sin filtro». */
export const validFilter = (f: string | null | undefined): TitleFilter => (TITLE_FILTERS.some(([k]) => k === f) ? (f as TitleFilter) : 'all');

/** Orden de la lista de juegos (botón X), en el orden en que se recorre; se recuerda por categoría. */
export const TITLE_ORDERS: TitleOrder[] = ['last', 'az', 'color'];
export const TITLE_ORDER_LABEL: Record<TitleOrder, string> = { last: 'Última partida', az: 'A–Z', color: 'Por color' };

/** Pestañas dentro de cada categoría (botón View): Todos · Mi lista · Colecciones. */
export type TitlesTab = 'list' | 'coll' | 'all' | 'fin';
export const TITLES_TABS: Array<[TitlesTab, string]> = [['list', 'Mi lista'], ['coll', 'Colecciones'], ['all', 'Catálogo']];

/** Estados de un juego, en el orden en que los recorre el selector. */
export const STATUS_ORDER: GameStatus[] = ['none', 'pending', 'playing', 'finished', 'dropped'];
export const STATUS_LABEL: Record<GameStatus, string> = {
  none: 'Sin estado', pending: 'Pendiente', playing: 'Jugando', finished: 'Terminado', dropped: 'Abandonado',
};
/** Estantería de «Mi lista»: color fijo de cada estado (los terminados se agrupan por año). */
export type ListKind = 'playing' | 'pending' | 'finished' | 'dropped' | 'none';
export const LIST_COLOR: Record<ListKind, Accent> = {
  playing: RAINBOW[4], pending: RAINBOW[1], finished: RAINBOW[3], dropped: RAINBOW[0],
  none: { n: 'Gris', c: '#8a93a3', dark: false },
};
export interface ListShelf {
  /** Clave estable (`playing`, `finished-2026`…): recuerda el desplazamiento de cada estantería. */
  key: string;
  kind: ListKind;
  label: string;
  /** Año de los terminados (0 = sin fecha). */
  year: number;
  /** Índices de títulos en la categoría, en el orden de la estantería. */
  items: number[];
}
/** Barra rápida de estado: paso 1 elige el estado; con «Terminado» el paso 2 pide fecha y nota. */
export interface StatusBarState {
  title: number;
  row: number;
  step: 'pick' | 'finish';
  /** 0 día · 1 mes · 2 año · 3 nota */
  field: number;
  y: number; m: number; d: number;
  rating: number;
}
/** Aviso de juegos parados: una fila por juego; `choice` 0 = Pendiente, 1 = Abandonado, 2 = Dejarlo. */
export interface StalledState {
  rows: Array<{ id: string; name: string; days: number; choice: number }>;
  row: number;
}
/** Un juego de «Seguir jugando» en Inicio: plataforma e índice del título. */
export interface RecentRef { pi: number; ti: number }
export const RECENT_N = 3;

const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;
const byName = new Intl.Collator('es', { numeric: true, sensitivity: 'base' });

/** Mismos criterios que `visor_core::scan::sort_shots`. `shots` llega ya ordenado por fecha (recientes primero). */
export function sortShots(shots: Shot[], mode: SortMode, favs: Set<string>): Shot[] {
  if (mode === 'recent') return shots;
  if (mode === 'oldest') return [...shots].reverse();
  if (mode === 'name') return [...shots].sort((a, b) => byName.compare(fileName(a.path), fileName(b.path)) || (a.path < b.path ? -1 : 1));
  const f: Shot[] = [], r: Shot[] = [];
  for (const s of shots) (favs.has(s.path) ? f : r).push(s);
  return [...f, ...r];
}

/** Último uso de un juego: la última partida o, si no la hay, su última captura (0 = nada). */
export const lastUse = (t: Title) => t.lastPlayed || t.shots[0]?.date || 0;

/** Tono (0..360, empezando en el rojo) del color de la portada, o null si no tiene color (sin acento o casi gris). */
function coverHue(t: Title): number | null {
  const hex = t.accentOverride ?? t.accent?.accent.color;
  const rgb = hex ? hexToRgb(hex) : null;
  if (!rgb) return null;
  const [, c, h] = rgbToLch(rgb);
  return c < 0.04 ? null : (h - 20 + 360) % 360;
}

/** Ordena índices de títulos: última partida (más reciente primero), A–Z o por tono del acento de la portada. */
export function sortTitles(titles: Title[], idx: number[], order: TitleOrder): number[] {
  const name = (a: number, b: number) => byName.compare(titles[a].name, titles[b].name) || a - b;
  if (order === 'az') return [...idx].sort(name);
  if (order === 'color') {
    const hue = new Map(idx.map((i) => [i, coverHue(titles[i])] as const));
    return [...idx].sort((a, b) => {
      const ha = hue.get(a), hb = hue.get(b);
      if (ha == null || hb == null) return ha == null && hb == null ? name(a, b) : ha == null ? 1 : -1;
      return ha - hb || name(a, b);
    });
  }
  // Los juegos sin nada de actividad quedan al final, por nombre.
  return [...idx].sort((a, b) => lastUse(titles[b]) - lastUse(titles[a]) || name(a, b));
}

/** Un color `#rrggbb` como acento de la interfaz. */
export const accentOf = (c: string, dark: boolean, n = 'Personalizado'): Accent => ({ n, c, dark });

class Store {
  config = $state<Config>(structuredClone(DEFAULT_CONFIG));
  lib = $state.raw<Library>({ platforms: [], hidden: [] });
  favs = new SvelteSet<string>();
  ready = $state(false);
  scanning = $state(false);
  /** Los titulos son `$state.raw`: sube al cambiar su escalado con Magpie para repintar el menu. */
  magpieTick = $state(0);
  /** Donde esta Magpie (se pide al abrir Ajustes). */
  magpieStatus = $state<MagpieStatus | null>(null);

  view = $state<View>('home');
  overlay = $state<Overlay>('');
  /** Pila de paneles anteriores: B vuelve al que abrió el actual. */
  overlayStack = $state<Overlay[]>([]);
  menu = $state<MenuState>({ kind: 'detail', title: 0, row: 0, items: [], confirm: false });
  form = $state<FormState>({ kind: 'exe', exe: '', rom: '', name: '', args: '', platform: 'pc', nameTouched: false, hero: '', logo: '', cover: '' });
  formRow = $state(0);
  addRow = $state(0);
  gdir = $state<GameDirState>({ loading: false, dir: '', items: [], i: 0, row: 0, added: 0, known: 0, empty: 0 });
  hiddenRow = $state(0);
  collections = $state.raw<Collection[]>([]);
  collForm = $state<CollFormState>({ id: null, platform: '', name: '', nameTouched: false, members: [], keep: [], color: null, step: 'form', sugg: [], row: 0, confirm: false, dirty: false });
  picker = $state<PickerState>({ q: '', ids: [], filter: 'all', at: 0, start: '' });
  addColl = $state<AddCollState>({ ti: 0, row: 0 });
  /** Colección cuyas capturas se están viendo (vista de capturas combinada); null = una vista normal. */
  collOpen = $state<string | null>(null);
  /** Filtro por juego en la vista de una colección: -1 = todos, si no el índice del miembro. */
  sagaFilter = $state(-1);
  /** Pestaña «Colecciones»: estantería y portada enfocadas (la estantería `collShelves.length` es «Nueva colección»), primera estantería visible y desplazamiento de cada una. */
  collAt = $state({ c: 0, i: 0 });
  collTop = $state(0);
  collFrom = $state<Record<string, number>>({});
  moveTo = $state<MoveState>({ paths: [], targets: [], row: 0, confirm: false });
  osk = $state<OskState>({ open: false, label: '', value: '', shift: false, r: 1, c: 0, hint: '' });
  accentPick = $state<AccentPickState>({ title: 0, row: 0, col: 0, hue: 220, palette: [] });
  /** Vista previa del acento mientras se elige: un `#rrggbb`, 'auto' (ignorar el manual) o null. */
  accentPreview = $state<string | null>(null);
  /** Modo de selección de capturas (para mover varias a la vez). */
  selMode = $state(false);
  sel = new SvelteSet<string>();
  anim = $state<'' | 'fwd' | 'back'>('');
  animKey = $state(0);

  p = $state(0); // plataforma
  t = $state(0); // titulo enfocado / abierto
  s = $state(0); // captura enfocada / abierta
  hp = $state(0); // plataforma enfocada en inicio
  goff = $state(0); // desplazamiento de la cuadricula de capturas
  /** Partida en curso lanzada desde el visor (la ficha muestra «Jugando» y un cronómetro). */
  /** Se incrementa al recargar los grafismos para releer las imágenes. */
  artVer = $state(0);
  playing = $state<{ id: string; since: number } | null>(null);
  /** Última partida cerrada con las estadísticas previas, para animar lo ganado en la ficha. */
  gain = $state<{ id: string; seconds: number; secs: number; week: number[]; day: string; at: number } | null>(null);
  /** Desde dónde se abrió el visor (para que B vuelva al mismo sitio). */
  viewerFrom = $state<'detail' | 'shots'>('shots');
  focus = $state('');
  fk: Record<string, string> = {}; // memoria de foco por pantalla

  favView = $state(false);
  favSnap = $state.raw<Shot[]>([]);
  sortMode = $state<SortMode>('recent');
  idle = $state(false);
  /** Zoom del visor: aumento (1 = ajustado al marco) y desplazamiento de la imagen en píxeles del escenario. */
  vz = $state({ k: 1, x: 0, y: 0 });
  /** Visor en modo «solo imagen»: sin barra, pistas ni datos. */
  vbare = $state(false);
  vdir = $state(0); // sentido de la ultima animacion del visor
  padConnected = $state(false);
  /** Ajustes espera un botón para capturar (mando, ratón o tecla). */
  learn = $state<LearnState | null>(null);
  setRow = $state(0);
  /** Sección de Ajustes abierta. */
  setSec = $state(0);
  fullscreen = $state(false);
  /** Monitor donde está el visor (Ajustes → Sistema). */
  monitorLabel = $state('');
  /** Salidas de audio de Windows (icono de la barra superior y Ajustes). */
  audioOutputs = $state<AudioOutput[]>([]);
  /** Desplegable de la salida de audio (barra superior): abierto y fila enfocada con el mando. */
  audioMenu = $state(false);
  audioRow = $state(0);
  /** Copia de seguridad: incluir los medios del glosario (phrases/) y, si hay una restauración preparada, su descripción (A otra vez para confirmar). */
  backupMedia = $state(false);
  backupBusy = $state(false);
  restoreReady = $state('');
  toastMsg = $state('');
  toastOn = $state(false);
  now = $state(new Date());

  platform = $derived(this.lib.platforms[this.p]);

  /* Estadísticas: clasificación por periodo o calendario; evolución de un juego. */
  statsTab = $state<'rank' | 'cal' | 'year'>('rank');
  /** Resumen anual: año elegido (null = el último con partidas), tarjeta actual y sentido de la última animación. */
  yearSel = $state<number | null>(null);
  yearCard = $state(0);
  yearDir = $state<1 | -1>(1);
  statsKind = $state<PeriodKind>('month');
  /** Periodo mostrado: 0 = el actual, -1 = el anterior… */
  statsOff = $state(0);
  /** Ventana del calendario: 0 = los últimos 6 meses, -1 = los 6 anteriores… */
  calOff = $state(0);
  evoKind = $state<EvoKind>('week');
  /** Pantalla de racha: anillo elegido (mapa y cifras) y pantalla a la que vuelve B. */
  streakKey = $state<GoalKey>('vn');
  streakFrom = $state<View>('home');
  /** Desde dónde se abrió la ficha (B vuelve allí). */
  detailFrom = $state<'titles' | 'stats' | 'gstats'>('titles');
  /** Estadísticas de juego: de un título (desde Evolución) o de la categoría abierta (desde Start en la lista), métrica elegida y pantalla de origen. */
  gsScope = $state<'title' | 'cat'>('title');
  gsMetric = $state<GMetric>('time');
  gsFrom = $state<'evo' | 'titles'>('evo');
  /** Cifras de la base del juego abierto en las estadísticas de juego (null mientras llegan). */
  gsDb = $state<GameStatsDb | null>(null);
  /** Página de las estadísticas de un título: resumen o periféricos (clics, rueda, teclas, mando). */
  gsPage = $state<'sum' | 'io'>('sum');
  /** Categoría de la clasificación (índice en `lib.platforms`; si ya no tiene partidas, la primera que las tenga). */
  statsPlatSel = $state(-1);
  statsPlats = $derived(playedPlatforms(this.lib));
  statsPlat = $derived(this.statsPlats.includes(this.statsPlatSel) ? this.statsPlatSel : (this.statsPlats[0] ?? -1));
  rank = $derived(rankPeriod(this.lib, this.statsKind, this.statsOff, this.now.getTime(), this.statsPlat));
  cal = $derived(calendar(this.lib, this.calOff, this.now.getTime()));
  /** Estadísticas de juego del título abierto y de la categoría abierta. */
  gsTitle = $derived.by(() => { const t = this.lib.platforms[this.p]?.titles[this.t]; return t ? titleStats(t) : null; });
  gsCat = $derived(categoryStats(this.lib, this.p, this.gsMetric));
  years = $derived(playYears(this.lib));
  /** Año mostrado en el resumen anual. */
  yearNow = $derived(this.yearSel !== null && this.years.includes(this.yearSel) ? this.yearSel : (this.years.at(-1) ?? new Date().getFullYear()));
  yr = $derived(yearReview(this.lib, this.yearNow, this.favs, this.now.getTime()));
  evo = $derived.by(() => {
    const t = this.lib.platforms[this.p]?.titles[this.t];
    return t ? evoBuckets(t, this.evoKind, this.now.getTime()) : [];
  });

  /** Filtro de la lista de títulos y orden visible resultante (índices reales en `platform.titles`). */
  titleFilter = $state<TitleFilter>('all');
  catalogSearch = $state('');
  catalogLetter = $state('');
  /** Pestaña de la lista de juegos (solo «Todos» tiene contenido por ahora). */
  titlesTab = $state<TitlesTab>('list');
  /** «Mi lista»: primera portada visible de cada estantería, primera estantería visible, barra de estado y aviso de parados. */
  listFrom = $state<Record<string, number>>({});
  listTop = $state(0);
  statusBar = $state<StatusBarState>({ title: 0, row: 0, step: 'pick', field: 0, y: 2026, m: 1, d: 1, rating: 0 });
  stalled = $state<StalledState>({ rows: [], row: 0 });
  /** Colecciones de la categoría abierta, en su orden, con los índices de sus juegos por año. */
  collShelves = $derived.by((): Array<{ coll: Collection; items: number[] }> => {
    const p = this.lib.platforms[this.p];
    if (!p) return [];
    return this.collections.filter((c) => c.platform === p.id).map((coll) => ({ coll, items: collItems(coll, p) }));
  });
  /** Título virtual con las capturas de todos los juegos de la colección abierta. */
  collTitle = $derived.by((): Title | null => {
    const c = this.collOpen ? this.collections.find((x) => x.id === this.collOpen) : undefined;
    const p = c ? this.lib.platforms.find((x) => x.id === c.platform) : undefined;
    return c && p ? collectionTitle(c, p.titles) : null;
  });
  /** Estanterías de «Mi lista» de la categoría abierta (sin ocultos; las vacías no existen). */
  shelves = $derived.by((): ListShelf[] => {
    const p = this.lib.platforms[this.p];
    if (!p) return [];
    const titles = p.titles;
    const idx = titles.map((_, i) => i).filter((i) => !titles[i].hidden && !titles[i].members);
    const of = (s: string) => idx.filter((i) => (titles[i].status ?? 'none') === s);
    const out: ListShelf[] = [];
    if (this.titlesTab === 'fin') {
      const fin = of('finished');
      const years = new Map<number, number[]>();
      for (const i of fin) {
        const ms = titles[i].finishedMs;
        const y = ms ? new Date(ms).getFullYear() : 0;
        (years.get(y) ?? years.set(y, []).get(y)!).push(i);
      }
      const sortedYears = [...years.keys()].filter((y) => y > 0).sort((a, b) => b - a);
      if (years.has(0)) sortedYears.push(0);
      for (const y of sortedYears) {
        const items = years.get(y)!.sort((a, b) => (titles[a].finishedMs ?? 0) - (titles[b].finishedMs ?? 0) || a - b);
        if (items.length) out.push({ key: 'fin-' + y, kind: 'finished', label: y ? String(y) : 'Sin fecha', year: y, items });
      }
      return out;
    }
    const add = (kind: ListKind, label: string, items: number[], year = 0, key: string = kind) => { if (items.length) out.push({ key, kind, label, year, items }); };
    add('playing', 'Jugando ahora', sortTitles(titles, of('playing'), 'last'));
    add('dropped', 'En pausa / Abandonados', sortTitles(titles, of('dropped'), 'last'));
    const pend = of('pending').sort((a, b) => (titles[a].listPos ?? 1e9) - (titles[b].listPos ?? 1e9) || byName.compare(titles[a].name, titles[b].name) || a - b);
    add('pending', 'Pendientes', pend);
    return out;
  });
  finShelves = $derived.by((): ListShelf[] => {
    const p = this.lib.platforms[this.p];
    if (!p) return [];
    const titles = p.titles;
    const idx = titles.map((_, i) => i).filter((i) => !titles[i].hidden && !titles[i].members && (titles[i].status ?? 'none') === 'finished');
    const years = new Map<number, number[]>();
    for (const i of idx) {
      const ms = titles[i].finishedMs;
      const y = ms ? new Date(ms).getFullYear() : 0;
      (years.get(y) ?? years.set(y, []).get(y)!).push(i);
    }
    const sortedYears = [...years.keys()].filter((y) => y > 0).sort((a, b) => b - a);
    if (years.has(0)) sortedYears.push(0);
    const out: ListShelf[] = [];
    for (const y of sortedYears) {
      const items = years.get(y)!.sort((a, b) => (titles[b].finishedMs ?? 0) - (titles[a].finishedMs ?? 0) || a - b);
      if (items.length) out.push({ key: 'fin-' + y, kind: 'finished', label: y ? String(y) : 'Sin fecha', year: y, items });
    }
    return out;
  });
  /** Orden activo de la categoría abierta. */
  titleOrder = $derived<TitleOrder>(this.config.titlesOrder?.[this.lib.platforms[this.p]?.id ?? ''] ?? 'last');
  tview = $derived.by((): number[] => {
    const titles = this.lib.platforms[this.p]?.titles ?? [];
    const all = titles.map((_, i) => i);
    let v: number[];
    switch (validFilter(this.titleFilter)) {
      case 'launch': v = all.filter((i) => titles[i].launchable); break;
      case 'shots': v = all.filter((i) => !titles[i].launchable); break;
      case 'favs': v = all.filter((i) => titles[i].shots.some((s) => this.favs.has(s.path))); break;
      default: v = all;
    }
    if (this.catalogSearch.trim()) {
      const q = this.catalogSearch.toLowerCase().trim();
      v = v.filter((i) => titles[i].name.toLowerCase().includes(q));
    }
    if (this.catalogLetter) {
      v = v.filter((i) => titles[i].name.toUpperCase().startsWith(this.catalogLetter));
    }
    return sortTitles(titles, v, this.titleOrder);
  });

  /** Tarjeta «Lectura de hoy» en Inicio (si existe alguna categoría de novelas). */
  readCard = $derived(!!this.config.root && this.lib.platforms.some((p) => p.id === 'vn' || p.id === 'jvn'));

  /** «Seguir jugando»: los juegos jugados (o con capturas) más recientemente, de todas las plataformas. */
  recent = $derived.by((): RecentRef[] => {
    const out: Array<RecentRef & { k: number }> = [];
    this.lib.platforms.forEach((p, pi) => {
      p.titles.forEach((t, ti) => {
        if (t.hidden) return;
        const k = Math.max(t.lastPlayed ?? 0, t.shots[0]?.date ?? 0);
        if (k > 0) out.push({ pi, ti, k });
      });
    });
    return out.sort((a, b) => b.k - a.k).slice(0, this.readCard ? RECENT_N - 1 : RECENT_N).map(({ pi, ti }) => ({ pi, ti }));
  });

  cur = $derived.by((): CurTitle => {
    if (this.favView) {
      return { name: 'Favoritas', cover: null, shots: sortShots(this.favSnap, this.sortMode, untrack(() => this.favs)), isFavs: true };
    }
    const t = this.collTitle ?? this.lib.platforms[this.p]?.titles[this.t];
    if (!t) return { name: '', cover: null, shots: [], isFavs: false };
    if (t.members && this.sagaFilter >= 0) {
      const gid = t.members[this.sagaFilter]?.id;
      const only = t.shots.filter((s) => s.gid === gid);
      return { name: t.name, cover: t.cover, shots: sortShots(only, this.sortMode, untrack(() => this.favs)), isFavs: false };
    }
    return { name: t.name, cover: t.cover, shots: sortShots(t.shots, this.sortMode, untrack(() => this.favs)), isFavs: false };
  });

  /** Acento del arcoíris (por índice) o color fijo, según el modo («Del juego» usa aquí el arcoíris de reserva). */
  acc(i: number): Accent {
    return RAINBOW[((i % 7) + 7) % 7];
  }
  platAcc(pi: number): Accent {
    return this.acc(this.lib.platforms[pi]?.accent ?? this.config.platforms[pi]?.accent ?? 4);
  }
  /** Acento de un juego: vista previa > color manual > color del arte (modo «Del juego») > arcoíris / fijo. */
  titleAcc(pi: number, ti: number): Accent {
    const rb = this.acc((this.lib.platforms[pi]?.accent ?? 4) + 1 + ti);
    const t = this.lib.platforms[pi]?.titles[ti];
    if (!t) return rb;
    const prev = this.accentPreview;
    if (prev && pi === this.p && ti === this.t && this.view !== 'home' && this.view !== 'titles') {
      if (prev !== 'auto') return accentOf(prev, inkDark(prev));
      return this.autoAcc(t, rb, true);
    }
    return this.autoAcc(t, rb, false);
  }
  private autoAcc(t: Title, rb: Accent, ignoreOverride: boolean): Accent {
    if (!ignoreOverride && t.accentOverride) return accentOf(t.accentOverride, inkDark(t.accentOverride));
    if (t.accent) return accentOf(t.accent.accent.color, t.accent.accent.dark, 'Del juego');
    return rb;
  }
  /** Pareja de tonos de la atmósfera actual. */
  atmosTarget = $derived.by((): { h1: number; h2: number } => {
    if (this.overlay === 'settings') return pair(1);
    if (this.favView) return pair(2 / 6);
    if (this.view === 'home') {
      const N = this.lib.platforms.length;
      return pair(N <= 1 ? 0 : this.hp / (N - 1));
    }
    if (this.view === 'titles') {
      const N = this.lib.platforms.length;
      return pair(N <= 1 ? 0 : this.p / (N - 1));
    }
    if (this.view === 'stats' || this.view === 'evo') return pair(5 / 6);
    if (this.view === 'streak') {
      const h1 = hueOf(GOAL_COL[this.streakKey].c);
      return { h1, h2: (h1 + 35) % 360 };
    }
    const t = this.collTitle ?? this.lib.platforms[this.p]?.titles[this.t];
    if (t) {
      return gamePair(t, this.accentPreview);
    }
    return pair(0);
  });

  /** Acento de la pantalla actual (para --acc del escenario). */
  chrome = $derived.by((): Accent => {
    if (this.view === 'home') return this.platAcc(this.hp);
    if (this.view === 'titles') return this.platAcc(this.p);
    if (this.view === 'stats') return this.acc(5);
    if (this.view === 'streak') return accentOf(GOAL_COL[this.streakKey].c, false, 'Racha');
    if (this.favView) return this.acc(2);
    return this.titleAcc(this.p, this.t);
  });
}

export const app = new Store();
