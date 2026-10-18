// Pantalla de frases (glosario) de la app principal: lista por juego con filtros y repaso SRS, manejable con el mando.
import { IS_TAURI, backend } from './backend';
import { blip } from './sound';
import { pushOverlay, popOverlay, toast } from './controller.svelte';
import { fmtDur } from './format';
import type { Phrase, ReadStats } from './novel-types';

export type PhFilter = 'all' | 'starred' | 'due';
export const FILTERS: PhFilter[] = ['all', 'starred', 'due'];
export const FILTER_LABEL: Record<PhFilter, string> = { all: 'Todas', starred: 'Destacadas', due: 'Pendientes' };

export const ph = $state({
  gameId: null as string | null,
  gameName: '',
  items: [] as Phrase[],
  row: 0,
  filter: 'all' as PhFilter,
  mode: 'list' as 'list' | 'review',
  queue: [] as Phrase[],
  qi: 0,
  revealed: false,
  media: '',
  due: 0,
  furi: true,
  /** Resumen de lectura del juego (o de todos); null = sin datos o sin soporte. */
  stats: null as ReadStats | null,
});

/* ---------- acceso al nucleo (o simulado en el navegador) ---------- */
const MOCK: Phrase[] = [
  ['窓の外では、雪が音もなく降り続いていた。', null, 'Fuera de la ventana, la nieve seguía cayendo sin hacer ruido.', true],
  ['明日、駅まで一緒に行ってくれないかな', '美雪', '¿Me acompañarías mañana hasta la estación?', false],
  ['うん、起きてるよ。寒いから、早めに出よう。', '僕', null, false],
].map(([text, who, tr, star], i) => ({
  id: `mock${i}`, gameId: 'mock', gameName: 'Juego de ejemplo', who: who as string | null, text: text as string,
  toks: [{ parts: [[text as string, null]], pos: 'x' as const }], words: [], grammar: [], context: [],
  createdAt: Date.now() - i * 864e5, lastSeenAt: Date.now() - i * 864e5, seen: 1 + i, translation: tr as string | null, explanation: null,
  starred: !!star, note: '', tags: [], shot: null, audio: null, synth: null, ankiNoteId: null,
  review: { due: i === 2 ? Date.now() + 864e5 : 0, intervalDays: 0, ease: 2.5, reps: 0, lapses: 0 },
}));

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!IS_TAURI) {
    const f = (args?.filter ?? {}) as { starred?: boolean; due?: boolean };
    if (cmd === 'phrases_list') return MOCK.filter((p) => (!f.starred || p.starred) && (!f.due || p.review.due <= Date.now())) as T;
    if (cmd === 'phrases_due_count') return MOCK.filter((p) => p.review.due <= Date.now()).length as T;
    if (cmd === 'phrases_media_dir') return '' as T;
    if (cmd === 'readstats_get') {
      const t = { lines: 540, chars: 15200, activeMs: 4.1 * 3600e3, phrases: 3, cph: 3700, days: 6 };
      return { today: 0, total: t, games: [], days: [], weeks: [], streak: 3, bestStreak: 5, charsPerDay: 2500, knownTotal: 120, knownDated: 40 } as T;
    }
    if (cmd === 'phrases_grade') {
      const p = MOCK.find((x) => x.id === args?.id)!;
      p.review.due = Date.now() + 864e5 * ([0.01, 1, 3, 4][args?.gradeN as number] ?? 1);
      return p as T;
    }
    if (cmd === 'phrases_update') {
      const p = MOCK.find((x) => x.id === args?.id)!;
      Object.assign(p, args?.patch as object);
      return p as T;
    }
    return null as T;
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

const filterArg = () => (ph.filter === 'starred' ? { starred: true } : ph.filter === 'due' ? { due: true } : {});

export async function reloadPhrases() {
  try {
    ph.items = await call<Phrase[]>('phrases_list', { gameId: ph.gameId, filter: filterArg() });
    ph.due = await call<number>('phrases_due_count', { gameId: ph.gameId });
  } catch (e) {
    toast(String(e), 3200);
  }
  ph.row = Math.max(0, Math.min(ph.row, ph.items.length - 1));
}

/** Abre las frases de un juego (`gameId` = null: de todos). Con `review`, empieza directamente el repaso. */
export async function openPhrases(gameId: string | null, gameName: string, review = false) {
  ph.gameId = gameId;
  ph.gameName = gameName;
  ph.row = 0;
  ph.filter = 'all';
  ph.mode = 'list';
  try { ph.media = await call<string>('phrases_media_dir'); } catch { ph.media = ''; }
  await reloadPhrases();
  try { ph.stats = await call<ReadStats>('readstats_get', { gameId }); } catch { ph.stats = null; }
  pushOverlay('phrases');
  blip(1320);
  if (review) await startReview();
}

export async function startReview() {
  let due: Phrase[] = [];
  try { due = await call<Phrase[]>('phrases_list', { gameId: ph.gameId, filter: { due: true } }); } catch { /* */ }
  if (!due.length) { toast('No hay frases pendientes de repaso ahora mismo', 2600); blip(520); return; }
  due.sort((a, b) => a.review.due - b.review.due);
  ph.queue = due;
  ph.qi = 0;
  ph.revealed = false;
  ph.mode = 'review';
  blip(1320);
}

export const mediaUrl = (file: string | null) => (file && ph.media ? backend.fullUrl(`${ph.media}/${file}`) : '');
let player: HTMLAudioElement | null = null;
export function playAudio(p: Phrase | undefined) {
  const f = p?.audio ?? p?.synth ?? null;
  const u = mediaUrl(f);
  if (!u) { toast('Esta frase no tiene audio', 1800); return; }
  player?.pause();
  player = new Audio(u);
  void player.play().catch(() => toast('No se pudo reproducir el audio', 2200));
}

async function grade(n: 0 | 1 | 2 | 3) {
  const p = ph.queue[ph.qi];
  if (!p) return;
  try { await call('phrases_grade', { gameId: p.gameId, id: p.id, gradeN: n }); } catch (e) { toast(String(e), 3200); }
  ph.qi++;
  ph.revealed = false;
  blip(n === 0 ? 520 : 1000 + n * 200);
  if (ph.qi >= ph.queue.length) { await reloadPhrases(); }
}

async function toggleStar() {
  const p = ph.items[ph.row];
  if (!p) return;
  try { await call('phrases_update', { gameId: p.gameId, id: p.id, patch: { starred: !p.starred } }); } catch (e) { toast(String(e), 3200); return; }
  await reloadPhrases();
  blip(1320);
}

export function phrasesCmd(c: string) {
  if (ph.mode === 'review') {
    const done = ph.qi >= ph.queue.length;
    if (c === 'b' || (done && c === 'a')) { ph.mode = 'list'; void reloadPhrases(); blip(520); return; }
    if (done) return;
    if (!ph.revealed) {
      if (c === 'a') { ph.revealed = true; blip(1320); }
      else if (c === 'rb') playAudio(ph.queue[ph.qi]);
      else if (c === 'y') { ph.furi = !ph.furi; blip(880); }
      return;
    }
    if (c === 'x') void grade(0);
    else if (c === 'lb') void grade(1);
    else if (c === 'a') void grade(2);
    else if (c === 'y') void grade(3);
    else if (c === 'rb') playAudio(ph.queue[ph.qi]);
    return;
  }
  if (c === 'up') { ph.row = Math.max(0, ph.row - 1); blip(880); }
  else if (c === 'down') { ph.row = Math.min(ph.items.length - 1, ph.row + 1); blip(880); }
  else if (c === 'lb') { ph.row = Math.max(0, ph.row - 6); blip(700); }
  else if (c === 'rb') { ph.row = Math.min(ph.items.length - 1, ph.row + 6); blip(700); }
  else if (c === 'a') void startReview();
  else if (c === 'x') void toggleStar();
  else if (c === 'y') { ph.filter = FILTERS[(FILTERS.indexOf(ph.filter) + 1) % FILTERS.length]; ph.row = 0; void reloadPhrases(); blip(1000); }
  else if (c === 'lt' || c === 'rt') playAudio(ph.items[ph.row]);
  else if (c === 'b' || c === 'menu') { popOverlay(); blip(520); }
}

export const closePhrases = () => phrasesCmd('b');
export function pickRow(i: number) { ph.row = i; }
export function reviewReveal() { phrasesCmd('a'); }

/** Resumen corto de lectura para la cabecera de la pantalla de frases (null si aun no hay lectura registrada). */
export function readSummary(): string | null {
  const s = ph.stats;
  if (!s || !s.total.chars) return null;
  const n = (x: number) => x.toLocaleString('es-ES');
  const parts = [`${n(s.total.chars)} caracteres`, fmtDur(Math.round(s.total.activeMs / 1000))];
  if (s.total.cph) parts.push(`${n(s.total.cph)} car/h`);
  if (s.streak > 1) parts.push(`racha de ${s.streak} días`);
  return parts.join(' · ');
}
