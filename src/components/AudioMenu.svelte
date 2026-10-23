<script lang="ts">
  // Desplegable «Salida de audio» (botón de la barra superior o Start › Salida de audio). Va fuera de #screen para
  // verse en cualquier pantalla, también en el visor. Con el mando: arriba/abajo eligen, A cambia, B cierra.
  import { app } from '../lib/store.svelte';
  import { audioKind, audioSvg } from '../lib/icons';
  import { switchAudio } from '../lib/controller.svelte';

  function pick(id: string, on: boolean) {
    app.audioMenu = false;
    if (!on) void switchAudio(0, id);
  }
  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; app.audioRow = i; }
  }
  // Un clic fuera lo cierra (los de #screen ya los recoge el controlador sin activar lo de debajo).
  function outside(e: MouseEvent) {
    if (app.audioMenu && !(e.target as Element | null)?.closest?.('.aud')) app.audioMenu = false;
  }
</script>

<svelte:window onclick={outside} />

<div class="audmenu aud glass" role="menu" aria-label="Salida de audio">
  <div class="audhead">Salida de audio</div>
  {#each app.audioOutputs as o, i (o.id)}
    <button type="button" role="menuitemradio" aria-checked={o.default} class="audit" class:on={o.default} class:f={app.audioRow === i}
      onmousemove={(e) => hover(e, i)} onclick={() => pick(o.id, o.default)}>
      <i>{@html audioSvg(audioKind(o.name), 'currentColor', 4)}</i>
      <span>{o.short}</span>
      {#if o.default}<b>✓</b>{/if}
    </button>
  {/each}
</div>

<style>
  .audmenu {
    position: absolute; top: 66px; right: 24px; z-index: 40; min-width: 280px; max-width: 420px; padding: 8px; cursor: default;
    display: flex; flex-direction: column; gap: 2px; font-size: 17px; animation: audIn 0.16s ease-out both;
  }
  @keyframes audIn { from { opacity: 0; transform: translateY(-6px); } }
  .audhead { padding: 4px 10px 6px; font-size: 13px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; color: #657083; }
  .audit {
    display: flex; align-items: center; gap: 10px; padding: 8px 10px; border: 0; border-radius: 10px; cursor: pointer;
    background: transparent; color: #3a4252; font: inherit; font-weight: 700; text-align: left; transition: background 0.12s;
  }
  .audit i { display: grid; place-items: center; width: 22px; height: 22px; color: #7a8496; flex: none; }
  .audit i :global(svg) { width: 100%; height: 100%; }
  .audit span { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .audit b { color: var(--acc); }
  .audit.f { background: rgba(255, 255, 255, 0.85); box-shadow: inset 0 0 0 2px var(--acc); }
  .audit.on, .audit.on i { color: var(--acc); }
  @media (prefers-reduced-motion: reduce) { .audmenu { animation: none; } }
</style>
