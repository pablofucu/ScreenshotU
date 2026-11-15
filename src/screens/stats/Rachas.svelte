<script lang="ts">
  // Estadísticas › Rachas: anillos de hoy, medallas de racha, mapa de 6 meses de la meta elegida,
  // títulos leídos o jugados este mes, franja horaria y resumen del año. Sin hucha de rescate.
  import { onMount } from 'svelte';
  import { app } from '../../lib/store.svelte';
  import { GOAL_COL, GOAL_TXT, dayDate, goalHistOf, goalMin, goalTrack, groupOf, readDetail, type GoalKey, type GoalTrack } from '../../lib/readgoals';
  import { createReadRings, fmtMin as fmtMinShared } from '../../lib/readrings.svelte';
  import { fmtClock, fmtHM } from '../../lib/stats';
  import MiniCover from '../../components/MiniCover.svelte';

  const WEEKS = 26, CELL = 16;
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const MON_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
  const WD = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];

  // Anillos, animaciones y «racha pendiente» compartidos con ReadRings; las pistas se calculan con historial de metas.
  const S = 140, SW = 15, GAP = 1.18;
  const rr = createReadRings({
    weeks: WEEKS,
    size: S,
    stroke: SW,
    gap: GAP,
    empty: 'var(--face)',
    strokeFor: (n) => (n > 2 ? { stroke: 9, gap: 1.25 } : { stroke: SW, gap: GAP }),
  });
  const now = $derived(rr.now);
  const today = $derived(rr.today);
  const tracks = $derived(
    app.goalKeys.map((k) => goalTrack(k, goalMin(app.config, k), app.lib.platforms, app.playing, now, WEEKS, goalHistOf(app.config, k))),
  );
  const many = $derived(tracks.length > 2);
  const sg = $derived(many ? { stroke: 9, gap: 1.25 } : { stroke: SW, gap: GAP });
  const rings = $derived(
    tracks.map((t, i) => {
      const r = S / 2 - sg.stroke / 2 - 1 - i * sg.stroke * sg.gap;
      return { t, r, C: 2 * Math.PI * r };
    }),
  );
  const { pending, shown } = rr;
  const sel = $derived(tracks.find((t) => t.key === app.streakKey) ?? tracks[0]);
  const C = $derived(GOAL_COL[sel?.key ?? 'vn']);
  const T = $derived(GOAL_TXT[groupOf(sel?.key ?? 'vn')]);
  const det = $derived(sel ? readDetail(sel.key, sel.goal, app.lib.platforms, app.playing, now) : null);

  function cellBg(t: GoalTrack, r: number, isToday: boolean) {
    if (isToday && pending(t)) r = Math.min(r, 0.6);
    if (!(r > 0)) return 'var(--face)';
    const c = GOAL_COL[t.key].c;
    if (r > 2) return `color-mix(in srgb, ${c} ${(100 - 50 * Math.min(1, (r - 2) / 6)).toFixed(1)}%, black)`;
    const p = r < 1 ? 15 + 45 * r : 60 + 40 * (r - 1);
    return `color-mix(in srgb, ${c} ${p.toFixed(1)}%, transparent)`;
  }

  /* ---------- formatos ---------- */
  const fmtMin = (m: number) => fmtMinShared(m, true);
  const goalLabel = (m: number) => (m % 60 === 0 ? `${m / 60} h` : fmtMin(m));
  const dm = (d: number) => { const x = dayDate(d); return `${x.getUTCDate()} ${MON[x.getUTCMonth()]}`; };
  function range(a: number, b: number) {
    if (!Number.isFinite(a) || !Number.isFinite(b)) return '';
    if (a === b) return dm(a);
    const x = dayDate(a), y = dayDate(b);
    return x.getUTCMonth() === y.getUTCMonth() ? `${x.getUTCDate()}–${dm(b)}` : `${dm(a)} – ${dm(b)}`;
  }
  const todayLabel = $derived.by(() => { const x = dayDate(today); return `${WD[x.getUTCDay()]} ${x.getUTCDate()} de ${MON_L[x.getUTCMonth()]}`; });

  /* ---------- anillos de hoy ---------- */
  function todayNote(t: GoalTrack): { ok: boolean; text: string } {
    if (t.done) {
      const d = t.key === sel?.key ? det : readDetail(t.key, t.goal, app.lib.platforms, app.playing, now);
      return { ok: true, text: d?.doneAt ? `Cumplida a las ${fmtClock(d.doneAt)}` : 'Meta cumplida' };
    }
    const left = Math.ceil(t.goal - t.today);
    return { ok: false, text: t.streak > 0 ? `Faltan ${fmtMin(left)} para seguir la racha` : `Faltan ${fmtMin(left)}` };
  }

  /* ---------- mapa ---------- */
  const firstDay = $derived(today - ((today + 3) % 7) - (WEEKS - 1) * 7);
  const monthLabels = $derived.by(() => {
    const out: Array<{ x: number; t: string }> = [];
    let prev = -1;
    for (let w = 0; w < WEEKS; w++) {
      const m = dayDate(firstDay + w * 7).getUTCMonth();
      if (m !== prev && w < WEEKS - 1) out.push({ x: w * CELL, t: MON[m] });
      prev = m;
    }
    return out[0]?.x === 0 && out[1]?.x <= CELL * 2 ? out.slice(1) : out;
  });

  /* ---------- horas ---------- */
  const hmax = $derived(Math.max(1, ...(det?.hours ?? [0])));
  const peak = $derived.by(() => {
    if (!det || det.hours.every((h) => h < 1)) return '';
    const i = det.hours.indexOf(Math.max(...det.hours));
    const h = (i + 7) % 24;
    return `Sobre todo de ${h}:00 a ${(h + 1) % 24}:00`;
  });
  const gmax = $derived(Math.max(1, ...(det?.games.map((g) => g.min) ?? [0])));

  /* ---------- entrada y racha que se enciende ---------- */
  const calm = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const drawn = $derived(rr.drawn);
  let root: HTMLDivElement | undefined = $state();
  onMount(() => {
    const todo = tracks.filter(pending).map((t) => t.key);
    // Primero la de la meta seleccionada.
    todo.sort((a, b) => (a === sel?.key ? -1 : b === sel?.key ? 1 : 0));
    todo.forEach((k, i) => rr.timers.push(setTimeout(() => light(k), calm() ? 0 : 1100 + i * 900)));
  });

  function light(k: GoalKey) {
    rr.light(k, root, {
      box: '.smed[data-g="{k}"]',
      num: '.snum',
      out: [{ transform: 'translateY(0)', opacity: 1 }, { transform: 'translateY(-60px)', opacity: 0 }],
      outOpts: { duration: 200, easing: 'ease-in', fill: 'forwards' },
      skip: calm,
      enter: (box, num) => {
        num.animate([{ transform: 'translateY(60px)', opacity: 0 }, { transform: 'translateY(-8px)', opacity: 1 }, { transform: 'translateY(0)' }], { duration: 480, easing: 'ease-out' });
        box.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.07)' }, { transform: 'scale(.98)' }, { transform: 'scale(1)' }], { duration: 650, easing: 'ease-out' });
        box.animate([{ boxShadow: `0 0 0 0 ${GOAL_COL[k].c}aa` }, { boxShadow: `0 0 0 26px ${GOAL_COL[k].c}00` }], { duration: 800, easing: 'ease-out' });
        const fl = box.querySelector<HTMLElement>('.sfl');
        fl?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.45) rotate(-12deg)' }, { transform: 'scale(1)' }], { duration: 650 });
        // Chispas que salen de la llama.
        const sp = box.querySelector<HTMLElement>('.sparks');
        sp?.querySelectorAll<HTMLElement>('i').forEach((e, i) => {
          const a = (i / 10) * Math.PI * 2, r = 46 + (i % 3) * 10;
          e.animate([{ transform: 'translate(0,0) scale(1)', opacity: 1 }, { transform: `translate(${Math.cos(a) * r}px,${Math.sin(a) * r}px) scale(.3)`, opacity: 0 }], { duration: 700, easing: 'cubic-bezier(.2,.8,.3,1)' });
        });
        if (k === sel?.key) {
          root?.querySelector('.scell.td')?.animate([{ transform: 'scale(2.4)' }, { transform: 'scale(1)' }], { duration: 450, easing: 'ease-out' });
          root?.querySelectorAll('.scell.st').forEach((e, i) =>
            e.animate([{ filter: 'brightness(1)', transform: 'scale(1)' }, { filter: 'brightness(1.8)', transform: 'scale(1.5)' }, { filter: 'brightness(1)', transform: 'scale(1)' }], { duration: 400, delay: 120 + i * 45 }));
        }
      },
    });
  }
