<script lang="ts">
  // Estadísticas › Semana: chips de la semana (tiempo total, meta diaria, racha, capturas, lo mejor, lectura japonesa)
  // y una columna por categoría con su color fijo; las categorías nunca se mezclan.
  import { onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_KEYS, goalMin, type GoalKey } from '../lib/readgoals';
  import { fmtHM, weekReview } from '../lib/stats';
  import { catIcon, catRank, catTone } from '../lib/chiptone';
  import { IS_TAURI, backend } from '../lib/backend';
  import type { WeekAch } from '../lib/types';
  import type { ReadStats } from '../lib/novel-types';
  import ChipPane from '../components/ChipPane.svelte';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import MiniCover from '../components/MiniCover.svelte';

  const DIAS = 'LMXJVSD';
  const goals = $derived(Object.fromEntries(GOAL_KEYS.filter((k) => goalMin(app.config, k) > 0).map((k) => [k, goalMin(app.config, k)])) as Partial<Record<GoalKey, number>>);
  const W = $derived(weekReview(app.lib, app.weekOff, app.favs, app.playing, goals, app.now.getTime()));
  const wid = $derived(`w${W.range.from}`);
  const plat = (pi: number) => app.lib.platforms[pi];
  const parts = (ms: number) => { const m = Math.round(ms / 60000); return m < 60 ? { v: m, u: 'min' } : { v: Math.floor(m / 60), u: `h ${String(m % 60).padStart(2, '0')}` }; };

  /* G · tiempo total frente a la semana anterior */
  const tot = $derived(parts(W.ms));
  const diff = $derived(W.ms - W.prevMs);
  const cmpMax = $derived(Math.max(W.ms, W.prevMs, 1));

  /* R · siete anillos: mejor meta de cada día (sin metas, días con actividad) */
  const days = $derived(Array.from({ length: 7 }, (_, i) => {
    const gs = W.blocks.filter((b) => b.goal);
    if (gs.length) return Math.min(1, Math.max(...gs.map((b) => b.goal!.perDay[i] ?? 0)));
    return W.blocks.reduce((a, b) => a + b.perDay[i], 0) > 0 ? 1 : 0;
  }));
  const hasGoals = $derived(W.blocks.some((b) => b.goal));
  const met = $derived(days.filter((v) => v >= 1).length);

  /* Lo más leído o jugado: el título con más tiempo de la semana, de cualquier categoría */
  const topT = $derived.by(() => {
    let r: { pi: number; ti: number; ms: number; kind: 'play' | 'read' } | null = null;
    for (const b of W.blocks) for (const x of b.top) if (!r || x.ms > r.ms) r = { pi: b.pi, ti: x.ti, ms: x.ms, kind: b.kind };
    return r;
  });
  /* Día con más tiempo y media de los días con actividad */
  const DN = ['Lunes', 'Martes', 'Miércoles', 'Jueves', 'Viernes', 'Sábado', 'Domingo'];
  const perDay = $derived(Array.from({ length: 7 }, (_, i) => W.blocks.reduce((a, b) => a + b.perDay[i], 0)));
  const bestDay = $derived(perDay.reduce((m, v, i) => (v > perDay[m] ? i : m), 0));
  const activeN = $derived(perDay.filter((v) => v > 0).length);

  /* Logros de la semana, repartidos a su categoría por el juego (los de juegos sin categoría se ignoran) */
  let achs = $state<WeekAch[]>([]);
  $effect(() => {
    const from = W.range.from, to = W.range.to;
    let live = true;
    backend.weekAchievements(from, to).then((l) => { if (live) achs = l; }).catch(() => { if (live) achs = []; });
    return () => { live = false; };
  });
  const achBy = $derived.by(() => {
    const where = new Map<string, { pi: number; ti: number }>();
    app.lib.platforms.forEach((p, pi) => p.titles.forEach((t, ti) => { if (t.gameId) where.set(t.gameId, { pi, ti }); }));
    const by = new Map<number, { n: number; rare: (WeekAch & { ti: number }) | null }>();
    for (const a of achs) {
      const w = where.get(a.gameId);
      if (!w) continue;
      const e = by.get(w.pi) ?? { n: 0, rare: null };
      e.n++;
      if (a.pct !== null && (e.rare === null || a.pct < (e.rare.pct ?? 101))) e.rare = { ...a, ti: w.ti };
      by.set(w.pi, e);
    }
    return by;
  });
  const pctf = (p: number) => `${p < 10 ? p.toLocaleString('es-ES', { maximumFractionDigits: 1 }) : Math.round(p)} %`;

  /* Novelas japonesas: caracteres, velocidad y palabras nuevas de la semana */
  let rs = $state<ReadStats | null>(null);
  const hasJvn = $derived(app.lib.platforms.some((p) => p.id === 'jvn'));
  onMount(() => {
    if (!IS_TAURI || !hasJvn) return;
    import('../lib/ipc').then(({ invoke }) => invoke<ReadStats>('readstats_get', { gameId: null })).then((s) => (rs = s)).catch(() => {});
  });
  const jp = $derived.by(() => {
    if (!rs) return null;
    const d0 = Math.floor((W.range.from - new Date(W.range.from).getTimezoneOffset() * 60000) / 864e5);
    const wd = rs.days.filter((d) => d.day >= d0 && d.day < d0 + 7);
    const chars = wd.reduce((a, d) => a + d.chars, 0), ms = wd.reduce((a, d) => a + d.activeMs, 0);
    return { chars, cph: ms > 0 ? Math.round(chars / (ms / 3600000)) : 0, fresh: wd.reduce((a, d) => a + d.newKnown, 0), phrases: wd.reduce((a, d) => a + (d.phrases ?? 0), 0) };
  });
  const nf = (n: number) => n.toLocaleString('es-ES');

  /* Columnas por categoría */
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const sunday = $derived(W.range.to - 864e5);
  const dm = (ms: number) => { const d = new Date(ms); return `${d.getDate()} ${MON[d.getMonth()]}`; };
  const cols = $derived([...W.blocks].sort((a, b) => catRank(plat(a.pi).id) - catRank(plat(b.pi).id)).map((b) => {
    const pl = plat(b.pi), max = Math.max(1, ...b.perDay);
    const tags: Array<{ t: string; done?: boolean }> = [];
    if (b.finished.length) tags.push({ t: b.finished.length === 1 ? `${pl.titles[b.finished[0]].name} terminado` : `${b.finished.length} terminados`, done: true });
    if (b.milestones.length) { const m = b.milestones[0]; tags.push({ t: `${pl.titles[m.ti].name} pasa de ${m.h} h` }); }
    const ac = achBy.get(b.pi);
    if (ac) tags.push({ t: `${ac.n} ${ac.n === 1 ? 'logro' : 'logros'}${ac.rare?.pct != null ? ` · el más raro, ${pctf(ac.rare.pct)}` : ''}` });
    if (b.goal) tags.push({ t: `Meta ${b.goal.met} de 7` });
    if (b.shots) tags.push({ t: `${b.shots} ${b.shots === 1 ? 'captura' : 'capturas'}` });
    if (b.consoleMs) tags.push({ t: `${fmtHM(b.consoleMs)} en la consola` });
    if (b.consoleUntil !== undefined && b.consoleUntil < sunday) tags.push({ t: `Datos de Nintendo hasta el ${dm(b.consoleUntil)}` });
    return {
      b, id: pl.id, bars: b.perDay.map((v) => ({ h: Math.round((v / max) * 100), top: v > 0 && v === max })),
      rings: b.goal ? b.goal.perDay.map((v) => Math.min(1, v)) : b.perDay.map((v) => (v > 0 ? 1 : 0)),
      d: b.ms - b.prevMs, tags: tags.slice(0, 4),
    };
  }));
  const label = $derived(app.weekOff === 0 ? 'lunes a domingo · toda la biblioteca' : `${dm(W.range.from)} a ${dm(sunday)} · toda la biblioteca`);
