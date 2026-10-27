<script lang="ts">
  // Estadísticas: clasificación de juegos por periodo (podio con hero y logo + lista con mini portadas)
  // y calendario de los últimos 6 meses con el detalle del día enfocado.
  import { app } from '../lib/store.svelte';
  import { RANK_VIEW_H, rankH, rankY } from '../lib/controller.svelte';
  import { CAL_WEEKS, PERIOD_KINDS, WDAYS, addDays, dayPlays, fmtClock, fmtHM, fmtLongDay, prevName } from '../lib/stats';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import HeroStrip from '../components/HeroStrip.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import Year from './Year.svelte';

  const R = $derived(app.rank);
  const rows = $derived(R.rows);
  const maxMs = $derived(Math.max(1, ...rows.map((r) => r.ms)));
  const all = $derived(app.statsKind === 'all');
  const now = $derived(app.now.getTime());
  const prevN = $derived(prevName(app.statsKind, app.statsOff, now));
  const avg = $derived(R.n ? R.ms / R.n : 0);
  // Días transcurridos del periodo (el actual aún no ha terminado).
  const elapsed = $derived.by(() => {
    if (all) return 0;
    const to = Math.min(R.range.to, addDays(new Date(now).setHours(0, 0, 0, 0), 1));
    return Math.max(0, Math.round((to - R.range.from) / 864e5));
  });
  const pname = (pi: number) => app.lib.platforms[pi]?.name ?? '';
  const plus = (d: number, f: (x: number) => string, unit = '') =>
    d > 0 ? { c: 'up', t: `▲ ${f(d)}${unit} más` } : d < 0 ? { c: 'down', t: `▼ ${f(-d)}${unit} menos` } : { c: 'eq', t: '= igual' };
  const delta = (ms: number, prev: number) =>
    all ? null : prev === 0 ? (ms > 0 ? { c: 'up', t: 'nuevo' } : null) : ms === prev ? { c: 'eq', t: '=' } : { c: ms > prev ? 'up' : 'down', t: `${ms > prev ? '▲' : '▼'} ${fmtHM(Math.abs(ms - prev))}` };
  const canNewer = $derived(!all && app.statsOff < 0);
  const canOlder = $derived(!all && R.older);

  // Calendario
  const C = $derived(app.cal);
  // Tonos: nada, menos de 30 min, menos de 1 h, menos de 2 h, más.
  const lvl = (ms: number) => (ms <= 0 ? 0 : ms < 18e5 ? 1 : ms < 36e5 ? 2 : ms < 72e5 ? 3 : 4);
  const hours = (ms: number) => (ms <= 0 ? '' : ms < 36e5 ? `${Math.round(ms / 6e4)} min` : `${Math.round(ms / 36e5)} h`);
  const CELL = 22, CX = 40, CY = 24;
  const selDay = $derived(app.focus.startsWith('cd') ? C.days[+app.focus.slice(2)] : undefined);
  const plays = $derived(selDay ? dayPlays(app.lib, selDay.start) : []);
  const monthLabels = $derived.by(() => {
    const out: Array<{ x: number; t: string }> = [];
    let last = -1;
    for (let w = 0; w < CAL_WEEKS; w++) {
      const d = new Date(C.days[w * 7].start);
      if (d.getMonth() !== last) {
        last = d.getMonth();
        if (w < CAL_WEEKS - 1) out.push({ x: CX + w * CELL, t: d.toLocaleDateString('es', { month: 'short' }).replace('.', '') });
      }
    }
    return out;
  });
  const monthMax = $derived(Math.max(1, ...C.months.map((m) => m.ms)));
  const calRange = $derived.by(() => {
    const a = new Date(C.days[0].start), b = new Date(C.days.at(-1)!.start);
    const f = (d: Date) => d.toLocaleDateString('es', { month: 'short', year: 'numeric' }).replace('.', '');
    return app.calOff === 0 ? 'Últimos 6 meses' : `${f(a)} – ${f(b)}`;
  });
  const fmtRange = (a: number, b: number) => {
    const f = (x: number) => new Date(x).toLocaleDateString('es', { day: 'numeric', month: 'short' }).replace('.', '');
    return a === b ? f(a) : `del ${f(a)} al ${f(b)}`;
  };
  const dayTotal = $derived(plays.reduce((a, p) => a + p.ms, 0));
  const SHOW = 4;
</script>

<TopBar crumbs={['Inicio', 'Estadísticas', app.statsTab === 'rank' ? 'Clasificación' : app.statsTab === 'cal' ? 'Calendario' : 'Resumen anual']} />

