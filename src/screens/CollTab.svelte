<script lang="ts">
  // Pestaña «Colecciones» de una categoría: una estantería («pestaña sobre balda», boceto M1) por colección, en su
  // orden, y al final «Nueva colección». Cada una es un rótulo de cristal con la aurora de sus juegos y, bajo las
  // portadas, una línea de tiempo (cada portada sobre su año); «Ver todas las capturas» (X) va en el rótulo.
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { nCapturas, nGames } from '../lib/format';
  import { inkFor } from '../lib/ops.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import { LIST_VIS, SHELF_STEP, SHELF_VIEW_W } from '../lib/milista.svelte';
  import { collLayout, openCollShots, shelfAccent, shelfVars, tabLook } from '../lib/coleccion.svelte';
  import { gameYear } from '../lib/collections';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const shelves = $derived(app.collShelves);
  // Se pintan la enfocada y sus vecinas (la última estantería es «Nueva colección»).
  const total = $derived(shelves.length + 1);
  const range = $derived([Math.max(0, app.collTop - 1), Math.min(total, app.collTop + 4)] as const);
  const idx = $derived(Array.from({ length: Math.max(0, range[1] - range[0]) }, (_, k) => range[0] + k));

  /** Las estanterías bajan unos píxeles para que el rótulo no choque con las pestañas de arriba. */
  const COLL_DY = 12;
  const coverOf = (ti: number) => titles[ti].cover ?? titles[ti].shots[0]?.path ?? null;

  function itemsOf(k: number): ShelfItem[] {
    const sh = shelves[k];
    const L = collLayout(sh);
    // Una colección sin juegos visibles enseña un hueco que abre su edición.
    if (!sh.items.length) return [{ k: `cl${k}.0`, act: `cgame:${k}:0`, path: null, name: 'Añadir juegos', icon: ICON.plus }];
    return sh.items.map((ti, j) => ({ k: `cl${k}.${j}`, act: `cgame:${k}:${j}`, path: coverOf(ti), name: titles[ti].name, label: String(gameYear(titles[ti]) ?? '–'), pip: app.pipStyle(app.p, ti), mark: L.marks[j], tint: L.tints[j] ?? undefined }));
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
      {@const L = collLayout(sh)}
      {@const here = app.collAt.c === k}
      <Shelf acc={acc.c} ink={look.ink} tabStyle={look.style} aurora={look.aurora} title={sh.coll.name}
        sub={here ? `${nGames(sh.items.length)} · ${nCapturas(shotsOf(k))}` : nCapturas(shotsOf(k))} chip={nGames(sh.items.length)} items={itemsOf(k)}
        focus={here ? app.collAt.i : -1} from={Math.min(app.collFrom[sh.coll.id] ?? 0, Math.max(0, L.totalW - SHELF_VIEW_W))} visible={LIST_VIS}
        positions={L.xs} rail={L.rail ?? 'linear-gradient(90deg,var(--g1),var(--g2))'} vars={shelfVars(sh, acc)}
        top={(k - app.collTop) * SHELF_STEP + COLL_DY} dim={!here}>
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
        top={(k - app.collTop) * SHELF_STEP + COLL_DY} dim={app.collAt.c < shelves.length} />
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
