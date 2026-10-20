// Estado de la app móvil: conexión, instantánea de la biblioteca, navegación por pila y preferencias.
import { SvelteSet } from 'svelte/reactivity';
import { RAINBOW, type Accent } from '@shared/icons';
import { inkDark } from '@shared/oklch';
import { withSagas } from '@shared/sagas';
import type { Library, Shot, Title } from '@shared/types';
import type { EvoKind, PeriodKind } from '@shared/stats';
import { AuthError, demoSource, RemoteSource, type Playing, type RemoteConfig, type ServerCfg, type Snapshot, type Source } from './source';
import { DEFAULT_PREFS, loadPrefs, loadServer, readSnap, saveServer, savePrefs, serverKey, writeSnap, type Prefs } from './cache';

export type Tab = 'home' | 'games' | 'stats' | 'favs' | 'settings';
export type Route =
  | { v: 'home' }
  | { v: 'games' }
  | { v: 'stats' }
  | { v: 'favs' }
  | { v: 'settings' }
  | { v: 'detail'; id: string }
  | { v: 'shots'; id: string }
  | { v: 'evo'; id: string }
  | { v: 'viewer' };

export interface TitleRef { pi: number; ti: number; t: Title }
export interface ViewerState { shots: Shot[]; i: number; title: string; from: string }

const byName = new Intl.Collator('es', { numeric: true, sensitivity: 'base' });
export const sortByName = (a: string, b: string) => byName.compare(a, b);
export const accentOf = (c: string, dark: boolean, n = 'Personalizado'): Accent => ({ n, c, dark });

const EMPTY_LIB: Library = { platforms: [], hidden: [] };
const DEFAULT_CFG: RemoteConfig = { goalVn: 60, goalJvn: 30, accentMode: 'art', accentFixed: 4 };

class Store {
  /* ---------- conexión ---------- */
  source = $state.raw<Source | null>(null);
  server = $state.raw<ServerCfg | 'demo' | null>(loadServer());
  /** Primera carga en marcha (pantalla de espera) o sincronización en segundo plano. */
  loading = $state(false);
  syncing = $state(false);
  error = $state('');
  /** La última sincronización falló: se enseñan los datos guardados. */
  offline = $state(false);
  syncedAt = $state(0);

  /* ---------- datos ---------- */
  snap = $state.raw<Snapshot | null>(null);
  lib = $state.raw<Library>(EMPTY_LIB);
  favs = new SvelteSet<string>();
  cfg = $state.raw<RemoteConfig>(DEFAULT_CFG);
  playing = $state.raw<Playing | null>(null);
  prefs = $state<Prefs>(loadPrefs());
  now = $state(Date.now());

  /** id de título → posición (incluye las sagas). */
  index = $derived.by(() => {
    const m = new Map<string, TitleRef>();
    this.lib.platforms.forEach((p, pi) => p.titles.forEach((t, ti) => m.set(t.id, { pi, ti, t })));
    return m;
  });
  totalShots = $derived(this.lib.platforms.reduce((a, p) => a + (p.id === 'sagas' ? 0 : p.titles.reduce((b, t) => b + t.shots.length, 0)), 0));
  /** Todas las favoritas (de la más reciente a la más antigua) con su juego. */
  favShots = $derived.by(() => {
    const out: Array<Shot & { tid: string }> = [];
    for (const p of this.lib.platforms) {
      if (p.id === 'sagas') continue;
      for (const t of p.titles) for (const s of t.shots) if (this.favs.has(s.path)) out.push({ ...s, game: t.name, gid: t.id, tid: t.id });
    }
    return out.sort((a, b) => b.date - a.date);
  });

  /* ---------- navegación ---------- */
  tab = $state<Tab>('home');
  stack = $state<Route[]>([{ v: 'home' }]);
  dir = $state<'fwd' | 'back' | ''>('');
  navKey = $state(0);
  /** Desplazamiento guardado de cada nivel de la pila (para volver al mismo sitio). */
  scrolls: number[] = [];
  route = $derived(this.stack[this.stack.length - 1]);
  viewer = $state.raw<ViewerState | null>(null);
  /** Hoja inferior abierta (selector, detalle de un día…). */
  sheet = $state<string>('');

  /* ---------- pantallas ---------- */
  gamesPlat = $state(0);
  gamesFilter = $state<'all' | 'played' | 'shots' | 'favs' | 'az'>('all');
  gamesQuery = $state('');
  statsTab = $state<'rank' | 'cal' | 'year'>('rank');
  statsKind = $state<PeriodKind>('month');
  statsOff = $state(0);
  calOff = $state(0);
  yearSel = $state<number | null>(null);
  evoKind = $state<EvoKind>('week');
  shotsSort = $state<'recent' | 'oldest' | 'favs'>('recent');
  toastMsg = $state('');

