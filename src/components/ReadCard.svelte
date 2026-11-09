<script lang="ts">
  // Ficha de metas de Inicio, de dos caras: «Lectura de hoy» (vn + jvn) y «Juego de hoy» (pc + switch). No alterna sola:
  // la cara inicial es la urgente o la de la partida más reciente, y se cambia con LB/RB o el anillo pequeño.
  import { untrack } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_COL, GOAL_GROUPS, GOAL_TXT, goalMin, lastPlayAt, type GoalGroup } from '../lib/readgoals';
  import { createReadRings } from '../lib/readrings.svelte';
  import { fmtGoal } from '../lib/readfmt';
  import ReadRings from './ReadRings.svelte';

  let { left, w }: { left: number; w: number } = $props();

  const faces = $derived(app.goalFaces);
  const keysOf = (g: GoalGroup) => GOAL_GROUPS[g].filter((k) => app.goalKeys.includes(k));

  // Pistas pequeñas de cada cara: urgencia y geometría del anillo pequeño.
  const rrR = createReadRings({ keys: () => keysOf('read'), size: 22, stroke: 3.5, gap: 1.3, empty: '' });
  const rrP = createReadRings({ keys: () => keysOf('play'), size: 22, stroke: 3.5, gap: 1.3, empty: '' });
  const rrOf = (g: GoalGroup) => (g === 'read' ? rrR : rrP);

  /** 3: racha por encender; 2: rescate; 1: racha en peligro hoy; 0: nada. */
  function level(g: GoalGroup) {
    const rr = rrOf(g);
    let n = 0;
    for (const t of rr.tracks) n = Math.max(n, rr.pending(t) ? 3 : t.rescue ? 2 : !t.done && t.streak > 0 ? 1 : 0);
    return n;
  }
  const pinned = $derived.by((): GoalGroup | null => {
    if (faces.length < 2) return null;
    const a = level('read'), b = level('play');
    return a > b ? 'read' : b > a ? 'play' : null;
  });

  // Cara inicial al montar Inicio: la fijada, o la del grupo con la partida más reciente.
  untrack(() => {
    const now = Date.now();
    const recent = (g: GoalGroup) => lastPlayAt(keysOf(g), app.lib.platforms, app.playing, now);
    const pick: GoalGroup = pinned ?? (recent('play') > recent('read') ? 'play' : 'read');
    app.readFace = faces.includes(pick) ? pick : (faces[0] ?? 'read');
  });
  $effect(() => {
    if (!faces.length) return;
    if (!faces.includes(app.readFace)) app.readFace = faces[0];
  });

  const other = $derived<GoalGroup>(app.readFace === 'read' ? 'play' : 'read');
  const accOf = (g: GoalGroup) => GOAL_COL[keysOf(g)[0] ?? GOAL_GROUPS[g][0]].c;
</script>

<div class="hsect" style="left:{left}px">{GOAL_TXT[app.readFace].card}</div>
{#if faces.length > 1}
  <div class="rind" style="right:{1280 - left - w}px">
    <span class="rmini" data-hact="face:{other}" title={GOAL_TXT[other].card}>
      <svg width="22" height="22" viewBox="0 0 22 22">
        {#each rrOf(other).rings as g (g.t.key)}
          <circle cx="11" cy="11" r={g.r} fill="none" stroke={GOAL_COL[g.t.key].soft} stroke-width="3.5" />
          <circle cx="11" cy="11" r={g.r} fill="none" stroke={GOAL_COL[g.t.key].c} stroke-width="3.5" stroke-linecap="round"
            stroke-dasharray={g.C} stroke-dashoffset={g.C * (1 - g.t.frac)} transform="rotate(-90 11 11)" />
        {/each}
      </svg>
    </span>
    <span class="rdots">
      {#each faces as g (g)}
        <i class:on={g === app.readFace} style="--c:{accOf(g)}">
        </i>
      {/each}
    </span>
  </div>
{/if}
<div class="glass rcard rread fx" class:f={app.focus === 'read'} data-k="read" data-act="read" style="left:{left}px;width:{w}px;--acc:var(--acc)">
  <div class="face">
    {#each faces as g (g)}
      <div class="rface" class:on={g === app.readFace}><ReadRings keys={keysOf(g)} active={g === app.readFace} /></div>
    {/each}
  </div>
</div>
<div class="rmeta" style="left:{left + 4}px;width:{w - 4}px">Meta diaria<span>&nbsp;· {keysOf(app.readFace).map((k) => `${fmtGoal(goalMin(app.config, k))} ${GOAL_COL[k].name}`).join(' · ')}</span></div>

<style>
  .rface{position:absolute;inset:0;opacity:0;pointer-events:none;transition:opacity .45s ease}
  .rface.on{opacity:1;pointer-events:auto}
  @media (prefers-reduced-motion: reduce){.rface{transition:none}}
  .rind{position:absolute;top:378px;height:24px;display:flex;align-items:center;gap:10px}
  .rmini{display:block;width:22px;height:22px;cursor:pointer}
  .rmini svg{display:block;overflow:visible}
  .rdots{display:flex;align-items:center;gap:6px}
  .rdots i{display:block;position:relative;width:7px;height:7px;border-radius:999px;overflow:hidden;background:color-mix(in srgb,var(--ink-3) 45%,transparent);transition:width .3s ease}
  .rdots i.on{width:20px;background:var(--c)}
</style>
