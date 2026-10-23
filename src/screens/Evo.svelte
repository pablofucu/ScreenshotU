<script lang="ts">
  // Evolución de un juego: horas (barras) y partidas (puntos) por días, semanas, meses o años.
  // La cabecera es el hero con el logo; la barra enfocada se detalla en las tarjetas de la derecha.
  import { app } from '../lib/store.svelte';
  import { EVO_KINDS, addDays, fmtClock, fmtDate, fmtHM, fmtLongDay } from '../lib/stats';
  import { fmtAgo } from '../lib/format';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import RecentCard from '../components/RecentCard.svelte';

  const P = $derived(app.platform);
  const T = $derived(P?.titles[app.t]);
  const B = $derived(app.evo);
  const sel = $derived(app.focus.startsWith('eb') ? +app.focus.slice(2) : B.length - 1);
  const b = $derived(B[sel]);
  const pb = $derived(sel > 0 ? B[sel - 1] : undefined);
  const now = $derived(app.now.getTime());

  const TITLE: Record<string, string> = { day: 'Últimos 14 días', week: 'Últimas 12 semanas', month: 'Últimos 12 meses', year: 'Por años' };
  const PREV: Record<string, string> = { day: 'el día anterior', week: 'la semana anterior', month: 'el mes anterior', year: 'el año anterior' };

  // Gráfica (coordenadas dentro del panel).
  const W = 800, H = 340, L = 62, R = 46, TOP = 50, BOT = 34;
  const PH = H - TOP - BOT;
  const cw = $derived((W - L - R) / Math.max(1, B.length));
  const axis = $derived.by(() => {
    const maxMin = Math.max(1, ...B.map((x) => x.ms / 6e4));
    const step = [15, 30, 60, 120, 180, 300, 600, 1200, 3000, 6000, 12000, 30000].find((s) => maxMin / s <= 4) ?? 60000;
    const ticks = Math.max(1, Math.ceil(maxMin / step));
    const nmax = Math.max(1, ...B.map((x) => x.n));
    const sstep = Math.max(1, Math.ceil(nmax / ticks));
    return { step, ticks, ymax: step * ticks, sstep, smax: sstep * ticks };
  });
  const yT = (ms: number) => TOP + PH * (1 - ms / 6e4 / axis.ymax);
  const yS = (n: number) => TOP + PH * (1 - n / axis.smax);
  const tickLabel = (m: number) => (m === 0 ? '0' : axis.step < 60 ? `${m} min` : `${m / 60} h`);
  const cx = (i: number) => L + i * cw + cw / 2;
  const dots = $derived(B.map((x, i) => `${cx(i)},${yS(x.n)}`).join(' '));

  const span = (x: { from: number; to: number }) => {
    const to = Math.min(x.to, addDays(new Date(now).setHours(0, 0, 0, 0), 1));
    return Math.max(0, Math.round((to - x.from) / 864e5));
  };
  const diff = $derived.by(() => {
    if (!b || !pb) return null;
    const d = b.ms - pb.ms;
    if (Math.abs(d) < 6e4) return { c: 'eq', t: `= igual que ${PREV[app.evoKind]}` };
    return { c: d > 0 ? 'up' : 'down', t: `${d > 0 ? '▲' : '▼'} ${fmtHM(Math.abs(d))} ${d > 0 ? 'más' : 'menos'} que ${PREV[app.evoKind]}` };
  });
  const first = $derived(T?.plays?.[0]?.[0] ?? T?.firstPlayed ?? null);
</script>

{#if T}
  <TopBar crumbs={[P?.name ?? '', T.name, 'Estadísticas']} />

  <div class="glass rcard evhead" style="--acc:{app.chrome.c}">
    <RecentCard t={T} />
    <div class="evtot">
      <div><small>Tiempo total</small><b>{fmtHM(T.playSeconds * 1000)}</b></div>
      <div><small>Partidas</small><b>{T.sessions}</b></div>
      <div><small>Media</small><b>{T.sessions ? fmtHM((T.playSeconds * 1000) / T.sessions) : '—'}</b></div>
      {#if first}<div><small>Primera vez</small><b>{fmtDate(first)}</b></div>{/if}
      {#if T.lastPlayed}<div><small>Última vez</small><b>{fmtAgo(T.lastPlayed)}</b></div>{/if}
    </div>
  </div>

  <div class="evbar">
    <span>{TITLE[app.evoKind]}</span>
    <div class="stabs pill">
      <span class="btn sh" data-hact="lb">LB</span>
      {#each EVO_KINDS as [k, l]}<span class="t" class:on={app.evoKind === k}>{l}</span>{/each}
      <span class="btn sh" data-hact="rb">RB</span>
    </div>
  </div>

  <div class="stpanel evchart">
    <span class="evleg"><i class="lb"></i>Horas <i class="ld"></i>Partidas</span>
    {#each Array(axis.ticks + 1) as _, k}
      <span class="gl" style="top:{yT(k * axis.step * 6e4)}px;left:{L}px;width:{W - L - R}px"></span>
      <span class="yl" style="top:{yT(k * axis.step * 6e4) - 9}px;width:{L - 10}px">{tickLabel(k * axis.step)}</span>
      <span class="yr" style="top:{yS(k * axis.sstep) - 9}px;left:{W - R + 8}px">{k * axis.sstep}</span>
    {/each}
    {#each B as x, i (x.from)}
      <div class="ecol fx" class:f={app.focus === 'eb' + i} data-k="eb{i}" data-act="evo-bar" style="left:{L + i * cw}px;top:{TOP}px;width:{cw}px;height:{PH}px">
        {#if x.ms > 0}<i style="height:{PH - (yT(x.ms) - TOP)}px"></i>{/if}
      </div>
      <span class="xl" class:on={i === sel} class:tight={cw < 60} style="left:{L + i * cw}px;width:{cw}px;top:{TOP + PH + 8}px">{x.label}</span>
    {/each}
    <svg class="edots" width={W} height={H} aria-hidden="true">
      <polyline points={dots} />
      {#each B as x, i}<circle cx={cx(i)} cy={yS(x.n)} r={i === sel ? 7 : 5} />{/each}
    </svg>
    {#if b && (b.ms > 0 || b.n > 0)}
      <div class="etip" style="left:{cx(sel)}px;top:{Math.max(2, Math.min(yT(b.ms), yS(b.n)) - 50)}px">{fmtHM(b.ms)}<small>{b.n} {b.n === 1 ? 'partida' : 'partidas'}</small></div>
    {/if}
  </div>

  {#if b}
    <div class="evside">
      <div class="rbox"><div class="k">{b.long}</div><div class="v acc">{b.ms ? fmtHM(b.ms) : 'Sin jugar'}</div>{#if diff}<div class="s {diff.c}">{diff.t}</div>{/if}</div>
      <div class="rbox"><div class="k">Partidas</div><div class="v">{b.n}</div>{#if b.n}<div class="s eq">media de {fmtHM(b.ms / b.n)}</div>{/if}</div>
      <div class="rbox"><div class="k">Partida más larga</div><div class="v">{b.longest ? fmtHM(b.longest) : '—'}</div>{#if b.longest}<div class="s eq">{fmtLongDay(b.longestAt, now)}, {fmtClock(b.longestAt)}</div>{/if}</div>
      <div class="rbox"><div class="k">Días con partida</div><div class="v">{b.days} <small>de {span(b)}</small></div></div>
    </div>
  {/if}

  <Hints list={[['LB/RB', 'Escala', 'rb'], ['B', 'Volver', 'b']]} />
{/if}
