<script lang="ts">
  // Dock inferior con las cinco secciones (los colores del arcoíris del visor).
  import { ICON, RAINBOW } from '@shared/icons';
  import { MI } from '../lib/icons';
  import { app, type Tab } from '../lib/store.svelte';

  const TABS: Array<{ t: Tab; n: string; ic: string; c: number }> = [
    { t: 'home', n: 'Inicio', ic: MI.home, c: 4 },
    { t: 'games', n: 'Juegos', ic: MI.grid, c: 3 },
    { t: 'stats', n: 'Estadísticas', ic: ICON.chart, c: 5 },
    { t: 'favs', n: 'Favoritas', ic: ICON.star, c: 2 },
    { t: 'settings', n: 'Ajustes', ic: ICON.gear, c: 6 },
  ];
</script>

<nav class="tabs pill">
  {#each TABS as x (x.t)}
    <button class:on={app.tab === x.t} style="--dc:{RAINBOW[x.c].c}" onclick={() => app.setTab(x.t)}><i>{@html x.ic}</i>{x.n}</button>
  {/each}
</nav>