</script>

{#if sel && det}
  <div class="streak" class:calm={calm()} class:many bind:this={root}>
    <!-- Hoy -->
    <div class="stpanel shoy in" style="--d:0s">
      <div class="sth">Hoy <small>{todayLabel}</small></div>
      <div class="shbody">
        <div class="sring" style="width:{S}px;height:{S}px">
          <svg width={S} height={S} viewBox="0 0 {S} {S}">
            {#each rings as g, i (g.t.key)}
              <circle cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={GOAL_COL[g.t.key].soft} stroke-width={sg.stroke} />
              <circle class="sarc" cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={GOAL_COL[g.t.key].c} stroke-width={sg.stroke} stroke-linecap="round"
                stroke-dasharray={g.C} stroke-dashoffset={drawn ? g.C * (1 - g.t.frac) : g.C}
                style="transition-delay:{0.25 + i * 0.25}s" transform="rotate(-90 {S / 2} {S / 2})" />
            {/each}
          </svg>
        </div>
        <div class="sleg">
          {#each tracks as t (t.key)}
            {@const c = GOAL_COL[t.key]}
            {@const note = todayNote(t)}
            <div class="sli" style="--c:{c.c};--soft:{c.soft};--ink:{c.ink}">
              {#if many}
                <div class="sln"><i></i>{c.tab}<span class="slv"><b>{fmtMin(t.today)}</b> / {goalLabel(t.goal)}</span></div>
              {:else}
                <div class="sln"><i></i>{c.full}</div>
                <div class="slv"><b>{fmtMin(t.today)}</b> / {goalLabel(t.goal)}</div>
              {/if}
              <div class="sgoal"><b style="width:{drawn ? t.frac * 100 : 0}%"></b></div>
              <div class="sln2" class:ok={note.ok}>
                {#if note.ok}<svg class="ic-ok" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5 9.5-10" /></svg>{/if}{note.text}
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Medallas de racha -->
    <div class="smeds" style="grid-template-columns:repeat({tracks.length},1fr)">
      {#each tracks as t, i (t.key)}
        {@const c = GOAL_COL[t.key]}
        {@const n = shown(t)}
        <div class="smed in" data-g={t.key} class:on={t.key === sel.key} data-hact={t.key === sel.key ? '' : 'goal:' + t.key} style="--c:{c.c};--soft:{c.soft};--ink:{c.ink};--d:{0.12 + i * 0.12}s">
          <span class="sflw">
            <svg class="sfl" viewBox="0 0 24 24"><path fill={c.c} fill-opacity=".22" stroke={c.c} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" /></svg>
            <span class="sparks">{#each Array(10) as _}<i></i>{/each}</span>
          </span>
          <div class="smt">
            <div class="snw"><span class="snum">{n}</span><small>{n === 1 ? 'día' : 'días'}</small>{#if t.owe > 0}<span class="spend">Pendiente</span>{:else if n > 0 && n >= t.best}<span class="srec">Récord</span>{/if}</div>
            <div class="ssub">{n > 0 ? `Racha de ${(many ? c.tab : c.full).toLowerCase()}` : `Sin racha de ${(many ? c.tab : c.full).toLowerCase()}`}</div>
            <div class="ssub b">{#if t.owe > 0 && t.open.length}Por recuperar: {fmtMin(t.owe)} · hasta el {dm(t.open[0].deadline)}{:else}{t.best > 0 ? `Mejor: ${t.best} ${t.best === 1 ? 'día' : 'días'} · ${range(t.bestFrom, t.bestTo)}` : `Cumple ${goalLabel(t.goal)} hoy para empezarla`}{/if}</div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Mapa de 6 meses del anillo elegido -->
    <div class="stpanel smap in" style="--c:{C.c};--edge:{C.edge};--d:.3s">
      <div class="sth">
        <span>Últimos 6 meses{#if !many} <small>meta {goalLabel(sel.goal)} al día</small>{/if}</span>
        {#if tracks.length > 1}
          <span class="stabs pill">
            {#each tracks as t (t.key)}<span class="t" class:on={t.key === sel.key} style="--c:{GOAL_COL[t.key].c}" data-hact={t.key === sel.key ? '' : 'goal:' + t.key}>{many ? GOAL_COL[t.key].tab : GOAL_COL[t.key].full}</span>{/each}
          </span>
        {/if}
      </div>
      <div class="smrow">
        <div class="sheat" style="width:{WEEKS * CELL}px">
          <div class="scols">
            {#each sel.weeks as w, wi}
              <div>
                {#each w as d}
                  <span class="scell" class:st={d.streak && !(d.today && pending(sel))} class:td={d.today} class:fu={d.future} class:fr={d.free} class:ow={d.owe}
                    style="--k:{wi};background:{d.future ? 'transparent' : d.free ? 'color-mix(in srgb, var(--warn) 24%, var(--tint-base))' : cellBg(sel, d.r, d.today)}">
                    {#if d.free}<svg class="smoon" viewBox="0 0 24 24"><path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" /></svg>{/if}
                  </span>
                {/each}
              </div>
            {/each}
          </div>
          <div class="smon">{#each monthLabels as m}<span style="left:{m.x}px">{m.t}</span>{/each}</div>
        </div>
        <div class="stiles">
          <div class="rbox"><div class="k">Este mes</div><div class="v">{det.month.met} <small>de {det.month.elapsed} {det.month.elapsed === 1 ? 'día' : 'días'}</small></div></div>
          <div class="rbox"><div class="k">Media diaria · 30 días</div><div class="v">{fmtMin(det.avg30)}</div></div>
        </div>
      </div>
    </div>

    <!-- Qué has leído este mes -->
    <div class="stpanel sgames in" style="--c:{C.c};--d:.42s">
      <div class="sth">{T.what}</div>
      {#if det.games.length}
        {#each det.games.slice(0, 4) as g (g.t.id)}
          <div class="sgr">
            <MiniCover t={g.t} acc={C.c} small />
            <span class="nm">{g.t.name}</span>
            <span class="gb"><b style="width:{drawn ? (g.min / gmax) * 100 : 0}%"></b></span>
            <span class="tm">{fmtHM(g.min * 60000)}</span>
          </div>
        {/each}
        {#if det.games.length > 4}<div class="smore">y {det.games.length - 4} más</div>{/if}
      {:else}
        <div class="snone">{({ vn: 'Este mes aún no has leído novelas visuales.', jvn: 'Este mes aún no has leído novelas japonesas.', pc: 'Este mes aún no has jugado a juegos de PC.', switch: 'Este mes aún no has jugado a la Switch.' })[sel.key]}</div>
      {/if}
    </div>

    <!-- A qué hora lees -->
    <div class="stpanel shours in" style="--c:{C.c};--d:.5s">
      <div class="sth">{T.when} <small>30 días</small></div>
      <div class="shb" class:nil={!det.hours.some((h) => h > 0)}>
        {#each det.hours as h, i}<i class:pk={h > 0 && h === hmax} style="height:{drawn && h > 0 ? Math.max(4, (h / hmax) * 100) : 0}%;transition-delay:{0.5 + i * 0.02}s"></i>{/each}
      </div>
      <div class="shl"><span>7:00</span><span>13:00</span><span>19:00</span><span>1:00</span></div>
      <div class="speak">{peak || T.none30}</div>
    </div>

    <!-- Este año -->
    <div class="stpanel syear in" style="--d:.58s">
      <div class="sth">Este año</div>
      <div class="syg">
        <div><span>Horas</span><b>{Math.round(det.year.min / 60)}</b></div>
        <div><span>Días cumplidos</span><b>{det.year.met}</b></div>
        <div><span>{T.titles}</span><b>{det.year.titles}</b></div>
        <div><span>Partidas</span><b>{det.year.sessions}</b></div>
      </div>
    </div>
  </div>
{:else}
  <div class="stnone">No hay metas diarias en la biblioteca.</div>
{/if}

<style>
  .streak{position:absolute;inset:0}
  .in{animation:sup .55s var(--e-soft) var(--d,0s) both}
  .calm .in,.calm .scell{animation:none}
  @keyframes sup{from{opacity:0;transform:translateY(22px)}to{opacity:1;transform:none}}
  .sth small{font-size:var(--t-s);font-weight:700;color:var(--ink-2);margin-left:var(--s-2)}

  /* hoy */
  .shoy{left:48px;top:144px;width:400px;height:284px;padding:14px 20px}
  .shbody{display:flex;align-items:center;gap:18px;height:228px}
  .sring{position:relative;flex:none}
  .sring svg{display:block;overflow:visible}
  .sarc{transition:stroke-dashoffset 1.1s var(--e-soft)}
  .sleg{flex:1;min-width:0;display:flex;flex-direction:column;gap:var(--s-3)}
  .sln{display:flex;align-items:center;gap:7px;font-size:var(--t-m);font-weight:800;color:var(--ink-2);white-space:nowrap}
  .sln i{width:11px;height:11px;border-radius:50%;background:var(--c);flex:none}
  .slv{font-size:var(--t-s);font-weight:700;color:var(--ink-2);margin:2px 0 4px}
  .slv b{font-size:var(--t-xl);font-weight:800;color:var(--ink)}
  .sgoal{height:8px;border-radius:var(--r-xs);background:var(--soft);overflow:hidden}
  .sgoal b{display:block;height:100%;border-radius:var(--r-xs);background:var(--c);transition:width 1.1s .4s var(--e-soft)}
  .sln2{display:flex;align-items:center;gap:var(--s-1);margin-top:var(--s-1);font-size:var(--t-s);font-weight:700;color:var(--ink-2);line-height:1.25}
  .sln2.ok{color:var(--ok-f)}
  :global(#stage[data-theme=dark]) .sln2.ok{color:var(--ok)}
  .ic-ok{width:14px;height:14px;flex:none;fill:none;stroke:currentColor;stroke-width:2.5;stroke-linecap:round;stroke-linejoin:round}

  /* medallas */
  .smeds{position:absolute;left:464px;top:144px;width:768px;height:102px;display:grid;gap:var(--s-4)}
  .smed{position:relative;display:flex;align-items:center;gap:var(--s-3);padding:0 18px;border-radius:var(--r-l);min-width:0;cursor:pointer;
    background:linear-gradient(180deg,color-mix(in srgb,var(--soft) 35%,var(--tint-base)),color-mix(in srgb,var(--soft) 35%,var(--tint-base)));
    box-shadow:inset 0 1.5px 0 var(--glass-top),0 0 0 1.5px color-mix(in srgb,var(--c) 40%,var(--tint-base)),0 16px 26px -18px color-mix(in srgb,var(--c) 70%,black)}
  .smed.on{cursor:default;box-shadow:inset 0 1.5px 0 var(--glass-top),0 0 0 2.5px var(--c),0 16px 26px -18px color-mix(in srgb,var(--c) 70%,black)}
  .sflw{position:relative;flex:none;width:54px;height:54px}
  .sfl{width:100%;height:100%;display:block}
  .sparks{position:absolute;left:50%;top:55%}
  .sparks i{position:absolute;left:-4px;top:-4px;width:8px;height:8px;border-radius:50%;background:var(--c);opacity:0}
  .smt{min-width:0}
  .snw{display:flex;align-items:baseline;gap:6px;height:46px;overflow:hidden}
  .snum{display:inline-block;font-size:var(--t-3xl);font-weight:800;line-height:46px;color:var(--ink)}
  .snw small{font-size:var(--t-l);font-weight:800;color:var(--ink)}
  .srec{align-self:center;margin-left:var(--s-1);font-size:var(--t-xs);font-weight:800;color:var(--on);background:var(--c);border-radius:var(--r-pill);padding:2px 9px;text-transform:uppercase;letter-spacing:.05em}
  .spend{align-self:center;margin-left:var(--s-1);font-size:var(--t-xs);font-weight:800;color:color-mix(in srgb,var(--warn) 22%,black);background:var(--warn);border-radius:var(--r-pill);padding:2px 9px;text-transform:uppercase;letter-spacing:.05em}
  .ssub{font-size:var(--t-s);font-weight:700;color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .ssub.b{font-size:var(--t-s);color:var(--ink-2)}

  /* mapa */
  .smap{left:464px;top:258px;width:768px;height:170px;padding:10px 20px 8px}
  .stabs{display:flex;align-items:center;gap:3px;height:28px;margin:-4px 0;padding:0 3px;border-radius:var(--r-pill)}
  .stabs .t{height:22px;padding:0 11px;border-radius:var(--r-pill);display:grid;place-items:center;font-size:var(--t-s);font-weight:800;color:var(--ink-2);cursor:pointer}
  .stabs .t.on{background:var(--c);color:var(--on);box-shadow:inset 0 1px 0 var(--hl-ring),0 0 0 1px color-mix(in srgb,var(--c) 70%,black);cursor:default}
  .smrow{display:flex;gap:18px;margin-top:6px}
  .sheat{flex:none}
  .scols{display:flex;gap:3px}
  .scols>div{display:flex;flex-direction:column;gap:3px}
  .scell{position:relative;display:grid;place-items:center;width:13px;height:13px;border-radius:4px;box-sizing:border-box;animation:spop .3s calc(.35s + var(--k) * 18ms) both}
  .scell.st{box-shadow:0 0 0 1.5px var(--edge)}
  .scell.fr{box-shadow:inset 0 0 0 1.5px var(--warn)}
  .scell.st.fr{box-shadow:0 0 0 1.5px var(--edge),inset 0 0 0 1.5px var(--warn)}
  .scell.ow{border:1.5px dashed var(--warn);box-shadow:none}
  .scell.st.ow{box-shadow:0 0 0 1.5px var(--edge)}
  .scell.td{box-shadow:0 0 0 2px var(--face-hi),0 0 0 3.5px var(--c)}
  .scell.fu{box-shadow:inset 0 0 0 1px var(--line)}
  .smoon{width:8px;height:8px;fill:none;stroke:color-mix(in srgb,var(--warn) 65%,var(--ink));stroke-width:2.6;stroke-linecap:round;stroke-linejoin:round;pointer-events:none}
  @keyframes spop{from{opacity:0;transform:scale(.2)}to{opacity:1;transform:scale(1)}}
  .smon{position:relative;height:14px;margin-top:3px;line-height:14px}
  .smon span{position:absolute;top:0;font-size:var(--t-s);font-weight:700;color:var(--ink-2)}
  .stiles{flex:1;min-width:0;display:flex;flex-direction:column;gap:var(--s-2)}
  .stiles .rbox{height:55px;padding:5px 14px;box-shadow:inset 0 0 0 1px var(--line);background:var(--face)}
  .stiles .rbox .k{font-size:var(--t-s)}
  .stiles .rbox .v{font-size:var(--t-xl)}
  .stiles .rbox .v small{font-size:var(--t-s)}

  /* modo compacto: más de dos metas */
  .many .sleg{gap:6px}
  .many .sln{font-size:var(--t-s);gap:6px}
  .many .sln .slv{margin:0 0 0 auto;font-size:var(--t-s)}
  .many .slv b{font-size:var(--t-m)}
  .many .sgoal{height:5px}
  .many .sln2{margin-top:2px;font-size:var(--t-s)}
  .many .smeds{gap:var(--s-2)}
  .many .smed{padding:0 var(--s-3);gap:var(--s-2)}
  .many .sflw{width:38px;height:38px}
  .many .snw{height:36px}
  .many .snum{font-size:32px;line-height:36px}
  .many .snw small{font-size:var(--t-s)}
  .many .srec,.many .spend{font-size:var(--t-xs);padding:1px 6px;margin-left:0}
  .many .ssub{font-size:var(--t-s)}
  .many .ssub.b{font-size:var(--t-xs)}
  .many .stabs .t{padding:0 var(--s-2);font-size:var(--t-s)}

  /* abajo */
  .sgames,.shours,.syear{top:442px;height:202px;padding:12px 20px 10px}
  .sgames{left:48px;width:470px}
  .shours{left:534px;width:380px}
  .syear{left:930px;width:302px}
  .sgr{display:grid;grid-template-columns:28px minmax(0,1fr) 120px 64px;align-items:center;gap:10px;height:35px}
  .sgr .nm{font-size:var(--t-m);font-weight:800;color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sgr .gb{height:9px;border-radius:var(--r-xs);background:var(--face);overflow:hidden}
  .sgr .gb b{display:block;height:100%;border-radius:var(--r-xs);background:var(--c);transition:width 1s .6s var(--e-soft)}
  .sgr .tm{font-size:var(--t-s);font-weight:800;color:var(--ink-2);text-align:right;white-space:nowrap}
  .smore{font-size:var(--t-s);font-weight:700;color:var(--ink-2);margin-top:0;line-height:15px}
  .snone{margin-top:36px;font-size:var(--t-m);font-weight:700;color:var(--ink-2);text-align:center}
  .shb{display:flex;align-items:flex-end;gap:3px;height:106px;margin-top:10px;border-bottom:2px solid var(--line)}
  .shb i{flex:1;border-radius:3px 3px 1px 1px;background:color-mix(in srgb,var(--c) 45%,var(--tint-base));transition:height .7s var(--e-soft)}
  .shb.nil{border-bottom-color:transparent}
  .shb i.pk{background:var(--c)}
  .shl{display:flex;justify-content:space-between;margin-top:var(--s-1);font-size:var(--t-xs);font-weight:700;color:var(--ink-2)}
  .speak{margin-top:6px;font-size:var(--t-s);font-weight:800;color:var(--ink-2)}
  .syg{display:grid;grid-template-columns:1fr 1fr;gap:var(--s-2);margin-top:10px}
  .syg div{border-radius:var(--r-s);background:var(--face);padding:7px var(--s-3);box-shadow:inset 0 0 0 1px var(--line)}
  .syg span{display:block;font-size:var(--t-s);font-weight:700;color:var(--ink-2);white-space:nowrap}
  .syg b{font-size:var(--t-2xl);font-weight:800;color:var(--ink)}
</style>
