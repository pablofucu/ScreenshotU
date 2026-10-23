<script lang="ts">
  import { untrack } from 'svelte';
  import { app } from '../lib/store.svelte';
  import {
    applyImage, ART_KINDS, closeSgdbPicker, isSgdbPath, KIND_COLS, KIND_LABEL, pickTitle, setKind, sg, sgdbCmd, sgdbFocus, usedImageId,
  } from '../lib/sgdb.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const VIEW_H = 430;
  const GAP = 14;
  const INNER_W = 1052;
  const p = $derived(sg.pick);
  const t = $derived(pickTitle());
  const cols = $derived(KIND_COLS[p.kind]);
  const cellW = $derived((INNER_W - GAP * (cols - 1)) / cols);
  // Proporciones de SteamGridDB: portada 2:3, hero 1920x620, logo apaisado.
  const cellH = $derived(Math.round(p.kind === 'cover' ? cellW * 1.5 : p.kind === 'hero' ? cellW * (620 / 1920) : cellW * 0.5));
  const rowH = $derived(cellH + GAP);
  const rowsN = $derived(Math.ceil(p.items.length / cols));
  const total = $derived(rowsN * rowH);
  // Desplazamiento mínimo para que la fila enfocada se vea entera (el padding de la cuadrícula es 8 + 8).
  let off = $state(0);
  $effect(() => {
    const top = Math.floor(p.i / cols) * rowH;
    let o = untrack(() => off);
    if (top < o) o = top;
    else if (top + cellH + 16 > o + VIEW_H) o = top + cellH + 16 - VIEW_H;
    off = Math.max(0, Math.min(o, total + 16 - VIEW_H));
  });
  const used = $derived(usedImageId(t, p.kind));
  const own = $derived(!!t && !!t[p.kind] && !isSgdbPath(t[p.kind]));
  const cur = $derived(p.items[p.i]);
  const GROW_H = 52;
  const goff = $derived(Math.max(0, Math.min(p.gi - 3, p.games.length - Math.floor(VIEW_H / GROW_H))) * GROW_H);

  const moved = makeMoved();
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeSgdbPicker(); }}>
  <div class="glass setpanel sgpanel" style="--acc:{app.chrome.c}">
    <div class="face">
      <div class="sghead">
        <div class="sgtitle">
          <h2>{t?.name ?? ''}</h2>
          <p class="msub">
            {#if own}La carpeta del juego tiene su propio {KIND_LABEL[p.kind].toLowerCase()}: tiene prioridad sobre SteamGridDB y el PC.
            {:else if p.game}SteamGridDB: <b>{p.game.name}</b> · Y para buscar otro juego · View para elegir una imagen del PC
            {:else}SteamGridDB · View para elegir una imagen del PC{/if}
          </p>
        </div>
        <div class="sgtabs">
          <span class="btn sh">LB</span>
          {#each ART_KINDS as k}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="sgtab" class:on={p.kind === k && p.mode === 'images'} onclick={() => setKind(k)}>{KIND_LABEL[k]}</div>
          {/each}
          <span class="btn sh">RB</span>
        </div>
      </div>

      <div class="sgview" style="height:{VIEW_H}px">
        {#if p.mode === 'games'}
          <div class="strack" style="transform:translateY({-goff}px)">
            {#each p.games as g, i (g.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="srow" class:f={p.gi === i} onmousemove={(e) => { if (moved(e)) p.gi = i; }} onclick={() => { p.gi = i; sgdbCmd('a'); }}>
                <span class="sl">{g.name}</span>
                <span class="sv">{g.year ?? ''}{g.verified ? ' · verificado' : ''}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="sggrid" style="grid-template-columns:repeat({cols},{cellW}px);grid-auto-rows:{cellH}px;gap:{GAP}px;transform:translateY({-off}px)">
            {#each p.items as img, i (img.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div
                class="sgc k-{p.kind}" class:f={p.i === i} class:used={used === img.id} data-sg={i}
                onmousemove={(e) => { if (moved(e) && p.i !== i) sgdbFocus(i); }} onclick={() => { sgdbFocus(i); void applyImage(i); }}
              >
                <img src={img.thumb} alt="" loading="lazy" decoding="async" draggable="false" />
                {#if used === img.id}<span class="sgused">En uso</span>{/if}
              </div>
            {/each}
          </div>
        {/if}
        {#if p.loading && !(p.mode === 'images' && p.items.length)}<p class="sgmsg">Buscando en SteamGridDB…</p>
        {:else if p.error}<p class="sgmsg" style="white-space:pre-line">{p.error}</p>
        {:else if p.mode === 'images' && !p.items.length && p.game}<p class="sgmsg">SteamGridDB no tiene ningún {KIND_LABEL[p.kind].toLowerCase()} para «{p.game.name}».<br />Prueba con otro juego (Y) o elige una imagen del PC (View).</p>{/if}
      </div>

      <div class="sgfoot">
        {#if p.mode === 'games'}
          <span>Elige el juego correcto: sus imágenes sustituyen a las de la búsqueda automática.</span>
        {:else if cur}
          <span>{p.i + 1} de {p.items.length}{p.more ? '+' : ''}</span>
          <span>{cur.width}×{cur.height}{cur.style ? ` · ${cur.style}` : ''}{cur.author ? ` · por ${cur.author}` : ''}</span>
          {#if p.busy}<span class="chip">Descargando…</span>{/if}
        {/if}
      </div>
    </div>
  </div>
  <Hints list={p.mode === 'games'
    ? [['A', 'Elegir juego', 'a'], ['Y', 'Buscar', 'y'], ['B', 'Volver', 'b']]
    : [['A', 'Usar', 'a'], ['View', 'Del PC', 'view'], ['X', 'Quitar', 'x'], ['Y', 'Otro juego', 'y'], ['LB/RB', 'Tipo', 'rb'], ['B', 'Atrás', 'b']]} />
</div>
