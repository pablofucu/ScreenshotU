<script lang="ts">
  // Estadísticas › Días y rachas: racha de jugar (llama) y racha de meta (anillo y lunas de días libres) arriba;
  // debajo, el mapa del año con una sola escala (acento vivo o el color de la categoría filtrada) y el día enfocado.
  import { app } from '../../lib/store.svelte';
  import { FREE_MAX, GOAL_COL, goalHistOf, goalMin, goalTrack, localDay, type GoalKey } from '../../lib/readgoals';
  import { addDays, dayPlays, fmtHM, fmtLongDay, playStreak } from '../../lib/stats';
  import { catTone, toneStyle } from '../../lib/chiptone';
  import { countUp } from '../../lib/countup';
  import IconoRacha from '../../components/racha/IconoRacha.svelte';
  import CIcon from '../../components/CIcon.svelte';

  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const now = $derived(app.now.getTime());
  const H = $derived(app.heat);
  const cat = $derived(app.daysCat);
  const plat = (pi: number) => app.lib.platforms[pi];
  const days = (n: number) => (n === 1 ? 'día' : 'días');
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}`; };
  const span = (a: number, b: number) => {
    const x = new Date(a), y = new Date(b);
    if (a === b) return dm(a);
    const yr = y.getFullYear() !== new Date(now).getFullYear() ? ` ${y.getFullYear()}` : '';
    return x.getMonth() === y.getMonth() ? `${x.getDate()}–${dm(b)}${yr}` : `${dm(a)} – ${dm(b)}${yr}`;
  };

  /* Racha de jugar (de la categoría filtrada, si la hay) */
  const P = $derived(playStreak(app.lib, now, cat >= 0 ? cat : undefined));

  /* Racha de meta: la de la categoría filtrada si tiene meta; si no, la elegida en la pantalla de racha. */
  const gkey = $derived.by((): GoalKey | null => {
    const ks = app.goalKeys;
    const id = cat >= 0 ? plat(cat)?.id : undefined;
    if (id && (ks as string[]).includes(id)) return id as GoalKey;
    return ks.includes(app.streakKey) ? app.streakKey : (ks[0] ?? null);
  });
  const G = $derived(gkey ? goalTrack(gkey, goalMin(app.config, gkey), app.lib.platforms, app.playing, now, 2, goalHistOf(app.config, gkey)) : null);
  const GC = $derived(gkey ? GOAL_COL[gkey] : null);
  const today = $derived(localDay(now));
  const moons = $derived.by(() => {
    if (!G) return [];
    const used = G.free.filter((f) => !f.bonus && f.day >= today - 6).length;
    return Array.from({ length: FREE_MAX }, (_, i) => i < FREE_MAX - Math.min(FREE_MAX, used));
  });
  const goalTxt = (m: number) => (m % 60 === 0 ? `${m / 60} h` : m > 60 ? `${Math.floor(m / 60)} h ${m % 60}` : `${m} min`);

  /* Mapa del año */
  const arr = $derived(cat >= 0 ? (H.by.get(cat) ?? new Array<number>(H.total.length).fill(0)) : H.total);
  const pad = $derived((new Date(H.from).getDay() + 6) % 7);
  const last = $derived.by(() => {
    const t = Math.round((new Date(now).setHours(0, 0, 0, 0) - H.from) / 864e5);
    return Math.min(H.total.length - 1, t);
  });
  const isNow = $derived(H.year === new Date(now).getFullYear());
  // Escala: nada, menos de 30 min, menos de 1 h, menos de 2 h, 2 h o más.
  const lvl = (ms: number) => (ms <= 0 ? 0 : ms < 18e5 ? 1 : ms < 36e5 ? 2 : ms < 72e5 ? 3 : 4);
  const cells = $derived(arr.map((ms, i) => ({ i, l: lvl(ms), fut: i > last, w: Math.floor((pad + i) / 7) })));
  const runFrom = $derived(isNow && P.cur ? last - P.cur + (P.playedToday ? 1 : 0) : Infinity);
  const runTo = $derived(P.playedToday ? last : last - 1);
  const months = $derived(MON.map((t, m) => {
    const i = Math.round((new Date(H.year, m, 1).getTime() - H.from) / 864e5);
    return { t, col: Math.floor((pad + i) / 7) + 1 };
  }));
  const played = $derived(arr.slice(0, last + 1).filter((v) => v > 0).length);
  const totalMs = $derived(arr.reduce((a, b) => a + b, 0));
  const hcStyle = $derived(cat >= 0 && plat(cat) ? `${toneStyle(catTone(plat(cat).id))};--hc:var(--v1)` : '--hc:var(--accv)');

  /* Día enfocado */
  const di = $derived(app.focus.startsWith('dy') ? +app.focus.slice(2) : last);
  const dayMs = $derived(addDays(H.from, Math.max(0, di)));
  const plays = $derived(dayPlays(app.lib, dayMs).filter((p) => cat < 0 || p.pi === cat).sort((a, b) => b.ms - a.ms));
  const dayTot = $derived(arr[di] ?? 0);
  const split = $derived(cat >= 0 ? [] : [...H.by.entries()].map(([pi, a]) => ({ pi, ms: a[di] ?? 0 })).filter((s) => s.ms > 0).sort((a, b) => b.ms - a.ms));
  const SHOW = 4;
  const wave = $derived(`${H.year}:${cat}`);
</script>

<div class="dz">
  <div class="streaks">
    <div class="glass sk" style="{toneStyle(1)}">
      <div class="face">
        <div class="big">
          <div class="lab"><IconoRacha name="flame" size={22} stroke="var(--v1)" />Racha de jugar</div>
          <div class="what">Días seguidos jugando{cat >= 0 ? ` a ${plat(cat)?.name}` : ' o leyendo algo'}, aunque sea un rato.</div>
        </div>
        <div class="num"><span use:countUp={{ to: P.cur, key: `dp:${cat}` }}>{P.cur}</span><small>{days(P.cur)}</small></div>
        <div class="sside">
          Mejor racha<b>{P.best.n} {days(P.best.n)}</b>
          <span>{P.best.n ? span(P.best.from, P.best.to) : 'aún sin racha'}</span>
        </div>
      </div>
    </div>

    <div class="glass sk" style={GC ? `--v1:${GC.c}` : toneStyle(4)}>
      <div class="face">
        {#if G && GC}
          <div class="big">
            <div class="lab"><CIcon name="target" />Racha de meta</div>
            <div class="what">Días seguidos llegando a <b>{goalTxt(G.goal)}</b> en {GC.full}. Una luna salva un día flojo.</div>
            <div class="today">
              <span class="bar"><i style="width:{Math.round(G.frac * 100)}%"></i></span>
              <span class="tt">{G.done ? 'Hoy, hecho' : `Hoy ${Math.round(G.today) < 60 ? `${Math.round(G.today)} min` : goalTxt(Math.round(G.today))} de ${goalTxt(G.goal)}`}</span>
            </div>
          </div>
          <div class="num"><span use:countUp={{ to: G.streak, key: `dg:${gkey}` }}>{G.streak}</span><small>{days(G.streak)}</small></div>
          <div class="sside">
            Mejor racha<b>{G.best} {days(G.best)}</b>
            <div class="moons" title="Días libres: {FREE_MAX} cada 7 días">
              {#each moons as on, k (k)}
                <span class="moon" class:off={!on} style="--d:{0.25 + k * 0.08}s">
                  <IconoRacha name="moon" size={20} stroke={on ? 'var(--amk)' : 'var(--ink-3)'} fill={on ? 'color-mix(in srgb, var(--am) 45%, transparent)' : 'none'} />
                </span>
              {/each}
            </div>
          </div>
        {:else}
          <div class="big">
            <div class="lab"><CIcon name="target" />Racha de meta</div>
            <div class="what">Sin meta diaria. Ponla en Ajustes y aquí verás los días seguidos que la cumples.</div>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <div class="glass hm" style={hcStyle}>
    <div class="face">
      <div class="hm-head">
        <div class="t">Tus días de {H.year}<small><span use:countUp={{ to: played, key: `dd:${wave}` }}>{played}</span> {played === 1 ? 'día jugado' : 'días jugados'} de {last + 1} · {fmtHM(totalMs)}</small></div>
        <div class="leg">
          {#if cat >= 0}<span class="ccat">{plat(cat)?.name}</span>{/if}
          Menos {#each [0, 1, 2, 3, 4] as l}<i class="l{l}"></i>{/each} Más
        </div>
      </div>
      <div class="hm-grid">
        <div class="hm-months">{#each months as m}<span style="grid-column:{m.col}">{m.t}</span>{/each}</div>
        <div class="hm-days"><span>lun</span><span></span><span>mié</span><span></span><span>vie</span><span></span><span>dom</span></div>
        {#key wave}
          <div class="hm-cells">
            {#each { length: pad } as _, k (k)}<i class="pad"></i>{/each}
            {#each cells as c (c.i)}
              {#if c.fut}
                <i class="fut"></i>
              {:else}
                <i class="l{c.l}" class:fx={true} class:f={app.focus === 'dy' + c.i} class:today={isNow && c.i === last}
                  class:run={c.i >= runFrom && c.i <= runTo} data-k="dy{c.i}" data-act="dy" style="--w:{c.w}"></i>
              {/if}
            {/each}
          </div>
        {/key}
      </div>
      <div class="day">
        <div class="dt">
          {fmtLongDay(dayMs, now)}
          <small>{dayTot ? `${fmtHM(dayTot)} jugadas` : 'Sin partidas'}</small>
          {#if split.length > 1}
            <div class="split">{#each split as s (s.pi)}<i style="flex:{s.ms};{toneStyle(catTone(plat(s.pi).id))}" title={plat(s.pi).name}></i>{/each}</div>
          {/if}
        </div>
        <div class="ses">
          {#each plays.slice(0, SHOW) as p (p.t.id + p.start)}
            <span style={toneStyle(catTone(plat(p.pi).id))}><i class="dot"></i>{p.t.name}<b>{fmtHM(p.ms)}</b></span>
          {:else}
            <span class="none">Ese día no jugaste{cat >= 0 ? ` a ${plat(cat)?.name}` : ''}.</span>
          {/each}
          {#if plays.length > SHOW}<span class="none">y {plays.length - SHOW} más</span>{/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .dz{position:absolute;left:var(--s-6);right:var(--s-6);top:132px;bottom:64px;display:flex;flex-direction:column;gap:var(--s-4)}
  .dz .glass{position:relative;cursor:default;animation:rise var(--d-3) var(--e-bounce) both}
  .dz .glass:nth-child(2){animation-delay:60ms}
  .dz .hm{animation-delay:120ms}

  .streaks{display:grid;grid-template-columns:1fr 1fr;gap:var(--s-4)}
  .sk{--r:24px;--bw:8px;height:128px}
  .sk .face{display:flex;align-items:center;gap:var(--s-4);padding:0 var(--s-5)}
  .sk .big{display:flex;flex-direction:column;gap:var(--s-1);flex:1;min-width:0}
  .sk .lab{display:flex;align-items:center;gap:var(--s-2);font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .sk .lab :global(svg){width:22px;height:22px;color:var(--v1)}
  .sk .what{font:500 var(--t-s)/1.3 var(--round);color:var(--ink-2)}
  .sk .what b{color:var(--ink)}
  .sk .num{font:800 var(--t-3xl)/1 var(--round);color:var(--ink);font-variant-numeric:tabular-nums;white-space:nowrap}
  .sk .num small{font-size:var(--t-l);color:var(--ink-2);font-weight:700;margin-left:4px}
  .sk .sside{display:flex;flex-direction:column;gap:var(--s-1);align-items:flex-end;font:500 var(--t-s)/1.2 var(--round);color:var(--ink-2);white-space:nowrap}
  .sk .sside b{font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .today{display:flex;align-items:center;gap:var(--s-2);margin-top:2px}
  .today .bar{flex:1;max-width:160px;height:8px;border-radius:4px;background:color-mix(in srgb,var(--v1) 18%,transparent);overflow:hidden}
  .today .bar i{display:block;height:100%;border-radius:inherit;background:var(--v1);transition:width var(--d-3) var(--e-soft)}
  .today .tt{font:700 var(--t-s)/1 var(--round);color:var(--ink);white-space:nowrap}
  .moons{display:flex;gap:6px;margin-top:2px}
  .moon{display:grid;place-items:center;width:30px;height:30px;border-radius:50%;background:color-mix(in srgb,var(--am) 16%,transparent);animation:pop .35s var(--e-bounce) var(--d) both}
  .moon.off{background:transparent;box-shadow:inset 0 0 0 1.5px var(--line)}

  .hm{--r:24px;--bw:8px;flex:1;min-height:0}
  .hm .face{padding:var(--s-3) var(--s-5);display:flex;flex-direction:column}
  .hm-head{display:flex;align-items:center;justify-content:space-between;margin-bottom:var(--s-2)}
  .hm-head .t{font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .hm-head .t small{font:500 var(--t-s)/1.3 var(--round);color:var(--ink-2);margin-left:var(--s-2)}
  .leg{display:flex;gap:4px;align-items:center;font:700 var(--t-xs)/1 var(--round);color:var(--ink-2)}
  .leg i{width:12px;height:12px;border-radius:3px}
  .leg .ccat{margin-right:var(--s-2);padding:4px 10px;border-radius:var(--r-pill);background:color-mix(in srgb,var(--hc) 18%,transparent);color:var(--ink)}
  .hm-grid{display:grid;grid-template-columns:30px 1fr;column-gap:6px}
  .hm-months{grid-column:2;display:grid;grid-template-columns:repeat(54,1fr);font:700 var(--t-xs)/1 var(--round);color:var(--ink-2);height:14px;margin-bottom:4px}
  .hm-days{display:grid;grid-template-rows:repeat(7,22px);gap:4px;font:700 11px/22px var(--round);color:var(--ink-3)}
  .hm-cells{display:grid;grid-template-rows:repeat(7,22px);grid-auto-flow:column;grid-template-columns:repeat(54,1fr);gap:4px}
  .hm-cells i{border-radius:4px;position:relative;transition:transform var(--d-1) var(--e-soft);animation:pop .32s var(--e-bounce) calc(var(--w,0) * 9ms + 120ms) both}
  .hm-cells i.pad{visibility:hidden;animation:none}
  .hm-cells i.fut{box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--line) 70%,transparent);animation:none}
  .l0{background:color-mix(in srgb,var(--ink-3) 14%,transparent)}
  .l1{background:color-mix(in srgb,var(--hc) 28%,transparent)}
  .l2{background:color-mix(in srgb,var(--hc) 52%,transparent)}
  .l3{background:color-mix(in srgb,var(--hc) 78%,transparent)}
  .l4{background:var(--hc);box-shadow:0 0 8px color-mix(in srgb,var(--hc) 60%,transparent)}
  .hm-cells i.run{outline:2px solid color-mix(in srgb,var(--arc-naranja) 80%,transparent);outline-offset:-1px}
  .hm-cells i.today::after{content:'';position:absolute;left:50%;top:50%;width:6px;height:6px;margin:-3px 0 0 -3px;border-radius:50%;background:var(--ink)}
  .hm-cells i.f{transform:scale(1.45);z-index:2;box-shadow:var(--ring)}

  .day{margin-top:auto;display:grid;grid-template-columns:300px 1fr;gap:var(--s-4);align-items:center;border-top:1px solid var(--line);padding-top:var(--s-3)}
  .day .dt{font:800 var(--t-l)/1.2 var(--round);color:var(--ink)}
  .day .dt::first-letter{text-transform:uppercase}
  .day .dt small{display:block;font:500 var(--t-s)/1.3 var(--round);color:var(--ink-2)}
  .split{display:flex;gap:2px;height:8px;border-radius:4px;overflow:hidden;margin-top:6px;width:220px}
  .split i{background:var(--v1)}
  .ses{display:flex;gap:var(--s-2);overflow:hidden}
  .ses span{display:inline-flex;align-items:center;gap:var(--s-2);height:30px;padding:0 var(--s-3);border-radius:var(--r-pill);
    background:color-mix(in srgb,var(--v1) 16%,transparent);font:700 var(--t-s)/1 var(--round);color:var(--ink);max-width:280px;flex-shrink:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .ses span b{color:var(--ink-2);font-weight:700}
  .ses .dot{width:8px;height:8px;border-radius:50%;background:var(--v1);flex-shrink:0}
  .ses .none{background:transparent;color:var(--ink-2);font-weight:600}

  @keyframes rise{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:none}}
  @keyframes pop{from{opacity:0;transform:scale(.4)}to{opacity:1;transform:none}}
</style>
