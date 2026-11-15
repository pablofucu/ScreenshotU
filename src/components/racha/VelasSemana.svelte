<script lang="ts">
  // Gráfico de los últimos 7 días como velas: base hasta la meta, exceso por encima,
  // tramo que paga días libres, tramo recuperado/pendiente, llamas, lunas y arcos de pago.
  import { untrack } from 'svelte';
  import type { GoalTrack } from '../../lib/readgoals';
  import { fmtGoal, fmtMin, freeDayName, freeLeft } from '../../lib/readfmt';
  import IconoRacha from './IconoRacha.svelte';

  const TT = 92;
  const TH = 244;
  const TW = 78;
  const TG = 32;
  const X0 = 392 + (798 - (7 * TW + 6 * TG)) / 2;

  const CANDLE_EMBERS = [
    { dx: -4, ex: '-10px', ed: '1.8s', edl: '0.2s' },
    { dx: 1, ex: '4px', ed: '2.1s', edl: '0.9s' },
    { dx: 5, ex: '12px', ed: '1.7s', edl: '1.5s' },
  ];

  let {
    track,
    pending,
    calm,
    igniteTick = 0,
  }: {
    track: GoalTrack;
    pending: boolean;
    calm: boolean;
    igniteTick?: number;
  } = $props();

  let stageEl: HTMLDivElement | undefined = $state();

  const cap = (s: string) => (s ? s.charAt(0).toUpperCase() + s.slice(1) : '');
  const baseGoal = $derived(Math.max(1, track.goal));

  const maxVal = $derived.by(() => {
    let hi = 1.6 * baseGoal;
    for (const d of track.week) {
      const top = Math.max(d.goal || baseGoal, d.min + (d.free ? d.free.paid + d.free.owe : 0));
      if (top * 1.12 > hi) hi = top * 1.12;
    }
    return hi;
  });

  const K = $derived((TH - 12) / maxVal);

  interface Layer {
    cls: 'b' | 'x' | 'xu' | 'p' | 'o';
    bottom: number;
    h: number;
    delay: number;
    lo: boolean;
    hi: boolean;
  }

  const candles = $derived.by(() => {
    return track.week.map((d, i) => {
      const g = Math.max(1, d.goal || baseGoal);
      const isWaitToday = d.today && pending;
      const effMin = isWaitToday ? Math.min(d.min, g * 0.6) : d.min;
      const b = Math.min(effMin, g) * K;
      const extraTot = isWaitToday ? 0 : Math.max(0, effMin - g) * K;
      const u = isWaitToday ? 0 : Math.min(d.gave * K, extraTot);
      const x = Math.max(0, extraTot - u);
      const p = d.free ? d.free.paid * K : 0;
      const o = d.free ? d.free.owe * K : 0;

      const raw: Array<Omit<Layer, 'lo' | 'hi'>> = [];
      if (b > 0.5) raw.push({ cls: 'b', bottom: 0, h: b, delay: 0.45 + i * 0.07 });
      if (x > 0.5) raw.push({ cls: 'x', bottom: b, h: x, delay: 0.75 + i * 0.07 });
      if (u > 0.5) raw.push({ cls: 'xu', bottom: b + x, h: u, delay: 0.85 + i * 0.07 });
      if (d.free && p > 0.5) raw.push({ cls: 'p', bottom: b, h: p, delay: 1.5 });
      if (d.free && o > 0.5) raw.push({ cls: 'o', bottom: b + p, h: o, delay: 1.0 });

      const layers: Layer[] = raw.map((l, idx) => ({
        ...l,
        lo: idx === 0,
        hi: idx === raw.length - 1,
      }));
      const topPx = layers.length ? layers[layers.length - 1].bottom + layers[layers.length - 1].h : 0;
      const isLit = d.met && !isWaitToday && topPx >= 2;
      const colX = X0 + i * (TW + TG);

      let tagTop = TT - 30;
      if (d.free || d.gave > 0 || d.broken) {
        const iconTop = TT + TH - 6 - topPx - (isLit ? 56 : 36);
        if (iconTop < tagTop) tagTop = Math.max(58, iconTop);
      }

      return { d, i, g, isWaitToday, layers, topPx, isLit, colX, tagTop };
    });
  });

  function spawnTravel(pts: Array<[number, number]>, delay: number, dur: number) {
    if (!stageEl || calm) return;
    const dot = document.createElement('i');
    dot.className = 'fdot';
    stageEl.appendChild(dot);
    const kf = pts.map((p, idx) => ({
      transform: `translate(${p[0]}px, ${p[1]}px) scale(${idx === 0 || idx === pts.length - 1 ? 0.5 : 1})`,
      opacity: idx === 0 || idx === pts.length - 1 ? 0 : 1,
    }));
    dot.animate(kf, { duration: dur, delay, easing: 'ease-in-out', fill: 'both' }).onfinish = () => dot.remove();
  }

  function quad(a: [number, number], b: [number, number], lift: number, n = 24): Array<[number, number]> {
    const cx = (a[0] + b[0]) / 2;
    const cy = Math.min(a[1], b[1]) - lift;
    const out: Array<[number, number]> = [];
    for (let i = 0; i <= n; i++) {
      const t = i / n;
      const u = 1 - t;
      out.push([u * u * a[0] + 2 * u * t * cx + t * t * b[0], u * u * a[1] + 2 * u * t * cy + t * t * b[1]]);
    }
    return out;
  }

  function spawnBurst(cx: number, cy: number, n = 18) {
    if (!stageEl || calm) return;
    const cols = ['var(--c)', 'color-mix(in srgb, var(--c) 50%, white)', '#fff'];
    for (let i = 0; i < n; i++) {
      const sp = document.createElement('i');
      sp.className = 'spark';
      sp.style.left = `${cx - 4}px`;
      sp.style.top = `${cy - 4}px`;
      sp.style.background = cols[i % cols.length];
      stageEl.appendChild(sp);
      const a = (Math.PI * 2 * i) / n + (i % 3) * 0.12;
      const r = 42 + (i % 4) * 12;
      sp.animate(
        [
          { transform: 'translate(0, 0) scale(1)', opacity: 1 },
          { transform: `translate(${Math.cos(a) * r}px, ${Math.sin(a) * r}px) scale(0.2)`, opacity: 0 },
        ],
        { duration: 720 + (i % 3) * 100, easing: 'cubic-bezier(0.2, 0.8, 0.3, 1)' },
      ).onfinish = () => sp.remove();
    }
  }

  // Arcos dorados cuando un día de la semana paga un día libre anterior.
  $effect(() => {
    const key = track.key;
    const week = track.week;
    const kPx = K;
    const isCalm = calm;
    void key;
    if (!stageEl || isCalm) return;

    const timers: ReturnType<typeof setTimeout>[] = [];
    const rem = week.map((w) => (w.free ? w.free.paid : 0));
    const paidAcc = week.map(() => 0);
    const flows: Array<{ from: number; to: number; amt: number; before: number }> = [];

    for (let j = 1; j < week.length; j++) {
      if (week[j].today && pending) continue;
      let e = week[j].gave;
      if (e <= 0) continue;
      for (let i = 0; i < j && e > 0; i++) {
        if (!week[i].free || rem[i] <= 0) continue;
        const amt = Math.min(e, rem[i]);
        if (amt > 0.5) {
          flows.push({ from: j, to: i, amt, before: paidAcc[i] });
          paidAcc[i] += amt;
          rem[i] -= amt;
          e -= amt;
        }
      }
    }

    const t0 = setTimeout(() => {
      flows.forEach((f, n) => {
        const xa = X0 + f.from * (TW + TG) + TW / 2;
        const ya = TT + TH - 6 - (week[f.from].min - week[f.from].gave / 2) * kPx;
        const xb = X0 + f.to * (TW + TG) + TW / 2;
        const yb = TT + TH - 6 - (week[f.to].min + f.before + f.amt / 2) * kPx;
        for (let k = 0; k < 3; k++) {
          spawnTravel(quad([xa, ya], [xb, yb], 70), 900 + n * 260 + k * 120, 800);
        }
      });
    }, 50);
    timers.push(t0);

    return () => {
      timers.forEach(clearTimeout);
      stageEl?.querySelectorAll('.fdot, .spark').forEach((el) => el.remove());
    };
  });

  // Al encender hoy, chispas en la vela de hoy y ola luminosa recorriendo la racha hacia atrás.
  let lastTick = 0;
  $effect(() => {
    const t = igniteTick;
    if (!t || t === lastTick || calm || !stageEl) {
      lastTick = t;
      return;
    }
    lastTick = t;
    const list = untrack(() => candles);
    const todayC = list[6];
    if (todayC && todayC.isLit) {
      spawnBurst(todayC.colX + TW / 2, TT + TH - 6 - todayC.topPx - 16, 18);
    }
    // Recorre hacia atrás las velas cumplidas que forman la racha sin cortes.
    let step = 0;
    for (let i = 5; i >= 0; i--) {
      const c = list[i];
      if (!c) break;
      if (c.isLit) {
        const el = stageEl.querySelector<HTMLElement>(`[data-candle="${i}"] .cflame`);
        el?.animate(
          [{ transform: 'scale(1)' }, { transform: 'scale(1.42, 1.58)' }, { transform: 'scale(1)' }],
          { duration: 480, delay: 220 + step * 110, easing: 'ease-out' },
        );
        step++;
      } else if (c.d.free) {
        step++;
      } else {
        break;
      }
    }
  });
