<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { ADD_ITEMS, addAct } from '../lib/ops.svelte';
  import { popOverlay } from '../lib/controller.svelte';
  import Hints from '../components/Hints.svelte';

  const INFO: Record<(typeof ADD_ITEMS)[number], { l: string; s: string; ic: string }> = {
    exe: { l: 'Ejecutable (.exe)', s: 'Un juego de PC', ic: ICON.pad },
    emu: { l: 'Emulador + ROM', s: 'Switch y otras consolas', ic: ICON.play },
    dir: { l: 'Carpeta con muchos juegos', s: 'Un asistente repasa cada subcarpeta', ic: ICON.folder },
    captures: { l: 'Solo capturas', s: 'Sin lanzador', ic: ICON.camera },
  };
  const H = 28 + 100 + ADD_ITEMS.length * 64 + 18;

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; app.addRow = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) popOverlay(); }}>
  <div class="glass menupanel addpanel" style="--acc:{app.chrome.c};top:{(720 - H) / 2}px;height:{H}px">
    <div class="face">
      <h2>Añadir juego</h2>
      <p class="msub">¿De dónde viene?</p>
      {#each ADD_ITEMS as it, i}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="srow mrow addrow" class:f={app.addRow === i} data-add={it} onmousemove={(e) => hover(e, i)} onclick={() => { app.addRow = i; addAct(i); }}>
          <span class="mic">{@html INFO[it].ic}</span>
          <span class="sl">{INFO[it].l}<small>{INFO[it].s}</small></span>
        </div>
      {/each}
    </div>
  </div>
  <Hints list={[['A', 'Elegir', 'a'], ['B', 'Cerrar', 'b']]} />
</div>
