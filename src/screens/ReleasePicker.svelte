<script lang="ts">
  import { z } from '../lib/format';
  // Selector de la fecha de lanzamiento (mismo aspecto que la fecha de «Terminado» en la barra de estado).
  import { app } from '../lib/store.svelte';
  import { popOverlay } from '../lib/controller.svelte';
  import { fmtRelease, MONTH_SHORT, rel, releaseClear, releaseField, releaseSave, releaseStep } from '../lib/release.svelte';
  import Hints from '../components/Hints.svelte';

  const t = $derived(app.platform?.titles[rel.title]);
  const boxes = $derived([
    [0, rel.d ? z(rel.d) : '—', 'Día'],
    [1, rel.m ? `${z(rel.m)} · ${MONTH_SHORT[rel.m - 1]}` : '—', 'Mes'],
    [2, String(rel.y), 'Año'],
  ] as Array<[number, string, string]>);
  const saved = $derived(t?.release ? `Guardada: ${fmtRelease(t.release)}${t.release.auto ? ' (de SteamGridDB)' : ''}` : 'Sin fecha guardada');
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) popOverlay(); }}>
  <div class="glass modal rpanel">
    <div class="face">
      <h2>Fecha de lanzamiento</h2>
      <p class="msub">{t?.name ?? ''}</p>
      <div class="frow">
        {#each boxes as [f, v, lab]}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="fcol">
            <div class="fbox" class:f={rel.field === f} class:off={f === 0 && !rel.m} class:wide={f === 1} onclick={() => releaseField(f)}>
              <button class="inc" tabindex="-1" onclick={(e) => { e.stopPropagation(); releaseStep(f, 1); }}>▲</button>
              <b>{v}</b>
              <button class="dec" tabindex="-1" onclick={(e) => { e.stopPropagation(); releaseStep(f, -1); }}>▼</button>
            </div>
            <small>{lab}{f !== 2 ? ' (opcional)' : ''}</small>
          </div>
        {/each}
      </div>
      <p class="rhint">{saved}. El año basta; mes y día son opcionales («—» = sin poner).</p>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="fbtns">
        <button type="button" class="b-main" onclick={releaseSave}>Guardar</button>
        <button type="button" class="b-soft pill" disabled={!t?.release} onclick={releaseClear}>Quitar fecha</button>
      </div>
    </div>
  </div>
  <Hints list={[['←→', 'Campo'], ['↑↓', 'Valor'], ['LB/RB', '±10 años'], ['A', 'Guardar', 'a'], ['X', 'Quitar fecha', 'x'], ['B', 'Cancelar', 'b']]} />
</div>

<style>
  .rpanel{left:240px;width:800px;top:150px;height:360px}
  .msub{margin-bottom:var(--s-3)}
  .frow{display:flex;justify-content:center;gap:var(--s-4);margin-top:var(--s-1)}
  .fcol{display:flex;flex-direction:column;align-items:center;gap:var(--s-2)}
  .fcol small{font:700 var(--t-s)/1 var(--round);color:var(--ink-3)}
  .fbox.wide{min-width:190px}
  .fbox.off{opacity:.5}
  .rhint{margin:var(--s-3) var(--s-1) 0;font:600 var(--t-m)/1.35 var(--round);color:var(--ink-2);text-align:center}
</style>
