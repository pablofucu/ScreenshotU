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
    /** `--p1/--p2` de la pareja del juego (el foco y el resto de variables de color de la portada). */
    pip?: string;
    /** Línea de tiempo (`rail`): false = no es la primera portada de su año, así que no lleva el año (salvo enfocada). */
    mark?: boolean;
    /** Línea de tiempo (`rail`): color del punto de la portada en el raíl. */
    tint?: string;
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
    positions, dividers = [], monthGroups, vars = '', rail = '',
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
    /** Variables de color propias de la estantería (`--g1 --g2 --f1 --f2 --gf --on`…): rótulo de cristal, anillo y foco. */
    vars?: string;
    /** Línea de tiempo bajo las portadas (requiere `positions`): `background` CSS del raíl, normalmente un degradado. */
    rail?: string;
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
  const railW = $derived(positions && positions.length ? positions[positions.length - 1] + SHELF_CW : 0);
  /** En la línea de tiempo solo la primera portada de cada año (y la enfocada) enseña su año. */
  const showLabel = (it: ShelfItem, p: number) => !!it.label && (!rail || it.mark !== false || focus === p);
  /** Línea de tiempo «regla»: un rótulo por año, centrado bajo sus portadas (×N si hay varios juegos ese año). */
  const yearGroups = $derived.by(() => {
    if (!rail || !positions) return null;
    const gs: { x0: number; x1: number; label: string; n: number; on: boolean }[] = [];
    items.forEach((it, p) => {
      const g = gs[gs.length - 1], x = positions[p];
      if (g && g.label === (it.label ?? '')) { g.x1 = x + SHELF_CW; g.n++; g.on ||= focus === p; }
      else gs.push({ x0: x, x1: x + SHELF_CW, label: it.label ?? '', n: 1, on: focus === p });
    });
    return gs;
  });
  /** Tramo de color de la regla: del centro de la primera portada al de la última. */
  const segX = $derived(positions && positions.length ? [positions[0] + SHELF_CW / 2, positions[positions.length - 1] + SHELF_CW / 2] : [0, 0]);
</script>

