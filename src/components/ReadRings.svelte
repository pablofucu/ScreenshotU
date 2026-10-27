<script lang="ts">
  // Tarjeta «Lectura de hoy» de Inicio: anillos concéntricos con los minutos de hoy (azul = novelas visuales,
  // rosa = novelas japonesas), una medalla con la racha de cada una y el mapa de las últimas semanas.
  // Al entrar se dibuja en cascada; al cumplir una meta «se enciende la racha» (una vez por día y anillo).
  import { GOAL_COL, rescueCls, type GoalKey } from '../lib/readgoals';
  import HuchaMini from './HuchaMini.svelte';
  import { createReadRings, fmtMin } from '../lib/readrings.svelte';

  const COL = GOAL_COL;
  const S = 112, SW = 11;

  // Geometría, pistas y «racha pendiente» compartidas con la pantalla de racha (Streak).
  const rr = createReadRings({ size: S, stroke: SW, gap: 1.15, empty: '#dde1e7' });
  const tracks = $derived(rr.tracks);
  const rings = $derived(rr.rings);
  const { pending, cellBg } = rr;

  let root: HTMLDivElement | undefined = $state();

  const scheduled = new Set<string>();
  $effect(() => {
    for (const t of tracks) {
      const id = `${t.key}:${rr.today}`;
      if (!pending(t) || scheduled.has(id)) continue;
      scheduled.add(id);
      // Tras la cascada de entrada si acaba de abrirse Inicio.
      const wait = Math.max(150, 1500 - (Date.now() - rr.mountedAt));
      rr.timers.push(setTimeout(() => light(t.key), wait));
    }
  });

  /** «La racha se enciende»: el número gira, la casilla de hoy se rellena y un brillo recorre la racha. */
  function light(k: GoalKey) {
    if (!root?.isConnected) return;
    rr.light(k, root, {
      box: '[data-g="{k}"]',
      num: '.rnum',
      out: [{ transform: 'translateY(0)' }, { transform: 'translateY(-30px)' }],
      outOpts: { duration: 180, easing: 'ease-in', fill: 'forwards' },
      enter: (box, num) => {
        num.animate([{ transform: 'translateY(30px)' }, { transform: 'translateY(-5px)' }, { transform: 'translateY(0)' }], { duration: 420, easing: 'ease-out' });
        box.querySelector('.rmed')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.2)' }, { transform: 'scale(.96)' }, { transform: 'scale(1)' }], { duration: 600, easing: 'ease-out' });
        box.querySelector('.rfl')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.8) rotate(-10deg)' }, { transform: 'scale(1)' }], { duration: 600 });
        box.querySelector('.cell.td')?.animate([{ transform: 'scale(2.4)' }, { transform: 'scale(1)' }], { duration: 450, easing: 'ease-out' });
        box.querySelectorAll('.cell.st').forEach((e, i) =>
          e.animate([{ filter: 'brightness(1)', transform: 'scale(1)' }, { filter: 'brightness(1.8)', transform: 'scale(1.5)' }, { filter: 'brightness(1)', transform: 'scale(1)' }], { duration: 400, delay: 120 + i * 55 }));
      },
    });
  }
</script>

