<script lang="ts">
  import { IS_TABLET } from '../lib/backend';
  import { app, accv, coverHue, LIST_COLOR, TITLE_FILTERS, TITLE_ORDER_LABEL, TITLES_TABS, type ListShelf } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtAgo, fmtDur, fmtShort, nCapturas } from '../lib/format';
  import { setFocus, titlesGrid, viewPos, setTitlesTab, cycleTitleOrder } from '../lib/controller.svelte';
  import { titleHex } from '../lib/collections';
  import { hexToRgb, rgbToLch } from '../lib/oklch';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import CollTab from './CollTab.svelte';
  import MiListaDual from './MiListaDual.svelte';
  import { buildBoard, computeMonthPositions, finShelfSub, itemLabel, LIST_VIS, SHELF_STEP, shelfSub } from '../lib/milista.svelte';
  import { collFocusAccent } from '../lib/coleccion.svelte';
  import { inkFor } from '../lib/ops.svelte';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const grid = $derived(app.config.titlesView === 'grid');
  // Lista visible (filtrada); la baldosa «Añadir juego» ocupa la posición siguiente a la última.
  const vis = $derived(app.tview);
  // La baldosa «Añadir juego» no existe en la tablet.
  const slots = $derived(vis.length + (IS_TABLET ? 0 : 1));
  // Medidas de la cuadrícula según el tamaño elegido (5 a 8 por fila).
  const G = $derived(titlesGrid());
  const PS = $derived(G.size);
  const pages = $derived(Math.max(1, Math.ceil(slots / PS)));
  const q = $derived(viewPos(app.t));
  const pg = $derived(Math.floor(q / PS));
  // Pestaña de la categoría: Todos, Mi lista o Colecciones.
  const tab = $derived(app.titlesTab);
  const hasListTitles = $derived(buildBoard().length > 0);
  // «Mi lista»: estanterías por estado. Se pintan la enfocada y sus vecinas; cada una con el color fijo de su estado.
  const shelves = $derived(app.shelves);
  const focusShelf = $derived(shelves.findIndex((s) => s.items.includes(app.t)));
  const listAcc = $derived(focusShelf >= 0 ? LIST_COLOR[shelves[focusShelf].kind] : app.chrome);
  const shelfRange = $derived([Math.max(0, app.listTop - 1), Math.min(shelves.length, app.listTop + 4)] as const);
  const shelfIdx = $derived(Array.from({ length: Math.max(0, shelfRange[1] - shelfRange[0]) }, (_, k) => shelfRange[0] + k));
  const shelfItems = (sh: ListShelf): ShelfItem[] =>
    sh.items.map((ti, k) => {
      const { label, stars } = itemLabel(sh, titles[ti], k);
      return { k: 'tit' + ti, act: 'tit:' + ti, path: coverOf(ti), name: titles[ti].name, label, stars, pip: app.pipStyle(app.p, ti) };
    });
  const finShelfItems = (sh: ListShelf): ShelfItem[] =>
    sh.items.map((ti, k) => ({
      k: 'tit' + ti, act: 'tit:' + ti, path: coverOf(ti), name: titles[ti].name, n: k + 1, stars: 0, pip: app.pipStyle(app.p, ti),
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
  // Las páginas se apilan en vertical (una pantalla de alto cada una), igual que la barra de la derecha.
  const PH = 720;
  // Posición y tamaño de la portada nº `j` de la página `page` (los textos y el marco fino se escalan con el tamaño).
  const cellStyle = (page: number, j: number) =>
    `left:${G.gx + (j % G.cols) * (G.cw + G.gap)}px;top:${page * PH + G.top + Math.floor(j / G.cols) * (G.ch + G.rg)}px;width:${G.cw}px;height:${G.ch}px;--r:${(16 * G.s).toFixed(1)}px;--bw:${(3 * G.s).toFixed(1)}px;--fr:${(13 * G.s).toFixed(1)}px;--ts:${(0.55 + 0.45 * G.s).toFixed(3)}`;

  // Juego enfocado en la pestaña «Colecciones» (o el de la lista en las demás).
  const ambTitle = $derived.by(() => {
    if (tab !== 'coll') return cur;
    const ti = app.collShelves[app.collAt.c]?.items[app.collAt.i];
    return ti === undefined ? undefined : titles[ti];
  });

  // Velo de color: tiñe el fondo de cuadraditos con los dos tonos del juego enfocado (al recorrer la lista, el fondo va cambiando de matiz).
  // Si no hay juego enfocado (baldosa «Añadir»), se conserva el último. Por color, el velo sigue la misma vuelta que
  // la interfaz (tono de la portada y el siguiente), así no salta entre portadas vecinas.
  const washTi = $derived(tab === 'coll' ? app.collShelves[app.collAt.c]?.items[app.collAt.i] : cur ? app.t : undefined);
  let wash = $state('');
  $effect(() => {
    if (tab === 'all' && app.titleOrder === 'color') { if (!app.overlay) wash = app.pairStyle(app.atmosTarget); return; }
    const ti = washTi;
    if (ti !== undefined && titles[ti]) wash = app.pipStyle(app.p, ti);
  });

  // Familia de color de un juego (misma regla que «Por color»: tono OKLCH del acento; sin color = Grises).
  const FAMS = ['Rojos', 'Naranjas', 'Amarillos', 'Verdes', 'Turquesas', 'Azules', 'Violetas', 'Rosas', 'Grises'];
  const famOf = (ti: number) => {
    const hex = titles[ti] ? titleHex(titles[ti]) : null;
    const rgb = hex ? hexToRgb(hex) : null;
    if (!rgb) return 8;
    const [, c, h] = rgbToLch(rgb);
    if (c < 0.04) return 8;
    if (h >= 325 || h < 20) return 7;
    return h < 45 ? 0 : h < 85 ? 1 : h < 115 ? 2 : h < 175 ? 3 : h < 225 ? 4 : h < 275 ? 5 : 6;
  };
  // Barra lateral como minimapa: con el orden «Por color», el tono de cada juego (el mismo que ordena la lista) con la
  // misma luz y croma para todos, así el degradado es continuo: empieza en el color de la categoría, da la vuelta
  // siempre hacia delante (interpolado en OKLCH, sin grises de mezcla) y los juegos sin color quedan en gris al fondo.
  // Con otro orden la barra es neutra: los colores saldrían a rayas sin sentido.
  // El tirador (70px) recorre 350px de los 420 de la barra, así que el color de cada posición va en ese tramo.
  const BAR_H = 420, THUMB = 70;
  // El gris lleva el tono del último color para que el paso al gris sea un simple apagado (sin barrer el arcoíris).
  const grey = (h = 255) => `oklch(0.8 0.01 ${h.toFixed(1)})`;
  const barCol = (h: number) => `oklch(0.74 0.14 ${h.toFixed(1)})`;
  const mapGrad = $derived.by(() => {
    const n = vis.length;
    if (n < 2 || app.titleOrder !== 'color') return '';
    const at = (pos: number) => `${(((THUMB / 2 + (pos / Math.max(1, slots - 1)) * (BAR_H - THUMB)) / BAR_H) * 100).toFixed(2)}%`;
    const hues = vis.map((ti) => coverHue(titles[ti]));
    // Los grises van todos detrás: `nc` juegos con color al principio.
    const nc = hues.findIndex((h) => h == null) < 0 ? n : hues.findIndex((h) => h == null);
    if (!nc) return grey();
    // Arriba del todo, el color de la categoría: la vuelta arranca siempre ahí aunque no haya juegos de ese tono.
    const stops: string[] = [`${barCol(app.secPair(app.p).h1)} 0%`];
    const S = Math.min(nc, 56);
    for (let k = 0; k < S; k++) {
      const pos = S < 2 ? 0 : Math.round((k * (nc - 1)) / (S - 1));
      stops.push(`${barCol(hues[pos]!)} ${at(pos)}`);
    }
    if (S < 2) stops.push(`${barCol(hues[0]!)} ${at(Math.max(0, nc - 1))}`);
    if (nc < n) stops.push(`${grey(hues[nc - 1]!)} ${at(nc)}`, `${grey(hues[nc - 1]!)} 100%`);
    return `linear-gradient(180deg in oklch increasing hue,${stops.join(',')})`;
  });
  let barEl = $state<HTMLElement>();
  let hov = $state<{ pos: number; y: number } | null>(null);
  /** Posición de la lista (0..slots-1) a la altura del puntero sobre la barra. */
  const barAt = (e: PointerEvent) => {
    const r = barEl!.getBoundingClientRect();
    const y = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height)) * BAR_H;
    const f = Math.min(1, Math.max(0, (y - THUMB / 2) / (BAR_H - THUMB)));
    return { pos: Math.round(f * Math.max(1, slots - 1)), y };
  };
  const barJump = (pos: number) => { const i = realAt(pos); if (app.focus !== 'tit' + i) setFocus('tit' + i); };
  const barDown = (e: PointerEvent) => {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const a = barAt(e); hov = a; barJump(a.pos);
  };
  const barMove = (e: PointerEvent) => {
    const a = barAt(e); hov = a;
    if (e.buttons & 1) barJump(a.pos);
  };
  const hovTi = $derived(hov ? realAt(hov.pos) : -1);
  // Con el orden «Por color»: familias de la primera y la última portada de la página («Página 2/3 · Verdes–Azules»).
  const pageFam = $derived.by(() => {
    if (app.titleOrder !== 'color' || !grid || tab !== 'all' || pages < 2) return '';
    const a = pg * PS, b = Math.min(vis.length - 1, a + PS - 1);
    if (a > b) return '';
    const f1 = FAMS[famOf(vis[a])], f2 = FAMS[famOf(vis[b])];
    return `Página ${pg + 1}/${pages} · ${f1 === f2 ? f1 : f1 + '–' + f2}`;
  });
</script>

{#snippet cell(i: number)}
  <div class="face">
    {#if i === titles.length}
      <div class="addin"><span>{@html ICON.plus}</span><b>Añadir juego</b></div>
    {:else}
      {#if coverOf(i)}<Thumb path={coverOf(i)!} />{:else}<div class="noart"><b>{titles[i].name}</b></div>{/if}{#if titles[i].hidden}<span class="hbadge">{@html ICON.eyeOff}</span>{/if}
    {/if}
  </div>
  <i class="ring"></i><i class="halo"></i>
{/snippet}

<div class="ambwash" style={wash}></div>
<TopBar crumbs={['Inicio', P?.name ?? '']} dots={grid && tab === 'all' ? pages : 0} dot={pg} />

<div class="ttabs" class:car={!grid && tab === 'all'} class:list-tabs={tab === 'list'} style={tabsAcc ? `--acc:${tabsAcc.c};--accv:${accv(tabsAcc)};--acc-ink:#fff` : ''}>
  <span class="btn sh" data-hact="lt">LT</span>
  {#each TITLES_TABS as [f, label]}
    <button type="button" class="tflt" class:on={tab === f} onclick={() => setTitlesTab(f)}>{label}</button>
  {/each}
  <span class="btn sh" data-hact="rt">RT</span>
  {#if tab === 'all' && titles.length > 1}
    <button type="button" class="tflt tord" onclick={cycleTitleOrder}>{TITLE_ORDER_LABEL[app.titleOrder]}</button>
  {/if}
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
    <span class="tag" style="background:{a.c};color:#fff">{chip}</span>
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
      <p>Crea una carpeta por juego dentro de <b>{P?.folder}</b> y copia dentro tus capturas (png, jpg, jpeg o webp){#if !IS_TABLET}, o añade un juego con la baldosa de abajo{/if}.</p>
    </div>
  </div>
  {#if !IS_TABLET}
  <div class="glass cover addtile fx" class:f={app.focus === 'tit0'} data-k="tit0" data-act="tit:0" style="left:552px;top:430px">
    <div class="face"><div class="addin"><span>{@html ICON.plus}</span><b>Añadir juego</b></div></div>
  </div>
  {/if}
{:else}
  {#if grid}
    {#if cur}
      {@const a = app.titleAcc(app.p, app.t)}
      <div class="thead">
        <h3>{cur.name}</h3>
        <span class="tag" style="background:{a.c};color:#fff">{cur.shots.length ? nCapturas(cur.shots.length) : 'Sin capturas'}</span>
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
    <div class="ttrack" style="width:1280px;height:{pages * PH}px;transform:translateY({-pg * PH}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        {@const j = pos % PS}
        <div class="glass cover tcov fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="{cellStyle(Math.floor(pos / PS), j)}{i < titles.length ? ';' + app.pipStyle(app.p, i) : ''}">
          {@render cell(i)}
        </div>
      {/each}
    </div>
    {#if tab === 'all' && pages > 1}
      <!-- Barra de desplazamiento vertical (las páginas bajan): por color es un minimapa con los colores de los juegos.
           Clic o arrastre = saltar a ese juego; el ratón encima enseña cuál es. -->
      <div class="sbar rb" bind:this={barEl} style="right:auto;left:1182px;top:214px;height:{BAR_H}px{mapGrad ? ';background:' + mapGrad : ''}">
        {#if app.titleOrder === 'az'}<em style="top:-20px">A</em>{/if}
        <i style="height:{THUMB}px;top:{(q / Math.max(1, slots - 1)) * (BAR_H - THUMB)}px"></i>
        {#if app.titleOrder === 'az'}<em style="bottom:-20px">Z</em>{/if}
        {#if hov}
          <div class="sbub glass-lite" style="top:{hov.y}px">
            {#if hovTi < titles.length}<span class="sdot" style="background:{titleHex(titles[hovTi]) ?? '#8a93a3'}"></span><b>{titles[hovTi].name}</b><small>{FAMS[famOf(hovTi)]}</small>{:else}<b>Añadir juego</b>{/if}
          </div>
        {/if}
        <div class="shit" role="slider" tabindex="-1" aria-label="Saltar a un juego" aria-valuenow={q} aria-valuemin={0} aria-valuemax={slots - 1}
          onpointerdown={barDown} onpointermove={barMove} onpointerleave={() => (hov = null)}></div>
      </div>
      {#if pageFam}<div class="pgfam pill">{pageFam}</div>{/if}
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
  {#if !hasListTitles}
    <Hints list={[
      ['A', 'Ir al Catálogo', 'a'],
      ['LT/RT', 'Pestaña', 'rt'],
      ['Start', 'Menú', 'menu'],
      ['B', 'Volver', 'b'],
    ]} />
  {:else}
    <Hints list={[
      ['A', 'Abrir', 'a'],
      ...(app.platform?.titles[app.t]?.launchable ? [['X', 'Jugar', 'x'] as H] : []),
      ['Y', 'Estado', 'y'],
      ['LT/RT', 'Pestaña', 'rt'],
      ['Start', 'Menú', 'menu'],
      ['B', 'Volver', 'b'],
    ]} />
  {/if}
{:else if tab === 'coll'}
  <Hints list={[
    ['A', app.collAt.c >= app.collShelves.length ? 'Nueva colección' : 'Abrir', 'a'],
    ...(app.collAt.c < app.collShelves.length ? [...(app.platform?.titles[app.collShelves[app.collAt.c]?.items[app.collAt.i]]?.launchable ? [['X', 'Jugar', 'x'] as H] : []), ['Y', 'Editar', 'y'] as H] : []),
    ['LT/RT', 'Pestaña', 'rt'],
    ['Start', 'Menú', 'menu'],
    ['B', 'Volver', 'b'],
  ]} />
{:else if titles.length === 0}
  <Hints list={[['A', 'Añadir juego', 'a'], ['LT/RT', 'Pestaña', 'rt'], ['B', 'Volver', 'b']]} />
{:else}
  <Hints list={[
    ['A', 'Abrir', 'a'],
    ...(cur?.launchable ? [['X', 'Jugar', 'x'] as H] : []),
    ...(tab === 'all' ? [['Y', 'Buscar', 'y'] as H, ['View', 'Vista', 'view'] as H] : [['Y', 'Menú', 'y'] as H]),
    ['LB/RB', 'Página', 'rb'],
    ['LT/RT', 'Pestaña', 'rt'],
    ['Start', 'Menú', 'menu'],
    ['B', 'Volver', 'b'],
  ]} />
{/if}
  <style>
  button.tflt { border: none; font-family: var(--round); font-size: var(--t-m); font-weight: 800; cursor: pointer; }
  button.tflt.tord { background: transparent; box-shadow: none; font-family: var(--round); font-size: var(--t-m); font-weight: 800; }
  button.btn.x { border: none; font-family: var(--round); font-size: var(--t-s); font-weight: 800; cursor: pointer; }
  .ttabs.list-tabs { top: 72px; }
  .tsearch {
    height: 32px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px 0 10px;
    margin-left: 10px;
    border-radius: var(--r-pill);
    background: var(--glass-chip);
    backdrop-filter: blur(10px);
    box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 1px var(--line), 0 2px 6px rgba(15, 23, 42, 0.05);
    transition: all 0.2s cubic-bezier(0.2, 0.8, 0.3, 1);
    color: var(--ink-2);
  }
  .tsearch:hover {
    background: var(--face-hi);
    box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 1px var(--line-2), 0 3px 8px rgba(15, 23, 42, 0.08);
  }
  .tsearch:focus-within {
    background: var(--face);
    box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 2px var(--acc), 0 4px 14px color-mix(in srgb,var(--acc) 25%,transparent);
    color: var(--acc);
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
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink);
    width: 145px;
    outline: none;
    padding: 0;
    transition: width 0.2s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .tsearch:focus-within input {
    width: 175px;
  }
  .tsearch input::placeholder {
    color: var(--ink-3);
    font-weight: 600;
    opacity: 0.9;
  }
  .tsearch .ts-x {
    border: none;
    background: var(--line);
    color: var(--ink-2);
    border-radius: 50%;
    width: 18px;
    height: 18px;
    font-size: var(--t-xs);
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
    background: var(--bad);
    color: #fff;
    transform: scale(1.1);
  }

</style>
