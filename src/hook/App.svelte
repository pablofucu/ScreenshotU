<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getApi, IN_TAURI, type HookApi } from './api';
  import { recommend } from './score';
  import type { NovelPrefs, ProcInfo, ReaderLine, SearchHit, ThStatus, ThreadInfo } from '../lib/novel-types';

  // `api`: backend ya preparado (la pagina de lectura lo pasa por su WebSocket); sin el, Tauri o el simulado.
  // `onclose`: va incrustada como panel en la pagina de lectura y se puede cerrar.
  let { api: given = null, onclose = null }: { api?: HookApi | null; onclose?: (() => void) | null } = $props();

  const PALETTE = ['#00A0D8', '#E8384F', '#3BB54A', '#F2B807', '#3F63E0', '#F28A1E', '#9B59D0', '#FF4F7B', '#1FB5A5', '#7A5230'];
  const GRAY = '#8a919c';
  const CODEPAGES: [number, string][] = [[0, 'Automática'], [1200, 'UTF-16'], [932, 'Shift-JIS'], [65001, 'UTF-8']];
  type Tab = 'filters' | 'chars' | 'codes' | 'search' | 'proc';
  const TABS: [Tab, string][] = [['filters', 'Filtros'], ['chars', 'Personajes'], ['codes', 'Código H'], ['search', 'Buscar hook'], ['proc', 'Proceso']];

  type FilterKey = Exclude<keyof NovelPrefs['filters'], 'regex'>;
  // Los «HOOK_去除重复…» y demás filtros de LunaTranslator, en el orden en que se aplican.
  const FILTER_OPTS: [FilterKey, string, string][] = [
    ['dedupeChars', 'Quitar caracteres repetidos', '明明日日 → 明日 (detecta cuántas veces se repiten)'],
    ['dedupePhrase', 'Quitar frase repetida', 'ABCABC → ABC (el hook saca la frase dos veces)'],
    ['dedupeSegments', 'Quitar trozos repetidos', 'S1S1S2S2 → S1S2'],
    ['dedupeIncremental', 'Quitar texto creciente', 'AABABCABCD → ABCD (se dibuja letra a letra en una salida)'],
    ['dedupeLines', 'Quitar líneas repetidas', 'Si el juego redibuja la misma línea'],
    ['joinLines', 'Unir saltos de línea', 'Una frase por línea'],
    ['removeControl', 'Quitar caracteres de control', 'Basura invisible del hook'],
    ['removeTags', 'Quitar etiquetas <…>', '<color=red>雪</color> → 雪'],
    ['removeBraces', 'Quitar llaves {…}', '{漢字/かんじ} → 漢字'],
    ['splitName', 'Separar nombre y diálogo', '美雪「…」 → nombre en color'],
  ];

  let api: HookApi | null = null;
  let proc = $state<ProcInfo | null>(null);
  let threads = $state<ThreadInfo[]>([]);
  let texts = $state<Record<string, string>>({});
  let selected = $state<string | null>(null);
  let nameThread = $state<string | null>(null);
  let gameId = $state<string | null>(null);
  let gameName = $state<string | null>(null);
  let prefs = $state<NovelPrefs | null>(null);
  let lines = $state<ReaderLine[]>([]);
  let logs = $state<string[]>([]);
  let query = $state('');
  let tab = $state<Tab>('filters');
  let err = $state('');
  let savedAt = $state(0);
  let copied = $state(false);

  // buscar hook / código H
  let searchText = $state('');
  let codepage = $state(0);
  let deep = $state(false);
  let searching = $state(false);
  let hits = $state<SearchHit[]>([]);
  let searchDone = $state<number | null>(null);
  let codeInput = $state('');
  let logEl = $state<HTMLElement | null>(null);
  let histEl = $state<HTMLElement | null>(null);

  const shown = (t: ThreadInfo) => texts[t.key] ?? t.last;
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return threads;
    return threads.filter(t => (t.name + ' ' + t.code + ' ' + shown(t)).toLowerCase().includes(q));
  });
  const current = $derived(threads.find(t => t.key === selected) ?? null);
  const recent = $derived(lines.slice(-6));

  function setErr(e: unknown) { err = String(e); setTimeout(() => { if (err === String(e)) err = ''; }, 5000); }
  async function run<T = void>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
    try { return await api!.call<T>(cmd, args); } catch (e) { setErr(e); return undefined; }
  }

  // ---- prefs con debounce ----
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  function flushSave() {
    if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
    if (!prefs || !gameId) return Promise.resolve();
    return run('novel_save_prefs', { gameId, prefs: $state.snapshot(prefs) });
  }
  function queueSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => { saveTimer = null; void flushSave(); }, 400);
  }
  async function saveHook() {
    if (!prefs) return;
    prefs.threadKey = selected;
    prefs.nameThreadKey = nameThread;
    await flushSave();
    savedAt = Date.now();
  }

  // ---- hilos ----
  function selectThread(key: string) { selected = key; void run('th_select_thread', { key }); }
  function toggleName(key: string) { nameThread = nameThread === key ? null : key; void run('th_select_name_thread', { key: nameThread }); }
  function removeHook(key: string) { void run('th_remove_hook', { key }); }
  function clearThreads() { threads = threads.filter(t => t.live || t.key === selected || t.key === nameThread); }
  async function copyCode(code: string) {
    try { await navigator.clipboard.writeText(code); copied = true; setTimeout(() => (copied = false), 1500); } catch (e) { setErr(e); }
  }

  // ---- códigos y búsqueda ----
  async function addCode(code: string) {
    code = code.trim();
    if (!code || !prefs) return;
    try {
      await api!.call('th_insert_code', { code });
      if (!prefs.hookCodes.includes(code)) { prefs.hookCodes.push(code); queueSave(); }
      codeInput = '';
    } catch (e) { setErr(e); }
  }
  function removeCode(c: string) { if (prefs) { prefs.hookCodes = prefs.hookCodes.filter(x => x !== c); queueSave(); } }
  async function startSearch() {
    if (!searchText.trim()) return;
    hits = []; searchDone = null; searching = true;
    const r = await run('th_search_text', { text: searchText.trim(), codepage, deep });
    if (r === undefined && err) searching = false;
  }
  function cancelSearch() { void run('th_search_cancel'); searching = false; }

  // ---- personajes ----
  function setColor(i: number, color: string | null) {
    if (!prefs) return;
    prefs.characters[i].color = color;
    queueSave();
  }
  function setHex(i: number, v: string) {
    v = v.trim();
    if (v === '') return setColor(i, null);
    if (!v.startsWith('#')) v = '#' + v;
    if (/^#[0-9a-fA-F]{6}$/.test(v)) setColor(i, v.toUpperCase());
  }
  const charColor = (name: string | null, fallback: string | null) =>
    (name && prefs?.characters.find(c => c.name === name)?.color) || fallback || GRAY;

  // ---- reglas regex ----
  function addRule() { prefs?.filters.regex.push({ pattern: '', replace: '', on: true }); queueSave(); }
  function delRule(i: number) { prefs?.filters.regex.splice(i, 1); queueSave(); }

  function upsertThread(t: ThreadInfo) {
    const i = threads.findIndex(x => x.key === t.key);
    if (i >= 0) threads[i] = t; else threads.push(t);
  }

  $effect(() => { lines.length; selected; if (histEl) void tick().then(() => { if (histEl) histEl.scrollTop = histEl.scrollHeight; }); });
  $effect(() => { logs.length; if (tab === 'proc' && logEl) void tick().then(() => { if (logEl) logEl.scrollTop = logEl.scrollHeight; }); });

  onMount(() => {
    const offs: (() => void)[] = [];
    let flush = () => { void flushSave(); };
    (async () => {
      api = given ?? (await getApi());
      const a = api;
      offs.push(
        await a.on<ProcInfo>('th-proc', p => (proc = p)),
        await a.on<ThreadInfo>('th-thread', upsertThread),
        await a.on<{ key: string }>('th-thread-removed', ({ key }) => {
          threads = threads.filter(t => t.key !== key);
          if (selected === key) selected = null;
          if (nameThread === key) nameThread = null;
        }),
        await a.on<{ key: string; text: string }>('th-text', ({ key, text }) => (texts[key] = text)),
        await a.on<ReaderLine>('th-line', l => {
          lines.push(l);
          if (lines.length > 40) lines.splice(0, lines.length - 40);
          if (l.who && prefs && !prefs.characters.some(c => c.name === l.who)) prefs.characters.push({ name: l.who, color: l.color });
        }),
        await a.on<SearchHit>('th-search', h => { if (!hits.some(x => x.code === h.code)) hits.push(h); }),
        await a.on<{ count: number }>('th-search-done', ({ count }) => { searching = false; searchDone = count; }),
        await a.on<string>('th-log', m => { logs.push(m); if (logs.length > 200) logs.splice(0, logs.length - 200); }),
      );
      const st = await run<ThStatus>('th_status');
      if (st) {
        proc = st.proc; selected = st.selected; nameThread = st.nameThread; gameId = st.gameId; gameName = st.gameName;
        for (const t of st.threads) upsertThread(t);
      }
      if (st?.gameId) {
        const p = await run<NovelPrefs>('novel_get_prefs', { gameId: st.gameId });
        if (p) prefs = p;
      }
    })();
    window.addEventListener('beforeunload', flush);
    return () => { offs.forEach(f => f()); window.removeEventListener('beforeunload', flush); };
  });
</script>

<div class="win">
  <div class="bg"><div class="blob w"></div><div class="blob a"></div></div>

  <!-- Barra superior -->
  <header class="top">
    <div class="glass avatar"><div class="face"><b class="jp">あ</b></div></div>
    <div class="crumb pill">Texthook <span class="sep">›</span> <span class="cur jp">{gameName ?? 'Sin juego'}</span></div>
    {#if !IN_TAURI && !given}<span class="badge a">Modo demo</span>{/if}
    <span class="sp"></span>
    <div class="proc pill">
      {#if proc}
        <span class="dot" class:off={!proc.connected}></span>{proc.exe} <small>· {proc.arch} bits · PID {proc.pid}{proc.connected ? '' : ' · desconectado'}</small>
      {:else}
        <span class="dot off"></span>Esperando al juego…
      {/if}
    </div>
    {#if onclose}<button class="ub" onclick={() => { void flushSave(); onclose?.(); }} aria-label="Cerrar" title="Cerrar (Esc)">✕</button>{/if}
  </header>

  <div class="main">
    <!-- Hilos -->
    <section class="panel">
      <div class="ptitle">Hilos de texto <span class="badge">{threads.length}</span><span class="sp"></span>
        <button class="ub sm" onclick={clearThreads} title="Quita de la lista los hilos sin actividad">Limpiar</button></div>
      <input class="inp" style="margin-bottom:10px;flex:none" placeholder="Filtrar hilos por texto o motor…" bind:value={query} />
      <div class="scroll">
        {#each filtered as t (t.key)}
          {@const r = recommend({ ...t, last: shown(t) })}
          <div class="thr" class:on={t.key === selected} class:dim={t.lines === 0 && !shown(t)} role="button" tabindex="0"
            onclick={() => selectThread(t.key)} onkeydown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectThread(t.key); } }}
            oncontextmenu={e => { e.preventDefault(); toggleName(t.key); }}>
            <i class="live" class:on={t.live}></i>
            <span class="nm"><span>{t.name}</span><span class="mono">{t.code}</span></span>
            <span class="meta">
              {#if t.key === nameThread}<span class="badge a">Nombre</span>{/if}
              {#if r.level !== 'none'}<span class="badge" class:g={r.level === 'good'} class:a={r.level === 'maybe'} class:r={r.level === 'bad'} title="{r.why} ({r.score}/100)">{r.label}</span>{/if}
              <span class="badge">{t.lines.toLocaleString('es-ES')}</span>
            </span>
            <span class="pv jp">{shown(t) || '—'}</span>
            <span class="acts">
              <button class="ub xs" onclick={e => { e.stopPropagation(); toggleName(t.key); }}>{t.key === nameThread ? 'Quitar hilo de nombre' : 'Usar como hilo de nombre'}</button>
              <button class="ub xs danger" onclick={e => { e.stopPropagation(); removeHook(t.key); }}>Quitar hook</button>
            </span>
          </div>
        {:else}
          <div class="empty">{threads.length ? 'Ningún hilo coincide con el filtro.' : 'Aún no hay hilos. Se irán añadiendo cuando el juego muestre texto.'}</div>
        {/each}
      </div>
    </section>

    <div class="right">
      <!-- Hilo elegido -->
      <section class="panel">
        {#if current}
          <div class="ptitle"><span class="big-title">{current.name} <small class="mono">{current.code}</small></span><span class="sp"></span>
            <button class="ub sm" onclick={() => copyCode(current.code)}>{copied ? '¡Copiado!' : 'Copiar código'}</button>
            <span class="ub sm acc" style="cursor:default">✓ En uso</span></div>
          <div class="scroll" bind:this={histEl}>
            <div class="hist jp">
              {#each recent as l, i (l.id)}
                <p class:last={i === recent.length - 1}>
                  {#if l.who}<span class="who" style="color:{charColor(l.who, l.color)}">{l.who}</span>{/if}{l.who ? '「' : ''}{l.text}{l.who ? '」' : ''}
                </p>
              {:else}
                <p style="color:#9aa2ae">Esperando líneas de este hilo…</p>
              {/each}
            </div>
          </div>
        {:else}
          <div class="ptitle">Vista previa</div>
          <div class="guide"><b>Elige un hilo de la lista</b>Avanza un par de diálogos en el juego y elige el hilo con el texto. Los marcados como «Recomendado» suelen ser el bueno.</div>
        {/if}
      </section>

      <!-- Herramientas -->
      <section class="panel">
        <div class="seg">
          {#each TABS as [id, label]}<button class:on={tab === id} onclick={() => (tab = id)}>{label}</button>{/each}
        </div>
        {#if err}<p class="err">{err}</p>{/if}

        {#if tab === 'filters'}
          {#if prefs}
            <div class="scroll">
              <div class="fgrid">
                {#each FILTER_OPTS as [key, t, d]}
                  <div class="opt"><button class="tg" class:on={prefs.filters[key]} aria-label={t} aria-pressed={prefs.filters[key]}
                    onclick={() => { prefs!.filters[key] = !prefs!.filters[key]; queueSave(); }}></button><div>{t}<small>{d}</small></div></div>
                {/each}
              </div>
              <div class="rules">
                <div class="ptitle" style="margin-bottom:8px"><span class="badge a">Regex</span>Reglas de reemplazo<span class="sp"></span><button class="ub sm" onclick={addRule}>+ Añadir</button></div>
                {#each prefs.filters.regex as r, i}
                  <div class="rule">
                    <button class="tg" class:on={r.on} aria-label="Activar regla" aria-pressed={r.on} onclick={() => { r.on = !r.on; queueSave(); }}></button>
                    <input class="inp mono" placeholder="patrón" bind:value={r.pattern} oninput={queueSave} />
                    <span class="arrow">→</span>
                    <input class="inp mono" placeholder="(vacío)" bind:value={r.replace} oninput={queueSave} />
                    <button class="ub sm danger" onclick={() => delRule(i)} aria-label="Quitar regla">✕</button>
                  </div>
                {:else}<div class="empty">Sin reglas. Ejemplo: <span class="mono">\[[^\]]*\]</span> → (vacío) quita lo que va entre corchetes.</div>{/each}
              </div>
            </div>
          {:else}<div class="empty">Cargando preferencias…</div>{/if}

        {:else if tab === 'chars'}
          <div class="scroll">
            {#if prefs && prefs.characters.length}
              {#each prefs.characters as c, i (c.name)}
                <div class="chr">
                  <span><span class="bub jp" style="--c:{c.color ?? GRAY}">{c.name}</span></span>
                  <div class="swatches">
                    <button class="sw gray" class:on={!c.color} title="Gris (por defecto)" aria-label="Gris (por defecto)" style="--c:{GRAY}" onclick={() => setColor(i, null)}></button>
                    {#each PALETTE as p}<button class="sw" class:on={c.color?.toUpperCase() === p} style="--c:{p}" aria-label={p} onclick={() => setColor(i, p)}></button>{/each}
                    <input class="inp mono" placeholder="#RRGGBB" value={c.color ?? ''} maxlength="7" onchange={e => setHex(i, e.currentTarget.value)} />
                  </div>
                </div>
              {/each}
            {:else}<div class="empty">Todavía no se ha detectado ningún personaje. Aparecerán al avanzar diálogos con nombre en el hilo elegido.</div>{/if}
          </div>

        {:else if tab === 'codes'}
          <div class="row" style="flex:none">
            <input class="inp mono" placeholder="Código H, p. ej. HQ-14*@44FA10" bind:value={codeInput} onkeydown={e => e.key === 'Enter' && addCode(codeInput)} />
            <button class="ub acc" onclick={() => addCode(codeInput)}>Añadir</button>
          </div>
          <div class="scroll">
            {#each prefs?.hookCodes ?? [] as c}
              <div class="item"><span class="mono">{c}</span>
                <button class="ub xs" onclick={() => copyCode(c)}>Copiar</button>
                <button class="ub xs danger" onclick={() => removeCode(c)}>Quitar</button></div>
            {:else}<div class="empty">No hay códigos guardados para este juego. Los que añadas aquí o desde la búsqueda se insertarán automáticamente al abrirlo.</div>{/each}
          </div>

        {:else if tab === 'search'}
          <div class="srch" style="flex:none">
            <div><p class="lbl">Texto que sale en pantalla</p><input class="inp jp-in" placeholder="Escribe una frase que se vea ahora en el juego" bind:value={searchText} onkeydown={e => e.key === 'Enter' && !searching && startSearch()} /></div>
            <div><p class="lbl">Codificación</p><select class="inp" bind:value={codepage}>{#each CODEPAGES as [v, n]}<option value={v}>{n}</option>{/each}</select></div>
            <div class="opt" style="padding-top:20px"><button class="tg" class:on={deep} aria-label="Búsqueda profunda" aria-pressed={deep} onclick={() => (deep = !deep)}></button>Búsqueda profunda</div>
            <div style="padding-top:20px">{#if searching}<button class="ub danger" onclick={cancelSearch}>Cancelar</button>{:else}<button class="ub acc" disabled={!searchText.trim()} onclick={startSearch}>Buscar</button>{/if}</div>
          </div>
          <div class="scroll">
            {#if searching && !hits.length}<div class="empty">Buscando… mantén el texto visible en el juego.</div>{/if}
            {#each hits as h (h.code)}
              <div class="hit"><span class="mono">{h.code}</span><span class="pv jp">{h.text}</span>
                <button class="ub sm act" onclick={() => addCode(h.code)} disabled={prefs?.hookCodes.includes(h.code)}>{prefs?.hookCodes.includes(h.code) ? 'Añadido' : 'Añadir'}</button></div>
            {/each}
            {#if searchDone !== null && !searching}<div class="empty">{searchDone ? `Búsqueda terminada: ${searchDone} resultado${searchDone === 1 ? '' : 's'}.` : 'Sin resultados. Prueba con otra codificación o la búsqueda profunda.'}</div>{/if}
          </div>

        {:else}
          <div class="procbar">
            <button class="ub" onclick={() => run('th_reinject')}>Reinyectar</button>
            <button class="ub danger" onclick={() => run('th_detach')}>Desconectar</button>
            <span class="sp"></span><span class="badge">{logs.length} entradas</span>
            <button class="ub sm" onclick={() => (logs = [])}>Vaciar consola</button>
          </div>
          <div class="log" bind:this={logEl}>{logs.length ? logs.join('\n') : 'La consola de LunaHook aparecerá aquí.'}</div>
        {/if}
      </section>
    </div>
  </div>

  <!-- Barra inferior -->
  <footer class="bottom pill">
    <span class="out"><button class="tg" class:on={prefs?.out.reader} disabled={!prefs} aria-label="Página de lectura" aria-pressed={!!prefs?.out.reader}
      onclick={() => { prefs!.out.reader = !prefs!.out.reader; queueSave(); }}></button>Página de lectura <small>localhost:6677</small></span>
    <span class="out"><button class="tg" class:on={prefs?.out.clipboard} disabled={!prefs} aria-label="Portapapeles" aria-pressed={!!prefs?.out.clipboard}
      onclick={() => { prefs!.out.clipboard = !prefs!.out.clipboard; queueSave(); }}></button>Portapapeles</span>
    <span class="sp"></span>
    {#key savedAt}{#if savedAt}<span class="saved">✓ Guardado</span>{/if}{/key}
    {#if !onclose}<button class="ub sm" onclick={() => run('novel_open_reader')}>Abrir lectura</button>{/if}
    <button class="ub sm acc" disabled={!prefs || !gameId} onclick={saveHook}>Guardar hook para este juego</button>
  </footer>
</div>
