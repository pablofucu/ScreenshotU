<script lang="ts">
  // Pantalla de racha: vista de los últimos 7 días como velas, llama grande con el contador
  // de racha, estado actual, barra del día, fichas de días libres y deuda por recuperar.
  import { onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_COL, goalHistOf, goalMin, goalTrack, type GoalKey, type GoalTrack } from '../lib/readgoals';
  import { createReadRings } from '../lib/readrings.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import IconoRacha from '../components/racha/IconoRacha.svelte';
  import RachaHero from '../components/racha/RachaHero.svelte';
  import VelasSemana from '../components/racha/VelasSemana.svelte';
  import DiasLibres from '../components/racha/DiasLibres.svelte';
  import PorRecuperar from '../components/racha/PorRecuperar.svelte';

  const rr = createReadRings({ size: 22, stroke: 3, gap: 1.3, empty: '' });
  const now = $derived(rr.now);
  const today = $derived(rr.today);
  const keys = $derived(app.goalKeys);

  const selKey = $derived<GoalKey | undefined>(keys.includes(app.streakKey) ? app.streakKey : keys[0]);
  const sel = $derived<GoalTrack | null>(
    selKey ? goalTrack(selKey, goalMin(app.config, selKey), app.lib.platforms, app.playing, now, undefined, goalHistOf(app.config, selKey)) : null,
  );
  const col = $derived(GOAL_COL[sel?.key ?? 'vn']);

  const calm = () => typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const pending = $derived(sel ? rr.pending(sel) : false);
  const shownStreak = $derived(sel ? rr.shown(sel) : 0);

  let root: HTMLDivElement | undefined = $state();
  let igniteTick = $state(0);
  let litToast = $state<{ streak: number } | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function showLitToast(streak: number) {
    clearTimeout(toastTimer);
    litToast = { streak };
    toastTimer = setTimeout(() => { litToast = null; }, 2600);
  }

  function light(k: GoalKey) {
    if (!root?.isConnected) return;
    rr.light(k, root, {
      box: '.hero[data-g="{k}"]',
      num: '.nn',
      out: [{ transform: 'translateY(0)', opacity: 1 }, { transform: 'translateY(-100%)', opacity: 0 }],
      outOpts: { duration: 200, easing: 'ease-in', fill: 'forwards' },
      skip: calm,
      enter: (box, num) => {
        num.animate(
          [{ transform: 'translateY(100%)', opacity: 0 }, { transform: 'translateY(-6%)', opacity: 1 }, { transform: 'translateY(0)' }],
          { duration: 420, easing: 'ease-out' },
        );
        box.querySelector<HTMLElement>('.flm')?.animate(
          [{ transform: 'scale(1)' }, { transform: 'scale(1.5) rotate(-8deg)' }, { transform: 'scale(1)' }],
          { duration: 650, easing: 'ease-out' },
        );
        igniteTick++;
        showLitToast(sel?.key === k ? sel.streak : Number(num.textContent) || 1);
      },
    });
  }

  const scheduled = new Set<string>();
  $effect(() => {
    const t = sel;
    if (!t || !rr.pending(t)) return;
    const id = `${t.key}:${today}`;
    if (scheduled.has(id)) return;
    scheduled.add(id);
    const wait = calm() ? 0 : 1200;
    const h = setTimeout(() => light(t.key), wait);
    rr.timers.push(h);
  });

  onMount(() => () => clearTimeout(toastTimer));
</script>

<TopBar crumbs={['Inicio', sel ? `Racha · ${col.full}` : 'Racha']} />

