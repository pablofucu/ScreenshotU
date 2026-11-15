<script lang="ts">
  // Caja inferior izquierda del detalle: las dos fichas de días libres cada 7 días.
  import type { GoalTrack } from '../../lib/readgoals';
  import { freeClose, freeDayName } from '../../lib/readfmt';
  import IconoRacha from './IconoRacha.svelte';

  let { track, today }: { track: GoalTrack; today: number } = $props();

  // Solo cuentan las fichas gastadas en [hoy-6, hoy-1] (la de hoy-7 ya volvió hoy).
  const used = $derived(track.free.filter((f) => f.day >= today - 6));
  const slots = $derived([used[0] ?? null, used[1] ?? null]);
</script>

<div class="box gin">
  <div class="t">Días libres<small>2 cada 7 días</small></div>
  <div class="coins">
    {#each slots as f, k (k)}
      {#if f}
        <div class="cw" title="Plazo hasta el {freeClose(f, true)}">
          <div class="coin used" style="--d:{0.8 + k * 0.12}s">
            <IconoRacha name="moon" size={22} />
          </div>
          <div class="txt">Usado el {freeDayName(f.day, false)}<small>· vuelve el {freeDayName(f.deadline, false)}</small></div>
        </div>
      {:else}
        <div class="cw">
          <div class="coin" style="--d:{0.8 + k * 0.12}s">
            <IconoRacha name="moon" size={24} stroke="var(--amk)" fill="color-mix(in srgb, var(--am) 30%, transparent)" />
          </div>
          <div class="txt">Disponible<small>por si fallas un día</small></div>
        </div>
      {/if}
    {/each}
  </div>
</div>

<style>
  .gin {
    background: var(--glass-in);
    border-radius: var(--r-m);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .box {
    position: absolute;
    left: 392px;
    top: 392px;
    width: 300px;
    height: 136px;
    padding: 14px 20px 16px;
    box-sizing: border-box;
    animation: rise 0.5s cubic-bezier(0.2, 0.9, 0.3, 1.1) 0.6s both;
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
  .coins {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
  }
  .cw {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: var(--t-s);
    font-weight: 800;
    line-height: 1.25;
    color: var(--ink);
    white-space: nowrap;
  }
  .txt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cw small {
    display: inline;
    margin-left: 6px;
    font-size: var(--t-xs);
    font-weight: 600;
    color: var(--ink-2);
  }
  .coin {
    position: relative;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--amk);
    background: radial-gradient(circle at 35% 30%, color-mix(in srgb, var(--am2) 30%, var(--tint-base)), var(--am-soft));
    box-shadow: inset 0 0 0 2px var(--am), 0 6px 14px -8px color-mix(in srgb, var(--am) 80%, transparent);
    animation: pop 0.5s cubic-bezier(0.3, 1.6, 0.5, 1) var(--d) both, bob 2.8s ease-in-out 1.4s infinite;
  }
  .coin.used {
    background: transparent;
    color: var(--ink-3);
    box-shadow: none;
    border: 2px dashed color-mix(in srgb, var(--ink-3) 55%, transparent);
    box-sizing: border-box;
    animation: pop 0.5s cubic-bezier(0.3, 1.6, 0.5, 1) var(--d) both;
  }

  @keyframes rise {
    from { opacity: 0; transform: translateY(40px); }
    to { opacity: 1; transform: none; }
  }
  @keyframes pop {
    from { opacity: 0; transform: scale(0.6); }
    to { opacity: 1; transform: scale(1); }
  }
  @keyframes bob {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-4px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .box, .coin, .coin.used {
      animation: none !important;
    }
  }
</style>
