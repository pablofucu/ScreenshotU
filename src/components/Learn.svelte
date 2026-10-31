<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { cancelLearn, comboLabel } from '../lib/learn.svelte';

  const l = $derived(app.learn!);
  const TITLE = { pad: 'Pulsa en el mando', mouse: 'Pulsa un botón del ratón', key: 'Pulsa la tecla' } as const;
  const SUB = {
    pad: 'Un botón o varios a la vez. Se guarda al soltarlos todos.',
    mouse: 'El botón central (rueda) o uno de los laterales.',
    key: 'Sola (F1–F24, Impr Pant, Pausa…) o con Ctrl, Alt o Shift.',
  } as const;
  const ic = $derived(l.kind === 'pad' ? ICON.pad : l.kind === 'mouse' ? ICON.mouse : ICON.key);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim learnscrim" onclick={(e) => { e.stopPropagation(); if (e.target === e.currentTarget) cancelLearn(); }}>
  <div class="glass menupanel learnpanel">
    <div class="face">
      <div class="lic">{@html ic}</div>
      <h2>{TITLE[l.kind]}</h2>
      <p class="msub">{SUB[l.kind]}</p>
      {#if l.kind === 'pad'}
        <div class="lcombo" class:on={l.combo !== 'off'}>{l.combo !== 'off' ? comboLabel(l.combo) : 'Esperando…'}</div>
        {#if !l.connected}
          <p class="lwarn">No se detecta ningún mando Xbox (XInput). Los mandos de PlayStation o Switch necesitan Steam Input, DS4Windows o similar.</p>
        {/if}
      {:else}
        <div class="lcombo">Esperando…</div>
      {/if}
      <div class="lfoot">
        <span class="lleft">{l.left} s</span>
        <button type="button" class="lcancel" onclick={(e) => { e.stopPropagation(); cancelLearn(); }}>Cancelar (Esc)</button>
      </div>
    </div>
  </div>
</div>

<style>
  .learnscrim { z-index: 40; }
  .learnpanel { top: 200px; min-height: 320px; left: 390px; width: 500px; }
  .learnpanel .face { align-items: center; text-align: center; gap: 6px; }
  .lic {
    width: 52px; height: 52px; border-radius: 16px; display: grid; place-items: center; margin-bottom: 4px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 30%, white), var(--acc));
    animation: lpulse 1.4s ease-in-out infinite;
  }
  .lic :global(svg) { width: 60%; height: 60%; }
  .learnpanel .msub { height: auto; white-space: normal; }
  .lcombo {
    margin-top: 6px; min-width: 220px; padding: 12px 24px; border-radius: 18px; font-size: 24px; font-weight: 800;
    color: var(--ink-3); background: var(--glass-in); box-shadow: inset 0 0 0 2px var(--line);
  }
  .lcombo.on { color: var(--acc-ink); background: var(--acc); box-shadow: none; }
  .lwarn { margin: 4px 0 0; font-size: 13px; font-weight: 600; color: var(--bad); line-height: 1.3; }
  .lfoot { margin-top: auto; width: 100%; display: flex; align-items: center; justify-content: space-between; }
  .lleft { font-size: 15px; font-weight: 700; color: var(--ink-3); }
  .lcancel {
    font: 700 15px var(--round); color: var(--ink-2); background: var(--glass-in); border: 0; border-radius: 14px;
    padding: 8px 16px; cursor: pointer;
  }
  .lcancel:hover { background: var(--glass-chip); }
  @keyframes lpulse { 50% { transform: scale(1.08); } }
</style>
