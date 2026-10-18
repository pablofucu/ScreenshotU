// Backend simulado para desarrollar y revisar la ventana en el navegador (fuera de Tauri).
import type { HookApi } from './api';
import type { NovelPrefs, ProcInfo, ReaderLine, SearchHit, ThStatus, ThreadInfo } from '../lib/novel-types';

const T = (key: string, name: string, code: string, lines: number, last: string, live: boolean): ThreadInfo => ({ key, name, code, lines, last, live });

const SCRIPT: { who: string | null; text: string }[] = [
  { who: null, text: '窓の外では、雪が音もなく降り続いていた。' },
  { who: null, text: '僕は布団の中で小さく息を吐いて、返事をするかどうか迷う。' },
  { who: '美雪', text: '……ねえ、まだ起きてる？' },
  { who: '美雪', text: '明日、駅まで一緒に行ってくれないかな' },
  { who: '悠斗', text: 'いいけど、どうしたんだよ急に。' },
  { who: null, text: '彼女は少しだけ笑って、毛布を引き寄せた。' },
  { who: '美雪', text: '会わせたい人がいるの。' },
  { who: '店主', text: 'いらっしゃい。ずいぶん久しぶりだね。' },
];
const COLORS: Record<string, string | null> = { 美雪: '#E8384F', 悠斗: '#00A0D8', 店主: null };

export function mockApi(): HookApi {
  const handlers = new Map<string, Set<(p: any) => void>>();
  const emit = (ev: string, p: unknown) => handlers.get(ev)?.forEach(h => h(p));

  const proc: ProcInfo = { pid: 8124, exe: 'shirogane.exe', arch: 32, connected: true };
  const threads: ThreadInfo[] = [
    T('t1', 'SiglusEngine', 'HQ-14*@44FA10', 128, '美雪「明日、駅まで一緒に行ってくれないかな」', true),
    T('t2', 'GetGlyphOutlineW', 'HW-8*@0:gdi32', 1024, '明明日日、、駅駅ままでで一一緒緒にに…', true),
    T('t3', 'SiglusEngine', 'HQ-1C*@44FB80', 41, '美雪', true),
    T('t4', 'TextOutA', 'HS8@0:gdi32', 6, 'セーブ　ロード　オート　スキップ', false),
    T('t5', 'GetTextExtentPoint32W', 'HW8@0:gdi32', 380, '明 日 、 駅 ま で', false),
    T('t6', 'Consola', 'console', 3, 'Inyectado en shirogane.exe (32 bits)', false),
    T('t7', 'Portapapeles', 'clipboard', 0, '', false),
  ];
  let selected: string | null = 't1';
  let nameThread: string | null = null;
  let prefs: NovelPrefs = {
    hookCodes: ['HQ-14*@44FA10:shirogane.exe'], threadKey: null, nameThreadKey: null,
    filters: { dedupeChars: true, joinLines: true, dedupeLines: false, splitName: true, dedupePhrase: true, dedupeSegments: false, dedupeIncremental: false, removeControl: true, removeTags: false, removeBraces: false, regex: [{ pattern: '\\[[^\\]]*\\]', replace: '', on: true }] },
    characters: [{ name: '美雪', color: '#E8384F' }, { name: '悠斗', color: null }],
    reader: { furigana: 'all', posColors: true, fontSize: 28 },
    out: { reader: true, clipboard: false },
  };
  let nextId = 1;
  let step = 0;
  let searchTimer: ReturnType<typeof setInterval> | null = null;

  setTimeout(() => {
    emit('th-proc', proc);
    ['Hook de SiglusEngine detectado (HQ-14*@44FA10)', 'Inyectado en shirogane.exe (32 bits)'].forEach(m => emit('th-log', m));
  }, 200);

  setInterval(() => {
    for (const t of threads.filter(t => t.live)) {
      t.lines++;
      if (t.key === 't2') t.last = [...SCRIPT[step % SCRIPT.length].text].map(c => c + c).join('');
      emit('th-thread', { ...t });
    }
    const s = SCRIPT[step % SCRIPT.length];
    step++;
    const t1 = threads[0];
    t1.last = s.who ? `${s.who}「${s.text}」` : s.text;
    emit('th-thread', { ...t1 });
    emit('th-text', { key: t1.key, text: t1.last });
    if (selected === 't1') {
      const line: ReaderLine = { id: nextId++, at: Date.now(), who: s.who, color: s.who ? COLORS[s.who] ?? null : null, text: s.text, toks: [] };
      emit('th-line', line);
    }
    emit('th-log', `[${new Date().toLocaleTimeString()}] t1 +${[...s.text].length} caracteres`);
  }, 3200);

  const calls: Record<string, (a: any) => unknown> = {
    th_status: (): ThStatus => ({ proc, threads: threads.map(t => ({ ...t })), selected, nameThread, gameId: 'demo', gameName: '白銀の約束' }),
    novel_get_prefs: () => structuredClone(prefs),
    novel_save_prefs: (a) => { prefs = structuredClone(a.prefs); },
    th_select_thread: (a) => { selected = a.key; },
    th_select_name_thread: (a) => { nameThread = a.key; },
    th_insert_code: (a) => { if (!/^[HRQ]/i.test(a.code)) throw 'Código H no válido'; },
    th_remove_hook: (a) => {
      const i = threads.findIndex(t => t.key === a.key);
      if (i >= 0) { threads.splice(i, 1); emit('th-thread-removed', { key: a.key }); }
    },
    th_search_text: (a) => {
      if (searchTimer) clearInterval(searchTimer);
      const hits: SearchHit[] = [
        { code: 'HS-4*@44FA10', text: a.text + '、駅まで一緒に' }, { code: 'HQ-14*@44FA10', text: '美雪「' + a.text + '」' },
        { code: 'HW-8*@0:gdi32', text: [...(a.text as string)].map(c => c + c).join('') }, { code: 'HB4@4A2210', text: a.text },
      ];
      let i = 0;
      searchTimer = setInterval(() => {
        if (i < hits.length) emit('th-search', hits[i++]);
        else { clearInterval(searchTimer!); searchTimer = null; emit('th-search-done', { count: hits.length }); }
      }, 900);
    },
    th_search_cancel: () => { if (searchTimer) clearInterval(searchTimer); searchTimer = null; emit('th-search-done', { count: 0 }); },
    th_reinject: () => emit('th-log', 'Reinyectando…'),
    th_detach: () => { proc.connected = false; emit('th-proc', { ...proc }); },
    novel_open_reader: () => emit('th-log', 'Abrir página de lectura'),
  };

  return {
    async call<T>(cmd: string, args?: Record<string, unknown>) {
      const f = calls[cmd];
      if (!f) throw new Error('mock: comando desconocido ' + cmd);
      return f(args ?? {}) as T;
    },
    async on<T>(ev: string, cb: (p: T) => void) {
      if (!handlers.has(ev)) handlers.set(ev, new Set());
      handlers.get(ev)!.add(cb);
      return () => { handlers.get(ev)?.delete(cb); };
    },
  };
}
