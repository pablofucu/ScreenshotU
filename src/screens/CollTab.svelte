<script lang="ts">
  // Pestaña «Colecciones» de una categoría: una estantería («pestaña sobre balda», boceto M1) por colección, en su
  // orden, y al final «Nueva colección». La pestaña lleva la línea de tiempo por años y «Ver todas las capturas» (X).
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { nCapturas, nGames } from '../lib/format';
  import { inkFor } from '../lib/ops.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import { LIST_VIS, SHELF_STEP } from '../lib/milista.svelte';
  import { openCollShots, shelfAccent, tabLook } from '../lib/coleccion.svelte';
  import { gameYear } from '../lib/collections';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const shelves = $derived(app.collShelves);
  // Se pintan la enfocada y sus vecinas (la última estantería es «Nueva colección»).
  const total = $derived(shelves.length + 1);
  const range = $derived([Math.max(0, app.collTop - 1), Math.min(total, app.collTop + 4)] as const);
  const idx = $derived(Array.from({ length: Math.max(0, range[1] - range[0]) }, (_, k) => range[0] + k));

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
      {@const acc = shelfAccent(sh)}
      {@const look = tabLook(games, acc)}
      {@const tl = timeline(k)}
      {@const here = app.collAt.c === k}
      <Shelf acc={acc.c} ink={look.ink} tabStyle={look.style} aurora={look.aurora} title={sh.coll.name} icon={ICON.saga}
        sub={here ? `${nGames(sh.items.length)} · ${nCapturas(shotsOf(k))}` : nCapturas(shotsOf(k))} chip={nGames(sh.items.length)} items={itemsOf(k)}
        focus={here ? app.collAt.i : -1} from={app.collFrom[sh.coll.id] ?? 0} visible={LIST_VIS}
        top={(k - app.collTop) * SHELF_STEP} dim={!here}>
        {#snippet extra()}
          {#if tl}
            <div class="tl">{tl.a}<span class="tr" style="width:{here ? 200 : 150}px">{#each tl.dots as d (d.j)}<i class:on={here && app.collAt.i === d.j} style="left:{d.left}%"></i>{/each}</span>{tl.b}</div>
          {/if}
        {/snippet}
        {#snippet end()}
          {#if here}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <span class="hint" onclick={(e) => { e.stopPropagation(); openCollShots(k); }}><span class="btn x">X</span>Capturas</span>
          {/if}
        {/snippet}
      </Shelf>
    {:else}
      <Shelf acc={app.chrome.c} ink={inkFor(app.chrome.c)} title="Nueva colección" icon={ICON.plus} sub={newSub}
        items={[{ k: 'cnew', act: 'cnew', path: null, name: 'Nueva colección', icon: ICON.plus }]}
        focus={app.collAt.c >= shelves.length ? 0 : -1} visible={LIST_VIS}
        top={(k - app.collTop) * SHELF_STEP} dim={app.collAt.c < shelves.length} />
    {/if}
  {/each}
</div>
{#if !shelves.length}
  <div class="glass note-card" style="left:300px;top:400px;height:240px">
    <div class="face">
      <h2>Agrupa tus juegos en colecciones</h2>
      <p>Una colección junta en una estantería varios juegos de {P?.name ?? 'esta categoría'} (Trails, Kingdom Hearts…), los ordena por año y reúne todas sus capturas. Solo es una agrupación: no mueve ni copia ningún archivo.</p>
    </div>
  </div>
{/if}

<style>
  /* Línea de tiempo en el rótulo: tinta neutra en reposo y la tinta del rótulo cuando está encendido. */
  .tl{display:flex;align-items:center;gap:9px;font-size:14px;font-weight:700;color:var(--ink-2);white-space:nowrap}
  .tr{position:relative;width:200px;height:4px;border-radius:4px;background:var(--line-2)}
  .tr i{position:absolute;top:50%;width:10px;height:10px;margin:-5px 0 0 -5px;border-radius:50%;background:var(--acc);box-shadow:0 0 0 2px var(--on);transition:width .2s,height .2s,margin .2s,box-shadow .2s}
  .tr i.on{width:16px;height:16px;margin:-8px 0 0 -8px;box-shadow:0 0 0 3px var(--on),0 0 10px 2px color-mix(in srgb,var(--acc) 60%,transparent)}
  :global(.bsh.lit) .tl{color:var(--ink);text-shadow:var(--tsh)}
  :global(.bsh.lit) .tr{background:color-mix(in srgb,var(--ink) 38%,transparent)}
  :global(.bsh.lit) .tr i{background:var(--ink);box-shadow:0 1px 2px rgba(0,0,0,.25)}
  :global(.bsh.lit) .tr i.on{box-shadow:0 0 0 3px color-mix(in srgb,var(--acc) 50%,white),0 0 10px 2px rgba(255,255,255,.9)}
</style>
