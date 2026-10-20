<script lang="ts">
  // Estadísticas: clasificación por periodo, calendario de los últimos 6 meses y resumen anual.
  import { app } from '../lib/store.svelte';
  import Rank from './stats/Rank.svelte';
  import Cal from './stats/Cal.svelte';
  import Year from './stats/Year.svelte';
</script>

<div class="stabs">
  <div class="seg pill">
    <button class:on={app.statsTab === 'rank'} onclick={() => (app.statsTab = 'rank')}>Clasificación</button>
    <button class:on={app.statsTab === 'cal'} onclick={() => (app.statsTab = 'cal')}>Calendario</button>
    <button class:on={app.statsTab === 'year'} onclick={() => (app.statsTab = 'year')}>Resumen anual</button>
  </div>
</div>
{#key app.statsTab}
  <div class="tabin">
    {#if app.statsTab === 'rank'}<Rank />{:else if app.statsTab === 'cal'}<Cal />{:else}<Year />{/if}
  </div>
{/key}

<style>
  .stabs{padding:4px 14px 6px}
  .tabin{animation:tin .25s ease-out}
  @keyframes tin{from{opacity:0;transform:translateY(8px)}to{opacity:1;transform:none}}
</style>