<div class="sttop">
  {#if app.statsTab === 'rank'}
    <div class="stabs pill">
      <span class="btn sh" data-hact="lb">LB</span>
      {#each PERIOD_KINDS as [k, l]}<span class="t" class:on={app.statsKind === k}>{l}</span>{/each}
      <span class="btn sh" data-hact="rb">RB</span>
    </div>
    <div class="stper pill" data-hact="x">
      <span class="btn sh">X</span>
      <b>{pname(app.statsPlat) || 'Sin partidas'}</b>
    </div>
    {#if !all}
      <div class="stper pill">
        <span class="btn sh" class:off={!canOlder} data-hact="lt">LT</span>
        <b>{R.range.label}</b>
        <span class="btn sh" class:off={!canNewer} data-hact="rt">RT</span>
      </div>
    {/if}
  {:else if app.statsTab === 'year'}
    {@const yi = app.years.indexOf(app.yearNow)}
    <div class="stper pill">
      <span class="btn sh" class:off={yi <= 0} data-hact="lt">LT</span>
      <b>{app.yearNow}</b>
      <span class="btn sh" class:off={yi < 0 || yi >= app.years.length - 1} data-hact="rt">RT</span>
    </div>
  {:else}
    <div class="stper pill">
      <span class="btn sh" class:off={!C.older} data-hact="lt">LT</span>
      <b>{calRange}</b>
      <span class="btn sh" class:off={app.calOff >= 0} data-hact="rt">RT</span>
    </div>
  {/if}
  <div class="stview pill" data-hact="y">
    <span class="btn y">Y</span>
    <span class="t" class:on={app.statsTab === 'rank'}>Clasificación</span>
    <span class="t" class:on={app.statsTab === 'cal'}>Calendario</span>
    <span class="t" class:on={app.statsTab === 'year'}>Resumen anual</span>
  </div>
</div>

{#if app.statsTab === 'year'}
  <Year />
  <Hints list={[['A', 'Siguiente', 'a'], ['LT', 'Año anterior', 'lt'], ['RT', 'Año siguiente', 'rt'], ['Y', 'Clasificación', 'y'], ['B', 'Volver', 'b']]} />
{:else if app.statsTab === 'rank'}
  <div class="stsum">
    <div class="rbox">
      <div class="k">Tiempo jugado</div>
      <div class="v acc">{fmtHM(R.ms)}</div>
      {#if !all}{@const d = plus(R.ms - R.prevMs, fmtHM)}<div class="s {d.c}">{d.t}{d.c === 'eq' ? '' : ` que ${prevN}`}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Partidas</div>
      <div class="v">{R.n}</div>
      {#if !all}{@const d = plus(R.n - R.prevN, String)}<div class="s {d.c}">{d.t}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Media por partida</div>
      <div class="v">{R.n ? fmtHM(avg) : '—'}</div>
    </div>
    <div class="rbox">
      {#if all}
        <div class="k">Juegos jugados</div><div class="v">{rows.length}</div>
      {:else}
        <div class="k">Días con partida</div><div class="v">{R.days} <small>de {elapsed}</small></div>
      {/if}
    </div>
  </div>

  {#if !rows.length}
    <div class="stnone">{all ? 'Aún no hay partidas registradas' : `Sin partidas · ${R.range.label}`}. El tiempo se cuenta al jugar desde el visor.</div>
  {/if}

  {#if rows.length}
    <div class="strows">
      <div class="inner" style="transform:translateY({-app.goff}px)">
        {#each rows as r, i (r.t.id)}
          {@const a = app.titleAcc(r.pi, r.ti).c}
          {@const d = delta(r.ms, r.prevMs)}
          {#if i < 3}
            <!-- Top 3: el hero se funde hacia el blanco y el logo va encima. -->
            <div class="rrow hrow fx" class:f={app.focus === 'sr' + i} data-k="sr{i}" data-act="sr:{r.pi}:{r.ti}" style="--g:{a}">
              <span class="medal m{i}">{i + 1}º</span>
              <HeroStrip t={r.t} />
              <span class="bar"><i style="width:{(r.ms / maxMs) * 100}%"></i>{#if r.prevMs > 0 && r.prevMs <= maxMs}<u style="left:{(r.prevMs / maxMs) * 100}%"></u>{/if}</span>
              <span class="tm">{fmtHM(r.ms)}<small>&nbsp;·&nbsp;{r.n} {r.n === 1 ? 'partida' : 'partidas'}</small></span>
              <span class="dl {d?.c ?? ''}">{d?.t ?? ''}</span>
            </div>
          {:else}
            <div class="rrow fx" class:f={app.focus === 'sr' + i} data-k="sr{i}" data-act="sr:{r.pi}:{r.ti}" style="--g:{a}">
              <span class="pos">{i + 1}º</span>
              <MiniCover t={r.t} acc={a} />
              <span class="nm">{r.t.name}<small>{pname(r.pi)}</small></span>
              <span class="bar"><i style="width:{(r.ms / maxMs) * 100}%"></i>{#if r.prevMs > 0 && r.prevMs <= maxMs}<u style="left:{(r.prevMs / maxMs) * 100}%"></u>{/if}</span>
              <span class="tm">{fmtHM(r.ms)}<small>&nbsp;·&nbsp;{r.n} {r.n === 1 ? 'partida' : 'partidas'}</small></span>
              <span class="dl {d?.c ?? ''}">{d?.t ?? ''}</span>
            </div>
          {/if}
        {/each}
      </div>
    </div>
    {#if rankY(rows.length - 1) + rankH(rows.length - 1) > RANK_VIEW_H && app.focus.startsWith('sr')}
      <div class="stmore">Puesto {+app.focus.slice(2) + 1} de {rows.length}</div>
    {/if}
  {/if}

  <Hints list={[['A', 'Ver juego', 'a'], ['X', 'Categoría', 'x'], ['Y', 'Calendario', 'y'], ['B', 'Volver', 'b']]} />
{:else}
  <div class="stsum">
    <div class="rbox"><div class="k">Días con partida</div><div class="v acc">{C.played} <small>de {C.elapsed}</small></div></div>
    <div class="rbox"><div class="k">Racha actual</div><div class="v">{C.curStreak} <small>{C.curStreak === 1 ? 'día' : 'días'}</small></div></div>
    <div class="rbox">
      <div class="k">Mejor racha</div><div class="v">{C.best.n} <small>{C.best.n === 1 ? 'día' : 'días'}</small></div>
      {#if C.best.n}<div class="s eq">{fmtRange(C.best.from, C.best.to)}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Día favorito</div>
      {#if C.fav}<div class="v">{WDAYS[C.fav.wd][0].toUpperCase() + WDAYS[C.fav.wd].slice(1)}</div><div class="s eq">{fmtHM(C.fav.avg)} de media</div>
      {:else}<div class="v">—</div>{/if}
    </div>
  </div>

  <div class="stpanel stcal">
    <div class="sth">Tiempo jugado por día
      <span class="leg">Menos {#each [0, 1, 2, 3, 4] as l}<i class="l{l}"></i>{/each} Más</span>
    </div>
    <div class="heat">
      {#each ['L', '', 'X', '', 'V', '', 'D'] as w, i}<span class="wl" style="top:{CY + i * CELL + 2}px">{w}</span>{/each}
      {#each monthLabels as m}<span class="ml" style="left:{m.x}px">{m.t}</span>{/each}
      {#each C.days as d, i (d.start)}
        <i class="hc l{lvl(d.ms)}" class:fx={!d.future} class:fut={d.future} class:f={app.focus === 'cd' + i} class:today={i === C.today}
          data-k="cd{i}" data-act="cd" style="left:{CX + Math.floor(i / 7) * CELL}px;top:{CY + (i % 7) * CELL}px"></i>
      {/each}
    </div>
    <div class="months">
      {#each C.months as m}
        <div><b>{hours(m.ms)}</b><i style="height:{Math.max(m.ms ? 4 : 0, (m.ms / monthMax) * 64)}px"></i>{m.label}</div>
      {/each}
    </div>
  </div>

  <div class="stpanel stday">
    {#if selDay}
      <div class="sth">{fmtLongDay(selDay.start, now)}</div>
      {#if plays.length}
        {#each plays.slice(0, SHOW) as p (p.t.id + p.start)}
          {@const a = app.titleAcc(p.pi, p.ti).c}
          <div class="ses">
            <span class="tm">{fmtClock(p.start)} – {fmtClock(p.end)}</span>
            <MiniCover t={p.t} acc={a} small />
            <span class="nm">{p.t.name}<small>{pname(p.pi)}{p.shots ? ` · ${p.shots} ${p.shots === 1 ? 'captura' : 'capturas'}` : ''}</small></span>
            <span class="du">{fmtHM(p.ms)}</span>
          </div>
        {/each}
        {#if plays.length > SHOW}<div class="smore">y {plays.length - SHOW} {plays.length - SHOW === 1 ? 'partida' : 'partidas'} más</div>{/if}
        <div class="dtot"><span>{plays.length} {plays.length === 1 ? 'partida' : 'partidas'} · {new Set(plays.map((p) => p.t.id)).size} {new Set(plays.map((p) => p.t.id)).size === 1 ? 'juego' : 'juegos'}</span><span>{fmtHM(dayTotal)}</span></div>
      {:else}
        <div class="snone">Ese día no jugaste.</div>
      {/if}
    {/if}
  </div>

  <Hints list={[['LT', 'Antes', 'lt'], ['RT', 'Después', 'rt'], ['Y', 'Resumen anual', 'y'], ['B', 'Volver', 'b']]} />
{/if}
