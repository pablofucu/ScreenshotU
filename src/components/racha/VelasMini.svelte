<script lang="ts">
  // Mini velas de los últimos 7 días para la tarjeta de Inicio: barras por capas (leído, de más,
  // recuperado y por recuperar), llama encima de los días cumplidos y luna en los días libres.
  import { dayDate, type WeekDay } from '../../lib/readgoals';

  let { week, goal, pending = false }: { week: WeekDay[]; goal: number; pending?: boolean } = $props();

  const H = 78;
  const MAX_H = 74;
  const GOAL_PX = H / 1.6;
  const DI = ['D', 'L', 'M', 'X', 'J', 'V', 'S'];
  const FLAME_D = 'M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235';
  const MOON_D = 'M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z';

  interface Layer {
    cls: 'b' | 'x' | 'xu' | 'p' | 'o';
    bottom: number;
    height: number;
    delay: number;
  }

  const refGoal = $derived(goal > 0 ? goal : (week.find((w) => w.goal > 0)?.goal ?? 0));
  const scale = $derived(refGoal > 0 ? GOAL_PX / refGoal : 0);

  const cols = $derived(week.map((w, i) => {
    const g = w.goal > 0 ? w.goal : refGoal;
    const raw: Array<{ cls: Layer['cls']; h: number; delay: number }> = [];
    if (scale > 0 && g > 0) {
      const bMin = Math.min(w.min, g);
      const exMin = Math.max(0, w.min - g);
      const usedMin = Math.min(exMin, Math.max(0, w.gave));
      const freeExMin = Math.max(0, exMin - usedMin);
      raw.push({ cls: 'b', h: bMin * scale, delay: 0.35 + i * 0.05 });
      if (freeExMin > 0) raw.push({ cls: 'x', h: freeExMin * scale, delay: 0.6 + i * 0.05 });
      if (usedMin > 0) raw.push({ cls: 'xu', h: usedMin * scale, delay: 0.7 + i * 0.05 });
      if (w.free) {
        if (w.free.paid > 0) raw.push({ cls: 'p', h: w.free.paid * scale, delay: 0.95 });
        if (w.free.owe > 0) raw.push({ cls: 'o', h: w.free.owe * scale, delay: 0.85 });
      }
    }
    const layers: Layer[] = [];
    let bot = 0;
    for (const r of raw) {
      const h = Math.min(r.h, MAX_H - bot);
      if (h > 0.5) {
        layers.push({ cls: r.cls, bottom: bot, height: h, delay: r.delay });
        bot += h;
      }
    }
    const topPx = layers.length ? layers[layers.length - 1].bottom + layers[layers.length - 1].height : 0;
    const lit = w.met && !(w.today && pending) && topPx >= 2;
    const moon = !lit && !!w.free;
    const wait = !lit && !w.free && w.today;
    const fd = (1.1 + ((i * 37) % 6) * 0.1).toFixed(2);
    const fdl = (((i * 23) % 10) * 0.1).toFixed(2);
    return {
      w,
      i,
      letter: DI[dayDate(w.day).getUTCDay()],
      layers,
      topPx,
      moonBot: Math.max(topPx, Math.min(MAX_H, w.min * scale)) + 3,
      goalBot: Math.min(MAX_H, g * scale) + 3,
      lit,
      moon,
      wait,
      fd,
      fdl,
    };
  }));
</script>

