<script lang="ts">
  // Calendario: mapa de calor de 26 semanas, rachas, día favorito, horas por mes y las partidas del día tocado.
  import { ICON } from '@shared/icons';
  import { CAL_WEEKS, WDAYS, calendar, dayPlays, fmtClock, fmtHM, fmtLongDay } from '@shared/stats';
  import MiniCover from '../../components/MiniCover.svelte';
  import { MI } from '../../lib/icons';
  import { app } from '../../lib/store.svelte';

  const C = $derived(calendar(app.lib, app.calOff, app.now));
  // Tonos: nada, menos de 30 min, menos de 1 h, menos de 2 h, más.
  const lvl = (ms: number) => (ms <= 0 ? 0 : ms < 18e5 ? 1 : ms < 36e5 ? 2 : ms < 72e5 ? 3 : 4);
  let sel = $state<number | null>(null);
  const selIdx = $derived(sel ?? (C.today >= 0 ? C.today : C.days.length - 1));
  const day = $derived(C.days[selIdx]);
  const plays = $derived(day ? dayPlays(app.lib, day.start) : []);
  const dayTotal = $derived(plays.reduce((a, p) => a + p.ms, 0));
  const monthLabels = $derived.by(() => {
    const out: Array<{ w: number; t: string }> = [];
    let last = -1;
    for (let w = 0; w < CAL_WEEKS; w++) {
      const d = new Date(C.days[w * 7].start);
      if (d.getMonth() !== last) {
        last = d.getMonth();
        if (w < CAL_WEEKS - 2) out.push({ w, t: d.toLocaleDateString('es', { month: 'short' }).replace('.', '') });
      }
    }
    return out;
  });
  const monthMax = $derived(Math.max(1, ...C.months.map((m) => m.ms)));
  const range = $derived.by(() => {
    if (app.calOff === 0) return 'Últimos 6 meses';
    const f = (x: number) => new Date(x).toLocaleDateString('es', { month: 'short', year: 'numeric' }).replace('.', '');
    return `${f(C.days[0].start)} – ${f(C.days.at(-1)!.start)}`;
  });
  const fmtRange = (a: number, b: number) => {
    const f = (x: number) => new Date(x).toLocaleDateString('es', { day: 'numeric', month: 'short' }).replace('.', '');
    return a === b ? f(a) : `del ${f(a)} al ${f(b)}`;
  };
  function nav(d: number) { app.calOff += d; sel = null; }
</script>

<div class="per">
  <button class="round pill" disabled={!C.older} onclick={() => nav(-1)} aria-label="Anteriores"><i class="dk">{@html ICON.chevL}</i></button>
  <b>{range}</b>
  <button class="round pill" disabled={app.calOff >= 0} onclick={() => nav(1)} aria-label="Siguientes"><i class="dk">{@html ICON.chevR}</i></button>
</div>

