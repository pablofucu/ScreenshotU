<script lang="ts">
  // Estadísticas › nivel Juego: tarjeta con la portada, cuatro cifras, cómo lo has jugado y tres más.
  // View alterna con Periféricos (las cifras de la base). LB/RB cambian la escala de las barras.
  import { onDestroy } from 'svelte';
  import { app, LIST_COLOR, STATUS_LABEL } from '../../lib/store.svelte';
  import { SERIES_KINDS, fmtDate, fmtDaySpan, fmtClock, fmtHHMM, fmtHM, fmtLongDay, fmtSpan, shotInRange, titleFavHour, titleRank, titleSeries, titleStreak } from '../../lib/stats';
  import { catTone, toneStyle } from '../../lib/chiptone';
  import { countUp, resetCounts } from '../../lib/countup';
  import TopBar from '../../components/TopBar.svelte';
  import Hints from '../../components/Hints.svelte';
  import StatChip from '../../components/StatChip.svelte';
  import CoverBg from '../../components/CoverBg.svelte';

  const P = $derived(app.platform);
  const T = $derived(P?.titles[app.t]);
  const S = $derived(app.gsTitle);
  const D = $derived(app.gsDb);
  const now = $derived(app.now.getTime());
  const tone = $derived(P ? catTone(P.id) : 4);
  const uid = $derived(`gt:${app.p}:${app.t}:${app.gsPage}`);
  const io = $derived(app.gsPage === 'io');

  const rank = $derived(T ? titleRank(app.lib, app.p, app.t) : { pos: 0, of: 0 });
  const ser = $derived(T ? titleSeries(T, app.gsScale, now) : null);
  const smax = $derived(Math.max(1, ...(ser?.bars.map((b) => b.ms) ?? [0])));
  const streak = $derived(T ? titleStreak(T) : null);
  const fav = $derived(T ? titleFavHour(T) : null);
  const first = $derived(D?.firstPlayedMs ?? S?.first ?? null);
  const favShots = $derived(T ? T.shots.filter((s) => app.favs.has(s.path)).length : 0);
  const nShots = $derived(T?.shots.length ?? 0);
  const finMs = $derived(T?.status === 'finished' ? (T.finishedMs ?? null) : null);

  const nf = (n: number) => n.toLocaleString('es-ES');
  const fmtBig = (n: number) => (n >= 1e6 ? `${(n / 1e6).toFixed(1).replace('.', ',')} M` : n >= 1e4 ? `${(n / 1e3).toFixed(1).replace('.', ',')} k` : nf(n));
  const parts = (ms: number) => { const m = Math.round(ms / 60000); return m < 60 ? { v: m, u: 'min' } : { v: Math.floor(m / 60), u: 'h' }; };
  const pct = $derived(D && D.measuredMs > 0 ? Math.round((D.activeMs / D.measuredMs) * 100) : null);
  const runs = $derived(Math.max(D?.runCount ?? 0, S?.sessions ?? 0));
  const hrTxt = (ms: number) => { const h = ms / 3.6e6; return h >= 10 ? String(Math.round(h)) : h >= 1 ? h.toFixed(1).replace('.', ',') : String(Math.round(ms / 60000)); };
  const hrU = (ms: number) => (ms >= 3.6e6 ? 'h' : 'min');
  onDestroy(() => resetCounts());
  void shotInRange; void fmtClock; void fmtLongDay; void fmtDate;
</script>

