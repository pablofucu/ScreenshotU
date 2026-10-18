<script lang="ts">
  import { app } from '../lib/store.svelte';
  import Hints from './Hints.svelte';

  type PickerRow = { l: string; v?: string; danger?: boolean };
  let {
    title, sub = '', rows, row, empty = '', visible = 8, onhover, onpick, onclose, hints,
  }: {
    title: string; sub?: string; rows: PickerRow[]; row: number; empty?: string; visible?: number;
    onhover: (i: number) => void; onpick: (i: number) => void; onclose: () => void; hints: Array<[string, string, string?]>;
  } = $props();

  const ROW_H = 52;
  // Alto a medida: cabecera + filas visibles (o el aviso de lista vacía), centrado en el escenario.
  const H = $derived(rows.length ? 28 + 104 + Math.min(rows.length, visible) * ROW_H + 28 : 250);
  const off = $derived(Math.max(0, Math.min(row - 3, rows.length - visible)));
  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; onhover(i); }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="glass setpanel pickpanel" style="--acc:{app.chrome.c};height:{H}px;top:{(720 - H) / 2}px">
    <div class="face">
      <h2>{title}</h2>
      <p class="msub">{sub}</p>
      {#if rows.length === 0}
        <p class="pickempty">{empty}</p>
      {:else}
        <div class="slist" style="height:{visible * ROW_H + 8}px">
          <div class="strack" style="transform:translateY({-off * ROW_H}px)">
            {#each rows as r, i}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="srow" class:f={row === i} class:danger={r.danger} data-row={i} onmousemove={(e) => hover(e, i)} onclick={() => { onhover(i); onpick(i); }}>
                <span class="sl">{r.l}</span>
                <span class="sv">{r.v ?? ''}</span>
              </div>
            {/each}
          </div>
        </div>
        {#if rows.length > visible}
          <div class="sscroll" style="top:104px"><i style="height:{(visible / rows.length) * 100}%;top:{(off / rows.length) * 100}%"></i></div>
        {/if}
      {/if}
    </div>
  </div>
  <Hints list={hints} />
</div>
