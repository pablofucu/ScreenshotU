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
  <div class="glass rpanel" style="--acc:{app.chrome.c}">
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
        <button type="button" class="fsave" onclick={releaseSave}><span class="btn a">A</span>Guardar</button>
        <button type="button" class="fsave alt" class:dis={!t?.release} onclick={releaseClear}><span class="btn x">X</span>Quitar fecha</button>
      </div>
    </div>
  </div>
  <Hints list={[['←→', 'Campo'], ['↑↓', 'Valor'], ['LB/RB', '±10 años'], ['A', 'Guardar', 'a'], ['X', 'Quitar fecha', 'x'], ['B', 'Cancelar', 'b']]} />
</div>

<style>
  .rpanel{left:240px;width:800px;top:150px;height:360px;--r:36px;--bw:14px;cursor:default}
  .rpanel::before{opacity:.3}
  .rpanel::after{opacity:.5}
  .rpanel .face{background:rgba(247,249,252,.95);padding:20px 30px 16px;display:flex;flex-direction:column;gap:6px;color:#3a414e}
  h2{margin:0;font-size:28px;font-weight:800}
  .msub{margin:0 0 8px;font-size:16px;font-weight:600;color:#5f6776;height:22px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .frow{display:flex;justify-content:center;gap:16px;margin-top:4px}
  .fcol{display:flex;flex-direction:column;align-items:center;gap:6px}
  .fcol small{font-size:14px;font-weight:700;color:#7a8291}
  .fbox{position:relative;min-width:150px;height:76px;border-radius:18px;display:flex;align-items:center;justify-content:center;gap:10px;cursor:pointer;
    background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(235,239,245,.85));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.25);transition:background .15s,transform .15s,opacity .15s}
  .fbox.wide{min-width:190px}
  .fbox.off{opacity:.5}
  .fbox b{font-size:30px;font-weight:800;color:#3a414e}
  .fbox button{position:absolute;left:50%;transform:translateX(-50%);border:0;background:none;font:800 12px var(--round);color:#8a93a3;cursor:pointer;padding:0 14px;line-height:1}
  .fbox .inc{top:6px}
  .fbox .dec{bottom:6px}
  .fbox.f{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 80%,white),var(--acc));box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--acc) 75%,black),0 8px 16px -8px var(--acc);transform:scale(1.04)}
  .fbox.f b,.fbox.f button{color:#fff;text-shadow:0 1px 2px rgba(0,0,0,.25)}
  .rhint{margin:10px 4px 0;font-size:15px;font-weight:600;color:#5f6776;text-align:center}
  .fbtns{margin-top:auto;margin-bottom:2px;align-self:center;display:flex;gap:14px}
  .fsave{display:flex;align-items:center;gap:10px;height:44px;padding:0 22px 0 8px;border-radius:999px;cursor:pointer;font-size:18px;font-weight:800;color:#fff;
    background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 80%,white),var(--acc));box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--acc) 75%,black)}
  .fsave.alt{background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(235,239,245,.85));color:#4a5260;box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.3)}
  .fsave.dis{opacity:.5}
</style>
