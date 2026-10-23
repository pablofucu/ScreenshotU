<script lang="ts">
  // Pestaña «Colecciones» de una categoría: una estantería («pestaña sobre balda», boceto M1) por colección, en su
  // orden, y al final «Nueva colección». La pestaña lleva la línea de tiempo por años y «Ver todas las capturas» (X).
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { nCapturas } from '../lib/format';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import { LIST_VIS, SHELF_STEP } from '../lib/milista.svelte';
  import { collAccent, nGames, openCollShots, tabLook } from '../lib/coleccion.svelte';
  import { gameYear } from '../lib/collections';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const shelves = $derived(app.collShelves);
  // Se pintan la enfocada y sus vecinas (la última estantería es «Nueva colección»).
  const total = $derived(shelves.length + 1);
  const range = $derived([Math.max(0, app.collTop - 1), Math.min(total, app.collTop + 4)] as const);
  const idx = $derived(Array.from({ length: Math.max(0, range[1] - range[0]) }, (_, k) => range[0] + k));
  const CAM = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13" r="3.5"/></svg>';

  const coverOf = (ti: number) => titles[ti].cover ?? titles[ti].shots[0]?.path ?? null;

  function itemsOf(k: number): ShelfItem[] {
    const sh = shelves[k];
    // Una colección sin juegos visibles enseña un hueco que abre su edición.
    if (!sh.items.length) return [{ k: `cl${k}.0`, act: `cgame:${k}:0`, path: null, name: 'Añadir juegos', icon: ICON.plus }];
    return sh.items.map((ti, j) => ({ k: `cl${k}.${j}`, act: `cgame:${k}:${j}`, path: coverOf(ti), name: titles[ti].name, label: String(gameYear(titles[ti]) ?? '–') }));
  }

  /** Línea de tiempo: un punto por juego según su año (de lanzamiento o, sin él, de su primera captura o partida). */
  function timeline(k: number) {
    const ys = shelves[k].items.map((ti) => gameYear(titles[ti]));
    const known = ys.filter((y): y is number => y !== null);
    if (!known.length) return null;
    const a = Math.min(...known), b = Math.max(...known);
    const dots = ys.flatMap((y, j) => (y === null ? [] : [{ j, left: b === a ? 50 : ((y - a) / (b - a)) * 100 }]));
    return { a, b, dots };
  }

  const shotsOf = (k: number) => shelves[k].items.reduce((n, ti) => n + titles[ti].shots.length, 0);
  const newSub = $derived(shelves.length ? 'Agrupa más juegos de esta categoría' : `Aún no tienes colecciones en ${P?.name ?? 'esta categoría'}`);
</script>

<div class="lclip">
  {#each idx as k (k < shelves.length ? shelves[k].coll.id : 'nueva')}
    {#if k < shelves.length}
      {@const sh = shelves[k]}
      {@const games = sh.items.map((ti) => titles[ti])}
      {@const acc = collAccent(sh.coll, games)}
      {@const look = tabLook(games, acc)}
      {@const tl = timeline(k)}
      {@const here = app.collAt.c === k}
      <Shelf acc={acc.c} ink={look.ink} tabStyle={look.style} aurora={look.aurora} title={sh.coll.name}
        sub="{nGames(sh.items.length)} · {nCapturas(shotsOf(k))}" items={itemsOf(k)}
        focus={here ? app.collAt.i : -1} from={app.collFrom[sh.coll.id] ?? 0} visible={LIST_VIS}
        top={(k - app.collTop) * SHELF_STEP} dim={!here}>
        {#snippet extra()}
          {#if tl}
            <div class="ctl">{tl.a}<span class="ctr">{#each tl.dots as d (d.j)}<i class:on={here && app.collAt.i === d.j} style="left:{d.left}%"></i>{/each}</span>{tl.b}</div>
          {/if}
        {/snippet}
        {#snippet end()}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="cpill" onclick={(e) => { e.stopPropagation(); openCollShots(k); }}>{@html CAM}Ver todas las capturas<span class="btn x">X</span></span>
        {/snippet}
      </Shelf>
    {:else}
      <Shelf acc={app.chrome.c} ink={app.chrome.dark ? '#3a2f00' : '#fff'} title="Nueva colección" sub={newSub}
        items={[{ k: 'cnew', act: 'cnew', path: null, name: 'Nueva colección', icon: ICON.plus }]}
        focus={app.collAt.c >= shelves.length ? 0 : -1} visible={LIST_VIS}
        top={(k - app.collTop) * SHELF_STEP} dim={app.collAt.c < shelves.length} />
    {/if}
  {/each}
</div>
{#if !shelves.length}
  <div class="cnote">Una colección junta en una estantería varios juegos de {P?.name ?? 'esta categoría'} (Trails, Kingdom Hearts…), los ordena por año y reúne todas sus capturas. Solo es una agrupación: no mueve ni copia ningún archivo.</div>
{/if}

<style>
  /* Zona de las estanterías (recorta lo que se desplaza fuera, a ancho completo para no cortar las sombras). */
  /* Alto justo para que de la tercera estantería solo asome el borde de su pestaña (sin meterse bajo las pistas). */
  .lclip{position:absolute;left:0;top:166px;width:1280px;height:494px;overflow:hidden}
  /* Línea de tiempo dentro de la pestaña. */
  .ctl{display:flex;align-items:center;gap:8px;font-size:12px;font-weight:700;color:var(--ink);text-shadow:0 1px 2px rgba(0,0,0,.25)}
  .ctr{position:relative;width:230px;height:4px;border-radius:4px;background:color-mix(in srgb,var(--ink) 38%,transparent)}
  .ctr i{position:absolute;top:50%;width:9px;height:9px;margin:-4.5px 0 0 -4.5px;border-radius:50%;background:var(--ink);box-shadow:0 1px 2px rgba(0,0,0,.25);transition:width .2s,height .2s,margin .2s,box-shadow .2s}
  .ctr i.on{width:15px;height:15px;margin:-7.5px 0 0 -7.5px;box-shadow:0 0 0 3px color-mix(in srgb,var(--acc) 60%,white),0 0 10px 2px rgba(255,255,255,.9)}
  /* «Ver todas las capturas» (X). */
  .cpill{display:inline-flex;align-items:center;gap:8px;height:38px;padding:0 5px 0 14px;border-radius:999px;font-size:14px;font-weight:700;color:#4a5260;white-space:nowrap;flex:none;cursor:pointer;
    background:linear-gradient(180deg,rgba(255,255,255,.92),rgba(255,255,255,.6));box-shadow:inset 0 1px 0 #fff,inset 0 -1px 0 rgba(110,125,150,.22),0 0 0 1px rgba(95,110,135,.22),0 8px 16px -10px rgba(40,50,70,.4);text-shadow:none}
  .cpill :global(svg){width:18px;height:18px;color:var(--acc)}
  .cpill:hover{background:linear-gradient(180deg,#fff,rgba(255,255,255,.8))}
  .cnote{position:absolute;left:96px;top:430px;width:760px;font-size:17px;font-weight:600;line-height:1.4;color:#5f6776}
</style>