{#if T && S && P}
<TopBar crumbs={['Inicio', 'Estadísticas', P.name, T.name]} />

<div class="cvp">
 <div class="cpane gtl" style="{toneStyle(tone)};transform:none;right:48px">
  <div class="gr">
    <div class="glass big" style="--g:{app.titleAcc(app.p, app.t).c}">
      <CoverBg t={T} cls="cv" />
      <div class="cap">
        <div class="nm">{T.name}</div>
        <div class="pl">
          <span class="pill-s">{P.name}</span>
          {#if T.status}<span class="pill-s"><i class="dot" style="background:{LIST_COLOR[T.status].c}"></i>{STATUS_LABEL[T.status]}</span>{/if}
        </div>
      </div>
    </div>

    <div class="rt">
      {#if !io}
        <div class="row4">
          <StatChip k="gt0" icon="clock" label="Tiempo total" {tone}>
            {@const m = parts(S.ms)}
            <div class="x-v"><span use:countUp={{ to: m.v, key: `${uid}:A` }}>{m.v}</span><small>{m.u}</small></div>
            <div class="x-s">{#if rank.pos}el <b>{rank.pos}.º</b> más jugado de {P.name}{:else}Sin partidas{/if}</div>
          </StatChip>
          <StatChip k="gt1" icon="target" label="Sesiones" {tone}>
            <div class="x-v"><span use:countUp={{ to: S.sessions, key: `${uid}:B` }}>{S.sessions}</span></div>
            <div class="x-s">{S.sessions ? `${fmtHM(S.avg)} de media` : 'Sin partidas'}</div>
          </StatChip>
          <StatChip k="gt2" icon="trophy" label={finMs ? 'Terminado' : 'Estado'} {tone}>
            {#if finMs}
              <div class="x-v sm">{fmtDate(finMs)}</div>
              <div class="x-s">{first ? `tras ${fmtSpan(first, finMs)}, desde el ${fmtDate(first)}` : 'sin fecha de inicio'}</div>
            {:else}
              <div class="x-v sm">{T.status ? STATUS_LABEL[T.status] : 'Sin estado'}</div>
              <div class="x-s">{first ? `jugado por primera vez el ${fmtDate(first)}` : 'sin partidas'}</div>
            {/if}
          </StatChip>
          <StatChip k="gt3" icon="cam" label="Capturas" {tone}>
            <div class="x-v"><span use:countUp={{ to: nShots, key: `${uid}:D` }}>{nShots}</span></div>
            <div class="x-s"><b>{favShots}</b> {favShots === 1 ? 'favorita' : 'favoritas'}</div>
          </StatChip>
        </div>

        <StatChip k="gt4" icon="trend" label="Cómo lo has jugado" cls="ser" {tone}>
          <div class="scl">
            <span class="chev" class:dis={app.gsScale === 'month'} data-hact="lb"><svg viewBox="0 0 12 12"><path d="M7.5 2 3.5 6l4 4" /></svg></span>
            <span class="lbl">{SERIES_KINDS.find(([k]) => k === app.gsScale)?.[1]}</span>
            <span class="chev" class:dis={app.gsScale === 'year'} data-hact="rb"><svg viewBox="0 0 12 12"><path d="M4.5 2l4 4-4 4" /></svg></span>
          </div>
          {#if ser && S.ms > 0}
            <div class="x-bars sb">
              {#each ser.bars as b, i (uid + app.gsScale + i)}
                <div class={b.cur ? 'x-top' : ''}>
                  <em>{b.ms > 0 ? hrTxt(b.ms) : ''}</em>
                  <i style="--h:{b.ms > 0 ? Math.max(4, (b.ms / smax) * 100) : 2}%;--i:{i}"></i>
                  <span>{b.label}</span>
                </div>
              {/each}
            </div>
            <div class="x-s cap2">{ser.caption}</div>
          {:else}
            <div class="x-s">Aún no hay partidas de este juego.</div>
          {/if}
        </StatChip>

        <div class="row3">
          <StatChip k="gt5" icon="watch" label="Partida más larga" {tone}>
            {#if S.longest}
              {@const l = parts(S.longest.ms)}
              <div class="x-v"><span>{l.v}</span><small>{l.u}</small></div>
              <div class="x-s">de <b>{fmtHHMM(S.longest.at)}</b> a <b>{fmtHHMM(S.longest.at + S.longest.ms)}</b>, {fmtDate(S.longest.at)}</div>
            {:else}
              <div class="x-v">—</div><div class="x-s">Sin partidas</div>
            {/if}
          </StatChip>
          <StatChip k="gt6" icon="flame" label="Racha con este juego" {tone}>
            {#if streak}
              <div class="x-v"><span use:countUp={{ to: streak.n, key: `${uid}:G` }}>{streak.n}</span><small>{streak.n === 1 ? 'día' : 'días'}</small></div>
              <div class="x-s">{fmtDaySpan(streak.from, streak.to, now)}</div>
            {:else}
              <div class="x-v">—</div><div class="x-s">Sin partidas</div>
            {/if}
          </StatChip>
          <StatChip k="gt7" icon="moon" label="Hora favorita" {tone}>
            {#if fav}
              <div class="x-v"><span>{String(fav.hour).padStart(2, '0')}:00</span></div>
              <div class="x-s">{fav.share >= 0.6 ? 'casi siempre' : 'sobre todo'} de <b>{fav.part}</b></div>
            {:else}
              <div class="x-v">—</div><div class="x-s">Sin partidas</div>
            {/if}
          </StatChip>
        </div>
      {:else}
        <div class="row4">
          <StatChip k="gt0" icon="clock" label="Tiempo interactivo" {tone}>
            {#if D && pct !== null}
              <div class="x-v sm">{fmtHM(D.activeMs)} <small>{pct} %</small></div>
              <div class="x-s">{fmtHM(D.measuredMs - D.activeMs)} pasivo o en segundo plano</div>
            {:else}
              <div class="x-v">—</div><div class="x-s">Se mide en las partidas nuevas</div>
            {/if}
          </StatChip>
          <StatChip k="gt1" icon="target" label="Ejecuciones" {tone}>
            <div class="x-v"><span use:countUp={{ to: runs, key: `${uid}:B` }}>{runs}</span></div>
          </StatChip>
          <StatChip k="gt2" icon="watch" label="Mayor sesión interactiva" {tone}>
            <div class="x-v sm">{D && D.longestActiveAt ? fmtHM(D.longestActiveMs) : '—'}</div>
            {#if D?.longestActiveAt}<div class="x-s">{fmtLongDay(D.longestActiveAt, now)}, {fmtClock(D.longestActiveAt)}</div>{/if}
          </StatChip>
          <StatChip k="gt3" icon="pad" label="Pulsaciones de mando" {tone}>
            <div class="x-v sm">{D && D.pad > 0 ? nf(D.pad) : '—'}</div>
            {#if D && D.peakPpm > 0}<div class="x-s">Pico: {D.peakPpm} por minuto</div>{/if}
          </StatChip>
        </div>
        <div class="row3">
          <StatChip k="gt4" icon="pad" label="Clics de ratón" {tone}>
            <div class="x-v sm">{D && D.clicks > 0 ? nf(D.clicks) : '—'}</div>
            <div class="x-s">{#if D && D.peakCpm > 0}Pico: {D.peakCpm} por minuto{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
          </StatChip>
          <StatChip k="gt5" icon="pad" label="Rueda del ratón" {tone}>
            <div class="x-v sm">{D && D.wheel > 0 ? nf(D.wheel) : '—'}</div>
            <div class="x-s">{#if D && D.wheel > 0}muescas{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
          </StatChip>
          <StatChip k="gt6" icon="pad" label="Distancia del cursor" {tone}>
            <div class="x-v sm">{D && D.mouseDist > 0 ? fmtBig(D.mouseDist) : '—'}</div>
            <div class="x-s">{#if D && D.mouseDist > 0}unidades, aprox. píxeles{:else if !app.config.inputMouse}Desactivado en Ajustes{/if}</div>
          </StatChip>
        </div>
        <div class="row3">
          <StatChip k="gt7" icon="grid" label="Pulsaciones de teclado" {tone}>
            <div class="x-v sm">{D && D.keys > 0 ? nf(D.keys) : '—'}</div>
            <div class="x-s">{#if D && D.peakKpm > 0}Pico: {D.peakKpm} por minuto{:else if !app.config.inputKeys}Desactivado en Ajustes{/if}</div>
          </StatChip>
        </div>
        <div class="x-s note">Solo se cuentan las pulsaciones, nunca qué tecla es. Se mide desde que se instaló esta versión.</div>
      {/if}
    </div>
  </div>
 </div>
</div>

<Hints list={[['LB/RB', 'Escala', 'rb'], ['View', io ? 'Resumen' : 'Periféricos', 'view'], ...(T.launchable ? [['X', 'Jugar', 'x']] : []), ['Y', 'Tu año', 'y'], ['B', 'Volver', 'b']] as Array<[string, string, string?]>} />
{/if}

<style>
  .gr{display:grid;grid-template-columns:300px minmax(0,1fr);gap:var(--s-4);height:476px}
  .big{position:relative;overflow:hidden;border-radius:var(--r-l);padding:0;animation:gtrise .5s var(--e-bounce) backwards}
  .big :global(.cv){position:absolute;inset:0;background-size:cover;background-position:center}
  .cap{position:absolute;left:0;right:0;bottom:0;padding:40px 16px 16px;display:flex;flex-direction:column;gap:var(--s-2);background:linear-gradient(180deg,transparent,rgba(10,12,20,.82))}
  .nm{font:800 var(--t-xl)/1.15 var(--round);color:#fff;text-shadow:0 2px 10px rgba(0,0,0,.5)}
  .pl{display:flex;flex-wrap:wrap;gap:var(--s-2)}
  .pill-s{display:inline-flex;align-items:center;gap:6px;height:26px;padding:0 12px;border-radius:var(--r-pill);background:rgba(255,255,255,.2);color:#fff;font:800 var(--t-xs)/1 var(--round);backdrop-filter:blur(6px)}
  .dot{display:inline-block;width:8px;height:8px;border-radius:50%}
  .rt{display:flex;flex-direction:column;gap:var(--s-4);min-width:0;min-height:0}
  .row4{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:var(--s-4)}
  .row3{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:var(--s-4)}
  .gtl :global(.row4 .st.st),.gtl :global(.row3 .st.st){height:112px}
  .gtl :global(.st.ser){flex:1;min-height:0;height:auto}
  .gtl :global(.st .face){padding:10px 14px;gap:6px}
  .gtl :global(.x-v){font-size:var(--t-2xl)}
  .gtl :global(.x-v.sm){font-size:var(--t-xl)}
  .gtl :global(.x-v small){font-size:var(--t-l)}
  .gtl :global(.x-s){white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .gtl :global(.ser .x-bars){flex:1;min-height:0;height:auto;gap:4px}
  .gtl :global(.ser .x-bars div){position:relative}
  .gtl :global(.ser .x-bars em){font:800 var(--t-xs)/1 var(--round);font-style:normal;color:var(--ink);min-height:10px}
  .gtl :global(.ser .x-bars i){flex:none;height:var(--h);min-height:2px}
  .gtl :global(.ser .x-bars div){justify-content:flex-end}
  .scl{position:absolute;top:8px;right:12px;display:flex;align-items:center;gap:var(--s-2);font:800 var(--t-s)/1 var(--round);color:var(--ink)}
  .scl .chev{display:grid;place-items:center;width:24px;height:24px;border-radius:50%;background:var(--glass-in);cursor:pointer}
  .scl .chev svg{width:12px;height:12px;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}
  .scl .chev.dis{opacity:.35}
  .cap2{text-align:center}
  .note{margin-top:var(--s-2)}
  .gtl :global(.row4 .st),.gtl :global(.row3 .st),.gtl :global(.st.ser){animation:gtrise .5s var(--e-bounce) backwards}
  .gtl :global(.row4 .st:nth-child(2)){animation-delay:55ms}
  .gtl :global(.row4 .st:nth-child(3)){animation-delay:110ms}
  .gtl :global(.row4 .st:nth-child(4)){animation-delay:165ms}
  .gtl :global(.st.ser){animation-delay:220ms}
  .gtl :global(.row3 .st:nth-child(1)){animation-delay:280ms}
  .gtl :global(.row3 .st:nth-child(2)){animation-delay:335ms}
  .gtl :global(.row3 .st:nth-child(3)){animation-delay:390ms}
  @keyframes gtrise{from{opacity:0;transform:translateY(16px) scale(.96)}}
  @media (prefers-reduced-motion: reduce){
    .big,.gtl :global(.row4 .st),.gtl :global(.row3 .st),.gtl :global(.st.ser){animation:none}
  }
</style>
