<script lang="ts">
  // Glosario de frases del juego en la pagina de lectura: buscar, filtrar (★, con audio, en Anki, pendientes), ver una
  // frase con su traduccion, explicacion, captura y audio, editar la nota, enviarla a Anki y repasarla con SRS.
  import { onMount } from 'svelte';
  import type { Phrase, PhraseFilter, ReaderLine } from '../lib/novel-types';
  import { lineHtml } from './render';
  import { demo, ICON } from './util';
  import { demoShot } from './demo';
  import ExplainView from './ExplainView.svelte';

  let { phrases, dueCount, furigana, onclose, send }: {
    phrases: Phrase[]; dueCount: number; furigana: boolean; onclose: () => void; send: (m: Record<string, unknown>) => void;
  } = $props();

  let filter = $state<PhraseFilter>({ query: '', starred: false, audio: false, anki: false, due: false });
  let selId = $state<string | null>(null);
  let confirmDel = $state(false);
  let review = $state<{ queue: string[]; i: number; revealed: boolean } | null>(null);
  let timer: number | undefined;

  const sel = $derived(phrases.find((p) => p.id === selId) ?? null);
  const rv = $derived(review ? (phrases.find((p) => p.id === review!.queue[review!.i]) ?? null) : null);

  function ask() {
    send({ type: 'glossaryList', filter });
  }
  onMount(ask);
  // Las busquedas se piden con un pequeno retardo mientras se escribe.
  function onFilter() {
    clearTimeout(timer);
    timer = window.setTimeout(ask, 160);
  }
  function toggle(k: 'starred' | 'audio' | 'anki' | 'due') {
    filter[k] = !filter[k];
    ask();
  }

  const asLine = (p: Phrase): ReaderLine => ({ id: 0, at: p.createdAt, who: p.who, color: null, text: p.text, toks: p.toks, words: p.words, grammar: p.grammar });
  const mediaUrl = (p: Phrase, kind: 'shot' | 'audio'): string => {
    if (demo) return kind === 'shot' ? demoShot(Number(p.id.replace('demo-', '')) || 1) : '';
    const f = kind === 'shot' ? p.shot : (p.audio ?? p.synth);
    return f ? `/media/${f}` : '';
  };
  const hasAudio = (p: Phrase) => !!(p.audio || p.synth);
  const date = (t: number) => new Date(t).toLocaleDateString('es-ES', { day: 'numeric', month: 'short' });
  const dueNow = (p: Phrase) => p.review.due <= Date.now();

  function pick(p: Phrase) {
    selId = p.id;
    confirmDel = false;
  }
  function saveNote(p: Phrase, e: Event) {
    const note = (e.target as HTMLTextAreaElement).value;
    if (note !== p.note) send({ type: 'phraseUpdate', id: p.id, note, filter });
  }
  function star(p: Phrase) {
    send({ type: 'phraseUpdate', id: p.id, starred: !p.starred, filter });
  }
  function del(p: Phrase) {
    if (!confirmDel) {
      confirmDel = true;
      return;
    }
    send({ type: 'phraseDelete', id: p.id, filter });
    selId = null;
    confirmDel = false;
  }
  const toAnki = (p: Phrase) => send({ type: 'phraseAnki', id: p.id });
  let player: HTMLAudioElement | undefined = $state();
  const play = () => player?.play();

  // ---- repaso
  function startReview() {
    const queue = phrases.filter(dueNow).map((p) => p.id);
    if (!queue.length) return;
    review = { queue, i: 0, revealed: false };
  }
  function grade(g: 0 | 1 | 2 | 3) {
    if (!review || !rv) return;
    send({ type: 'phraseGrade', id: rv.id, grade: g, filter });
    review = { ...review, i: review.i + 1, revealed: false };
  }
  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (e.key === 'Escape') {
      if (review) review = null;
      else onclose();
      e.stopPropagation();
      return;
    }
    if (!review || e.ctrlKey || e.altKey || e.metaKey || t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA') return;
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      if (!review.revealed) review = { ...review, revealed: true };
    } else if (review.revealed && '1234'.includes(e.key) && e.key) grade((Number(e.key) - 1) as 0 | 1 | 2 | 3);
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="veil" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal gmodal" role="dialog" aria-label="Glosario de frases">
    <div class="mh">
      <span class="logo" style="background:linear-gradient(180deg,#a07ae2,var(--gram))">{@html ICON.book}</span>
      <span>Glosario de frases</span>
      <span class="mut" style="font-size:14px;font-weight:700">{phrases.length} {phrases.length === 1 ? 'frase' : 'frases'}</span>
      <span class="sp"></span>
      {#if review}
        <button class="btn" onclick={() => (review = null)}>Salir del repaso</button>
      {:else}
        <button class="btn lost" disabled={!dueCount} onclick={startReview}>Repasar{dueCount ? ` · ${dueCount}` : ''}</button>
      {/if}
      <button class="ib" onclick={onclose} aria-label="Cerrar">✕</button>
    </div>

    {#if review}
      <div class="rvbox">
        {#if rv}
          <div class="rvcount">{review.i + 1} / {review.queue.length}</div>
          {#if mediaUrl(rv, 'shot')}<div class="shot rvshot"><img src={mediaUrl(rv, 'shot')} alt="" /></div>{/if}
          {#if rv.who}<div class="rvwho">{rv.who}</div>{/if}
          <p class="tx jp rvtx" class:pos={false} lang="ja">{@html lineHtml(asLine(rv), furigana, {})}</p>
          {#if !review.revealed}
            <button class="btn anki" onclick={() => review && (review = { ...review, revealed: true })}>Mostrar respuesta <span class="kbd">Espacio</span></button>
          {:else}
            <div class="rvans">
              {#if rv.explanation}<ExplainView e={rv.explanation} />
              {:else if rv.translation}<div class="nat">{rv.translation}</div>
              {:else}<div class="mut">Esta frase no tiene traducción guardada.</div>{/if}
              {#if hasAudio(rv)}<audio bind:this={player} controls autoplay src={mediaUrl(rv, 'audio')}></audio>{/if}
            </div>
            <div class="rvbtns">
              <button class="btn" onclick={() => grade(0)}>Otra vez <span class="kbd">1</span></button>
              <button class="btn" onclick={() => grade(1)}>Difícil <span class="kbd">2</span></button>
              <button class="btn anki" onclick={() => grade(2)}>Bien <span class="kbd">3</span></button>
              <button class="btn" onclick={() => grade(3)}>Fácil <span class="kbd">4</span></button>
            </div>
          {/if}
        {:else}
          <div class="rvdone"><b>¡Repaso terminado!</b><span>No quedan frases pendientes ahora mismo.</span><button class="btn anki" onclick={() => (review = null)}>Volver al glosario</button></div>
        {/if}
      </div>
    {:else}
      <div class="gbar">
        <input class="gsearch" type="search" placeholder="Buscar en las frases, traducciones y notas…" bind:value={filter.query} oninput={onFilter} />
        <div class="seg">
          <button class:on={filter.starred} onclick={() => toggle('starred')}>★ Destacadas</button>
          <button class:on={filter.audio} onclick={() => toggle('audio')}>Con audio</button>
          <button class:on={filter.anki} onclick={() => toggle('anki')}>En Anki</button>
          <button class:on={filter.due} onclick={() => toggle('due')}>Pendientes{dueCount ? ` · ${dueCount}` : ''}</button>
        </div>
      </div>
      <div class="gbody">
        <div class="glist">
          {#each phrases as p (p.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="grow" class:on={selId === p.id} onclick={() => pick(p)}>
              <div class="gtx jp" lang="ja">{#if p.who}<b>{p.who}</b>{/if}{p.text}</div>
              <div class="gtr">{p.translation ?? 'Sin traducir'}</div>
              <div class="gbadges">
                {#if p.starred}<i class="bd star">★</i>{/if}
                {#if hasAudio(p)}<i class="bd">♪</i>{/if}
                {#if p.ankiNoteId !== null}<i class="bd anki">Anki</i>{/if}
                {#if dueNow(p)}<i class="bd due">repasar</i>{/if}
              </div>
            </div>
          {:else}
            <div class="nowords">{filter.query || filter.starred || filter.audio || filter.anki || filter.due ? 'Ninguna frase cumple el filtro.' : 'Aún no hay frases guardadas: se añaden al traducir, explicar, destacar (★), grabar audio o enviar a Anki.'}</div>
          {/each}
        </div>
        <div class="gdet">
          {#if sel}
            <div class="gdh">
              <span class="mut">{sel.who ?? 'Narración'} · vista {sel.seen} {sel.seen === 1 ? 'vez' : 'veces'} · {date(sel.createdAt)}</span>
              <span class="sp"></span>
              <button class="btn star" class:on={sel.starred} onclick={() => star(sel)}>{@html sel.starred ? ICON.star : ICON.starO}{sel.starred ? 'Destacada' : 'Destacar'}</button>
            </div>
            <p class="tx jp gdtx" lang="ja">{@html lineHtml(asLine(sel), furigana, {})}</p>
            {#if mediaUrl(sel, 'shot')}<div class="shot gdshot"><img src={mediaUrl(sel, 'shot')} alt="Captura del juego" /></div>{/if}
            {#if hasAudio(sel)}
              <div class="audrow">{#if mediaUrl(sel, 'audio')}<audio bind:this={player} controls src={mediaUrl(sel, 'audio')}></audio>{:else}<span>Audio guardado (demo)</span>{/if}
                <span class="mut">{sel.audio ? 'Grabado del juego' : 'Voz sintética'}</span></div>
            {/if}
            {#if sel.explanation}<div class="explain gdex"><ExplainView e={sel.explanation} /></div>
            {:else if sel.translation}<div class="nat gdnat">{sel.translation}</div>
            {:else}<div class="mut" style="margin:8px 0">Sin traducción guardada: pídela desde la frase en la lectura.</div>{/if}
            <label class="field">Nota
              <textarea class="gnote" rows="2" placeholder="Tu nota sobre la frase…" value={sel.note} onblur={(e) => saveNote(sel, e)}></textarea>
            </label>
            <div class="bacts">
              <button class="btn anki" onclick={() => toAnki(sel)}>{@html ICON.anki}{sel.ankiNoteId !== null ? 'Enviar otra vez a Anki' : 'Enviar a Anki'}</button>
              <button class="btn" style="color:var(--bad)" onclick={() => del(sel)}>{confirmDel ? 'Pulsa otra vez para borrar' : 'Borrar'}</button>
            </div>
          {:else}
            <div class="gempty">Elige una frase de la lista para ver su traducción, explicación, captura y audio.</div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