  /* ---------- acentos (mismas reglas que el escritorio) ---------- */
  get accentMode() { return this.prefs.accentMode ?? this.cfg.accentMode; }
  get accentFixed() { return this.prefs.accentMode ? this.prefs.accentFixed : this.cfg.accentFixed; }
  acc(i: number): Accent {
    return RAINBOW[this.accentMode === 'fixed' ? this.accentFixed : ((i % 7) + 7) % 7];
  }
  platAcc(pi: number): Accent {
    return this.acc(this.lib.platforms[pi]?.accent ?? 4);
  }
  titleAcc(pi: number, ti: number): Accent {
    const rb = this.acc((this.lib.platforms[pi]?.accent ?? 4) + 1 + ti);
    const t = this.lib.platforms[pi]?.titles[ti];
    if (!t) return rb;
    if (t.accentOverride) return accentOf(t.accentOverride, inkDark(t.accentOverride));
    if (this.accentMode === 'art' && t.accent) return accentOf(t.accent.accent.color, t.accent.accent.dark, 'Del juego');
    return rb;
  }
  accById(id: string): Accent {
    const r = this.index.get(id);
    return r ? this.titleAcc(r.pi, r.ti) : RAINBOW[4];
  }
  /** Acento de la pantalla actual (fondo, barra superior). */
  chrome = $derived.by((): Accent => {
    const r = this.route;
    if (r.v === 'detail' || r.v === 'shots' || r.v === 'evo') return this.accById(r.id);
    if (r.v === 'games') return this.platAcc(this.gamesPlat);
    if (r.v === 'stats') return this.acc(5);
    if (r.v === 'favs') return this.acc(2);
    if (r.v === 'settings') return this.acc(4);
    return RAINBOW[4];
  });

  setPrefs(p: Partial<Prefs>) {
    this.prefs = { ...this.prefs, ...p };
    savePrefs(this.prefs);
  }

  /* ---------- datos ---------- */
  apply(snap: Snapshot) {
    this.snap = snap;
    this.cfg = { ...DEFAULT_CFG, ...snap.config };
    this.lib = snap.sagas?.length ? withSagas(snap.library, snap.sagas) : snap.library;
    this.favs.clear();
    for (const f of snap.favorites ?? []) this.favs.add(f);
    this.playing = snap.playing ?? null;
    if (this.gamesPlat >= this.lib.platforms.length) this.gamesPlat = 0;
  }

  /** Conecta con el servidor guardado: primero lo guardado en el móvil, luego lo nuevo del PC. */
  async start() {
    const s = this.server;
    if (!s) return;
    this.error = '';
    if (s === 'demo') {
      this.loading = true;
      try {
        this.source = await demoSource();
        const snap = await this.source.snapshot();
        if (snap) this.apply(snap);
        this.syncedAt = Date.now();
        this.offline = false;
      } finally { this.loading = false; }
      return;
    }
    const src = new RemoteSource(s);
    this.source = src;
    const cached = await readSnap(serverKey(s));
    if (cached) {
      this.apply(cached.snap);
      src.setEtag(cached.etag);
      this.syncedAt = cached.savedAt;
    } else {
      this.loading = true;
    }
    await this.sync();
    this.loading = false;
  }

  /** Pide la instantánea al PC (si no ha cambiado, el PC contesta 304 y no se baja nada). */
  async sync(fresh = false) {
    const src = this.source;
    if (!src || this.syncing) return;
    this.syncing = true;
    try {
      const snap = await src.snapshot(fresh);
      if (snap) {
        this.apply(snap);
        if (src instanceof RemoteSource && this.server && this.server !== 'demo') {
          void writeSnap(serverKey(this.server), snap, src.getEtag());
          if (snap.name && this.server.name !== snap.name) {
            this.server = { ...this.server, name: snap.name };
            saveServer(this.server);
          }
        }
      }
      this.syncedAt = Date.now();
      this.offline = false;
      this.error = '';
    } catch (e) {
      this.offline = true;
      this.error = e instanceof Error ? e.message : String(e);
      if (e instanceof AuthError && !this.snap) this.disconnect(this.error);
    } finally {
      this.syncing = false;
    }
  }

  connect(s: ServerCfg | 'demo') {
    this.server = s;
    saveServer(s);
    this.snap = null;
    this.lib = EMPTY_LIB;
    this.resetNav('home');
    return this.start();
  }

  disconnect(msg = '') {
    this.server = null;
    saveServer(null);
    this.source = null;
    this.snap = null;
    this.lib = EMPTY_LIB;
    this.favs.clear();
    this.error = msg;
    this.resetNav('home');
  }

  /* ---------- navegación ---------- */
  private scrollEl() { return document.getElementById('scroller'); }
  go(r: Route) {
    this.scrolls[this.stack.length - 1] = this.scrollEl()?.scrollTop ?? 0;
    this.dir = 'fwd';
    this.stack = [...this.stack, r];
    this.navKey++;
  }
  /** Vuelve atrás; false si ya se está en la raíz de la pestaña de inicio (la app puede cerrarse). */
  back(): boolean {
    if (this.sheet) { this.sheet = ''; return true; }
    if (this.stack.length > 1) {
      this.dir = 'back';
      this.stack = this.stack.slice(0, -1);
      this.navKey++;
      return true;
    }
    if (this.tab !== 'home') { this.setTab('home'); return true; }
    return false;
  }
  setTab(t: Tab) {
    if (this.tab === t && this.stack.length === 1) {
      this.scrollEl()?.scrollTo({ top: 0, behavior: 'smooth' });
      return;
    }
    this.resetNav(t);
  }
  resetNav(t: Tab) {
    this.tab = t;
    this.dir = '';
    this.stack = [{ v: t }];
    this.scrolls = [];
    this.sheet = '';
    this.navKey++;
  }
  savedScroll() { return this.dir === 'back' ? (this.scrolls[this.stack.length - 1] ?? 0) : 0; }

  openTitle(id: string) { this.go({ v: 'detail', id }); }
  openViewer(shots: Shot[], i: number, title: string) {
    this.viewer = { shots, i, title, from: this.route.v };
    this.go({ v: 'viewer' });
  }

  toast(msg: string) {
    this.toastMsg = msg;
    clearTimeout(this.toastT);
    this.toastT = setTimeout(() => (this.toastMsg = ''), 2600);
  }
  private toastT: ReturnType<typeof setTimeout> | undefined;
}

export const app = new Store();
export { DEFAULT_PREFS };
