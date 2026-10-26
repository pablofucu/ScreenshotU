<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { App } from '@capacitor/app';
  import { app } from './lib/store.svelte';
  import { buildBackground } from './lib/bg';
  import TopBar from './components/TopBar.svelte';
  import TabBar from './components/TabBar.svelte';
  import Home from './screens/Home.svelte';
  import Games from './screens/Games.svelte';
  import Stats from './screens/Stats.svelte';
  import Favs from './screens/Favs.svelte';
  import Detail from './screens/Detail.svelte';
  import Shots from './screens/Shots.svelte';
  import Viewer from './screens/Viewer.svelte';

  const bg = buildBackground();

  const header = $derived.by(() => {
    if (!app.server) return { title: 'Conectar', sub: '' };
    const r = app.route;
    switch (r.v) {
      case 'home':
        return { title: 'Inicio', sub: '' };
      case 'games':
        return { title: 'Juegos', sub: app.lib.platforms[app.gamesPlat]?.name ?? '' };
      case 'stats':
        return { title: 'Estadísticas', sub: '' };
      case 'favs':
        return { title: 'Favoritas', sub: '' };
      case 'settings':
        return { title: 'Ajustes', sub: '' };
      case 'detail': {
        const t = app.index.get(r.id)?.t;
        return { title: t?.name ?? 'Detalle', sub: '' };
      }
      case 'shots': {
        const t = app.index.get(r.id)?.t;
        return { title: 'Capturas', sub: t?.name ?? '' };
      }
      case 'evo': {
        const t = app.index.get(r.id)?.t;
        return { title: 'Evolución', sub: t?.name ?? '' };
      }
      case 'viewer':
        return { title: app.viewer?.title ?? 'Visor', sub: '' };
      default:
        return { title: 'Visor', sub: '' };
    }
  });

  onMount(() => {
    if (app.server) {
      void app.start();
    }
    const timer = setInterval(() => {
      app.now = Date.now();
    }, 10000);
    const backListener = App.addListener('backButton', () => {
      if (!app.back()) {
        void App.exitApp();
      }
    });
    return () => {
      clearInterval(timer);
      void backListener.then((h) => h.remove());
    };
  });

  $effect(() => {
    void app.navKey;
    tick().then(() => {
      const el = document.getElementById('scroller');
      if (el) el.scrollTop = app.savedScroll();
    });
  });

  $effect(() => {
    const root = document.getElementById('app');
    if (root) {
      root.style.setProperty('--acc', app.chrome.c);
      root.style.setProperty('--acc-ink', app.chrome.dark ? '#3a2f00' : '#fff');
    }
  });
</script>

<div class="bg">{@html bg}</div>

{#if app.route.v !== 'viewer'}
  <TopBar title={header.title} sub={header.sub} />
{/if}

<div id="scroller" style={app.route.v === 'viewer' ? 'padding:0;overflow:hidden;z-index:30;' : undefined}>
  {#if !app.server}
    <div class="screen pad" style="display:flex;align-items:center;justify-content:center;min-height:50vh;">
      <div class="glass panel" style="width:100%;max-width:320px;text-align:center;">
        <span class="face" style="padding:28px 20px;display:flex;flex-direction:column;align-items:center;gap:10px;">
          <h2 style="margin:0;font-size:22px;font-weight:800;">Conectar</h2>
          <p class="muted" style="margin:0;font-size:14px;">Conéctate a tu visor de escritorio para ver tus juegos y capturas.</p>
        </span>
      </div>
    </div>
  {:else}
    {#key app.navKey}
      <div class="screen {app.dir}" style={app.route.v === 'viewer' ? 'animation:none;height:100%;' : undefined}>
        {#if app.route.v === 'home'}
          <Home />
        {:else if app.route.v === 'games'}
          <Games />
        {:else if app.route.v === 'stats'}
          <Stats />
        {:else if app.route.v === 'favs'}
          <Favs />
        {:else if app.route.v === 'settings'}
          <div class="pad" style="padding-top:14px;">
            <div class="glass panel">
              <span class="face" style="padding:24px 20px;text-align:center;">
                <h2 style="margin:0 0 8px;font-size:20px;font-weight:800;">Ajustes</h2>
                <p class="muted" style="margin:0;font-size:14px;">Próximamente disponible en el móvil.</p>
              </span>
            </div>
          </div>
        {:else if app.route.v === 'detail'}
          <Detail id={app.route.id} />
        {:else if app.route.v === 'shots'}
          <Shots id={app.route.id} />
        {:else if app.route.v === 'evo'}
          <div class="pad" style="padding-top:14px;">
            <div class="glass panel">
              <span class="face" style="padding:24px 20px;text-align:center;">
                <h2 style="margin:0 0 8px;font-size:20px;font-weight:800;">Evolución</h2>
                <p class="muted" style="margin:0;font-size:14px;">Gráfica de evolución próximamente.</p>
              </span>
            </div>
          </div>
        {:else if app.route.v === 'viewer'}
          <Viewer />
        {/if}
      </div>
    {/key}
  {/if}
</div>

{#if app.route.v !== 'viewer'}
  <TabBar />
{/if}

<div id="toast" class:on={!!app.toastMsg}>{app.toastMsg}</div>
