<script lang="ts">
  // Ventana de confirmacion para enviar una palabra o la frase a Anki: vista previa anverso/reverso, captura del
  // juego (las 3 ultimas), audio (grabado / sintetico / ninguno), mazo y tipo de nota.
  import type { AnkiState, ReaderLine, RecState, ShotRef, WordInfo } from '../lib/novel-types';
  import { sentenceWithWord } from './render';
  import { ICON, audioUrl, mmss, shotUrl, showReading } from './util';

  export interface SendOpts { shotLineId: number | null; audio: 'rec' | 'synth' | 'none'; deck: string; model: string }

  let {
    line, word, translation, anki, shots, hasAudio, hasSynth, audioVer, rec, recSecs, sending, error, onsend, onclose, onrecord, onsynth,
  }: {
    line: ReaderLine;
    word: WordInfo | null;
    translation: string | null;
    anki: AnkiState | null;
    shots: ShotRef[];
    hasAudio: boolean;
    hasSynth: boolean;
    audioVer: number;
    rec: RecState;
    recSecs: number;
    sending: boolean;
    error: string;
    onsend: (o: SendOpts) => void;
    onclose: () => void;
    onrecord: () => void;
    onsynth: () => void;
  } = $props();

  const thumbs = $derived(shots.slice(0, 3));
  let shotSel = $state<number | null>(null);
  let audio = $state<'rec' | 'synth' | 'none'>('none');
  let deck = $state('');
  let model = $state('');
  let ready = false;

  // Valores iniciales (una sola vez al abrir): captura de esta frase, audio grabado si lo hay, mazo y modelo de Anki.
  $effect.pre(() => {
    if (ready) return;
    ready = true;
    shotSel = shots.find((s) => s.lineId === line.id)?.lineId ?? shots[0]?.lineId ?? null;
    audio = hasAudio ? 'rec' : 'none';
    deck = anki?.deck ?? '';
    model = anki?.model ?? '';
  });

  // Si Anki contesta despues de abrir (o se reconecta), se rellenan el mazo y el tipo de nota que falten.
  $effect(() => {
    if (!anki?.ok) return;
    if (!deck) deck = anki.deck;
    if (!model || !anki.models.includes(model)) model = anki.model;
  });

  const r = $derived(word?.defs[0]?.reading ?? '');
  const newWords = $derived((line.words ?? []).slice(0, 6));
  const recording = $derived(rec === 'recording');

  function pickAudio(a: 'rec' | 'synth' | 'none') {
    audio = a;
    if (a === 'synth' && !hasSynth) onsynth();
  }
  const send = () => onsend({ shotLineId: shotSel, audio, deck, model });
  let shotOk = $state<Record<number, boolean>>({});
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="veil" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-label="Enviar a Anki">
    <div class="mh">
      <span class="logo">★</span>
      <span>{word ? `Enviar «${word.base}» a Anki` : 'Enviar la frase a Anki'}</span>
      <span class="sp"></span>
      <button class="ib" onclick={onclose} aria-label="Cerrar">✕</button>
    </div>
    <div class="mgrid">
      <div class="cardpv">
        <div class="side">
          <div class="lbl">Anverso</div>
          {#if word}
            <div class="big jp" lang="ja">{word.base}</div>
            <div class="sent" lang="ja">{@html sentenceWithWord(line, word.tok)}</div>
          {:else}
            <div class="sent" lang="ja" style="font-size:24px">{@html sentenceWithWord(line, null)}</div>
          {/if}
        </div>
        <div class="side">
          <div class="lbl">Reverso</div>
          <div class="back">
            {#if word}
              <div class="jp" lang="ja" style="font-size:20px;font-weight:700">{word.base}{#if showReading(word.base, r)}【{r}】{/if}</div>
              {#if word.defs[0]}
                <ol style="margin:4px 0 0;padding-left:20px">{#each word.defs[0].senses.slice(0, 4) as s}<li>{s.glosses.join('; ')}</li>{/each}</ol>
              {:else}<span class="mut">Sin entrada en el diccionario.</span>{/if}
            {:else}
              <div style="font-weight:700">{translation ?? 'Sin traducción (pídela antes con «Traducción» si la quieres en la tarjeta).'}</div>
              <div class="mut" style="font-size:13px;margin-top:4px">
                {#each newWords as w}<span class="jp" lang="ja">{w.base}【{w.defs[0]?.reading ?? ''}】</span> {w.defs[0]?.senses[0]?.glosses[0] ?? ''} · {/each}
              </div>
            {/if}
          </div>
          {#if shotSel !== null}<div class="shot" style="margin-top:10px"><img src={shotUrl(shotSel)} alt="" onerror={() => (shotOk[shotSel!] = false)} /><span class="badge">Captura</span></div>{/if}
        </div>
      </div>

      <div>
        <div class="lbl">Captura del juego</div>
        <div class="shot">
          {#if shotSel !== null && shotOk[shotSel] !== false}<img src={shotUrl(shotSel)} alt="Captura del juego" />{:else}<span class="noshot">Sin captura</span>{/if}
        </div>
        {#if thumbs.length}
          <div class="thumbs">
            {#each thumbs as s (s.lineId)}
              <button class:on={shotSel === s.lineId} title={`Hace ${Math.max(1, Math.round((Date.now() - s.at) / 1000))} s`} onclick={() => (shotSel = s.lineId)} aria-label="Elegir captura">
                <img src={shotUrl(s.lineId)} alt="" />
              </button>
            {/each}
          </div>
        {/if}

        <label class="field">Mazo
          <select bind:value={deck} disabled={!anki?.ok}>
            {#each anki?.decks ?? [] as d}<option value={d}>{d}</option>{/each}
            {#if anki && !anki.decks.includes(anki.deck)}<option value={anki.deck}>{anki.deck}</option>{/if}
          </select>
        </label>
        <label class="field">Tipo de nota
          <select bind:value={model} disabled={!anki?.ok}>
            {#each anki?.models ?? [] as m}<option value={m}>{m}</option>{/each}
            {#if anki && !anki.models.includes(anki.model)}<option value={anki.model}>{anki.model}</option>{/if}
          </select>
        </label>

        <div class="field">Audio de la frase
          <div class="audsel">
            <div class="seg">
              <button class:on={audio === 'rec'} disabled={!hasAudio} onclick={() => pickAudio('rec')}>Grabado</button>
              <button class:on={audio === 'synth'} onclick={() => pickAudio('synth')}>Sintético</button>
              <button class:on={audio === 'none'} onclick={() => pickAudio('none')}>Ninguno</button>
            </div>
          </div>
        </div>
        <div class="audrow">
          {#if audio === 'rec' || audio === 'none'}
            <button class="btn" class:rec={true} class:on={recording} onclick={onrecord}>
              {@html ICON.mic}{recording ? `Parar · ${mmss(recSecs)}` : hasAudio ? 'Volver a grabar' : 'Grabar'}
            </button>
            {#if hasAudio && !recording}
              {#if audioUrl(line.id, 'rec')}<audio controls src={audioUrl(line.id, 'rec', audioVer)}></audio>{:else}<span>Audio grabado (demo)</span>{/if}
            {/if}
          {:else if hasSynth}
            {#if audioUrl(line.id, 'synth')}<audio controls src={audioUrl(line.id, 'synth', audioVer)}></audio>{:else}<span>Voz sintética lista (demo)</span>{/if}
          {:else}
            <span class="spin"></span><span>Generando la voz sintética…</span>
          {/if}
        </div>
      </div>
    </div>
    <div class="mf">
      {#if error}<span class="merr">{error}</span>{:else if anki && !anki.ok}<span class="merr">{anki.error ?? 'Anki no responde. Ábrelo con AnkiConnect instalado.'}</span>{/if}
      <button class="btn" onclick={onclose}>Cancelar</button>
      <button class="btn anki" disabled={sending || !anki?.ok || recording} onclick={send}>
        {#if sending}<span class="spin"></span>Enviando…{:else}Enviar a Anki{/if}
      </button>
    </div>
  </div>
</div>
