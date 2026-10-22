<script lang="ts">
  import { app, LIST_COLOR } from '../lib/store.svelte';
  import { closeOverlay } from '../lib/controller.svelte';
  import { STALLED_LABEL, stalledApply } from '../lib/milista.svelte';
  import Hints from '../components/Hints.svelte';

  const s = $derived(app.stalled);
  const n = $derived(s.rows.length);
  const COLORS = [LIST_COLOR.pending.c, LIST_COLOR.dropped.c, '#8a93a3'];
  // Lista con desplazamiento: la fila enfocada siempre queda dentro de las VIS visibles (la última es «Aplicar»).
  const VIS = 5, RH = 58;
  let off = $state(0);
  $effect(() => {
    const i = s.row;
    let o = Math.min(off, Math.max(0, n + 1 - VIS));
    if (i < o) o = i;
    else if (i >= o + VIS) o = i - VIS + 1;
    if (o !== off) off = o;
  });
  const days = (d: number) => (d >= 60 ? `${Math.round(d / 30)} meses` : `${d} días`);
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="glass stalledpanel" style="--acc:{LIST_COLOR.playing.c}">
    <div class="face">
      <h2>{n === 1 ? 'Un juego parado' : `${n} juegos parados`}</h2>
      <p class="msub">Siguen en «Jugando», pero llevan más de {app.config.listStalledDays} días sin jugarse. ¿Qué hacemos?</p>
      <div class="slist" style="height:{Math.min(VIS, n + 1) * RH}px">
        <div class="strk" style="transform:translateY({-off * RH}px)">
          {#each s.rows as r, i (r.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="srow2" class:f={s.row === i} onclick={() => (s.row = i)}>
              <span class="nm">{r.name}<small>hace {days(r.days)}</small></span>
              <span class="opts">
                {#each STALLED_LABEL as l, k}
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <span class="opt" class:on={r.choice === k} style="--c:{COLORS[k]}" onclick={(e) => { e.stopPropagation(); s.row = i; r.choice = k; }}>{l}</span>
                {/each}
              </span>
            </div>
          {/each}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="srow2 apply" class:f={s.row === n} onclick={() => void stalledApply()}><span class="btn a">A</span>Aplicar</div>
        </div>
      </div>
    </div>
  </div>
  <Hints list={[['←→', 'Elegir'], ['↑↓', 'Juego'], ['A', s.row >= n ? 'Aplicar' : 'Siguiente', 'a'], ['B', 'Más tarde', 'b']]} />
</div>

<style>
  .stalledpanel{left:220px;width:840px;top:110px;height:480px;--r:36px;--bw:14px;cursor:default}
  .stalledpanel::before{opacity:.3}
  .stalledpanel::after{opacity:.5}
  .stalledpanel .face{background:rgba(247,249,252,.95);padding:22px 30px 18px;display:flex;flex-direction:column;gap:6px;color:#3a414e}
  h2{margin:0;font-size:28px;font-weight:800}
  .msub{margin:0 0 10px;font-size:16px;font-weight:600;color:#5f6776}
  .slist{position:relative;overflow:hidden;flex:none}
  .strk{transition:transform .22s cubic-bezier(.25,1,.4,1)}
  .srow2{height:54px;margin-bottom:4px;padding:0 16px 0 20px;border-radius:18px;display:flex;align-items:center;gap:14px;font-size:19px;font-weight:700;color:#4a5260;cursor:pointer;box-sizing:border-box}
  .srow2.f{background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(232,238,246,.9));box-shadow:inset 0 1px 0 #fff,0 0 0 2px var(--acc),0 6px 14px -8px var(--acc)}
  .nm{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .nm small{margin-left:10px;font-size:14px;font-weight:600;color:#7a8394}
  .opts{display:flex;gap:6px;flex:none}
  .opt{height:34px;padding:0 14px;border-radius:999px;display:flex;align-items:center;font-size:15px;font-weight:800;color:#5f6776;background:rgba(255,255,255,.8);box-shadow:inset 0 0 0 1px rgba(95,110,135,.25)}
  .opt.on{color:#fff;text-shadow:0 1px 2px rgba(0,0,0,.25);background:linear-gradient(180deg,color-mix(in srgb,var(--c) 70%,white),var(--c));box-shadow:inset 0 1px 0 rgba(255,255,255,.7),0 0 0 1px color-mix(in srgb,var(--c) 75%,black)}
  .apply{justify-content:center;font-weight:800;color:#3a414e}
  .apply.f{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 80%,white),var(--acc));color:#fff;text-shadow:0 1px 2px rgba(0,0,0,.25)}
</style>
