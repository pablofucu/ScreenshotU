<script lang="ts">
  // Baldosa y hoja de mando: lista de mandos XInput conectados y prueba de vibración. Batería omitida según contrato.
  let {
    pads = 0,
    view = 'tile',
    onOpenSheet,
    onRumble,
  }: {
    pads: number;
    view?: 'tile' | 'sheet';
    onOpenSheet?: () => void;
    onRumble?: (port: number) => void;
  } = $props();

  const ports = [0, 1, 2, 3];
  const connected = $derived(ports.filter((p) => (pads & (1 << p)) !== 0));
</script>

{#if view === 'tile'}
  <div
    class="glass tile t-pad"
    style="--i:5"
    data-f
    data-id="pad"
    onclick={onOpenSheet}
    onkeydown={(e) => { if (e.key === 'Enter') onOpenSheet?.(); }}
    role="button"
    tabindex="0"
  >
    <div class="lbl">
      <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
      Mando
      <em>{connected.length > 0 ? `${connected.length} conectado${connected.length > 1 ? 's' : ''}` : 'Sin mandos'}</em>
    </div>
    <div class="pb" style="margin-top:auto">
      <b class="big">
        {#if connected.length === 0}
          0<small> mandos</small>
        {:else if connected.length === 1}
          P{connected[0] + 1}<small> conectado</small>
        {:else}
          {connected.length}<small> mandos</small>
        {/if}
      </b>
    </div>
    <div class="sub">
      <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
      Identificar y puertos
    </div>
  </div>
{:else}
  <div class="vpad">
    {#each ports as p}
      {#if (pads & (1 << p)) !== 0}
        <div class="padcard">
          <div class="padart">
            <svg viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
          </div>
          <div style="flex:1;min-width:0">
            <b class="big2">Mando {p + 1}</b>
            <div class="sub">Puerto XInput {p} · Conectado</div>
          </div>
          <div
            class="btn sm"
            data-f
            data-id="pad-rumble-{p}"
            onclick={() => onRumble?.(p)}
            onkeydown={(e) => { if (e.key === 'Enter') onRumble?.(p); }}
            role="button"
            tabindex="0"
          >
            <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
            Vibrar para reconocerlo
          </div>
        </div>
      {:else}
        <div class="row dim">
          <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
          <span><b>Mando {p + 1}</b><small>Puerto {p} · no conectado</small></span>
        </div>
      {/if}
    {/each}
  </div>
{/if}
