<script lang="ts">
  // Estadísticas › Semana: chips de la semana (tiempo total, meta diaria, racha, capturas, lo mejor, lectura japonesa)
  // y una columna por categoría con su color fijo; las categorías nunca se mezclan.
  import { onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { calm } from '../lib/controller.svelte';
  import { GOAL_KEYS, goalMin, type GoalKey } from '../lib/readgoals';
  import { fmtHM, weekReview, type Highlight } from '../lib/stats';
  import { catIcon, catTone } from '../lib/chiptone';
  import { countUp } from '../lib/countup';
  import { IS_TAURI, backend } from '../lib/backend';
  import type { WeekAch } from '../lib/types';
  import type { ReadStats } from '../lib/novel-types';
  import ChipPane from '../components/ChipPane.svelte';
  import StatChip from '../components/StatChip.svelte';
  import CIcon from '../components/CIcon.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import ThumbImg from '../components/ThumbImg.svelte';

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

  /* F · racha más larga de las metas */
  const streak = $derived(Math.max(0, ...W.blocks.map((b) => b.goal?.streak ?? 0)));
  const freeDays = $derived(Math.max(0, ...W.blocks.map((b) => b.goal?.free ?? 0)));

  /* K · capturas */
  const shots = $derived(W.blocks.reduce((a, b) => a + b.shots, 0));
  const fan = $derived(W.blocks.map((b) => b.shotPick).filter((s): s is NonNullable<typeof s> => !!s).slice(0, 3));
  const FAN = [['-14deg', '4px'], ['14deg', '4px'], ['0deg', '-6px']];

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

  /* Q · lo mejor, rota cada 3 s; el logro más raro (si lo tiene menos del 10 % de jugadores) entra como uno más */
  let bi = $state(0), prevBi = $state(-1);
  const best = $derived.by((): Highlight[] => {
    let r: { pi: number; a: WeekAch & { ti: number } } | null = null;
    for (const [pi, e] of achBy) if (e.rare && (e.rare.pct ?? 101) < 10 && (!r || e.rare.pct! < r.a.pct!)) r = { pi, a: e.rare };
    if (!r) return W.highlights;
    const h: Highlight = { kind: 'rare', pi: r.pi, ti: r.a.ti, value: r.a.pct!, text: `${r.a.displayName} · ${plat(r.pi).titles[r.a.ti].name} (${pctf(r.a.pct!)})` };
    return [...W.highlights.slice(0, 2), h, ...W.highlights.slice(2)];
  });
  $effect(() => {
    void wid;
    bi = 0; prevBi = -1;
    if (best.length < 2 || calm()) return;
    const id = setInterval(() => { prevBi = bi; bi = (bi + 1) % best.length; setTimeout(() => (prevBi = -1), 320); }, 3000);
    return () => clearInterval(id);
  });
  const HI: Record<Highlight['kind'], { icon: string; label: string }> = {
    milestone: { icon: 'trophy', label: 'Hito' }, streak: { icon: 'flame', label: 'Racha' },
    finished: { icon: 'check', label: 'Terminado' }, record: { icon: 'clock', label: 'Récord de partida' },
    rare: { icon: 'star', label: 'Logro raro' },
  };

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
  const cols = $derived(W.blocks.map((b) => {
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
  <h3 class="sh"><CIcon name="cal" />La semana<span>{label}</span></h3>
  <div class="cgrid">
    <StatChip k="wk0" icon="swap" label="Tiempo total" tone={4}>
      <div class="vrow">
        <div class="x-v"><span use:countUp={{ to: tot.v, key: `week:G:${wid}` }}>{tot.v}</span><small>{tot.u}</small></div>
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

    <StatChip k="wk2" icon="flame" label="Racha" tone={0}>
      <div style="display:flex;align-items:center;gap:var(--s-3)">
        <svg class="flame" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3c1 4 6 6 6 11a6 6 0 0 1-12 0c0-3 2-4 2-7 2 1 3 3 3 5 1-1 1-4 1-9z" class="tint" /><path d="M12 3c1 4 6 6 6 11a6 6 0 0 1-12 0c0-3 2-4 2-7 2 1 3 3 3 5 1-1 1-4 1-9z" /></svg>
        <div>
          <div class="x-v"><span use:countUp={{ to: streak, key: `week:F:${wid}` }}>{streak}</span><small>{streak === 1 ? 'día' : 'días'}</small></div>
          <div class="x-s">{!W.blocks.some((b) => b.goal) ? 'Sin meta diaria' : freeDays ? `${freeDays} ${freeDays === 1 ? 'día libre' : 'días libres'}` : 'Seguidos con la meta'}</div>
        </div>
      </div>
      <div class="chain">
        {#each Array(14) as _, i}<i class:on={i >= 14 - Math.min(14, streak)} class:x-today={i === 13 && streak > 0 && app.weekOff === 0} style="--i:{i}"></i>{/each}
      </div>
    </StatChip>

    <StatChip k="wk3" icon="cam" label="Capturas" tone={1}>
      <div class="x-v"><span use:countUp={{ to: shots, key: `week:K:${wid}` }}>{shots}</span><small>{shots === 1 ? 'captura' : 'capturas'}</small></div>
      <div class="fan">
        {#each fan as s, i (s.path)}<ThumbImg path={s.path} style="--a:{FAN[i][0]};--y:{FAN[i][1]};--i:{i}" />{/each}
        {#if fan.some((s) => s.fav)}<svg class="x-star" viewBox="0 0 24 24" fill="currentColor" stroke="var(--tint-base)" stroke-width="1.5" stroke-linejoin="round"><path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z" /></svg>{/if}
      </div>
    </StatChip>

    <StatChip k="wk4" icon="trophy" label="Lo mejor de la semana" wide tone={2}>
      {#if best.length}
        <div class="x-best">
          {#each best as h, i}
            <div class:on={i === bi} class:out={i === prevBi} style="--v1:var(--arc-{['rojo', 'naranja', 'amarillo', 'verde', 'azul', 'anil', 'violeta'][catTone(plat(h.pi).id) % 7]})">
              <span class="bico"><CIcon name={HI[h.kind].icon} class="" /></span>
              <div><h4>{h.text}</h4><p>{HI[h.kind].label}</p></div>
            </div>
          {/each}
        </div>
        <div class="x-dots">{#each best as _, i}<i class:on={i === bi}></i>{/each}</div>
      {:else}
        <div class="x-s">Aún no hay nada que destacar esta semana.</div>
      {/if}
    </StatChip>

    {#if jp}
      <StatChip k="wk5" icon="book" label="Novelas japonesas · lectura" wide tone={5}>
        <div class="jpr">
          <div><div class="x-v"><span use:countUp={{ to: jp.chars, key: `week:J:${wid}` }}>{jp.chars}</span></div><div class="x-s">caracteres leídos</div></div>
          <div><div class="x-v">{nf(jp.cph)}<small>/h</small></div><div class="x-s">velocidad</div></div>
          <div><div class="x-v"><span use:countUp={{ to: jp.fresh, key: `week:N:${wid}` }}>{jp.fresh}</span></div><div class="x-s">palabras nuevas</div></div>
          {#if jp.phrases}<div><div class="x-v"><span use:countUp={{ to: jp.phrases, key: `week:P:${wid}` }}>{jp.phrases}</span></div><div class="x-s">frases al glosario</div></div>{/if}
        </div>
      </StatChip>
    {/if}
  </div>

  <h3 class="sh"><CIcon name="grid" />Por categoría<span>cada una en su bloque, con su color; nunca se mezclan</span></h3>
  {#if cols.length}
    <div class="cols">
      {#each cols as c, i}
        <StatChip k="wk{6 + i}" cls="col" tone={catTone(c.id)} act={c.b.top[0] ? `sr:${c.b.pi}:${c.b.top[0].ti}` : undefined}>
          <div class="ch"><CIcon name={catIcon(c.id)} class="" />{c.b.name}</div>
          <div class="tt"><b>{fmtHM(c.b.ms)}</b>{#if c.d !== 0}<span class="delta" class:dn={c.d < 0}><CIcon name="up" class="" />{fmtHM(Math.abs(c.d))}</span>{/if}</div>
          {#if c.b.kind === 'play'}
            <div class="x-bars">{#each c.bars as b, j}<div class={b.top ? 'x-top' : ''}><i style="--h:{Math.max(4, b.h)}%;--i:{j}"></i><span>{DIAS[j]}</span></div>{/each}</div>
          {:else}
            <div class="x-wk">{#each c.rings as p, j}<div><i class={p >= 1 ? 'x-ok' : ''} style="--p:{p};--i:{j}"></i><span>{DIAS[j]}</span></div>{/each}</div>
          {/if}
          <div class="sub">{c.b.kind === 'play' ? 'Más jugados' : 'Lo más leído'}</div>
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
</ChipPane>

<style>
  .jpr{display:flex;gap:var(--s-6);align-items:flex-end;margin-top:auto}
  .cnone{padding:var(--s-6);font:600 var(--t-m)/1.4 var(--round);color:var(--ink-2)}
</style>
