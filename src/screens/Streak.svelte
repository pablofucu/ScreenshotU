<script lang="ts">
  // Pantalla «Racha de lectura»: la tarjeta «Lectura de hoy» de Inicio a pantalla completa. Arriba, los anillos de hoy,
  // una medalla con la racha de cada anillo y el mapa de 6 meses del anillo elegido (LB/RB); abajo, qué novelas se han
  // leído este mes, a qué horas y el resumen del año. Si hay una racha cumplida sin «encender», se enciende aquí.
  import { onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { GOAL_COL, dayDate, markRescueSeen, readDetail, rescueCls, rescueSeen, type GoalKey, type GoalTrack } from '../lib/readgoals';
  import { createReadRings, fmtMin as fmtMinShared, rescueClose, rescueDay } from '../lib/readrings.svelte';
  import { blip } from '../lib/sound';
  import { fmtClock, fmtHM } from '../lib/stats';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import HuchaRescate from '../components/HuchaRescate.svelte';

  const WEEKS = 26, CELL = 17;
  const MON = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const MON_L = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
  const WD = ['domingo', 'lunes', 'martes', 'miércoles', 'jueves', 'viernes', 'sábado'];

  // Pistas, anillos y «racha pendiente» compartidos con la tarjeta de Inicio (ReadRings).
  const S = 150, SW = 16;
  const rr = createReadRings({ weeks: WEEKS, size: S, stroke: SW, gap: 1.18, empty: 'var(--face)' });
  const now = $derived(rr.now);
  const today = $derived(rr.today);
  const tracks = $derived(rr.tracks);
  const rings = $derived(rr.rings);
  const { pending, cellBg } = rr;
  const sel = $derived(tracks.find((t) => t.key === app.streakKey) ?? tracks[0]);
  const C = $derived(GOAL_COL[sel?.key ?? 'vn']);
  const det = $derived(sel ? readDetail(sel.key, sel.goal, app.lib.platforms, app.playing, now) : null);

  const shown = rr.shown;

  /* ---------- rescate de racha ---------- */
  // La hucha ocupa el hueco del mapa SOLO mientras hay un rescate pendiente: entera si es el del anillo que se ve,
  // partida en dos si lo hay en las dos categorías; sin rescate (o con el rescate en el otro anillo) sigue el mapa.
  const resc = $derived(tracks.filter((t) => t.rescue));
  const huch = $derived(resc.length > 1 ? 'duo' : sel?.rescue ? 'wide' : '');
  // Aviso «¡Hucha llena!» (2,6 s, una vez por rescate pagado): uno tras otro si se pagan dos a la vez.
  let toast = $state<{ k: GoalKey; text: string } | null>(null);
  const toastQ: Array<{ k: GoalKey; text: string }> = [];
  function nextToast() {
    const n = toastQ.shift();
    toast = n ?? null;
    if (!n) return;
    blip(1320);
    rr.timers.push(setTimeout(nextToast, 2600));
  }
  $effect(() => {
    let added = false;
    for (const t of tracks) {
      if (t.paidDay == null || rescueSeen(t.key, t.paidDay)) continue;
      markRescueSeen(t.key, t.paidDay);
      toastQ.push({ k: t.key, text: `${GOAL_COL[t.key].full}: el ${rescueDay(t.paidDay, true)} cuenta · ${t.streak} ${t.streak === 1 ? 'día' : 'días'}` });
      added = true;
    }
    if (added && !toast) nextToast();
  });

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
  function todayNote(t: GoalTrack) {
    if (t.done) {
      const d = t.key === sel?.key ? det : readDetail(t.key, t.goal, app.lib.platforms, app.playing, now);
      return d?.doneAt ? `✓ Cumplida a las ${fmtClock(d.doneAt)}` : '✓ Meta cumplida';
    }
    const left = Math.ceil(t.goal - t.today);
    return t.streak > 0 ? `Faltan ${fmtMin(left)} para seguir la racha` : `Faltan ${fmtMin(left)}`;
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
    // Primero la del anillo que se está viendo.
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

<TopBar crumbs={['Inicio', 'Racha de lectura']} />

{#if sel && det}
  <div class="streak" class:calm={calm()} class:huch={!!huch} bind:this={root}>
    <!-- Hoy -->
    <div class="stpanel shoy in" style="--d:0s">
      <div class="sth">Hoy <small>{todayLabel}</small></div>
      <div class="shbody">
        <div class="sring" style="width:{S}px;height:{S}px">
          <svg width={S} height={S} viewBox="0 0 {S} {S}">
            {#each rings as g, i (g.t.key)}
              <circle cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={GOAL_COL[g.t.key].soft} stroke-width={SW} />
              <circle class="sarc" cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={GOAL_COL[g.t.key].c} stroke-width={SW} stroke-linecap="round"
                stroke-dasharray={g.C} stroke-dashoffset={drawn ? g.C * (1 - g.t.frac) : g.C}
                style="transition-delay:{0.25 + i * 0.25}s" transform="rotate(-90 {S / 2} {S / 2})" />
            {/each}
          </svg>
        </div>
        <div class="sleg">
          {#each tracks as t (t.key)}
            {@const c = GOAL_COL[t.key]}
            <div class="sli {rescueCls(t.key)}" style="--c:{c.c};--soft:{c.soft};--ink:{c.ink}">
              <div class="sln"><i></i>{c.full}</div>
              <div class="slv"><b>{fmtMin(t.today)}</b> / {goalLabel(t.goal)}</div>
              <div class="sgoal"><b style="width:{drawn ? t.frac * 100 : 0}%"></b></div>
              <div class="sln2" class:ok={t.done}>{todayNote(t)}{#if t.rescue && t.done}<b class="shu"> · +{t.rescue.gotToday} min a la hucha</b>{/if}</div>
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
        <div class="smed in {rescueCls(t.key)}" data-g={t.key} class:on={t.key === sel.key} style="--c:{c.c};--soft:{c.soft};--ink:{c.ink};--d:{0.12 + i * 0.12}s">
          <span class="sflw">
            <svg class="sfl" viewBox="0 0 24 24"><path fill={c.c} fill-opacity=".22" stroke={c.c} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" /></svg>
            <span class="sparks">{#each Array(10) as _}<i></i>{/each}</span>
          </span>
          <div class="smt">
            <div class="snw"><span class="snum">{n}</span><small>{n === 1 ? 'día' : 'días'}</small>{#if t.rescue}<span class="spend">Pendiente</span>{:else if n > 0 && n >= t.best}<span class="srec">Récord</span>{/if}</div>
            <div class="ssub">{n > 0 ? `Racha de ${c.full.toLowerCase()}` : `Sin racha de ${c.full.toLowerCase()}`}</div>
            <div class="ssub b">{#if t.rescue}Recuperado {t.rescue.got} de {t.rescue.need} min · hasta el {rescueClose(t.rescue, true)}{:else}{t.best > 0 ? `Mejor: ${t.best} ${t.best === 1 ? 'día' : 'días'} · ${range(t.bestFrom, t.bestTo)}` : `Cumple ${goalLabel(t.goal)} hoy para empezarla`}{/if}</div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Hucha de rescate (en el hueco del mapa) o mapa de 6 meses del anillo elegido -->
    {#if huch === 'wide' && sel.rescue}
      <HuchaRescate r={sel.rescue} now={now} tabs={tracks.map((t) => t.key)} style="left:464px;top:224px;width:768px;height:352px" />
    {:else if huch === 'duo'}
      <div class="duo">
        {#each resc as t (t.key)}
          <HuchaRescate r={t.rescue!} now={now} wide={false} style="position:relative;left:auto;top:auto;width:auto;height:auto" />
        {/each}
      </div>
    {:else}
    <div class="stpanel smap in {rescueCls(sel.key)}" style="--c:{C.c};--edge:{C.edge};--d:.3s">
      <div class="sth">
        <span>Últimos 6 meses <small>meta {goalLabel(sel.goal)} al día</small></span>
        {#if tracks.length > 1}
          <span class="stabs pill">
            <span class="btn sh" data-hact="lb">LB</span>
            {#each tracks as t (t.key)}<span class="t" class:on={t.key === sel.key} style="--c:{GOAL_COL[t.key].c}" data-hact={t.key === sel.key ? '' : t.key === tracks[0].key ? 'lb' : 'rb'}>{GOAL_COL[t.key].full}</span>{/each}
            <span class="btn sh" data-hact="rb">RB</span>
          </span>
        {/if}
      </div>
      <div class="smrow">
        <div class="sheat" style="width:{WEEKS * CELL}px">
          <div class="scols">
            {#each sel.weeks as w, wi}
              <div>
                {#each w as d, di}
                  <span class="scell" class:st={d.streak && !(d.today && pending(sel))} class:td={d.today} class:fu={d.future} class:rd={d.rescued}
                    style="--k:{wi};background:{d.future ? 'transparent' : d.rescued ? 'var(--rc)' : cellBg(sel, d.r, d.today)}"></span>
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
    {/if}

    <!-- Qué has leído este mes -->
    <div class="stpanel sgames in" style="--c:{C.c};--d:.42s">
      <div class="sth">Qué has leído este mes</div>
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
        <div class="snone">Este mes aún no has leído {sel.key === 'vn' ? 'novelas visuales' : 'novelas japonesas'}.</div>
      {/if}
    </div>

    <!-- A qué hora lees -->
    <div class="stpanel shours in" style="--c:{C.c};--d:.5s">
      <div class="sth">A qué hora lees <small>30 días</small></div>
      <div class="shb">
        {#each det.hours as h, i}<i class:pk={h > 0 && h === hmax} style="height:{drawn ? Math.max(h > 0 ? 4 : 2, (h / hmax) * 100) : 0}%;transition-delay:{0.5 + i * 0.02}s"></i>{/each}
      </div>
      <div class="shl"><span>7:00</span><span>13:00</span><span>19:00</span><span>1:00</span></div>
      <div class="speak">{peak || 'Sin lecturas en los últimos 30 días'}</div>
    </div>

    <!-- Este año -->
    <div class="stpanel syear in" style="--d:.58s">
      <div class="sth">Este año</div>
      <div class="syg">
        <div><span>Horas</span><b>{Math.round(det.year.min / 60)}</b></div>
        <div><span>Días cumplidos</span><b>{det.year.met}</b></div>
        <div><span>Novelas</span><b>{det.year.titles}</b></div>
        <div><span>Partidas</span><b>{det.year.sessions}</b></div>
      </div>
    </div>

    {#if toast}
      <div class="stoast {rescueCls(toast.k)}">
        <svg class="stfl" viewBox="0 0 24 24"><path fill="currentColor" fill-opacity=".25" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" /></svg>
        <div><b>¡Hucha llena!</b><span>{toast.text}</span></div>
      </div>
    {/if}
  </div>
  <Hints list={[...(tracks.length > 1 ? [['LB/RB', 'Novelas / japonés', 'lb'] as [string, string, string]] : []), ['B', 'Volver', 'b']]} />
{:else}
  <div class="stnone">No hay categorías de novelas en la biblioteca.</div>
  <Hints list={[['B', 'Volver', 'b']]} />
{/if}

<style>
  .streak{position:absolute;inset:0}
  .in{animation:sup .55s cubic-bezier(.2,.8,.3,1) var(--d,0s) both}
  .calm .in,.calm .scell{animation:none}
  @keyframes sup{from{opacity:0;transform:translateY(22px)}to{opacity:1;transform:none}}
  .sth small{font-size:14px;font-weight:700;color:var(--ink-3);margin-left:8px}

  /* hoy */
  .shoy{left:48px;top:88px;width:400px;height:320px;padding:16px 20px}
  .shbody{display:flex;align-items:center;gap:18px;height:252px}
  .sring{position:relative;flex:none}
  .sring svg{display:block;overflow:visible}
  .sarc{transition:stroke-dashoffset 1.1s cubic-bezier(.3,.8,.3,1)}
  .sleg{flex:1;min-width:0;display:flex;flex-direction:column;gap:16px}
  .sln{display:flex;align-items:center;gap:7px;font-size:15px;font-weight:800;color:var(--ink-2);white-space:nowrap}
  .sln i{width:11px;height:11px;border-radius:50%;background:var(--c);flex:none}
  .slv{font-size:14px;font-weight:700;color:var(--ink-3);margin:2px 0 5px}
  .slv b{font-size:22px;font-weight:800;color:var(--ink)}
  .sgoal{height:8px;border-radius:4px;background:var(--soft);overflow:hidden}
  .sgoal b{display:block;height:100%;border-radius:4px;background:var(--c);transition:width 1.1s .4s cubic-bezier(.3,.8,.3,1)}
  .sln2{margin-top:5px;font-size:12.5px;font-weight:700;color:#7a8291;line-height:1.25}
  .sln2.ok{color:var(--ok)}
  .sln2 .shu{color:var(--rink)}

  /* medallas */
  .smeds{position:absolute;left:464px;top:88px;width:768px;height:120px;display:grid;gap:16px}
  .smed{position:relative;display:flex;align-items:center;gap:14px;padding:0 20px;border-radius:24px;min-width:0;
    background:linear-gradient(180deg,color-mix(in srgb,var(--soft) 40%,white),var(--soft));
    box-shadow:inset 0 1.5px 0 var(--glass-top),0 0 0 1.5px color-mix(in srgb,var(--c) 40%,white),0 16px 26px -18px color-mix(in srgb,var(--c) 70%,black)}
  .smed.on{box-shadow:inset 0 1.5px 0 var(--glass-top),0 0 0 2.5px var(--c),0 16px 26px -18px color-mix(in srgb,var(--c) 70%,black)}
  .sflw{position:relative;flex:none;width:62px;height:62px}
  .sfl{width:100%;height:100%;display:block}
  .sparks{position:absolute;left:50%;top:55%}
  .sparks i{position:absolute;left:-4px;top:-4px;width:8px;height:8px;border-radius:50%;background:var(--c);opacity:0}
  .smt{min-width:0}
  .snw{display:flex;align-items:baseline;gap:6px;height:52px;overflow:hidden}
  .snum{display:inline-block;font-size:50px;font-weight:800;line-height:52px;color:var(--ink)}
  .snw small{font-size:18px;font-weight:800;color:var(--ink)}
  .srec{align-self:center;margin-left:4px;font-size:11px;font-weight:800;color:var(--on);background:var(--c);border-radius:999px;padding:2px 9px;text-transform:uppercase;letter-spacing:.05em}
  .spend{align-self:center;margin-left:4px;font-size:11px;font-weight:800;color:var(--on);background:var(--rc);border-radius:999px;padding:2px 9px;text-transform:uppercase;letter-spacing:.05em}
  .ssub{font-size:14px;font-weight:700;color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .ssub.b{font-size:13px;color:var(--ink-3)}

  /* mapa */
  .smap{left:464px;top:224px;width:768px;height:184px;padding:14px 20px 8px}
  .stabs{display:flex;align-items:center;gap:3px;height:28px;margin:-4px 0;padding:0 3px;border-radius:999px}
  .stabs .t{height:22px;padding:0 11px;border-radius:999px;display:grid;place-items:center;font-size:14px;font-weight:800;color:var(--ink-2);cursor:pointer}
  .stabs .t.on{background:var(--c);color:var(--on);box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 0 0 1px color-mix(in srgb,var(--c) 70%,black);cursor:default}
  .stabs .btn{cursor:pointer;transform:scale(.78);margin:0 -3px}
  .smrow{display:flex;gap:18px;margin-top:6px}
  .sheat{flex:none}
  .scols{display:flex;gap:3px}
  .scols>div{display:flex;flex-direction:column;gap:3px}
  .scell{display:block;width:14px;height:14px;border-radius:4px;animation:spop .3s calc(.35s + var(--k) * 18ms) both}
  .scell.st{box-shadow:0 0 0 1.5px var(--edge)}
  .scell.td{box-shadow:0 0 0 2px var(--face-hi),0 0 0 3.5px var(--c)}
  .scell.fu{box-shadow:inset 0 0 0 1px var(--line)}
  /* Día reparado (rescate pagado): parche del color del rescate con puntada */
  .scell.rd{box-shadow:0 0 0 1.5px var(--edge);outline:1.5px dashed rgba(255,255,255,.9);outline-offset:-4px}
  @keyframes spop{from{opacity:0;transform:scale(.2)}to{opacity:1;transform:scale(1)}}
  .smon{position:relative;height:14px;margin-top:3px;line-height:14px}
  .smon span{position:absolute;top:0;font-size:12px;font-weight:700;color:var(--ink-3)}
  .stiles{flex:1;min-width:0;display:flex;flex-direction:column;gap:10px}
  .stiles .rbox{height:58px;padding:6px 14px;box-shadow:inset 0 0 0 1px rgba(95,110,135,.14);background:var(--face)}
  .stiles .rbox .k{font-size:12.5px}
  .stiles .rbox .v{font-size:22px}
  .stiles .rbox .v small{font-size:14px}

  /* hucha de rescate: ocupa el hueco del mapa y del resto de paneles de la derecha */
  .duo{position:absolute;left:464px;top:224px;width:768px;height:352px;display:grid;grid-template-columns:1fr 1fr;gap:18px}
  .huch .shours,.huch .syear{display:none}
  .huch .sgames{width:400px}
  .huch .sgr{grid-template-columns:28px minmax(0,1fr) 64px 56px}
  .stoast{position:absolute;z-index:25;left:50%;top:300px;transform:translate(-50%,0);display:flex;align-items:center;gap:16px;padding:18px 28px;border-radius:26px;
    background:linear-gradient(180deg,var(--face-hi),var(--rsoft));animation:stoast .45s cubic-bezier(.3,1.4,.4,1) both;
    box-shadow:inset 0 1.5px 0 var(--glass-top),0 0 0 2px var(--rc),0 0 40px 6px color-mix(in srgb,var(--rc) 45%,transparent),0 30px 50px -20px color-mix(in srgb,var(--rc) 60%,black)}
  .stoast b{display:block;font-size:30px;font-weight:800;color:var(--rink)}
  .stoast span{font-size:16px;font-weight:700;color:var(--rink);opacity:.85}
  .stfl{width:64px;height:64px;color:var(--rc);flex:none}
  @keyframes stoast{from{opacity:0;transform:translate(-50%,0) scale(.6)}to{opacity:1;transform:translate(-50%,0) scale(1)}}

  /* abajo */
  .sgames,.shours,.syear{top:424px;height:226px;padding:14px 20px 10px}
  .sgames{left:48px;width:470px}
  .shours{left:534px;width:380px}
  .syear{left:930px;width:302px}
  .sgr{display:grid;grid-template-columns:28px minmax(0,1fr) 120px 64px;align-items:center;gap:10px;height:37px}
  .sgr .nm{font-size:15px;font-weight:800;color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sgr .gb{height:9px;border-radius:5px;background:var(--face);overflow:hidden}
  .sgr .gb b{display:block;height:100%;border-radius:5px;background:var(--c);transition:width 1s .6s cubic-bezier(.3,.8,.3,1)}
  .sgr .tm{font-size:14px;font-weight:800;color:var(--ink-2);text-align:right;white-space:nowrap}
  .smore{font-size:13px;font-weight:700;color:var(--ink-3);margin-top:0;line-height:15px}
  .snone{margin-top:40px;font-size:15px;font-weight:700;color:var(--ink-3);text-align:center}
  .shb{display:flex;align-items:flex-end;gap:3px;height:120px;margin-top:12px;border-bottom:2px solid var(--line)}
  .shb i{flex:1;border-radius:3px 3px 1px 1px;background:color-mix(in srgb,var(--c) 45%,white);transition:height .7s cubic-bezier(.3,.8,.3,1)}
  .shb i.pk{background:var(--c)}
  .shl{display:flex;justify-content:space-between;margin-top:4px;font-size:11.5px;font-weight:700;color:var(--ink-3)}
  .speak{margin-top:6px;font-size:13.5px;font-weight:800;color:var(--ink-2)}
  .syg{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-top:12px}
  .syg div{border-radius:14px;background:var(--face);padding:8px 12px;box-shadow:inset 0 0 0 1px rgba(95,110,135,.12)}
  .syg span{display:block;font-size:12.5px;font-weight:700;color:var(--ink-3);white-space:nowrap}
  .syg b{font-size:26px;font-weight:800;color:var(--ink)}
</style>