<div class="shelf" class:dim class:dk={ink !== '#fff'} class:with-ord={hasOrd} class:tinted={!!vars} style="top:{top}px;--acc:{acc};--on-acc:{ink}{vars ? ';' + vars : ';--ink:' + ink}">
  <div class="bsh" class:lit={!dim} class:soft={dim} class:hasaur={aurora.length > 0 || !!tabStyle} class:noic={!icon}>
    {#if aurora.length}
      <div class="aur">{#each aurora as c, k}<b style="left:{4 + k * 20}%;background:{c}"></b>{/each}</div>
    {:else if tabStyle}
      <div class="aur multi" style={tabStyle}></div>
    {/if}
    {#if icon}<span class="mic">{@html icon}</span>{/if}
    <h3>{title}</h3>
    {#if chip && dim}<span class="tag">{chip}</span>{/if}
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
            style="left:{positions ? positions[p] : p * SHELF_PITCH}px;width:{SHELF_CW}px;height:{SHELF_CH}px{it.pip ? ';' + it.pip : ''}">
            {#if it.n !== undefined}<span class="ord">{it.n}</span>{/if}
            <div class="face">
              {#if it.path}<Thumb path={it.path} />{:else if it.icon}<div class="addin"><span>{@html it.icon}</span><b>{it.name}</b></div>{:else}<div class="noart"><b>{it.name}</b></div>{/if}
            </div>
          </div>
        {/each}
      </div>
      <div class="slip" class:railed={!!rail && !!positions}>
        <div class="rtrack" style="transform:translateX({-scrollX}px)">
          {#if rail && positions}
            <span class="rline" style="width:{railW}px"></span>
            <span class="rail" style="left:{segX[0]}px;width:{segX[1] - segX[0]}px;background:{rail};background-size:{railW}px 100%;background-position:{-segX[0]}px 0"></span>
          {/if}
          {#if yearGroups}
            {#each yearGroups as g}
              <span class="slab ylab" class:on={g.on} style="left:{g.x0}px;width:{g.x1 - g.x0}px">
                <s><b>{g.label}</b>{#if g.n > 1}<small>×{g.n}</small>{/if}</s>
              </span>
            {/each}
            {#each win as p (items[p].k)}
              {@const it = items[p]}
              <i class="rstem" style="left:{positions![p] + SHELF_CW / 2}px"></i>
              <i class="rdot" class:on={focus === p} style="left:{positions![p] + SHELF_CW / 2}px{it.tint ? ';--dc:' + it.tint : ''}"></i>
            {/each}
          {:else if monthGroups}
            {#each monthGroups as g}
              <span class="slab mlab" class:on={g.has} style="left:{g.x0}px;width:{g.x1 - g.x0}px">
                <s>{g.label}</s>
              </span>
            {/each}
          {:else}
            {#each win as p (items[p].k)}
              {@const it = items[p]}
              {@const lab = showLabel(it, p)}
              <span class="slab" class:on={focus === p} style="left:{positions ? positions[p] : p * SHELF_PITCH}px;width:{SHELF_CW}px">
                {#if lab || it.stars}<s>{#if lab}<b>{it.label}</b>{/if}{#if it.stars}<em>{'★'.repeat(it.stars)}</em>{/if}</s>{/if}
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
    --hi:color-mix(in srgb,var(--acc) 35%,var(--tint-base));--lo:color-mix(in srgb,var(--acc) 70%,black);--tsh:0 1px 2px rgba(0,0,0,.28)}
  .shelf.dk{--tsh:none}
  .shelf.dim .scov{opacity:.86}
  .shelf.dim .scov.hid{opacity:.5}

  /* ---- Rótulo: píldora de cristal, teñida en reposo y encendida con el foco ---- */
  .bsh{position:relative;display:inline-flex;align-items:center;gap:14px;height:40px;max-width:100%;box-sizing:border-box;padding:0 6px 0 8px;margin:0 0 6px 8px;border-radius:var(--r-pill);white-space:nowrap;z-index:3;isolation:isolate}
  .bsh.hasaur{overflow:hidden}
  .bsh>:global(*:not(.aur)){position:relative}
  .bsh>:global(*:last-child:not(.aur)){margin-right:8px}
  .bsh h3{margin:0;font-size:var(--t-xl);font-weight:800;color:var(--ink);line-height:1;max-width:430px;overflow:hidden;text-overflow:ellipsis;min-width:0}
  .bsh .m{font-size:var(--t-m);font-weight:600;color:var(--ink-2)}
  .bsh :global(.hint){height:32px;font-size:var(--t-m);background:var(--glass-chip);box-shadow:none;text-shadow:none;color:color-mix(in srgb,var(--ink),var(--ink-2))}
  /* Colecciones: botón de cristal con anillo del par de tonos; al pulsarlo se llena con el degradado (como el rótulo). */
  .tinted .bsh :global(.hint){padding:0 14px 0 4px;color:var(--ink);background:var(--soft-top);
    box-shadow:inset 0 1px 0 var(--hl),0 0 0 1.5px color-mix(in srgb,var(--g1) 55%,transparent);transition:transform .12s,background .12s,color .12s,box-shadow .12s}
  .tinted .bsh :global(.hint .btn){background:linear-gradient(135deg,var(--g1),var(--g2));color:#fff;text-shadow:none;box-shadow:inset 0 1px 0 rgba(255,255,255,.45)}
  .tinted .bsh :global(.hint:active){transform:scale(.95);color:#fff;background:linear-gradient(115deg,var(--g1),var(--g2));box-shadow:inset 0 2px 6px rgba(0,0,0,.25)}
  .tinted .bsh :global(.hint:active .btn){background:#fff;color:var(--g2)}
  .tag{font-size:var(--t-s);font-weight:800;padding:3px 11px;border-radius:var(--r-pill);color:var(--ink);background:var(--acc);box-shadow:inset 0 1px 0 rgba(255,255,255,.45),0 0 0 1px color-mix(in srgb,var(--acc) 70%,black)}
  .aur{position:absolute;inset:0;pointer-events:none;overflow:hidden;border-radius:inherit}
  .aur b{position:absolute;top:-30%;width:34%;height:160%;border-radius:50%;filter:blur(16px)}
  .aur.multi{opacity:.5}
  .soft .aur{opacity:.38}
  .lit .aur{opacity:.5}
  .soft{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 22%,var(--tint-base)),color-mix(in srgb,var(--acc) 38%,var(--tint-base)));
    box-shadow:inset 0 1px 0 var(--hl),inset 0 -6px 10px -6px var(--hl-bot),0 0 0 1px color-mix(in srgb,var(--acc) 50%,rgba(95,110,135,.3)),0 10px 20px -12px color-mix(in srgb,var(--acc) 55%,rgba(40,50,70,.5))}
  .soft h3{color:color-mix(in srgb,var(--acc) 50%,var(--ink))}
  .shelf.dk .soft h3{color:color-mix(in srgb,var(--acc) 30%,var(--ink))}
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
  .bsb>.face{background:linear-gradient(180deg,var(--glass-top),var(--glass-bot))}
  .bsb>.face::after{box-shadow:inset 0 0 0 1px rgba(20,30,50,.12),inset 0 2px 5px -2px rgba(0,0,0,.18);z-index:5}
  .rtrack{position:absolute;left:0;top:0;height:100%;transition:transform .3s cubic-bezier(.25,1,.4,1)}
  .rtrack.view{left:9px;top:10px;height:139px}
  .scov{--r:16px;--bw:6px;top:0}
  .scov .face{border-radius:var(--r-s)}
  .scov.hid{opacity:.55}
  /* Baldosas de acción («Nueva colección», «Añadir juegos»): la cara de .addtile de app.css, a la medida de la portada. */
  .scov .addin{gap:8px}
  .scov .addin span{width:40px;height:40px;border-radius:var(--r-s)}
  .scov .addin b{font-size:var(--t-s);padding:0 4px;overflow-wrap:anywhere}
  .scov :global(.noart){padding:6px}
  .scov :global(.noart b){font-size:var(--t-s);line-height:1.15;overflow-wrap:anywhere}

  /* ---- Borde de etiquetas ---- */
  .slip{position:absolute;left:0;right:0;bottom:0;height:26px;z-index:4;overflow:hidden;
    background:linear-gradient(180deg,var(--glass-chip),var(--face));
    box-shadow:inset 0 1px 0 var(--glass-top),0 -6px 12px -10px rgba(40,50,70,.4)}
  .slip .rtrack{left:9px}
  .slab{position:absolute;top:0;height:100%;display:flex;align-items:center;justify-content:center;gap:5px;white-space:nowrap;
    font-size:var(--t-s);font-weight:700;color:var(--ink-2);transition:color .2s}
  .slab em{font-style:normal;font-size:var(--t-s);letter-spacing:-.5px;color:#d9a520}
  .slab s{text-decoration:none;display:flex;align-items:center;gap:4px;padding:2px 9px;border-radius:var(--r-pill)}
  .slab.on s{background:var(--acc);color:var(--ink);text-shadow:var(--tsh);font-weight:800;box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 0 0 1px color-mix(in srgb,var(--acc) 70%,black)}
  .slab.on em{color:#FFE27A}
  .shelf.dk .slab.on em{color:#7a5a00}
  .sm{position:absolute;top:54px;z-index:6;font-style:normal;font-size:var(--t-m);font-weight:800;color:var(--ink-2);pointer-events:none;
    padding:3px 10px;border-radius:var(--r-pill);background:var(--glass-chip);box-shadow:0 0 0 1px var(--line)}
  .sm.l{left:8px}
  .sm.r{right:8px}
  /* Nº de orden sobre la portada */
  .ord{position:absolute;z-index:4;left:-7px;top:-7px;min-width:28px;height:28px;box-sizing:border-box;padding:0 6px;border-radius:var(--r-pill);display:grid;place-items:center;font-size:var(--t-s);font-weight:800;color:color-mix(in srgb,var(--ink),var(--ink-2));
    background:linear-gradient(180deg,var(--face-hi),var(--face));box-shadow:0 0 0 1px var(--line),0 3px 6px -2px rgba(40,50,70,.35)}
  .scov.f .ord{background:var(--acc);color:var(--ink);box-shadow:0 0 0 1px color-mix(in srgb,var(--acc) 70%,black),0 3px 6px -2px rgba(40,50,70,.35)}
  .shelf.with-ord .rtrack.view{top:11px}
  /* Separadores y etiquetas de mes */
  .mdiv{position:absolute;top:6px;bottom:6px;width:0;border-left:1.5px dashed rgba(95,110,135,.35);pointer-events:none}
  .slab.mlab{letter-spacing:.3px}
  .slab.mlab s{text-transform:lowercase}

  /* ---- Línea de tiempo (colecciones), «regla»: tallo bajo cada portada, línea gris con el tramo de color de sus juegos,
     puntos blancos con anillo del color del juego y un rótulo por año (el enfocado, en píldora con el degradado) ---- */
  .slip.railed{height:40px}
  .rline{position:absolute;left:0;top:9px;height:2px;border-radius:2px;background:rgba(60,70,90,.16)}
  .rail{position:absolute;top:8px;height:4px;border-radius:4px}
  .rstem{position:absolute;top:0;width:2px;height:9px;margin-left:-1px;background:rgba(60,70,90,.2)}
  .rdot{position:absolute;top:10px;width:10px;height:10px;margin:-5px 0 0 -5px;border-radius:50%;background:var(--tint-base);
    box-shadow:0 0 0 2.5px var(--dc,var(--f1)),0 1px 3px rgba(40,50,70,.3);transition:width .2s,height .2s,margin .2s,box-shadow .2s}
  .rdot.on{width:14px;height:14px;margin:-7px 0 0 -7px;box-shadow:0 0 0 3px var(--dc,var(--f1)),0 0 0 5px rgba(255,255,255,.85),0 0 10px 3px color-mix(in srgb,var(--dc,var(--f1)) 55%,transparent)}
  .slip.railed .slab{top:17px;height:22px;font-size:var(--t-m);font-weight:800;color:var(--ink)}
  .slab.ylab small{font-size:var(--t-xs);font-weight:700;color:var(--ink-2)}
  .slab.ylab s{padding:1px 10px;transition:transform .2s}
  .slab.ylab.on small{color:inherit;opacity:.85}

  /* ---- Colecciones v5: rótulo de cristal con la aurora de sus juegos y un anillo del par de tonos ---- */
  .shelf.tinted{--tsh:none}
  .tinted .bsh.noic{padding-left:18px}
  .tinted .bsh{-webkit-backdrop-filter:blur(16px) saturate(1.7);backdrop-filter:blur(16px) saturate(1.7);
    background:linear-gradient(180deg,var(--glass-top),var(--glass-mid) 55%,var(--glass-bot));
    box-shadow:inset 0 1px 0 var(--glass-top),inset 0 -6px 10px -6px var(--hl-bot),inset 0 -1px 0 rgba(110,125,150,.22),0 0 0 1px rgba(95,110,135,.22),0 10px 20px -12px rgba(40,50,70,.4)}
  .tinted .bsh.hasaur{overflow:visible}
  .tinted .bsh::before{content:'';position:absolute;inset:-3px;border-radius:var(--r-pill);padding:2.5px;box-sizing:border-box;pointer-events:none;z-index:2;
    background:linear-gradient(115deg,var(--g1),var(--g2),var(--g1));background-size:200% 100%;
    -webkit-mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);-webkit-mask-composite:xor;mask-composite:exclude;
    opacity:.45;transition:opacity .3s}
  .tinted .bsh.lit::before{opacity:1;animation:shelfSheen 4s linear infinite}
  .tinted .bsh.lit{box-shadow:inset 0 1px 0 var(--glass-top),inset 0 -6px 10px -6px var(--hl-bot),inset 0 -1px 0 rgba(110,125,150,.22),0 0 0 1px rgba(95,110,135,.18),0 10px 22px -10px color-mix(in srgb,var(--f2) 65%,rgba(40,50,70,.5))}
  .tinted .aur{opacity:.5}
  .tinted .lit .aur{opacity:.9}
  .tinted .bsh h3,.tinted .bsh.lit h3{color:var(--ink);text-shadow:none}
  .tinted .bsh .m,.tinted .bsh.lit .m{color:var(--ink-2);text-shadow:none}
  .tinted .tag{background:var(--gf);color:var(--on);box-shadow:inset 0 1px 0 rgba(255,255,255,.45),0 0 0 1px color-mix(in srgb,var(--f2) 70%,black)}
  .tinted .bsh .mic,.tinted .bsh.lit .mic{background:rgba(255,255,255,.92);box-shadow:inset 0 1px 0 var(--hl),0 0 0 1px color-mix(in srgb,var(--f1) 45%,transparent),0 4px 10px -5px var(--f2)}
  .tinted .bsh .mic :global(svg[fill=none] *),.tinted .bsh.lit .mic :global(svg[fill=none] *){stroke:var(--f2)}
  .tinted .bsh .mic :global(svg:not([fill=none]) *),.tinted .bsh.lit .mic :global(svg:not([fill=none]) *){fill:var(--f2)}
  .tinted .bsh .mic :global(svg[fill=none] rect),.tinted .bsh.lit .mic :global(svg[fill=none] rect){fill:none}
  .tinted .slab.on s{background:var(--gf);color:var(--on);text-shadow:none;box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 4px 10px -4px var(--f2)}
  .tinted .scov.f .ord{background:var(--gf);color:var(--on);box-shadow:0 3px 6px -2px rgba(40,50,70,.35)}
  /* Títulos «letras de caramelo»: el propio texto lleva el par de tonos de la colección, con bisel blanco, sobre cristal sin aurora. */
  .tinted .aur{display:none}
  .tinted .bsh{background:linear-gradient(180deg,var(--glass-chip),var(--glass-top))}
  .tinted .bsh.soft{background:linear-gradient(180deg,var(--glass-top),var(--glass-mid) 55%,var(--glass-bot))}
  .tinted .bsh::before{opacity:.25}
  .tinted .bsh.lit::before{opacity:.7;animation:none}
  .tinted .bsh h3,.tinted .bsh.lit h3{font:800 25px/1.15 var(--round);padding:2px 1px;color:transparent;
    background:linear-gradient(115deg,var(--f1),var(--f2),var(--f1));background-size:200% 100%;-webkit-background-clip:text;background-clip:text;
    filter:drop-shadow(0 1px 0 rgba(255,255,255,.95)) drop-shadow(0 2px 3px color-mix(in srgb,var(--f2) 35%,transparent))}
  .tinted .bsh.lit h3{animation:shelfSheen 6s linear infinite}
  .tinted .bsh.soft h3{background:none;color:var(--ink-2);filter:none;font-size:var(--t-xl)}
  .tinted .tag,.tinted .bsh .m{font:800 var(--t-s) var(--round)}
  .tinted .bsh.soft .tag{background:var(--glass-chip);color:var(--ink-2);box-shadow:inset 0 0 0 1.5px color-mix(in srgb,var(--g1) 55%,transparent)}
  :global(#stage[data-theme=dark]) .tinted .bsh h3{background-image:linear-gradient(115deg,var(--g1),var(--g2),var(--g1));filter:drop-shadow(0 0 8px color-mix(in srgb,var(--g2) 45%,transparent))}
  :global(#stage[data-theme=dark]) .tinted .bsh.soft h3{background:none;filter:none}
  @keyframes shelfSheen{to{background-position:200% 0}}
  @media (prefers-reduced-motion: reduce){.shelf,.rtrack{transition:none}.tinted .bsh.lit::before{animation:none}.rdot,.slab.ylab s,.tinted .bsh :global(.hint){transition:none}}
</style>
