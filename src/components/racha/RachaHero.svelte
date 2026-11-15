<script lang="ts">
  // Columna izquierda de la pantalla de racha: llama grande con brasas, número de días,
  // chip de estado, mensaje explicativo y barra de progreso de hoy.
  import { dayDate, GOAL_COL, groupOf, type GoalTrack, type StreakState } from '../../lib/readgoals';
  import { fmtGoal, fmtMin, freeDayName } from '../../lib/readfmt';
  import IconoRacha from './IconoRacha.svelte';

  const MON_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
  const EMBERS = [
    { dx: -10, ex: '-16px', ed: '2.1s', edl: '0s' },
    { dx: 8, ex: '14px', ed: '2.4s', edl: '0.5s' },
    { dx: -4, ex: '-8px', ed: '1.9s', edl: '1.1s' },
    { dx: 12, ex: '18px', ed: '2.6s', edl: '1.6s' },
    { dx: -12, ex: '-18px', ed: '2.3s', edl: '0.8s' },
    { dx: 3, ex: '10px', ed: '2.0s', edl: '2.0s' },
  ];

  let {
    track,
    shownStreak,
    pending,
    today,
    calm,
  }: {
    track: GoalTrack;
    shownStreak: number;
    pending: boolean;
    today: number;
    calm: boolean;
  } = $props();

  const col = $derived(GOAL_COL[track.key]);
  const isPlay = $derived(groupOf(track.key) === 'play');
  const effState: StreakState = $derived(pending && track.state === 'lit' ? 'ok' : track.state);

  const sinceText = $derived.by(() => {
    const rec = `${track.best} ${track.best === 1 ? 'día' : 'días'}`;
    if (shownStreak > 0) {
      const end = track.done && !pending ? today : today - 1;
      const from = dayDate(end - shownStreak + 1);
      return `Desde el ${from.getUTCDate()} de ${MON_L[from.getUTCMonth()]} · récord: ${rec}`;
    }
    if (track.best > 0) return `Récord: ${rec}`;
    return `Cumple ${fmtGoal(track.goal)} hoy para empezarla`;
  });

  // Barra de hoy: si está pendiente de encender, se frena antes de la meta hasta que la animación la llena.
  const goal = $derived(Math.max(1, track.goal));
  const todayShown = $derived(pending ? Math.min(track.today, goal * 0.6) : track.today);
  const needFree = $derived(track.urgent ? track.urgent.owe : track.open.length ? track.open[0].owe : 0);
  const barMax = $derived(Math.max(goal + (todayShown > goal ? (todayShown - goal) + needFree : needFree), todayShown, 1) * 1.08);
  const pct = (v: number) => `${Math.min(100, Math.max(0, (v / barMax) * 100)).toFixed(2)}%`;
  const gaveToday = $derived(Math.round(track.week[6]?.gave ?? 0));
  const leftMeta = $derived(Math.max(1, Math.ceil(track.goal - track.today)));
  const lastOpen = $derived(track.open.length ? track.open[track.open.length - 1] : null);
</script>

