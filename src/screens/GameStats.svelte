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
  // Cifras de la base: el tiempo interactivo solo existe en las partidas jugadas desde que se mide.
  const D = $derived(app.gsDb);
  const first = $derived(D?.firstPlayedMs ?? S?.first ?? null);
  const pct = $derived(D && D.measuredMs > 0 ? Math.round((D.activeMs / D.measuredMs) * 100) : null);
  const fmtN = (n: number) => n.toLocaleString('es');
  /** 12 400 000 -> «12,4 M»; 8 300 -> «8,3 k». */
  const fmtBig = (n: number) => (n >= 1e6 ? `${(n / 1e6).toFixed(1).replace('.', ',')} M` : n >= 1e4 ? `${(n / 1e3).toFixed(1).replace('.', ',')} k` : fmtN(n));
  const runs = $derived(Math.max(D?.runCount ?? 0, S?.sessions ?? 0));
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

  <div class="glass rcard evhead">
    <RecentCard t={T} />
  </div>
  <div class="sttop" style="top:98px;left:auto;right:64px">
    <div class="stview pill" data-hact="y">
      <span class="btn y">Y</span>
      <span class="t" class:on={app.gsPage === 'sum'}>Resumen</span>
      <span class="t" class:on={app.gsPage === 'io'}>Periféricos</span>
    </div>
  </div>

  {#if app.gsPage === 'sum'}
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
      <div class="v">{first ? fmtDate(first) : '—'}</div>
    </div>
  </div>

  <div class="stsum" style="top:492px">
    <div class="rbox">
      <div class="k">Tiempo interactivo</div>
      {#if D && pct !== null}
        <div class="v acc">{fmtHM(D.activeMs)} <small>{pct}%</small></div>
        <div class="s eq">{fmtHM(D.measuredMs - D.activeMs)} pasivo o en segundo plano</div>
      {:else}
        <div class="v">—</div>
        <div class="s eq">Se mide en las partidas nuevas</div>
      {/if}
    </div>
    <div class="rbox">
      <div class="k">Ejecuciones</div>
      <div class="v">{runs}</div>
    </div>
    <div class="rbox">
      <div class="k">Mayor sesión interactiva</div>
      <div class="v">{D && D.longestActiveAt ? fmtHM(D.longestActiveMs) : '—'}</div>
      {#if D?.longestActiveAt}<div class="s eq">{fmtLongDay(D.longestActiveAt, now)}, {fmtClock(D.longestActiveAt)}</div>{/if}
    </div>
    <div class="rbox">
      <div class="k">Pulsaciones de mando</div>
      <div class="v">{D && D.pad > 0 ? D.pad.toLocaleString('es') : '—'}</div>
      {#if D && D.peakPpm > 0}<div class="s eq">Pico: {D.peakPpm} por minuto</div>{/if}
    </div>
  </div>

  <Hints list={[['Y', 'Periféricos', 'y'], ['B', 'Volver', 'b']]} />
  {:else}
  <div class="stsum" style="top:260px;grid-template-columns:repeat(3,1fr)">
    <div class="rbox">
      <div class="k">Clics de ratón</div>
      <div class="v">{D && D.clicks > 0 ? fmtN(D.clicks) : '—'}</div>
      <div class="s eq">{#if D && D.peakCpm > 0}Pico: {D.peakCpm} por minuto{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
    </div>
    <div class="rbox">
      <div class="k">Rueda del ratón</div>
      <div class="v">{D && D.wheel > 0 ? fmtN(D.wheel) : '—'}</div>
      <div class="s eq">{#if D && D.wheel > 0}muescas{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
    </div>
    <div class="rbox">
      <div class="k">Distancia del cursor</div>
      <div class="v">{D && D.mouseDist > 0 ? fmtBig(D.mouseDist) : '—'}</div>
      <div class="s eq">{#if D && D.mouseDist > 0}unidades del ratón, aprox. píxeles{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
    </div>
  </div>
  <div class="stsum" style="top:376px;grid-template-columns:repeat(3,1fr)">
    <div class="rbox">
      <div class="k">Pulsaciones de teclado</div>
      <div class="v">{D && D.keys > 0 ? fmtN(D.keys) : '—'}</div>
      <div class="s eq">{#if D && D.peakKpm > 0}Pico: {D.peakKpm} por minuto{:else if !app.config.inputKeys}Desactivado en Ajustes{/if}</div>
    </div>
    <div class="rbox">
      <div class="k">Pulsaciones de mando</div>
      <div class="v">{D && D.pad > 0 ? fmtN(D.pad) : '—'}</div>
      <div class="s eq">{#if D && D.peakPpm > 0}Pico: {D.peakPpm} por minuto{/if}</div>
    </div>
  </div>
  <div class="stnone" style="top:520px;font-size:15px;font-weight:600">Solo se cuentan las pulsaciones, nunca qué tecla es. Se mide desde que se instaló esta versión.</div>

  <Hints list={[['Y', 'Resumen', 'y'], ['B', 'Volver', 'b']]} />
  {/if}
{/if}
