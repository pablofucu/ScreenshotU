<script lang="ts">
  // Resumen anual («Tu año jugando»): tarjetas a pantalla completa que se recorren con izquierda/derecha.
  // Cada tarjeta usa el acento del juego del que habla (o el arcoíris cuando habla de todo el año).
  import { app } from '../lib/store.svelte';
  import { YEAR_CARDS } from '../lib/controller.svelte';
  import { RAINBOW } from '../lib/icons';
  import { backend } from '../lib/backend';
  import { MONTH_NAMES, MONTH_ABBR, WDAYS, fmtClock, fmtDate, fmtHM, fmtLongDay } from '../lib/stats';
  import HeroStrip from '../components/HeroStrip.svelte';
  import MiniCover from '../components/MiniCover.svelte';

  const Y = $derived(app.yr);
  const now = $derived(app.now.getTime());
  const card = $derived(app.yearCard);
  const cap = (s: string) => s[0].toUpperCase() + s.slice(1);
  const accOf = (pi: number, ti: number) => app.titleAcc(pi, ti).c;
  const pname = (pi: number) => app.lib.platforms[pi]?.name ?? '';
  const plural = (n: number, a: string, b: string) => `${n} ${n === 1 ? a : b}`;
  const rainbow = `linear-gradient(90deg,${RAINBOW.map((r) => r.c).join(',')})`;

  const hours = $derived(Math.floor(Y.ms / 36e5));
  const mins = $derived(Math.round((Y.ms % 36e5) / 6e4));
  const top = $derived(Y.games[0]);
  // Juegos marcados como terminados este año.
  const finished = $derived(app.lib.platforms.reduce((n, p) => n + (p.id === 'sagas' ? 0 : p.titles.filter((t) => !t.hidden && t.status === 'finished' && t.finishedMs && new Date(t.finishedMs).getFullYear() === Y.year).length), 0));
  const top5 = $derived(Y.games.slice(0, 5));
  const max5 = $derived(Math.max(1, ...top5.map((g) => g.ms)));
  const monthMax = $derived(Math.max(1, ...Y.months));
  const wdMax = $derived(Math.max(1, ...Y.wdMs));
  // Semana de lunes a domingo.
  const WD_ORDER = [1, 2, 3, 4, 5, 6, 0];
  const range = (a: number, b: number) => {
    const f = (x: number) => { const d = new Date(x); return `${d.getDate()} ${MONTH_ABBR[d.getMonth()]}`; };
    return a === b ? f(a) : `del ${f(a)} al ${f(b)}`;
  };
  const hm = (ms: number) => (ms <= 0 ? '' : ms < 36e5 ? `${Math.round(ms / 6e4)} min` : `${Math.round(ms / 36e5)} h`);
  const dayName = (wd: number) => cap(WDAYS[wd]);
  /** La fecha de una captura es la hora local codificada como UTC. */
  const shotDay = (date: number) => { const d = new Date(date); return fmtDate(new Date(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()).getTime()); };

  // Captura favorita / más reciente del año.
  let shotSrc = $state('');
  $effect(() => {
    const p = Y.shot?.path;
    shotSrc = '';
    if (!p) return;
    const ctl = new AbortController();
    backend.thumb(p, ctl.signal).then((u) => { if (!ctl.signal.aborted) shotSrc = u; }, () => {});
    return () => ctl.abort();
  });

  // Comparación con el año anterior.
  const rows = $derived.by(() => {
    const p = Y.prev;
    if (!p) return [];
    return [
      { k: 'Horas jugadas', a: Y.ms, b: p.ms, f: fmtHM },
      { k: 'Partidas', a: Y.n, b: p.n, f: String },
      { k: 'Días jugados', a: Y.days, b: p.days, f: String },
      { k: 'Capturas', a: Y.shots, b: p.shots, f: String },
    ].map((r) => {
      const d = r.a - r.b, m = Math.max(1, r.a, r.b);
      const pct = r.b > 0 ? Math.round((d / r.b) * 100) : null;
      return { ...r, wa: (r.a / m) * 100, wb: (r.b / m) * 100, d, dt: d === 0 ? '= igual' : `${d > 0 ? '▲' : '▼'} ${r.f(Math.abs(d))}${pct === null ? '' : ` (${d > 0 ? '+' : '−'}${Math.abs(pct)} %)`}` };
    });
  });
</script>

