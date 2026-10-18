// Lineas de ejemplo (de la maqueta design/lectura-propuestas.html) para `reader.html?demo`, sin WebSocket.
import type { ReadLive, ReadStats, AnkiState, DictState, Explanation, GrammarHit, KanjiInfo, NovelPrefs, Phrase, PhraseFilter, Pos, ReaderLine, ReaderMsg, Tok, WordDef, WordInfo } from '../lib/novel-types';
import type { Link } from './ws';

/** '降[ふ]り続[つづ]いて|v|降り続く' -> Tok con trozos (base, furigana) y forma de diccionario opcional. */
function tok(src: string): Tok {
  const [txt, pos, base] = src.split('|');
  const parts: [string, string | null][] = [];
  const re = /([^\[\]]+)\[([^\]]+)\]|([^\[\]]+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(txt))) {
    if (m[1] !== undefined) {
      // Los kana iniciales ("り続") no llevan furigana: solo el resto (los kanji).
      const [, kana, kanji] = /^([぀-ヿ]*)(.*)$/.exec(m[1])!;
      if (kana) parts.push([kana, null]);
      parts.push([kanji, m[2]]);
    } else parts.push([m[3], null]);
  }
  return base ? { parts, pos: pos as Pos, base } : { parts, pos: pos as Pos };
}

const RAW: { who?: string; t: string[] }[] = [
  { t: ['窓[まど]|n', 'の|p', '外[そと]|n', 'で|p', 'は|p', '、|x', '雪[ゆき]|n', 'が|p', '音[おと]|n', 'も|p', 'なく|a|無い', '降[ふ]り続[つづ]いて|v|降り続く', 'いた|aux', '。|x'] },
  { t: ['僕[ぼく]|n', 'は|p', '布団[ふとん]|n', 'の|p', '中[なか]|n', 'で|p', '小[ちい]さく|a|小さい', '息[いき]|n', 'を|p', '吐[は]いて|v|吐く', '、|x', '返事[へんじ]|n', 'を|p', 'する|v', 'かどうか|p', '迷[まよ]う|v', '。|x'] },
  { who: '美雪', t: ['……|x', 'ねえ|i', '、|x', 'まだ|adv', '起[お]きてる|v|起きる', '？|x'] },
  { who: '美雪', t: ['明日[あした]|n', '、|x', '駅[えき]|n', 'まで|p', '一緒[いっしょ]|n', 'に|p', '行[い]って|v|行く', 'くれ|aux', 'ない|aux', 'かな|p'] },
  { who: '僕', t: ['うん|i', '、|x', '起[お]きてる|v|起きる', 'よ|p', '。|x', '寒[さむ]い|a', 'から|p', '、|x', '早[はや]めに|adv|早めに', '出[で]よう|v|出る', '。|x'] },
  { who: '遥', t: ['二人[ふたり]|n', 'とも|p', '、|x', 'まだ|adv', '寝[ね]ない|v|寝る', 'の|p', '？|x'] },
];

