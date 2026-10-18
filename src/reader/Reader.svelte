<script lang="ts">
  // Pagina de estudio: la frase actual con su barra de ayuda (traduccion tapada, «Estoy perdido», guardar, Anki),
  // las palabras en acordeon (nuevas, gramatica, conocidas) y un pie con las estadisticas de la sesion.
  // Todo lo que usa IA, Anki o audio lo hace la app: aqui solo se piden cosas por WebSocket y se pintan las respuestas.
  import { onMount, tick } from 'svelte';
  import type { AnkiState, DictState, Explanation, NovelPrefs, Phrase, ReadLive, ReadStats, ReaderLine, ReaderMsg, RecState, ShotRef, WordInfo } from '../lib/novel-types';
  import { connect, type Link } from './ws';
  import { createDemoLink, demoPrefs } from './demo';
  import { lineHtml } from './render';
  import { ICON, demo, fmtDur, fmtNum, mmss } from './util';
  import WordList from './WordList.svelte';
  import AnkiModal, { type SendOpts } from './AnkiModal.svelte';
  import ExplainView from './ExplainView.svelte';
  import Glossary from './Glossary.svelte';
  import Stats from './Stats.svelte';
  import HookPanel from './HookPanel.svelte';
  import type { HookApi } from '../hook/api';

  const GRAY = '#8a919c';
  const MAX_LINES = 500;
  const FS_MIN = 16;
  const FS_MAX = 72;

  /** Estado de una linea: lo que la app ha respondido (IA, glosario, audio). */
  interface LineSt {
    translation: string | null; tText: string; tLoading: boolean; tError: string;
    explanation: Explanation | null; eLoading: boolean; eError: string;
    starred: boolean; inAnki: boolean; hasAudio: boolean; hasSynth: boolean; audioVer: number;
  }
  const BLANK: LineSt = {
    translation: null, tText: '', tLoading: false, tError: '', explanation: null, eLoading: false, eError: '',
    starred: false, inAnki: false, hasAudio: false, hasSynth: false, audioVer: 0,
  };

  let lines = $state<ReaderLine[]>([]);
  let prefs = $state<NovelPrefs | null>(null);
  let game = $state<string | null>(null);
  let closed = $state(false); // el juego se cerro
  let online = $state(false);
  let dicts = $state<DictState | null>(null);
  let reader = $state<{ furigana: 'all' | 'off'; posColors: boolean; fontSize: number }>({ furigana: 'all', posColors: true, fontSize: 30 });
  let stick = $state(true); // seguir el final del historial
  let selId = $state<number | null>(null); // frase elegida en el historial (null = la ultima)
  let scroller: HTMLElement | undefined = $state();
  let latestH = $state(0);
  let link: Link | null = null;

  // palabras
  let known = $state(new Set<string>());
  let filter = $state<'all' | 'new'>('all');
  let openKey = $state<string | null>(null);
  let hoverTok = $state(-1);
  let hoverRow = $state<string | null>(null);
  let wordList: WordList | undefined = $state();
  let ankiDone = $state(new Set<string>());

  // ayuda
  let st = $state<Record<number, LineSt>>({});
  let trShown = $state(false);
  let lost = $state(false);
  let heldT = false;

  // Anki / grabacion
  let anki = $state<AnkiState | null>(null);
  let shots = $state<ShotRef[]>([]);
  let ankiFor = $state<{ lineId: number; word: WordInfo | null } | null>(null);
  let sending = $state(false);
  let ankiErr = $state('');
  let rec = $state<{ state: RecState; secs: number }>({ state: 'idle', secs: 0 });
  let toastMsg = $state<{ msg: string; bad: boolean } | null>(null);
  let toastTimer: number | undefined;

  // glosario
  let gOpen = $state(false);
  let glossary = $state<{ phrases: Phrase[]; dueCount: number }>({ phrases: [], dueCount: 0 });

  // estadisticas de la sesion
  let startedAt = $state(Date.now());
  let now = $state(Date.now());
  let chars = $state(0);
  let sentCount = $state(0);
  const seenIds = new Set<number>();
  // Sesion de lectura segun la app (caracteres japoneses sin repetidas, tiempo activo sin pausas) y ventana de estadisticas.
  let live = $state<ReadLive | null>(null);
  let sOpen = $state(false);
  let readStats = $state<ReadStats | null>(null);

  // Configuracion del texthook: los comandos y eventos de la ventana «Texthook» van por el WebSocket.
  let hOpen = $state(false);
  let hookSeq = 0;
  const hookCalls = new Map<number, { ok: (v: unknown) => void; fail: (e: unknown) => void }>();
  const hookSubs = new Map<string, Set<(p: unknown) => void>>();
  const hookApi: HookApi | null = demo
    ? null // modo demo: la ventana usa su propio backend simulado
    : {
        call<T>(cmd: string, args?: Record<string, unknown>) {
          if (!online) return Promise.reject('Sin conexión con la app');
          const id = ++hookSeq;
          return new Promise<T>((ok, fail) => {
            hookCalls.set(id, { ok: ok as (v: unknown) => void, fail });
            send({ type: 'hook', id, cmd, args: args ?? {} });
          });
        },
        on<T>(event: string, cb: (p: T) => void) {
          const set = hookSubs.get(event) ?? new Set();
          hookSubs.set(event, set);
          const f = cb as (p: unknown) => void;
          set.add(f);
          return Promise.resolve(() => void set.delete(f));
        },
      };

  const latest = $derived(lines.length ? lines[lines.length - 1] : null);
  const current = $derived((selId !== null && lines.find((l) => l.id === selId)) || latest);
  const cur = $derived<LineSt>((current && st[current.id]) || BLANK);
  const hasDict = $derived(!!dicts?.names.length);
  // Altura de la franja de frases: la de la ultima frase; al subir por el historial (o con una anterior elegida) se amplia.
  const expanded = $derived(!stick || selId !== null);
  const stageH = $derived(expanded ? '55vh' : `min(${Math.max(latestH, reader.fontSize * 2.6) + 30}px, 55vh)`);
  const newWordCount = $derived.by(() => {
    const s = new Set<string>();
    for (const l of lines) for (const w of l.words ?? []) if (!known.has(w.base)) s.add(w.base);
    return s.size;
  });
  const minutes = $derived(Math.max(0, Math.floor((now - startedAt) / 60000)));
  // Tiempo activo en pantalla: el de la app mas el tramo en curso mientras no haya pausa larga.
  const activeMs = $derived(live ? live.activeMs + (live.lastMs > 0 && now > live.lastMs && now - live.lastMs <= live.pauseMs ? now - live.lastMs : 0) : 0);
  const speed = $derived(live && activeMs >= 30000 && live.chars > 0 ? Math.round((live.chars * 3600000) / activeMs) : 0);
  // Resaltado de palabra <-> fila: lo que se ve marcado en el texto de la frase actual.
  const hlProp = $derived.by((): number | [number, number] => {
    if (hoverRow?.startsWith('g:')) return current?.grammar?.[Number(hoverRow.slice(2))]?.toks ?? -1;
    if (hoverRow?.startsWith('w:')) return Number(hoverRow.slice(2));
    return hoverTok;
  });

  function setPrefs(p: NovelPrefs | null) {
    prefs = p;
    if (p?.reader) reader = { furigana: p.reader.furigana === 'off' ? 'off' : 'all', posColors: !!p.reader.posColors, fontSize: p.reader.fontSize || 30 };
  }

  function addLine(l: ReaderLine) {
    const i = lines.findIndex((x) => x.id === l.id);
    if (i >= 0) {
      lines[i] = l;
      return;
    }
    lines.push(l);
    if (!seenIds.has(l.id)) {
      seenIds.add(l.id);
      chars += l.text.length;
    }
    if (lines.length > MAX_LINES) lines.splice(0, lines.length - MAX_LINES);
    if (stick) selId = null;
  }

  const lineSt = (id: number): LineSt => (st[id] ??= { ...BLANK });

  function toast(msg: string, bad = false) {
    toastMsg = { msg, bad };
    clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toastMsg = null), 2600);
  }

  function onMsg(m: ReaderMsg) {
    if (m.type === 'hello') {
      game = m.game;
      closed = false;
      setPrefs(m.prefs);
      if (m.dicts) dicts = m.dicts;
      known = new Set(m.known ?? []);
      anki = m.anki ?? anki;
      shots = m.shots ?? [];
      live = m.readLive ?? null;
      lines = [];
      st = {};
      selId = null;
      lost = false;
      trShown = false;
      (m.history ?? []).forEach(addLine);
      stick = true;
      if (m.needHook && m.game) hOpen = true; // primera vez con este juego: hay que elegir el hilo de texto
      send({ type: 'glossaryList', filter: {} }); // para el contador de repasos pendientes
      send({ type: 'ankiTest' }); // el estado guardado puede ser de antes de abrir Anki
    } else if (m.type === 'line') {
      closed = false;
      addLine(m.line);
    } else if (m.type === 'prefs') {
      setPrefs(m.prefs);
    } else if (m.type === 'dicts') {
      dicts = m.dicts;
    } else if (m.type === 'known') {
      known = new Set(m.list);
    } else if (m.type === 'ai') {
      const s = lineSt(m.lineId);
      if (m.kind === 'translate') {
        if (m.error) {
          s.tLoading = false;
          s.tError = m.error;
        } else {
          s.tText = m.text ?? '';
          s.tError = '';
          if (m.done) {
            s.tLoading = false;
            s.translation = m.text ?? s.tText;
          }
        }
      } else if (m.error) {
        s.eLoading = false;
        s.eError = m.error;
      } else if (m.explanation) {
        s.eLoading = false;
        s.eError = '';
        s.explanation = m.explanation;
        s.translation ??= m.explanation.translation;
      }
    } else if (m.type === 'phrase') {
      const s = lineSt(m.lineId);
      s.starred = m.starred;
      s.inAnki = m.inAnki;
      if (m.hasAudio !== s.hasAudio || m.hasSynth !== s.hasSynth) s.audioVer = Date.now();
      s.hasAudio = m.hasAudio;
      s.hasSynth = m.hasSynth;
      if (m.translation) s.translation = m.translation;
      if (m.explanation) s.explanation = m.explanation;
    } else if (m.type === 'anki') {
      anki = m.state;
    } else if (m.type === 'shots') {
      shots = m.shots;
    } else if (m.type === 'rec') {
      rec = { state: m.state, secs: m.secs };
      if (m.state === 'done') {
        toast(`Audio grabado · ${m.secs} s`);
        // Al regrabar, la frase ya tenia audio: sin version nueva el reproductor seguiria con la grabacion anterior.
        if (m.lineId) lineSt(m.lineId).audioVer = Date.now();
      }
    } else if (m.type === 'ankiResult') {
      if (ankiFor && ankiFor.lineId === m.lineId) {
        sending = false;
        if (m.ok) {
          ankiFor = null;
          sentCount++;
          ankiDone.add(m.word ?? '');
          ankiDone = new Set(ankiDone);
          lineSt(m.lineId).inAnki = true;
          toast(m.word ? `«${m.word}» enviada a Anki` : 'Frase enviada a Anki');
        } else ankiErr = m.error ?? 'No se pudo enviar a Anki.';
      }
    } else if (m.type === 'glossary') {
      glossary = { phrases: m.phrases, dueCount: m.dueCount };
    } else if (m.type === 'toast') {
      toast(m.msg, !!m.bad);
    } else if (m.type === 'readLive') {
      live = m.live;
    } else if (m.type === 'stats') {
      readStats = m.stats;
    } else if (m.type === 'hookRes') {
      const c = hookCalls.get(m.id);
      hookCalls.delete(m.id);
      if (m.ok) c?.ok(m.value);
      else c?.fail(m.error ?? 'Error');
    } else if (m.type === 'hookEvt') {
      hookSubs.get(m.event)?.forEach((f) => f(m.payload));
    } else if (m.type === 'bye') {
      closed = true;
    }
  }

  // Color de la burbuja: el del personaje en las preferencias (asi se recolorea al cambiarlo) o el de la linea; gris por defecto.
  function colorOf(l: ReaderLine): string {
    const c = prefs?.characters?.find((c) => c.name === l.who);
    if (c) return c.color ?? l.color ?? GRAY;
    return l.color ?? GRAY;
  }

  // ---- barra de herramientas
  function changed() {
    link?.send({ type: 'prefs', reader: { furigana: reader.furigana, posColors: reader.posColors, fontSize: reader.fontSize } });
    if (prefs) prefs = { ...prefs, reader: { ...reader } };
  }
  const toggleFurigana = () => {
    reader.furigana = reader.furigana === 'all' ? 'off' : 'all';
    changed();
  };
  const toggleColors = () => {
    reader.posColors = !reader.posColors;
    changed();
  };
  const size = (d: number) => {
    reader.fontSize = Math.min(FS_MAX, Math.max(FS_MIN, reader.fontSize + d));
    changed();
  };

  // ---- historial: scroll automatico al final
  // El historial va en `column-reverse`: el navegador ancla el final (scrollTop 0 = abajo del todo, negativo hacia arriba),
  // asi la ultima frase sigue entera aunque la franja cambie de altura o lleguen lineas mientras se anima.
  const atBottom = () => !scroller || Math.abs(scroller.scrollTop) < 24;
  function onScroll() {
    stick = atBottom();
  }
  async function toBottom() {
    await tick();
    scroller?.scrollTo({ top: 0 });
  }
  function goBottom() {
    stick = true;
    selId = null;
    toBottom();
  }
  $effect(() => {
    // Se reejecuta al llegar lineas o cambiar el tamano/furigana (cambia la altura).
    lines.length;
    reader.fontSize;
    reader.furigana;
    stageH;
    if (stick) toBottom();
  });

  // Al cambiar de frase: se tapa la traduccion, se cierra la explicacion y el acordeon.
  let lastId: number | null = null;
  $effect(() => {
    const id = current?.id ?? null;
    if (id !== lastId) {
      lastId = id;
      trShown = false;
      lost = false;
      openKey = null;
      hoverTok = -1;
      hoverRow = null;
      wordList?.scrollTop();
      send({ type: 'current', lineId: selId ?? 0 }); // la grabacion con atajo va a la frase elegida
    }
  });

  /** Mide la ultima frase (`active`) para ajustar la altura de la franja. */
  function measure(el: HTMLElement, active: boolean) {
    let ro: ResizeObserver | undefined;
    const set = (a: boolean) => {
      ro?.disconnect();
      ro = undefined;
      if (!a) return;
      ro = new ResizeObserver(() => (latestH = el.offsetHeight));
      ro.observe(el);
    };
    set(active);
    return { update: set, destroy: () => ro?.disconnect() };
  }

  // ---- palabra del texto <-> fila
  function tokAt(e: Event): number {
    const w = (e.target as HTMLElement | null)?.closest?.('.w') as HTMLElement | null;
    return w?.dataset.i ? Number(w.dataset.i) : -1;
  }
  function pick(l: ReaderLine, e: MouseEvent) {
    if (window.getSelection()?.toString()) return; // seleccionando texto (Yomitan, copiar)
    if (l.id !== current?.id) {
      selId = l.id === latest?.id ? null : l.id;
      return;
    }
    const i = tokAt(e);
    if (i < 0) return;
    const t = l.toks[i];
    const base = t?.base ?? t?.parts.map((p) => p[0]).join('');
    const w = l.words?.find((w) => w.base === base);
    const gi = l.grammar?.findIndex((g) => i >= g.toks[0] && i <= g.toks[1]) ?? -1;
    const key = w ? `w:${w.tok}` : gi >= 0 ? `g:${gi}` : null;
    if (!key) return;
    if (w && known.has(w.base) && filter === 'new') filter = 'all';
    openKey = key;
    tick().then(() => wordList?.reveal(key));
  }

  // ---- acciones
  const send = (m: Record<string, unknown>) => link?.send(m);

  function toggleKnown(base: string) {
    const n = new Set(known);
    n.has(base) ? n.delete(base) : n.add(base);
    known = n; // optimista; la app contesta con la lista buena
    if (openKey && current?.words?.find((w) => `w:${w.tok}` === openKey)?.base === base && n.has(base)) openKey = null;
    toast(n.has(base) ? `${base} marcada como conocida` : `${base} vuelve a la lista`);
    send({ type: 'known', base });
  }

  function requestTranslate() {
    if (!current || cur.translation || cur.tLoading) return;
    const s = lineSt(current.id);
    s.tLoading = true;
    s.tError = '';
    s.tText = '';
    send({ type: 'translate', lineId: current.id });
  }
  function toggleTrans() {
    trShown = !trShown;
    if (trShown) requestTranslate();
  }
  function toggleLost() {
    lost = !lost;
    if (lost && current && !cur.explanation && !cur.eLoading) {
      const s = lineSt(current.id);
      s.eLoading = true;
      s.eError = '';
      send({ type: 'explain', lineId: current.id });
    }
  }
  function toggleStar() {
    if (!current) return;
    const s = lineSt(current.id);
    s.starred = !s.starred;
    send({ type: 'star', lineId: current.id });
    toast(s.starred ? 'Frase guardada en el glosario' : 'Frase quitada de las destacadas');
  }
  function toggleRec() {
    if (current) send({ type: 'record', lineId: rec.state === 'recording' ? undefined : current.id });
  }
  function openAnki(word: WordInfo | null) {
    if (!current) return;
    ankiFor = { lineId: current.id, word };
    if (!anki?.ok) testAnki();
    sending = false;
    ankiErr = '';
  }
  function sendAnki(o: SendOpts) {
    if (!ankiFor) return;
    sending = true;
    ankiErr = '';
    const w = ankiFor.word;
    send({ type: 'anki', lineId: ankiFor.lineId, word: w?.base ?? null, tok: w?.tok ?? null, ...o });
  }
  const testAnki = () => send({ type: 'ankiTest' });

  function onKeyDown(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable)) return;
    if (e.ctrlKey || e.altKey || e.metaKey || ankiFor || hOpen) return;
    const k = e.key.toLowerCase();
    if (k === 't') {
      if (!e.repeat && !trShown) {
        heldT = true;
        trShown = true;
        requestTranslate();
      }
    } else if (k === 'e') {
      if (!e.repeat) toggleLost();
    } else if (k === 'a') {
      if (!e.repeat) openAnki(openKey?.startsWith('w:') ? (current?.words?.find((w) => `w:${w.tok}` === openKey) ?? null) : null);
    }
  }
  function onKeyUp(e: KeyboardEvent) {
    if (e.key.toLowerCase() === 't' && heldT) {
      heldT = false;
      trShown = false;
    }
  }

  const dictCmd = (type: 'openDicts' | 'reloadDicts') => link?.send({ type });

  onMount(() => {
    const tick = window.setInterval(() => (now = Date.now()), 5000);
    if (demo) {
      setPrefs(structuredClone(demoPrefs) as NovelPrefs);
      link = createDemoLink(onMsg, (o) => (online = o));
    } else link = connect(onMsg, (o) => (online = o));
    return () => {
      clearInterval(tick);
      link?.close();
    };
  });

  const initial = (n: string) => Array.from(n.trim())[0] ?? '?';
