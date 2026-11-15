<script lang="ts">
  // Cara de la ficha de Inicio: racha de la categoría, mini velas de los últimos 7 días y fichas de días libres.
  // Al cumplir la meta de hoy «se enciende la racha» (una vez por día y categoría).
  import { GOAL_COL, type GoalKey, type GoalTrack } from '../lib/readgoals';
  import { fmtMin, freeDayName } from '../lib/readfmt';
  import { createReadRings } from '../lib/readrings.svelte';
  import VelasMini from './racha/VelasMini.svelte';

  let { keys, active = true }: { keys: GoalKey[]; active?: boolean } = $props();

  const FLAME_D = 'M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235';
  const MOON_D = 'M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z';
  const WARN_D = 'M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636 -2.87l-8.106 -13.536a1.914 1.914 0 0 0 -3.274 0z';

  const rr = createReadRings({ keys: () => keys, size: 22, stroke: 3.5, gap: 1.3, empty: 'var(--face)' });
  const tracks = $derived(rr.tracks);
  const { pending } = rr;

  let root: HTMLDivElement | undefined = $state();

  const scheduled = new Set<string>();
  const waiting = new Map<string, ReturnType<typeof setTimeout>>();
  let wasOff = false;
  $effect(() => {
    if (!active) {
      // Cara oculta: nada pendiente de encender; se reprograma al volver a estar activa.
      for (const [id, h] of waiting) { clearTimeout(h); scheduled.delete(id); }
      waiting.clear();
      wasOff = true;
      return;
    }
    for (const t of tracks) {
      const id = `${t.key}:${rr.today}`;
      if (!pending(t) || scheduled.has(id)) continue;
      scheduled.add(id);
      const wait = Math.max(wasOff ? 450 : 150, 1500 - (Date.now() - rr.mountedAt));
      const h = setTimeout(() => { waiting.delete(id); light(t.key); }, wait);
      waiting.set(id, h);
      rr.timers.push(h);
    }
    wasOff = false;
  });

  /** «La racha se enciende»: el número gira, la vela de hoy prende y el pulso recorre la racha. */
  function light(k: GoalKey) {
    if (!root?.isConnected || !active) return;
    rr.light(k, root, {
      box: '[data-g="{k}"]',
      num: '.rnum',
      out: [{ transform: 'translateY(0)' }, { transform: 'translateY(-100%)' }],
      outOpts: { duration: 180, easing: 'ease-in', fill: 'forwards' },
      enter: (box, num) => {
        num.animate([{ transform: 'translateY(100%)' }, { transform: 'translateY(-6%)' }, { transform: 'translateY(0)' }], { duration: 420, easing: 'ease-out' });
        box.querySelector('.flm')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.45) rotate(-8deg)' }, { transform: 'scale(1)' }], { duration: 600, easing: 'ease-out' });
        box.querySelector('.cst')?.animate([{ opacity: 0, transform: 'translateY(6px)' }, { opacity: 1, transform: 'none' }], { duration: 350 });
        const flames = [...box.querySelectorAll('.mt .top-ic.flame svg')];
        flames.slice(0, -1).reverse().forEach((el, i) =>
          el.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.4,1.55)' }, { transform: 'scale(1)' }], { duration: 480, delay: 200 + i * 100, easing: 'ease-out' }));
      },
    });
  }

  function statusOf(t: GoalTrack) {
    if (t.state === 'urgent' && t.urgent) {
      const when = t.urgent.left <= 1 ? 'hoy' : 'mañana';
      return { cls: 'r', icon: 'warn' as const, text: `Plazo: ${when} · ${fmtMin(t.urgent.owe, true)}` };
    }
    if (t.open.length > 0) {
      const lastDl = t.open[t.open.length - 1].deadline;
      return { cls: '', icon: 'moon' as const, text: `Debes ${fmtMin(t.owe, true)} · ${freeDayName(lastDl, false)}` };
    }
    if (t.done && !pending(t)) {
      return { cls: 'c', icon: 'flame' as const, text: 'Encendida' };
    }
    if (t.done) {
      return { cls: 'g', icon: null, text: 'Meta cumplida' };
    }
    const leftM = Math.max(1, Math.ceil(t.goal - t.today));
    return { cls: 'g', icon: null, text: `Hoy faltan ${fmtMin(leftM, true)}` };
  }
</script>