<div class="rrings" bind:this={root}>
  <div class="rr-ring" style="width:{S}px;height:{S}px">
    <svg width={S} height={S} viewBox="0 0 {S} {S}">
      {#each rings as g, i (g.t.key)}
        <circle cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].soft} stroke-width={SW} />
        <circle class="arc" cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].c} stroke-width={SW} stroke-linecap="round"
          stroke-dasharray={g.C} stroke-dashoffset={rr.drawn ? g.C * (1 - g.t.frac) : g.C}
          style="transition-delay:{i * 0.2}s" transform="rotate(-90 {S / 2} {S / 2})" />
      {/each}
    </svg>
    <div class="rr-mins">
      {#each tracks as t (t.key)}<span style="color:{COL[t.key].ink}">{fmtMin(t.today)}</span>{/each}
    </div>
  </div>
  <div class="rr-side">
    {#each tracks as t, i (t.key)}
      {@const c = COL[t.key]}
      {@const n = pending(t) ? t.streak - 1 : t.streak}
      <div class="rr-col {rescueCls(t.key)}" data-g={t.key} style="--c:{c.c};--soft:{c.soft};--ink:{c.ink};--edge:{c.edge};--d:{0.3 + i * 0.14}s">
        <div class="rmed">
          <svg class="rfl" viewBox="0 0 24 24" fill="none" stroke={c.c} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" /></svg>
          <span class="rnw"><span class="rnum">{n}</span></span>
          <span class="rlab" title="{n} {n === 1 ? 'día seguido' : 'días seguidos'}">{c.name}{#if t.rescue}<small class="rp">pendiente</small>{:else}<small>{n > 0 && n >= t.best ? 'récord' : `mejor ${t.best}`}</small>{/if}</span>
        </div>
        {#if t.rescue}
          <HuchaMini r={t.rescue} now={rr.now} />
        {:else}
          <div class="rmap">
            {#each t.weeks as w, wi}
              <div>
                {#each w as d, di}
                  <span class="cell" class:st={d.streak && !(d.today && pending(t))} class:td={d.today} class:fu={d.future} class:rd={d.rescued}
                    style="--k:{wi + di};background:{d.future ? 'transparent' : d.rescued ? 'var(--rc)' : cellBg(t, d.r, d.today)}"></span>
                {/each}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .rrings{position:absolute;inset:0;display:flex;align-items:center;gap:14px;padding:0 12px;
    background:linear-gradient(180deg,#fbfcfd,#eef1f5);color:#3a414e}
  .rr-ring{position:relative;flex:none}
  .rr-ring svg{display:block;overflow:visible}
  .arc{transition:stroke-dashoffset .9s cubic-bezier(.3,.8,.3,1)}
  .rr-mins{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;font-size:16px;font-weight:800;line-height:1.2}
  .rr-side{flex:1;display:flex;gap:10px;min-width:0}
  .rr-col{flex:1;display:flex;flex-direction:column;gap:8px;min-width:0}
  .rmed{display:grid;grid-template-columns:18px max-content minmax(0,1fr);align-items:center;column-gap:4px;height:48px;padding:0 8px;border-radius:14px;
    background:linear-gradient(180deg,color-mix(in srgb,var(--soft) 45%,white),var(--soft));
    box-shadow:inset 0 1px 0 #fff,0 0 0 1px color-mix(in srgb,var(--c) 40%,white);
    animation:rdrop .55s ease-out var(--d) both}
  .rfl{width:18px;height:18px}
  .rnw{height:26px;overflow:hidden}
  .rnum{display:inline-block;font-size:24px;font-weight:800;line-height:26px;color:var(--ink)}
  .rlab{font-size:12px;font-weight:800;line-height:1.15;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:#4a5260}
  .rlab small{display:block;font-size:10px;font-weight:800;color:var(--ink);text-transform:uppercase;letter-spacing:.04em}
  .rlab small.rp{color:var(--rink)}
  .rmap{display:flex;gap:2px}
  .rmap>div{display:flex;flex-direction:column;gap:2px}
  .cell{width:8px;height:8px;border-radius:2px;animation:rpop .3s calc(.5s + var(--k) * 35ms) both}
  .cell.st{box-shadow:0 0 0 1.5px var(--edge)}
  .cell.td:not(.st){box-shadow:0 0 0 1px var(--c)}
  .cell.rd{box-shadow:0 0 0 1px var(--rc);outline:none}
  @keyframes rdrop{0%{opacity:0;transform:translateY(-30px)}70%{opacity:1;transform:translateY(5px)}100%{opacity:1;transform:translateY(0)}}
  @keyframes rpop{from{opacity:0;transform:scale(.2)}to{opacity:1;transform:scale(1)}}
</style>