<div class="glass panel heat">
  <span class="face">
    <div class="months">{#each monthLabels as m (m.w)}<span style="left:calc(18px + (100% - 18px) * {m.w / CAL_WEEKS})">{m.t}</span>{/each}</div>
    <div class="map">
      <div class="wd">{#each ['L', '', 'X', '', 'V', '', 'D'] as l}<span>{l}</span>{/each}</div>
      {#each Array(CAL_WEEKS) as _, w}
        <div class="wk">
          {#each Array(7) as __, d}
            {@const k = w * 7 + d}
            {@const x = C.days[k]}
            <button class="c l{lvl(x.ms)}" class:fu={x.future} class:td={k === C.today} class:sel={k === selIdx} disabled={x.future} onclick={() => (sel = k)} aria-label={fmtLongDay(x.start)}></button>
          {/each}
        </div>
      {/each}
    </div>
    <div class="legend">Menos<i class="c l0"></i><i class="c l1"></i><i class="c l2"></i><i class="c l3"></i><i class="c l4"></i>Más</div>
  </span>
</div>

{#if day}
  <div class="glass panel dayp">
    <span class="face">
      <div class="dh"><b>{fmtLongDay(day.start)}</b><span>{dayTotal ? fmtHM(dayTotal) : 'Sin partidas'}</span></div>
      {#each plays as p, k (k)}
        {@const a = app.titleAcc(p.pi, p.ti)}
        <button class="pl" onclick={() => app.openTitle(p.t.id)}>
          <MiniCover t={p.t} acc={a.c} size={28} />
          <span class="pn"><b>{p.t.name}</b><small>{fmtClock(p.start)} – {fmtClock(p.end)}{p.shots ? ` · ${p.shots} ${p.shots === 1 ? 'captura' : 'capturas'}` : ''}</small></span>
          <span class="pt" style="color:{a.c}">{fmtHM(p.ms)}</span>
        </button>
      {/each}
    </span>
  </div>
{/if}

<div class="facts">
  <div class="glass panel f"><span class="face"><i style="color:#F27A1A">{@html MI.flame}</i><small>Racha actual</small><b>{C.curStreak} {C.curStreak === 1 ? 'día' : 'días'}</b></span></div>
  <div class="glass panel f"><span class="face"><i style="color:#E8384F">{@html MI.flame}</i><small>Mejor racha</small><b>{C.best.n} {C.best.n === 1 ? 'día' : 'días'}</b>{#if C.best.n}<em>{fmtRange(C.best.from, C.best.to)}</em>{/if}</span></div>
  <div class="glass panel f"><span class="face"><small>Día favorito</small><b>{C.fav ? WDAYS[C.fav.wd][0].toUpperCase() + WDAYS[C.fav.wd].slice(1) : '—'}</b>{#if C.fav}<em>media {fmtHM(C.fav.avg)}</em>{/if}</span></div>
  <div class="glass panel f"><span class="face"><small>Días jugados</small><b>{C.played}<span class="of"> de {C.elapsed}</span></b></span></div>
</div>

{#if C.months.length}
  <div class="sect">Horas por mes</div>
  <div class="glass panel mon"><span class="face">
    {#each C.months as m}
      <div class="mr"><span class="ml">{m.label}</span><span class="bar"><i style="width:{(m.ms / monthMax) * 100}%"></i></span><span class="mv">{m.ms ? fmtHM(m.ms) : '—'}</span></div>
    {/each}
  </span></div>
{/if}

<style>
  .per{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:2px 14px 10px}
  .per b{font-size:16px;font-weight:800;color:var(--ink)}
  .per button:disabled{opacity:.35}
  .dk{width:18px;height:18px;display:block;filter:invert(.4)}
  .dk :global(svg){width:18px;height:18px;display:block}
  .heat{margin:0 12px;--r:22px;--bw:6px}
  .heat .face{padding:24px 10px 10px}
  .months{position:absolute;left:10px;right:10px;top:6px;height:14px}
  .months span{position:absolute;font-size:10.5px;font-weight:800;color:var(--dim)}
  .map{display:grid;grid-template-columns:14px repeat(26,1fr);gap:2px}
  .wd,.wk{display:grid;grid-template-rows:repeat(7,auto);gap:2px}
  .wd span{font-size:8.5px;font-weight:800;color:var(--dim2);line-height:1;display:flex;align-items:center;aspect-ratio:1/1}
  .c{display:block;width:100%;aspect-ratio:1;border-radius:2.5px;padding:0;background:#dde1e7}
  .c.l1{background:color-mix(in srgb,var(--acc) 30%,#e5e8ee)}.c.l2{background:color-mix(in srgb,var(--acc) 55%,#e5e8ee)}
  .c.l3{background:color-mix(in srgb,var(--acc) 78%,#e5e8ee)}.c.l4{background:var(--acc)}
  .c.fu{background:transparent;box-shadow:inset 0 0 0 1px rgba(160,168,182,.25)}
  .c.td{box-shadow:0 0 0 1.5px var(--ink2)}
  .c.sel{box-shadow:0 0 0 2px #fff,0 0 0 3.5px var(--acc);position:relative;z-index:1}
  .legend{display:flex;align-items:center;justify-content:flex-end;gap:3px;margin-top:8px;font-size:10px;font-weight:700;color:var(--dim)}
  .legend .c{width:10px;margin:0 1px}
  .dayp{margin:10px 12px 0;--r:22px;--bw:6px}
  .dayp .face{padding:12px 12px 6px}
  .dh{display:flex;align-items:baseline;justify-content:space-between;gap:8px;margin-bottom:6px}
  .dh b{font-size:15px;font-weight:800}
  .dh span{font-size:13px;font-weight:800;color:var(--acc)}
  .pl{display:flex;align-items:center;gap:10px;width:100%;padding:6px 0;text-align:left;border-top:1px solid rgba(160,168,182,.25)}
  .pn{flex:1;min-width:0}
  .pn b{display:block;font-size:14px;font-weight:800;color:var(--ink2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .pn small{font-size:11.5px;font-weight:700;color:var(--dim)}
  .pt{font-size:14px;font-weight:800}
  .facts{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:12px 12px 0}
  .f{--r:20px;--bw:6px}
  .f .face{display:flex;flex-direction:column;gap:1px;padding:10px 12px;min-height:84px;position:relative}
  .f i{position:absolute;right:10px;top:10px;width:20px;height:20px}
  .f i :global(svg){width:20px;height:20px;display:block}
  .f small{font-size:11px;font-weight:800;color:var(--dim);text-transform:uppercase;letter-spacing:.04em}
  .f b{font-size:21px;font-weight:800}
  .f .of{font-size:13px;color:var(--dim)}
  .f em{font-style:normal;font-size:11.5px;font-weight:700;color:var(--dim)}
  .mon{margin:0 12px;--r:22px;--bw:6px}
  .mon .face{padding:10px 12px;display:flex;flex-direction:column;gap:7px}
  .mr{display:flex;align-items:center;gap:10px}
  .ml{width:46px;font-size:12px;font-weight:800;color:var(--dim)}
  .bar{flex:1;height:10px;border-radius:5px;background:rgba(160,168,182,.22);overflow:hidden}
  .bar i{display:block;height:100%;border-radius:5px;background:linear-gradient(90deg,color-mix(in srgb,var(--acc) 55%,white),var(--acc))}
  .mv{width:58px;text-align:right;font-size:12px;font-weight:800;color:var(--ink2)}
</style>
