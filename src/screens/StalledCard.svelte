<script lang="ts">
  import { app, LIST_COLOR } from '../lib/store.svelte';
  import { closeOverlay } from '../lib/controller.svelte';
  import { STALLED_LABEL, stalledApply } from '../lib/milista.svelte';
  import Hints from '../components/Hints.svelte';

  const s = $derived(app.stalled);
  const n = $derived(s.rows.length);
  // Un color por opción (pendiente, abandonado, dejarlo): son los de los estados en LIST_COLOR.
  const CHOICES = [LIST_COLOR.pending, LIST_COLOR.dropped, LIST_COLOR.none];
  const inkOf = (a: { dark: boolean }) => (a.dark ? '#3a2f00' : '#fff');
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
  <div class="glass menupanel stalledpanel" style="--acc:{LIST_COLOR.playing.c};--acc-ink:{inkOf(LIST_COLOR.playing)}">
    <div class="face">
      <h2>{n === 1 ? 'Un juego parado' : `${n} juegos parados`}</h2>
      <p class="msub">Siguen en «Jugando», pero llevan más de {app.config.listStalledDays} días sin jugarse. ¿Qué hacemos?</p>
      <div class="slist" style="height:{Math.min(VIS, n + 1) * RH}px">
        <div class="strk" style="transform:translateY({-off * RH}px)">
          {#each s.rows as r, i (r.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="srow" class:f={s.row === i} onclick={() => (s.row = i)}>
              <span class="nm">{r.name}<small>hace {days(r.days)}</small></span>
              <span class="opts">
                {#each STALLED_LABEL as l, k}
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <span class="tflt" class:on={r.choice === k} style="--acc:{CHOICES[k].c};--acc-ink:{inkOf(CHOICES[k])}" onclick={(e) => { e.stopPropagation(); s.row = i; r.choice = k; }}>{l}</span>
                {/each}
              </span>
            </div>
          {/each}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="srow apply" class:f={s.row === n} onclick={() => void stalledApply()}><span class="btn a">A</span>Aplicar</div>
        </div>
      </div>
    </div>
  </div>
  <Hints list={[['←→', 'Elegir'], ['↑↓', 'Juego'], ['A', s.row >= n ? 'Aplicar' : 'Siguiente', 'a'], ['B', 'Más tarde', 'b']]} />
</div>

<style>
  .stalledpanel{left:220px;width:840px;top:110px;height:480px}
  .slist{position:relative;overflow:hidden;flex:none}
  .strk{transition:transform .22s cubic-bezier(.25,1,.4,1)}
  .stalledpanel .srow{margin-bottom:4px;padding:0 16px 0 20px}
  .nm{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .nm small{margin-left:10px;font-size:14px;font-weight:600;color:#6f7787}
  .srow.f .nm small{color:inherit;opacity:.85}
  .opts{display:flex;gap:6px;flex:none}
  .opts .tflt{cursor:pointer;text-shadow:none}
  .apply{justify-content:center;font-weight:800;color:#3a414e}
  .apply.f{color:var(--acc-ink)}
</style>
