<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { ADD_ITEMS, addAct } from '../lib/ops.svelte';
  import { popOverlay } from '../lib/controller.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const INFO: Record<(typeof ADD_ITEMS)[number], { l: string; s: string; ic: string }> = {
    exe: { l: 'Ejecutable (.exe)', s: 'Un juego de PC', ic: ICON.pad },
    emu: { l: 'Emulador + ROM', s: 'Switch y otras consolas', ic: ICON.play },
    dir: { l: 'Carpeta con muchos juegos', s: 'Un asistente repasa cada subcarpeta', ic: ICON.folder },
    captures: { l: 'Solo capturas', s: 'Sin lanzador', ic: ICON.camera },
  };
  const H = 28 + 100 + ADD_ITEMS.length * 64 + 18;

  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { app.addRow = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) popOverlay(); }}>
  <div class="glass menupanel addpanel" style="top:{(720 - H) / 2}px;height:{H}px">
    <div class="face">
      <h2>Añadir juego</h2>
      <p class="msub">¿De dónde viene?</p>
      {#each ADD_ITEMS as it, i}
        <button type="button" class="srow mrow addrow" class:f={app.addRow === i} data-add={it} onmousemove={(e) => hover(e, i)} onclick={() => { app.addRow = i; addAct(i); }}>
          <span class="mic">{@html INFO[it].ic}</span>
          <span class="sl">{INFO[it].l}<small>{INFO[it].s}</small></span>
        </button>
      {/each}
    </div>
  </div>
  <Hints list={[['A', 'Elegir', 'a'], ['B', 'Cerrar', 'b']]} />
</div>
