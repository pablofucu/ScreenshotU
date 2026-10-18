<script lang="ts">
  // Panel de palabras en acordeon: una fila por palabra (con su primera acepcion); clic y se despliega.
  // Grupos: nuevas, «Gramática» y «Conocidas» (atenuadas). Resaltado fila <-> palabra del texto.
  import type { DictState, GrammarHit, ReaderLine, WordInfo } from '../lib/novel-types';
  import { sentenceWithWord } from './render';
  import { ICON, POS_NAME, freqLabel, oneLine, showReading } from './util';

  let {
    line, known, ankiDone, dicts, filter = $bindable('all'), openKey = $bindable(null), hoverTok, hoverRow = $bindable(null),
    onknown, onanki,
  }: {
    line: ReaderLine | null;
    known: Set<string>;
    ankiDone: Set<string>;
    dicts: DictState | null;
    filter: 'all' | 'new';
    openKey: string | null;
    hoverTok: number;
    hoverRow: string | null;
    onknown: (base: string) => void;
    onanki: (w: WordInfo) => void;
  } = $props();

  const words = $derived<WordInfo[]>(line?.words ?? []);
  const fresh = $derived(words.filter((w) => !known.has(w.base)));
  const knownW = $derived(words.filter((w) => known.has(w.base)));
  const grams = $derived<GrammarHit[]>(line?.grammar ?? []);
  const hasDict = $derived(!!dicts?.names.length);
  let listEl: HTMLElement | undefined = $state();

  const wKey = (w: WordInfo) => `w:${w.tok}`;
  const gKey = (i: number) => `g:${i}`;
  const toggle = (k: string) => (openKey = openKey === k ? null : k);

  /** Desplaza la fila hasta verse y la hace destellar. */
  export function reveal(key: string) {
    const el = listEl?.querySelector<HTMLElement>(`[data-key="${key}"]`);
    if (!el) return;
    el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    el.classList.remove('flash');
    void el.offsetWidth;
    el.classList.add('flash');
  }
  export function scrollTop() {
    listEl?.scrollTo({ top: 0 });
  }
</script>

<div class="list" bind:this={listEl}>
  {#snippet wordItem(w: WordInfo)}
    {@const key = wKey(w)}
    {@const k = known.has(w.base)}
    {@const done = ankiDone.has(w.base)}
    {@const r = w.defs[0]?.reading ?? ''}
    {@const fl = w.freq ? freqLabel(w.freq) : null}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="it" class:open={openKey === key} class:known={k} class:hl={hoverRow === key || hoverTok === w.tok} data-key={key}
      onmouseenter={() => (hoverRow = key)} onmouseleave={() => (hoverRow = null)}>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="row" onclick={() => toggle(key)}>
        <span class="term jp" lang="ja">{#if showReading(w.base, r)}<ruby>{w.base}<rt>{r}</rt></ruby>{:else}{w.base}{/if}</span>
        <span class="pt {w.pos}">{POS_NAME[w.pos] ?? w.pos}</span>
        <span class="one">{w.defs.length ? oneLine(w) : hasDict ? 'Sin entrada en el diccionario' : dicts?.loading ? 'Cargando diccionarios…' : 'Añade un diccionario para ver las acepciones'}</span>
        {#if w.surface !== w.base}<span class="form jp" lang="ja" title="Forma en la frase">{w.surface}</span>{/if}
        {#if fl && w.freq}<span class="freq {fl[1]}" title="Puesto {w.freq.toLocaleString('es-ES')} en frecuencia">{fl[0]}</span>{/if}
        <span class="acts">
          <button class="ib k" class:on={k} title={k ? 'Quitar de conocidas' : 'Ya la conozco'}
            onclick={(e) => { e.stopPropagation(); onknown(w.base); }}>{@html ICON.check}</button>
          <button class="ib ak" class:done title={done ? 'Ya está en Anki' : 'Enviar a Anki con captura'}
            onclick={(e) => { e.stopPropagation(); onanki(w); }}>{@html ICON.anki}</button>
        </span>
        <span class="chev">›</span>
      </div>
      {#if openKey === key}
        <div class="body">
          {#each w.defs as d, di}
            {#if di > 0 || d.term !== w.base}
              <div class="alt jp" lang="ja">{d.term}{#if showReading(d.term, d.reading)}<span class="rd">【{d.reading}】</span>{/if}{#if dicts && dicts.names.length > 1}<small>{d.dict}</small>{/if}</div>
            {/if}
            <ol>
              {#each d.senses as s}
                <li>{#each s.tags as t}<span class="tag">{t}</span>{/each}{s.glosses.join('; ')}</li>
              {/each}
            </ol>
          {:else}
            <p class="nodef">{hasDict ? 'Sin entrada en el diccionario.' : 'Añade un diccionario para ver las acepciones.'}</p>
          {/each}
          {#if w.kanji?.length}
            <div class="kj">
              {#each w.kanji as kj}
                <div><b class="jp" lang="ja">{kj.ch}</b><span>{kj.meanings.slice(0, 3).join('; ')}<br /><span class="mut jp">{[...kj.onyomi, ...kj.kunyomi].join(' · ')}</span></span></div>
              {/each}
            </div>
          {/if}
          {#if line}<div class="ex" lang="ja"><span class="mut">En esta frase · </span>{@html sentenceWithWord(line, w.tok)}</div>{/if}
          <div class="bacts">
            <button class="btn anki" onclick={() => onanki(w)}>{@html ICON.anki} {done ? 'Ya en Anki · enviar otra vez' : 'Enviar a Anki con captura'}</button>
            <button class="btn" onclick={() => onknown(w.base)}>{k ? 'Quitar de conocidas' : '✓ Ya la conozco'}</button>
          </div>
        </div>
      {/if}
    </div>
  {/snippet}

  {#each fresh as w (w.tok)}{@render wordItem(w)}{/each}

  {#if grams.length}
    <div class="grp">Gramática</div>
    {#each grams as g, i (i)}
      {@const key = gKey(i)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="it gram" class:open={openKey === key} class:hl={hoverRow === key || (hoverTok >= g.toks[0] && hoverTok <= g.toks[1])} data-key={key}
        onmouseenter={() => (hoverRow = key)} onmouseleave={() => (hoverRow = null)}>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="row" onclick={() => toggle(key)}>
          <span class="term jp" lang="ja">{g.term}</span>
          <span class="pt g">Gramática</span>
          <span class="one">{g.defs[0]?.senses[0]?.glosses.join('; ') ?? ''}</span>
          <span class="chev">›</span>
        </div>
        {#if openKey === key}
          <div class="body">
            {#each g.defs as d}
              <ol>{#each d.senses as s}<li>{#each s.tags as t}<span class="tag">{t}</span>{/each}{s.glosses.join('; ')}</li>{/each}</ol>
            {/each}
            {#if line}<div class="ex" lang="ja"><span class="mut">En esta frase · </span>{@html sentenceWithWord(line, g.toks)}</div>{/if}
          </div>
        {/if}
      </div>
    {/each}
  {/if}

  {#if knownW.length}
    {#if filter === 'all'}
      <div class="grp">Conocidas · {knownW.length}</div>
      {#each knownW as w (w.tok)}{@render wordItem(w)}{/each}
    {:else}
      <div class="grp">{knownW.length} conocida{knownW.length > 1 ? 's' : ''} oculta{knownW.length > 1 ? 's' : ''} <button onclick={() => (filter = 'all')}>Mostrar</button></div>
    {/if}
  {/if}

  {#if !words.length && !grams.length}
    <div class="nowords">Esta frase no tiene palabras que buscar.</div>
  {/if}
</div>