<div class="mt">
  {#each cols as c (c.w.day)}
    <div class="col" class:td={c.w.today}>
      <div class="mtb" class:met={c.lit} data-lit={c.w.today ? '' : undefined}>
        {#each c.layers as lay, li}
          <i
            class={lay.cls}
            class:lo={li === 0}
            class:hi={li === c.layers.length - 1}
            style="bottom:{lay.bottom.toFixed(1)}px;height:{lay.height.toFixed(1)}px;--d:{lay.delay.toFixed(2)}s"
          ></i>
        {/each}
        {#if c.lit}
          <span class="cnd top-ic on flame" style="bottom:{c.topPx.toFixed(1)}px;--ig:{(c.w.today ? 0.1 : 0.55 + c.i * 0.08).toFixed(2)}s">
            <svg width="12" height="14" viewBox="0 0 24 24" fill="none" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" style="--fd:{c.fd}s;--fdl:{c.fdl}s">
              <path d={FLAME_D} />
            </svg>
          </span>
        {:else if c.moon}
          <span class="top-ic moon on" style="bottom:{c.moonBot.toFixed(1)}px;--ig:{(0.65 + c.i * 0.08).toFixed(2)}s">
            <svg width="11" height="13" viewBox="0 0 24 24" fill="none" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" style="--fdl:{c.fdl}s">
              <path d={MOON_D} />
            </svg>
          </span>
        {:else if c.wait}
          <span class="top-ic wait on" style="bottom:{c.goalBot.toFixed(1)}px;--ig:0.85s">
            <svg width="12" height="14" viewBox="0 0 24 24" fill="none" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
              <path d={FLAME_D} />
            </svg>
          </span>
        {/if}
      </div>
      <span>{c.letter}</span>
    </div>
  {/each}
  <i class="meta" style="bottom:{(4 + 14 + GOAL_PX).toFixed(1)}px"></i>
</div>

<style>
  .mt{position:relative;display:flex;gap:7px;align-items:flex-end;height:100px;padding-top:4px;flex:none}
  .col{display:flex;flex-direction:column;align-items:center;gap:4px}
  .col span{font-size:10px;font-weight:800;line-height:14px;height:14px;color:var(--ink-3)}
  .col.td span{color:var(--ci)}
  .mtb{position:relative;width:20px;height:78px;border-radius:7px;background:color-mix(in srgb,var(--ink-3) 14%,transparent);
    overflow:visible;box-shadow:inset 0 0 0 1px var(--line)}
  .col.td .mtb{box-shadow:inset 0 0 0 2px var(--c);animation:vpulse 2.2s ease-in-out infinite}
  .mtb.met{box-shadow:inset 0 0 0 1px var(--line),inset 0 -18px 18px -12px color-mix(in srgb,var(--c) 35%,transparent)}
  .col.td .mtb.met{box-shadow:inset 0 0 0 2px var(--c),inset 0 -18px 18px -12px color-mix(in srgb,var(--c) 35%,transparent)}
  .mtb>i{position:absolute;left:2px;right:2px;display:block;translate:0 -2px;border:0 solid transparent;border-top-width:1px;
    background-clip:padding-box;border-radius:1px;box-sizing:border-box;animation:vgrow .65s var(--e-soft) var(--d) both}
  .mtb>i.lo{border-top-width:0;border-bottom-left-radius:5px;border-bottom-right-radius:5px}
  .mtb>i.hi{border-top-left-radius:5px;border-top-right-radius:5px}
  .mtb .b{background:linear-gradient(0deg,var(--c),color-mix(in srgb,var(--c) 62%,white))}
  :global(#stage[data-theme='dark']) .mtb .b{background:linear-gradient(0deg,color-mix(in srgb,var(--c) 85%,black),var(--c))}
  .mtb .x{background:color-mix(in srgb,var(--c) 20%,var(--tint-base));box-shadow:inset 0 0 0 1.5px color-mix(in srgb,var(--c) 70%,transparent)}
  .mtb .xu{background:color-mix(in srgb,var(--am) 30%,var(--tint-base));box-shadow:inset 0 0 0 1.5px var(--am)}
  .mtb .p{background:linear-gradient(0deg,var(--am),var(--am2))}
  .mtb .o{background:var(--hatch);box-shadow:inset 0 0 0 1.5px var(--am)}
  .meta{position:absolute;left:-4px;right:-4px;height:0;border-top:2px dashed color-mix(in srgb,var(--ink-2) 60%,transparent);z-index:2;pointer-events:none}
  .top-ic{position:absolute;left:50%;z-index:5;pointer-events:none;transform:translateX(-50%);transform-origin:50% 100%}
  .top-ic svg{display:block;overflow:visible}
  .cnd svg{stroke:var(--c);fill:color-mix(in srgb,var(--c) 28%,var(--tint-base));transform-origin:50% 95%;
    animation:vflick var(--fd,1.4s) ease-in-out var(--fdl,0s) infinite;filter:drop-shadow(0 0 4px color-mix(in srgb,var(--c) 70%,transparent))}
  .top-ic.moon svg{stroke:var(--am);fill:color-mix(in srgb,var(--am) 28%,var(--tint-base));
    animation:vbob 3s ease-in-out var(--fdl,0s) infinite;filter:drop-shadow(0 0 4px color-mix(in srgb,var(--am) 55%,transparent))}
  .top-ic.wait svg{stroke:var(--c);fill:none;opacity:.55;stroke-dasharray:3 3;animation:vbreath 2.4s ease-in-out infinite}
  .top-ic.on{animation:vign .55s cubic-bezier(.3,1.7,.5,1) var(--ig,0s) both}
  @keyframes vgrow{from{height:0}}
  @keyframes vign{0%{opacity:0;transform:translateX(-50%) scale(0)}100%{opacity:1;transform:translateX(-50%) scale(1)}}
  @keyframes vflick{0%,100%{transform:scale(1,1) rotate(0)}25%{transform:scale(.94,1.08) rotate(-4deg)}50%{transform:scale(1.05,.95) rotate(2deg)}75%{transform:scale(.97,1.05) rotate(4deg)}}
  @keyframes vbob{0%,100%{transform:translateY(0)}50%{transform:translateY(-3px)}}
  @keyframes vbreath{0%,100%{opacity:.35;transform:scale(.94)}50%{opacity:.7;transform:scale(1.04)}}
  @keyframes vpulse{0%,100%{box-shadow:inset 0 0 0 2px var(--c),0 0 0 0 color-mix(in srgb,var(--c) 50%,transparent)}50%{box-shadow:inset 0 0 0 2px var(--c),0 0 0 6px color-mix(in srgb,var(--c) 0%,transparent)}}
</style>
