<script lang="ts">
  // Estadísticas › Panorama: la rejilla de chips de UNA categoría (LT/RT la cambia) en el periodo elegido (LB/RB).
  // Todas las cifras salen solo de los títulos de esa categoría; nunca se mezclan con otras.
  import { app } from '../lib/store.svelte';
  import { addDays, catMilestone, catPeriod, catShots, catWeeks, dayHeat, dayStart, fmtHM, hourHistogram, periodRange, type Range } from '../lib/stats';
  import { GOAL_KEYS, goalHistOf, goalMin, goalTrack, type GoalKey } from '../lib/readgoals';
  import { catTone } from '../lib/chiptone';
  import { countUp } from '../lib/countup';
  import ChipPane from '../components/ChipPane.svelte';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import CoverBg from '../components/CoverBg.svelte';
  import ThumbImg from '../components/ThumbImg.svelte';

  let { pi }: { pi: number } = $props();

  const WD = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const FAN = [['-14deg', '4px'], ['14deg', '4px'], ['0deg', '-6px']];

  const plat = $derived(app.lib.platforms[pi]);
  const tone = $derived(plat ? catTone(plat.id) : 4);
  const now = $derived(app.now.getTime());
  const range = $derived.by((): Range => {
    const p = app.panoPer;
    if (p === 'all') return periodRange('all', 0, now);
    if (p === 'd30') { const to = addDays(dayStart(now), 1); return { from: addDays(to, -30), to, label: 'Últimos 30 días' }; }
    return periodRange('year', p - app.now.getFullYear(), now);
  });
  const perTxt = $derived(app.panoPer === 'all' ? 'desde siempre' : app.panoPer === 'd30' ? 'en los últimos 30 días' : `en ${app.panoPer}`);
  const pid = $derived(`pn:${pi}:${app.panoPer}`);

  const P = $derived(catPeriod(app.lib, pi, range));
  const W = $derived(catWeeks(app.lib, pi, 8, now));
  const M = $derived(catMilestone(app.lib, pi));
  const S = $derived(catShots(app.lib, pi, range, app.favs));
  const hours = $derived(hourHistogram(app.lib, pi, range));
  const heatYear = $derived(typeof app.panoPer === 'number' ? app.panoPer : app.now.getFullYear());
  const heat = $derived(dayHeat(app.lib, pi, heatYear));

  /* B · meta de hoy, solo si la categoría tiene meta diaria */
  const gk = $derived(plat && (GOAL_KEYS as string[]).includes(plat.id) && goalMin(app.config, plat.id as GoalKey) > 0 ? (plat.id as GoalKey) : null);
  const G = $derived(gk ? goalTrack(gk, goalMin(app.config, gk), app.lib.platforms, app.playing, now, undefined, goalHistOf(app.config, gk)) : null);

  /* Claves en orden de lectura; B solo existe con meta */
  const ids = $derived(['A', ...(G ? ['B'] : []), 'C', 'D', 'L', 'H', 'K', 'J', 'I']);
  const key = (id: string) => `pn${ids.indexOf(id)}`;

  /* A · odómetro de horas */
  const totH = $derived(Math.round(P.ms / 3.6e6));
  const digits = $derived([...String(totH)]);

  /* D · esta semana frente a la anterior y chispa de 8 semanas */
  const wkMs = $derived(W.weeks[W.weeks.length - 1] ?? 0);
  const prevMs = $derived(W.weeks[W.weeks.length - 2] ?? 0);
  const pct = $derived(prevMs > 0 ? Math.round(((wkMs - prevMs) / prevMs) * 100) : null);
  const spark = $derived.by(() => {
    const mx = Math.max(1, ...W.weeks), n = W.weeks.length;
    const pts = W.weeks.map((v, i) => [4 + (248 * i) / Math.max(1, n - 1), 76 - (64 * v) / mx]);
    const ln = pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`).join(' ');
    return { ln, ar: `${ln} L252 84 L4 84Z`, end: pts[pts.length - 1] ?? [252, 76] };
  });
  const parts = (ms: number) => { const m = Math.round(ms / 60000); return m < 60 ? { v: m, u: 'min' } : { v: Math.floor(m / 60), u: 'h' }; };

  /* L · partida más larga */
  const L = $derived(P.longest);
  const lh = $derived(L ? Math.floor(L.ms / 3.6e6) : 0), lm = $derived(L ? Math.round((L.ms % 3.6e6) / 60000) : 0);
  const dayTxt = (ms: number) => { const d = new Date(ms); return `${WD[d.getDay()]} ${d.getDate()} ${MON[d.getMonth()]}${d.getFullYear() !== app.now.getFullYear() ? ' ' + d.getFullYear() : ''}`; };

  /* J · reloj de 24 h: la ventana de 3 h con más juego */
  const clock = $derived.by(() => {
    const mx = Math.max(...hours);
    if (!(mx > 0)) return null;
    let best = 0, bs = -1;
    for (let h = 0; h < 24; h++) { const s = hours[h] + hours[(h + 1) % 24] + hours[(h + 2) % 24]; if (s > bs) { bs = s; best = h; } }
    const peak = hours.indexOf(mx), cx = 61, R1 = 34, R2 = 54;
    const p = (r: number, a: number) => `${(cx + r * Math.cos(a)).toFixed(1)} ${(cx + r * Math.sin(a)).toFixed(1)}`;
    const segs = hours.map((v, i) => {
      const a0 = (i / 24) * 2 * Math.PI - Math.PI / 2 + 0.02, a1 = ((i + 1) / 24) * 2 * Math.PI - Math.PI / 2 - 0.02;
      const r2 = R1 + (R2 - R1) * Math.max(0.12, v / mx);
      const on = (i - best + 24) % 24 < 3;
      return { d: `M${p(R1, a0)} L${p(r2, a0)} A${r2} ${r2} 0 0 1 ${p(r2, a1)} L${p(R1, a1)} A${R1} ${R1} 0 0 0 ${p(R1, a0)}Z`, on };
    });
    const lbl = ['0', '6', '12', '18'].map((t, k) => { const a = (k * Math.PI) / 2 - Math.PI / 2; return { t, x: cx + 21 * Math.cos(a), y: cx + 21 * Math.sin(a) + 4 }; });
    return { segs, lbl, peak, from: best, to: (best + 3) % 24 || 24, rot: peak * 15 + 7.5 };
  });

  /* I · el año en días: semanas en columnas, de lunes a domingo */
  const heatCells = $derived.by(() => {
    const pad = (new Date(heatYear, 0, 1).getDay() + 6) % 7;
    const mx = Math.max(1, ...heat);
    const cells: { l: number; i: number; e?: boolean }[] = Array.from({ length: pad }, (_, i) => ({ l: 0, i: 0, e: true }));
    heat.forEach((v, d) => cells.push({ l: v > 0 ? Math.max(0.15, Math.sqrt(v / mx)) : 0, i: Math.floor((d + pad) / 7) }));
    return { cells, cols: Math.ceil(cells.length / 7), days: heat.filter((v) => v > 0).length };
  });
</script>

<ChipPane>
  {#if !plat}
    <div class="stnone fx" class:f={app.focus === 'pn0'} data-k="pn0">Aún no hay partidas en ninguna categoría.</div>
  {:else}
   <div class="pn">
    <h3 class="sh"><CIcon name="clock" />Lo esencial<span>solo títulos de {plat.name}, {perTxt}</span></h3>
    <div class="cgrid">
      <StatChip k={key('A')} icon="clock" label="Tiempo jugado" {tone}>
        <div class="odo2" aria-label="{totH} h">
          {#each digits as d, i (pid + ':' + digits.length + ':' + i)}
            {#if i && (digits.length - i) % 3 === 0}<span class="x-sep"></span>{/if}
            <span class="x-c"><b style="--n:{d};--i:{i}">{#each Array(20) as _, j}<span>{j % 10}</span>{/each}</b></span>
          {/each}
          <span class="x-u">h</span>
        </div>
        <div class="x-s" style="margin-top:auto"><b>+{fmtHM(wkMs)}</b> esta semana · {P.games} {P.games === 1 ? 'juego' : 'juegos'}</div>
      </StatChip>

      {#if G}
        <StatChip k={key('B')} icon="target" label="Meta de hoy" {tone}>
          <div class="ringw">
            <svg class="x-ring" viewBox="0 0 120 120" style="--len:302;--p:{Math.min(1, G.frac)}">
              <circle cx="60" cy="60" r="48" fill="none" stroke="var(--glass-in)" stroke-width="14" />
              <circle cx="60" cy="60" r="48" fill="none" stroke="var(--line)" stroke-width="1" />
              <circle class="arc" cx="60" cy="60" r="48" fill="none" stroke="url(#pnGB)" stroke-width="14" stroke-linecap="round" transform="rotate(-90 60 60)" />
              <g class="tip"><circle cx="60" cy="12" r="9" fill="var(--tint-base)" stroke="var(--v1)" stroke-width="3" /></g>
              <defs><linearGradient id="pnGB" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="var(--v1)" /><stop offset="1" stop-color="var(--v2)" /></linearGradient></defs>
              <text x="60" y="64" text-anchor="middle" font-weight="800" font-size="28" fill="var(--ink)">{Math.round(G.today)}</text>
              <text x="60" y="82" text-anchor="middle" font-weight="700" font-size="12" fill="var(--ink-2)">de {G.goal} min</text>
            </svg>
            <div class="x-s">
              {#if G.done}<b>Meta cumplida</b> hoy.{:else}Te faltan <b>{Math.max(1, Math.ceil(G.goal - G.today))} min</b> para cerrar el día.{/if}<br />
              {G.streak ? `Racha de ${G.streak} ${G.streak === 1 ? 'día' : 'días'}.` : 'Sin racha todavía.'}
            </div>
          </div>
        </StatChip>
      {/if}

      <StatChip k={key('C')} icon="cal" label="Días con actividad" {tone}>
        <svg class="gauge" viewBox="0 0 260 116" style="--len:283;--p:{W.days / 7}">
          <path d="M40 104a90 90 0 0 1 180 0" fill="none" stroke="var(--glass-in)" stroke-width="18" stroke-linecap="round" />
          <path class="arc" d="M40 104a90 90 0 0 1 180 0" fill="none" stroke="url(#pnGC)" stroke-width="18" stroke-linecap="round" />
          <defs><linearGradient id="pnGC"><stop offset="0" stop-color="var(--v1)" /><stop offset="1" stop-color="var(--v2)" /></linearGradient></defs>
          <g class="ndl"><path d="M130 104L127 44h6z" fill="var(--ink)" /></g>
          <circle cx="130" cy="104" r="9" fill="var(--ink)" /><circle cx="130" cy="104" r="4" fill="var(--tint-base)" />
          <text x="12" y="112" font-size="11" font-weight="800" fill="var(--ink-2)">0</text>
          <text x="240" y="112" font-size="11" font-weight="800" fill="var(--ink-2)">7</text>
        </svg>
        <div class="x-s" style="text-align:center;margin-top:-6px"><b style="font-size:var(--t-xl)">{W.days} de 7</b> días esta semana</div>
      </StatChip>

      <StatChip k={key('D')} icon="trend" label="Esta semana" {tone}>
        {@const w = parts(wkMs)}
        <div class="vrow">
          <div class="x-v"><span use:countUp={{ to: w.v, key: `${pid}:D` }}>{w.v}</span><small>{w.u}</small></div>
          {#if pct !== null}<span class="delta" class:dn={pct < 0}><CIcon name="up" class="" />{Math.abs(pct)} %</span>{/if}
        </div>
        <svg class="spark" viewBox="0 0 260 84" preserveAspectRatio="none">
          <defs><linearGradient id="pnGD" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--v1)" stop-opacity=".35" /><stop offset="1" stop-color="var(--v1)" stop-opacity="0" /></linearGradient></defs>
          <path class="ar" d={spark.ar} fill="url(#pnGD)" />
          <path class="x-ln" d={spark.ln} fill="none" stroke="var(--v1)" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" />
          <circle class="pt" cx={spark.end[0]} cy={spark.end[1]} r="6" fill="var(--v1)" stroke="var(--tint-base)" stroke-width="3" />
        </svg>
      </StatChip>
    </div>

    <h3 class="sh"><CIcon name="star" />Récords e hitos<span>lo que más destaca</span></h3>
    <div class="cgrid">
      <StatChip k={key('L')} icon="watch" label="Partida más larga" {tone} act={L ? `sr:${L.pi}:${L.ti}` : undefined}>
        {#if L}
          <div class="rec">
            <svg class="watch" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="13" r="8.5" class="tint" stroke="none" /><circle cx="12" cy="13" r="8.5" /><path d="M10 2h4M12 2v2.5M19 6l1.5-1.5" /><path class="x-sw" d="M12 13V8" /><circle cx="12" cy="13" r="1.2" fill="currentColor" /></svg>
            <div class="x-v">{#if lh}<span use:countUp={{ to: lh, key: `${pid}:Lh` }}>{lh}</span><small>h</small> {/if}<span use:countUp={{ to: lm, key: `${pid}:Lm` }}>{lm}</span><small>min</small></div>
          </div>
          <div class="who"><CoverBg t={L.t} cls="pcv" /><div class="x-s"><b>{L.t.name}</b><br />{dayTxt(L.start)}</div></div>
        {:else}
          <div class="x-s">Sin partidas {perTxt}.</div>
        {/if}
      </StatChip>

      <StatChip k={key('H')} icon="flag" label="Próximo hito" {tone} act={M ? `sr:${M.pi}:${M.ti}` : undefined}>
        {#if M}
          <div class="x-v"><span use:countUp={{ to: Math.floor(M.cur), key: `${pid}:H` }}>{Math.floor(M.cur)}</span><small>h</small></div>
          <div class="x-s">A <b>{fmtHM(M.left * 3.6e6)}</b> de las {M.h} h en <b>{M.t.name}</b></div>
          <div class="mile" style="--w:{Math.min(100, (M.cur / M.h) * 100)}%">
            <div class="tr"></div><div class="fl"></div>
            <svg class="flag" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 22V3" /><path d="M5 3h12l-2.5 4.5L17 12H5" fill="color-mix(in srgb,currentColor 25%,transparent)" /></svg>
            <span class="x-tk" style="left:0;transform:none">0</span><span class="x-tk" style="left:50%">{M.h / 2} h</span><span class="x-tk" style="right:0;left:auto;transform:none">{M.h} h</span>
          </div>
        {:else}
          <div class="x-s">Todos los títulos han pasado ya el último hito.</div>
        {/if}
      </StatChip>

      <StatChip k={key('K')} icon="cam" label="Capturas" {tone}>
        <div class="x-v"><span use:countUp={{ to: S.count, key: `${pid}:K` }}>{S.count}</span><small>{S.count === 1 ? 'captura' : 'capturas'}</small></div>
        <div class="fan">
          {#each S.pick as s, i (s.path)}<ThumbImg path={s.path} style="--a:{FAN[i][0]};--y:{FAN[i][1]};--i:{i}" />{/each}
          {#if S.pick.some((s) => s.fav)}<svg class="x-star" viewBox="0 0 24 24" fill="currentColor" stroke="var(--tint-base)" stroke-width="1.5" stroke-linejoin="round"><path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z" /></svg>{/if}
        </div>
      </StatChip>

      <StatChip k={key('J')} icon="moon" label="Hora favorita" {tone}>
        {#if clock}
          <div class="clockw">
            <svg class="x-clock" viewBox="0 0 122 122" aria-hidden="true">
              {#each clock.segs as s, i}<path class="seg" style="--i:{i}" d={s.d} fill={s.on ? 'var(--v1)' : 'color-mix(in srgb,var(--ink-3) 40%,transparent)'} />{/each}
              <circle cx="61" cy="61" r="28" fill="var(--glass-in)" stroke="var(--line)" />
              <g class="hand" style="transform:rotate({clock.rot}deg)"><path d="M61 61V38" stroke="var(--ink)" stroke-width="3" stroke-linecap="round" /></g>
              <circle cx="61" cy="61" r="4" fill="var(--ink)" />
              {#each clock.lbl as l}<text x={l.x} y={l.y} text-anchor="middle" font-size="9" font-weight="800" fill="var(--ink-2)">{l.t}</text>{/each}
            </svg>
            <div><div class="x-v" style="font-size:var(--t-2xl)">{clock.peak} h</div><div class="x-s">Juegas sobre todo<br />de <b>{clock.from} a {clock.to} h</b></div></div>
          </div>
        {:else}
          <div class="x-s">Sin partidas {perTxt}.</div>
        {/if}
      </StatChip>

      <StatChip k={key('I')} icon="grid" label="{heatYear}, día a día · {heatCells.days} con partidas · más color, más horas" cls="full" {tone}>
        <div class="x-heat" style="grid-template-columns:repeat({heatCells.cols},1fr)">
          {#each heatCells.cells as c}<i class:e={c.e} style="--l:{c.l};--i:{c.i}"></i>{/each}
        </div>
      </StatChip>
    </div>
   </div>
  {/if}
</ChipPane>

<style>
  /* Todo en una pantalla (1280×720): cabeceras justas, chips de 140 px y el calendario de calor en una franja. */
  .pn :global(.sh){margin:12px 0 8px}
  .pn :global(.sh:first-child){margin-top:0}
  .pn :global(.st.st){height:140px}
  .pn :global(.st .face){padding:10px 14px;gap:6px}
  .pn :global(.st.st.full){height:110px}
  .pn :global(.x-heat){height:auto;flex:1;min-height:0}
  .pn :global(.x-ring){width:92px;height:92px}
  .pn :global(.x-clock){width:92px;height:92px}
  .pn :global(.gauge){height:80px}
  .pn :global(.spark){height:56px}
  .pn :global(.watch){width:48px;height:48px}
  .pn :global(.fan){position:absolute;right:28px;top:12px;width:110px;height:64px;margin:0}
  .pn :global(.st .fan img){width:84px;height:50px}
  .pn :global(.st:has(.fan) .x-v small){display:none}
  .pn :global(.mile){height:22px;flex:none}
  .pn :global(.mile .tr),.pn :global(.mile .fl){top:6px}
  .pn :global(.mile .x-tk){display:none}
  .pn :global(.mile .flag){top:-16px}
  :global(.cpane .st.full) { grid-column: 1 / -1; }
  :global(.st .who .pcv) { width: 44px; height: 28px; flex: none; border-radius: var(--r-xs); background: var(--glass-in) center / cover no-repeat; box-shadow: 0 0 0 1px var(--line-2); }
  :global(.x-heat i.e) { visibility: hidden; }
</style>
