// Tipos compartidos de «Novelas japonesas» (ver docs/novelas-japonesas.md). Espejo de src-tauri/src/novel.rs y visor_core::jp.
export interface ProcInfo { pid: number; exe: string; arch: 32 | 64; connected: boolean }
export interface ThreadInfo { key: string; name: string; code: string; lines: number; last: string; live: boolean }
export interface ThStatus { proc: ProcInfo | null; threads: ThreadInfo[]; selected: string | null; nameThread: string | null; gameId: string | null; gameName: string | null }
export interface SearchHit { code: string; text: string }
export type Pos = 'n' | 'v' | 'a' | 'adv' | 'p' | 'aux' | 'i' | 'x';
export interface Tok { parts: [string, string | null][]; pos: Pos; base?: string }
export interface Sense { tags: string[]; glosses: string[] }
export interface WordDef { term: string; reading: string; dict: string; senses: Sense[] }
/** Palabra principal de una linea: `tok` = indice en `toks`. */
export interface KanjiInfo { ch: string; meanings: string[]; onyomi: string[]; kunyomi: string[] }
/** `freq` = puesto en el diccionario de frecuencias (JPDB): menor = mas comun. */
export interface WordInfo { tok: number; surface: string; base: string; pos: Pos; defs: WordDef[]; freq?: number; kanji?: KanjiInfo[] }
/** Punto gramatical: tokens `toks[0]..=toks[1]` de la linea y su entrada del diccionario de gramatica. */
export interface GrammarHit { toks: [number, number]; term: string; defs: WordDef[] }
export interface DictState { names: string[]; loading: boolean; dir: string }
export interface ReaderLine { id: number; at: number; who: string | null; color: string | null; text: string; toks: Tok[]; words?: WordInfo[]; grammar?: GrammarHit[] }
export interface RegexRule { pattern: string; replace: string; on: boolean }
export interface Character { name: string; color: string | null }
export interface NovelPrefs {
  hookCodes: string[];
  threadKey: string | null;
  nameThreadKey: string | null;
  filters: { dedupeChars: boolean; joinLines: boolean; dedupeLines: boolean; splitName: boolean;
    dedupePhrase: boolean; dedupeSegments: boolean; dedupeIncremental: boolean; removeControl: boolean; removeTags: boolean; removeBraces: boolean; regex: RegexRule[] };
  characters: Character[];
  reader: { furigana: 'all' | 'off'; posColors: boolean; fontSize: number };
  out: { reader: boolean; clipboard: boolean };
}
export interface ChainPiece { kind: 'v' | 'g' | 'n'; piece: string; meaning: string }
export interface Explanation { translation: string; literal: string; chain: ChainPiece[]; notes: string[] }
export type RecState = 'idle' | 'recording' | 'done';
/** Estado de AnkiConnect para la cabecera y la ventana de envio. */
export interface AnkiState { ok: boolean; deck: string; model: string; decks: string[]; models: string[]; error?: string }
/** Captura guardada en memoria para una linea (`/shot/<lineId>` la sirve la app). */
export interface ShotRef { lineId: number; at: number }
/** Frase del glosario (espejo de visor_core::jp::phrases::Phrase). */
export interface Phrase {
  id: string; gameId: string; gameName: string; who: string | null; text: string;
  toks: Tok[]; words: WordInfo[]; grammar: GrammarHit[];
  context: { who: string | null; text: string }[];
  createdAt: number; lastSeenAt: number; seen: number;
  translation: string | null; explanation: Explanation | null;
  starred: boolean; note: string; tags: string[];
  shot: string | null; audio: string | null; synth: string | null; ankiNoteId: number | null;
  review: { due: number; intervalDays: number; ease: number; reps: number; lapses: number };
}
export interface PhraseFilter { query: string; starred: boolean; audio: boolean; anki: boolean; due: boolean }
/** Estadisticas de lectura (espejo de visor_core::jp::readstats). Los dias son dias locales desde 1970-01-01. */
export interface ReadTotals { lines: number; chars: number; activeMs: number; phrases: number; cph: number; days: number }
export interface ReadGame extends ReadTotals { gameId: string; gameName: string; lastDay: number }
export interface ReadStats {
  today: number; total: ReadTotals; games: ReadGame[];
  days: { day: number; chars: number; activeMs: number; newKnown: number }[];
  weeks: { start: number; newKnown: number }[];
  streak: number; bestStreak: number; charsPerDay: number; knownTotal: number; knownDated: number;
}
/** Sesion de lectura en curso. `lastMs`/`pauseMs` permiten seguir el reloj en la pagina. */
export interface ReadLive { startMs: number; lines: number; chars: number; activeMs: number; cph: number; lastMs: number; pauseMs: number }
export type ReaderMsg =
  | { type: 'hello'; game: string | null; prefs: NovelPrefs | null; history: ReaderLine[]; dicts?: DictState; known?: string[]; anki?: AnkiState; shots?: ShotRef[]; readLive?: ReadLive | null; needHook?: boolean }
  | { type: 'readLive'; live: ReadLive | null }
  | { type: 'stats'; stats: ReadStats }
  | { type: 'line'; line: ReaderLine }
  | { type: 'prefs'; prefs: NovelPrefs }
  | { type: 'dicts'; dicts: DictState }
  | { type: 'known'; list: string[] }
  | { type: 'ai'; lineId: number; kind: 'translate' | 'explain'; text?: string; explanation?: Explanation; done: boolean; error?: string }
  | { type: 'phrase'; lineId: number; starred: boolean; inAnki: boolean; hasAudio: boolean; hasSynth: boolean; translation?: string | null; explanation?: Explanation | null }
  | { type: 'rec'; state: RecState; secs: number; lineId?: number }
  | { type: 'anki'; state: AnkiState }
  | { type: 'shots'; shots: ShotRef[] }
  | { type: 'ankiResult'; lineId: number; word?: string; ok: boolean; noteId?: number; error?: string }
  | { type: 'toast'; msg: string; bad?: boolean }
  | { type: 'glossary'; phrases: Phrase[]; dueCount: number }
  | { type: 'hookRes'; id: number; ok: boolean; value?: unknown; error?: string }
  | { type: 'hookEvt'; event: string; payload: unknown }
  | { type: 'bye' };
