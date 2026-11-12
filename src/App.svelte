<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { app } from './lib/store.svelte';
  import { learnKeydown } from './lib/learn.svelte';
  import { boot, onKeyDown, onScreenClick, onScreenDblClick, onScreenMove, startGamepad, startMouse, hideToast, cmd, intro } from './lib/controller.svelte';
  import { buildBackground } from './lib/bg';
  import { lerpH, rbGrad, variablesCSS } from './lib/atmos';
  import { IS_TABLET } from './lib/backend';
  import { lastSyncLabel, remoteStatus } from './lib/remote/status.svelte';
  const RBY = rbGrad('180deg');
  // Tramo de arcoíris de la sección actual (subrayado de pestañas); dentro de un juego, su degradado.
  const rbsec = $derived.by(() => {
    if (app.secK === null) return 'var(--grad)';
    const d = 1 / (app.lib.platforms.length + 2);
    const a = Math.max(0, Math.min(app.secK * d - d / 2, 1 - d));
    return rbGrad('90deg', a, a + d);
  });
  import Home from './screens/Home.svelte';
  import Titles from './screens/Titles.svelte';
  import Detail from './screens/Detail.svelte';
  import Shots from './screens/Shots.svelte';
  import Viewer from './screens/Viewer.svelte';
  import Stats from './screens/Stats.svelte';
  import Evo from './screens/Evo.svelte';
  import Streak from './screens/Streak.svelte';
  import GameStats from './screens/GameStats.svelte';
  import Settings from './screens/Settings.svelte';
  import ActionMenu from './screens/ActionMenu.svelte';
  import GameSettings from './screens/GameSettings.svelte';
  import GameForm from './screens/GameForm.svelte';
  import AddChooser from './screens/AddChooser.svelte';
  import GameDirWizard from './screens/GameDirWizard.svelte';
  import HiddenList from './screens/HiddenList.svelte';
  import NintendoImport from './screens/NintendoImport.svelte';
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
  import ReleasePicker from './screens/ReleasePicker.svelte';
  import AchievementsModal from './screens/AchievementsModal.svelte';
  import Search from './screens/Search.svelte';
  import Osk from './components/Osk.svelte';
  import PairRequest from './components/PairRequest.svelte';
  import UseOtherPc from './components/UseOtherPc.svelte';
  import AudioMenu from './components/AudioMenu.svelte';
  import ResumePill from './components/ResumePill.svelte';

  let w = $state(1280);
  let h = $state(720);
  // El escenario es siempre 1280x720 y se escala para caber (letterbox): igual en cualquier pantalla.
  // Escala en múltiplos de 1/16 de píxel de dispositivo: 1280·k/16 y 720·k/16 son enteros, así el
  // escenario cae exacto en píxeles físicos (evita bordes redondeados cortados y texto desplazado en 4K).
  const dpr = $derived.by(() => { void w; void h; return window.devicePixelRatio || 1; });
  const sc = $derived(Math.max(0.1, Math.floor(Math.min(w / 1280, h / 720) * dpr * 16)) / 16 / dpr);
  const ox = $derived(Math.round(((w - 1280 * sc) / 2) * dpr) / dpr);
  const oy = $derived(Math.round(((h - 720 * sc) / 2) * dpr) / dpr);
  const bg = buildBackground();
  let error = $state('');

  let curAtmos: { h1: number; h2: number } | null = null;
  let atmosRaf = 0;
  // Al cambiar de pantalla el color salta directo: degradarlo 700 ms repintaba todo el escenario
  // a la vez que la entrada de la pantalla nueva.
  let atmosKey = -1;

  // Las variables van en el propio atributo style del escenario: con setProperty se perdían cada vez que
  // Svelte reescribía el style (al cambiar el tamaño de la ventana).
  let atmosCss = $state('');
  function applyAtmos(vars: Record<string, string>) {
    atmosCss = Object.entries(vars).map(([k, v]) => `${k}:${v}`).join(';');
  }

  $effect(() => {
    const target = app.atmosTarget;
    if (!curAtmos) {
      curAtmos = { ...target };
      applyAtmos(variablesCSS(curAtmos.h1, curAtmos.h2));
      return;
    }
    cancelAnimationFrame(atmosRaf);
    const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const key = untrack(() => app.animKey);
    const jump = key !== atmosKey;
    atmosKey = key;
    // En la tablet el fundido de 700 ms repinta todo el cristal en cada fotograma: se salta.
    if (reduced || jump || IS_TABLET) {
      curAtmos = { ...target };
      applyAtmos(variablesCSS(curAtmos.h1, curAtmos.h2));
      return;
    }
    const from = { ...curAtmos };
    if (from.h1 === target.h1 && from.h2 === target.h2) return;
    const t0 = performance.now();
    const dur = 700;
    const step = (now: number) => {
      const p = Math.min(1, (now - t0) / dur);
      const e = 1 - Math.pow(1 - p, 3);
      const h1 = lerpH(from.h1, target.h1, e);
      const h2 = lerpH(from.h2, target.h2, e);
      curAtmos = { h1, h2 };
      applyAtmos(variablesCSS(h1, h2));
      if (p < 1) {
        atmosRaf = requestAnimationFrame(step);
      }
    };
    atmosRaf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(atmosRaf);
  });
  // Va después del efecto anterior: un cambio de pantalla sin cambio de color no cuenta para el siguiente.
  $effect(() => { atmosKey = app.animKey; });

  // Hints de los paneles (fuera de #screen y del teclado en pantalla, que ya atienden los suyos).
  function onStageClick(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t.closest('#screen, .oskscrim')) return;
    const act = t.closest<HTMLElement>('[data-hact]')?.dataset.hact;
    if (act) cmd(act);
  }

  onMount(() => {
    if (IS_TABLET) document.documentElement.classList.add('tablet');
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
    curAtmos = { ...app.atmosTarget };
    applyAtmos(variablesCSS(curAtmos.h1, curAtmos.h2));
    const tagNode = (el: HTMLElement) => {
      if (!el.hasAttribute('role')) el.setAttribute('role', 'button');
    };
    const tagTree = (root: ParentNode) => {
      root.querySelectorAll<HTMLElement>('[data-act],[data-set],[data-item],.scat').forEach(tagNode);
    };
    const mo = new MutationObserver((mutations) => {
      for (const m of mutations) {
        for (let i = 0; i < m.addedNodes.length; i++) {
          const n = m.addedNodes[i];
          if (n instanceof HTMLElement) {
            if (n.matches('[data-act],[data-set],[data-item],.scat')) tagNode(n);
            tagTree(n);
          }
        }
      }
    });
    if (stage) mo.observe(stage, { childList: true, subtree: true });
    if (stage) tagTree(stage);
    return () => { window.removeEventListener('mousemove', show); clearTimeout(t); mo.disconnect(); cancelAnimationFrame(atmosRaf); };
  });
