<script lang="ts">
  import { app, LIST_COLOR, STATUS_LABEL, TITLE_FILTERS, TITLE_ORDER_LABEL, TITLES_TABS, type ListShelf } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtAgo, fmtDur, fmtShort, nCapturas } from '../lib/format';
  import { untrack } from 'svelte';
  import { backend, decodeImg, thumbNow, thumbReady } from '../lib/backend';
  import { titlesGrid, viewPos } from '../lib/controller.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import CollTab from './CollTab.svelte';
  import MiListaDual from './MiListaDual.svelte';
  import { computeMonthPositions, finShelfSub, itemLabel, LIST_VIS, SHELF_STEP, shelfSub } from '../lib/milista.svelte';
  import { collFocusAccent } from '../lib/coleccion.svelte';
  import { inkFor } from '../lib/ops.svelte';

  const P = $derived(app.platform);
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
  // Pestaña de la categoría: Todos, Mi lista o Colecciones.
  const tab = $derived(app.titlesTab);
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
  const finShelfItems = (sh: ListShelf): ShelfItem[] =>
    sh.items.map((ti, k) => ({
      k: 'tit' + ti, act: 'tit:' + ti, path: coverOf(ti), name: titles[ti].name, n: k + 1, stars: 0,
    }));
  // «Terminados»: cuántos juegos se terminaron cada mes del año (barritas del rótulo).
  const monthCounts = (sh: ListShelf) => {
    const c = Array<number>(12).fill(0);
    for (const ti of sh.items) if (titles[ti].finishedMs) c[new Date(titles[ti].finishedMs!).getMonth()]++;
    return c;
  };
  // Icono de color del rótulo de cada estantería (el del estado).
  const svg = (d: string) => `<svg viewBox="0 0 48 48" fill="none" stroke="#fff" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
  const LIST_ICON: Record<ListShelf['kind'], string> = {
    playing: ICON.play,
    pending: svg('<circle cx="24" cy="24" r="16"/><path d="M24 15v10l6 4"/>'),
    finished: ICON.check,
    dropped: svg('<path d="M11 24h26"/>'),
    none: svg('<path d="M10 14h28M10 24h28M10 34h28"/>'),
  };
  const curShelf = $derived(focusShelf >= 0 ? shelves[focusShelf] : null);
  const cur = $derived(titles[app.t]);
  const coverOf = (i: number) => titles[i].cover ?? titles[i].shots[0]?.path ?? null;
  const realAt = (pos: number) => (pos < vis.length ? vis[pos] : titles.length);
  const favCount = $derived(cur ? cur.shots.reduce((n, s) => n + (app.favs.has(s.path) ? 1 : 0), 0) : 0);
  // Filtro activo de «Todos» (Start › Filtrar): solo se enseña como un chip junto al orden.
  const filtered = $derived(app.titleFilter !== 'all');
  const filterName = $derived(TITLE_FILTERS.find(([f]) => f === app.titleFilter)?.[1] ?? '');
  // Acento de la barra de pestañas: el del estado (Mi lista), el de la colección enfocada (Colecciones) o el del juego enfocado (cuadrícula de «Todos»).
  type H = [string, string, string];
  const tabsAcc = $derived((tab === 'list' || tab === 'fin') ? listAcc : tab === 'coll' ? collFocusAccent() : null);

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

  // Fondo ambiental: el hero (o la portada) del juego enfocado, muy desenfocado, con fundido cruzado entre juegos.
  // La capa nueva entra encima (ya decodificada) y las de debajo se quitan solo cuando acabó de entrar; si no hay
  // imagen, el fondo se apaga con un fundido en vez de vaciarse de golpe.
  const ambTitle = $derived.by(() => {
    if (tab !== 'coll') return cur;
    const ti = app.collShelves[app.collAt.c]?.items[app.collAt.i];
    return ti === undefined ? undefined : titles[ti];
  });
  const ambSrc = (t: typeof cur | undefined) => (t ? (t.hero ?? t.cover ?? t.shots[0]?.path ?? null) : null);
  let lid = 0;
  const firstAmb = untrack(() => { const p = ambSrc(ambTitle); const u = p ? thumbNow(p) : undefined; return u && thumbReady(u) ? u : ''; });
  // `still`: capa pintada al montar con una imagen ya cargada (sin fundido de entrada).
  let layers = $state<Array<{ u: string; id: number; still?: boolean }>>(firstAmb ? [{ u: firstAmb, id: ++lid, still: true }] : []);
  let ambOff = $state(false);
  $effect(() => {
    const src = ambSrc(ambTitle);
    const ctl = new AbortController();
    // En la pestaña «Colecciones», la baldosa «Nueva colección» no tiene juego: se conserva el último fondo.
    if (!src) {
      if (tab !== 'coll' || !untrack(() => layers.length)) ambOff = true;
      return;
    }
    const t = setTimeout(() => {
      backend.thumb(src, ctl.signal).then(async (u) => {
        await decodeImg(u);
        if (ctl.signal.aborted) return;
        ambOff = false;
        if (untrack(() => layers.at(-1)?.u) === u) return;
        layers = [...untrack(() => layers).slice(-2), { u, id: ++lid }];
      }, () => {});
    }, 90);
    return () => { clearTimeout(t); ctl.abort(); };
  });
  /** La capa `id` terminó de entrar: las de debajo ya no se ven. */
  const settled = (id: number) => { if (layers.some((l) => l.id < id)) layers = layers.filter((l) => l.id >= id); };
</script>

{#snippet cell(i: number)}
  <div class="face">
    {#if i === titles.length}
      <div class="addin"><span>{@html ICON.plus}</span><b>Añadir juego</b></div>
    {:else}
      {#if coverOf(i)}<Thumb path={coverOf(i)!} />{:else}<div class="noart"><b>{titles[i].name}</b></div>{/if}<i class="pip"></i>{#if titles[i].hidden}<span class="hbadge">{@html ICON.eyeOff}</span>{/if}{#if titles[i].status === 'finished' || titles[i].status === 'dropped'}<span class="sbadge {titles[i].status}" style="--c:{LIST_COLOR[titles[i].status!].c}" title={STATUS_LABEL[titles[i].status!]}>{#if titles[i].status === 'finished'}{@html ICON.check}{:else}<i></i>{/if}</span>{/if}
    {/if}
  </div>
{/snippet}

<div class="amb" class:car={!grid} class:off={ambOff}>
  {#each layers as l (l.id)}<div class="amblayer" class:still={l.still} style="background-image:url({l.u})" onanimationend={() => settled(l.id)}></div>{/each}
</div>
<TopBar crumbs={['Inicio', P?.name ?? '']} dots={grid && tab === 'all' ? pages : 0} dot={pg} />

<div class="ttabs" class:car={!grid && tab === 'all'} class:list-tabs={tab === 'list'} style={tabsAcc ? `--acc:${tabsAcc.c};--acc-ink:${tabsAcc.dark ? '#3a2f00' : '#fff'}` : ''}>
  <span class="btn sh" data-hact="lt">LT</span>
  {#each TITLES_TABS as [f, label]}
    <span class="tflt" class:on={tab === f}>{label}</span>
  {/each}
  <span class="btn sh" data-hact="rt">RT</span>
  {#if tab === 'all' && titles.length > 1}<span class="btn x" data-hact="x">X</span><span class="tflt tord">{TITLE_ORDER_LABEL[app.titleOrder]}</span>{/if}
  {#if tab === 'all' && filtered}<span class="tflt tchip" data-hact="nofilter" title="Quitar el filtro">{filterName}<small>{vis.length}</small><b aria-label="Quitar el filtro">✕</b></span>{/if}
  {#if tab === 'all'}
    <div class="tsearch" class:has-val={Boolean(app.catalogSearch)}>
      <span class="ts-ic">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="7"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
      </span>
      <input
        type="text"
        placeholder="Buscar juego..."
        bind:value={app.catalogSearch}
        spellcheck="false"
        autocomplete="off"
      />
      {#if app.catalogSearch}
        <button type="button" class="ts-x" onclick={() => (app.catalogSearch = '')} title="Borrar búsqueda">✕</button>
      {/if}
    </div>
  {/if}
</div>

{#snippet fhead(t: NonNullable<typeof cur>, a: { c: string; dark?: boolean }, chip: string)}
  <div class="thead">
    <h3>{t.name}</h3>
    <span class="chip" style="background:{a.c};color:{a.dark ? '#3a2f00' : '#fff'}">{chip}</span>
    {#if t.playSeconds >= 60}<span>Jugado <b>{fmtDur(t.playSeconds)}</b></span>{/if}
    {#if t.lastPlayed}<span>Última partida <b>{fmtAgo(t.lastPlayed).toLowerCase()}</b></span>
    {:else if t.shots.length}<span>Última captura <b>{fmtShort(t.shots[0].date)}</b></span>{/if}
    {#if t.shots.length}<span>{nCapturas(t.shots.length)}</span>{/if}
  </div>
{/snippet}

{#if tab === 'list'}
  <MiListaDual />
{:else if tab === 'coll'}
  {#if ambTitle && app.collShelves[app.collAt.c]}
    {@const csh = app.collShelves[app.collAt.c]}
    {@render fhead(ambTitle, collFocusAccent(), `${csh.coll.name} · ${app.collAt.i + 1} de ${csh.items.length}`)}
  {/if}
  <CollTab />
{:else if titles.length === 0}
  <div class="glass note-card" style="left:300px;top:170px">
    <div class="face">
      <h2>Aún no hay juegos en {P?.name}</h2>
      <p>Crea una carpeta por juego dentro de <b>{P?.folder}</b> y copia dentro tus capturas (png, jpg, jpeg o webp), o añade un juego con la baldosa de abajo.</p>
    </div>
  </div>
  <div class="glass cover addtile fx" class:f={app.focus === 'tit0'} data-k="tit0" data-act="tit:0" style="left:552px;top:430px">
    <div class="face"><div class="addin"><span>{@html ICON.plus}</span><b>Añadir juego</b></div></div>
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
      <div class="thead"><h3>Ningún juego</h3><span>con el filtro «{filterName}» (Start › Filtrar para cambiarlo)</span></div>
    {:else}
      <div class="thead"><h3>Añadir juego</h3><span>Un ejecutable, un emulador o una carpeta de capturas</span></div>
    {/if}
  {/if}
  {#if grid}
    <div class="ttrack" style="width:{pages * 1280}px;transform:translateX({-pg * 1280}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        {@const j = pos % PS}
        <div class="glass cover tcov fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="{cellStyle(Math.floor(pos / PS), j)}{i < titles.length ? ';' + app.pipStyle(app.p, i) : ''}">
          {@render cell(i)}
        </div>
      {/each}
    </div>
    <div class="side" class:off={pg === 0} data-hact="lb" style="left:12px">{@html ICON.arrowL}</div>
    <div class="side" class:off={pg >= pages - 1} data-hact="rb" style="right:12px">{@html ICON.arrowR}</div>
    {#if tab === 'all' && pages > 1}
      <!-- Barra de desplazamiento: tira del arcoíris de principio a fin de la lista; el tirador sigue al enfocado. -->
      <div class="sbar rb" style="right:auto;left:1182px;top:214px;height:420px">
        {#if app.titleOrder === 'az'}<em style="top:-20px">A</em>{/if}
        <i style="height:70px;top:{(q / Math.max(1, slots - 1)) * 350}px"></i>
        {#if app.titleOrder === 'az'}<em style="bottom:-20px">Z</em>{/if}
      </div>
    {/if}
  {:else}
    <div class="ctrack" style="transform:translateX({640 - 100 - q * 240}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        <div class="glass cover ccover fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="left:{pos * 240}px{i < titles.length ? ';' + app.pipStyle(app.p, i) : ''}">
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
      <div class="cinfo"><h3>Ningún juego</h3><p>con el filtro «{filterName}» (Start › Filtrar para cambiarlo)</p></div>
    {:else}
      <div class="cinfo"><h3>Añadir juego</h3><p>Un ejecutable, un emulador o una carpeta de capturas</p></div>
    {/if}
  {/if}
{/if}

<!-- Pistas: solo lo principal de cada pestaña; el resto (filtro, tamaño, audio, Ajustes) está en Start. -->
{#if tab === 'list'}
  <Hints list={[
    ['A', 'Abrir', 'a'],
    ['X', 'Fijar', 'x'],
    ['Y', 'Opciones', 'y'],
    ['LB/RB', 'Pestaña', 'rb'],
    ['Start', 'Menú', 'menu'],
    ['B', 'Volver', 'b'],
  ]} />
{:else if tab === 'coll'}
  <Hints list={[
    ['A', app.collAt.c >= app.collShelves.length ? 'Nueva colección' : 'Abrir', 'a'],
    ...(app.collAt.c < app.collShelves.length ? [['X', 'Jugar / Capturas', 'x'] as H, ['Y', 'Editar', 'y'] as H] : []),
    ['LB/RB', 'Pestaña', 'rb'],
    ['Start', 'Menú', 'menu'],
    ['B', 'Volver', 'b'],
  ]} />
{:else if titles.length === 0}
  <Hints list={[['A', 'Añadir juego', 'a'], ['LB/RB', 'Pestaña', 'rb'], ['B', 'Volver', 'b']]} />
{:else}
  <Hints list={[
    ['A', 'Abrir', 'a'],
    ...(cur?.launchable ? [['X', 'Jugar', 'x'] as H] : []),
    ['Y', 'Buscar / Vista', 'y'],
    ['LB/RB', 'Pestaña', 'rb'],
    ['LT/RT', 'Página', 'rt'],
    ['Start', 'Menú', 'menu'],
    ['B', 'Volver', 'b'],
  ]} />
{/if}
  <style>
  .ttabs.list-tabs { top: 72px; }
  .tsearch {
    height: 32px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px 0 10px;
    margin-left: 10px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.76);
    backdrop-filter: blur(10px);
    box-shadow: inset 0 1px 0 #fff, 0 0 0 1px rgba(95, 110, 135, 0.22), 0 2px 6px rgba(15, 23, 42, 0.05);
    transition: all 0.2s cubic-bezier(0.2, 0.8, 0.3, 1);
    color: #64748b;
  }
  .tsearch:hover {
    background: rgba(255, 255, 255, 0.92);
    box-shadow: inset 0 1px 0 #fff, 0 0 0 1px rgba(95, 110, 135, 0.32), 0 3px 8px rgba(15, 23, 42, 0.08);
  }
  .tsearch:focus-within {
    background: #ffffff;
    box-shadow: inset 0 1px 0 #fff, 0 0 0 2px var(--acc, #00A0D8), 0 4px 14px rgba(0, 160, 216, 0.25);
    color: var(--acc, #00A0D8);
  }
  .tsearch .ts-ic {
    display: grid;
    place-items: center;
    flex: none;
    transition: color 0.2s;
  }
  .tsearch .ts-ic svg {
    width: 15px;
    height: 15px;
  }
  .tsearch input {
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: 13.5px;
    font-weight: 700;
    color: #2a313d;
    width: 145px;
    outline: none;
    padding: 0;
    transition: width 0.2s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .tsearch:focus-within input {
    width: 175px;
  }
  .tsearch input::placeholder {
    color: #8a93a3;
    font-weight: 600;
    opacity: 0.9;
  }
  .tsearch .ts-x {
    border: none;
    background: rgba(0, 0, 0, 0.08);
    color: #475569;
    border-radius: 50%;
    width: 18px;
    height: 18px;
    font-size: 10px;
    font-weight: 800;
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 0;
    flex: none;
    margin-right: -4px;
    transition: background 0.15s, color 0.15s, transform 0.1s;
  }
  .tsearch .ts-x:hover {
    background: #E8384F;
    color: #fff;
    transform: scale(1.1);
  }

</style>
