<script lang="ts">
  // Tarjeta «Lectura de hoy» de Inicio: anillos concéntricos con los minutos de hoy (azul = novelas visuales,
  // rosa = novelas japonesas), una medalla con la racha de cada una y el mapa de las últimas semanas.
  // Al entrar se dibuja en cascada; al cumplir una meta «se enciende la racha» (una vez por día y anillo).
  import { onDestroy, onMount, tick } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { goalTrack, localDay, type GoalKey, type GoalTrack } from '../lib/readgoals';
  import { blip } from '../lib/sound';

  const COL: Record<GoalKey, { c: string; soft: string; ink: string; edge: string; name: string }> = {
    vn: { c: '#00A0D8', soft: '#cfe6f1', ink: '#00739c', edge: '#0073a0', name: 'novelas' },
    jvn: { c: '#E5487A', soft: '#f6d3df', ink: '#a82b58', edge: '#b02c5c', name: 'japonés' },
  };
  const S = 112, SW = 11;

  const now = $derived(app.now.getTime());
  const today = $derived(localDay(now));
  const tracks = $derived(
    (['vn', 'jvn'] as GoalKey[])
      .filter((k) => app.lib.platforms.some((p) => p.id === k))
      .map((k) => goalTrack(k, k === 'vn' ? app.config.goalVn : app.config.goalJvn, app.lib.platforms, app.playing, now)),
  );
  const rings = $derived(tracks.map((t, i) => {
    const r = S / 2 - SW / 2 - 1 - i * SW * 1.15;
    return { t, r, C: 2 * Math.PI * r };
  }));

  // Racha ya celebrada (día local) por anillo: se guarda para no repetirla al volver a Inicio.
  const KEY = (k: GoalKey) => `visor.goalLit.${k}`;
  const stored = (k: GoalKey) => { try { return Number(localStorage.getItem(KEY(k)) ?? 0); } catch { return 0; } };
  let lit = $state<Partial<Record<GoalKey, number>>>({});
  /** Meta cumplida hoy cuya racha aún no se ha «encendido»: se enseña el día anterior hasta la animación. */
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
      // Tras la cascada de entrada si acaba de abrirse Inicio.
      const wait = Math.max(150, 1500 - (Date.now() - mountedAt));
      timers.push(setTimeout(() => light(t.key), wait));
    }
  });

  /** «La racha se enciende»: el número gira, la casilla de hoy se rellena y un brillo recorre la racha. */
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
    num.animate([{ transform: 'translateY(0)' }, { transform: 'translateY(-30px)' }], { duration: 180, easing: 'ease-in', fill: 'forwards' }).onfinish = async () => {
      done();
      await tick();
      num.getAnimations().forEach((a) => a.cancel());
      num.animate([{ transform: 'translateY(30px)' }, { transform: 'translateY(-5px)' }, { transform: 'translateY(0)' }], { duration: 420, easing: 'ease-out' });
      box.querySelector('.rmed')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.2)' }, { transform: 'scale(.96)' }, { transform: 'scale(1)' }], { duration: 600, easing: 'ease-out' });
      box.querySelector('.rfl')?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.8) rotate(-10deg)' }, { transform: 'scale(1)' }], { duration: 600 });
      box.querySelector('.cell.td')?.animate([{ transform: 'scale(2.4)' }, { transform: 'scale(1)' }], { duration: 450, easing: 'ease-out' });
      box.querySelectorAll('.cell.st').forEach((e, i) =>
        e.animate([{ filter: 'brightness(1)', transform: 'scale(1)' }, { filter: 'brightness(1.8)', transform: 'scale(1.5)' }, { filter: 'brightness(1)', transform: 'scale(1)' }], { duration: 400, delay: 120 + i * 55 }));
      blip(1320);
      setTimeout(() => blip(1760), 110);
    };
  }

  const fmtMin = (m: number) => {
    const n = Math.floor(m);
    return n < 60 ? `${n}m` : `${Math.floor(n / 60)}h ${String(n % 60).padStart(2, '0')}m`;
  };
  function cellBg(t: GoalTrack, v: number, isToday: boolean) {
    if (isToday && pending(t)) v = Math.min(v, 0.6);
    const c = COL[t.key].c;
    return v >= 1 ? c : v >= 0.5 ? c + 'aa' : v > 0 ? c + '55' : '#dde1e7';
  }
</script>

<div class="rrings" bind:this={root}>
  <div class="rr-ring" style="width:{S}px;height:{S}px">
    <svg width={S} height={S} viewBox="0 0 {S} {S}">
      {#each rings as g, i (g.t.key)}
        <circle cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].soft} stroke-width={SW} />
        <circle class="arc" cx={S / 2} cy={S / 2} r={g.r} fill="none" stroke={COL[g.t.key].c} stroke-width={SW} stroke-linecap="round"
          stroke-dasharray={g.C} stroke-dashoffset={drawn ? g.C * (1 - Math.min(1, g.t.today / g.t.goal)) : g.C}
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
      <div class="rr-col" data-g={t.key} style="--c:{c.c};--soft:{c.soft};--ink:{c.ink};--edge:{c.edge};--d:{0.3 + i * 0.14}s">
        <div class="rmed">
          <svg class="rfl" viewBox="0 0 24 24" fill="none" stroke={c.c} stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" /></svg>
          <span class="rnw"><span class="rnum">{n}</span></span>
          <span class="rlab" title="{n} {n === 1 ? 'día seguido' : 'días seguidos'}">{c.name}<small>{n > 0 && n >= t.best ? 'récord' : `mejor ${t.best}`}</small></span>
        </div>
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
  .rmap{display:flex;gap:2px}
  .rmap>div{display:flex;flex-direction:column;gap:2px}
  .cell{width:8px;height:8px;border-radius:2px;animation:rpop .3s calc(.5s + var(--k) * 35ms) both}
  .cell.st{box-shadow:0 0 0 1.5px var(--edge)}
  .cell.td:not(.st){box-shadow:0 0 0 1px var(--c)}
  @keyframes rdrop{0%{opacity:0;transform:translateY(-30px)}70%{opacity:1;transform:translateY(5px)}100%{opacity:1;transform:translateY(0)}}
  @keyframes rpop{from{opacity:0;transform:scale(.2)}to{opacity:1;transform:scale(1)}}
</style>
