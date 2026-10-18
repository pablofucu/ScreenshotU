<script lang="ts">
  import { app, STATUS_LABEL, TITLE_FILTERS } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtAgo, fmtDur, fmtShort, nCapturas } from '../lib/format';
  import { backend } from '../lib/backend';
  import { PAGE_SIZE, viewPos } from '../lib/controller.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';

  const P = $derived(app.platform);
  const saga = $derived(P?.id === 'sagas');
  const titles = $derived(P?.titles ?? []);
  const grid = $derived(app.config.titlesView === 'grid');
  // Lista visible (filtrada); la baldosa «Añadir juego» ocupa la posición siguiente a la última.
  const vis = $derived(app.tview);
  const slots = $derived(vis.length + 1);
  const pages = $derived(Math.max(1, Math.ceil(slots / PAGE_SIZE)));
  const q = $derived(viewPos(app.t));
  const pg = $derived(Math.floor(q / PAGE_SIZE));
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
    az: titles.length,
  }));

  // Ventana virtual: solo se pintan la página actual y las vecinas (cuadrícula) o ±9 (carrusel).
  const range = $derived(
    grid
      ? [Math.max(0, (pg - 1) * PAGE_SIZE), Math.min(slots, (pg + 2) * PAGE_SIZE)]
      : [Math.max(0, q - 9), Math.min(slots, q + 10)],
  );
  const idx = $derived(Array.from({ length: Math.max(0, range[1] - range[0]) }, (_, k) => range[0] + k));
  const CW = 160, CH = 214, GAP = 44;
  const gx = (1280 - 5 * CW - 4 * GAP) / 2;

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
<TopBar crumbs={['Inicio', P?.name ?? '']} dots={grid ? pages : 0} dot={pg} />

{#if titles.length === 0}
  <div class="glass note-card" style="left:300px;top:130px;--acc:{app.chrome.c}">
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
    <div class="tfilters" class:car={!grid}>
      <span class="btn sh" data-hact="lt">LT</span>
      {#each TITLE_FILTERS as [f, label]}
        <span class="tflt" class:on={app.titleFilter === f}>{label}{#if f !== 'all' && f !== 'az'}<small>{counts[f]}</small>{/if}</span>
      {/each}
      <span class="btn sh" data-hact="rt">RT</span>
    </div>
  {/if}
  {#if grid}
    <div class="ttrack" style="width:{pages * 1280}px;transform:translateX({-pg * 1280}px)">
      {#each idx as pos (realAt(pos))}
        {@const i = realAt(pos)}
        {@const j = pos % PAGE_SIZE}
        <div class="glass cover tcov fx" class:addtile={i === titles.length} class:dim={titles[i]?.hidden} class:f={app.focus === 'tit' + i} data-k="tit{i}" data-act="tit:{i}" data-i={i}
          style="left:{Math.floor(pos / PAGE_SIZE) * 1280 + gx + (j % 5) * (CW + GAP)}px;top:{(saga ? 160 : 190) + Math.floor(j / 5) * (CH + 24)}px;--acc:{i === titles.length ? app.chrome.c : app.titleAcc(app.p, i).c}">
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

{#if titles.length === 0}
  <Hints list={[['A', saga ? 'Crear saga' : 'Añadir juego', 'a'], ['B', 'Volver', 'b']]} />
{:else}
  <Hints list={[['A', 'Abrir', 'a'], ['B', 'Volver', 'b'], ...(grid ? [['LB/RB', 'Página', 'rb'] as [string, string, string]] : []), ...(saga ? [] : [['LT/RT', 'Filtro', 'rt'] as [string, string, string]]), ['Y', grid ? 'Carrusel' : 'Cuadrícula', 'y']]} />
{/if}