// Mini diccionario de ejemplo: forma de diccionario -> [lectura, [etiquetas, glosas][]]
const DICT: Record<string, [string, [string[], string[]][]]> = {
  窓: ['まど', [[['n'], ['ventana']]]],
  外: ['そと', [[['n'], ['exterior', 'fuera']], [['n'], ['al aire libre']]]],
  雪: ['ゆき', [[['n'], ['nieve']], [['n'], ['nevada']], [['n'], ['blancura (como la nieve)']]]],
  音: ['おと', [[['n'], ['sonido', 'ruido']], [['n'], ['nota (musical)']]]],
  無い: ['ない', [[['adj-i'], ['inexistente', 'no estar', 'no haber']], [['adj-i'], ['sin', 'carente de']]]],
  降り続く: ['ふりつづく', [[['v5k', 'vi'], ['seguir lloviendo/nevando', 'caer sin parar']]]],
  僕: ['ぼく', [[['pn'], ['yo (masculino)']]]],
  布団: ['ふとん', [[['n'], ['futón', 'colchón japonés']]]],
  中: ['なか', [[['n'], ['dentro', 'interior']], [['n'], ['en medio de', 'entre']]]],
  小さい: ['ちいさい', [[['adj-i'], ['pequeño', 'diminuto']], [['adj-i'], ['bajo (sonido)', 'débil']]]],
  息: ['いき', [[['n'], ['aliento', 'respiración']]]],
  吐く: ['はく', [[['v5k', 'vt'], ['espirar', 'echar (aliento)']], [['v5k', 'vt'], ['vomitar']], [['v5k', 'vt'], ['decir', 'soltar (palabras)']]]],
  返事: ['へんじ', [[['n', 'vs'], ['respuesta', 'contestación']]]],
  迷う: ['まよう', [[['v5u', 'vi'], ['dudar', 'vacilar']], [['v5u', 'vi'], ['perderse', 'extraviarse']]]],
  ねえ: ['ねえ', [[['int'], ['oye', 'mira (para llamar la atención)']]]],
  まだ: ['まだ', [[['adv'], ['todavía', 'aún']], [['adv'], ['sólo', 'apenas']]]],
  起きる: ['おきる', [[['v1', 'vi'], ['levantarse', 'despertarse']], [['v1', 'vi'], ['estar despierto', 'quedarse levantado']], [['v1', 'vi'], ['ocurrir', 'suceder']]]],
  明日: ['あした', [[['n', 'adv'], ['mañana']]]],
  駅: ['えき', [[['n'], ['estación (de tren)']]]],
  一緒: ['いっしょ', [[['n', 'vs'], ['juntos', 'en compañía']], [['n'], ['lo mismo', 'igual']]]],
  行く: ['いく', [[['v5k-s', 'vi'], ['ir', 'marcharse']], [['v5k-s', 'vi'], ['avanzar', 'progresar']]]],
  うん: ['うん', [[['int'], ['sí', 'vale']]]],
  寒い: ['さむい', [[['adj-i'], ['frío (tiempo, temperatura)']]]],
  早めに: ['はやめに', [[['adv'], ['con antelación', 'temprano']]]],
  出る: ['でる', [[['v1', 'vi'], ['salir', 'irse']], [['v1', 'vi'], ['aparecer', 'presentarse']], [['v1', 'vi'], ['asistir', 'participar']]]],
  二人: ['ふたり', [[['n'], ['dos personas', 'pareja']]]],
  寝る: ['ねる', [[['v1', 'vi'], ['dormir']], [['v1', 'vi'], ['acostarse', 'irse a la cama']]]],
};


// Frecuencia (puesto JPDB) y kanji de ejemplo
const FREQ: Record<string, number> = {
  窓: 2300, 外: 410, 雪: 1800, 音: 900, 無い: 60, 降り続く: 21000, 僕: 120, 布団: 4200, 中: 30, 小さい: 700, 息: 1900, 吐く: 3500, 返事: 1600,
  迷う: 1700, ねえ: 800, まだ: 150, 起きる: 800, 明日: 350, 駅: 1200, 一緒: 700, 行く: 40, うん: 200, 寒い: 1500, 早めに: 9000, 出る: 90, 二人: 600, 寝る: 900,
};
const KJ: Record<string, [string, string, string, string][]> = {
  窓: [['窓', 'ventana', 'ソウ', 'まど']],
  外: [['外', 'fuera', 'ガイ・ゲ', 'そと・ほか']],
  雪: [['雪', 'nieve', 'セツ', 'ゆき']],
  音: [['音', 'sonido', 'オン・イン', 'おと・ね']],
  降り続く: [['降', 'caer; bajar', 'コウ', 'ふ(る)・お(りる)'], ['続', 'continuar', 'ゾク', 'つづ(く)']],
  布団: [['布', 'tela', 'フ', 'ぬの'], ['団', 'grupo; redondo', 'ダン・トン', 'かたまり']],
  明日: [['明', 'claro; luz', 'メイ', 'あか(るい)・あ(ける)'], ['日', 'día; sol', 'ニチ・ジツ', 'ひ・か']],
  駅: [['駅', 'estación', 'エキ', '—']],
  一緒: [['一', 'uno', 'イチ', 'ひと(つ)'], ['緒', 'cordón; comienzo', 'ショ・チョ', 'お']],
  行く: [['行', 'ir; hacer', 'コウ・ギョウ', 'い(く)・おこな(う)']],
  起きる: [['起', 'levantarse', 'キ', 'お(きる)・お(こす)']],
  寒い: [['寒', 'frío', 'カン', 'さむ(い)']],
  出る: [['出', 'salir', 'シュツ', 'で(る)・だ(す)']],
  寝る: [['寝', 'dormir', 'シン', 'ね(る)']],
  二人: [['二', 'dos', 'ニ', 'ふた(つ)'], ['人', 'persona', 'ジン・ニン', 'ひと']],
};

