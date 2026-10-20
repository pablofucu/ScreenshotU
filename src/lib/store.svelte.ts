import { SvelteSet } from 'svelte/reactivity';
import { untrack } from 'svelte';
import { RAINBOW, type Accent } from './icons';
import type { AudioOutput, Config, GameStatus, Library, MagpieStatus, Saga, Shot, SortMode as SortModeT, Swatch, Title } from './types';
import { inkDark } from './oklch';
import { GOAL_COL, type GoalKey } from './readgoals';
import { calendar, evoBuckets, playYears, rankPeriod, yearReview, type EvoKind, type PeriodKind } from './stats';

export type View = 'home' | 'titles' | 'detail' | 'shots' | 'viewer' | 'stats' | 'evo' | 'streak';
export type SortMode = SortModeT;

export const DEFAULT_CONFIG: Config = {
  root: null,
  titlesView: 'grid',
  shotsView: 'grid',
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
};

export type MenuItem =
  | 'folder' | 'reload' | 'remove' | 'hide' | 'unhide' | 'addshots' | 'accent'
  | 'select' | 'unselect' | 'selall' | 'move' | 'reveal' | 'editsaga' | 'delsaga' | 'sgdb' | 'magpie' | 'phrases' | 'status' | 'saves' | 'category' | `cat:${string}`;
export type Overlay = '' | 'settings' | 'menu' | 'form' | 'add' | 'gdir' | 'hidden' | 'moveto' | 'accent' | 'saga' | 'sgdb' | 'sgdbtitles' | 'phrases' | 'saves';
export interface SagaFormState {
  /** null = saga nueva. */
  id: string | null;
  name: string;
  nameTouched: boolean;
  members: string[];
  /** 0 nombre · 1..n juegos · n+1 guardar */
  row: number;
}
export interface MenuState {
  /** detail = ficha del juego; shots = cuadrícula o visor de capturas. */
  kind: 'detail' | 'shots';
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

/** Filtros de la lista de títulos (LT/RT). */
export type TitleFilter = 'all' | 'launch' | 'shots' | 'favs' | 'playing' | 'pending' | 'finished' | 'az';
export const TITLE_FILTERS: Array<[TitleFilter, string]> = [
  ['all', 'Todos'], ['launch', 'Con lanzador'], ['shots', 'Solo capturas'], ['favs', 'Con favoritas'],
  ['playing', 'Jugando'], ['pending', 'Pendientes'], ['finished', 'Terminados'], ['az', 'A–Z'],
];

/** Estados de un juego, en el orden en que los recorre el selector. */
export const STATUS_ORDER: GameStatus[] = ['none', 'pending', 'playing', 'finished', 'dropped'];
export const STATUS_LABEL: Record<GameStatus, string> = {
  none: 'Sin estado', pending: 'Pendiente', playing: 'Jugando', finished: 'Terminado', dropped: 'Abandonado',
};
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
  sagas = $state.raw<Saga[]>([]);
  sagaForm = $state<SagaFormState>({ id: null, name: '', nameTouched: false, members: [], row: 0 });
  /** Filtro por juego en la vista de una saga: -1 = todos, si no el índice del miembro. */
  sagaFilter = $state(-1);
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
  audioMenu = $state(false);
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
  detailFrom = $state<'titles' | 'stats'>('titles');
  rank = $derived(rankPeriod(this.lib, this.statsKind, this.statsOff, this.now.getTime()));
  cal = $derived(calendar(this.lib, this.calOff, this.now.getTime()));
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
  tview = $derived.by((): number[] => {
    const titles = this.lib.platforms[this.p]?.titles ?? [];
    const all = titles.map((_, i) => i);
    if (this.lib.platforms[this.p]?.id === 'sagas') return all;
    switch (this.titleFilter) {
      case 'launch': return all.filter((i) => titles[i].launchable);
      case 'shots': return all.filter((i) => !titles[i].launchable);
      case 'favs': return all.filter((i) => titles[i].shots.some((s) => this.favs.has(s.path)));
      case 'playing': return all.filter((i) => titles[i].status === 'playing');
      case 'pending': return all.filter((i) => titles[i].status === 'pending');
      case 'finished': return all.filter((i) => titles[i].status === 'finished');
      case 'az': return [...all].sort((a, b) => byName.compare(titles[a].name, titles[b].name));
      default: return all;
    }
  });

  /** Tarjeta «Lectura de hoy» en Inicio (si existe alguna categoría de novelas); ocupa el sitio de Sagas, que pasa al dock. */
  readCard = $derived(!!this.config.root && this.lib.platforms.some((p) => p.id === 'vn' || p.id === 'jvn'));

  /** «Seguir jugando»: los juegos jugados (o con capturas) más recientemente, de todas las plataformas. */
  recent = $derived.by((): RecentRef[] => {
    const out: Array<RecentRef & { k: number }> = [];
    this.lib.platforms.forEach((p, pi) => {
      if (p.id === 'sagas') return;
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
    const t = this.lib.platforms[this.p]?.titles[this.t];
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
    return RAINBOW[this.config.accentMode === 'fixed' ? this.config.accentFixed : ((i % 7) + 7) % 7];
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
    if (this.config.accentMode === 'art' && t.accent) return accentOf(t.accent.accent.color, t.accent.accent.dark, 'Del juego');
    return rb;
  }
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