{#if sel}
  <div
    class="streak"
    bind:this={root}
    style="--c:{col.c};--c2:{col.edge};--ci:{col.ink};--c-fill:{col.ink}"
  >
    {#if keys.length > 1}
      <div class="cats pill" class:wide={keys.length > 2}>
        {#each keys as k (k)}
          {@const c = GOAL_COL[k]}
          <span
            class="t"
            class:on={k === sel.key}
            style="--tc:{c.c};--tci:{c.ink};--tc2:{c.edge}"
            data-hact={k === sel.key ? '' : 'goal:' + k}
          >
            <i style="background:{c.c}"></i>{c.tab}
          </span>
        {/each}
      </div>
    {/if}

    <div class="glass main">
      <div class="face">
        {#key sel.key}
          <RachaHero track={sel} {shownStreak} {pending} {today} calm={calm()} />
          <VelasSemana track={sel} {pending} calm={calm()} {igniteTick} />
          <DiasLibres track={sel} {today} />
          <PorRecuperar track={sel} />
        {/key}
      </div>
    </div>

    {#if litToast}
      <div class="ltoast pill" role="status">
        <span class="ti"><IconoRacha name="flame" size={18} stroke="var(--on)" /></span>
        Racha encendida · {litToast.streak} {litToast.streak === 1 ? 'día' : 'días'}
      </div>
    {/if}
  </div>

  <Hints
    list={[
      ...(keys.length > 1 ? [['LB/RB', 'Categoría', 'lb'] as [string, string, string]] : []),
      ['Y', 'Historial', 'y'],
      ['B', 'Volver', 'b'],
    ]}
  />
{:else}
  <div class="stnone">No hay metas diarias en la biblioteca.</div>
  <Hints list={[['B', 'Volver', 'b']]} />
{/if}

<style>
  .streak {
    position: absolute;
    inset: 0;
    --am: var(--warn);
    --am2: color-mix(in srgb, var(--warn) 65%, white);
    --amk: color-mix(in srgb, var(--warn) 58%, black);
    --am-soft: color-mix(in srgb, var(--warn) 22%, var(--tint-base));
    --badk: var(--bad-f);
    --bad-soft: color-mix(in srgb, var(--bad) 16%, var(--tint-base));
    --hatch: repeating-linear-gradient(-45deg, color-mix(in srgb, var(--am) 55%, transparent) 0 3px, transparent 3px 7px);
  }
  :global(#stage[data-theme='dark']) .streak {
    --ci: color-mix(in srgb, var(--c) 70%, white);
    --amk: color-mix(in srgb, var(--warn) 78%, white);
    --am-soft: color-mix(in srgb, var(--warn) 24%, var(--tint-base));
    --badk: color-mix(in srgb, var(--bad) 65%, white);
    --bad-soft: color-mix(in srgb, var(--bad) 24%, var(--tint-base));
  }

  .cats {
    position: absolute;
    left: 50%;
    top: 22px;
    transform: translateX(-50%);
    display: flex;
    gap: 4px;
    padding: 4px;
    height: 40px;
    box-sizing: border-box;
    align-items: center;
    z-index: 6;
  }
  .cats.wide {
    left: 690px;
  }
  .cats .t {
    height: 32px;
    padding: 0 14px;
    border-radius: var(--r-pill);
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--t-s);
    font-weight: 800;
    color: var(--ink-2);
    cursor: pointer;
    white-space: nowrap;
  }
  .cats .t i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }
  .cats .t.on {
    background: linear-gradient(115deg, var(--tci), color-mix(in srgb, var(--tci) 70%, var(--tc2)));
    color: var(--on);
    cursor: default;
  }
  .cats .t.on i {
    background: var(--on) !important;
  }

  .main {
    left: 24px;
    top: 84px;
    width: 1232px;
    height: 566px;
    cursor: default;
  }
  .main > .face {
    background: linear-gradient(180deg, var(--face-hi), var(--face));
  }

  .ltoast {
    position: absolute;
    left: 50%;
    top: 96px;
    z-index: 20;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    height: 48px;
    padding: 0 22px 0 8px;
    border-radius: var(--r-pill);
    font-size: var(--t-l);
    font-weight: 800;
    color: var(--ink);
    pointer-events: none;
    animation: ltoast 2.6s var(--e-soft) both;
  }
  .ltoast .ti {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--c-fill);
  }

  @keyframes ltoast {
    0% { opacity: 0; transform: translate(-50%, -14px) scale(0.9); }
    15% { opacity: 1; transform: translate(-50%, 0) scale(1); }
    85% { opacity: 1; transform: translate(-50%, 0) scale(1); }
    100% { opacity: 0; transform: translate(-50%, -8px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ltoast {
      animation: none;
    }
  }
</style>