function words(toks: Tok[]): WordInfo[] {
  const out: WordInfo[] = [];
  toks.forEach((t, i) => {
    if (!['n', 'v', 'a', 'adv', 'i'].includes(t.pos)) return;
    const surface = t.parts.map((p) => p[0]).join('');
    const base = t.base ?? surface;
    if (out.some((w) => w.base === base)) return;
    const e = DICT[base];
    const defs: WordDef[] = e ? [{ term: base, reading: e[0], dict: 'Demo', senses: e[1].map(([tags, glosses]) => ({ tags, glosses })) }] : [];
    const kanji: KanjiInfo[] = (KJ[base] ?? []).map(([ch, m, on, kun]) => ({ ch, meanings: [m], onyomi: on.split('・'), kunyomi: kun.split('・') }));
    const w: WordInfo = { tok: i, surface, base, pos: t.pos, defs };
    if (FREQ[base]) w.freq = FREQ[base];
    if (kanji.length) w.kanji = kanji;
    out.push(w);
  });
  return out;
}

// Gramatica de ejemplo por linea: [desde, hasta, termino, significado]
const GRAM: [number, number, string, string][][] = [
  [[11, 12, '〜ていた', 'estaba haciendo (acción en curso en el pasado)']],
  [[14, 14, 'かどうか', 'si… o no (duda indirecta)']],
  [[4, 4, '〜てる', 'forma coloquial de 〜ている (estado / acción en curso)']],
  [[3, 3, 'まで', 'hasta (límite de lugar o tiempo)'], [6, 8, '〜てくれない', 'petición amable: «¿me harías el favor de…?»'], [9, 9, 'かな', 'me pregunto si…; duda suave, petición tímida']],
  [[2, 2, '〜てる', 'forma coloquial de 〜ている (estado / acción en curso)'], [6, 6, 'から', 'porque; como (causa, tras una frase)'], [9, 9, '〜よう', 'volitivo: «hagamos…», «voy a…»']],
  [[1, 1, 'とも', 'ambos; todos (tras un numeral de personas)']],
];
const grammar = (i: number): GrammarHit[] =>
  (GRAM[i] ?? []).map(([a, b, term, g]) => ({ toks: [a, b], term, defs: [{ term, reading: '', dict: 'Gramática', senses: [{ tags: [], glosses: [g] }] }] }));

export const demoLines: ReaderLine[] = RAW.map((r, i) => {
  const toks = r.t.map(tok);
  return { id: i + 1, at: Date.now() + i, who: r.who ?? null, color: null, text: toks.map((t) => t.parts.map((p) => p[0]).join('')).join(''), toks, words: words(toks), grammar: grammar(i) };
});

export const demoDicts: DictState = { names: ['Demo', 'Gramática', 'JPDB', 'KANJIDIC'], loading: false, dir: '' };
export const demoKnown = ['明日', 'うん', '外', '出る'];
export const demoAnki: AnkiState = { ok: true, deck: 'Japonés::Minado', model: 'Lapis', decks: ['Japonés::Minado', 'Japonés::Novelas'], models: ['Lapis', 'JP Mining Note', 'Básico'] };