{#if !app.years.length}
  <div class="stnone">Aún no hay partidas registradas. El tiempo se cuenta al jugar desde el visor.</div>
{:else}
  {#key app.yearNow + ':' + card}
    <div class="ycard stpanel" style="--dir:{app.yearDir}" data-hact="a">
      <div class="yrb" style="background:{rainbow}"></div>

      {#if card === 0}
        <div class="yk">{Y.year} · Tu año jugando</div>
        <div class="y1">
          <div class="ybig">
            <div class="yhrs" style="background-image:{rainbow}"><b>{hours}</b><span>h</span>{#if mins}<em>{mins} min</em>{/if}</div>
            <div class="ysub">jugados en {Y.year}</div>
          </div>
          <div class="ystack">
            <div class="rbox"><div class="k">Partidas</div><div class="v">{Y.n}</div><div class="s eq">{Y.n ? `media de ${fmtHM(Y.ms / Y.n)}` : ''}</div></div>
            <div class="rbox"><div class="k">Días jugados</div><div class="v">{Y.days} <small>de {Y.elapsed}</small></div><div class="s eq">{Math.round((Y.days / Y.elapsed) * 100)} % de los días</div></div>
            <div class="rbox"><div class="k">Juegos distintos</div><div class="v">{Y.games.length}</div><div class="s eq">{finished ? `${finished} ${finished === 1 ? 'terminado' : 'terminados'} este año` : ''}</div></div>
          </div>
        </div>

      {:else if card === 1 && top}
        {@const a = accOf(top.pi, top.ti)}
        <div class="yk" style="color:{a}">{Y.year} · Juego del año</div>
        <div class="y2" style="--acc:{a};--g:{a}">
          <div class="yleft">
            <div class="ybanner"><HeroStrip t={top.t} /></div>
            <div class="ystats">
              <div class="rbox"><div class="k">Tiempo</div><div class="v acc">{fmtHM(top.ms)}</div></div>
              <div class="rbox"><div class="k">Partidas</div><div class="v">{top.n}</div></div>
              <div class="rbox"><div class="k">Del total del año</div><div class="v">{Math.round((top.ms / Math.max(1, Y.ms)) * 100)} <small>%</small></div></div>
            </div>
          </div>
          <div class="ycov"><MiniCover t={top.t} acc={a} /></div>
        </div>

      {:else if card === 2}
        <div class="yk">{Y.year} · Tus 5 juegos más jugados</div>
        <div class="y3">
          {#each top5 as g, i (g.t.id)}
            {@const a = accOf(g.pi, g.ti)}
            <div class="yrow" style="--g:{a}">
              <span class="medal" class:m1={i === 1} class:m2={i === 2} class:mn={i > 2}>{i + 1}º</span>
              <MiniCover t={g.t} acc={a} />
              <span class="nm">{g.t.name}<small>{pname(g.pi)} · {plural(g.n, 'partida', 'partidas')}</small></span>
              <span class="bar"><i style="width:{(g.ms / max5) * 100}%"></i></span>
              <span class="tm">{fmtHM(g.ms)}</span>
            </div>
          {/each}
        </div>

      {:else if card === 3}
        <div class="yk">{Y.year} · Tu mes estrella</div>
        <div class="y4">
          <div class="ymeta">
            <div class="ysub">Mes con más horas</div>
            <div class="ymon">{Y.bestMonth >= 0 ? cap(MONTH_NAMES[Y.bestMonth]) : '—'}</div>
            {#if Y.bestMonth >= 0}<div class="ysub b">{fmtHM(Y.months[Y.bestMonth])}</div>{/if}
          </div>
          <div class="ybars">
            {#each Y.months as m, i}
              <div class:best={i === Y.bestMonth}>
                <u>{hm(m)}</u>
                <i style="height:{Math.max(m ? 6 : 0, (m / monthMax) * 240)}px"></i>
                <span>{cap(MONTH_ABBR[i])}</span>
              </div>
            {/each}
          </div>
        </div>

      {:else if card === 4}
        <div class="yk">{Y.year} · Constancia</div>
        <div class="y5">
          <div class="ypan">
            <div class="ysub">Mejor racha</div>
            <div class="ymon">{Y.streak.n} <small>{Y.streak.n === 1 ? 'día seguido' : 'días seguidos'}</small></div>
            {#if Y.streak.n}<div class="ysub b">{range(Y.streak.from, Y.streak.to)}</div>{/if}
          </div>
          <div class="ypan">
            <div class="ysub">Día favorito</div>
            <div class="ymon">{Y.favWd ? dayName(Y.favWd.wd) : '—'}</div>
            {#if Y.favWd}<div class="ysub b">{fmtHM(Y.favWd.ms)} en {plural(Y.favWd.days, 'día', 'días')}</div>{/if}
            <div class="ywd">
              {#each WD_ORDER as wd}
                <div class:best={Y.favWd?.wd === wd}>
                  <i style="height:{Math.max(Y.wdMs[wd] ? 5 : 0, (Y.wdMs[wd] / wdMax) * 110)}px"></i>
                  <span>{dayName(wd)[0]}</span>
                </div>
              {/each}
            </div>
          </div>
        </div>

      {:else if card === 5}
        {#if Y.longest}
          {@const L = Y.longest}
          {@const a = accOf(L.pi, L.ti)}
          <div class="yk" style="color:{a}">{Y.year} · Tu maratón</div>
          <div class="y2" style="--acc:{a};--g:{a}">
            <div class="yleft">
              <div class="ybanner"><HeroStrip t={L.t} /></div>
              <div class="ystats two">
                <div class="rbox"><div class="k">Partida más larga</div><div class="v acc">{fmtHM(L.ms)}</div></div>
                <div class="rbox"><div class="k">{fmtLongDay(L.start, now)}</div><div class="v sm">{fmtClock(L.start)} – {fmtClock(L.end)}</div></div>
              </div>
            </div>
            <div class="ycov"><MiniCover t={L.t} acc={a} /></div>
          </div>
        {:else}
          <div class="yk">{Y.year} · Tu maratón</div>
          <div class="ynone">No hay partidas registradas este año.</div>
        {/if}

      {:else if card === 6}
        {@const sh = Y.shot}
        <div class="yk" style="color:{sh ? accOf(sh.pi, sh.ti) : 'inherit'}">{Y.year} · Tus capturas</div>
        {#if sh}
          {@const a = accOf(sh.pi, sh.ti)}
          <div class="y7" style="--acc:{a};--g:{a}">
            <div class="yshots">
              <div class="ybig"><div class="yhrs" style="background-image:{rainbow}"><b>{Y.shots}</b></div><div class="ysub">{Y.shots === 1 ? 'captura hecha' : 'capturas hechas'} en {Y.year}</div></div>
              <div class="ysub b">Más capturas en</div>
              {#each Y.shotGames.slice(0, 3) as g (g.t.id)}
                {@const ga = accOf(g.pi, g.ti)}
                <div class="yrow s" style="--g:{ga}"><MiniCover t={g.t} acc={ga} small /><span class="nm">{g.t.name}</span><span class="tm">{g.n}</span></div>
              {/each}
            </div>
            <figure class="yfav">
              <div class="yimg">{#if shotSrc}<img alt="" src={shotSrc} draggable="false" />{/if}</div>
              <figcaption><b>{sh.fav ? '♥ Tu favorita' : 'La más reciente'}</b><span>{sh.t.name} · {shotDay(sh.date)}</span></figcaption>
            </figure>
          </div>
        {:else}
          <div class="ynone">No hiciste capturas en {Y.year}.</div>
        {/if}

      {:else}
        <div class="yk">{Y.year}{Y.prev ? ` frente a ${Y.prev.year}` : ''}</div>
        {#if Y.prev}
          <div class="y8">
            <div class="yleg"><i class="a"></i>{Y.year}<i class="b"></i>{Y.prev.year}</div>
            {#each rows as r}
              <div class="ycmp">
                <span class="k">{r.k}</span>
                <span class="bars"><i class="a" style="width:{r.wa}%"></i><i class="b" style="width:{r.wb}%"></i></span>
                <span class="vals"><b>{r.f(r.a)}</b><small>{r.f(r.b)}</small></span>
                <span class="dl {r.d > 0 ? 'up' : r.d < 0 ? 'down' : 'eq'}">{r.dt}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="ynone">{Y.year} es tu primer año con partidas registradas.<small>El año que viene podrás compararlo.</small></div>
        {/if}
      {/if}
    </div>
  {/key}

  <div class="ydots">
    <span class="yarr" class:off={card === 0} data-hact="left">‹</span>
    {#each Array(YEAR_CARDS) as _, i}<i class:on={i === card}></i>{/each}
    <span class="yarr" class:off={card === YEAR_CARDS - 1} data-hact="right">›</span>
  </div>
{/if}
