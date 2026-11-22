<script lang="ts">
  // Estadísticas › nivel Categoría: seis chips, las horas del día y la clasificación de los juegos de UNA categoría.
  // Todo sale solo de los títulos de esa categoría (LT/RT la cambia sin dar la vuelta, LB/RB cambia el periodo).
  import { onDestroy } from 'svelte';
  import { app, LIST_COLOR, STATUS_LABEL } from '../../lib/store.svelte';
  import { GS_ROW_H, GS_VIEW_H } from '../../lib/controller.svelte';
  import { GMETRICS, catPerList, catPerRange, catSummary, fmtDate, fmtHM, hourHistogram, nextRoundMilestone, peakHour, type CatPer, type GMetric } from '../../lib/stats';
  import { catTone, toneStyle } from '../../lib/chiptone';
  import { countUp, resetCounts } from '../../lib/countup';
  import TopBar from '../../components/TopBar.svelte';
  import Hints from '../../components/Hints.svelte';
  import StatChip from '../../components/StatChip.svelte';
  import CIcon from '../../components/CIcon.svelte';
  import MiniCover from '../../components/MiniCover.svelte';

  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const pi = $derived(app.p);
  const plat = $derived(app.lib.platforms[pi]);
  const tone = $derived(plat ? catTone(plat.id) : 4);
  const now = $derived(app.now.getTime());

  const pers = $derived(catPerList(app.lib, pi));
  const per = $derived<CatPer>(pers.includes(app.gsPer) ? app.gsPer : 'all');
  const pidx = $derived(pers.indexOf(per));
  const perLabel = (p: CatPer) => (p === 'all' ? 'Siempre' : p === 'd30' ? 'Últimos 30 días' : String(p));
  const perTxt = $derived(per === 'all' ? 'desde siempre' : per === 'd30' ? 'en los últimos 30 días' : `en ${per}`);
  const range = $derived(catPerRange(per, now));
  const uid = $derived(`gc:${pi}:${per}`);

  const S = $derived(catSummary(app.lib, pi, range, now));
  const M = $derived(nextRoundMilestone(S.allMs));
  const hours = $derived(hourHistogram(app.lib, pi, range));
  const peak = $derived(peakHour(hours));
  const hmax = $derived(Math.max(1, ...hours));
  const C = $derived(app.gsCat);
  const maxV = $derived(Math.max(1, ...C.rows.map((r) => r.v)));
  const mlabel = $derived(GMETRICS.find(([k]) => k === app.gsMetric)?.[1] ?? '');

  const nf = (n: number) => n.toLocaleString('es-ES');
  const parts = (ms: number) => { const m = Math.round(ms / 60000); return m < 60 ? { v: m, u: 'min' } : { v: Math.floor(m / 60), u: 'h' }; };
  const short = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}${d.getFullYear() !== app.now.getFullYear() ? ' ' + d.getFullYear() : ''}`; };
  const fmtV = (m: GMetric, v: number) => (m === 'sessions' ? String(v) : fmtHM(v));
  const unit = (m: GMetric, v: number) => (m === 'sessions' ? (v === 1 ? 'partida' : 'partidas') : '');
  const tm = $derived(parts(S.ms));
  const lg = $derived(S.longest ? parts(S.longest.ms) : null);
  onDestroy(() => resetCounts());
</script>

<TopBar crumbs={['Inicio', 'Estadísticas', plat?.name ?? '']} />

<div class="sx">
  <span class="gl cats">
    {#each app.statsPlats as p (p)}
      <span class="o" class:gem={p === pi} data-hact="gcat:{p}">{app.lib.platforms[p].name}</span>
    {/each}
  </span>
  <span class="gl" style="margin-left:auto">
    <span class="chev" class:dis={pidx <= 0} data-hact="lb"><svg viewBox="0 0 12 12"><path d="M7.5 2 3.5 6l4 4" /></svg></span>
    <span class="lbl">{perLabel(per)}</span>
    <span class="chev" class:dis={pidx < 0 || pidx >= pers.length - 1} data-hact="rb"><svg viewBox="0 0 12 12"><path d="M4.5 2l4 4-4 4" /></svg></span>
  </span>
</div>

{#if plat}
<div class="cvp">
 <div class="cpane gsl" style="{toneStyle(tone)};transform:none;right:48px">
  <div class="cl">
    <div class="lf">
      <div class="cg">
        <StatChip k="gc0" icon="clock" label="Tiempo jugado" {tone}>
          <div class="x-v"><span use:countUp={{ to: tm.v, key: `${uid}:A` }}>{tm.v}</span><small>{tm.u}</small></div>
          <div class="x-s">el <b>{S.share} %</b> de todo lo que juegas</div>
        </StatChip>
        <StatChip k="gc1" icon="cal" label="Días con actividad" {tone}>
          <div class="x-v"><span use:countUp={{ to: S.days, key: `${uid}:B` }}>{S.days}</span></div>
          <div class="x-s">de <b>{nf(S.ofDays)}</b> {S.ofDays === 1 ? 'día' : 'días'}</div>
        </StatChip>
        <StatChip k="gc2" icon="trophy" label="Terminados" {tone}>
          <div class="x-v"><span use:countUp={{ to: S.finished, key: `${uid}:C` }}>{S.finished}</span></div>
          <div class="x-s">{perTxt}</div>
        </StatChip>
        <StatChip k="gc3" icon="watch" label="Partida más larga" {tone}>
          {#if lg && S.longest}
            <div class="x-v">{lg.v}<small>{lg.u}</small>{#if lg.u === 'h' && S.longest.ms % 3.6e6 >= 60000}<span class="mn"> {String(Math.round((S.longest.ms % 3.6e6) / 60000)).padStart(2, '0')}</span>{/if}</div>
            <div class="x-s"><b>{S.longest.t.name}</b>, {short(S.longest.start)}</div>
          {:else}
            <div class="x-v">—</div>
            <div class="x-s">Sin partidas {perTxt}</div>
          {/if}
        </StatChip>
        <StatChip k="gc4" icon="cam" label="Capturas" {tone}>
          <div class="x-v"><span use:countUp={{ to: S.shots, key: `${uid}:E` }}>{S.shots}</span></div>
          <div class="x-s">{per === 'all' ? 'en toda la categoría' : `de ${nf(S.shotsAll)} en total`}</div>
        </StatChip>
        <StatChip k="gc5" icon="flag" label="Próximo hito" {tone}>
          <div class="x-v"><span use:countUp={{ to: M.h, key: `${uid}:F` }}>{M.h}</span><small>h</small></div>
          <div class="x-s">te faltan <b>{fmtHM(M.leftMs)}</b></div>
        </StatChip>
      </div>

      <StatChip k="gc6" icon="moon" label={peak >= 0 ? `A qué hora juegas · lo que más, a las ${String(peak).padStart(2, '0')}:00` : 'A qué hora juegas'} cls="hrs" {tone}>
        {#if peak >= 0}
          <div class="x-bars">
            {#each hours as v, h (uid + ':' + h)}
              <div class={h === peak ? 'x-top' : ''}><i style="--h:{Math.max(3, (v / hmax) * 100)}%;--i:{h}"></i><span>{h % 6 === 0 || h === 23 ? h : ''}</span></div>
            {/each}
          </div>
        {:else}
          <div class="x-s">Sin partidas {perTxt}.</div>
        {/if}
      </StatChip>
    </div>

    <div class="glass rkp">
      <div class="rkh">
        <h3>Clasificación</h3>
        <span class="sc" data-hact="view"><span class="btn sh">View</span><b>{mlabel}</b></span>
      </div>
      {#if !C.rows.length}
        <div class="rnone">Aún no hay partidas en esta categoría. El tiempo se cuenta al jugar desde el visor.</div>
      {:else}
        <div class="rkv" style="height:{GS_VIEW_H}px">
          <div class="rki" style="transform:translateY({-app.goff}px)">
            {#each C.rows as r, i (r.t.id)}
              {@const a = app.titleAcc(r.pi, r.ti).c}
              <div class="rk fx" class:f={app.focus === 'gs' + i} data-k="gs{i}" data-act="gs:{r.pi}:{r.ti}" style="height:{GS_ROW_H - 6}px;--i:{Math.min(i, 8)}">
                <span class="p">{i + 1}</span>
                <MiniCover t={r.t} acc={a} small />
                <span class="n">{r.t.name}
                  <small>{#if r.t.status}<i class="dot" style="background:{LIST_COLOR[r.t.status].c}"></i>{STATUS_LABEL[r.t.status]} · {/if}{r.ts.sessions} {r.ts.sessions === 1 ? 'partida' : 'partidas'}</small>
                </span>
                <span class="b"><u style="--w:{(r.v / maxV) * 100}%"></u></span>
                <span class="v">{fmtV(app.gsMetric, r.v)}{#if unit(app.gsMetric, r.v)}<small>&nbsp;{unit(app.gsMetric, r.v)}</small>{/if}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
 </div>
</div>
{:else}
  <div class="stnone">Aún no hay partidas en ninguna categoría. El tiempo se cuenta al jugar desde el visor.</div>
{/if}

<Hints list={[['A', 'Ver juego', 'a'], ['LT/RT', 'Categoría', 'rt'], ['LB/RB', 'Periodo', 'rb'], ['View', 'Ordenar', 'view'], ['Y', 'Tu año', 'y'], ['B', 'Volver', 'b']]} />

<style>
  /* Todo en una pantalla (1280×720): a la izquierda seis chips y las horas del día; a la derecha, la clasificación. */
  .sx .cats{max-width:calc(100% - 330px);overflow:hidden}
  .cl{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr);gap:var(--s-4);height:476px}
  .lf{display:flex;flex-direction:column;gap:var(--s-4);min-width:0}
  .cg{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:var(--s-4)}
  .gsl :global(.st.st){height:112px}
  .gsl :global(.st.st.hrs){height:auto;flex:1;min-height:0}
  .gsl :global(.st .face){padding:10px 14px;gap:6px}
  .gsl :global(.cg .x-v){font-size:var(--t-2xl)}
  .gsl :global(.cg .x-v small){font-size:var(--t-l)}
  .gsl :global(.cg .x-s){white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .gsl :global(.hrs .x-bars){flex:1;min-height:0;height:auto;gap:3px}
  .gsl :global(.hrs .x-bars span){min-height:10px}
  .mn{font:800 var(--t-xl)/1 var(--round);color:var(--ink)}

  /* Clasificación */
  .rkp{position:relative;display:flex;flex-direction:column;padding:14px 14px 10px;min-width:0;box-sizing:border-box}
  .rkh{display:flex;align-items:center;justify-content:space-between;gap:var(--s-3);height:40px;margin-bottom:6px}
  .rkh h3{margin:0;font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .sc{display:flex;align-items:center;gap:var(--s-2);height:32px;padding:0 12px 0 4px;border-radius:var(--r-pill);background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line);cursor:pointer;font:800 var(--t-s)/1 var(--round);color:var(--ink)}
  .sc .btn{height:24px;min-width:0}
  .rkv{overflow:hidden;margin:0 -4px;padding:0 4px}
  .rki{transition:transform .18s ease-out}
  .rk{display:grid;grid-template-columns:26px 26px minmax(0,1fr) 78px 70px;align-items:center;gap:10px;margin-bottom:6px;padding:0 12px;border-radius:var(--r-m);box-sizing:border-box;cursor:pointer;background:var(--glass-in);
    animation:gsrow .4s var(--e-bounce) backwards;animation-delay:calc(var(--i) * 40ms + 380ms)}
  .rk.f{background:color-mix(in srgb,var(--v1) 14%,var(--tint-base));box-shadow:inset 0 0 0 3px var(--v1),0 8px 16px -10px var(--v1)}
  .rk .p{font:800 var(--t-s)/1 var(--round);color:var(--ink-2);text-align:center}
  .rk .n{display:block;min-width:0;font:800 var(--t-m)/1.15 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .rk .n small{display:flex;align-items:center;gap:5px;font:600 var(--t-xs)/1.2 var(--round);color:var(--ink-3);white-space:nowrap;overflow:hidden}
  .rk .dot{display:inline-block;flex:none;width:8px;height:8px;border-radius:50%}
  .rk .b{position:relative;height:10px;border-radius:var(--r-s);background:var(--line);overflow:hidden}
  .rk .b u{position:absolute;left:0;top:0;bottom:0;width:var(--w);border-radius:var(--r-s);background:linear-gradient(90deg,var(--v1),var(--v2));transform-origin:left;animation:gsgrow .8s var(--e-bounce) backwards;animation-delay:calc(var(--i) * 40ms + 420ms)}
  .rk .v{font:800 var(--t-m)/1.1 var(--round);text-align:right;white-space:nowrap;color:var(--ink)}
  .rk .v small{font:600 var(--t-xs)/1 var(--round);color:var(--ink-3)}
  .rnone{padding:var(--s-5) var(--s-3);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}

  /* Entrada escalonada de los chips y del panel */
  .gsl :global(.cg .st){animation:gsrise .5s var(--e-bounce) backwards}
  .gsl :global(.cg .st:nth-child(2)){animation-delay:55ms}
  .gsl :global(.cg .st:nth-child(3)){animation-delay:110ms}
  .gsl :global(.cg .st:nth-child(4)){animation-delay:165ms}
  .gsl :global(.cg .st:nth-child(5)){animation-delay:220ms}
  .gsl :global(.cg .st:nth-child(6)){animation-delay:275ms}
  .gsl :global(.st.hrs){animation:gsrise .5s var(--e-bounce) 330ms backwards}
  .rkp{animation:gsrise .5s var(--e-bounce) 120ms backwards}
  @keyframes gsrise{from{opacity:0;transform:translateY(16px) scale(.96)}}
  @keyframes gsrow{from{opacity:0;transform:translateX(18px)}}
  @keyframes gsgrow{from{transform:scaleX(0)}}
  @media (prefers-reduced-motion: reduce){
    .gsl :global(.cg .st),.gsl :global(.st.hrs),.rkp,.rk,.rk .b u{animation:none}
  }
</style>
