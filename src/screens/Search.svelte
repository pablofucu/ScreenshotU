<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { closeOverlay } from '../lib/controller.svelte';
  import { srch, searchGroups, setSearchQ, openSearchResult, playSearchResult } from '../lib/search.svelte';
  import Hints from '../components/Hints.svelte';
  import MiniCover from '../components/MiniCover.svelte';

  const groups = $derived(searchGroups());
  const cur = $derived(srch.r >= 0 ? groups[srch.r] : undefined);
  const canPlay = $derived(!!cur && !!app.lib.platforms[cur.pi]?.titles[cur.items[srch.c]]?.launchable);
  const hints = $derived<Array<[string, string, string]>>([
    ['A', srch.r < 0 ? 'Escribir' : 'Abrir', 'a'],
    ...(canPlay ? [['X', 'Jugar', 'x'] as [string, string, string]] : []),
    ['B', 'Cerrar', 'b'],
  ]);

  function onInput(e: Event) { setSearchQ((e.currentTarget as HTMLInputElement).value); }
  function focusRow(r: number, c: number) { srch.r = r; srch.c = c; }
</script>

<div class="scrim srch" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="srch-wrap">
    <div class="gsearch" class:f={srch.r < 0}>
      <span class="ts-ic">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="7"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
      </span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" placeholder="Buscar juego..." value={srch.q} oninput={onInput} onfocus={() => { srch.r = -1; }}
        spellcheck="false" autocomplete="off" autofocus />
      {#if srch.q}<button type="button" class="ts-x" onclick={() => setSearchQ('')} title="Borrar búsqueda">✕</button>{/if}
    </div>

    <div class="srch-list">
      {#each groups as g, r (g.pi)}
        <div class="srow-g" class:f={srch.r === r}>
          <h4>{g.name}</h4>
          <div class="srch-covs">
            {#each g.items as ti, c (ti)}
              {@const t = app.lib.platforms[g.pi].titles[ti]}
              <button type="button" class="srch-it" class:f={srch.r === r && srch.c === c} title={t.name}
                onmousemove={() => { if (srch.r !== r || srch.c !== c) focusRow(r, c); }}
                onclick={() => { focusRow(r, c); openSearchResult(g.pi, ti); }}
                oncontextmenu={(e) => { e.preventDefault(); focusRow(r, c); playSearchResult(g.pi, ti); }}>
                <MiniCover {t} acc={app.titleAcc(g.pi, ti).c} />
                <span>{t.name}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
      {#if srch.q.trim() && !groups.length}
        <p class="srch-none">Ningún juego coincide con «{srch.q.trim()}»</p>
      {:else if !srch.q.trim()}
        <p class="srch-none">Escribe el nombre de un juego.</p>
      {/if}
    </div>
  </div>
  <Hints list={hints} />
</div>

<style>
  .srch-wrap { position: absolute; inset: 40px 56px 90px; display: flex; flex-direction: column; gap: 18px; min-height: 0; }
  .gsearch { flex: none; display: flex; align-items: center; gap: 12px; height: 56px; padding: 0 20px; border-radius: 999px; background: var(--glass-chip); box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 1px var(--line); color: var(--ink-2); }
  .gsearch:focus-within, .gsearch.f { background: var(--face); box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 2px var(--acc); color: var(--acc); }
  .gsearch .ts-ic { display: grid; place-items: center; flex: none; }
  .gsearch .ts-ic svg { width: 22px; height: 22px; }
  .gsearch input { flex: 1; min-width: 0; border: none; background: transparent; font: inherit; font-size: 22px; font-weight: 700; color: var(--ink); outline: none; padding: 0; }
  .gsearch input::placeholder { color: var(--ink-3); font-weight: 600; }
  .gsearch .ts-x { border: none; background: var(--line); color: var(--ink-2); border-radius: 50%; width: 26px; height: 26px; cursor: pointer; font: inherit; font-size: 12px; }
  .srch-list { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 12px; }
  .srow-g { flex: none; padding: 12px 16px 14px; border-radius: var(--r-m); background: var(--panel); box-shadow: 0 0 0 1px var(--line); }
  .srow-g h4 { margin: 0 0 10px; font-size: var(--t-lg, 18px); font-weight: 800; color: var(--ink); }
  .srch-covs { display: flex; gap: 12px; overflow: hidden; }
  .srch-it { flex: none; width: 96px; display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 6px; border: 0; border-radius: var(--r-s, 12px); background: transparent; font: inherit; color: var(--ink-2); cursor: pointer; }
  .srch-it span:last-child { width: 100%; font-size: 12px; font-weight: 700; text-align: center; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .srch-it.f { background: var(--glass-chip); box-shadow: var(--ring); color: var(--ink); }
  .srch-none { margin: 24px 8px; font-size: 18px; font-weight: 700; color: var(--ink-2); }
</style>
