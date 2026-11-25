<script lang="ts">
  // Estadísticas › Semana: riel de categorías a la izquierda (LB/RB o clic) y, a la derecha, el resumen de la categoría
  // elegida: total con la racha, barras por día, lo más jugado y las metas como siete cuadritos. Nunca mezcla categorías:
  // «Todas» solo apila cada una con su color.
  import { onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_KEYS, goalMin, type GoalKey } from '../lib/readgoals';
  import { fmtHM, weekReview, type WeekBlock } from '../lib/stats';
  import { statsCats } from '../lib/controller.svelte';
  import { catColor, catIcon, catTone } from '../lib/chiptone';
  import { IS_TAURI, backend } from '../lib/backend';
  import type { WeekAch } from '../lib/types';
  import type { ReadStats } from '../lib/novel-types';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import IconoRacha from '../components/racha/IconoRacha.svelte';

  const DIAS = 'LMXJVSD';
  /** Dos horas en un día = cuadrito lleno. */
  const FULL_MS = 2 * 3600000;
  const goals = $derived(Object.fromEntries(GOAL_KEYS.filter((k) => goalMin(app.config, k) > 0).map((k) => [k, goalMin(app.config, k)])) as Partial<Record<GoalKey, number>>);
  const W = $derived(weekReview(app.lib, app.weekOff, app.favs, app.playing, goals, app.now.getTime()));
  const plat = (pi: number) => app.lib.platforms[pi];
  const parts = (ms: number) => { const m = Math.round(ms / 60000); return m < 60 ? { v: m, u: 'min' } : { v: Math.floor(m / 60), u: `h ${String(m % 60).padStart(2, '0')}` }; };
  const byPi = $derived(new Map(W.blocks.map((b) => [b.pi, b])));

  /* Riel: Todas y cada categoría de la biblioteca, en el mismo orden que LB/RB */
  const rail = $derived(statsCats().map((pi) => {
    if (pi < 0) return { pi, name: 'Todas', icon: 'grid', col: 'var(--acc)', ms: W.ms };
    const p = plat(pi);
    return { pi, name: p.name, icon: catIcon(p.id), col: catColor(p.id), ms: byPi.get(pi)?.ms ?? 0 };
  }));
  const sel = $derived(app.statsCat);
  const blocks = $derived<WeekBlock[]>(sel < 0 ? W.blocks : (byPi.get(sel) ? [byPi.get(sel)!] : []));
  const selCol = $derived(sel < 0 ? 'var(--acc)' : catColor(plat(sel)?.id ?? ''));
  const selName = $derived(sel < 0 ? 'Todas las categorías' : plat(sel)?.name ?? '');

  /* Total, diferencia con la semana anterior y días activos */
  const ms = $derived(sel < 0 ? W.ms : blocks[0]?.ms ?? 0);
  const prevMs = $derived(sel < 0 ? W.prevMs : blocks[0]?.prevMs ?? 0);
  const tot = $derived(parts(ms));
  const diff = $derived(ms - prevMs);
  const perDay = $derived(Array.from({ length: 7 }, (_, i) => blocks.reduce((a, b) => a + b.perDay[i], 0)));
  const activeN = $derived(perDay.filter((v) => v > 0).length);

  /* Racha: la de la categoría elegida; en «Todas», la más larga de las que tienen meta */
  const streak = $derived.by(() => {
    let r: { n: number; name: string } | null = null;
    for (const b of blocks) if (b.goal && b.goal.streak > 0 && (!r || b.goal.streak > r.n)) r = { n: b.goal.streak, name: b.name };
    return r;
  });

  /* Barras: cada día apila sus categorías con su color; con una categoría con meta, línea discontinua de la meta */
  const goalMs = $derived(sel >= 0 && blocks[0]?.goal ? (goals[plat(sel).id as GoalKey] ?? 0) * 60000 : 0);
  const barMax = $derived(Math.max(1, goalMs, ...perDay));
  const bars = $derived(Array.from({ length: 7 }, (_, i) => ({
    ms: perDay[i],
    segs: blocks.filter((b) => b.perDay[i] > 0).map((b) => ({ col: catColor(plat(b.pi).id), h: (b.perDay[i] / barMax) * 100 })),
  })));
  const todayI = $derived(app.weekOff === 0 ? (app.now.getDay() + 6) % 7 : -1);

  /* Lo más jugado o leído de la selección */
  const tops = $derived(blocks.flatMap((b) => b.top.map((x) => ({ pi: b.pi, ti: x.ti, ms: x.ms }))).sort((a, b) => b.ms - a.ms).slice(0, 3));
  const kind = $derived(blocks.length && blocks.every((b) => b.kind === 'read') ? 'read' : 'play');

  /* Metas: una fila por categoría con meta (si no hay ninguna, por categoría con tiempo). Cada cuadrito se rellena
     según el tiempo del día (0 vacío, 2 h o más del todo) y los días con la meta cumplida llevan su marca. */
  const hasGoals = $derived(blocks.some((b) => b.goal));
  const rows = $derived((hasGoals ? blocks.filter((b) => b.goal) : blocks.filter((b) => b.ms > 0)).map((b) => ({
    pi: b.pi, name: b.name, icon: catIcon(plat(b.pi).id), col: catColor(plat(b.pi).id), met: b.goal?.met,
    days: b.perDay.map((v, i) => ({ p: Math.min(1, v / FULL_MS), ok: !!b.goal && b.goal.perDay[i] >= 1, ms: v })),
  })));

  /* Logros de la semana, repartidos a su categoría por el juego (los de juegos sin categoría se ignoran) */
  let achs = $state<WeekAch[]>([]);
  $effect(() => {
    const from = W.range.from, to = W.range.to;
    let live = true;
    backend.weekAchievements(from, to).then((l) => { if (live) achs = l; }).catch(() => { if (live) achs = []; });
    return () => { live = false; };
  });
  const achBy = $derived.by(() => {
    const where = new Map<string, number>();
    app.lib.platforms.forEach((p, pi) => p.titles.forEach((t) => { if (t.gameId) where.set(t.gameId, pi); }));
    const by = new Map<number, number>();
    for (const a of achs) { const pi = where.get(a.gameId); if (pi !== undefined) by.set(pi, (by.get(pi) ?? 0) + 1); }
    return by;
  });

  /* Lo destacado de la selección, en una línea de etiquetas */
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const sunday = $derived(W.range.to - 864e5);
  const dm = (t: number) => { const d = new Date(t); return `${d.getDate()} ${MON[d.getMonth()]}`; };
  const tags = $derived.by(() => {
    const out: Array<{ t: string; done?: boolean }> = [];
    const fin = blocks.flatMap((b) => b.finished.map((ti) => plat(b.pi).titles[ti].name));
    if (fin.length) out.push({ t: fin.length === 1 ? `${fin[0]} terminado` : `${fin.length} terminados`, done: true });
    const mil = blocks.flatMap((b) => b.milestones.map((m) => ({ ...m, pi: b.pi }))).sort((a, b) => b.h - a.h)[0];
    if (mil) out.push({ t: `${plat(mil.pi).titles[mil.ti].name} pasa de ${mil.h} h` });
    const ac = blocks.reduce((a, b) => a + (achBy.get(b.pi) ?? 0), 0);
    if (ac) out.push({ t: `${ac} ${ac === 1 ? 'logro' : 'logros'}` });
    const sh = blocks.reduce((a, b) => a + b.shots, 0);
    if (sh) out.push({ t: `${sh} ${sh === 1 ? 'captura' : 'capturas'}` });
    for (const b of blocks) if (b.consoleUntil !== undefined && b.consoleUntil < sunday) out.push({ t: `Datos de Nintendo hasta el ${dm(b.consoleUntil)}` });
    return out.slice(0, 3);
  });

  /* Novelas japonesas: caracteres, velocidad y palabras nuevas de la semana (solo con esa categoría elegida) */
  let rs = $state<ReadStats | null>(null);
  const hasJvn = $derived(app.lib.platforms.some((p) => p.id === 'jvn'));
  onMount(() => {
    if (!IS_TAURI || !hasJvn) return;
    import('../lib/ipc').then(({ invoke }) => invoke<ReadStats>('readstats_get', { gameId: null })).then((s) => (rs = s)).catch(() => {});
  });
  const jp = $derived.by(() => {
    if (!rs || sel < 0 || plat(sel)?.id !== 'jvn') return null;
    const d0 = Math.floor((W.range.from - new Date(W.range.from).getTimezoneOffset() * 60000) / 864e5);
    const wd = rs.days.filter((d) => d.day >= d0 && d.day < d0 + 7);
    const chars = wd.reduce((a, d) => a + d.chars, 0), t = wd.reduce((a, d) => a + d.activeMs, 0);
    return { chars, cph: t > 0 ? Math.round(chars / (t / 3600000)) : 0, fresh: wd.reduce((a, d) => a + d.newKnown, 0) };
  });
  const nf = (n: number) => n.toLocaleString('es-ES');
  const tone = $derived(sel < 0 ? 4 : catTone(plat(sel)?.id ?? ''));
