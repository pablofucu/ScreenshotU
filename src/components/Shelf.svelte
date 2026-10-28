<script lang="ts" module>
  /** Una portada de la balda. `k` y `act` son el foco y la acción de la app (`tit12` / `tit:12`). */
  export interface ShelfItem {
    k: string;
    act: string;
    path: string | null;
    name: string;
    /** Texto bajo la portada (en el borde de etiquetas) y, si hay nota, sus estrellas. */
    label?: string;
    stars?: number;
    hidden?: boolean;
    /** Glifo SVG (blanco) en vez de portada: baldosas de acción como «Nueva colección» (llevan el nombre debajo). */
    icon?: string;
    /** Número de orden del juego dentro del año (1..N). */
    n?: number;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import Thumb from './Thumb.svelte';
  import { SHELF_CH, SHELF_CW, SHELF_PITCH, SHELF_VIEW_W } from '../lib/milista.svelte';

  // «Balda de cristal» (variante V2 · Rótulo teñido): un rótulo en píldora con el icono de menú, el título, un chip y los
  // datos, y debajo la balda de cristal (neutra) con las portadas y un borde de etiquetas. El color de la estantería vive
  // en el rótulo: teñido suave en reposo (con su aurora) y encendido al enfocar la estantería.
  // Ranuras: `extra` (línea de tiempo tras los datos) y `end` (botón a la derecha del rótulo). Las colecciones pueden
  // teñir el rótulo (`tabStyle`: un degradado propio, `aurora`: manchas de color flotando por dentro) y cambiar la tinta
  // del texto (`ink`, para colores claros como el amarillo).
  let {
    acc, title, sub = '', items, focus = -1, from = 0, visible = 9, top = 0, dim = false, ink = '#fff', tabStyle = '', aurora = [], icon = '', chip = '', extra, end,
    positions, dividers = [], monthGroups,
  }: {
    acc: string;
    title: string;
    sub?: string;
    items: ShelfItem[];
    /** Posición de la portada enfocada en `items` (-1 = ninguna). */
    focus?: number;
    /** Primera portada visible (o desplazamiento en píxeles si se usan `positions`). */
    from?: number;
    visible?: number;
    top?: number;
    /** Estantería sin foco: rótulo teñido en vez de encendido. */
    dim?: boolean;
    ink?: string;
    tabStyle?: string;
    aurora?: string[];
    /** Glifo SVG del icono de color del rótulo. */
    icon?: string;
    /** Chip de color con el recuento (solo en reposo). */
    chip?: string;
    extra?: Snippet;
    end?: Snippet;
    /** Posiciones horizontales calculadas para cada portada (pestaña «Terminados»). */
    positions?: number[];
    /** Divisores verticales centrados entre meses. */
    dividers?: number[];
    /** Grupos de meses para la tira inferior de etiquetas («slip»). */
    monthGroups?: Array<{ label: string; x0: number; x1: number; has: boolean }>;
  } = $props();

  const scrollX = $derived(positions ? from : from * SHELF_PITCH);

  // Ventana virtual: solo se pintan las portadas visibles y un par de vecinas a cada lado.
  const lo = $derived(Math.max(0, from - 2));
  const hi = $derived(Math.min(items.length, from + visible + 2));
  const win = $derived.by(() => {
    if (positions) {
      return items
        .map((_, p) => p)
        .filter((p) => positions[p] + SHELF_CW >= scrollX - 250 && positions[p] <= scrollX + SHELF_VIEW_W + 250);
    }
    return Array.from({ length: Math.max(0, hi - lo) }, (_, k) => lo + k);
  });
  const more = $derived(
    positions
      ? items.filter((_, p) => positions[p] + SHELF_CW > scrollX + SHELF_VIEW_W).length
      : Math.max(0, items.length - (from + visible)),
  );
  const hasLeft = $derived(positions ? scrollX > 5 : from > 0);
  const hasOrd = $derived(items.some((it) => it.n !== undefined));
</script>

<div class="shelf" class:dim class:dk={ink !== '#fff'} class:with-ord={hasOrd} style="top:{top}px;--acc:{acc};--ink:{ink}">
  <div class="bsh" class:lit={!dim} class:soft={dim} class:hasaur={aurora.length > 0 || !!tabStyle}>
    {#if aurora.length}
      <div class="aur">{#each aurora as c, k}<b style="left:{4 + k * 20}%;background:{c}"></b>{/each}</div>
    {:else if tabStyle}
      <div class="aur multi" style={tabStyle}></div>
    {/if}
    {#if icon}<span class="mic">{@html icon}</span>{/if}
    <h3>{title}</h3>
    {#if chip && dim}<span class="chip">{chip}</span>{/if}
    {#if sub}<span class="m">{sub}</span>{/if}
    {#if extra}{@render extra()}{/if}
    {#if end}{@render end()}{/if}
  </div>
  <div class="glass bsb">
    <div class="face">
      <div class="rtrack view" style="transform:translateX({-scrollX}px)">
        {#if dividers.length}
          {#each dividers as dx}
            <span class="mdiv" style="left:{dx}px"></span>
          {/each}
        {/if}
        {#each win as p (items[p].k)}
          {@const it = items[p]}
          <div class="glass scov fx" class:addtile={!it.path && !!it.icon} class:f={focus === p} class:hid={it.hidden} data-k={it.k} data-act={it.act}
            style="left:{positions ? positions[p] : p * SHELF_PITCH}px;width:{SHELF_CW}px;height:{SHELF_CH}px">
            {#if it.n !== undefined}<span class="ord">{it.n}</span>{/if}
            <div class="face">
              {#if it.path}<Thumb path={it.path} />{:else if it.icon}<div class="addin"><span>{@html it.icon}</span><b>{it.name}</b></div>{:else}<div class="noart"><b>{it.name}</b></div>{/if}
            </div>
          </div>
        {/each}
      </div>
      <div class="slip">
        <div class="rtrack" style="transform:translateX({-scrollX}px)">
          {#if monthGroups}
            {#each monthGroups as g}
              <span class="slab mlab" class:on={g.has} style="left:{g.x0}px;width:{g.x1 - g.x0}px">
                <s>{g.label}</s>
              </span>
            {/each}
          {:else}
            {#each win as p (items[p].k)}
              {@const it = items[p]}
              <span class="slab" class:on={focus === p} style="left:{positions ? positions[p] : p * SHELF_PITCH}px;width:{SHELF_CW}px">
                {#if it.label || it.stars}<s>{#if it.label}<b>{it.label}</b>{/if}{#if it.stars}<em>{'★'.repeat(it.stars)}</em>{/if}</s>{/if}
              </span>
            {/each}
          {/if}
        </div>
      </div>
      {#if hasLeft}<i class="sm l">‹</i>{/if}
      {#if more > 0}<i class="sm r">+{more} ›</i>{/if}
    </div>
  </div>
</div>

<style>
  /* Línea de 96 px: el icono del rótulo, la primera portada, la cabecera y la barra de pestañas empiezan en x=96. */
  .shelf{position:absolute;left:80px;width:1120px;transition:top .3s cubic-bezier(.25,1,.4,1),opacity .25s;
    --hi:color-mix(in srgb,var(--acc) 35%,white);--lo:color-mix(in srgb,var(--acc) 70%,black);--tsh:0 1px 2px rgba(0,0,0,.28)}
  .shelf.dk{--tsh:none}
  .shelf.dim .scov{opacity:.86}
  .shelf.dim .scov.hid{opacity:.5}

  /* ---- Rótulo: píldora de cristal, teñida en reposo y encendida con el foco ---- */
  .bsh{position:relative;display:inline-flex;align-items:center;gap:14px;height:40px;max-width:100%;box-sizing:border-box;padding:0 6px 0 8px;margin:0 0 6px 8px;border-radius:999px;white-space:nowrap;z-index:3;isolation:isolate}
  .bsh.hasaur{overflow:hidden}
  .bsh>:global(*:not(.aur)){position:relative}
  .bsh>:global(*:last-child:not(.aur)){margin-right:8px}
  .bsh h3{margin:0;font-size:22px;font-weight:800;color:#3a414e;line-height:1;max-width:430px;overflow:hidden;text-overflow:ellipsis;min-width:0}
  .bsh .m{font-size:15px;font-weight:600;color:#5f6776}
  .bsh :global(.hint){height:32px;font-size:15px;background:rgba(255,255,255,.9);box-shadow:none;text-shadow:none;color:#4a5260}
  .chip{font-size:14px;font-weight:800;padding:3px 11px;border-radius:999px;color:var(--ink);background:var(--acc);box-shadow:inset 0 1px 0 rgba(255,255,255,.45),0 0 0 1px color-mix(in srgb,var(--acc) 70%,black)}
  .aur{position:absolute;inset:0;pointer-events:none;overflow:hidden;border-radius:inherit}
  .aur b{position:absolute;top:-30%;width:34%;height:160%;border-radius:50%;filter:blur(16px)}
  .aur.multi{opacity:.5}
  .soft .aur{opacity:.38}
  .lit .aur{opacity:.5}
  .soft{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 22%,white),color-mix(in srgb,var(--acc) 38%,#eef1f5));
    box-shadow:inset 0 1px 0 #fff,inset 0 -6px 10px -6px rgba(255,255,255,.75),0 0 0 1px color-mix(in srgb,var(--acc) 50%,rgba(95,110,135,.3)),0 10px 20px -12px color-mix(in srgb,var(--acc) 55%,rgba(40,50,70,.5))}
  .soft h3{color:color-mix(in srgb,var(--acc) 50%,#2f3540)}
  .shelf.dk .soft h3{color:color-mix(in srgb,var(--acc) 30%,#2f3540)}
  .lit{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 84%,white),var(--acc) 70%,color-mix(in srgb,var(--acc) 88%,black));
    box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),inset 0 -2px 5px -2px rgba(0,0,0,.18),0 0 0 1px color-mix(in srgb,var(--acc) 75%,black),0 8px 16px -8px color-mix(in srgb,var(--acc) 70%,black)}
  .lit h3,.lit .m{color:var(--ink);text-shadow:var(--tsh)}
  /* El icono de menú enfocado: cuadro blanco con el glifo del color de la estantería (receta .srow.f .mic). */
  .lit .mic{background:rgba(255,255,255,.92);box-shadow:none;color:color-mix(in srgb,var(--acc) 80%,black)}
  .lit .mic :global(svg[fill=none] *){stroke:color-mix(in srgb,var(--acc) 80%,black)}
  .lit .mic :global(svg:not([fill=none]) *){fill:color-mix(in srgb,var(--acc) 80%,black)}
  .lit .mic :global(svg[fill=none] rect){fill:none}

  /* ---- Balda: el cristal común, neutro ---- */
  .bsb{position:relative;width:100%;height:190px;--r:26px;--bw:7px;cursor:default}
  .bsb::before{opacity:.4}
  .bsb>.face{background:linear-gradient(180deg,rgba(250,251,253,.75),rgba(236,240,245,.6))}
  .bsb>.face::after{box-shadow:inset 0 0 0 1px rgba(20,30,50,.12),inset 0 2px 5px -2px rgba(0,0,0,.18);z-index:5}
  .rtrack{position:absolute;left:0;top:0;height:100%;transition:transform .3s cubic-bezier(.25,1,.4,1)}
  .rtrack.view{left:9px;top:10px;height:139px}
  .scov{--r:16px;--bw:6px;top:0}
  .scov .face{border-radius:11px}
  .scov.hid{opacity:.55}
  /* Baldosas de acción («Nueva colección», «Añadir juegos»): la cara de .addtile de app.css, a la medida de la portada. */
  .scov .addin{gap:8px}
  .scov .addin span{width:40px;height:40px;border-radius:14px}
  .scov .addin b{font-size:13px;padding:0 4px;overflow-wrap:anywhere}
  .scov :global(.noart){padding:6px}
  .scov :global(.noart b){font-size:13px;line-height:1.15;overflow-wrap:anywhere}

  /* ---- Borde de etiquetas ---- */
  .slip{position:absolute;left:0;right:0;bottom:0;height:26px;z-index:4;overflow:hidden;
    background:linear-gradient(180deg,rgba(255,255,255,.88),rgba(222,227,235,.88));
    box-shadow:inset 0 1px 0 #fff,0 -6px 12px -10px rgba(40,50,70,.4)}
  .slip .rtrack{left:9px}
  .slab{position:absolute;top:0;height:100%;display:flex;align-items:center;justify-content:center;gap:5px;white-space:nowrap;
    font-size:13px;font-weight:700;color:#5f6776;transition:color .2s}
  .slab em{font-style:normal;font-size:13px;letter-spacing:-.5px;color:#d9a520}
  .slab s{text-decoration:none;display:flex;align-items:center;gap:4px;padding:2px 9px;border-radius:999px}
  .slab.on s{background:var(--acc);color:var(--ink);text-shadow:var(--tsh);font-weight:800;box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 0 0 1px color-mix(in srgb,var(--acc) 70%,black)}
  .slab.on em{color:#FFE27A}
  .shelf.dk .slab.on em{color:#7a5a00}
  .sm{position:absolute;top:54px;z-index:6;font-style:normal;font-size:15px;font-weight:800;color:#5f6776;pointer-events:none;
    padding:3px 10px;border-radius:999px;background:rgba(255,255,255,.82);box-shadow:0 0 0 1px rgba(95,110,135,.2)}
  .sm.l{left:8px}
  .sm.r{right:8px}
  /* Nº de orden sobre la portada */
  .ord{position:absolute;z-index:4;left:-7px;top:-7px;min-width:28px;height:28px;box-sizing:border-box;padding:0 6px;border-radius:999px;display:grid;place-items:center;font-size:14px;font-weight:800;color:#4a5260;
    background:linear-gradient(180deg,#fff,#e9edf2);box-shadow:0 0 0 1px rgba(95,110,135,.3),0 3px 6px -2px rgba(40,50,70,.35)}
  .scov.f .ord{background:var(--acc);color:var(--ink);box-shadow:0 0 0 1px color-mix(in srgb,var(--acc) 70%,black),0 3px 6px -2px rgba(40,50,70,.35)}
  .shelf.with-ord .rtrack.view{top:11px}
  /* Separadores y etiquetas de mes */
  .mdiv{position:absolute;top:6px;bottom:6px;width:0;border-left:1.5px dashed rgba(95,110,135,.35);pointer-events:none}
  .slab.mlab{letter-spacing:.3px}
  .slab.mlab s{text-transform:lowercase}
  @media (prefers-reduced-motion: reduce){.shelf,.rtrack{transition:none}}
</style>
