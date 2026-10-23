<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from './lib/store.svelte';
  import { learnKeydown } from './lib/learn.svelte';
  import { boot, onKeyDown, onScreenClick, onScreenDblClick, onScreenMove, startGamepad, startMouse, hideToast, cmd, intro } from './lib/controller.svelte';
  import { buildBackground } from './lib/bg';
  import Home from './screens/Home.svelte';
  import Titles from './screens/Titles.svelte';
  import Detail from './screens/Detail.svelte';
  import Shots from './screens/Shots.svelte';
  import Viewer from './screens/Viewer.svelte';
  import Stats from './screens/Stats.svelte';
  import Evo from './screens/Evo.svelte';
  import Streak from './screens/Streak.svelte';
  import Settings from './screens/Settings.svelte';
  import ActionMenu from './screens/ActionMenu.svelte';
  import GameForm from './screens/GameForm.svelte';
  import AddChooser from './screens/AddChooser.svelte';
  import GameDirWizard from './screens/GameDirWizard.svelte';
  import HiddenList from './screens/HiddenList.svelte';
  import MoveTo from './screens/MoveTo.svelte';
  import AccentPicker from './screens/AccentPicker.svelte';
  import CollForm from './screens/CollForm.svelte';
  import GamePicker from './screens/GamePicker.svelte';
  import AddColl from './screens/AddColl.svelte';
  import SgdbTitles from './screens/SgdbTitles.svelte';
  import SgdbPicker from './screens/SgdbPicker.svelte';
  import Phrases from './screens/Phrases.svelte';
  import Saves from './screens/Saves.svelte';
  import Library from './screens/Library.svelte';
  import StatusBar from './screens/StatusBar.svelte';
  import StalledCard from './screens/StalledCard.svelte';
  import Osk from './components/Osk.svelte';

  let w = $state(1280);
  let h = $state(720);
  // El escenario es siempre 1280x720 y se escala para caber (letterbox): igual en cualquier pantalla.
  // Escala en múltiplos de 1/16 de píxel de dispositivo: 1280·k/16 y 720·k/16 son enteros, así el
  // escenario cae exacto en píxeles físicos (evita bordes redondeados cortados y texto desplazado en 4K).
  const dpr = $derived.by(() => { void w; void h; return window.devicePixelRatio || 1; });
  const sc = $derived(Math.max(1, Math.floor(Math.min(w / 1280, h / 720) * dpr * 16)) / 16 / dpr);
  const ox = $derived(Math.round(((w - 1280 * sc) / 2) * dpr) / dpr);
  const oy = $derived(Math.round(((h - 720 * sc) / 2) * dpr) / dpr);
  const bg = buildBackground();
  let error = $state('');

  // Hints de los paneles (fuera de #screen y del teclado en pantalla, que ya atienden los suyos).
  function onStageClick(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t.closest('#screen, .oskscrim')) return;
    const act = t.closest<HTMLElement>('[data-hact]')?.dataset.hact;
    if (act) cmd(act);
  }

  onMount(() => {
    boot().catch((e) => { console.error(e); error = String(e); });
    startGamepad();
    startMouse();
    let t: ReturnType<typeof setTimeout>;
    const show = () => {
      document.body.classList.remove('nocursor');
      clearTimeout(t);
      t = setTimeout(() => document.body.classList.add('nocursor'), 4000);
    };
    window.addEventListener('mousemove', show);
    show();
    // Los elementos clicables son divs: se marcan como botones para los lectores de pantalla.
    const stage = document.getElementById('stage');
    let raf = 0;
    const tag = () => {
      raf = 0;
      stage?.querySelectorAll<HTMLElement>('[data-act],[data-set],[data-item],.scat').forEach((el) => {
        if (!el.hasAttribute('role')) el.setAttribute('role', 'button');
      });
    };
    const mo = new MutationObserver(() => { if (!raf) raf = requestAnimationFrame(tag); });
    if (stage) mo.observe(stage, { childList: true, subtree: true });
    tag();
    return () => { window.removeEventListener('mousemove', show); mo.disconnect(); cancelAnimationFrame(raf); };
  });
</script>

<svelte:window bind:innerWidth={w} bind:innerHeight={h} onkeydown={onKeyDown} onkeyup={learnKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div id="stage" onclick={onStageClick} style="transform:translate({ox}px,{oy}px) scale({sc});--acc:{app.chrome.c};--acc-ink:{app.chrome.dark ? '#3a2f00' : '#fff'}">
  <div class="bg">{@html bg}</div>
  {#if app.ready}
    {#key app.animKey}
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div id="screen" class="screen {app.anim}" use:intro onmousemove={onScreenMove} onclick={onScreenClick} ondblclick={onScreenDblClick}>
        {#if app.view === 'home'}<Home />
        {:else if app.view === 'titles'}<Titles />
        {:else if app.view === 'detail'}<Detail />
        {:else if app.view === 'shots'}<Shots />
        {:else if app.view === 'stats'}<Stats />
        {:else if app.view === 'evo'}<Evo />
        {:else if app.view === 'streak'}<Streak />
        {:else}<Viewer />{/if}
      </div>
    {/key}
    {#if app.overlay === 'settings'}<Settings />{/if}
    {#if app.overlay === 'form'}<GameForm />{/if}
    {#if app.overlay === 'menu'}<ActionMenu />{/if}
    {#if app.overlay === 'add'}<AddChooser />{/if}
    {#if app.overlay === 'gdir'}<GameDirWizard />{/if}
    {#if app.overlay === 'hidden'}<HiddenList />{/if}
    {#if app.overlay === 'moveto'}<MoveTo />{/if}
    {#if app.overlay === 'accent'}<AccentPicker />{/if}
    {#if app.overlay === 'collform'}<CollForm />{/if}
    {#if app.overlay === 'picker'}<GamePicker />{/if}
    {#if app.overlay === 'addcoll'}<AddColl />{/if}
    {#if app.overlay === 'sgdbtitles'}<SgdbTitles />{/if}
    {#if app.overlay === 'sgdb'}<SgdbPicker />{/if}
    {#if app.overlay === 'phrases'}<Phrases />{/if}
    {#if app.overlay === 'saves'}<Saves />{/if}
    {#if app.overlay === 'library'}<Library />{/if}
    {#if app.overlay === 'status'}<StatusBar />{/if}
    {#if app.overlay === 'stalled'}<StalledCard />{/if}
    {#if app.osk.open}<Osk />{/if}
  {/if}
  {#if error}<div class="note pill" style="left:50%;top:330px">No se pudo iniciar: {error}</div>{/if}
  <div id="toast" class:on={app.toastOn} onclick={hideToast} role="presentation">{app.toastMsg}</div>
</div>
