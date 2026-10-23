<script lang="ts">
  // Selector de juegos de una colección (a pantalla completa): buscador con teclado en pantalla, cuadrícula de portadas
  // de la categoría con check en las elegidas y filtro «Todos / Seleccionados». Ventana virtual por filas.
  import { app, accentOf } from '../lib/store.svelte';
  import { ICON, RAINBOW } from '../lib/icons';
  import { inkDark } from '../lib/oklch';
  import { PICK_COLS, PICK_ROWS, pickerAct, pickerResults, pickerSearch, pickerSetFilter, platformGames } from '../lib/coleccion.svelte';
  import { gameYear } from '../lib/collections';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';

  const p = $derived(app.picker);
  const f = $derived(app.collForm);
  const plat = $derived(app.lib.platforms.find((x) => x.id === f.platform));
  const res = $derived(pickerResults());
  const total = $derived(platformGames(f.platform).length);
  const acc = $derived(f.color !== null ? accentOf(RAINBOW[f.color].c, RAINBOW[f.color].dark) : accentOf(app.chrome.c, inkDark(app.chrome.c)));
  const sel = $derived(new Set(p.ids));

  // Cada celda: portada y hueco; la cuadrícula se desplaza para que la fila enfocada quede en el centro.
  const CW = 118, CH = 158, GX = 10, PY = 168, X0 = 69;
  const rowsN = $derived(Math.ceil(res.length / PICK_COLS));
  const off = $derived(Math.max(0, Math.min(Math.floor(p.at / PICK_COLS) - 1, rowsN - PICK_ROWS)));
  const lo = $derived(Math.max(0, (off - 1) * PICK_COLS));
  const hi = $derived(Math.min(res.length, (off + PICK_ROWS + 1) * PICK_COLS));
  const win = $derived(Array.from({ length: Math.max(0, hi - lo) }, (_, k) => lo + k));
  const cur = $derived(res[p.at]);
  const coverOf = (i: number) => res[i].cover ?? res[i].shots[0]?.path ?? null;

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; p.at = i; }
  }
</script>

<div class="scrim pkscrim" role="presentation" style="--acc:{acc.c};--acc-ink:{acc.dark ? '#3a2f00' : '#fff'}">
  <div class="pkhead">
    <h2>Elige los juegos</h2>
    <p>{f.name || 'Nueva colección'} · {plat?.name ?? ''}</p>
  </div>
  <div class="pkbar">
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="pksearch" onclick={pickerSearch}>
      <span class="pkic">{@html ICON.search}</span>
      <b class:ph={!p.q}>{p.q || 'Buscar juego…'}</b>
      <span class="btn y">Y</span>
    </div>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span class="tflt" class:on={p.filter === 'all'} onclick={() => pickerSetFilter('all')}>Todos<small>{total}</small></span>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <span class="tflt" class:on={p.filter === 'sel'} onclick={() => pickerSetFilter('sel')}>Seleccionados<small>{p.ids.length}</small></span>
    <span class="pkcount">{p.q ? `${res.length} resultados` : `${res.length} ${res.length === 1 ? 'juego' : 'juegos'}`}</span>
  </div>
  <div class="pkgrid" style="height:{PICK_ROWS * PY}px">
    {#if !res.length}
      <p class="pkempty">{p.filter === 'sel' && !p.ids.length ? 'Aún no has marcado ningún juego.' : 'Ningún juego coincide con la búsqueda.'}</p>
    {/if}
    <div class="pktrack" style="transform:translateY({-off * PY}px)">
      {#each win as i (res[i].id)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="glass pkcov" class:f={p.at === i} class:on={sel.has(res[i].id)} data-pk={i}
          style="left:{X0 - 69 + (i % PICK_COLS) * (CW + GX)}px;top:{Math.floor(i / PICK_COLS) * PY + 6}px;width:{CW}px;height:{CH}px;--r:16px;--bw:6px"
          onmousemove={(e) => hover(e, i)} onclick={() => pickerAct(i)}>
          <div class="face">
            {#if coverOf(i)}<Thumb path={coverOf(i)!} />{:else}<div class="noart"><b>{res[i].name}</b></div>{/if}
            {#if sel.has(res[i].id)}<span class="pkchk">{@html ICON.check}</span>{/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
  <div class="pkname">
    {#if cur}<b>{cur.name}</b><span>{gameYear(cur) ?? 'sin fecha'}{sel.has(cur.id) ? ' · en la colección' : ''}</span>{/if}
  </div>
  <Hints list={[
    ['A', 'Marcar', 'a'], ['X', 'Marcar todos', 'x'], ['Y', 'Buscar', 'y'], ['LT/RT', 'Filtro', 'rt'], ['LB/RB', 'Página', 'rb'], ['Start', 'Guardar', 'menu'], ['B', 'Atrás', 'b'],
  ]} />
</div>

<style>
  .pkscrim{background:rgba(236,238,242,.95)}
  .pkhead{position:absolute;left:69px;top:26px}
  .pkhead h2{margin:0;font-size:30px;font-weight:800;color:#3a414e}
  .pkhead p{margin:2px 0 0;font-size:16px;font-weight:600;color:#5f6776;max-width:760px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .pkbar{position:absolute;left:69px;right:69px;top:92px;display:flex;align-items:center;gap:10px}
  .pksearch{display:flex;align-items:center;gap:10px;width:380px;height:44px;padding:0 8px 0 14px;border-radius:999px;cursor:pointer;margin-right:10px;
    background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(240,243,248,.85));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.28),0 8px 16px -10px rgba(40,50,70,.4)}
  .pksearch b{flex:1;font-size:18px;font-weight:700;color:#3a414e;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .pksearch b.ph{color:#8a93a3;font-weight:600}
  .pkic{width:22px;height:22px;flex:none;border-radius:7px;display:grid;place-items:center;background:var(--acc)}
  .pkic :global(svg){width:70%;height:70%}
  .tflt{cursor:pointer}
  .pkcount{margin-left:auto;font-size:15px;font-weight:700;color:#5f6776}
  .pkgrid{position:absolute;left:0;right:0;top:150px;overflow:hidden}
  .pktrack{position:absolute;left:69px;top:0;transition:transform .22s cubic-bezier(.25,1,.4,1)}
  .pkcov{--r:16px;--bw:6px}
  .pkcov .face{border-radius:11px}
  .pkcov.on{box-shadow:inset 0 1.5px 0 #fff,0 0 0 2px #3BB54A,0 8px 16px -8px rgba(59,181,74,.7)}
  .pkchk{position:absolute;right:6px;top:6px;width:30px;height:30px;border-radius:50%;display:grid;place-items:center;
    background:linear-gradient(180deg,#6fd58a,#2fa653);box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 3px 8px -2px rgba(0,0,0,.5);z-index:3}
  .pkchk :global(svg){width:62%;height:62%}
  .pkempty{position:absolute;left:0;right:0;top:150px;text-align:center;font-size:20px;font-weight:700;color:#5f6776;margin:0}
  .pkname{position:absolute;right:69px;top:34px;display:flex;flex-direction:column;align-items:flex-end;gap:2px;max-width:420px;white-space:nowrap}
  .pkname b{font-size:22px;font-weight:800;color:#3a414e;max-width:100%;overflow:hidden;text-overflow:ellipsis}
  .pkname span{font-size:15px;font-weight:700;color:#5f6776}
  @media (prefers-reduced-motion: reduce){.pktrack{transition:none}}
</style>
