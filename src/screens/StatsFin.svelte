<script lang="ts">
  // Estadísticas › Terminados: estantería del año (desglose por categoría), medalla del último terminado,
  // comparación con el año anterior y «Mes a mes», un carril por categoría; nunca se mezclan.
  import { app } from '../lib/store.svelte';
  import { finishedYear, fmtHM, type FinRef } from '../lib/stats';
  import { catIcon, catTone, toneStyle } from '../lib/chiptone';
  import { countUp } from '../lib/countup';
  import ChipPane from '../components/ChipPane.svelte';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import CoverBg from '../components/CoverBg.svelte';

  const ARC = ['rojo', 'naranja', 'amarillo', 'verde', 'azul', 'anil', 'violeta'];
  const MH = ['Ene', 'Feb', 'Mar', 'Abr', 'May', 'Jun', 'Jul', 'Ago', 'Sep', 'Oct', 'Nov', 'Dic'];
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const F = $derived(finishedYear(app.lib, app.finYear));
  const fid = $derived(`fin${F.year}`);
  const plat = (pi: number) => app.lib.platforms[pi];
  const arc = (pi: number) => `var(--arc-${ARC[catTone(plat(pi).id) % 7]})`;
  const act = (r: FinRef) => `sr:${r.pi}:${r.ti}`;
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}`; };
  const ago = (ms: number) => {
    const day = (x: number) => { const d = new Date(x); return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 864e5; };
    const d = day(app.now.getTime()) - day(ms);
    return d <= 0 ? 'hoy' : d === 1 ? 'ayer' : d < 60 ? `hace ${d} días` : `el ${dm(ms)}`;
  };

  /* M · estantería: los últimos terminados del año, el más reciente marcado */
  const all = $derived(F.lanes.flatMap((l) => l.months.flat()).sort((a, b) => a.finishedMs - b.finishedMs));
  const shelf = $derived(all.slice(-9));

  /* G · frente al año anterior */
  const diff = $derived(F.count - F.prevCount);
  const cmpMax = $derived(Math.max(F.count, F.prevCount, 1));

  /* Mes a mes: cada portada en su mes; las claves siguen el orden de los carriles */
  const nowY = $derived(app.now.getFullYear()), nowM = $derived(app.now.getMonth());
  const fut = (m: number) => F.year > nowY || (F.year === nowY && m > nowM);
  const keyed = $derived.by(() => {
    let i = 3;
    return F.lanes.map((l) => ({ l, months: l.months.map((m) => m.map((r) => ({ r, k: `fn${i++}` }))) }));
  });
  const det = $derived(keyed.flatMap((x) => x.months.flat()).find((c) => c.k === app.focus)?.r ?? F.last);
</script>

<ChipPane>
  <h3 class="sh"><CIcon name="check" />El año<span>todas las categorías, cada una en su carril</span></h3>
  <div class="cgrid">
    <StatChip k="fn0" icon="check" label="Terminados en {F.year}" wide tone={6} act={F.last ? act(F.last) : undefined}>
      {#if F.count}
        <div class="shelf">
          {#each shelf as r, i (r.pi + ':' + r.ti)}
            <CoverBg t={r.t} cls="cv{r === F.last ? ' new' : ''}" style="--i:{i}" title={r.t.name} />
          {/each}
          <div class="cnt">
            <div class="x-v"><span use:countUp={{ to: F.count, key: `fin:M:${fid}` }}>{F.count}</span></div>
            <span class="tag2"><i></i>{F.count === 1 ? 'Terminado' : 'Terminados'}</span>
            <div class="por">{#each F.lanes as l (l.pi)}<span style="--c:{arc(l.pi)}" title={l.name}><i></i>{l.count}</span>{/each}</div>
          </div>
        </div>
      {:else}
        <div class="x-s">Ningún juego terminado en {F.year}. Al marcar uno como terminado aparecerá aquí.</div>
      {/if}
    </StatChip>

    <StatChip k="fn1" icon="trophy" label="Último terminado" tone={6} act={F.last ? act(F.last) : undefined}>
      {#if F.last}
        <div class="medw">
          <svg class="x-medal" viewBox="0 0 96 96" aria-hidden="true">
            <g class="rib"><path d="M30 6h14l10 28H40z" fill="color-mix(in srgb,var(--done) 55%,var(--tint-base))" /><path d="M66 6H52L42 34h14z" fill="var(--done)" /></g>
            <g class="disc"><circle cx="48" cy="58" r="28" fill="var(--done)" /><circle cx="48" cy="58" r="22" fill="color-mix(in srgb,var(--done) 60%,var(--tint-base))" /><path class="x-chk" d="M37 58l7 7 14-15" fill="none" stroke="#fff" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" /><path class="glint" d="M30 46a22 22 0 0 1 12-10" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round" /></g>
          </svg>
          <div class="x-s">
            <b style="font-size:var(--t-l)">{F.last.t.name}</b><br />
            {plat(F.last.pi).name} · {ago(F.last.finishedMs)}<br />
            <span class="tag2" style="margin-top:6px"><i></i>Terminado</span>
          </div>
        </div>
      {:else}
        <div class="x-s">Aún no hay terminados este año.</div>
      {/if}
    </StatChip>

    <StatChip k="fn2" icon="swap" label="Frente a {F.year - 1}" tone={4}>
      <div class="vrow">
        <div class="x-v"><span use:countUp={{ to: F.count, key: `fin:G:${fid}` }}>{F.count}</span><small>{F.count === 1 ? 'título' : 'títulos'}</small></div>
        {#if diff !== 0}<span class="delta" class:dn={diff < 0}><CIcon name={diff < 0 ? 'down' : 'up'} class="" />{Math.abs(diff)} {diff > 0 ? 'más' : 'menos'}</span>{/if}
      </div>
      <div class="cmp">
        <div class="now"><span>{F.year}</span><u><i style="--w:{(F.count / cmpMax) * 100}%"></i></u><b>{F.count}</b></div>
        <div><span>{F.year - 1}</span><u><i style="--w:{(F.prevCount / cmpMax) * 100}%"></i></u><b>{F.prevCount}</b></div>
      </div>
    </StatChip>
  </div>

  <h3 class="sh"><CIcon name="cal" />Mes a mes<span>cada portada, en el mes en que la terminaste</span></h3>
  {#if F.count}
    <div class="lanes glass">
      <div class="face">
        <div class="yg">
          <div></div>
          {#each MH as m, j}<div class="mh" class:now={F.year === nowY && j === nowM}>{m}</div>{/each}
          {#each keyed as x (x.l.pi)}
            {@const tone = toneStyle(catTone(x.l.id))}
            <div class="l" style={tone}><CIcon name={catIcon(x.l.id)} class="" />{x.l.name}<em>{x.l.count}</em></div>
            {#each x.months as cells, j}
              <div class="cell" class:fut={fut(j)} style={tone}>
                {#each cells as c, n (c.k)}<CoverBg t={c.r.t} cls="cvf fx" k={c.k} act={act(c.r)} style="--i:{j + n}" title={c.r.t.name} />{/each}
              </div>
            {/each}
          {/each}
        </div>
        {#if det}
          <div class="det" style={toneStyle(catTone(plat(det.pi).id))}>
            <i class="cdot"></i><b>{det.t.name}</b>· {plat(det.pi).name} · {dm(det.finishedMs)}{#if det.ms > 0} · {fmtHM(det.ms)} jugadas{/if}
            <span class="tag2" style="margin-left:auto"><i></i>Terminado</span>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="cnone">Sin terminados en {F.year}.</div>
  {/if}
</ChipPane>

<style>
  .cnone{padding:var(--s-6);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}
  .yg .cell{flex-wrap:wrap;overflow:visible}
</style>
