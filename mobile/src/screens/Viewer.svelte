<script lang="ts">
  // Visor a pantalla completa: deslizar para pasar de captura, pellizcar o doble toque para ampliar,
  // deslizar hacia abajo para cerrar y un toque para mostrar u ocultar la interfaz.
  import { onDestroy, onMount, tick } from 'svelte';
  import { SystemBars } from '@capacitor/core';
  import { fmt, resText } from '@shared/format';
  import { ICON } from '@shared/icons';
  import Img from '../components/Img.svelte';
  import { MI } from '../lib/icons';
  import { urlOf } from '../lib/img';
  import { app } from '../lib/store.svelte';

  const V = $derived(app.viewer);
  const shots = $derived(V?.shots ?? []);
  let i = $state(app.viewer?.i ?? 0);
  const cur = $derived(shots[i]);
  let ui = $state(true);
  let orig = $state(app.prefs.originals);

  /* ---------- URLs (con caché y precarga de las vecinas) ---------- */
  const urls = $state<Record<string, string>>({});
  const want = (p: string, kind: 'preview' | 'file') => {
    const k = kind + p;
    if (urls[k] !== undefined) return;
    urls[k] = '';
    urlOf(p, kind).then((u) => (urls[k] = u), () => {});
  };
  $effect(() => {
    for (const k of [i, i + 1, i - 1, i + 2]) if (shots[k]) want(shots[k].path, 'preview');
    if (orig && cur) want(cur.path, 'file');
  });
  const srcOf = (k: number) => {
    const s = shots[k];
    if (!s) return '';
    return (orig && k === i && urls['file' + s.path]) || urls['preview' + s.path] || '';
  };
  // Mientras no llega la grande, la miniatura (ya en caché) evita el negro.
  let thumbs = $state<Record<string, string>>({});
  $effect(() => {
    for (const k of [i - 1, i, i + 1]) {
      const s = shots[k];
      if (s && thumbs[s.path] === undefined) { thumbs[s.path] = ''; urlOf(s.path, 'thumb').then((u) => (thumbs[s.path] = u), () => {}); }
    }
  });

  /* ---------- gestos ---------- */
  let stage: HTMLDivElement | undefined = $state();
  let W = $state(window.innerWidth), H = $state(window.innerHeight);
  let dx = $state(0), dy = $state(0), anim = $state(false);
  let s = $state(1), px = $state(0), py = $state(0), zanim = $state(false);
  const pts = new Map<number, { x: number; y: number }>();
  let mode: '' | 'swipe' | 'down' | 'pan' | 'pinch' = '';
  let st = { x: 0, y: 0, t: 0, px: 0, py: 0, s: 1, d: 1, cx: 0, cy: 0 };
  let moved = false;
  let lastTap = { t: 0, x: 0, y: 0 };
  let tapTimer: ReturnType<typeof setTimeout> | undefined;

  const clampPan = () => {
    const mx = ((s - 1) * W) / 2, my = ((s - 1) * H) / 2;
    px = Math.max(-mx, Math.min(mx, px));
    py = Math.max(-my, Math.min(my, py));
  };
  const rel = (p: { x: number; y: number }) => ({ x: p.x - W / 2, y: p.y - H / 2 });
  function pinchInfo() {
    const [a, b] = [...pts.values()];
    return { d: Math.hypot(a.x - b.x, a.y - b.y) || 1, c: rel({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }) };
  }

  function down(e: PointerEvent) {
    stage?.setPointerCapture(e.pointerId);
    pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
    zanim = false;
    if (pts.size === 1) {
      mode = s > 1 ? 'pan' : '';
      moved = false;
      st = { ...st, x: e.clientX, y: e.clientY, t: performance.now(), px, py };
    } else if (pts.size === 2) {
      if (mode === 'swipe' || mode === 'down') { dx = 0; dy = 0; }
      mode = 'pinch';
      moved = true;
      const { d, c } = pinchInfo();
      st = { ...st, s, d, cx: c.x, cy: c.y, px, py };
    }
  }
  function move(e: PointerEvent) {
    if (!pts.has(e.pointerId)) return;
    pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (mode === 'pinch' && pts.size >= 2) {
      const { d, c } = pinchInfo();
      const ns = Math.max(1, Math.min(5, (st.s * d) / st.d));
      const qx = (st.cx - st.px) / st.s, qy = (st.cy - st.py) / st.s;
      s = ns;
      px = c.x - qx * ns;
      py = c.y - qy * ns;
      clampPan();
      return;
    }
    const ddx = e.clientX - st.x, ddy = e.clientY - st.y;
    if (!moved && Math.hypot(ddx, ddy) > 8) moved = true;
    if (!moved) return;
    if (mode === 'pan') { px = st.px + ddx; py = st.py + ddy; clampPan(); return; }
    if (!mode) mode = Math.abs(ddx) >= Math.abs(ddy) ? 'swipe' : ddy > 0 ? 'down' : 'swipe';
    if (mode === 'swipe') {
      const edge = (ddx > 0 && i === 0) || (ddx < 0 && i === shots.length - 1);
      dx = edge ? ddx * 0.3 : ddx;
    } else if (mode === 'down') dy = Math.max(0, ddy);
  }
  function up(e: PointerEvent) {
    if (!pts.delete(e.pointerId)) return;
    if (mode === 'pinch') {
      if (pts.size === 1) {
        const [p] = [...pts.values()];
        mode = 'pan';
        st = { ...st, x: p.x, y: p.y, px, py };
      } else if (pts.size === 0) { if (s < 1.05) resetZoom(); mode = ''; }
      return;
    }
    if (pts.size) return;
    const dt = performance.now() - st.t;
    if (!moved) { tap(e.clientX, e.clientY); mode = ''; return; }
    if (mode === 'swipe') {
      const v = dx / Math.max(1, dt);
      if ((dx < -W * 0.2 || v < -0.5) && i < shots.length - 1) slide(1);
      else if ((dx > W * 0.2 || v > 0.5) && i > 0) slide(-1);
      else { anim = true; dx = 0; setTimeout(() => (anim = false), 260); }
    } else if (mode === 'down') {
      if (dy > 110) close();
      else { anim = true; dy = 0; setTimeout(() => (anim = false), 260); }
    }
    mode = '';
  }
  function tap(x: number, y: number) {
    const now = performance.now();
    if (now - lastTap.t < 300 && Math.hypot(x - lastTap.x, y - lastTap.y) < 40) {
      clearTimeout(tapTimer);
      lastTap.t = 0;
      zanim = true;
      if (s > 1) resetZoom();
      else {
        const c = rel({ x, y });
        s = 2.5;
        px = -c.x * 1.5;
        py = -c.y * 1.5;
        clampPan();
      }
      return;
    }
    lastTap = { t: now, x, y };
    tapTimer = setTimeout(() => (ui = !ui), 300);
  }
  function resetZoom() { s = 1; px = 0; py = 0; }
  function slide(d: number) {
    anim = true;
    dx = -d * W;
    setTimeout(() => {
      anim = false;
      i += d;
      dx = 0;
      resetZoom();
    }, 240);
  }
  function goTo(k: number) {
    if (k === i) return;
    resetZoom();
    i = k;
  }
  function close() { app.back(); }

  /* ---------- tira de miniaturas ---------- */
  let strip: HTMLDivElement | undefined = $state();
  $effect(() => {
    void i;
    tick().then(() => strip?.querySelector<HTMLElement>('.on')?.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' }));
  });

  const onResize = () => { W = window.innerWidth; H = window.innerHeight; };
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowRight' && i < shots.length - 1) slide(1);
    else if (e.key === 'ArrowLeft' && i > 0) slide(-1);
  }
  onMount(() => {
    window.addEventListener('resize', onResize);
    SystemBars.hide().catch(() => {});
  });
  onDestroy(() => {
    window.removeEventListener('resize', onResize);
    clearTimeout(tapTimer);
    SystemBars.show().catch(() => {});
    if (app.viewer) app.viewer = { ...app.viewer, i };
  });
  const game = $derived(cur?.game ?? '');
  const bgUrl = $derived(cur ? thumbs[cur.path] : '');