</script>

<svelte:window bind:innerWidth={w} bind:innerHeight={h} onkeydown={onKeyDown} onkeyup={learnKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div id="stage" onclick={onStageClick} data-theme={app.config.theme === 'dark' ? 'dark' : 'light'} style="transform:translate({ox}px,{oy}px) scale({sc});{atmosCss};--rby:{RBY};--rbsec:{rbsec};--acc:var(--f1);--accv:var(--v1);--acc-ink:var(--on)">
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
        {:else if app.view === 'gstats'}<GameStats />
        {:else if app.view === 'streak'}<Streak />
        {:else}<Viewer />{/if}
      </div>
    {/key}
    {#if app.overlay === 'settings'}<Settings />{/if}
    {#if app.overlay === 'form'}<GameForm />{/if}
    {#if app.overlay === 'menu'}<ActionMenu />{/if}
    {#if app.overlay === 'gameset'}<GameSettings />{/if}
    {#if app.overlay === 'add'}<AddChooser />{/if}
    {#if app.overlay === 'gdir'}<GameDirWizard />{/if}
    {#if app.overlay === 'hidden'}<HiddenList />{/if}
    {#if app.overlay === 'ntimport'}<NintendoImport />{/if}
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
    {#if app.overlay === 'release'}<ReleasePicker />{/if}
    {#if app.overlay === 'achievements'}<AchievementsModal />{/if}
    {#if app.overlay === 'search'}<Search />{/if}
    {#if app.audioMenu}<AudioMenu />{/if}
    {#if app.osk.open}<Osk />{/if}
  {/if}
  {#if error}<div class="note pill" style="left:50%;top:330px">No se pudo iniciar: {error}</div>{/if}
  {#if IS_TABLET}<ResumePill />{/if}
  {#if !IS_TABLET}<PairRequest /><UseOtherPc />{/if}
  {#if IS_TABLET && !remoteStatus.online}
    <!-- Tablet: el PC no responde; se enseñan los últimos datos guardados y los cambios esperan para enviarse. -->
    <div class="offpill" role="status" style="position:absolute;left:50%;top:12px;transform:translateX(-50%);padding:6px 16px;border-radius:var(--r-pill);background:rgba(40,46,58,.78);-webkit-backdrop-filter:blur(12px);backdrop-filter:blur(12px);box-shadow:inset 0 1px 0 rgba(255,255,255,.25),0 10px 24px -10px rgba(0,0,0,.5);color:var(--on);font-size:var(--t-m);font-weight:700;z-index:49;pointer-events:none;white-space:nowrap">Sin conexión{lastSyncLabel() ? ` · datos de ${lastSyncLabel()}` : ''}{remoteStatus.pending ? ` · ${remoteStatus.pending} cambio${remoteStatus.pending === 1 ? '' : 's'} por enviar` : ''}</div>
  {/if}
  <div id="toast" class:on={app.toastOn} onclick={hideToast} role="presentation">{app.toastMsg}</div>
</div>
