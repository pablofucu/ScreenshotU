<script lang="ts">
  import { z } from '../lib/format';
  import { app } from '../lib/store.svelte';
  import { ICON, audioKind, audioSvg } from '../lib/icons';
  import { closeAudioMenu, openAudioMenu, toast } from '../lib/controller.svelte';
  import { backend, IS_TABLET } from '../lib/backend';

  // `pad`: el botón de audio entra en la navegación del mando (arriba desde el contenido de la pantalla).
  let { crumbs, dots = 0, dot = 0, pad = false }: { crumbs: string[]; dots?: number; dot?: number; pad?: boolean } = $props();

  const curOut = $derived(app.audioOutputs.find((o) => o.default));

  // El desplegable está en AudioMenu.svelte (fuera de #screen); aquí solo el botón. Con el mando se abre desde
  // Start › Salida de audio. Escape/B lo cierran por el controlador.
  function toggleMenu() {
    if (app.audioMenu) closeAudioMenu(); else void openAudioMenu();
  }

  // Apagar el PC (dos pulsaciones para evitar accidentes) y, en la tablet, despertarlo con Wake-on-LAN.
  const MAC_KEY = 'visor-pc-mac';
  let armed = $state(false);
  let armT: ReturnType<typeof setTimeout> | undefined;
  const readMac = () => { try { return localStorage.getItem(MAC_KEY) ?? ''; } catch { return ''; } };
  if (IS_TABLET) {
    // Con el PC encendido se guarda su MAC para poder despertarlo luego, cuando esté apagado.
    void backend.pcMac().then((m) => { if (m) try { localStorage.setItem(MAC_KEY, m); } catch { /* sin almacenamiento */ } }).catch(() => {});
  }
  async function powerOff() {
    if (!armed) {
      armed = true;
      clearTimeout(armT);
      armT = setTimeout(() => (armed = false), 4000);
      toast('Pulsa otra vez para apagar el PC', 3000);
      return;
    }
    clearTimeout(armT);
    armed = false;
    try { await backend.shutdownPc(); toast('Apagando el PC…', 3000); } catch (e) { toast(String(e), 3000); }
  }
  async function wakePc() {
    const mac = readMac();
    const w = (window as unknown as { __VISOR_WAKE__?: (m: string) => Promise<void> }).__VISOR_WAKE__;
    if (!mac) { toast('Aún no se conoce la MAC del PC: conéctate una vez con el PC encendido', 3500); return; }
    try { await w?.(mac); toast('Despertando el PC…', 3000); } catch (e) { toast(String(e), 3000); }
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
    {#if IS_TABLET}
      <button type="button" class="audbtn" title="Despertar el PC (Wake-on-LAN)" aria-label="Despertar el PC" onclick={wakePc}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M12 3v7"/><path d="M5.6 7.2a8 8 0 0 0 0 11.3 8 8 0 0 0 12.8 0"/><path d="M18.4 18.5a8 8 0 0 0-3-13.3" stroke-dasharray="1 3.2"/></svg>
      </button>
    {/if}
    <button type="button" class="audbtn" class:armed={armed} title={armed ? 'Pulsa otra vez para confirmar' : 'Apagar el PC'} aria-label="Apagar el PC" onclick={powerOff}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 3v9"/><path d="M6.4 6.9a8 8 0 1 0 11.2 0"/></svg>
    </button>
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
      <svg width="30" height="20" viewBox="0 0 30 20"><rect x="1" y="3" width="28" height="14" rx="7" fill={app.padConnected ? 'var(--acc)' : 'var(--ink-3)'} /><circle cx="8" cy="10" r="3" fill="var(--on)" /><circle cx="22" cy="10" r="2" fill="var(--on)" /></svg>
    </span>
  </div>
</div>

<style>
  .aud { position: relative; display: flex; }
  .audbtn {
    width: 34px; height: 30px; padding: 0; display: grid; place-items: center; cursor: pointer;
    background: transparent; border: 0; border-radius: 10px; color: var(--ink-3); opacity: 0.75;
    transition: opacity 0.15s, color 0.15s, background 0.15s;
  }
  .audbtn.f { box-shadow: inset 0 0 0 2px var(--acc); }
  .audbtn :global(svg) { width: 22px; height: 22px; }
  .audbtn.armed { opacity: 1; color: #fff; background: #d64545; }
  .audbtn:hover, .audbtn.open, .audbtn.f { opacity: 1; color: var(--acc); background: var(--glass-in); }
</style>