</script>

<svelte:window onkeydown={key} />

{#if V && cur}
  <div class="vw" class:idle={!ui} style="--fade:{1 - Math.min(0.7, dy / 400)}">
    {#if bgUrl}<div class="vbg" style="background-image:url({bgUrl})"></div>{/if}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="stage" bind:this={stage} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}>
      <div class="track" class:anim style="transform:translate3d({dx}px,{dy}px,0) scale({1 - Math.min(0.25, dy / 1200)})">
        {#each [i - 1, i, i + 1] as k (k)}
          {#if shots[k]}
            <div class="slide" style="left:{(k - i) * 100}%">
              <div class="zoom" class:zanim style={k === i ? `transform:translate3d(${px}px,${py}px,0) scale(${s})` : ''}>
                {#if thumbs[shots[k].path]}<img class="lo" alt="" src={thumbs[shots[k].path]} draggable="false" />{/if}
                {#if srcOf(k)}<img class="hi" alt="" src={srcOf(k)} draggable="false" />{/if}
              </div>
            </div>
          {/if}
        {/each}
      </div>
    </div>

    <div class="vtop">
      <button class="round pill dark" aria-label="Cerrar" onclick={close}>{@html MI.back}</button>
      <div class="vinfo pill">
        <span class="n">{i + 1}<small> / {shots.length}</small></span>
        <span class="dim">{fmt(cur.date)}</span>
        {#if app.favs.has(cur.path)}<i class="vstar">{@html ICON.starY}</i>{/if}
      </div>
      <button class="round pill dark" class:hd={orig} aria-label="Original" onclick={() => (orig = !orig)}>{@html MI.hd}</button>
    </div>
    <div class="vsub">
      {#if game}<b>{game}</b>{:else}<b>{V.title}</b>{/if}
      {#if resText(cur.width, cur.height)}<span>{resText(cur.width, cur.height)}{orig ? ' · original' : ''}</span>{/if}
    </div>

    <div class="vstrip" bind:this={strip}>
      {#each shots as sh, k (sh.path)}
        {#if Math.abs(k - i) < 40}
          <button class="vthumb" class:on={k === i} onclick={() => goTo(k)} style="left:{k * 70}px">
            <Img paths={[sh.path]} />
          </button>
        {/if}
      {/each}
      <span class="spacer" style="left:{shots.length * 70}px"></span>
    </div>
  </div>
{/if}

<style>
  .vw{position:fixed;inset:0;z-index:30;background:#111419;overflow:hidden;touch-action:none}
  .vbg{position:absolute;inset:-60px;background-size:cover;background-position:center;filter:blur(40px) brightness(.5) saturate(1.3);opacity:var(--fade);transition:background-image .3s}
  .stage{position:absolute;inset:0;touch-action:none}
  .track{position:absolute;inset:0;will-change:transform}
  .track.anim{transition:transform .24s cubic-bezier(.25,1,.4,1)}
  .slide{position:absolute;top:0;width:100%;height:100%;overflow:hidden}
  .zoom{position:absolute;inset:0;will-change:transform}
  .zoom.zanim{transition:transform .25s cubic-bezier(.25,1,.4,1)}
  .zoom img{position:absolute;inset:0;width:100%;height:100%;object-fit:contain;pointer-events:none}
  .lo{filter:blur(2px)}
  .vtop{position:absolute;left:0;right:0;top:0;padding:calc(var(--sat) + 10px) 12px 0;display:flex;gap:8px;align-items:center;z-index:3;transition:opacity .25s,transform .25s}
  .dark{background:rgba(30,34,42,.55);box-shadow:inset 0 1px 0 rgba(255,255,255,.25),0 0 0 1px rgba(255,255,255,.18)}
  .dark.hd{background:var(--acc)}
  .vinfo{flex:1;min-width:0;display:flex;align-items:center;gap:10px;height:40px;padding:0 14px;border-radius:999px;font-size:14px;font-weight:800;color:var(--ink);white-space:nowrap;overflow:hidden}
  .vinfo .n{color:var(--acc);font-size:16px}
  .vinfo .n small{color:var(--dim);font-size:12px}
  .vinfo .dim{color:var(--dim);font-weight:700;overflow:hidden;text-overflow:ellipsis}
  .vstar{width:20px;height:20px;flex:none;margin-left:auto}
  .vstar :global(svg){width:20px;height:20px;display:block}
  .vsub{position:absolute;left:0;right:0;bottom:calc(var(--sab) + 84px);text-align:center;z-index:3;color:#fff;text-shadow:0 1px 4px rgba(0,0,0,.7);transition:opacity .25s;pointer-events:none}
  .vsub b{display:block;font-size:15px;font-weight:800}
  .vsub span{font-size:12px;font-weight:700;opacity:.85}
  .vstrip{position:absolute;left:0;right:0;bottom:0;height:calc(var(--sab) + 76px);overflow-x:auto;overflow-y:hidden;z-index:3;transition:opacity .25s;scrollbar-width:none;
    background:linear-gradient(180deg,rgba(255,255,255,.12),rgba(255,255,255,.04));-webkit-backdrop-filter:blur(18px) saturate(1.4);backdrop-filter:blur(18px) saturate(1.4);box-shadow:inset 0 1px 0 rgba(255,255,255,.3)}
  .vstrip::-webkit-scrollbar{display:none}
  .vthumb{position:absolute;top:10px;width:62px;height:40px;margin-left:10px;border-radius:9px;overflow:hidden;opacity:.7;box-shadow:0 0 0 1.5px rgba(255,255,255,.5);transition:transform .2s,opacity .2s}
  .vthumb.on{opacity:1;transform:scale(1.14);box-shadow:0 0 0 2.5px var(--acc),0 0 12px var(--acc)}
  .spacer{position:absolute;top:0;width:calc(50vw);height:1px}
  .vw.idle .vtop,.vw.idle .vsub,.vw.idle .vstrip{opacity:0;pointer-events:none}
</style>
