<script lang="ts">
  // Estadísticas › Terminados: tira de años arriba y, debajo y en grande, el año elegido con un carril por categoría
  // (nunca se mezclan), en el orden y el color de las rachas. Cada mes enseña las carátulas de lo terminado en él;
  // si son varias, se reparten el hueco en rejilla. Debajo, una línea con el juego enfocado.
  import { app } from '../lib/store.svelte';
  import { finishedYear, finishedByYear, fmtHM, type FinRef } from '../lib/stats';
  import { catColor, catIcon, catRank, catTone, toneStyle } from '../lib/chiptone';
  import ChipPane from '../components/ChipPane.svelte';
  import CIcon from '../components/CIcon.svelte';
  import CoverBg from '../components/CoverBg.svelte';

  const MH = ['Ene', 'Feb', 'Mar', 'Abr', 'May', 'Jun', 'Jul', 'Ago', 'Sep', 'Oct', 'Nov', 'Dic'];
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const LBL = 132, GAP = 6, BODY = 320;

  const F = $derived(finishedYear(app.lib, app.finYear, app.statsCat));
  const plat = (pi: number) => app.lib.platforms[pi];
  const act = (r: FinRef) => `sr:${r.pi}:${r.ti}`;
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]} ${d.getFullYear()}`; };

  /* Tira de años: los que tienen terminados y el actual aunque tenga 0. */
  const nowY = $derived(app.now.getFullYear()), nowM = $derived(app.now.getMonth());
  const byYearList = $derived(finishedByYear(app.lib, app.statsCat));
  const byYear = $derived(new Map(byYearList.map((e) => [e.year, e])));
  const years = $derived([...new Set([...app.finYears, nowY])].sort((a, b) => a - b));
  const ordered = (m: Map<number, number>) => [...m].sort((a, b) => catRank(plat(a[0]).id) - catRank(plat(b[0]).id));
  let stripEl: HTMLElement | undefined = $state();
  $effect(() => {
    void app.finYear; void years.length;
    const el = stripEl?.querySelector<HTMLElement>('.fyr.on');
    if (stripEl && el) stripEl.scrollTo({ left: el.offsetLeft - (stripEl.clientWidth - el.offsetWidth) / 2, behavior: 'smooth' });
  });

  /* Carriles: toda categoría que haya terminado algo alguna vez (así no saltan al cambiar de año), en el orden de las rachas. */
  const lanes = $derived.by(() => {
    const ever = new Set(byYearList.flatMap((e) => [...e.by.keys()]));
    const got = new Map(F.lanes.map((l) => [l.pi, l]));
    return [...ever].sort((a, b) => catRank(plat(a).id) - catRank(plat(b).id) || a - b).map((pi) => got.get(pi) ?? {
      pi, id: plat(pi).id, name: plat(pi).name, count: 0, months: Array.from({ length: 12 }, () => [] as FinRef[]),
    });
  });

  /* Tamaño de las carátulas: cada carril se lleva su parte del alto y cada mes reparte su celda entre sus carátulas. */
  let gw = $state(1100);
  const colW = $derived(Math.max(40, (gw - LBL - 12 * GAP) / 12));
  const laneH = $derived(Math.max(70, Math.min(220, Math.floor((BODY - GAP * (lanes.length - 1)) / Math.max(1, lanes.length)))));
  function fit(n: number): { w: number; h: number } {
    let best = { w: 0, h: 0 };
    for (let c = 1; c <= n; c++) {
      const r = Math.ceil(n / c);
      const w = Math.min((colW - 8 - 3 * (c - 1)) / c, ((laneH - 8 - 3 * (r - 1)) / r) * 0.7);
      if (w > best.w) best = { w, h: w / 0.7 };
    }
    return best;
  }

  const fut = (m: number) => F.year > nowY || (F.year === nowY && m > nowM);
  const keyOf = (r: FinRef) => `fn${r.pi}_${r.ti}`;
  /** La carátula enfocada o, si no hay ninguna, el último terminado. */
  const sel = $derived(lanes.flatMap((l) => l.months.flat()).find((r) => keyOf(r) === app.focus) ?? F.last);
</script>

<ChipPane>
  <div class="years" bind:this={stripEl}>
    {#each years as y (y)}
      {@const e = byYear.get(y)}
      <div class="fyr" class:on={y === F.year} data-hact="fy:{y}">
        <div class="yh"><span>{y}</span><b>{e?.count ?? 0}</b></div>
        <div class="mix">{#if e}{#each ordered(e.by) as [pi, n] (pi)}<i style="flex:{n};--c:{catColor(plat(pi).id)}"></i>{/each}{/if}</div>
      </div>
    {/each}
  </div>

  {#if lanes.length}
    <div class="board">
      <div class="yg" bind:clientWidth={gw} style="grid-template-columns:{LBL}px repeat(12,minmax(0,1fr));gap:{GAP}px">
        <div class="tot"><b>{F.count}</b> en {F.year}</div>
        {#each MH as m, j}<div class="mh" class:now={F.year === nowY && j === nowM}>{m}</div>{/each}
        {#each lanes as l, li (l.pi)}
          {@const tone = toneStyle(catTone(l.id))}
          <div class="l" style="{tone};height:{laneH}px"><CIcon name={catIcon(l.id)} class="" /><span>{l.name}</span><em>{l.count}</em></div>
          {#each l.months as cells, j}
            {@const sz = fit(cells.length)}
            <div class="cell" class:fut={fut(j)} class:has={cells.length > 0} style="{tone};height:{laneH}px">
              {#each cells as r, n (keyOf(r))}
                <CoverBg t={r.t} cls="cvf fx" k={keyOf(r)} act={act(r)} style="--i:{li * 3 + j + n};width:{sz.w}px;height:{sz.h}px" title={r.t.name} />
              {/each}
            </div>
          {/each}
        {/each}
      </div>
    </div>
    <div class="cap">
      {#if sel}
        <i style="--c:{catColor(plat(sel.pi).id)}"></i><b>{sel.t.name}</b><span>{plat(sel.pi).name} · terminado el {dm(sel.finishedMs)}{#if sel.ms > 0} · {fmtHM(sel.ms)} jugado{/if}</span>
      {:else}
        <span>Ningún juego terminado en {F.year}.</span>
      {/if}
    </div>
  {:else}
    <div class="cnone">Aún no hay juegos terminados. Al marcar uno como terminado aparecerá aquí, en su mes y con el color de su categoría.</div>
  {/if}
</ChipPane>

<style>
  .years,.board,.cap{flex:none}
  .years{display:flex;gap:var(--s-2);margin-bottom:var(--s-3);overflow-x:auto;scrollbar-width:none;padding:4px 2px}
  .years::-webkit-scrollbar{display:none}
  .fyr{flex:1 0 88px;height:48px;padding:7px 12px;border-radius:var(--r-m);background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line);display:flex;flex-direction:column;justify-content:space-between;cursor:pointer;transition:transform var(--d-2) var(--e-bounce)}
  .yh{display:flex;justify-content:space-between;align-items:baseline;font:800 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .yh b{font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .mix{display:flex;height:6px;border-radius:3px;overflow:hidden;gap:2px;background:color-mix(in srgb,var(--ink-2) 14%,transparent)}
  .mix i{background:var(--c)}
  .fyr.on{background:var(--panel);box-shadow:inset 0 0 0 2px var(--ink);transform:scale(1.04)}

  .board{border-radius:var(--r-l);background:var(--panel);padding:12px 14px}
  .yg{display:grid;align-items:center}
  .tot{font:700 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .tot b{font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .mh{font:800 var(--t-xs)/1 var(--round);color:var(--ink-2);text-align:center;padding-bottom:2px}
  .mh.now{color:var(--ink)}
  .l{display:flex;align-items:center;gap:8px;font:800 var(--t-s)/1.1 var(--round);color:var(--ink);padding-left:10px;border-left:4px solid var(--v1);border-radius:2px}
  .l span{min-width:0;overflow:hidden;text-overflow:ellipsis}
  .l :global(svg){width:20px;height:20px;flex:none;color:var(--v1);fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}
  .l em{margin-left:auto;font:800 var(--t-m)/1 var(--round);font-style:normal;color:var(--v1);font-variant-numeric:tabular-nums}
  .cell{border-radius:10px;display:flex;flex-wrap:wrap;align-content:center;align-items:center;justify-content:center;gap:3px;background:color-mix(in srgb,var(--v1) 7%,transparent)}
  .cell.has{background:color-mix(in srgb,var(--v1) 16%,transparent)}
  .cell.fut{background:transparent;opacity:.5}
  .yg :global(.cvf){border-radius:6px;background-color:color-mix(in srgb,var(--v1) 45%,var(--panel));background-size:cover;background-position:center;cursor:pointer;box-shadow:0 4px 10px -5px rgba(20,30,50,.55);transition:transform .25s var(--e-bounce),box-shadow .2s;animation:drop .5s var(--e-bounce) both;animation-delay:calc(var(--i) * 30ms)}
  .yg :global(.cvf:hover),.yg :global(.cvf.f){transform:scale(1.18);box-shadow:0 0 0 3px var(--v1),0 8px 18px -6px var(--v1);position:relative;z-index:2}

  .cap{display:flex;align-items:center;gap:10px;margin-top:var(--s-3);padding:0 6px;font:600 var(--t-m)/1.2 var(--round);color:var(--ink-2);white-space:nowrap;overflow:hidden}
  .cap i{width:12px;height:12px;border-radius:4px;background:var(--c);flex:none}
  .cap b{color:var(--ink);font-weight:800;overflow:hidden;text-overflow:ellipsis}
  .cnone{padding:var(--s-6);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}
</style>
