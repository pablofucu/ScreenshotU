<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { FILTER_LABEL, closePhrases, mediaUrl, ph, phrasesCmd, pickRow, readSummary } from '../lib/phrases.svelte';
  import { lineHtml } from '../reader/render';
  import type { Phrase, ReaderLine } from '../lib/novel-types';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const ROW_H = 74;
  const VIS = 6;
  const off = $derived(Math.max(0, Math.min(ph.row - 2, ph.items.length - VIS)));
  const summary = $derived(ph.stats ? readSummary() : null);
  const sel = $derived(ph.items[ph.row]);
  const card = $derived(ph.queue[ph.qi]);
  const done = $derived(ph.mode === 'review' && ph.qi >= ph.queue.length);
  const asLine = (p: Phrase): ReaderLine => ({ id: 0, at: 0, who: p.who, color: null, text: p.text, toks: p.toks, words: [], grammar: [] });
  const dueNow = (p: Phrase) => p.review.due <= Date.now();
  const hints = $derived<Array<[string, string, string?]>>(
    ph.mode === 'list'
      ? [['A', 'Repasar', 'a'], ['X', 'Destacar', 'x'], ['Y', FILTER_LABEL[ph.filter], 'y'], ['LT', 'Audio', 'lt'], ['B', 'Atrás', 'b']]
      : done ? [['A', 'Volver', 'a']]
      : ph.revealed ? [['X', 'Otra vez', 'x'], ['LB', 'Difícil', 'lb'], ['A', 'Bien', 'a'], ['Y', 'Fácil', 'y'], ['RB', 'Audio', 'rb'], ['B', 'Salir', 'b']]
      : [['A', 'Mostrar', 'a'], ['Y', 'Furigana', 'y'], ['RB', 'Audio', 'rb'], ['B', 'Salir', 'b']],
  );
  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { pickRow(i); }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closePhrases(); }}>
  <div class="glass setpanel phrpanel" style="--acc:{app.chrome.c}">
    <div class="face">
      {#if ph.mode === 'list'}
        <h2>Frases{ph.gameName ? ` · ${ph.gameName}` : ''}</h2>
        <p class="msub">{ph.items.length} {FILTER_LABEL[ph.filter].toLowerCase()} · {ph.due} para repasar. A empieza el repaso; Y cambia el filtro.</p>
        {#if summary}<p class="msub phread">Lectura: {summary}</p>{/if}
        {#if ph.items.length === 0}
          <p class="pickempty">{ph.filter === 'all' ? 'Aún no hay frases guardadas. Se añaden desde la página de lectura al traducir, explicar, destacar, grabar audio o enviar a Anki.' : 'Ninguna frase cumple este filtro (Y lo cambia).'}</p>
        {:else}
          <div class="phbody">
            <div class="phlist" style="height:{VIS * ROW_H}px">
              <div class="strack" style="transform:translateY({-off * ROW_H}px)">
                {#each ph.items as p, i (p.id)}
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <div class="srow phrow" class:f={ph.row === i} onmousemove={(e) => hover(e, i)} onclick={() => { pickRow(i); phrasesCmd('a'); }}>
                    <span class="phtx"><b class="jp">{p.who ? p.who + ' ' : ''}{p.text}</b><small>{p.translation ?? 'Sin traducir'}</small></span>
                    <span class="phbd">{p.starred ? '★' : ''}{p.audio || p.synth ? ' ♪' : ''}{p.ankiNoteId !== null ? ' Anki' : ''}{dueNow(p) ? ' ·' : ''}</span>
                  </div>
                {/each}
              </div>
            </div>
            <div class="phdet">
              {#if sel}
                {#if sel.shot && mediaUrl(sel.shot)}<img class="phshot" src={mediaUrl(sel.shot)} alt="" />{/if}
                <p class="phbig jp" lang="ja">{@html lineHtml(asLine(sel), ph.furi, {})}</p>
                <p class="phtr">{sel.translation ?? 'Sin traducción guardada.'}</p>
                {#if sel.note}<p class="phnote">{sel.note}</p>{/if}
                <p class="phmeta">Vista {sel.seen} {sel.seen === 1 ? 'vez' : 'veces'} · {dueNow(sel) ? 'toca repasar' : 'repaso en ' + Math.max(1, Math.ceil((sel.review.due - Date.now()) / 864e5)) + ' d'}</p>
              {/if}
            </div>
          </div>
        {/if}
      {:else if done}
        <h2>¡Repaso terminado!</h2>
        <p class="msub">Has repasado {ph.queue.length} {ph.queue.length === 1 ? 'frase' : 'frases'}. Quedan {ph.due} pendientes.</p>
      {:else if card}
        <h2>Repaso · {ph.qi + 1} / {ph.queue.length}</h2>
        <p class="msub">{card.gameName}{card.who ? ' · ' + card.who : ''}</p>
        <div class="rvcard">
          {#if card.shot && mediaUrl(card.shot)}<img class="phshot" src={mediaUrl(card.shot)} alt="" />{/if}
          <p class="phbig jp" lang="ja">{@html lineHtml(asLine(card), ph.furi, {})}</p>
          {#if ph.revealed}
            <p class="phtr">{card.explanation?.translation ?? card.translation ?? 'Sin traducción guardada.'}</p>
            {#if card.explanation?.literal}<p class="phmeta">{card.explanation.literal}</p>{/if}
            {#if card.explanation?.notes?.length}<p class="phmeta">{card.explanation.notes[0].replace(/\*\*/g, '')}</p>{/if}
          {:else}
            <p class="phmeta">Pulsa A para ver la traducción.</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
  <Hints list={hints} />
</div>

<style>
  .phrpanel { left: 90px; top: 50px; width: 1100px; height: 610px; }
  .phread { margin-top: -4px; color: var(--ok); }
  .phbody { display: grid; grid-template-columns: 1fr 1fr; gap: 22px; margin-top: 8px; }
  .phlist { overflow: hidden; position: relative; }
  .phrow { height: 68px; margin: 3px 0; }
  .phtx { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .phtx b { font-size: 19px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .phtx small { font-size: 14px; font-weight: 600; opacity: 0.75; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .phbd { font-size: 14px; font-weight: 800; white-space: nowrap; opacity: 0.85; }
  .phdet { display: flex; flex-direction: column; gap: 10px; overflow-y: auto; overflow-x: hidden; scrollbar-width: none; }
  .phdet::-webkit-scrollbar { display: none; }
  .phshot { width: 100%; max-height: 220px; object-fit: cover; border-radius: 16px; box-shadow: 0 0 0 1px var(--line); }
  .phbig { font-family: var(--jp, 'Yu Gothic UI', sans-serif); font-size: 30px; line-height: 1.7; margin: 0; color: var(--ink); }
  .phbig :global(rt) { font-size: 0.45em; color: var(--ink-3); }
  .phtr { font-size: 19px; font-weight: 700; margin: 0; color: var(--ink); }
  .phnote { font-size: 15px; margin: 0; padding: 8px 12px; border-radius: 12px; background: var(--glass-in); }
  .phmeta { font-size: 14px; font-weight: 600; margin: 0; color: var(--ink-3); }
  .rvcard { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; text-align: center; overflow: hidden; }
  .rvcard .phshot { max-width: 420px; }
</style>