</script>

<div class="velas-stage" bind:this={stageEl}>
  <div class="dh">
    <div class="t">
      Tus últimos 7 días
      <small>La semana avanza contigo: siempre ves los siete últimos</small>
    </div>
    <div class="legend">
      <span><IconoRacha name="flame" size={14} stroke="var(--c)" fill="color-mix(in srgb, var(--c) 25%, transparent)" />Cumplido</span>
      <span><IconoRacha name="moon" size={14} stroke="var(--am)" fill="color-mix(in srgb, var(--am) 25%, transparent)" />Día libre</span>
      <span><i class="lg-x"></i>De más</span>
      <span><i class="lg-p"></i>Recuperado</span>
      <span><i class="lg-l"></i>Por recuperar</span>
    </div>
  </div>

  <div class="dmeta" style="left:{X0 - 6}px;width:{7 * TW + 6 * TG + 12}px;top:{TT + TH - 6 - baseGoal * K}px">
    <b>Meta · {fmtGoal(track.goal)}</b>
  </div>

  {#each candles as c (c.d.day)}
    <div
      class="tube"
      class:td={c.d.today}
      class:lb={!!c.d.free}
      class:met={c.isLit}
      class:br={c.d.broken}
      data-candle={c.i}
      data-lit={c.d.today ? '' : undefined}
      style="left:{c.colX}px;top:{TT}px;height:{TH}px"
    >
      {#each c.layers as l (l.cls)}
        <i
          class={l.cls}
          class:lo={l.lo}
          class:hi={l.hi}
          style="bottom:{l.bottom.toFixed(1)}px;height:{l.h.toFixed(1)}px;--d:{l.delay}s"
        ></i>
      {/each}

      {#if c.isLit}
        <span
          class="cnd top-ic on"
          style="bottom:{c.topPx + 3}px;--ig:{c.d.today ? 0.12 : 1.0 + c.i * 0.12}s"
        >
          <span class="cflame" style="--fd:{1.15 + (c.i % 3) * 0.22}s;--fdl:{(c.i % 4) * 0.25}s">
            <IconoRacha
              name="flame"
              size={28}
              stroke="var(--c)"
              fill="color-mix(in srgb, var(--c) 28%, var(--tint-base))"
            />
          </span>
        </span>
        {#if !calm}
          {#each CANDLE_EMBERS as e}
            <span
              class="emb"
              style="left:{TW / 2 - 2 + e.dx}px;bottom:{c.topPx + 22}px;--ex:{e.ex};--ed:{e.ed};--edl:{e.edl}"
            ></span>
          {/each}
        {/if}
      {:else if c.d.free}
        <span
          class="top-ic moon on"
          style="bottom:{Math.max(c.topPx, c.d.min * K) + 12}px;--ig:{1.1 + c.i * 0.12}s"
        >
          <span class="micon" style="--fdl:{(c.i % 3) * 0.4}s">
            <IconoRacha
              name="moon"
              size={24}
              stroke="var(--am)"
              fill="color-mix(in srgb, var(--am) 28%, var(--tint-base))"
            />
          </span>
        </span>
      {:else if c.d.today}
        <span class="top-ic wait on" style="bottom:{c.g * K + 10}px;--ig:1.1s">
          <IconoRacha name="flame" size={28} stroke="var(--c)" fill="none" />
        </span>
      {/if}
    </div>

    <div class="dlab" class:td={c.d.today} style="left:{c.colX + TW / 2 - 55}px;top:{TT + TH + 10}px;--d:{0.4 + c.i * 0.05}s">
      {c.d.today ? 'Hoy' : freeDayName(c.d.day, false)}
      <small>{fmtMin(Math.round(c.d.min), true)}</small>
    </div>

    {#if c.d.free}
      <span
        class="dtag tag"
        class:c={c.d.free.done}
        class:r={!c.d.free.done && c.d.free.left <= 2}
        style="left:{c.colX + TW / 2}px;top:{c.tagTop}px;--d:1.05s"
      >
        {#if c.d.free.done}
          <IconoRacha name="check" size={12} />Recuperado
        {:else}
          <IconoRacha name="clock" size={12} />{cap(freeLeft(c.d.free))}
        {/if}
      </span>
    {:else if c.d.gave > 0 && !c.isWaitToday}
      <span class="dtag tag" style="left:{c.colX + TW / 2}px;top:{c.tagTop}px;--d:0.95s">
        <IconoRacha name="back" size={12} />Paga {fmtMin(Math.round(c.d.gave), true)}
      </span>
    {:else if c.d.broken}
      <span class="dtag tag r" style="left:{c.colX + TW / 2}px;top:{c.tagTop}px;--d:0.95s">
        <IconoRacha name="warn" size={12} />Rota
      </span>
    {/if}
  {/each}
</div>

<style>
  .velas-stage {
    position: absolute;
    inset: 0;
  }
  .dh {
    position: absolute;
    left: 392px;
    top: 22px;
    width: 798px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .dh .t {
    font-size: var(--t-l);
    font-weight: 800;
    color: var(--ink);
  }
  .dh .t small {
    display: block;
    font-size: var(--t-s);
    font-weight: 600;
    color: var(--ink-2);
  }
  .legend {
    display: flex;
    gap: 12px;
    align-items: center;
    font-size: var(--t-xs);
    font-weight: 700;
    color: var(--ink-2);
  }
  .legend span {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .legend i {
    width: 12px;
    height: 12px;
    border-radius: 4px;
    display: block;
  }
  .lg-x {
    background: color-mix(in srgb, var(--c) 20%, var(--tint-base));
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--c) 70%, transparent);
  }
  .lg-l {
    background: var(--hatch);
    box-shadow: inset 0 0 0 1.5px var(--am);
  }
  .lg-p {
    background: linear-gradient(0deg, var(--am), var(--am2));
  }

  .dmeta {
    position: absolute;
    height: 0;
    border-top: 2px dashed color-mix(in srgb, var(--ink-2) 55%, transparent);
    z-index: 4;
    animation: fade 0.5s ease 0.8s both;
  }
  .dmeta b {
    position: absolute;
    right: 100%;
    top: -11px;
    margin-right: 8px;
    height: 20px;
    padding: 0 8px;
    display: flex;
    align-items: center;
    border-radius: var(--r-pill);
    background: var(--face-hi);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--t-xs);
    font-weight: 800;
    letter-spacing: 0.02em;
    color: var(--ink-2);
    white-space: nowrap;
  }

  .tube {
    position: absolute;
    width: 78px;
    border-radius: 22px;
    background: color-mix(in srgb, var(--ink-3) 9%, transparent);
    box-shadow: inset 0 0 0 1px var(--line), inset 0 10px 16px -10px rgba(0, 0, 0, 0.1);
    overflow: visible;
    clip-path: inset(-70px -30px 0 -30px round 22px);
  }
  .tube::after {
    content: '';
    position: absolute;
    left: 8px;
    top: 8px;
    bottom: 8px;
    width: 9px;
    border-radius: 9px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.65), rgba(255, 255, 255, 0));
    pointer-events: none;
    z-index: 3;
  }
  .tube.met {
    box-shadow: inset 0 0 0 1px var(--line), inset 0 -40px 40px -30px color-mix(in srgb, var(--c) 30%, transparent);
  }
  .tube.td {
    box-shadow: inset 0 0 0 2.5px var(--c);
    animation: pulse 2.2s ease-in-out infinite;
  }
  .tube.met.td {
    box-shadow: inset 0 0 0 2.5px var(--c), inset 0 -40px 40px -30px color-mix(in srgb, var(--c) 35%, transparent);
  }
  .tube.lb {
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--am) 75%, transparent);
  }
  .tube.br {
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--bad) 75%, transparent);
  }

  .tube > i {
    position: absolute;
    left: 6px;
    right: 6px;
    display: block;
    translate: 0 -6px;
    border: 0 solid transparent;
    border-top-width: 2px;
    background-clip: padding-box;
    border-radius: 3px;
    box-sizing: border-box;
    animation: grow 0.7s cubic-bezier(0.3, 0.8, 0.3, 1) var(--d) both;
    transition: height 0.55s var(--e-soft), bottom 0.55s var(--e-soft);
  }
  .tube > i.lo {
    border-top-width: 0;
    border-bottom-left-radius: 16px;
    border-bottom-right-radius: 16px;
  }
  .tube > i.hi {
    border-top-left-radius: 16px;
    border-top-right-radius: 16px;
  }
  .tube .b {
    background: linear-gradient(0deg, var(--c), color-mix(in srgb, var(--c) 62%, white));
  }
  :global(#stage[data-theme='dark']) .tube .b {
    background: linear-gradient(0deg, color-mix(in srgb, var(--c) 85%, black), var(--c));
  }
  .tube .x {
    background: color-mix(in srgb, var(--c) 20%, var(--tint-base));
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--c) 70%, transparent);
  }
  .tube .xu {
    background: color-mix(in srgb, var(--am) 30%, var(--tint-base));
    box-shadow: inset 0 0 0 1.5px var(--am);
  }
  .tube .p {
    background: linear-gradient(0deg, var(--am), var(--am2));
  }
  .tube .o {
    background: var(--hatch);
    box-shadow: inset 0 0 0 1.5px var(--am);
  }

  .top-ic {
    position: absolute;
    left: 50%;
    z-index: 5;
    pointer-events: none;
    transform: translateX(-50%);
    transform-origin: 50% 100%;
    transition: bottom 0.55s var(--e-soft);
  }
  .top-ic.on {
    animation: ign 0.6s cubic-bezier(0.3, 1.7, 0.5, 1) var(--ig, 0s) both;
  }
  .cflame {
    display: block;
    transform-origin: 50% 95%;
    animation: flick2 var(--fd, 1.4s) ease-in-out var(--fdl, 0s) infinite;
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--c) 70%, transparent));
  }
  .micon {
    display: block;
    animation: bob 3s ease-in-out var(--fdl, 0s) infinite;
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--am) 55%, transparent));
  }
  .top-ic.wait :global(svg) {
    opacity: 0.55;
    stroke-dasharray: 3 3;
    animation: breath 2.4s ease-in-out infinite;
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

  .dlab {
    position: absolute;
    width: 110px;
    text-align: center;
    font-size: var(--t-m);
    font-weight: 800;
    color: var(--ink);
    animation: fade 0.4s ease var(--d) both;
  }
  .dlab small {
    display: block;
    font-size: var(--t-xs);
    font-weight: 700;
    margin-top: 1px;
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
  }
  .dlab.td,
  .dlab.td small {
    color: var(--ci);
  }

  .dtag {
    position: absolute;
    translate: -50% 0;
    transform-origin: 50% 100%;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 8px 0 6px;
    border-radius: var(--r-pill);
    font-size: var(--t-xs);
    font-weight: 800;
    white-space: nowrap;
    z-index: 5;
    background: var(--am-soft);
    color: var(--amk);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--am) 55%, transparent);
    animation: pop 0.45s cubic-bezier(0.3, 1.6, 0.5, 1) var(--d) both;
  }
  .dtag.r {
    background: var(--bad-soft);
    color: var(--badk);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--bad) 55%, transparent);
  }
  .dtag.c {
    background: color-mix(in srgb, var(--c) 14%, var(--tint-base));
    color: var(--ci);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 45%, transparent);
  }

  :global(.velas-stage .fdot) {
    position: absolute;
    left: 0;
    top: 0;
    z-index: 15;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: var(--am);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--am) 30%, transparent), 0 0 12px var(--am);
    pointer-events: none;
  }
  :global(.velas-stage .spark) {
    position: absolute;
    z-index: 30;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    pointer-events: none;
  }

  @keyframes grow {
    from { height: 0; }
  }
  @keyframes fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  @keyframes pop {
    from { opacity: 0; transform: scale(0.6); }
    to { opacity: 1; transform: scale(1); }
  }
  @keyframes ign {
    0% { opacity: 0; transform: translateX(-50%) scale(0); }
    100% { opacity: 1; transform: translateX(-50%) scale(1); }
  }
  @keyframes flick2 {
    0%, 100% { transform: scale(1, 1) rotate(0); }
    25% { transform: scale(0.94, 1.08) rotate(-4deg); }
    50% { transform: scale(1.05, 0.95) rotate(2deg); }
    75% { transform: scale(0.97, 1.05) rotate(4deg); }
  }
  @keyframes pulse {
    0%, 100% { box-shadow: inset 0 0 0 2.5px var(--c), 0 0 0 0 color-mix(in srgb, var(--c) 50%, transparent); }
    50% { box-shadow: inset 0 0 0 2.5px var(--c), 0 0 0 7px color-mix(in srgb, var(--c) 0%, transparent); }
  }
  @keyframes bob {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-4px); }
  }
  @keyframes breath {
    0%, 100% { opacity: 0.35; transform: scale(0.94); }
    50% { opacity: 0.7; transform: scale(1.04); }
  }
  @keyframes embr {
    0% { opacity: 0; transform: translate(0, 0) scale(1); }
    20% { opacity: 1; }
    100% { opacity: 0; transform: translate(var(--ex), -60px) scale(0.3); }
  }

  @media (prefers-reduced-motion: reduce) {
    .tube > i, .top-ic.on, .dlab, .dtag, .dmeta {
      animation: none !important;
      transition: none !important;
    }
    .tube.td, .cflame, .micon, .top-ic.wait :global(svg), .emb {
      animation: none !important;
    }
  }
</style>
