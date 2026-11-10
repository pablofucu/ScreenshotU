<script lang="ts">
  // Baldosa de capturas y hoja ampliada con vista previa y borrado diferido a la papelera.
  import { getThumbUrl } from './data';
  import { fmtShort } from '../lib/format';

  let {
    shots = [],
    view = 'tile',
    selectedShot = 0,
    hasUndo = false,
    onSelectShot,
    onOpenSheet,
    onCapture,
    onDeleteShot,
    onUndoDelete,
  }: {
    shots: Array<{ path: string; thumb: string; at: number }>;
    view?: 'tile' | 'sheet';
    selectedShot?: number;
    hasUndo?: boolean;
    onSelectShot?: (index: number) => void;
    onOpenSheet?: (index: number) => void;
    onCapture?: () => void;
    onDeleteShot?: (index: number) => void;
    onUndoDelete?: () => void;
  } = $props();

  let thumbCache = $state<Record<string, string>>({});

  $effect(() => {
    // En la baldosa sólo se ven las primeras: el resto no se pide al disco hasta abrir la hoja.
    for (const s of view === 'sheet' ? shots : shots.slice(0, 4)) {
      if (!s.thumb && !thumbCache[s.path]) {
        getThumbUrl(s.path).then((url) => {
          thumbCache[s.path] = url;
        });
      }
    }
  });

  function getImgSrc(s: { path: string; thumb: string }): string {
    return s.thumb || thumbCache[s.path] || '';
  }

  function relTime(at: number): string {
    const diff = Math.max(0, Date.now() - at);
    if (diff < 60000) return 'ahora';
    if (diff < 3600000) return `hace ${Math.floor(diff / 60000)} min`;
    if (diff < 86400000) return `hace ${Math.floor(diff / 3600000)} h`;
    if (diff < 172800000) return 'ayer';
    return fmtShort(at);
  }

  const curShot = $derived(shots[selectedShot] || shots[0] || null);
</script>

{#if view === 'tile'}
  <div class="glass tile t-shots" style="--i:2">
    <div class="lbl">
      <svg class="i" viewBox="0 0 24 24"><rect x="3" y="7" width="18" height="13" rx="3"/><path d="M8 7l2-3h4l2 3"/><circle cx="12" cy="13.5" r="3.5"/></svg>
      Últimas capturas
      <em>{shots.length > 0 ? `${shots.length} recientes` : 'Sin capturas'}</em>
    </div>
    <div class="strip">
      <div
        class="ctl capbtn"
        data-f
        data-id="cap"
        onclick={onCapture}
        onkeydown={(e) => { if (e.key === 'Enter') onCapture?.(); }}
        role="button"
        tabindex="0"
        title="Capturar pantalla (X)"
      >
        <svg class="i" viewBox="0 0 24 24"><rect x="3" y="7" width="18" height="13" rx="3"/><path d="M8 7l2-3h4l2 3"/><circle cx="12" cy="13.5" r="3.5"/></svg>
        Capturar
        <i class="gk x">X</i>
      </div>

      {#each shots.slice(0, 4) as s, i}
        {@const src = getImgSrc(s)}
        <div
          class="th"
          style={src ? `background-image:url("${src}")` : ''}
          data-f
          data-id="s{i}"
          onclick={() => onOpenSheet?.(i)}
          onkeydown={(e) => {
            if (e.key === 'Enter') onOpenSheet?.(i);
            if (e.key.toLowerCase() === 'y') onDeleteShot?.(i);
          }}
          role="button"
          tabindex="0"
        >
          <small>{relTime(s.at)}</small>
        </div>
      {/each}
    </div>
  </div>
{:else}
  {#if !curShot}
    <div class="ach-empty" style="font-size:var(--t-l);text-align:center;padding:40px">No hay capturas en esta partida</div>
  {:else}
    {@const curSrc = getImgSrc(curShot)}
    <div class="vshots">
      <div class="th prevw" style={curSrc ? `background-image:url("${curSrc}")` : ''}>
        <small>{relTime(curShot.at)}</small>
      </div>

      <div class="side">
        <div class="sgrid">
          {#each shots.slice(0, 6) as s, i}
            {@const src = getImgSrc(s)}
            <div
              class="th"
              class:sel={i === selectedShot}
              style={src ? `background-image:url("${src}")` : ''}
              data-f
              data-id="sh-grid-{i}"
              onclick={() => onSelectShot?.(i)}
              onkeydown={(e) => {
                if (e.key === 'Enter') onSelectShot?.(i);
                if (e.key.toLowerCase() === 'y') onDeleteShot?.(i);
              }}
              role="button"
              tabindex="0"
            >
              <small>{relTime(s.at)}</small>
            </div>
          {/each}
        </div>

        <div class="sub">
          {shots.length} {shots.length === 1 ? 'captura reciente' : 'capturas recientes'}
        </div>

        <div class="acts">
          <div
            class="btn sm"
            data-f
            data-id="sh-trash"
            onclick={() => onDeleteShot?.(selectedShot)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key.toLowerCase() === 'y') onDeleteShot?.(selectedShot); }}
            role="button"
            tabindex="0"
          >
            <svg class="i" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6"/></svg>
            Borrar <i class="k y">Y</i>
          </div>

          {#if hasUndo}
            <div
              class="btn sm"
              data-f
              data-id="sh-undo"
              onclick={onUndoDelete}
              onkeydown={(e) => { if (e.key === 'Enter') onUndoDelete?.(); }}
              role="button"
              tabindex="0"
            >
              <svg class="i" viewBox="0 0 24 24"><path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 0 12h-3"/></svg>
              Deshacer
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
{/if}
