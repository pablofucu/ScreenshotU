<script lang="ts">
  // Metas diarias (la tarjeta «Lectura de hoy» del visor, a lo ancho del móvil): anillos concéntricos con los
  // minutos de hoy (azul = novelas visuales, rosa = novelas japonesas), la racha de cada una y el mapa de las
  // últimas semanas. Al cumplir una meta «se enciende la racha» (una vez por día y anillo, como en el PC).
  import { onDestroy, onMount, tick } from 'svelte';
  import { goalTrack, localDay, type GoalKey, type GoalTrack } from '@shared/readgoals';
  import { MI } from '../lib/icons';
  import { app } from '../lib/store.svelte';

  const COL: Record<GoalKey, { c: string; soft: string; ink: string; edge: string; name: string; long: string }> = {
    vn: { c: '#00A0D8', soft: '#cfe6f1', ink: '#00739c', edge: '#0073a0', name: 'Novelas', long: 'Novelas visuales' },
    jvn: { c: '#E5487A', soft: '#f6d3df', ink: '#a82b58', edge: '#b02c5c', name: 'Japonés', long: 'Novelas japonesas' },
  };
  const S = 128, SW = 13;

  const now = $derived(app.now);
  const today = $derived(localDay(now));
  const tracks = $derived(
    (['vn', 'jvn'] as GoalKey[])
      .filter((k) => app.lib.platforms.some((p) => p.id === k))
      .map((k) => goalTrack(k, k === 'vn' ? app.cfg.goalVn : app.cfg.goalJvn, app.lib.platforms, app.playing, now)),
  );
  const rings = $derived(tracks.map((t, i) => {
    const r = S / 2 - SW / 2 - 1 - i * SW * 1.15;
    return { t, r, C: 2 * Math.PI * r };
  }));

  const KEY = (k: GoalKey) => `visor.m.goalLit.${k}`;
  const stored = (k: GoalKey) => { try { return Number(localStorage.getItem(KEY(k)) ?? 0); } catch { return 0; } };
  let lit = $state<Partial<Record<GoalKey, number>>>({});
  const pending = (t: GoalTrack) => t.done && lit[t.key] !== today && stored(t.key) !== today;

  let drawn = $state(false);
  let mountedAt = 0;
  let root: HTMLDivElement | undefined = $state();
  onMount(() => {
    mountedAt = Date.now();
    requestAnimationFrame(() => requestAnimationFrame(() => (drawn = true)));
  });
  const scheduled = new Set<string>();
  const timers: ReturnType<typeof setTimeout>[] = [];
  onDestroy(() => timers.forEach(clearTimeout));
  $effect(() => {
    for (const t of tracks) {
      const id = `${t.key}:${today}`;
      if (!pending(t) || scheduled.has(id)) continue;
      scheduled.add(id);
      timers.push(setTimeout(() => light(t.key), Math.max(150, 1500 - (Date.now() - mountedAt))));
    }
  });

  function light(k: GoalKey) {
    if (!root?.isConnected) return;
    const day = today;
    const done = () => {
      lit[k] = day;
      try { localStorage.setItem(KEY(k), String(day)); } catch { /* sin almacenamiento */ }
    };
    const box = root.querySelector<HTMLElement>(`[data-g="${k}"]`);
    const num = box?.querySelector<HTMLElement>('.rnum');
    if (!box || !num) { done(); return; }
    num.animate([{ transform: 'translateY(0)' }, { transform: 'translateY(-34px)' }], { duration: 180, easing: 'ease-in', fill: 'forwards' }).onfinish = async () => {
      done();
      await tick();
      num.getAnimations().forEach((a) => a.cancel());
      num.animate([{ transform: 'translateY(34px)' }, { transform: 'translateY(-5px)' }, { transform: 'translateY(0)' }], { duration: 420, easing: 'ease-out' });
      box.querySelector('.rmed')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.12)' }, { transform: 'scale(.97)' }, { transform: 'scale(1)' }], { duration: 600, easing: 'ease-out' });
      box.querySelector('.rfl')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.8) rotate(-10deg)' }, { transform: 'scale(1)' }], { duration: 600 });
      box.querySelector('.cell.td')?.animate([{ transform: 'scale(2.4)' }, { transform: 'scale(1)' }], { duration: 450, easing: 'ease-out' });
      box.querySelectorAll('.cell.st').forEach((e, i) =>
        e.animate([{ filter: 'brightness(1)', transform: 'scale(1)' }, { filter: 'brightness(1.8)', transform: 'scale(1.5)' }, { filter: 'brightness(1)', transform: 'scale(1)' }], { duration: 400, delay: 120 + i * 55 }));
      try { navigator.vibrate?.([20, 60, 30]); } catch { /* sin vibración */ }
    };
  }

  const fmtMin = (m: number) => {
    const n = Math.floor(m);
    return n < 60 ? `${n} min` : `${Math.floor(n / 60)} h ${String(n % 60).padStart(2, '0')}`;
  };
  function cellBg(t: GoalTrack, v: number, isToday: boolean) {
    if (isToday && pending(t)) v = Math.min(v, 0.6);
    const c = COL[t.key].c;
    return v >= 1 ? c : v >= 0.5 ? c + 'aa' : v > 0 ? c + '55' : '#dde1e7';
  }
  const left = (t: GoalTrack) => Math.max(0, Math.ceil(t.goal - t.today));
