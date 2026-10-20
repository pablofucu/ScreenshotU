<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON, audioKind, audioSvg } from '../lib/icons';
  import { refreshAudio, switchAudio } from '../lib/controller.svelte';

  let { crumbs, dots = 0, dot = 0 }: { crumbs: string[]; dots?: number; dot?: number } = $props();

  const curOut = $derived(app.audioOutputs.find((o) => o.default));

  function toggleMenu() {
    app.audioMenu = !app.audioMenu;
    if (app.audioMenu) void refreshAudio();
  }
  function pick(id: string) {
    app.audioMenu = false;
    void switchAudio(0, id);
  }
  function outside(e: MouseEvent) {
    if (app.audioMenu && !(e.target as Element | null)?.closest?.('.aud')) app.audioMenu = false;
  }

  const z = (n: number) => String(n).padStart(2, '0');
  const DAYS = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
  const clock = $derived({
    d: `${z(app.now.getDate())}/${z(app.now.getMonth() + 1)}`,
    w: DAYS[app.now.getDay()],
    t: `${z(app.now.getHours())}:${z(app.now.getMinutes())}`,
  });
</script>

<svelte:window onclick={outside} onkeydown={(e) => { if (e.key === 'Escape' && app.audioMenu) app.audioMenu = false; }} />

<div class="top">
  <div class="glass avatar" aria-hidden="true"><div class="face">{@html ICON.person}</div></div>
  <div class="crumb pill">
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
        <button type="button" class="audbtn" class:open={app.audioMenu} title={curOut ? `Salida de audio: ${curOut.short}` : 'Salida de audio'} aria-label="Cambiar la salida de audio" aria-expanded={app.audioMenu} onclick={toggleMenu}>
          {@html audioSvg(audioKind(curOut?.name ?? ''), 'currentColor', 4)}
        </button>
        {#if app.audioMenu}
          <div class="audmenu glass" role="menu">
            <div class="audhead">Salida de audio</div>
            {#each app.audioOutputs as o (o.id)}
              <button type="button" role="menuitemradio" aria-checked={o.default} class="audit" class:on={o.default} onclick={() => pick(o.id)}>
                <i>{@html audioSvg(audioKind(o.name), 'currentColor', 4)}</i>
                <span>{o.short}</span>
                {#if o.default}<b>✓</b>{/if}
              </button>
            {/each}
          </div>
        {/if}
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
  .audbtn :global(svg) { width: 22px; height: 22px; }
  .audbtn:hover, .audbtn.open { opacity: 1; color: var(--acc); background: rgba(255, 255, 255, 0.55); }
  .audmenu {
    position: absolute; top: 38px; right: -6px; z-index: 30; min-width: 260px; padding: 8px;
    display: flex; flex-direction: column; gap: 2px; font-size: 17px;
  }
  .audhead { padding: 4px 10px 6px; font-size: 13px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; color: #657083; }
  .audit {
    display: flex; align-items: center; gap: 10px; padding: 8px 10px; border: 0; border-radius: 10px; cursor: pointer;
    background: transparent; color: #3a4252; font: inherit; font-weight: 700; text-align: left;
  }
  .audit i { display: grid; place-items: center; width: 22px; height: 22px; color: #7a8496; }
  .audit i :global(svg) { width: 100%; height: 100%; }
  .audit span { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .audit b { color: var(--acc); }
  .audit:hover { background: rgba(255, 255, 255, 0.7); }
  .audit.on, .audit.on i { color: var(--acc); }
</style>
