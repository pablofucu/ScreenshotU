<script lang="ts">
  // Caja inferior derecha del detalle: deuda pendiente de días libres abiertos y sus plazos.
  import { groupOf, type GoalTrack } from '../../lib/readgoals';
  import { fmtMin, freeDayName, freeLeft } from '../../lib/readfmt';
  import IconoRacha from './IconoRacha.svelte';

  let { track }: { track: GoalTrack } = $props();

  const isPlay = $derived(groupOf(track.key) === 'play');
  const totPaid = $derived(track.open.reduce((a, q) => a + q.paid, 0));
  const totNeed = $derived(track.open.reduce((a, q) => a + q.need, 0));
  const pct = $derived(totNeed > 0 ? Math.min(100, Math.max(0, (totPaid / totNeed) * 100)).toFixed(1) : '0');
  const urgent = $derived(track.open.length > 0 && track.open[0].left <= 2);
</script>

<div class="box gin">
  {#if track.open.length > 0}
    <div class="t">
      Por recuperar<small>{track.open.length > 1 ? `${track.open.length} días libres` : `del ${freeDayName(track.open[0].day, true)}`}</small>
    </div>
    <div class="owe">
      <span class="big" class:r={urgent}>{fmtMin(track.owe, true)}</span>
      <span class="oline">pendientes · llevas <b>{fmtMin(totPaid, true)}</b> de {fmtMin(totNeed, true)}</span>
    </div>
    <div class="obar"><i style="width:{pct}%"></i></div>
    <div class="oline">
      {#each track.open as q, idx (q.day)}
        {#if idx > 0} · {/if}Hasta el <b>{freeDayName(q.deadline, false)}</b> ({freeLeft(q)})
      {/each}. Cuenta lo que {isPlay ? 'juegues' : 'leas'} de más.
    </div>
  {:else}
    <div class="t">Por recuperar</div>
    <div class="okbig">
      <span class="ok"><IconoRacha name="check" size={24} stroke="var(--on)" /></span>
      Nada pendiente
    </div>
    <div class="oline" style="margin-top:8px">
      {isPlay ? 'Jugar' : 'Leer'} de más no se guarda por adelantado: solo paga un día libre que ya hayas usado.
    </div>
  {/if}
</div>

<style>
  .gin {
    background: var(--glass-in);
    border-radius: var(--r-m);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .box {
    position: absolute;
    left: 708px;
    top: 392px;
    width: 482px;
    height: 136px;
    padding: 14px 22px 18px;
    box-sizing: border-box;
    animation: rise 0.4s cubic-bezier(0.2, 0.8, 0.2, 1) 0.2s both;
  }
  .t {
    font-size: var(--t-l);
    font-weight: 800;
    color: var(--ink);
  }
  .t small {
    font-size: var(--t-s);
    color: var(--ink-2);
    font-weight: 600;
    margin-left: 6px;
  }
  .owe {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-top: 2px;
    line-height: 1;
  }
  .owe .big {
    font-size: 32px;
    font-weight: 800;
    color: var(--amk);
    font-variant-numeric: tabular-nums;
  }
  .owe .big.r {
    color: var(--badk);
  }
  .oline {
    font-size: var(--t-s);
    color: var(--ink-2);
    font-weight: 600;
  }
  .oline b {
    color: var(--ink);
  }
  .obar {
    position: relative;
    height: 12px;
    border-radius: var(--r-pill);
    background: var(--hatch);
    box-shadow: inset 0 0 0 1.5px var(--am);
    margin: 10px 0 8px;
    overflow: hidden;
  }
  .obar i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: var(--r-pill);
    background: linear-gradient(90deg, var(--am), var(--am2));
    transform-origin: 0 50%;
    animation: barin 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) 0.45s both;
  }
  .okbig {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 10px;
    font-size: var(--t-xl);
    font-weight: 800;
    color: var(--ink);
  }
  .okbig .ok {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--c-fill);
    color: var(--on);
  }

  @keyframes rise {
    from { opacity: 0; transform: translateY(14px); }
    to { opacity: 1; transform: none; }
  }
  @keyframes barin {
    from { transform: scaleX(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .box, .obar i {
      animation: none !important;
    }
  }
</style>
