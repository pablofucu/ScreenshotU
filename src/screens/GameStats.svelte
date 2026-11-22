<script lang="ts">
  // Estadísticas de juego: dos niveles. Categoría (Start en la lista, A en Semana) y Juego (A en la clasificación,
  // Terminados, Semana o Evolución).
  import { app } from '../lib/store.svelte';
  import CatLevel from './stats/CatLevel.svelte';
  import GameLevel from './stats/GameLevel.svelte';
</script>

{#if app.gsScope === 'cat'}
  <CatLevel />
{:else}
  <GameLevel />
{/if}
