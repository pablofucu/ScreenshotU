<script lang="ts">
  // Hucha en miniatura de la tarjeta de Inicio: ocupa el hueco del mapa pequeño de la categoría (7×8 + 6×2 = 68 px de
  // alto, ~130 px de ancho) mientras tiene un rescate pendiente. La barra se parte en casillas legibles: una por minuto
  // si faltan pocos, y si no, de 5 o 10 min (como mucho 12); cada casilla se rellena con lo de días anteriores (macizo)
  // y lo de hoy (claro). El color (.r-vn / .r-jv) viene de la clase global de app.css.
  import { rescueCls, type Rescue } from '../lib/readgoals';
  import { rescueClose, rescueLeft } from '../lib/readrings.svelte';

  import { onMount } from 'svelte';

  let { r, now }: { r: Rescue; now: number } = $props();

  // Al entrar, el contador sube desde 0 hasta lo reunido (tras caer la tarjeta).
  let shown = $state(0);
  onMount(() => {
    let raf = 0;
    const t0 = performance.now() + 700, dur = 700;
    const step = (t: number) => {
      const k = Math.max(0, Math.min(1, (t - t0) / dur));
      shown = Math.round(r.got * (1 - (1 - k) ** 3));
      if (k < 1) raf = requestAnimationFrame(step); else shown = r.got;
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });

  const need = $derived(Math.max(1, r.need));
  const unit = $derived(need <= 12 ? 1 : need <= 60 ? 5 : 10 * Math.ceil(need / 120));
  /** Casillas: parte rellena de cada una con lo de días anteriores (a) y con todo lo reunido (b), en %. */
  const cells = $derived(Array.from({ length: Math.ceil(need / unit) }, (_, i) => {
    const lo = i * unit, w = Math.min(need, lo + unit) - lo;
    const pct = (v: number) => Math.round(Math.max(0, Math.min(1, (v - lo) / w)) * 100);
    return { a: pct(r.gotBefore), b: pct(r.got) };
  }));
</script>

<div class="hucha {rescueCls(r.key)}" title="Rescate pendiente: {r.got} de {r.need} min · cierra en {rescueLeft(r, now)} ({rescueClose(r)})">
  <div class="mh"><span class="mk">Rescate</span><span class="mn"><b>{shown}</b>/{r.need}<small> min</small></span></div>
  <div class="mbar">
    {#each cells as c, i}<i style="--a:{c.a}%;--b:{c.b}%;--k:{i}"><s class="fb"></s><s class="fa"></s></i>{/each}
  </div>
  <div class="mf"><span>quedan <b>{rescueLeft(r, now)}</b></span><span>{rescueClose(r)}</span></div>
</div>

<style>
  .hucha{height:68px;box-sizing:border-box;display:flex;flex-direction:column;justify-content:space-between;gap:4px;
    padding:6px 10px;border-radius:12px;min-width:0;width:100%;
    background:linear-gradient(180deg,color-mix(in srgb,var(--rsoft) 40%,white),var(--rsoft));
    box-shadow:inset 0 1px 0 #fff,0 0 0 1px color-mix(in srgb,var(--rc) 35%,white);
    animation:hdrop .55s ease-out .55s both}
  .mh,.mf{display:flex;justify-content:space-between;align-items:baseline;gap:6px;white-space:nowrap;min-width:0}
  .mh{line-height:16px}
  .mk{font-size:9.5px;font-weight:800;text-transform:uppercase;letter-spacing:.06em;color:var(--rink)}
  .mn{font-size:11px;font-weight:800;color:color-mix(in srgb,var(--rink) 70%,#8a92a0);font-variant-numeric:tabular-nums}
  .mn b{font-size:15px;color:var(--rink)}
  .mn small{font-size:9.5px}
  .mbar{display:flex;gap:3px;height:10px;flex:none}
  .mbar i{position:relative;overflow:hidden;flex:1;min-width:0;border-radius:3px;background:#fff;
    box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--rc) 30%,white);
    animation:hpop .3s calc(.9s + var(--k) * 60ms) both}
  .mbar s{position:absolute;left:0;top:0;bottom:0;transform-origin:left;
    animation:hfill .6s cubic-bezier(.3,.8,.3,1) calc(1.1s + var(--k) * 90ms) both}
  .fb{width:var(--b);background:color-mix(in srgb,var(--rc) 55%,white)}
  .fa{width:var(--a);background:var(--rc)}
  .mf{line-height:13px;font-size:10px;font-weight:700;color:#7a8291}
  .mf>span{overflow:hidden;text-overflow:ellipsis}
  .mf b{color:var(--rink);font-weight:800}
  @keyframes hdrop{0%{opacity:0;transform:translateY(-24px) scale(.96)}70%{opacity:1;transform:translateY(4px) scale(1)}100%{opacity:1;transform:none}}
  @keyframes hpop{from{opacity:0;transform:scale(.4)}to{opacity:1;transform:scale(1)}}
  @keyframes hfill{from{transform:scaleX(0)}to{transform:scaleX(1)}}
</style>