<div class="rrings" bind:this={root}>
  {#each tracks as t (t.key)}
    {@const c = GOAL_COL[t.key]}
    {@const n = pending(t) ? t.streak - 1 : t.streak}
    {@const st = statusOf(t)}
    {@const usedFree = t.free.filter((f) => f.deadline > rr.today)}
    <div
      class="cv1"
      data-g={t.key}
      style="--c:{c.c};--c2:color-mix(in srgb,{c.c} 78%,black);--ci:{c.ink};--am:var(--warn);--am2:color-mix(in srgb,var(--warn) 65%,white);--amk:color-mix(in srgb,var(--warn) 55%,black);--am-soft:color-mix(in srgb,var(--warn) 22%,var(--tint-base));--hatch:repeating-linear-gradient(-45deg,color-mix(in srgb,var(--warn) 55%,transparent) 0 3px,transparent 3px 7px)"
    >
      <div class="cl">
        <div class="k">{c.full}</div>
        <div class="cn">
          <svg class="flm" width="26" height="32" viewBox="0 0 24 24" fill="none" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d={FLAME_D} />
          </svg>
          <span class="nw"><span class="rnum">{n}</span></span>
          <small>{n === 1 ? 'día' : 'días'}</small>
        </div>
        <span class="cst">
          <span class="tag {st.cls}">
            {#if st.icon === 'warn'}
              <svg class="ico" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><path d={WARN_D} /><path d="M12 9v4M12 16h.01" /></svg>
            {:else if st.icon === 'moon'}
              <svg class="ico" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><path d={MOON_D} /></svg>
            {:else if st.icon === 'flame'}
              <svg class="ico" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><path d={FLAME_D} /></svg>
            {/if}
            {st.text}
          </span>
        </span>
      </div>

      <VelasMini week={t.week} goal={t.goal} pending={pending(t)} />

      <div class="fch">
        <span class="k">libres</span>
        {#each [0, 1] as ci}
          {#if ci < t.tokens}
            <div class="coin" title="Día libre disponible">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                <path d={MOON_D} style="fill:color-mix(in srgb,var(--am) 30%,transparent)" />
              </svg>
            </div>
          {:else}
            {@const fd = usedFree[ci - t.tokens]}
            <div class="coin used" title={fd ? `Vuelve el ${freeDayName(fd.deadline, false)}` : 'Día libre usado'}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                <path d={MOON_D} />
              </svg>
            </div>
          {/if}
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .rrings{position:absolute;inset:0;background:linear-gradient(180deg,var(--face-hi),var(--face));color:var(--ink)}
  .cv1{position:absolute;inset:0;display:flex;align-items:center;padding:0 var(--s-4) 0 18px;gap:14px}
  :global(#stage[data-theme='dark']) .cv1{--ci:color-mix(in srgb,var(--c) 72%,white);--amk:color-mix(in srgb,var(--warn) 78%,white)}
  .cl{width:150px;flex:none;min-width:0}
  .k{font-size:var(--t-xs);font-weight:800;letter-spacing:.08em;text-transform:uppercase;color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .cn{display:flex;align-items:flex-end;gap:var(--s-1);margin:2px 0 var(--s-2)}
  .flm{width:26px;height:32px;margin:0 2px 6px 0;stroke:var(--c);fill:color-mix(in srgb,var(--c) 16%,transparent);transform-origin:50% 90%;
    animation:rflick 1.6s ease-in-out infinite;filter:drop-shadow(0 0 6px color-mix(in srgb,var(--c) 55%,transparent))}
  .nw{display:inline-block;height:48px;overflow:hidden;vertical-align:bottom}
  .rnum{display:block;font-size:48px;font-weight:800;line-height:48px;letter-spacing:-.03em;font-variant-numeric:tabular-nums;
    background:linear-gradient(120deg,var(--ci),var(--c2));-webkit-background-clip:text;background-clip:text;color:transparent}
  .cn small{font-size:var(--t-m);font-weight:800;color:var(--ink-2);padding-bottom:6px}
  .cst{display:block;max-width:100%}
  .tag{display:inline-flex;align-items:center;gap:4px;height:22px;padding:0 var(--s-2);border-radius:var(--r-pill);font-size:var(--t-xs);font-weight:800;
    background:var(--am-soft);color:var(--amk);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--am) 55%,transparent);
    white-space:nowrap;max-width:100%;overflow:hidden;text-overflow:ellipsis}
  .tag.c{background:var(--ci);color:var(--on);box-shadow:none}
  .tag.r{background:var(--bad-f);color:var(--on);box-shadow:none}
  .tag.g{background:color-mix(in srgb,var(--ink-3) 20%,var(--tint-base));color:var(--ink-2);box-shadow:none}
  .ico{display:block;flex:none}
  .fch{display:flex;flex-direction:column;gap:6px;align-items:center;margin-left:auto}
  .fch .k{font-size:9px}
  .coin{width:30px;height:30px;border-radius:50%;display:grid;place-items:center;color:var(--amk);
    background:radial-gradient(circle at 35% 30%,color-mix(in srgb,var(--am2) 30%,var(--tint-base)),var(--am-soft));
    box-shadow:inset 0 0 0 2px var(--am),0 6px 14px -8px color-mix(in srgb,var(--am) 80%,transparent)}
  .coin.used{background:transparent;color:var(--ink-3);box-shadow:none;border:2px dashed color-mix(in srgb,var(--ink-3) 55%,transparent);box-sizing:border-box}
  @keyframes rflick{0%,100%{transform:scale(1,1) rotate(0)}25%{transform:scale(.94,1.08) rotate(-4deg)}50%{transform:scale(1.05,.95) rotate(2deg)}75%{transform:scale(.97,1.05) rotate(4deg)}}
</style>