</script>

{#if tracks.length}
  <div class="glass panel goals" bind:this={root}>
    <div class="face">
      <div class="head">
        <div class="ring" style="width:{S}px;height:{S}px">
          <svg width={S} height={S} viewBox="0 0 {S} {S}">
            {#each rings as g, i (g.t.key)}
              <circle cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].soft} stroke-width={SW} />
              <circle class="arc" cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].c} stroke-width={SW} stroke-linecap="round"
                stroke-dasharray={g.C} stroke-dashoffset={drawn ? g.C * (1 - g.t.frac) : g.C}
                style="transition-delay:{i * 0.2}s" transform="rotate(-90 {S / 2} {S / 2})" />
            {/each}
          </svg>
          <div class="mins">
            {#each tracks as t (t.key)}<span style="color:{COL[t.key].ink}">{fmtMin(t.today)}</span>{/each}
          </div>
        </div>
        <div class="side">
          {#each tracks as t, i (t.key)}
            {@const c = COL[t.key]}
            {@const n = pending(t) ? t.streak - 1 : t.streak}
            <div class="rmed" data-g={t.key} style="--c:{c.c};--soft:{c.soft};--ink:{c.ink};--edge:{c.edge};--d:{0.3 + i * 0.14}s">
              <i class="rfl" style="color:{c.c}">{@html MI.flame}</i>
              <span class="rnw"><span class="rnum">{n}</span></span>
              <span class="rlab">{c.name}<small>{n > 0 && n >= t.best ? 'récord' : `mejor ${t.best}`}</small></span>
            </div>
          {/each}
        </div>
      </div>
      {#each tracks as t (t.key)}
        {@const c = COL[t.key]}
        <div class="maprow" data-g={t.key} style="--c:{c.c};--edge:{c.edge}">
          <span class="mlab" style="color:{c.ink}">{c.long}<small>{t.done ? '¡Meta cumplida!' : `Faltan ${fmtMin(left(t))}`} · meta {fmtMin(t.goal)}</small></span>
          <div class="rmap">
            {#each t.weeks as w, wi}
              <div>
                {#each w as d, di}
                  <span class="cell" class:st={d.streak && !(d.today && pending(t))} class:td={d.today} class:fu={d.future}
                    style="--k:{wi + di};background:{d.future ? 'transparent' : cellBg(t, d.v, d.today)}"></span>
                {/each}
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .goals{margin:0 14px}
  .goals>.face{padding:14px 14px 12px;background:linear-gradient(180deg,#fbfcfd,#eef1f5)}
  .head{display:flex;align-items:center;gap:12px}
  .ring{position:relative;flex:none}
  .ring svg{display:block;overflow:visible}
  .arc{transition:stroke-dashoffset .9s cubic-bezier(.3,.8,.3,1)}
  .mins{position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;font-size:15px;font-weight:800;line-height:1.25}
  .side{flex:1;display:flex;flex-direction:column;gap:8px;min-width:0}
  .rmed{display:grid;grid-template-columns:20px max-content minmax(0,1fr);align-items:center;column-gap:5px;height:54px;padding:0 10px;border-radius:16px;
    background:linear-gradient(180deg,color-mix(in srgb,var(--soft) 45%,white),var(--soft));
    box-shadow:inset 0 1px 0 #fff,0 0 0 1px color-mix(in srgb,var(--c) 40%,white);animation:rdrop .55s ease-out var(--d) both}
  .rfl{width:20px;height:20px;display:block}
  .rfl :global(svg){width:20px;height:20px;display:block}
  .rnw{height:30px;overflow:hidden}
  .rnum{display:inline-block;font-size:27px;font-weight:800;line-height:30px;color:var(--ink)}
  .rlab{font-size:12.5px;font-weight:800;line-height:1.15;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:#4a5260}
  .rlab small{display:block;font-size:10px;font-weight:800;color:var(--ink);text-transform:uppercase;letter-spacing:.04em}
  .maprow{display:flex;align-items:center;gap:10px;margin-top:10px}
  .mlab{flex:none;width:96px;font-size:11.5px;font-weight:800;line-height:1.2}
  .mlab small{display:block;font-size:10px;font-weight:700;color:#6f7787}
  .rmap{flex:1;display:flex;gap:3px;justify-content:space-between}
  .rmap>div{display:flex;flex-direction:column;gap:3px}
  .cell{width:clamp(7px,calc((100vw - 214px) / 13),13px);aspect-ratio:1;border-radius:3px;animation:rpop .3s calc(.5s + var(--k) * 35ms) both}
  .cell.st{box-shadow:0 0 0 1.5px var(--edge)}
  .cell.td:not(.st){box-shadow:0 0 0 1px var(--c)}
  @keyframes rdrop{0%{opacity:0;transform:translateY(-30px)}70%{opacity:1;transform:translateY(5px)}100%{opacity:1;transform:translateY(0)}}
  @keyframes rpop{from{opacity:0;transform:scale(.2)}to{opacity:1;transform:scale(1)}}
</style>
