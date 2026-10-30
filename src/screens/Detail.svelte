<script lang="ts">
  import { app, LIST_COLOR, STATUS_LABEL } from '../lib/store.svelte';
  import { backend, decodeImg, thumbNow } from '../lib/backend';
  import { ICON } from '../lib/icons';
  import { fmtDur, nCapturas, z } from '../lib/format';
  import { DETAIL_SHOTS } from '../lib/controller.svelte';
  import { openStatusBar } from '../lib/milista.svelte';
  import { openPhrases } from '../lib/phrases.svelte';
  import { fmtRelease, openRelease } from '../lib/release.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';
  import AchievementCard from '../components/AchievementCard.svelte';
  import { achState, loadGameAchievements, VARIANT_NAMES } from '../lib/achievements.svelte';

  const P = $derived(app.platform);
  const T = $derived(P?.titles[app.t]);
  const shots = $derived(T?.shots ?? []);
  const row = $derived(shots.slice(0, DETAIL_SHOTS));
  const coverPath = $derived(T?.cover ?? shots[0]?.path ?? null);

  // Grafismos: se sirven tal cual (sin miniatura). Si fallan al cargar se usa el recurso alternativo.
  const heroUrl = $derived(T?.hero ? (app.artVer, backend.artUrl(T.hero)) : null);
  const logoUrl = $derived(T?.logo ? (app.artVer, backend.artUrl(T.logo)) : null);
  let heroBad = $state('');
  let logoBad = $state('');
  const showHero = $derived(!!heroUrl && heroBad !== heroUrl);
  const showLogo = $derived(!!logoUrl && logoBad !== logoUrl);

  // Sin hero: ampliación borrosa y oscurecida de la portada o de la primera captura.
  let blur = $state('');
  $effect(() => {
    const src = showHero ? null : coverPath;
    if (!src) { blur = ''; return; }
    const known = thumbNow(src);
    if (known) { blur = known; return; }
    const ctl = new AbortController();
    backend.thumb(src, ctl.signal).then((u) => decodeImg(u).then(() => { if (!ctl.signal.aborted) blur = u; }), () => {});
    return () => ctl.abort();
  });

  // Estadísticas
  const secs = $derived(T?.playSeconds ?? 0);
  const hh = $derived(Math.min(999, Math.floor(secs / 3600)));
  const mm = $derived(Math.floor(secs / 60) % 60);
  const hs = $derived(String(hh).padStart(3, '0'));
  const ms = $derived(String(mm).padStart(2, '0'));
  const week = $derived(T?.week?.length === 7 ? T.week : Array(7).fill(0));
  const wmax = $derived(Math.max(...week, 1));
  const wtot = $derived(week.reduce((a, b) => a + b, 0));
  const nSes = $derived(T?.sessions ?? 0);
  // Con partidas registradas, los paneles de horas abren la evolución del juego.
  const hasPlays = $derived((T?.plays?.length ?? 0) > 0);
  const fmtWeek = (m: number) => (m > 0 ? `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')} min` : 'sin partidas');

  // Partida recién cerrada: el contador avanza desde el valor previo y la semana crece.
  // Se detecta antes de pintar para que el contador no repita la animación desde cero.
  let gain = $state<{ id: string; label: string; from: string; addMin: number; week0: number[] | null; wtot0: number } | null>(null);
  $effect.pre(() => {
    const g = app.gain;
    if (!g || !T || g.id !== T.gameId || secs === g.secs) return;
    app.gain = null;
    if (Date.now() - g.at > 30 * 60e3) return;
    const h0 = Math.min(999, Math.floor(g.secs / 3600));
    const m0 = Math.floor(g.secs / 60) % 60;
    const week0 = g.day === new Date().toDateString() && g.week.length === 7 ? g.week : null;
    gain = {
      id: g.id,
      label: `+${fmtDur(g.seconds)}`,
      from: String(h0).padStart(3, '0') + String(m0).padStart(2, '0'),
      addMin: Math.floor((secs - g.secs) / 60),
      week0,
      wtot0: week0 ? week0.reduce((a, b) => a + b, 0) : wtot,
    };
  });
  const gx = $derived(gain && gain.id === T?.gameId ? gain : null);

  // Total semanal contando hacia arriba durante la animación.
  let wshow = $state<number | null>(null);
  $effect(() => {
    if (!gx || gx.wtot0 >= wtot || matchMedia('(prefers-reduced-motion: reduce)').matches) { wshow = null; return; }
    const a = gx.wtot0, b = wtot, t0 = performance.now() + 500;
    let raf = 0;
    const step = (now: number) => {
      const k = Math.min(1, Math.max(0, (now - t0) / 1400));
      wshow = Math.round(a + (b - a) * (1 - (1 - k) ** 3));
      if (k < 1) raf = requestAnimationFrame(step); else wshow = null;
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });
  const weekText = $derived(fmtWeek(wshow ?? wtot));
  const wmax0 = $derived(gx?.week0 ? Math.max(...gx.week0, 1) : wmax);
  const barH = (v: number, max: number) => (v ? Math.max(6, (v / max) * 40) : 4);

  // Celda del contador: tira de cifras de abajo (inicio) arriba (final). Con ganancia avanza
  // hacia delante desde la cifra previa y da vueltas completas extra si el tiempo ganado la recorre entera.
  const PLACE = [6000, 600, 60, 10, 1]; // minutos que vale cada cifra hhh:mm
  function strip(d: number, i: number) {
    if (!gx) return { n: d, digits: Array.from({ length: d + 1 }, (_, k) => d - k), up: false };
    const a = +gx.from[i];
    const n = ((d - a + 10) % 10) + 10 * Math.min(2, Math.floor(gx.addMin / (PLACE[i] * 10)));
    return { n, digits: Array.from({ length: n + 1 }, (_, k) => (((d - k) % 10) + 10) % 10), up: n > 0 };
  }

  // Botón Jugar / Jugando
  const playing = $derived(!!T?.gameId && app.playing?.id === T.gameId);
  const via = $derived(T?.source === 'manual' ? 'Ejecutable propio' : `Con ${T?.launcher ?? 'lanzador'}`);
  let tick = $state(Date.now());
  $effect(() => {
    if (!playing) return;
    tick = Date.now();
    const id = setInterval(() => (tick = Date.now()), 1000);
    return () => clearInterval(id);
  });
  const sess = $derived.by(() => {
    const s = Math.max(0, Math.floor((tick - (app.playing?.since ?? tick)) / 1000));
    return `${z(Math.floor(s / 3600))}:${z(Math.floor(s / 60) % 60)}:${z(s % 60)}`;
  });
  const longName = $derived((T?.name.length ?? 0) > 22);
  const isJvn = $derived(!!T?.gameId && (P?.id === 'jvn' || P?.id === 'vn' || P?.icon === 'vn'));
  $effect(() => {
    if (T) {
      const gid = T.id ?? T.gameId ?? T.dir;
      void loadGameAchievements(gid);
    }
  });
</script>

{#if T}
  <TopBar crumbs={['Inicio', P?.name ?? '', T.name]} />

  <div class="glass banner static">
    <div class="face">
      {#if showHero}
        <img class="hero" alt="" src={heroUrl} draggable="false" onerror={() => (heroBad = heroUrl ?? '')} />
      {:else}
        <div class="hfb" class:plain={!blur} style={blur ? `background-image:url(${blur})` : ''}></div>
      {/if}
      <div class="bshade"></div>
    </div>
  </div>
  {#if showLogo}
    <img class="dlogo" alt={T.name} src={logoUrl} draggable="false" onerror={() => (logoBad = logoUrl ?? '')} />
  {:else}
    <div class="dlogo txt" class:sm={longName}>{T.name}</div>
  {/if}

  <AchievementCard
    gameId={T.id ?? T.gameId ?? T.dir}
    title={T.name}
    coverPath={coverPath}
    summary={achState.summary}
    focused={app.focus === 'dach'}
  />

  <div class="dchips">
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="stpill pill fx {T.status ? `st-${T.status}` : 'st-none'}"
      class:f={app.focus === 'dst'}
      data-k="dst"
      data-act="status"
      role="button"
      tabindex="-1"
      onclick={() => openStatusBar(app.t)}
      style={T.status ? `--c:${LIST_COLOR[T.status].c}` : ''}
      title="Cambiar estado del juego"
    >
      {@html T.status === 'finished' ? ICON.check : ICON.checks}
      <b>{T.status ? STATUS_LABEL[T.status] : 'Sin estado'}</b>
      {#if T.status === 'finished' && T.finishedMs}
        <small>{new Date(T.finishedMs).toLocaleDateString('es-ES', { day: 'numeric', month: 'short', year: 'numeric' })}</small>
      {/if}
      {#if T.status === 'finished' && T.rating}
        <small>{'★'.repeat(T.rating)}</small>
      {/if}
    </div>

    {#if !T.members}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div
        class="relpill pill fx"
        class:f={app.focus === 'drel'}
        data-k="drel"
        data-act="release"
        role="button"
        tabindex="-1"
        title="Cambiar fecha de lanzamiento"
        onclick={() => openRelease(app.t)}
      >
        {@html ICON.calendar}
        {#if T.release}
          <b>{T.release.month ? 'Salió el' : 'Salió en'}</b>
          <small>{fmtRelease(T.release)}</small>
        {:else}
          <b>Fecha de lanzamiento…</b>
        {/if}
      </div>
    {/if}
  </div>

  {#if T.launchable}
    <div class="glass play fx" class:f={app.focus === 'play'} class:on={playing} data-k="play" data-act="play">
      <div class="face">
        <span class="pi">{@html ICON.play}</span>
        <span><b>{playing ? 'Jugando' : 'Jugar'}</b><small>{playing ? `Sesión ${sess}` : via}</small></span>
      </div>
    </div>
  {:else}
    <div class="nolaunch pill"><b>Sin lanzador</b><small>Usa «Añadir juego» en Inicio</small></div>
  {/if}

  <div class="dpanel dhours pill" class:fx={hasPlays} class:f={app.focus === 'dhrs'} data-k="dhrs" data-act="evo">
    <div class="hsub">Horas jugadas{nSes > 0 ? ` · ${nSes} ${nSes === 1 ? 'sesión' : 'sesiones'}` : ''}</div>
    {#snippet cell(d: number, i: number)}
      {@const st = strip(d, i)}
      <span class="c" class:up={st.up} style="--gd:{(gx ? 500 + (4 - i) * 110 : i * 90) + 1500}ms"><b style="--n:{-st.n};animation-delay:{gx ? 500 + (4 - i) * 110 : i * 90}ms">{#each st.digits as v}<span>{v}</span>{/each}</b></span>
    {/snippet}
    {#if gx}{#key gx}<span class="gainchip">{gx.label}</span>{/key}{/if}
    {#key `${T.gameId ?? T.dir}|${hs}|${ms}`}
      <div class="odo" class:gain={!!gx}>
        {#each [...hs] as d, i}{@render cell(+d, i)}{/each}<span class="u">h</span>
        {#each [...ms] as d, i}{@render cell(+d, i + 3)}{/each}<span class="u">min</span>
      </div>
    {/key}
  </div>

  {#if T.launchable}
    <div class="dpanel dweek pill" class:fx={hasPlays} class:f={app.focus === 'dwk'} data-k="dwk" data-act="evo">
      <div class="chart">
        {#if gx}{#key gx}<span class="gainchip wk">{gx.label}</span>{/key}{/if}
        <div class="hsub" style="margin:0">Esta semana · <span class:wcount={wshow !== null}>{weekText}</span></div>
        {#key gx}
          <div class="weekbars" class:gain={!!gx}>
            {#each week as v, i}
              {@const v0 = Math.min(v, gx?.week0?.[i] ?? v)}
              <i class:z={!v} class:today={i === 6} style="height:{barH(v, wmax)}px;--h0:{barH(v0, wmax0)}px;--dl:{i * 70}ms">{#if v > v0}<em style="height:{((v - v0) / v) * 100}%"></em>{/if}</i>
            {/each}
          </div>
        {/key}
      </div>
    </div>
  {/if}

  <div class="dsect">Capturas<span>{shots.length}</span></div>
  {#each row as s, i (s.path)}
    <div class="glass dshot fx" class:f={app.focus === 'ds' + i} data-k="ds{i}" data-act="dshot:{i}" style="left:{64 + i * 192}px">
      <div class="face"><Thumb path={s.path} /></div>
    </div>
  {/each}
  {#if shots.length}
    <div class="glass dall fx" class:f={app.focus === 'dall'} data-k="dall" data-act="dall" style="left:{64 + row.length * 192}px;width:{Math.round(176 * 0.8)}px">
      <div class="face"><div>Ver todas<span>{nCapturas(shots.length)}</span></div></div>
    </div>
  {:else}
    <div class="dnone">Todavía no hay capturas de este juego</div>
  {/if}

  <Hints list={[
    ['A', app.focus === 'play' ? (playing ? 'En marcha' : 'Jugar') : app.focus === 'dach' ? 'Ver logros' : app.focus === 'dst' ? 'Cambiar estado' : app.focus === 'drel' ? 'Cambiar fecha' : app.focus === 'dhrs' || app.focus === 'dwk' ? 'Estadísticas' : 'Seleccionar', 'a'],
    ...(app.focus === 'dach' ? [['Y', 'Cambiar diseño (' + (VARIANT_NAMES[app.config.achievementsCardVariant ?? achState.summary?.variant ?? 1] ?? '') + ')', 'y'] as [string, string, string]] : []),
    ...(T.launchable && !playing ? [['X', 'Jugar', 'x'] as [string, string, string]] : []),
    ['B', 'Volver', 'b'],
    ['Start', 'Opciones', 'menu'],
  ]} />
{/if}

<style>
  /* Estado y fecha de lanzamiento, uno tras otro sobre el hero */
  .dchips{position:absolute;left:100px;top:108px;z-index:6;display:flex;gap:10px}
  .dchips :global(.stpill){position:static;cursor:pointer}
  .dchips :global(.stpill.st-none){background:rgba(40,48,64,.82);color:#94a3b8;border:1px dashed rgba(255,255,255,.25)}
  .relpill{height:36px;padding:0 16px 0 12px;border-radius:999px;display:flex;align-items:center;gap:8px;font-size:17px;font-weight:800;color:#fff;box-sizing:border-box;cursor:pointer;
    background:linear-gradient(180deg,rgba(60,68,84,.78),rgba(30,36,48,.82));box-shadow:inset 0 1px 0 rgba(255,255,255,.35),0 6px 14px -6px rgba(0,0,0,.5)}
  .relpill :global(svg){width:18px;height:18px}
  .relpill small{font-size:15px;font-weight:700;opacity:.92}


</style>