export const demoPrefs = {
  hookCodes: [],
  threadKey: null,
  nameThreadKey: null,
  filters: { dedupeChars: false, joinLines: false, dedupeLines: false, splitName: false, dedupePhrase: false, dedupeSegments: false, dedupeIncremental: false, removeControl: false, removeTags: false, removeBraces: false, regex: [] },
  characters: [
    { name: '美雪', color: '#ff4f7b' },
    { name: '僕', color: '#00A0D8' },
    { name: '遥', color: null },
  ],
  reader: { furigana: 'all', posColors: true, fontSize: 30 },
  out: { reader: true, clipboard: false },
} satisfies NovelPrefs;

/** Captura de ejemplo (SVG) para la demo; el matiz cambia con la linea. */
export function demoShot(lineId: number): string {
  const h = (lineId * 47) % 360;
  const l = demoLines.find((x) => x.id === lineId);
  const txt = (l?.text ?? '').replace(/[<&]/g, '');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180"><defs><linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="hsl(${h},35%,32%)"/><stop offset="1" stop-color="hsl(${h},40%,75%)"/></linearGradient></defs><rect width="320" height="180" fill="url(#s)"/><rect x="40" y="18" width="150" height="104" rx="4" fill="#cfdcf2" opacity=".35" stroke="#e9eef8" stroke-width="5"/><ellipse cx="250" cy="150" rx="46" ry="60" fill="#2a2f45"/><circle cx="250" cy="78" r="26" fill="#f3d9c7"/><rect x="10" y="128" width="300" height="46" rx="8" fill="#121a2e" opacity=".82"/><text x="22" y="156" fill="#fff" font-size="12" font-family="sans-serif">${txt}</text></svg>`;
  return 'data:image/svg+xml;utf8,' + encodeURIComponent(svg);
}

// ---------------------------------------------------------------- textos simulados de la IA

const TRANS = [
  'Fuera de la ventana, la nieve seguía cayendo sin hacer ruido.',
  'Bajo el futón, solté un suspiro pequeño, dudando si responder o no.',
  '…Oye, ¿sigues despierto?',
  '¿Me acompañarías mañana hasta la estación?',
  'Sí, estoy despierto. Como hace frío, salgamos con tiempo.',
  'Vosotros dos, ¿todavía no os vais a dormir?',
];
const EXPL: Explanation[] = [
  { translation: TRANS[0], literal: 'Lit.: «En el exterior de la ventana, nieve, sin sonido, seguía cayendo».',
    chain: [{ kind: 'v', piece: '降る', meaning: 'caer (lluvia, nieve)' }, { kind: 'v', piece: '続く', meaning: 'continuar' }, { kind: 'g', piece: '〜ていた', meaning: 'estaba …-ando (pasado continuo)' }],
    notes: ['**音もなく**: «sin siquiera un sonido»; も enfatiza la ausencia.', '**降り続く** = 降る + 続く: verbo compuesto, «seguir cayendo».', 'Es narración (sin nombre): describe la escena.'] },
  { translation: TRANS[1], literal: 'Lit.: «Dentro del futón, respiré pequeño, y si dar respuesta o no, dudé».',
    chain: [{ kind: 'v', piece: '吐く', meaning: 'exhalar' }, { kind: 'n', piece: '返事をする', meaning: 'responder' }, { kind: 'g', piece: 'かどうか', meaning: 'si… o no' }],
    notes: ['**小さく息を吐いて**: suspiro leve; el て enlaza las dos acciones.', '**かどうか** convierte la decisión en una duda indirecta.'] },
  { translation: TRANS[2], literal: 'Lit.: «…Oye, ¿todavía estás despierta/o?»',
    chain: [{ kind: 'v', piece: '起きる', meaning: 'estar despierto' }, { kind: 'g', piece: '〜てる', meaning: '〜ている coloquial: estado' }],
    notes: ['**ねえ** llama la atención en tono íntimo.', '**起きてる** = 起きている: «estar despierto» (estado).'] },
  { translation: TRANS[3], literal: 'Lit.: «Mañana, hasta la estación, juntos, ¿no irías por mí…, me pregunto?».',
    chain: [{ kind: 'v', piece: '行く', meaning: 'ir' }, { kind: 'g', piece: '〜てくれる', meaning: 'hacer algo por mí' }, { kind: 'g', piece: 'ない', meaning: 'negación → petición' }, { kind: 'g', piece: 'かな', meaning: '«me pregunto…»: suaviza' }],
    notes: ['**〜てくれない？** es una petición amable: «¿no me harías el favor de…?».', '**かな** la vuelve tímida, casi pensando en voz alta.', '**まで** marca el destino final: «hasta la estación».'] },
  { translation: TRANS[4], literal: 'Lit.: «Sí, estoy despierto. Porque hace frío, salgamos temprano».',
    chain: [{ kind: 'v', piece: '出る', meaning: 'salir' }, { kind: 'g', piece: '出よう', meaning: 'forma volitiva: «salgamos»' }],
    notes: ['**起きてる** = 起きている, forma coloquial.', '**から** tras una frase da la causa: «como hace frío…».', '**〜よう** propone hacer algo juntos.'] },
  { translation: TRANS[5], literal: 'Lit.: «Los dos, ¿todavía no os dormís?»',
    chain: [{ kind: 'n', piece: '二人とも', meaning: 'los dos' }, { kind: 'v', piece: '寝る', meaning: 'dormir' }, { kind: 'n', piece: 'の？', meaning: 'pregunta coloquial' }],
    notes: ['**とも** tras un número de personas = «todos ellos».', '**〜の？** convierte la frase en pregunta en tono familiar.'] },
];

// ---------------------------------------------------------------- backend simulado

function phraseOf(l: ReaderLine, ctx: ReaderLine[]): Phrase {
  return {
    id: 'demo-' + l.id, gameId: 'demo', gameName: 'Demo', who: l.who, text: l.text, toks: l.toks, words: l.words ?? [], grammar: l.grammar ?? [],
    context: ctx.slice(-3).map((c) => ({ who: c.who, text: c.text })), createdAt: Date.now(), lastSeenAt: Date.now(), seen: 1,
    translation: null, explanation: null, starred: false, note: '', tags: [], shot: null, audio: null, synth: null, ankiNoteId: null,
    review: { due: 0, intervalDays: 0, ease: 2.5, reps: 0, lapses: 0 },
  };
}

/** Sustituye al WebSocket en `reader.html?demo`: responde con datos simulados (sin DeepSeek, Anki ni audio reales). */
export function createDemoLink(onMsg: (m: ReaderMsg) => void, onState: (o: boolean) => void): Link {
  const timers = new Set<number>();
  const later = (ms: number, f: () => void) => {
    const t = window.setTimeout(() => {
      timers.delete(t);
      f();
    }, ms);
    timers.add(t);
  };
  const store = new Map<number, Phrase>();
  const known = new Set(demoKnown);
  let rec: { t: number; secs: number; lineId: number } | null = null;
  let shown = 4;
  const lineOf = (id: number) => demoLines.find((l) => l.id === id)!;
  const ensure = (id: number) => {
    let p = store.get(id);
    if (!p) {
      p = phraseOf(lineOf(id), demoLines.slice(Math.max(0, id - 4), id - 1));
      store.set(id, p);
    }
    return p;
  };
  const pushPhrase = (id: number) => {
    const p = ensure(id);
    onMsg({ type: 'phrase', lineId: id, starred: p.starred, inAnki: p.ankiNoteId !== null, hasAudio: !!p.audio, hasSynth: !!p.synth, translation: p.translation, explanation: p.explanation });
  };
  const glossary = (f?: Partial<PhraseFilter>) => {
    const q = (f?.query ?? '').toLowerCase();
    const list = [...store.values()].filter((p) =>
      (!f?.starred || p.starred) && (!f?.audio || p.audio) && (!f?.anki || p.ankiNoteId !== null) && (!f?.due || p.review.due <= Date.now()) &&
      (!q || p.text.includes(q) || (p.translation ?? '').toLowerCase().includes(q) || p.note.toLowerCase().includes(q)));
    onMsg({ type: 'glossary', phrases: structuredClone(list), dueCount: [...store.values()].filter((p) => p.review.due <= Date.now()).length });
  };
  const shotsFor = (upTo: number) => Array.from({ length: Math.min(3, upTo) }, (_, i) => ({ lineId: upTo - i, at: Date.now() - i * 8000 }));

  // estadisticas simuladas: sesion en curso (caracteres japoneses, tiempo activo) y 30 dias de historial
  const jpChars = (t: string) => (t.match(/[ぁ-ゖァ-ヺー㐀-鿿]/g) ?? []).length;
  const live: ReadLive = { startMs: Date.now(), lines: 0, chars: 0, activeMs: 0, cph: 0, lastMs: 0, pauseMs: 300000 };
  const countLine = (l: ReaderLine) => {
    const t = Date.now();
    if (live.lastMs) live.activeMs += Math.min(t - live.lastMs, live.pauseMs);
    live.lastMs = t;
    live.lines++;
    live.chars += jpChars(l.text);
    live.cph = live.activeMs >= 30000 ? Math.round((live.chars * 3600000) / live.activeMs) : 0;
    onMsg({ type: 'readLive', live: { ...live } });
  };
  demoLines.slice(0, shown).forEach((l, i) => {
    live.lines++;
    live.chars += jpChars(l.text);
    live.activeMs += i ? 7000 : 0;
  });
  live.lastMs = Date.now();
  const demoStats = (): ReadStats => {
    const today = Math.floor(Date.now() / 864e5);
    const days = Array.from({ length: 30 }, (_, i) => {
      const k = 29 - i;
      const chars = k === 0 ? live.chars : k % 6 === 3 ? 0 : 900 + ((k * 7919) % 2600);
      return { day: today - k, chars, activeMs: chars * 1200, newKnown: k % 4 === 0 ? 3 + (k % 5) : 0 };
    });
    const sum = days.reduce((a, d) => a + d.chars, 0);
    const ms = days.reduce((a, d) => a + d.activeMs, 0);
    const wk = today - ((today + 3) % 7);
    const weeks = Array.from({ length: 8 }, (_, i) => ({ start: wk - 7 * (7 - i), newKnown: [12, 18, 9, 22, 15, 27, 20, 11][i] }));
    const tot = (chars: number, activeMs: number) => ({ lines: Math.round(chars / 28), chars, activeMs, phrases: Math.round(chars / 900), cph: Math.round((chars * 3600000) / activeMs), days: 24 });
    return {
      today, days, weeks, total: tot(sum + 41200, ms + 41200 * 1200),
      games: [
        { gameId: 'demo', gameName: 'Demo', lastDay: today, ...tot(sum, ms) },
        { gameId: 'otro', gameName: 'Otra novela de ejemplo', lastDay: today - 34, ...tot(41200, 41200 * 1250), days: 13 },
      ],
      streak: 2, bestStreak: 9, charsPerDay: Math.round(sum / days.filter((d) => d.chars).length), knownTotal: 640, knownDated: 134,
    };
  };

  // arranque: como el `hello` real
  for (const id of [1, 4]) {
    const p = ensure(id);
    p.translation = TRANS[id - 1];
    p.explanation = EXPL[id - 1];
    p.starred = id === 4;
  }
  later(30, () => {
    onState(true);
    onMsg({ type: 'hello', game: 'Demo', prefs: structuredClone(demoPrefs) as NovelPrefs, history: demoLines.slice(0, shown), dicts: demoDicts, known: [...known], anki: demoAnki, shots: shotsFor(shown), readLive: { ...live } });
    pushPhrase(1);
    pushPhrase(4);
  });
  const iv = window.setInterval(() => {
    if (shown >= demoLines.length) return clearInterval(iv);
    const l = demoLines[shown++];
    onMsg({ type: 'line', line: l });
    countLine(l);
    onMsg({ type: 'shots', shots: shotsFor(shown) });
    if (store.has(l.id)) pushPhrase(l.id);
  }, 6000);

  const upd = (id: number, f: (p: Phrase) => void) => {
    f(ensure(id));
    pushPhrase(id);
  };

  function handle(m: Record<string, any>) {
    const id: number = m.lineId;
    switch (m.type) {
      case 'known':
        known.has(m.base) ? known.delete(m.base) : known.add(m.base);
        onMsg({ type: 'known', list: [...known] });
        break;
      case 'star':
        upd(id, (p) => (p.starred = !p.starred));
        break;
      case 'translate': {
        const p = ensure(id);
        if (p.translation) return pushPhrase(id);
        const full = TRANS[id - 1] ?? 'Traducción de ejemplo.';
        const ws = full.split(' ');
        let n = 0;
        const step = () => {
          n++;
          const done = n >= ws.length;
          onMsg({ type: 'ai', lineId: id, kind: 'translate', text: ws.slice(0, n).join(' '), done });
          if (done) upd(id, (q) => (q.translation = full));
          else later(90, step);
        };
        later(500, step);
        break;
      }
      case 'explain': {
        const p = ensure(id);
        if (p.explanation) return onMsg({ type: 'ai', lineId: id, kind: 'explain', explanation: p.explanation, done: true });
        later(1100, () => {
          const e = EXPL[id - 1] ?? { translation: 'Explicación de ejemplo.', literal: '', chain: [], notes: [] };
          onMsg({ type: 'ai', lineId: id, kind: 'explain', explanation: e, done: true });
          upd(id, (q) => {
            q.explanation = e;
            q.translation ??= e.translation;
          });
        });
        break;
      }
      case 'anki':
        later(700, () => {
          upd(id, (p) => (p.ankiNoteId = 1700000000000 + id));
          onMsg({ type: 'ankiResult', lineId: id, word: m.word, ok: true, noteId: 1700000000000 + id });
        });
        break;
      case 'record':
        if (rec) {
          clearInterval(rec.t);
          const secs = rec.secs;
          const rid = rec.lineId;
          rec = null;
          onMsg({ type: 'rec', state: 'done', secs, lineId: rid });
          upd(rid, (p) => (p.audio = 'demo.wav'));
          later(2500, () => onMsg({ type: 'rec', state: 'idle', secs: 0 }));
        } else {
          const r = { t: 0, secs: 0, lineId: id };
          r.t = window.setInterval(() => {
            r.secs++;
            onMsg({ type: 'rec', state: 'recording', secs: r.secs, lineId: id });
          }, 1000);
          rec = r;
          onMsg({ type: 'rec', state: 'recording', secs: 0, lineId: id });
        }
        break;
      case 'synth':
        later(400, () => upd(id, (p) => (p.synth = 'demo-synth.wav')));
        break;
      case 'glossaryList':
        glossary(m.filter);
        break;
      case 'phraseUpdate': {
        const p = [...store.values()].find((x) => x.id === m.id);
        if (!p) break;
        if ('starred' in m) p.starred = m.starred;
        if ('note' in m) p.note = m.note;
        glossary(m.filter);
        break;
      }
      case 'phraseDelete': {
        const e = [...store.entries()].find(([, x]) => x.id === m.id);
        if (e) store.delete(e[0]);
        glossary(m.filter);
        break;
      }
      case 'phraseAnki': {
        const p = [...store.values()].find((x) => x.id === m.id);
        later(600, () => {
          if (p) p.ankiNoteId = 1700000000000;
          onMsg({ type: 'toast', msg: 'Frase enviada a Anki' });
          glossary(m.filter);
        });
        break;
      }
      case 'phraseGrade': {
        const p = [...store.values()].find((x) => x.id === m.id);
        if (p) {
          p.review.reps++;
          p.review.due = Date.now() + [600e3, 864e5, 3 * 864e5, 4 * 864e5][m.grade];
        }
        glossary(m.filter);
        break;
      }
      case 'statsGet':
        later(250, () => onMsg({ type: 'stats', stats: demoStats() }));
        break;
      case 'ankiTest':
        later(300, () => onMsg({ type: 'anki', state: demoAnki }));
        break;
      default:
        break;
    }
  }

  return {
    send(msg) {
      handle(msg as Record<string, any>);
    },
    close() {
      clearInterval(iv);
      timers.forEach(clearTimeout);
    },
  };
}