<div class="hero" data-g={track.key}>
  <div class="k">Racha de {col.name}</div>
  <div class="n">
    <span class="flm-wrap">
      {#if !calm}
        {#each EMBERS as e}
          <span class="emb" style="left:{28 + e.dx}px;top:22px;--ex:{e.ex};--ed:{e.ed};--edl:{e.edl}"></span>
        {/each}
      {/if}
      <svg class="flm" width="58" height="70" viewBox="0 0 24 24" fill="none" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" />
      </svg>
    </span>
    <span class="nw"><span class="nn">{shownStreak}</span></span>
    <small>{shownStreak === 1 ? 'día' : 'días'}</small>
  </div>
  <div class="best">{sinceText}</div>
  <div class="chip-wrap">
    {#if effState === 'urgent'}
      <span class="chip bad"><IconoRacha name="warn" size={15} stroke="var(--on)" />Último plazo: {track.urgent && track.urgent.left <= 1 ? 'hoy' : 'mañana'}</span>
    {:else if effState === 'tope'}
      <span class="chip warn"><IconoRacha name="lock" size={15} />Sin días libres hasta el {Number.isFinite(track.tokenBack) ? freeDayName(track.tokenBack, false) : ''}</span>
    {:else if effState === 'debt'}
      <span class="chip warn"><IconoRacha name="moon" size={15} />Día libre · debes {fmtMin(track.owe, true)}</span>
    {:else if effState === 'lit'}
      <span class="chip lit"><IconoRacha name="flame" size={15} stroke="var(--on)" />Encendida hoy</span>
    {:else}
      <span class="chip ok"><IconoRacha name="check" size={15} stroke="var(--on)" />Al día</span>
    {/if}
  </div>
</div>

<div class="msg gin">
  {#if effState === 'urgent' && track.urgent}
    Al día libre del <b class="a">{freeDayName(track.urgent.day, true)}</b> le faltan <b class="r">{fmtMin(track.urgent.owe, true)}</b> y su plazo acaba <b class="r">{track.urgent.left <= 1 ? 'hoy' : 'mañana'}</b>. Si no lo recuperas, la racha se rompe.
  {:else if effState === 'tope'}
    Has usado tus <b class="a">2 días libres</b>. Si fallas antes del <b>{Number.isFinite(track.tokenBack) ? freeDayName(track.tokenBack, false) : ''}</b>, la racha se rompe.{#if lastOpen} Aún debes <b class="a">{fmtMin(track.owe, true)}</b> hasta el <b>{freeDayName(lastOpen.deadline, false)}</b>.{/if}
  {:else if effState === 'debt' && track.open.length}
    {@const f = track.open[0]}
    El <b class="a">{freeDayName(f.day, true)}</b> no llegaste: usaste un día libre. Recupera <b class="a">{fmtMin(f.owe, true)}</b> antes del <b>{freeDayName(f.deadline, false)}</b> {isPlay ? 'jugando' : 'leyendo'} por encima de la meta, poco a poco.
  {:else if effState === 'lit'}
    Racha encendida. Hoy ya has cumplido. Lo que {isPlay ? 'juegues' : 'leas'} de más <b>no se guarda</b> para otro día.
  {:else if pending}
    Meta de hoy alcanzada. Encendiendo la racha de {col.name}…
  {:else}
    Vas al día. Te faltan <b>{fmtMin(leftMeta, true)}</b> de {col.name} para encender la racha de hoy.
  {/if}
</div>

<div class="tday gin">
  <div class="row">
    <span class="k">Hoy · {freeDayName(today, true)}</span>
    <span class="k">meta {fmtGoal(track.goal)}{#if track.nextGoal !== track.goal && track.nextGoal > 0} · mañana {fmtGoal(track.nextGoal)}{/if}</span>
  </div>
  <div class="big">{fmtMin(Math.round(track.today), true)} <small>de {fmtGoal(track.goal)}</small></div>
  <div class="tbar">
    {#if needFree > 0}
      <i class="tz" style="left:{pct(Math.max(goal, todayShown))};width:{pct(needFree)}"></i>
    {/if}
    <i class="tf" style="width:{pct(todayShown)}"></i>
    <i class="tm" style="left:{pct(goal)}"></i>
  </div>
  <div class="sub">
    {#if track.done && !pending}
      Meta cumplida{#if gaveToday > 0} · <b>{fmtMin(gaveToday, true)}</b> de más ya van al día libre{:else if track.open.length} · faltan <b>{fmtMin(track.owe, true)}</b> para el día libre{/if}
    {:else if pending}
      Meta de hoy cumplida
    {:else}
      Faltan <b>{fmtMin(leftMeta, true)}</b> para la meta{#if needFree > 0} y <b>{fmtMin(needFree, true)}</b> más para el día libre{/if}
    {/if}
  </div>
</div>

<div class="vline"></div>

<style>
  .k {
    font-size: var(--t-xs);
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-2);
  }
  .gin {
    background: var(--glass-in);
    border-radius: var(--r-m);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .hero {
    position: absolute;
    left: 28px;
    top: 24px;
    width: 320px;
    animation: fade 0.4s ease 0.25s both;
  }
  .hero .n {
    display: flex;
    align-items: flex-end;
    gap: 10px;
    margin-top: 6px;
  }
  .flm-wrap {
    position: relative;
    display: block;
    width: 58px;
    height: 70px;
    margin-bottom: 14px;
    flex: none;
  }
  .flm {
    display: block;
    width: 58px;
    height: 70px;
    stroke: var(--c);
    fill: color-mix(in srgb, var(--c) 16%, transparent);
    transform-origin: 50% 90%;
    animation: flick2 1.6s ease-in-out infinite;
    filter: drop-shadow(0 0 8px color-mix(in srgb, var(--c) 60%, transparent));
  }
  .emb {
    position: absolute;
    z-index: 5;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--c);
    box-shadow: 0 0 6px var(--c);
    pointer-events: none;
    opacity: 0;
    animation: embr var(--ed) ease-out var(--edl) infinite;
  }
  .nw {
    display: inline-block;
    height: 104px;
    overflow: hidden;
    vertical-align: bottom;
  }
  .nn {
    display: block;
    font-size: 104px;
    line-height: 104px;
    font-weight: 800;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
    background: linear-gradient(120deg, var(--ci), var(--c2));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .hero .n small {
    font-size: 26px;
    font-weight: 800;
    color: var(--ink-2);
    padding-bottom: 16px;
  }
  .hero .best {
    font-size: var(--t-s);
    color: var(--ink-2);
    margin-top: 4px;
    font-weight: 600;
  }
  .chip-wrap {
    margin-top: 12px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: var(--r-pill);
    font-size: var(--t-s);
    font-weight: 800;
    color: var(--on);
    background: var(--c-fill);
    white-space: nowrap;
    box-shadow: none;
    cursor: default;
  }
  .chip.lit {
    background: var(--c-fill);
  }
  .chip.warn {
    background: var(--am-soft);
    color: var(--amk);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--am) 60%, transparent);
  }
  .chip.bad {
    background: var(--bad-f);
    color: var(--on);
  }
  .msg {
    position: absolute;
    left: 28px;
    top: 238px;
    width: 320px;
    padding: 14px 16px;
    box-sizing: border-box;
    font-size: var(--t-m);
    line-height: 1.45;
    font-weight: 600;
    color: var(--ink);
    animation: fade 0.4s ease 0.4s both;
  }
  .msg b {
    color: var(--ci);
  }
  .msg b.a {
    color: var(--amk);
  }
  .msg b.r {
    color: var(--badk);
  }
  .tday {
    position: absolute;
    left: 28px;
    top: 404px;
    width: 320px;
    height: 124px;
    padding: 14px 16px;
    box-sizing: border-box;
    animation: fade 0.4s ease 0.5s both;
  }
  .tday .row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }
  .tday .big {
    font-size: var(--t-2xl);
    font-weight: 800;
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }
  .tday .big small {
    font-size: var(--t-m);
    color: var(--ink-2);
    font-weight: 700;
  }
  .tbar {
    position: relative;
    height: 16px;
    border-radius: var(--r-pill);
    background: color-mix(in srgb, var(--ink-3) 18%, transparent);
    margin: 10px 0 8px;
    overflow: visible;
  }
  .tbar .tf {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: var(--r-pill);
    background: linear-gradient(90deg, var(--c), color-mix(in srgb, var(--c) 62%, white));
    transform-origin: 0 50%;
    transition: width 0.6s var(--e-soft);
    animation: barin 1s cubic-bezier(0.3, 0.8, 0.3, 1) 0.5s both;
  }
  .tbar .tz {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 0 999px 999px 0;
    background: var(--hatch);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--am) 60%, transparent);
  }
  .tbar .tm {
    position: absolute;
    top: -5px;
    bottom: -5px;
    width: 3px;
    border-radius: 2px;
    background: var(--ink);
  }
  .tday .sub {
    font-size: var(--t-s);
    color: var(--ink-2);
    font-weight: 600;
  }
  .tday .sub b {
    color: var(--ink);
  }
  .vline {
    position: absolute;
    left: 372px;
    top: 24px;
    bottom: 24px;
    width: 1px;
    background: var(--line);
  }

  @keyframes fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @keyframes barin {
    from { transform: scaleX(0); }
  }
  @keyframes flick2 {
    0%, 100% { transform: scale(1, 1) rotate(0); }
    25% { transform: scale(0.94, 1.08) rotate(-4deg); }
    50% { transform: scale(1.05, 0.95) rotate(2deg); }
    75% { transform: scale(0.97, 1.05) rotate(4deg); }
  }
  @keyframes embr {
    0% { opacity: 0; transform: translate(0, 0) scale(1); }
    20% { opacity: 1; }
    100% { opacity: 0; transform: translate(var(--ex), -60px) scale(0.3); }
  }
  @media (prefers-reduced-motion: reduce) {
    .hero, .msg, .tday, .tbar .tf {
      animation: none !important;
      transition: none !important;
    }
    .flm, .emb {
      animation: none !important;
    }
  }
</style>