</script>

<ChipPane>
 <div class="wk">
  <h3 class="sh"><CIcon name="cal" />La semana<span>{label}</span></h3>
  <div class="cgrid">
    <StatChip k="wk0" icon="swap" label="Tiempo total" tone={4}>
      <div class="vrow">
        <div class="x-v">{tot.v}<small>{tot.u}</small></div>
        {#if W.ms > 0 || W.prevMs > 0}
          <span class="delta" class:dn={diff < 0}><CIcon name="up" class="" />{fmtHM(Math.abs(diff))}</span>
        {/if}
      </div>
      <div class="cmp">
        <div class="now"><span>{app.weekOff === 0 ? 'Esta' : 'Elegida'}</span><u><i style="--w:{(W.ms / cmpMax) * 100}%"></i></u><b>{fmtHM(W.ms)}</b></div>
        <div><span>Anterior</span><u><i style="--w:{(W.prevMs / cmpMax) * 100}%"></i></u><b>{fmtHM(W.prevMs)}</b></div>
      </div>
    </StatChip>

    <StatChip k="wk1" icon="check" label={hasGoals ? 'Meta diaria' : 'Días activos'} tone={3}>
      <div class="x-s"><b style="font-size:var(--t-xl)">{met} de 7</b> {hasGoals ? 'días cumplidos' : 'días con partidas'}</div>
      <div class="x-wk">
        {#each days as p, i}<div><i class={p >= 1 ? 'x-ok' : ''} style="--p:{p};--i:{i}"></i><span>{DIAS[i]}</span></div>{/each}
      </div>
    </StatChip>

    <StatChip k="wk2" icon={topT?.kind === 'play' ? 'swap' : 'book'} label={topT?.kind === 'play' ? 'Lo más jugado' : 'Lo más leído'} tone={topT ? catTone(plat(topT.pi).id) : 0} act={topT ? `sr:${topT.pi}:${topT.ti}` : undefined}>
      {#if topT}
        {@const t = plat(topT.pi).titles[topT.ti]}
        <div class="topt"><MiniCover {t} acc="var(--v1)" /><div><b>{t.name}</b><span>{fmtHM(topT.ms)} · {plat(topT.pi).name}</span></div></div>
      {:else}
        <div class="x-s">Nada esta semana.</div>
      {/if}
    </StatChip>

    <StatChip k="wk3" icon="clock" label="Mejor día" tone={1}>
      {#if perDay[bestDay] > 0}
        <div class="x-v" style="font-size:var(--t-xl)">{DN[bestDay]}</div>
        <div class="x-s"><b>{fmtHM(perDay[bestDay])}</b> · media de {fmtHM(W.ms / Math.max(1, activeN))} al día</div>
      {:else}
        <div class="x-s">Sin actividad esta semana.</div>
      {/if}
    </StatChip>
  </div>

  {#if jp}
    <div class="r2">
      <StatChip k="wk5" icon="book" label="Novelas japonesas · lectura" tone="jvn">
        <div class="jpr">
          <div><div class="x-v">{nf(jp.chars)}</div><div class="x-s">caracteres leídos</div></div>
          <div><div class="x-v">{nf(jp.cph)}<small>/h</small></div><div class="x-s">velocidad</div></div>
          <div><div class="x-v">{nf(jp.fresh)}</div><div class="x-s">palabras nuevas</div></div>
          {#if jp.phrases}<div><div class="x-v">{nf(jp.phrases)}</div><div class="x-s">frases al glosario</div></div>{/if}
        </div>
      </StatChip>
    </div>
  {/if}

  {#if cols.length}
    <div class="cols" style="grid-template-columns:repeat({Math.max(3, cols.length)},minmax(0,1fr))">
      {#each cols as c, i}
        <StatChip k="wk{6 + i}" cls="col" tone={catTone(c.id)} act="gc:{c.b.pi}">
          <div class="ch"><CIcon name={catIcon(c.id)} class="" /><span>{c.b.name}</span>
            <div class="tt"><b>{fmtHM(c.b.ms)}</b>{#if c.d !== 0}<span class="delta" class:dn={c.d < 0}><CIcon name={c.d < 0 ? 'down' : 'up'} class="" />{fmtHM(Math.abs(c.d))}</span>{/if}</div>
          </div>
          {#if c.b.kind === 'play'}
            <div class="x-bars">{#each c.bars as b, j}<div class={b.top ? 'x-top' : ''}><i style="--h:{Math.max(4, b.h)}%;--i:{j}"></i><span>{DIAS[j]}</span></div>{/each}</div>
          {:else}
            <div class="x-wk">{#each c.rings as p, j}<div><i class={p >= 1 ? 'x-ok' : ''} style="--p:{p};--i:{j}"></i><span>{DIAS[j]}</span></div>{/each}</div>
          {/if}
          <div class="tops">
            {#each c.b.top as r (r.ti)}
              {@const t = plat(c.b.pi).titles[r.ti]}
              <div><MiniCover {t} acc="var(--v1)" small /><span>{t.name}</span><b>{fmtHM(r.ms)}</b></div>
            {/each}
          </div>
          <div class="tags">{#each c.tags as g}<span class={g.done ? 'd' : ''}><i></i>{g.t}</span>{/each}</div>
        </StatChip>
      {/each}
    </div>
  {:else}
    <div class="cnone">Sin actividad esta semana. Las categorías aparecerán aquí cuando haya partidas, capturas o terminados.</div>
  {/if}
 </div>
</ChipPane>

<style>
  /* Todo en una pantalla (1280×720): cifras arriba, «lo mejor» y lectura en una franja, y una columna por categoría. */
  .wk :global(.sh){margin:0 0 8px}
  .wk :global(.cgrid .st.st){height:112px}
  .wk :global(.st .face){padding:10px 14px;gap:6px}
  .r2{display:grid;grid-template-columns:minmax(0,1fr);gap:var(--s-4);margin:12px 0 0}
  .cols{margin-top:12px}
  .topt{display:flex;align-items:center;gap:10px;min-width:0}
  .topt > div{display:flex;flex-direction:column;gap:3px;min-width:0}
  .topt b{font:800 var(--t-m)/1.15 var(--round);color:var(--ink);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .topt span{font:600 var(--t-s)/1.2 var(--round);color:var(--ink-2)}
  .r2 :global(.st.st){height:88px}
  .wk :global(.cgrid .x-v){font-size:var(--t-2xl)}
  .wk :global(.cgrid .cmp){gap:4px}
  .wk :global(.cgrid .cmp div){grid-template-columns:58px 1fr 54px}
  .wk :global(.cgrid .cmp u){height:8px}
  .wk :global([data-k="wk0"] .x-v){font-size:var(--t-xl)}
  .wk :global([data-k="wk0"] .cmp){gap:2px}
  .wk :global([data-k="wk0"] .cmp div){font-size:var(--t-xs);line-height:12px}
  .wk :global(.cpane .st.col),.wk :global(.st.col.col){height:226px}
  .wk :global(.col .ch){font-size:var(--t-m)}
  .wk :global(.col .ch > span){flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .wk :global(.col .tt b){font-size:var(--t-l)}
  .wk :global(.col .x-bars){height:52px}
  .wk :global(.col .x-wk i){width:22px;height:22px}
  .wk :global(.col .tops){gap:4px}
  .wk :global(.col .tops i){height:22px}
  .jpr{display:flex;gap:var(--s-6);align-items:flex-end;margin-top:auto}
  .cnone{padding:var(--s-6);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}
</style>
