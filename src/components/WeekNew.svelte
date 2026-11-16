<script lang="ts">
  // Aviso de semana nueva (Inicio): cifras de la semana pasada, una fila por categoría con actividad.
  // A abre Estadísticas › Semana en esa semana; B lo descarta. El mando lo recoge `weekNewCmd` (controlador).
  import { app } from '../lib/store.svelte';
  import { openWeekNew } from '../lib/controller.svelte';
  import { GOAL_KEYS, goalMin, type GoalKey } from '../lib/readgoals';
  import { fmtHM, weekReview } from '../lib/stats';
  import { catIcon, catTone, toneStyle } from '../lib/chiptone';
  import { closeWeek, lastWeekFrom, offerWeek, weekSeen, wn } from '../lib/weeknew.svelte';
  import CIcon from './CIcon.svelte';

  const from = $derived(lastWeekFrom(app.now.getTime()));
  const goals = $derived(Object.fromEntries(GOAL_KEYS.filter((k) => goalMin(app.config, k) > 0).map((k) => [k, goalMin(app.config, k)])) as Partial<Record<GoalKey, number>>);
  // Solo se calcula si la semana pasada está sin ver (o el aviso abierto).
  const W = $derived(wn.open || !weekSeen(from) ? weekReview(app.lib, -1, app.favs, app.playing, goals, app.now.getTime()) : null);

  $effect(() => {
    if (W && !app.scanning && app.lib.platforms.length) offerWeek(W.range.from, W.ms);
  });

  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}`; };
  const span = $derived(W ? `Del ${dm(W.range.from)} al ${dm(W.range.to - 864e5)}` : '');
  const rows = $derived((W?.blocks ?? []).filter((b) => b.ms > 0 || (b.goal && b.goal.met > 0)).slice(0, 4).map((b) => {
    const id = app.lib.platforms[b.pi]?.id ?? '';
    return { b, id, tone: toneStyle(catTone(id)), icon: catIcon(id) };
  }));
  const pct = $derived(W && W.prevMs > 0 ? Math.round(((W.ms - W.prevMs) / W.prevMs) * 100) : null);
</script>

{#if wn.open && W}
  <div class="scrim wnscrim" role="presentation">
    <div class="glass float wnmodal" role="alertdialog" aria-modal="true" aria-label="Resumen de la semana pasada">
      <div class="wntile"><CIcon name="cal" /></div>
      <h1>Tu semana</h1>
      <p class="sub">{span} · <b>{fmtHM(W.ms)}</b> · {W.activeDays} de 7 días con partidas{#if pct !== null}<span class="dl" class:dn={pct < 0}> · {pct >= 0 ? '+' : ''}{pct} % frente a la anterior</span>{/if}</p>
      <div class="wnrows">
        {#each rows as r (r.b.pi)}
          <div class="wnrow" style={r.tone}>
            <i><CIcon name={r.icon} /></i>
            <span class="nm">{r.b.name}</span>
            {#if r.b.goal}<span class="gl">{r.b.goal.met}/7 metas</span>{/if}
            <b>{fmtHM(r.b.ms)}</b>
          </div>
        {/each}
      </div>
      <div class="mb">
        <button type="button" class="b-main f" onclick={() => openWeekNew()}><span class="btn a big">A</span>Ver la semana</button>
        <button type="button" class="b-soft pill" onclick={() => closeWeek()}><span class="btn b big">B</span>Más tarde</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .wnscrim { z-index: 55; display: block; }
  .wnmodal { left: 340px; top: 110px; width: 600px; height: auto; padding: 30px 36px 28px; text-align: center; cursor: default; border-radius: var(--r-l); overflow: hidden; }
  .wnmodal::before { content: ''; position: absolute; left: 0; right: 0; top: 0; height: 6px; background: var(--grad); z-index: 4; }
  .wntile { position: relative; z-index: 1; width: 60px; height: 60px; margin: 0 auto 12px; border-radius: 20px; display: grid; place-items: center; color: var(--on);
    background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 40%, var(--face-hi)), var(--acc));
    box-shadow: inset 0 1px 0 var(--face-hi), 0 0 0 1px color-mix(in srgb, var(--acc) 70%, black); }
  .wntile :global(svg) { width: 32px; height: 32px; }
  h1 { position: relative; z-index: 1; margin: 0; font-size: var(--t-2xl); font-weight: 800; color: var(--ink); }
  .sub { position: relative; z-index: 1; margin: 6px 0 0; font-size: var(--t-m); font-weight: 600; color: var(--ink-2); }
  .sub b { color: var(--ink); font-weight: 800; }
  .dl { color: var(--ok-f, var(--ok)); }
  .dl.dn { color: var(--ink-3); }
  .wnrows { position: relative; z-index: 1; display: flex; flex-direction: column; gap: 8px; margin-top: 18px; text-align: left; }
  .wnrow { display: flex; align-items: center; gap: 12px; height: 48px; padding: 0 16px 0 8px; border-radius: var(--r-m); background: var(--face-hi); box-shadow: inset 0 0 0 1px var(--line); }
  .wnrow i { width: 34px; height: 34px; border-radius: 11px; display: grid; place-items: center; color: var(--on); background: var(--gv); }
  .wnrow i :global(svg) { width: 20px; height: 20px; }
  .nm { flex: 1; min-width: 0; font-size: var(--t-l); font-weight: 700; color: var(--ink); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .gl { font-size: var(--t-s); font-weight: 700; color: var(--f1); }
  .wnrow b { font-size: var(--t-l); font-weight: 800; color: var(--ink); font-variant-numeric: tabular-nums; }
  .mb { position: relative; z-index: 1; display: flex; gap: 16px; justify-content: center; margin-top: 22px; }
  .mb :global(.b-main), .mb :global(.b-soft) { height: 52px; padding: 0 28px 0 12px; font-size: var(--t-xl); cursor: pointer; }
  .mb :global(.b-soft) { display: inline-flex; align-items: center; gap: var(--s-2); border: 0; font-weight: 700; color: var(--ink); }
  .mb :global(.btn.big) { min-width: 36px; height: 36px; font-size: var(--t-l); border-radius: 14px; }
</style>
