<script lang="ts">
  // Ficha de metas de Inicio, de una cara por categoría (vn, jvn, pc, switch). No alterna sola:
  // la categoría inicial es la urgente o la de la partida más reciente, y se cambia con LB/RB o los puntos.
  import { untrack } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_COL, GOAL_TXT, goalMin, groupOf, lastPlayAt, type GoalKey } from '../lib/readgoals';
  import { createReadRings } from '../lib/readrings.svelte';
  import { fmtGoal } from '../lib/readfmt';
  import ReadRings from './ReadRings.svelte';

  let { left, w }: { left: number; w: number } = $props();

  const keys = $derived(app.goalKeys);
  const rr = createReadRings({ keys: () => app.goalKeys, size: 22, stroke: 3.5, gap: 1.3, empty: '' });

  /** 3: racha por encender; 2: rescate; 1: racha en peligro hoy; 0: nada. */
  function level(k: GoalKey) {
    const t = rr.tracks.find((x) => x.key === k);
    return t ? (rr.pending(t) ? 3 : t.rescue ? 2 : !t.done && t.streak > 0 ? 1 : 0) : 0;
  }

  // Categoría inicial al montar Inicio: la de más nivel, o la de la partida más reciente.
  untrack(() => {
    if (app.homeGoal && keys.includes(app.homeGoal)) return;
    const now = Date.now();
    let best: GoalKey | undefined, bl = 0;
    for (const k of keys) { const l = level(k); if (l > bl) { bl = l; best = k; } }
    if (!best) {
      let bt = -1;
      for (const k of keys) { const t = lastPlayAt([k], app.lib.platforms, app.playing, now); if (t > bt) { bt = t; best = k; } }
    }
    app.homeGoal = best ?? keys[0] ?? '';
  });
  $effect(() => {
    if (keys.length && (!app.homeGoal || !keys.includes(app.homeGoal))) app.homeGoal = keys[0];
  });

  const k = $derived(app.homeGoal);
</script>

{#if k}
  <div class="hsect" style="left:{left}px">{GOAL_TXT[groupOf(k)].card + ' · ' + GOAL_COL[k].tab}</div>
  {#if keys.length > 1}
    <div class="rind" style="right:{1280 - left - w}px">
      <span class="rdots">
        {#each keys as g (g)}
          <i class:on={g === k} style="--c:{GOAL_COL[g].c}" data-hact="hgoal:{g}" title={GOAL_COL[g].name}></i>
        {/each}
      </span>
    </div>
  {/if}
  <div class="glass rcard rread fx" class:f={app.focus === 'read'} data-k="read" data-act="read" style="left:{left}px;width:{w}px;--acc:var(--acc)">
    <div class="face">
      <div class="rface on"><ReadRings keys={[k]} active /></div>
    </div>
  </div>
  <div class="rmeta" style="left:{left + 4}px;width:{w - 4}px">Meta diaria<span>&nbsp;· {fmtGoal(goalMin(app.config, k))} {GOAL_COL[k].name}</span></div>
{/if}

<style>
  .rface{position:absolute;inset:0}
  .rind{position:absolute;top:378px;height:24px;display:flex;align-items:center;gap:10px}
  .rdots{display:flex;align-items:center;gap:6px}
  .rdots i{display:block;position:relative;width:7px;height:7px;border-radius:999px;overflow:hidden;cursor:pointer;background:color-mix(in srgb,var(--ink-3) 45%,transparent);transition:width .3s ease}
  .rdots i.on{width:20px;background:var(--c)}
</style>
