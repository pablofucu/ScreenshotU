<script lang="ts">
  // Hucha en miniatura de la tarjeta de Inicio: ocupa el hueco del mapa pequeño de la categoría (7×8 + 6×2 = 68 px de
  // alto) mientras tiene un rescate pendiente. Una casilla por minuto que faltó; macizas las de días anteriores,
  // claras las de hoy y rayadas las que faltan. El color (.r-vn / .r-jv) viene de la clase global de app.css.
  import { rescueCls, type Rescue } from '../lib/readgoals';
  import { rescueClose, rescueLeft } from '../lib/readrings.svelte';

  let { r, now }: { r: Rescue; now: number } = $props();
</script>

<div class="mini {rescueCls(r.key)}" title="Rescate pendiente: {r.got} de {r.need} min">
  <div class="mjl"><span>rescate</span><span>{r.got}/{r.need} min</span></div>
  <div class="mjar">
    {#each Array(r.need) as _, i}<i class:d1={i < r.gotBefore} class:d2={i >= r.gotBefore && i < r.got}></i>{/each}
  </div>
  <div class="mjt">cierra en {rescueLeft(r, now)} · {rescueClose(r)}</div>
</div>

<style>
  .mini{height:68px;display:flex;flex-direction:column;justify-content:center;gap:5px;padding:0 2px;min-width:0}
  .mjar{display:flex;gap:2px;height:12px}
  .mjar i{flex:1;border-radius:3px;background:repeating-linear-gradient(135deg,#e3e6eb 0 2px,#eef0f3 2px 4px)}
  .mjar i.d1{background:var(--rc)}
  .mjar i.d2{background:color-mix(in srgb,var(--rc) 60%,white)}
  .mjl{font-size:11px;font-weight:800;color:var(--rink);display:flex;justify-content:space-between;white-space:nowrap}
  .mjl span:first-child{text-transform:uppercase;letter-spacing:.04em}
  .mjt{font-size:10.5px;font-weight:700;color:#8a92a0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
</style>
