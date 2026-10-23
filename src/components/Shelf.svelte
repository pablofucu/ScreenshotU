<script lang="ts" module>
  /** Una portada de la balda. `k` y `act` son el foco y la acción de la app (`tit12` / `tit:12`). */
  export interface ShelfItem {
    k: string;
    act: string;
    path: string | null;
    name: string;
    /** Texto bajo la portada (en el borde de color) y, si hay nota, sus estrellas. */
    label?: string;
    stars?: number;
    hidden?: boolean;
    /** Glifo SVG (blanco) en vez de portada: baldosas de acción como «Nueva colección». */
    icon?: string;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import Thumb from './Thumb.svelte';
  import { SHELF_CH, SHELF_CW, SHELF_PITCH } from '../lib/milista.svelte';

  // «Pestaña sobre balda» (boceto M1): pestaña de cristal con un solo color (claro → color → oscuro), título y datos,
  // y debajo la balda con las portadas y un borde de color con una etiqueta bajo cada una.
  // Ranuras: `extra` (línea de tiempo bajo los datos) y `end` (botón a la derecha de la pestaña). Las colecciones
  // pueden teñir la pestaña (`tabStyle`: un fondo propio, `aurora`: manchas de color flotando por dentro) y cambiar
  // la tinta del texto (`ink`, para pestañas claras como el amarillo).
  let {
    acc, title, sub = '', items, focus = -1, from = 0, visible = 9, top = 0, dim = false, ink = '#fff', tabStyle = '', aurora = [], extra, end,
  }: {
    acc: string;
    title: string;
    sub?: string;
    items: ShelfItem[];
    /** Posición de la portada enfocada en `items` (-1 = ninguna). */
    focus?: number;
    /** Primera portada visible. */
    from?: number;
    visible?: number;
    top?: number;
    dim?: boolean;
    ink?: string;
    tabStyle?: string;
    aurora?: string[];
    extra?: Snippet;
    end?: Snippet;
  } = $props();

  // Ventana virtual: solo se pintan las portadas visibles y un par de vecinas a cada lado.
  const lo = $derived(Math.max(0, from - 2));
  const hi = $derived(Math.min(items.length, from + visible + 2));
  const win = $derived(Array.from({ length: Math.max(0, hi - lo) }, (_, k) => lo + k));
  const more = $derived(Math.max(0, items.length - (from + visible)));
</script>

<div class="shelf" class:dim style="top:{top}px;--acc:{acc};--ink:{ink}">
  <div class="stab" class:tint={aurora.length > 0 || !!tabStyle} class:hasaur={aurora.length > 0} style={tabStyle}>
    {#if aurora.length}
      <div class="aur">{#each aurora as c, k}<b style="left:{8 + k * 21}%;background:{c}"></b>{/each}</div>
    {/if}
    <h3>{title}</h3>
    {#if sub || extra}
      <div class="sinfo">
        {#if sub}<span class="smeta">{sub}</span>{/if}
        {#if extra}{@render extra()}{/if}
      </div>
    {/if}
    {#if end}{@render end()}{/if}
  </div>
  <div class="sboard">
    <div class="sview">
      <div class="rtrack" style="transform:translateX({-from * SHELF_PITCH}px)">
        {#each win as p (items[p].k)}
          {@const it = items[p]}
          <div class="glass scov fx" class:f={focus === p} class:hid={it.hidden} data-k={it.k} data-act={it.act}
            style="left:{p * SHELF_PITCH}px;width:{SHELF_CW}px;height:{SHELF_CH}px">
            <div class="face">
              {#if it.path}<Thumb path={it.path} />{:else if it.icon}<div class="sicon">{@html it.icon}</div>{:else}<div class="noart"><b>{it.name}</b></div>{/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
    <div class="slip">
      <div class="rtrack" style="transform:translateX({-from * SHELF_PITCH}px)">
        {#each win as p (items[p].k)}
          {@const it = items[p]}
          <span class="slab" class:on={focus === p} style="left:{p * SHELF_PITCH}px;width:{SHELF_CW}px">
            {#if it.label || it.stars}<s>{#if it.label}<b>{it.label}</b>{/if}{#if it.stars}<em>{'★'.repeat(it.stars)}</em>{/if}</s>{/if}
          </span>
        {/each}
      </div>
    </div>
    {#if from > 0}<i class="sm l">‹</i>{/if}
    {#if more > 0}<i class="sm r">+{more} ›</i>{/if}
  </div>
</div>

<style>
  .shelf{position:absolute;left:56px;width:1168px;transition:top .3s cubic-bezier(.25,1,.4,1),opacity .25s;
    --hi:color-mix(in srgb,var(--acc) 35%,white);--lo:color-mix(in srgb,var(--acc) 70%,black)}
  .shelf.dim{opacity:.72}
  .stab{display:inline-flex;align-items:center;gap:22px;max-width:100%;padding:8px 10px 8px 22px;border-radius:20px 20px 6px 20px;margin-bottom:-6px;position:relative;z-index:3;min-height:50px;box-sizing:border-box;
    background:linear-gradient(180deg,var(--hi),var(--acc) 62%,var(--lo));
    box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),inset 0 -2px 6px -2px rgba(0,0,0,.25),0 0 0 1px var(--lo),0 10px 20px -10px var(--acc)}
  .stab h3{margin:0;font-size:27px;font-weight:800;color:var(--ink);text-shadow:0 1px 2px rgba(0,0,0,.25);white-space:nowrap;line-height:1.1;max-width:430px;overflow:hidden;text-overflow:ellipsis}
  .sinfo{display:flex;flex-direction:column;gap:5px;padding-right:12px}
  .smeta{font-size:15px;font-weight:600;color:var(--ink);white-space:nowrap;text-shadow:0 1px 2px rgba(0,0,0,.2)}
  /* Pestañas teñidas (colecciones): el texto lleva una sombra suave para leerse sobre cualquier color. */
  .stab.tint h3,.stab.tint .smeta{text-shadow:0 1px 3px rgba(0,0,0,.4)}
  .stab.hasaur{overflow:hidden}
  .aur{position:absolute;inset:0;pointer-events:none}
  .aur b{position:absolute;top:-30%;width:34%;height:160%;border-radius:50%;filter:blur(22px);opacity:.55}
  .stab>:global(*:not(.aur)){position:relative}
  .sicon{position:absolute;inset:0;display:grid;place-items:center;background:linear-gradient(160deg,color-mix(in srgb,var(--acc) 40%,white),color-mix(in srgb,var(--acc) 85%,#445));filter:saturate(.9)}
  .sicon :global(svg){width:44%;height:44%;filter:drop-shadow(0 2px 3px rgba(0,0,0,.3))}
  .sboard{position:relative;height:179px;border-radius:6px 22px 22px 22px;overflow:hidden;
    background:linear-gradient(180deg,rgba(255,255,255,.62),rgba(255,255,255,.26));
    -webkit-backdrop-filter:blur(14px) saturate(1.5);backdrop-filter:blur(14px) saturate(1.5);
    box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.2),0 14px 22px -16px rgba(40,50,70,.45)}
  .sview{position:absolute;left:18px;right:18px;top:16px;height:139px}
  .rtrack{position:absolute;left:0;top:0;height:100%;transition:transform .3s cubic-bezier(.25,1,.4,1)}
  .scov{--r:16px;--bw:6px;top:0}
  .scov .face{border-radius:11px}
  .scov.hid{opacity:.55}
  .scov :global(.noart){padding:6px}
  .scov :global(.noart b){font-size:12px;line-height:1.15;overflow-wrap:anywhere}
  .slip{position:absolute;left:0;right:0;bottom:0;height:24px;
    background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 22%,white),color-mix(in srgb,var(--acc) 48%,#dfe3e9));
    box-shadow:inset 0 2px 0 rgba(255,255,255,.9),0 -8px 14px -10px rgba(40,50,70,.4);overflow:hidden}
  .slip .rtrack{left:18px}
  .slab{position:absolute;top:0;height:100%;display:flex;align-items:center;justify-content:center;gap:5px;white-space:nowrap;
    font-size:12px;font-weight:800;letter-spacing:.03em;color:color-mix(in srgb,var(--acc) 55%,#1e2433);transition:color .2s}
  .slab em{font-style:normal;font-size:10px;letter-spacing:-.5px;color:color-mix(in srgb,var(--acc) 80%,#8a5a00)}
  .slab s{text-decoration:none;display:flex;align-items:center;gap:5px;padding:1px 8px;border-radius:999px}
  .slab.on s{background:var(--lo);color:#fff;text-shadow:0 1px 1px rgba(0,0,0,.3)}
  .slab.on em{color:#FFD84A}
  .sm{position:absolute;top:56px;z-index:4;font-style:normal;font-size:15px;font-weight:800;color:#5f6776;pointer-events:none;
    padding:2px 8px;border-radius:999px;background:rgba(255,255,255,.7);box-shadow:0 0 0 1px rgba(95,110,135,.2)}
  .sm.l{left:4px}
  .sm.r{right:4px}
  @media (prefers-reduced-motion: reduce){.shelf,.rtrack{transition:none}}
</style>
