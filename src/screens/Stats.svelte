<script lang="ts">
  // Estadísticas: clasificación de juegos por periodo (podio con hero y logo + lista con mini portadas)
  // y calendario de los últimos 6 meses con el detalle del día enfocado.
  import { app } from '../lib/store.svelte';
  import { RANK_VIEW_H, rankH, rankY } from '../lib/controller.svelte';
  import { CAL_WEEKS, STATS_KINDS, WDAYS, addDays, dayPlays, fmtClock, fmtHM, fmtLongDay } from '../lib/stats';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import HeroStrip from '../components/HeroStrip.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import Year from './Year.svelte';

  const KIND_LABEL: Record<string, string> = { month: 'Mes', year: 'Año', all: 'Siempre' };
  const VIEW_LABEL = { rank: 'Clasificación', cal: 'Calendario', year: 'Resumen anual' };
  const R = $derived(app.rank);
  const rows = $derived(R.rows);
  const maxMs = $derived(Math.max(1, ...rows.map((r) => r.ms)));
  const all = $derived(app.statsKind === 'all');
  const now = $derived(app.now.getTime());
  const avg = $derived(R.n ? R.ms / R.n : 0);
  // Días transcurridos del periodo (el actual aún no ha terminado).
  const elapsed = $derived.by(() => {
    if (all) return 0;
    const to = Math.min(R.range.to, addDays(new Date(now).setHours(0, 0, 0, 0), 1));
    return Math.max(0, Math.round((to - R.range.from) / 864e5));
  });
  const pname = (pi: number) => app.lib.platforms[pi]?.name ?? '';
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

<TopBar crumbs={['Inicio', 'Estadísticas', VIEW_LABEL[app.statsTab]]} />

<div class="sx">
  {#if app.statsTab === 'rank'}
    <span class="cat">{pname(app.statsPlat) || 'Sin partidas'}</span>
    <span class="dot"></span>
    <span class="gl" data-hact="x">{#each STATS_KINDS as k}<span class="o" class:gem={app.statsKind === k}>{KIND_LABEL[k]}</span>{/each}</span>
    <span class="gl" class:off={all}>
      <span class="chev" class:dis={!canOlder} data-hact="lb">{@render chev(true)}</span>
      <span class="lbl">{R.range.label}</span>
      <span class="chev" class:dis={!canNewer} data-hact="rb">{@render chev(false)}</span>
    </span>
  {:else if app.statsTab === 'year'}
    {@const yi = app.years.indexOf(app.yearNow)}
    <span class="cat">Resumen anual</span>
    <span class="gl">
      <span class="chev" class:dis={yi <= 0} data-hact="lb">{@render chev(true)}</span>
      <span class="lbl">{app.yearNow}</span>
      <span class="chev" class:dis={yi < 0 || yi >= app.years.length - 1} data-hact="rb">{@render chev(false)}</span>
    </span>
  {:else}
    <span class="cat">Calendario</span>
    <span class="gl">
      <span class="chev" class:dis={!C.older} data-hact="lb">{@render chev(true)}</span>
      <span class="lbl cal">{calRange}</span>
      <span class="chev" class:dis={app.calOff >= 0} data-hact="rb">{@render chev(false)}</span>
    </span>
  {/if}
  <span class="gl vsw" data-hact="y">
    <span class="gem">{VIEW_LABEL[app.statsTab]}</span>
    <span class="dots">{#each ['rank', 'cal', 'year'] as v}<i class:on={app.statsTab === v}></i>{/each}</span>
  </span>
</div>

{#snippet chev(left: boolean)}
  <svg viewBox="0 0 12 12"><path d={left ? 'M7.5 2 3.5 6l4 4' : 'M4.5 2l4 4-4 4'} /></svg>
{/snippet}

{#if app.statsTab === 'year'}
  <Year />
  <Hints list={[['A', 'Siguiente', 'a'], ['LB/RB', 'Año', 'rb'], ['Y', 'Clasificación', 'y'], ['B', 'Volver', 'b']]} />
{:else if app.statsTab === 'rank'}
  <div class="stsum">
    <div class="rbox">
      <div class="k">Tiempo jugado</div>
      <div class="v acc">{fmtHM(R.ms)}</div>
    </div>
    <div class="rbox">
      <div class="k">Partidas</div>
      <div class="v">{R.n}</div>
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
          {#if i < 3}
            <!-- Top 3: el hero se funde hacia el blanco y el logo va encima. -->
            <div class="rrow hrow fx" class:f={app.focus === 'sr' + i} data-k="sr{i}" data-act="sr:{r.pi}:{r.ti}" style="--g:{a}">
              <span class="medal m{i}">{i + 1}º</span>
              <HeroStrip t={r.t} />
              <span class="bar"><i style="width:{(r.ms / maxMs) * 100}%"></i></span>
              <span class="tm">{fmtHM(r.ms)}<small>&nbsp;·&nbsp;{r.n} {r.n === 1 ? 'partida' : 'partidas'}</small></span>
            </div>
          {:else}
            <div class="rrow fx" class:f={app.focus === 'sr' + i} data-k="sr{i}" data-act="sr:{r.pi}:{r.ti}" style="--g:{a}">
              <span class="pos">{i + 1}º</span>
              <MiniCover t={r.t} acc={a} />
              <span class="nm">{r.t.name}<small>{pname(r.pi)}</small></span>
              <span class="bar"><i style="width:{(r.ms / maxMs) * 100}%"></i></span>
              <span class="tm">{fmtHM(r.ms)}<small>&nbsp;·&nbsp;{r.n} {r.n === 1 ? 'partida' : 'partidas'}</small></span>
            </div>
          {/if}
        {/each}
      </div>
    </div>
    {#if rankY(rows.length - 1) + rankH(rows.length - 1) > RANK_VIEW_H && app.focus.startsWith('sr')}
      <div class="stmore">Puesto {+app.focus.slice(2) + 1} de {rows.length}</div>
    {/if}
  {/if}

  <Hints list={[['A', 'Ver juego', 'a'], ['LB/RB', 'Fecha', 'rb'], ['X', 'Periodo', 'x'], ['LT/RT', 'Categoría', 'rt'], ['Y', 'Calendario', 'y'], ['B', 'Volver', 'b']]} />
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

  <Hints list={[['LB/RB', 'Fecha', 'rb'], ['Y', 'Resumen anual', 'y'], ['B', 'Volver', 'b']]} />
{/if}
