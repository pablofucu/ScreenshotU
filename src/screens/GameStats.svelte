<script lang="ts">
  // Estadísticas de juego. De un título (desde Evolución): tiempo, partidas, récords y fechas.
  // De una categoría (Start en la lista): clasificación por métrica SOLO de esa categoría.
  import { app } from '../lib/store.svelte';
  import { GMETRICS, fmtClock, fmtDate, fmtHM, fmtLongDay, type GMetric } from '../lib/stats';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import RecentCard from '../components/RecentCard.svelte';
  import MiniCover from '../components/MiniCover.svelte';

  const ROWS = 5;
  const P = $derived(app.platform);
  const T = $derived(P?.titles[app.t]);
  const S = $derived(app.gsTitle);
  const C = $derived(app.gsCat);
  const cat = $derived(app.gsScope === 'cat');
  const now = $derived(app.now.getTime());
  const rows = $derived(C.rows.slice(0, ROWS));
  const maxV = $derived(Math.max(1, ...rows.map((r) => r.v)));
  const fmtV = (m: GMetric, v: number) => (m === 'sessions' ? `${v}` : fmtHM(v));
  const unit = (m: GMetric, v: number) => (m === 'sessions' ? (v === 1 ? 'partida' : 'partidas') : '');
</script>

{#if cat && P}
  <TopBar crumbs={[P.name, 'Estadísticas de juego']} />

  <div class="sttop">
    <div class="stabs pill">
      <span class="btn sh" data-hact="lb">LB</span>
      {#each GMETRICS as [k, l]}<span class="t" class:on={app.gsMetric === k}>{l}</span>{/each}
      <span class="btn sh" data-hact="rb">RB</span>
    </div>
  </div>

  <div class="stsum">
    <div class="rbox"><div class="k">Tiempo jugado</div><div class="v acc">{fmtHM(C.ms)}</div></div>
    <div class="rbox"><div class="k">Partidas</div><div class="v">{C.sessions}</div></div>
    <div class="rbox"><div class="k">Juegos jugados</div><div class="v">{C.games}</div></div>
    <div class="rbox">
      <div class="k">Sesión más larga</div>
      <div class="v">{C.longest ? fmtHM(C.longest.ms) : '—'}</div>
      {#if C.longest}<div class="s eq">{C.longest.t.name}</div>{/if}
    </div>
  </div>

  {#if !rows.length}
    <div class="stnone">Aún no hay partidas en esta categoría. El tiempo se cuenta al jugar desde el visor.</div>
  {:else}
    <div class="strows">
      <div class="inner">
        {#each rows as r, i (r.t.id)}
          {@const a = app.titleAcc(r.pi, r.ti).c}
          <div class="rrow fx" class:f={app.focus === 'gs' + i} data-k="gs{i}" data-act="gs:{r.pi}:{r.ti}" style="--g:{a}">
            <span class="pos">{i + 1}º</span>
            <MiniCover t={r.t} acc={a} />
            <span class="nm">{r.t.name}<small>{r.ts.sessions} {r.ts.sessions === 1 ? 'partida' : 'partidas'} · {fmtHM(r.ts.ms)}</small></span>
            <span class="bar"><i style="width:{(r.v / maxV) * 100}%"></i></span>
            <span class="tm">{fmtV(app.gsMetric, r.v)}{#if unit(app.gsMetric, r.v)}<small>&nbsp;{unit(app.gsMetric, r.v)}</small>{/if}</span>
            <span class="dl"></span>
          </div>
        {/each}
      </div>
    </div>
    {#if C.rows.length > ROWS}<div class="stmore">Primeros {ROWS} de {C.rows.length}</div>{/if}
  {/if}

  <Hints list={[['A', 'Ver juego', 'a'], ['LB/RB', 'Métrica', 'rb'], ['B', 'Volver', 'b']]} />
{:else if T && S}
  <TopBar crumbs={[P?.name ?? '', T.name, 'Estadísticas de juego']} />

  <div class="glass rcard evhead" style="--acc:{app.chrome.c}">
    <RecentCard t={T} />
  </div>

  <div class="stsum" style="top:260px">
    <div class="rbox"><div class="k">Tiempo total</div><div class="v acc">{fmtHM(S.ms)}</div></div>
    <div class="rbox"><div class="k">Partidas</div><div class="v">{S.sessions}</div></div>
    <div class="rbox"><div class="k">Media por partida</div><div class="v">{S.sessions ? fmtHM(S.avg) : '—'}</div></div>
    <div class="rbox"><div class="k">Días con partida</div><div class="v">{S.days}</div></div>
  </div>

  <div class="stsum" style="top:376px;grid-template-columns:repeat(3,1fr)">
    <div class="rbox">
      <div class="k">Sesión más larga</div>
      <div class="v">{S.longest ? fmtHM(S.longest.ms) : '—'}</div>
      {#if S.longest}<div class="s eq">{fmtLongDay(S.longest.at, now)}, {fmtClock(S.longest.at)}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Última sesión</div>
      <div class="v">{S.last ? fmtHM(S.last.ms) : '—'}</div>
      {#if S.last}<div class="s eq">{fmtLongDay(S.last.at, now)}, {fmtClock(S.last.at)}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Primera vez</div>
      <div class="v">{S.first ? fmtDate(S.first) : '—'}</div>
    </div>
  </div>

  <Hints list={[['B', 'Volver', 'b']]} />
{/if}