</script>

<!-- `cpane` da a las tarjetas el estilo de los chips de Estadísticas (chips.css); aquí no hay desplazamiento. -->
<div class="sw cpane">
  <nav class="rail">
    {#each rail as r, i (r.pi)}
      <div class="ri" class:on={r.pi === sel} class:zero={r.ms === 0} style="--c:{r.col};--i:{i}" data-hact="cat:{r.pi}">
        <span class="ic"><CIcon name={r.icon} class="" /></span>
        <span class="nm">{r.name}</span>
        <b>{r.ms > 0 ? fmtHM(r.ms) : '—'}</b>
      </div>
    {/each}
  </nav>

  {#key sel}
  <div class="main" style="--c:{selCol}">
    <StatChip k="wk0" cls="hero" {tone}>
      <div class="x-k"><CIcon name="clock" />{selName}</div>
      <div class="tot">{tot.v}<small>{tot.u}</small></div>
      <div class="sub">
        {#if ms > 0 || prevMs > 0}<span class="delta" class:dn={diff < 0}><CIcon name="up" class="" />{fmtHM(Math.abs(diff))} {diff < 0 ? 'menos' : 'más'} que la anterior</span>{/if}
        <span>{activeN} de 7 días</span>
      </div>
      {#if streak}
        <div class="streak"><IconoRacha name="flame" size={18} strokeWidth={2.4} /><b>{streak.n}</b> {streak.n === 1 ? 'día' : 'días'} de racha{#if sel < 0}<small>{streak.name}</small>{/if}</div>
      {/if}
      {#if jp}
        <div class="jpr"><span><b>{nf(jp.chars)}</b> caracteres</span><span><b>{nf(jp.cph)}</b>/h</span><span><b>{nf(jp.fresh)}</b> palabras</span></div>
      {:else if tags.length}
        <div class="tags">{#each tags as g}<span class:d={g.done}>{g.t}</span>{/each}</div>
      {/if}
    </StatChip>

    <StatChip k="wk1" cls="chart" {tone} label="Día a día" icon="cal">
      <div class="bars">
        {#if goalMs}<div class="goal" style="--g:{goalMs / barMax}"><span>meta</span></div>{/if}
        {#each bars as b, i}
          <div class="day" class:today={i === todayI} style="--i:{i}">
            <div class="stk">
              {#each b.segs as s}<i style="--h:{s.h}%;background:{s.col}"></i>{/each}
            </div>
            <span class="v">{b.ms > 0 ? fmtHM(b.ms) : ''}</span>
            <span class="d">{DIAS[i]}</span>
          </div>
        {/each}
      </div>
    </StatChip>

    <StatChip k="wk2" cls="tops" {tone} label={kind === 'read' ? 'Lo más leído' : 'Lo más jugado'} icon={kind === 'read' ? 'book' : 'swap'} act={tops[0] ? `sr:${tops[0].pi}:${tops[0].ti}` : undefined}>
      {#if tops.length}
        {#each tops as r, i (r.pi + ':' + r.ti)}
          {@const t = plat(r.pi).titles[r.ti]}
          <div class="tr" style="--i:{i}"><MiniCover {t} acc={catColor(plat(r.pi).id)} small /><span>{t.name}</span><b>{fmtHM(r.ms)}</b></div>
        {/each}
      {:else}
        <div class="none">Nada esta semana.</div>
      {/if}
    </StatChip>

    <StatChip k="wk3" cls="goals" {tone} label={hasGoals ? 'Metas de la semana' : 'Días jugados'} icon="check">
      {#if rows.length}
        <div class="sq-h"><span></span>{#each DIAS as d}<span>{d}</span>{/each}<span></span></div>
        {#each rows as r, j (r.pi)}
          <div class="sq-r" style="--c:{r.col}">
            <span class="nm"><CIcon name={r.icon} class="" />{r.name}</span>
            {#each r.days as d, i}<i class:ok={d.ok} class:dk={d.p > 0.45} style="--p:{d.p};--i:{i + j * 7}" title={d.ms ? fmtHM(d.ms) : ''}>{#if d.ok}<svg viewBox="0 0 12 12"><path d="M2.6 6.3 5 8.6l4.4-5" /></svg>{/if}</i>{/each}
            <b>{r.met !== undefined ? `${r.met}/7` : ''}</b>
          </div>
        {/each}
        <div class="leg" style="--c:{rows[0].col}"><span>0</span><i style="--p:0"></i><i style="--p:.33"></i><i style="--p:.66"></i><i style="--p:1"></i><span>2 h</span>{#if hasGoals}<i class="ok dk" style="--p:.66"><svg viewBox="0 0 12 12"><path d="M2.6 6.3 5 8.6l4.4-5" /></svg></i><span>meta cumplida</span>{/if}</div>
      {:else}
        <div class="none">Sin partidas esta semana.</div>
      {/if}
    </StatChip>
  </div>
  {/key}
</div>

<style>
  /* Lienzo 1280×720: riel de 232 px y dos filas de tarjetas a su derecha. */
  .sw{position:absolute;left:48px;right:48px;top:144px;bottom:64px;padding:0;transition:none;display:grid;grid-template-columns:232px 1fr;gap:var(--s-4)}
  .rail{display:flex;flex-direction:column;gap:6px;overflow:hidden}
  .ri{display:grid;grid-template-columns:30px 1fr auto;align-items:center;gap:8px;height:46px;padding:0 12px 0 8px;border-radius:14px;
    cursor:pointer;color:var(--ink);background:color-mix(in srgb,var(--glass,#fff) 55%,transparent);border:1px solid transparent;
    transition:background .2s,border-color .2s,transform .25s var(--e-bounce);animation:railIn .4s var(--e-bounce) both;animation-delay:calc(var(--i)*35ms)}
  .ri:hover{transform:translateX(3px)}
  .ri .ic{display:grid;place-items:center;width:30px;height:30px;border-radius:10px;color:#fff;background:var(--c)}
  .ri .ic :global(svg){width:17px;height:17px}
  .ri .nm{font:700 var(--t-s)/1.1 var(--round);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .ri b{font:800 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .ri.zero{opacity:.6}
  .ri.on{opacity:1;background:color-mix(in srgb,var(--c) 16%,var(--glass,#fff));border-color:color-mix(in srgb,var(--c) 55%,transparent);transform:translateX(6px)}
  .ri.on b{color:var(--ink)}
  @keyframes railIn{from{opacity:0;transform:translateX(-14px)}}

  .main{display:grid;grid-template-columns:300px 1fr;grid-template-rows:1fr 1fr;gap:var(--s-4);min-height:0}
  .main :global(.st.st){position:relative;height:auto;min-height:0;animation:cardIn .42s var(--e-bounce) both}
  .main :global(.st:nth-child(2)){animation-delay:50ms}
  .main :global(.st:nth-child(3)){animation-delay:100ms}
  .main :global(.st:nth-child(4)){animation-delay:150ms}
  .main :global(.st .face){padding:12px 16px;gap:8px;height:100%;box-sizing:border-box}
  @keyframes cardIn{from{opacity:0;transform:translateY(12px) scale(.97)}}

  /* Resumen */
  .tot{font:900 56px/1 var(--round);color:var(--ink);letter-spacing:-.02em}
  .tot small{font-size:var(--t-l);font-weight:800;margin-left:4px;color:var(--ink-2)}
  .sub{display:flex;flex-direction:column;gap:4px;font:600 var(--t-s)/1.2 var(--round);color:var(--ink-2)}
  .streak{display:flex;align-items:center;gap:6px;align-self:flex-start;padding:5px 12px 5px 9px;border-radius:999px;
    font:700 var(--t-s)/1 var(--round);color:var(--ink);background:color-mix(in srgb,var(--arc-naranja) 16%,transparent)}
  .streak :global(svg){color:var(--arc-naranja)}
  .streak b{font-weight:900}
  .streak small{margin-left:4px;color:var(--ink-2);font-weight:600}
  .tags{display:flex;flex-wrap:wrap;gap:5px;margin-top:auto}
  .tags span{padding:3px 9px;border-radius:999px;font:600 var(--t-xs)/1.3 var(--round);color:var(--ink-2);background:color-mix(in srgb,var(--ink) 7%,transparent)}
  .tags span.d{color:var(--ink);background:color-mix(in srgb,var(--c) 20%,transparent)}
  .jpr{display:flex;flex-wrap:wrap;gap:4px 12px;margin-top:auto;font:600 var(--t-s)/1.2 var(--round);color:var(--ink-2)}
  .jpr b{color:var(--ink);font-weight:800}

  /* Barras por día: apiladas con el color de cada categoría, sin más adorno */
  .bars{position:relative;flex:1;display:grid;grid-template-columns:repeat(7,1fr);gap:10px;min-height:0;padding-bottom:20px}
  .day{position:relative;display:flex;flex-direction:column;justify-content:flex-end;align-items:center;min-height:0}
  .day .stk{position:relative;width:100%;max-width:42px;height:calc(100% - 18px);display:flex;flex-direction:column-reverse;border-radius:10px;overflow:hidden;
    background:color-mix(in srgb,var(--ink) 6%,transparent)}
  .day .stk i{display:block;height:var(--h);flex:none;transform-origin:bottom;animation:grow .55s var(--e-bounce) both;animation-delay:calc(120ms + var(--i)*45ms)}
  .day .v{position:absolute;top:0;font:700 var(--t-xs)/1 var(--round);color:var(--ink-2)}
  .day .d{position:absolute;bottom:-20px;font:800 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .day.today .d{color:var(--ink)}
  .day.today .d::after{content:'';position:absolute;left:50%;bottom:-6px;width:5px;height:5px;margin-left:-2.5px;border-radius:50%;background:var(--c)}
  @keyframes grow{from{transform:scaleY(0)}}
  .goal{position:absolute;left:0;right:0;bottom:calc(20px + (100% - 38px) * var(--g));border-top:2px dashed color-mix(in srgb,var(--c) 70%,transparent);z-index:1;pointer-events:none}
  .goal span{position:absolute;right:0;top:-16px;font:700 var(--t-xs)/1 var(--round);color:var(--ink-2)}

  /* Lo más jugado */
  .tr{display:grid;grid-template-columns:auto 1fr auto;align-items:center;gap:10px;font:700 var(--t-s)/1.2 var(--round);color:var(--ink);
    animation:cardIn .4s var(--e-bounce) both;animation-delay:calc(160ms + var(--i)*60ms)}
  .tr span{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .tr b{color:var(--ink-2);font-weight:800}
  .none{font:600 var(--t-s)/1.4 var(--round);color:var(--ink-2)}

  /* Metas: siete cuadritos por categoría, más oscuros cuanto más tiempo */
  .sq-h,.sq-r{display:grid;grid-template-columns:minmax(0,1fr) repeat(7,34px) 34px;gap:7px;align-items:center}
  .sq-h span{font:800 var(--t-xs)/1 var(--round);color:var(--ink-2);text-align:center}
  .sq-r .nm{display:flex;align-items:center;gap:6px;font:700 var(--t-s)/1.1 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sq-r .nm :global(svg){width:15px;height:15px;flex:none;color:var(--c)}
  .sq-r i,.leg i{display:block;width:34px;height:34px;border-radius:9px;box-sizing:border-box;
    background:color-mix(in srgb,var(--c,var(--acc)) calc(var(--p)*88% + 6%),color-mix(in srgb,var(--ink) 5%,transparent));
    animation:pop .35s var(--e-bounce) both;animation-delay:calc(140ms + var(--i,0)*18ms)}
  .sq-r i,.leg i.ok{display:grid;place-items:center}
  .sq-r i svg,.leg i svg{width:60%;height:60%;fill:none;stroke:var(--c);stroke-width:2;stroke-linecap:round;stroke-linejoin:round}
  .sq-r i.dk svg,.leg i.dk svg{stroke:#fff}
  .sq-r b{font:800 var(--t-s)/1 var(--round);color:var(--ink-2);text-align:right}
  .main :global(.goals .face){gap:6px}
  .leg{display:flex;align-items:center;gap:5px;margin-top:auto;font:600 var(--t-xs)/1 var(--round);color:var(--ink-2)}
  .leg i{width:14px;height:14px;border-radius:4px;animation:none}
  .leg span+i.ok{margin-left:8px}
  @keyframes pop{from{transform:scale(.4);opacity:0}}
</style>