</script>

<svelte:window onkeydown={onKeyDown} onkeyup={onKeyUp} />

<div class="bg" aria-hidden="true">
  <i class="blob w" style="left:-8%;top:-14%;width:46vmax;height:46vmax"></i>
  <i class="blob a" style="right:-12%;top:8%;width:42vmax;height:42vmax"></i>
  <i class="blob w" style="left:30%;bottom:-24%;width:52vmax;height:52vmax"></i>
  <i class="blob a" style="left:-10%;bottom:-18%;width:34vmax;height:34vmax"></i>
</div>

<header class="top">
  <div class="crumb pill">
    <span class="dot" class:on={online} title={online ? 'Conectado' : 'Sin conexión con la app'}></span>
    <span>Lectura</span>
    {#if game}<span class="sep">›</span><span class="cur jp">{game}</span>{/if}
    {#if !online}<span class="off">Sin conexión…</span>{:else if closed}<span class="off">Juego cerrado</span>{/if}
  </div>
  <span class="sp"></span>
  {#if rec.state !== 'idle'}
    <div class="recpill pill" class:done={rec.state === 'done'}><i></i>{rec.state === 'recording' ? `REC ${mmss(rec.secs)}` : `Grabado · ${rec.secs} s`}</div>
  {/if}
  <button class="ankist pill" class:ok={anki?.ok} class:bad={anki && !anki.ok} onclick={testAnki} title={anki?.ok ? 'AnkiConnect conectado (clic para comprobar)' : (anki?.error ?? 'Sin comprobar') + ' · clic para reintentar'}>
    <i></i>Anki{#if anki?.ok}<b>· {anki.deck}</b>{:else if anki}<b>· sin conexión</b>{/if}
  </button>
  <button class="ankist pill" onclick={() => (gOpen = true)} title="Frases guardadas del juego, repaso con SRS">{@html ICON.book}Glosario{#if glossary.dueCount}<b style="color:var(--gram)">· {glossary.dueCount}</b>{/if}</button>
  <button class="ankist pill" onclick={() => (sOpen = true)} title="Estadísticas de lectura: velocidad, días, palabras nuevas">{@html ICON.chart}Estadísticas</button>
  <button class="ankist pill" onclick={() => (hOpen = true)} title="Configuración del texthook: hilo de texto, filtros, personajes, códigos H">{@html ICON.gear}Texthook</button>
  <div class="tools pill" role="toolbar" aria-label="Opciones de lectura">
    <button class="tl" class:on={reader.furigana === 'all'} onclick={toggleFurigana} aria-pressed={reader.furigana === 'all'}>ふりがな</button>
    <button class="tl" class:on={reader.posColors} onclick={toggleColors} aria-pressed={reader.posColors}>Colores</button>
    <span class="sep"></span>
    <button class="tl" onclick={() => size(-2)} disabled={reader.fontSize <= FS_MIN} aria-label="Reducir texto">A−</button>
    <span class="fs">{reader.fontSize}</span>
    <button class="tl" onclick={() => size(2)} disabled={reader.fontSize >= FS_MAX} aria-label="Aumentar texto">A+</button>
  </div>
</header>

<div class="layout" style="--fs:{reader.fontSize}px">
  <!-- Frases: la ultima abajo del todo de la franja; deslizando hacia arriba, las anteriores -->
  <section class="stage" class:hist={expanded} style="height:{lines.length ? stageH : 'auto'}">
    <main bind:this={scroller} onscroll={onScroll}>
      <div class="log">
        {#each lines as l, i (l.id)}
          {@const last = i === lines.length - 1}
          {@const sel = l.id === current?.id}
          <article class="line" class:latest={last && !closed} class:sel class:narr={!l.who} style={l.who ? `--c:${colorOf(l)}` : ''} use:measure={last}>
            {#if l.who}<div class="ico" aria-hidden="true">{initial(l.who)}</div>{/if}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
            <div class="bubble" onclick={(e) => pick(l, e)}
              onmouseover={(e) => sel && (hoverTok = tokAt(e))} onmouseleave={() => (hoverTok = -1)}>
              {#if l.who}<div class="name">{l.who}</div>{/if}
              <p class="tx jp" class:pos={reader.posColors} lang="ja">{@html lineHtml(l, reader.furigana === 'all', { known, grammar: l.grammar, hl: sel ? hlProp : -1 })}</p>
            </div>
          </article>
        {/each}
        {#if lines.length === 0}
          <div class="empty">
            {#if closed}
              <b>Juego cerrado</b>
              <span>La lectura se reanudará al abrir otra novela.</span>
            {:else}
              <b>Esperando texto…</b>
              <span class="dots"><i></i><i></i><i></i></span>
            {/if}
          </div>
        {:else if closed}
          <div class="closed pill">Juego cerrado</div>
        {/if}
      </div>
    </main>
    {#if expanded && lines.length}
      <button class="goend" onclick={goBottom}>Última frase <span aria-hidden="true">↓</span></button>
    {/if}
  </section>

  {#if current}
    <!-- Ayuda bajo la frase: nada se traduce solo; cada boton pide a la app (y la app cachea en el glosario) -->
    <div class="help">
      <button class="trans" class:show={trShown && !cur.tError} class:err={trShown && !!cur.tError} onclick={toggleTrans}
        title="Clic o mantén T para ver la traducción">
        {#if trShown && cur.tError}
          <span>{cur.tError}</span>
        {:else if trShown && cur.tLoading && !cur.tText}
          <i class="spin"></i><em style="display:inline">Traduciendo…</em>
        {:else}
          <em>Traducción</em>
          <span>{cur.translation ?? (cur.tText || 'Aquí aparece la traducción de la frase')}</span>
        {/if}
      </button>
      <button class="btn lost" class:on={lost} onclick={toggleLost}>{@html ICON.help}Estoy perdido <span class="kbd">E</span></button>
      <button class="btn star" class:on={cur.starred} onclick={toggleStar} title="Guardar la frase en el glosario">{@html cur.starred ? ICON.star : ICON.starO}{cur.starred ? 'Guardada' : 'Guardar'}</button>
      <button class="btn rec" class:on={rec.state === 'recording'} onclick={toggleRec} title="Grabar el audio del juego (también con el atajo global)">
        {@html ICON.mic}{rec.state === 'recording' ? `Parar ${mmss(rec.secs)}` : cur.hasAudio ? 'Regrabar' : 'Grabar'}
      </button>
      <button class="btn anki" onclick={() => openAnki(null)}>{@html ICON.camera}{cur.inAnki ? 'Frase en Anki' : 'Frase a Anki'} <span class="kbd">A</span></button>
    </div>

    {#if lost}
      <div class="explain">
        <div class="eh">Explicación de la frase <span class="ai">IA · sólo cuando la pides</span><span class="sp"></span>
          <button class="ib" onclick={() => (lost = false)} aria-label="Cerrar">✕</button></div>
        {#if cur.explanation}
          <ExplainView e={cur.explanation} />
        {:else if cur.eError}
          <div class="err">{cur.eError}</div>
        {:else}
          <div class="wait"><i class="spin"></i>Analizando la frase…</div>
        {/if}
      </div>
    {/if}
  {/if}

  <!-- Palabras de la frase en acordeon -->
  {#if lines.length}
    <section class="words">
      <div class="wbar">
        <span class="wtitle">{selId !== null ? 'Palabras de la frase elegida' : 'Palabras de la frase'}</span>
        {#if selId !== null}<button class="chip acc" onclick={goBottom}>Volver a la última</button>{/if}
        <span class="sp"></span>
        {#if dicts?.loading}<span class="dinfo">Cargando diccionarios…</span>{/if}
        {#if !demo}
          <button class="chip" onclick={() => dictCmd('openDicts')} title="Carpeta donde van los .zip de Yomitan">Diccionarios</button>
          <button class="chip" onclick={() => dictCmd('reloadDicts')} disabled={dicts?.loading} title="Volver a leer la carpeta">Recargar</button>
        {/if}
        <div class="seg" role="group" aria-label="Filtro de palabras">
          <button class:on={filter === 'all'} onclick={() => (filter = 'all')}>Todas</button>
          <button class:on={filter === 'new'} onclick={() => (filter = 'new')}>Sólo nuevas</button>
        </div>
      </div>
      {#if !hasDict && !dicts?.loading}
        <div class="hint">
          <b>Sin diccionario.</b> Copia en la carpeta <i>Diccionarios</i> el <span class="mono">.zip</span> de un diccionario de Yomitan
          (JMdict en inglés o <span class="mono">jmdict-es</span> en español; también valen los de gramática, frecuencias JPDB y KANJIDIC) y pulsa <i>Recargar</i>.
        </div>
      {/if}
      <WordList bind:this={wordList} line={current} {known} {ankiDone} {dicts} bind:filter bind:openKey {hoverTok} bind:hoverRow onknown={toggleKnown} onanki={openAnki} />
      <div class="foot">
        {#if live}
          <span title="Caracteres japoneses leídos en esta sesión (sin repetidas)"><b>{fmtNum(live.chars)}</b> caracteres</span>
          <span title="Tiempo activo: las pausas largas sin líneas nuevas no cuentan"><b>{fmtDur(activeMs)}</b></span>
          {#if speed}<span><b>{fmtNum(speed)}</b> car/h</span>{/if}
        {:else}
          <span><b>{chars.toLocaleString('es-ES')}</b> caracteres</span>
          <span><b>{minutes} min</b></span>
        {/if}
        <span><b>{newWordCount}</b> palabras nuevas</span>
        <span><b>{sentCount}</b> a Anki</span>
        <span class="sp"></span>
        {#if hasDict}<span title={dicts?.dir}>{dicts?.names.join(' · ')}</span>{/if}
      </div>
    </section>
  {/if}
</div>

{#if ankiFor && current}
  {@const fl = lines.find((l) => l.id === ankiFor!.lineId) ?? current}
  {@const fs = st[fl.id] ?? BLANK}
  <AnkiModal line={fl} word={ankiFor.word} translation={fs.translation} {anki} {shots} hasAudio={fs.hasAudio} hasSynth={fs.hasSynth} audioVer={fs.audioVer}
    rec={rec.state} recSecs={rec.secs} {sending} error={ankiErr} onsend={sendAnki} onclose={() => (ankiFor = null)} onrecord={toggleRec}
    onsynth={() => send({ type: 'synth', lineId: fl.id })} />
{/if}

{#if gOpen}
  <Glossary phrases={glossary.phrases} dueCount={glossary.dueCount} furigana={reader.furigana === 'all'} onclose={() => (gOpen = false)} {send} />
{/if}

{#if sOpen}
  <Stats stats={readStats} onclose={() => (sOpen = false)} {send} />
{/if}

{#if hOpen}
  <HookPanel api={hookApi} onclose={() => (hOpen = false)} />
{/if}

{#if toastMsg}<div class="toast" class:bad={toastMsg.bad}><i>{toastMsg.bad ? '!' : '✓'}</i><span>{toastMsg.msg}</span></div>{/if}
