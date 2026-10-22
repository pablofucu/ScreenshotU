<script lang="ts">
  import { app, LIST_COLOR, STATUS_LABEL, TITLE_FILTERS, TITLE_ORDER_LABEL, TITLES_TABS, type ListShelf } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtAgo, fmtDur, fmtShort, nCapturas } from '../lib/format';
  import { backend } from '../lib/backend';
  import { titlesGrid, viewPos } from '../lib/controller.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import { itemLabel, LIST_VIS, SHELF_STEP, shelfSub } from '../lib/milista.svelte';

  const P = $derived(app.platform);
  const saga = $derived(P?.id === 'sagas');
  const titles = $derived(P?.titles ?? []);
  const grid = $derived(app.config.titlesView === 'grid');
  // Lista visible (filtrada); la baldosa «Añadir juego» ocupa la posición siguiente a la última.
  const vis = $derived(app.tview);
  const slots = $derived(vis.length + 1);
  // Medidas de la cuadrícula según el tamaño elegido (5 a 8 por fila).
  const G = $derived(titlesGrid());
  const PS = $derived(G.size);
  const pages = $derived(Math.max(1, Math.ceil(slots / PS)));
  const q = $derived(viewPos(app.t));
  const pg = $derived(Math.floor(q / PS));
  // Pestaña de la categoría: Sagas no tiene; «Mi lista» y «Colecciones» llegan en las fases 3 y 4.
  const tab = $derived(saga ? 'all' : app.titlesTab);
  const tabInfo = { name: 'Colecciones', text: 'Estanterías con los juegos de esta categoría agrupados a tu manera: sagas, series o lo que quieras.' };
  // «Mi lista»: estanterías por estado. Se pintan la enfocada y sus vecinas; cada una con el color fijo de su estado.
  const shelves = $derived(app.shelves);
  const focusShelf = $derived(shelves.findIndex((s) => s.items.includes(app.t)));
  const listAcc = $derived(focusShelf >= 0 ? LIST_COLOR[shelves[focusShelf].kind] : app.chrome);
  const shelfRange = $derived([Math.max(0, app.listTop - 1), Math.min(shelves.length, app.listTop + 4)] as const);
  const shelfIdx = $derived(Array.from({ length: Math.max(0, shelfRange[1] - shelfRange[0]) }, (_, k) => shelfRange[0] + k));
  const shelfItems = (sh: ListShelf): ShelfItem[] =>
    sh.items.map((ti, k) => {
      const { label, stars } = itemLabel(sh, titles[ti], k);
      return { k: 'tit' + ti, act: 'tit:' + ti, path: coverOf(ti), name: titles[ti].name, label, stars };
    });
  const curShelf = $derived(focusShelf >= 0 ? shelves[focusShelf] : null);
  const cur = $derived(titles[app.t]);
  const coverOf = (i: number) => titles[i].cover ?? titles[i].shots[0]?.path ?? null;
  const realAt = (pos: number) => (pos < vis.length ? vis[pos] : titles.length);
  const favCount = $derived(cur ? cur.shots.reduce((n, s) => n + (app.favs.has(s.path) ? 1 : 0), 0) : 0);
  const filterName = $derived(TITLE_FILTERS.find(([f]) => f === app.titleFilter)?.[1] ?? '');
  const counts = $derived.by(() => ({
    all: titles.length,
    launch: titles.filter((t) => t.launchable).length,
    shots: titles.filter((t) => !t.launchable).length,
    favs: titles.filter((t) => t.shots.some((s) => app.favs.has(s.path))).length,
    playing: titles.filter((t) => t.status === 'playing').length,
    pending: titles.filter((t) => t.status === 'pending').length,
    finished: titles.filter((t) => t.status === 'finished').length,
  }));

  // Ventana virtual: solo se pintan la página actual y las vecinas (cuadrícula) o ±9 (carrusel).
  const range = $derived(
    grid
      ? [Math.max(0, (pg - 1) * PS), Math.min(slots, (pg + 2) * PS)]
      : [Math.max(0, q - 9), Math.min(slots, q + 10)],
  );
  const idx = $derived(Array.from({ length: Math.max(0, range[1] - range[0]) }, (_, k) => range[0] + k));
  // Posición y tamaño de la portada nº `j` de la página `page` (los textos y bordes se escalan con el tamaño).
  const cellStyle = (page: number, j: number) =>
    `left:${page * 1280 + G.gx + (j % G.cols) * (G.cw + G.gap)}px;top:${G.top + Math.floor(j / G.cols) * (G.ch + G.rg)}px;width:${G.cw}px;height:${G.ch}px;--r:${(22 * G.s).toFixed(1)}px;--bw:${(8 * G.s).toFixed(1)}px;--fr:${(15 * G.s).toFixed(1)}px;--ts:${(0.55 + 0.45 * G.s).toFixed(3)}`;

  // Fondo ambiental: el hero (o la portada) del juego enfocado, muy desenfocado, con fundido entre juegos.
  let layers = $state<Array<{ u: string; id: number }>>([]);
  let lid = 0;
  $effect(() => {
    const src = cur ? (cur.hero ?? cur.cover ?? cur.shots[0]?.path) : null;
    if (!src) { layers = []; return; }
    const ctl = new AbortController();
    const t = setTimeout(() => {
      backend.thumb(src, ctl.signal).then((u) => {
        if (ctl.signal.aborted || layers.at(-1)?.u === u) return;
        layers = [...layers.slice(-1), { u, id: ++lid }];
      }, () => {});
    }, 90);
    return () => { clearTimeout(t); ctl.abort(); };
  });
