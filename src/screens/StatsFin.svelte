<script lang="ts">
  // Estadísticas › Terminados: tira de años arriba, el año elegido con un carril por categoría (nunca se mezclan)
  // en medio y, abajo, cuatro cifras. Las portadas crecen o encogen según cuántos carriles haya.
  import { app } from '../lib/store.svelte';
  import { finishedYear, finishedByYear, fmtHM, type FinRef } from '../lib/stats';
  import { catIcon, catTone, toneStyle } from '../lib/chiptone';
  import { countUp } from '../lib/countup';
  import ChipPane from '../components/ChipPane.svelte';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import CoverBg from '../components/CoverBg.svelte';

  const ARC = ['rojo', 'naranja', 'amarillo', 'verde', 'azul', 'anil', 'violeta'];
  const MH = ['Ene', 'Feb', 'Mar', 'Abr', 'May', 'Jun', 'Jul', 'Ago', 'Sep', 'Oct', 'Nov', 'Dic'];
  const MONTH = ['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio', 'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre'];
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  /** Estaciones como tramos de meses seguidos (diciembre vuelve a ser invierno). */
  const SEAS: { n: string; from: number; to: number; c: string }[] = [
    { n: 'Invierno', from: 0, to: 1, c: 'azul' }, { n: 'Primavera', from: 2, to: 4, c: 'verde' },
    { n: 'Verano', from: 5, to: 7, c: 'amarillo' }, { n: 'Otoño', from: 8, to: 10, c: 'naranja' }, { n: 'Invierno', from: 11, to: 11, c: 'azul' },
  ];
  const F = $derived(finishedYear(app.lib, app.finYear, app.finCat));
  const fid = $derived(`fin${F.year}${app.finCat}`);
  const plat = (pi: number) => app.lib.platforms[pi];
  const arc = (pi: number) => `var(--arc-${ARC[catTone(plat(pi).id) % 7]})`;
  const act = (r: FinRef) => `sr:${r.pi}:${r.ti}`;
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}`; };

  const all = $derived(F.lanes.flatMap((l) => l.months.flat()));
  const diff = $derived(F.count - F.prevCount);
  const cmpMax = $derived(Math.max(F.count, F.prevCount, 1));
  const totMs = $derived(all.reduce((a, r) => a + r.ms, 0));
  const withMs = $derived(all.filter((r) => r.ms > 0).length);

  /* Tira de años: los que tienen terminados y el actual aunque tenga 0. */
  const nowY = $derived(app.now.getFullYear()), nowM = $derived(app.now.getMonth());
  const byYear = $derived(new Map(finishedByYear(app.lib, app.finCat).map((e) => [e.year, e])));
  const years = $derived([...new Set([...app.finYears, nowY])].sort((a, b) => a - b));
  let stripEl: HTMLElement | undefined = $state();
  $effect(() => {
    void app.finYear; void years.length;
    const el = stripEl?.querySelector<HTMLElement>('.yr.on');
    if (stripEl && el) stripEl.scrollTo({ left: el.offsetLeft - (stripEl.clientWidth - el.offsetWidth) / 2, behavior: 'smooth' });
  });

  /* Alto de cada carril: reparte el hueco bajo las cabeceras entre las categorías con terminados. */
  const laneH = $derived(Math.max(46, Math.min(160, Math.floor((236 - 6 * (F.lanes.length - 1)) / Math.max(1, F.lanes.length)))));
  const covH = $derived(Math.min(laneH - 12, 110));

  const fut = (m: number) => F.year > nowY || (F.year === nowY && m > nowM);
  const keyed = $derived.by(() => {
    let i = 4;
    return F.lanes.map((l) => ({ l, months: l.months.map((m) => m.map((r) => ({ r, k: `fn${i++}` }))) }));
  });
  /** La carátula enfocada o, si no hay ninguna, el último terminado. */
  const sel = $derived(keyed.flatMap((x) => x.months.flat()).find((c) => c.k === app.focus)?.r ?? F.last);
</script>

<ChipPane>
  <div class="years" bind:this={stripEl}>
    {#each years as y (y)}
      {@const e = byYear.get(y)}
      <div class="yr" class:on={y === F.year} data-hact="fy:{y}">
        <div class="yh"><span>{y}</span><b>{e?.count ?? 0}</b></div>
        <div class="mix">{#if e}{#each [...e.by] as [pi, n] (pi)}<i style="flex:{n};--c:{arc(pi)}"></i>{/each}{/if}</div>
      </div>
    {/each}
  </div>

  {#if F.count}
    <div class="lanes glass">
      <div class="face">
        <div class="yg" style="--lh:{laneH}px;--ch:{covH}px">
          <div></div>
          {#each SEAS as s}<div class="sea" style="grid-column:{s.from + 2} / {s.to + 3};--c:var(--arc-{s.c})">{s.n}</div>{/each}
          <div></div>
          {#each MH as m, j}<div class="mh" class:now={F.year === nowY && j === nowM}>{m}</div>{/each}
          {#each keyed as x (x.l.pi)}
            {@const tone = toneStyle(catTone(x.l.id))}
            <div class="l" style={tone}><CIcon name={catIcon(x.l.id)} class="" />{x.l.name}<em>{x.l.count}</em></div>
            {#each x.months as cells, j}
              <div class="cell" class:fut={fut(j)} class:many={cells.length > 1} style={tone}>
                {#each cells as c, n (c.k)}<CoverBg t={c.r.t} cls="cvf fx" k={c.k} act={act(c.r)} style="--i:{j + n}" title={c.r.t.name} />{/each}
              </div>
            {/each}
          {/each}
        </div>
      </div>
    </div>
  {:else}
    <div class="cnone">Ningún juego terminado en {F.year}. Al marcar uno como terminado aparecerá aquí, en su mes.</div>
  {/if}

  <div class="strip">
    <StatChip k="fn0" icon="check" label="Terminados en {F.year}" tone={6} act={F.last ? act(F.last) : undefined}>
      <div class="row">
        <div class="x-v"><span use:countUp={{ to: F.count, key: `fin:M:${fid}` }}>{F.count}</span></div>
        <div class="por">{#each F.lanes as l (l.pi)}<span style="--c:{arc(l.pi)}" title={l.name}><i></i>{l.count}</span>{/each}</div>
      </div>
    </StatChip>

    <StatChip k="fn1" icon="swap" label="Frente a {F.year - 1}" tone={4}>
      <div class="row">
        <div class="x-v"><span use:countUp={{ to: F.count, key: `fin:G:${fid}` }}>{F.count}</span><small>/ {F.prevCount}</small></div>
        {#if diff !== 0}<span class="delta" class:dn={diff < 0}><CIcon name={diff < 0 ? 'down' : 'up'} class="" />{diff > 0 ? '+' : '−'}{Math.abs(diff)}</span>{/if}
      </div>
      <div class="cmp">
        <div class="now"><u><i style="--w:{(F.count / cmpMax) * 100}%"></i></u></div>
        <div><u><i style="--w:{(F.prevCount / cmpMax) * 100}%"></i></u></div>
      </div>
    </StatChip>

    <StatChip k="fn2" icon="clock" label="Horas hasta terminarlos" tone={2}>
      <div class="row">
        <div class="x-v">{Math.round(totMs / 36e5)}<small>h</small></div>
        {#if withMs}<div class="x-s">{fmtHM(totMs / withMs)}<br />de media</div>{/if}
      </div>
    </StatChip>

    <StatChip k="fn3" icon="trophy" label="Seleccionado" tone={6} act={sel ? act(sel) : undefined}>
      {#if sel}
        <div class="row">
          <CoverBg t={sel.t} cls="lcv" title={sel.t.name} />
          <div class="x-s"><b>{sel.t.name}</b><br />{MONTH[new Date(sel.finishedMs).getMonth()]} · {plat(sel.pi).name}<br />{dm(sel.finishedMs)}{#if sel.ms > 0} · {fmtHM(sel.ms)}{/if}</div>
        </div>
      {:else}
        <div class="x-s">Aún no hay terminados este año.</div>
      {/if}
    </StatChip>
  </div>
</ChipPane>

<style>
  .years{display:flex;gap:var(--s-2);margin-bottom:var(--s-3);overflow-x:auto;scrollbar-width:none;padding:4px 2px}
  .years::-webkit-scrollbar{display:none}
  .yr{flex:1 0 96px;height:52px;padding:8px 12px;border-radius:var(--r-m);background:var(--glass-in);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--ink-2) 25%,transparent);display:flex;flex-direction:column;justify-content:space-between;cursor:pointer;transition:transform var(--d-2) var(--e-bounce)}
  .yh{display:flex;justify-content:space-between;align-items:baseline;font:800 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .yh b{font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .mix{display:flex;height:6px;border-radius:3px;overflow:hidden;gap:2px;background:color-mix(in srgb,var(--ink-2) 14%,transparent)}
  .mix i{background:var(--c)}
  .yr.on{background:var(--gf);box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 6px 14px -8px var(--f2);transform:scale(1.04)}
  .yr.on .yh,.yr.on .yh b{color:var(--on)}
  .strip{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:var(--s-4);margin-top:var(--s-3)}
  .strip :global(.st.st){height:96px;min-height:0}
  .strip :global(.st .face){padding:8px 14px;gap:4px}
  .row{display:flex;align-items:center;gap:var(--s-3);flex:1}
  .row .x-v{font-size:var(--t-xl)}
  .row .x-s{font:600 var(--t-s)/1.3 var(--round);color:var(--ink-2);min-width:0;overflow:hidden;text-overflow:ellipsis}
  .row .x-s b{color:var(--ink);font-weight:800;white-space:nowrap}
  .por{display:flex;flex-wrap:wrap;gap:6px 10px;margin-left:auto;justify-content:flex-end}
  .por span{display:inline-flex;align-items:center;gap:4px;font:800 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .por i{width:10px;height:10px;border-radius:3px;background:var(--c)}
  :global(.lcv){width:36px;height:50px;flex:none;border-radius:var(--r-xs);background-size:cover;background-position:center;box-shadow:0 0 0 2px var(--done)}
  .strip .cmp{gap:4px;margin-top:2px}
  .strip .cmp div{grid-template-columns:1fr}
  .strip .cmp u{height:8px}
  .cnone{padding:var(--s-6);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}

  .yg{row-gap:6px}
  .sea{font:800 var(--t-xs)/1 var(--round);color:var(--ink);padding:5px 10px;border-radius:var(--r-pill);background:color-mix(in srgb,var(--c) 22%,var(--glass-in));box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--c) 40%,transparent)}
  .yg .cell{height:var(--lh);overflow:visible}
  .yg :global(.cvf){animation-timing-function:var(--e-bounce)}
  .yg :global(.cvf){width:calc(var(--ch) * .72);height:var(--ch)}
  .yg .cell.many :global(.cvf:not(:first-child)){margin-left:calc(var(--ch) * -.42)}
</style>
