<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { gdirAct, gdirExe, GDIR_ROWS, type GdirRow } from '../lib/ops.svelte';
  import { cmd } from '../lib/controller.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const g = $derived(app.gdir);
  const it = $derived(g.items[g.i]);
  const RH = 46;
  const H = $derived(28 + 44 + 70 + GDIR_ROWS.length * (RH + 6) + 74);
  const plat = $derived(app.config.platforms.find((p) => p.id === it?.platform)?.name ?? it?.platform ?? '');
  const short = (p: string, n = 36) => (p.length > n ? '…' + p.slice(-(n - 1)) : p);
  const L: Record<GdirRow, string> = {
    exe: 'Ejecutable', name: 'Nombre', platform: 'Plataforma', add: 'Añadir y seguir', skip: 'Saltar esta carpeta', finish: 'Terminar',
  };
  const text = (r: GdirRow): string => {
    if (!it) return '';
    if (r === 'exe') {
      const e = gdirExe(it);
      const n = it.exes.length + (it.custom ? 1 : 0);
      return e ? `${n > 1 ? '◀ ' : ''}${short(e.replace(/^.*[\/]([^\/]+[\/][^\/]+)$/, '$1'), 30)}${n > 1 ? ' ▶' : ''}` : 'Elegir…';
    }
    if (r === 'name') return it.name || 'Escribir…';
    if (r === 'platform') return `◀ ${plat} ▶`;
    return '';
  };

  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { g.row = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) cmd('b'); }}>
  <div class="glass formpanel" style="height:{g.loading || !it ? 220 : H}px;top:{(720 - (g.loading || !it ? 220 : H)) / 2}px">
    <div class="face">
      {#if g.loading || !it}
        <h2>Carpeta con juegos</h2>
        <p class="fhint">Buscando ejecutables en las subcarpetas…</p>
      {:else}
        <h2>Juego {g.i + 1} de {g.items.length}</h2>
        <p class="fhint" style="margin:0 0 4px">
          Carpeta <b>{it.folder}</b>{#if it.exes.length > 1} · {it.exes.length} ejecutables posibles{/if}
        </p>
        {#each GDIR_ROWS as r, i}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="srow frow" class:f={g.row === i} class:save={r === 'add'} style="height:{RH}px" data-frow={r}
            onmousemove={(e) => hover(e, i)} onclick={() => { g.row = i; void gdirAct(i, 0); }}>
            <span class="sl fl">{L[r]}</span>
            {#if r === 'exe' || r === 'name' || r === 'platform'}<span class="sv" class:ph={r === 'name' && !it.name}>{text(r)}</span>{/if}
          </div>
        {/each}
        <p class="fhint">
          ◀ ▶ en «Ejecutable» recorre los .exe de la carpeta; A lo busca a mano.
          {#if g.added}<br />Añadidos hasta ahora: <b>{g.added}</b>{/if}
          {#if g.known || g.empty}<br />Omitidas: {g.known} ya añadidas · {g.empty} sin ejecutables{/if}
        </p>
      {/if}
    </div>
  </div>
  <Hints list={[['A', 'Elegir', 'a'], ['X', 'Saltar', 'x'], ['B', 'Cerrar', 'b']]} />
</div>