</script>

{#snippet cell(i: number)}
  <div class="face">
    {#if i === titles.length}
      <div class="addin"><span>{@html ICON.plus}</span><b>{saga ? 'Crear saga' : 'Añadir juego'}</b></div>
    {:else if titles[i].members}
      <div class="sstack">
        {#each titles[i].members!.filter((m) => m.cover).slice(0, 3) as m, k (m.id)}
          <div class="sc sc{k}"><Thumb path={m.cover!} /></div>
        {:else}
          <div class="noart"><b>{titles[i].name}</b></div>
        {/each}
        <span class="sname">{titles[i].name}</span>
      </div>
    {:else}
      {#if coverOf(i)}<Thumb path={coverOf(i)!} />{:else}<div class="noart"><b>{titles[i].name}</b></div>{/if}{#if titles[i].hidden}<span class="hbadge">{@html ICON.eyeOff}</span>{/if}{#if titles[i].status === 'finished' || titles[i].status === 'dropped'}<span class="sbadge {titles[i].status}" title={STATUS_LABEL[titles[i].status!]}>{#if titles[i].status === 'finished'}{@html ICON.check}{:else}<i></i>{/if}</span>{/if}
    {/if}
  </div>
{/snippet}

<div class="amb" class:car={!grid}>
  {#each layers as l (l.id)}<div class="amblayer" style="background-image:url({l.u})"></div>{/each}
</div>
<TopBar crumbs={['Inicio', P?.name ?? '']} dots={grid && tab === 'all' ? pages : 0} dot={pg} />

{#if !saga}
  <div class="ttabs" class:car={!grid && tab === 'all'} style={tab === 'list' ? `--acc:${listAcc.c};--acc-ink:${listAcc.dark ? '#3a2f00' : '#fff'}` : grid && cur ? `--acc:${app.titleAcc(app.p, app.t).c};--acc-ink:${app.titleAcc(app.p, app.t).dark ? '#3a2f00' : '#fff'}` : ''}>
    <span class="btn sh" data-hact="view">View</span>
    {#each TITLES_TABS as [f, label]}
      <span class="tflt" class:on={tab === f}>{label}</span>
    {/each}
    {#if tab === 'all' && titles.length > 1}<span class="btn x" data-hact="x">X</span><span class="tflt tord">{TITLE_ORDER_LABEL[app.titleOrder]}</span>{/if}
  </div>
{/if}

{#if tab === 'list'}
  {#if !shelves.length}
    <div class="glass note-card" style="left:300px;top:230px;--acc:{app.chrome.c}">
      <div class="face">
        <h2>Aún no hay juegos en {P?.name}</h2>
        <p>Cuando haya juegos aparecerán aquí ordenados por estado. Con Y marcas si lo estás jugando, lo tienes pendiente, lo terminaste o lo abandonaste.</p>
      </div>
    </div>
  {:else}
    <div class="lclip">
      {#each shelfIdx as k (shelves[k].key)}
        {@const sh = shelves[k]}
        <Shelf acc={LIST_COLOR[sh.kind].c} title={sh.label} sub={shelfSub(sh, titles)} items={shelfItems(sh)}
          focus={k === focusShelf ? sh.items.indexOf(app.t) : -1} from={app.listFrom[sh.key] ?? 0} visible={LIST_VIS}
          top={(k - app.listTop) * SHELF_STEP} dim={k !== focusShelf} />
      {/each}
    </div>
  {/if}
{:else if tab !== 'all'}
  <!-- Hueco de la fase 4 (Colecciones): se rellena aquí. -->
  <div class="glass note-card soon" style="left:300px;top:230px;--acc:{app.chrome.c}">
    <div class="face">
      <span class="chip soonchip">Próximamente</span>
      <h2>{tabInfo.name}</h2>
      <p>{tabInfo.text}</p>
    </div>
  </div>
{:else if titles.length === 0}
  <div class="glass note-card" style="left:300px;top:{saga ? 130 : 170}px;--acc:{app.chrome.c}">
    <div class="face">
      {#if saga}
        <h2>Aún no tienes sagas</h2>
        <p>Una saga junta en una sola vista las capturas de varios juegos (por ejemplo Mass Effect 1, 2 y 3). Solo es una agrupación: no mueve ni copia ningún archivo. Crea la primera con la baldosa de abajo.</p>
      {:else}
        <h2>Aún no hay juegos en {P?.name}</h2>
        <p>Crea una carpeta por juego dentro de <b>{P?.folder}</b> y copia dentro tus capturas (png, jpg, jpeg o webp), o añade un juego con la baldosa de abajo.</p>
      {/if}
    </div>
  </div>
  <div class="glass cover addtile fx" class:f={app.focus === 'tit0'} data-k="tit0" data-act="tit:0" style="left:552px;top:430px;--acc:{app.chrome.c}">
    <div class="face"><div class="addin"><span>{@html ICON.plus}</span><b>{saga ? 'Crear saga' : 'Añadir juego'}</b></div></div>
  </div>
{:else}
  {#if grid}
    {#if cur}
      {@const a = app.titleAcc(app.p, app.t)}
      <div class="thead">
        <h3>{cur.name}</h3>
        <span class="chip" style="background:{a.c};color:{a.dark ? '#3a2f00' : '#fff'}">{cur.shots.length ? nCapturas(cur.shots.length) : 'Sin capturas'}</span>
        {#if cur.playSeconds >= 60}<span>Jugado <b>{fmtDur(cur.playSeconds)}</b></span>{/if}
        {#if cur.lastPlayed}<span>Última partida <b>{fmtAgo(cur.lastPlayed).toLowerCase()}</b></span>
        {:else if cur.shots.length}<span>Última captura <b>{fmtShort(cur.shots[0].date)}</b></span>{/if}
        {#if favCount}<span class="tfav">{@html ICON.starY}<b>{favCount}</b></span>{/if}
      </div>
    {:else if !vis.length}
      <div class="thead"><h3>Ningún juego</h3><span>con el filtro «{filterName}»</span></div>
    {/if}
  {/if}
  {#if !saga}
    <div class="tfilters lo" class:car={!grid} style={grid && cur ? `--acc:${app.titleAcc(app.p, app.t).c};--acc-ink:${app.titleAcc(app.p, app.t).dark ? '#3a2f00' : '#fff'}` : ''}>
      <span class="btn sh" data-hact="lt">LT</span>
      {#each TITLE_FILTERS as [f, label]}
        <span class="tflt" class:on={app.titleFilter === f}>{label}{#if f !== 'all'}<small>{counts[f]}</small>{/if}</span>
      {/each}
      <span class="btn sh" data-hact="rt">RT</span>
    </div>
  {/if}
  {#if grid}
    <div class="ttrack" style="width:{pages * 1280}px;transform:translateX({-pg * 1280}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        {@const j = pos % PS}
        <div class="glass cover tcov fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="{cellStyle(Math.floor(pos / PS), j)};--acc:{i === titles.length ? app.chrome.c : app.titleAcc(app.p, i).c}">
          {@render cell(i)}
        </div>
      {/each}
    </div>
    <div class="side" class:off={pg === 0} data-hact="lb" style="left:12px">{@html ICON.arrowL}</div>
    <div class="side" class:off={pg >= pages - 1} data-hact="rb" style="right:12px">{@html ICON.arrowR}</div>
  {:else}
    <div class="ctrack" style="transform:translateX({640 - 100 - q * 240}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        <div class="glass cover ccover fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="left:{pos * 240}px;--acc:{i === titles.length ? app.chrome.c : app.titleAcc(app.p, i).c}">
          {@render cell(i)}
        </div>
      {/each}
    </div>
    {#if cur}
      <div class="cinfo">
        <h3>{cur.name}</h3>
        <p>{cur.shots.length ? `${nCapturas(cur.shots.length)} · última el ${fmtShort(cur.shots[0].date)}` : 'Sin capturas'}{cur.playSeconds >= 60 ? ` · jugado ${fmtDur(cur.playSeconds)}` : ''} · {q + 1} de {vis.length}</p>
      </div>
    {:else if !vis.length}
      <div class="cinfo"><h3>Ningún juego</h3><p>con el filtro «{filterName}»</p></div>
    {/if}
  {/if}
{/if}

{#if tab === 'list' && shelves.length}
  <Hints list={[
    ['A', 'Abrir', 'a'], ['B', 'Volver', 'b'],
    ...(cur?.launchable ? [['X', 'Jugar', 'x'] as [string, string, string]] : []),
    ['Y', 'Estado', 'y'],
    ['LB/RB', curShelf?.kind === 'pending' ? 'Mover' : 'Saltar', 'rb'],
    ['View', 'Pestaña', 'view'],
  ]} />
{:else if tab !== 'all'}
  <Hints list={[['B', 'Volver', 'b'], ['View', 'Pestaña', 'view']]} />
{:else if titles.length === 0}
  <Hints list={[['A', saga ? 'Crear saga' : 'Añadir juego', 'a'], ['B', 'Volver', 'b'], ...(saga ? [] : [['View', 'Pestaña', 'view'] as [string, string, string]])]} />
{:else}
  <Hints list={[
    ['A', 'Abrir', 'a'], ['B', 'Volver', 'b'],
    ...(grid ? [['LB/RB', 'Página', 'rb'] as [string, string, string]] : []),
    ...(saga ? [] : [['LT/RT', 'Filtro', 'rt'] as [string, string, string], ['X', 'Orden', 'x'] as [string, string, string], ['View', 'Pestaña', 'view'] as [string, string, string]]),
    ...(grid ? [['Stick D', 'Tamaño'] as [string, string]] : []),
    ['Y', grid ? 'Carrusel' : 'Cuadrícula', 'y'],
  ]} />
{/if}

<style>
  /* «Mi lista»: zona de las estanterías (recorta lo que se desplaza fuera, a ancho completo para no cortar las sombras). */
  .lclip{position:absolute;left:0;top:166px;width:1280px;height:510px;overflow:hidden}
</style>
