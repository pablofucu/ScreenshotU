<script lang="ts">
  import { z } from '../lib/format';
  import { app } from '../lib/store.svelte';
  import { ICON, audioKind, audioSvg } from '../lib/icons';
  import { closeAudioMenu, openAudioMenu } from '../lib/controller.svelte';

  // `pad`: el botón de audio entra en la navegación del mando (arriba desde el contenido de la pantalla).
  let { crumbs, dots = 0, dot = 0, pad = false }: { crumbs: string[]; dots?: number; dot?: number; pad?: boolean } = $props();

  const curOut = $derived(app.audioOutputs.find((o) => o.default));

  // El desplegable está en AudioMenu.svelte (fuera de #screen); aquí solo el botón. Con el mando se abre desde
  // Start › Salida de audio. Escape/B lo cierran por el controlador.
  function toggleMenu() {
    if (app.audioMenu) closeAudioMenu(); else void openAudioMenu();
  }

  const DAYS = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
  const clock = $derived({
    d: `${z(app.now.getDate())}/${z(app.now.getMonth() + 1)}`,
    w: DAYS[app.now.getDay()],
    t: `${z(app.now.getHours())}:${z(app.now.getMinutes())}`,
  });
</script>

<div class="top">
  <div class="glass avatar" aria-hidden="true"><div class="face">{@html ICON.person}</div></div>
  <div class="crumb pill" class:withp={dots > 1}>
    {#each crumbs as x, i}
      {#if i === crumbs.length - 1}<span class="cur">{x}</span>{:else}<span>{x}</span><span class="sep">›</span>{/if}
    {/each}
  </div>
  {#if dots > 1}
    <div class="pagep pill">Página {dot + 1} de {dots}<i><b style="left:{(dot / dots) * 100}%;width:{100 / dots}%"></b></i></div>
  {/if}
  <div class="clock">
    {#if app.audioOutputs.length}
      <div class="aud">
        <!-- Con `pad` el clic lo recoge el controlador (.fx › data-act); abierto, el clic lo cierra aquí. -->
        <button type="button" class="audbtn" class:fx={pad} class:f={pad && app.focus === 'audio'} data-k={pad ? 'audio' : undefined} data-act={pad ? 'audio' : undefined}
          class:open={app.audioMenu} title={curOut ? `Salida de audio: ${curOut.short}` : 'Salida de audio'} aria-label="Cambiar la salida de audio" aria-expanded={app.audioMenu}
          onclick={() => { if (!pad || app.audioMenu) toggleMenu(); }}>
          {@html audioSvg(audioKind(curOut?.name ?? ''), 'currentColor', 4)}
        </button>
      </div>
    {/if}
    <span>{clock.d} <small>({clock.w})</small></span><span>{clock.t}</span>
    <span class="batt" title={app.padConnected ? 'Mando conectado' : 'Sin mando'}>
      <svg width="30" height="20" viewBox="0 0 30 20"><rect x="1" y="3" width="28" height="14" rx="7" fill={app.padConnected ? 'var(--acc)' : '#a3abb8'} /><circle cx="8" cy="10" r="3" fill="#fff" /><circle cx="22" cy="10" r="2" fill="#fff" /></svg>
    </span>
  </div>
</div>

<style>
  .aud { position: relative; display: flex; }
  .audbtn {
    width: 34px; height: 30px; padding: 0; display: grid; place-items: center; cursor: pointer;
    background: transparent; border: 0; border-radius: 10px; color: #7a8496; opacity: 0.75;
    transition: opacity 0.15s, color 0.15s, background 0.15s;
  }
  .audbtn.f { box-shadow: inset 0 0 0 2px var(--acc); }
  .audbtn :global(svg) { width: 22px; height: 22px; }
  .audbtn:hover, .audbtn.open, .audbtn.f { opacity: 1; color: var(--acc); background: rgba(255, 255, 255, 0.55); }
</style>
