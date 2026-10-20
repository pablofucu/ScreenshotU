<script lang="ts">
  // Favoritas: todas las capturas marcadas con la estrella, de todos los juegos.
  import { nCapturas } from '@shared/format';
  import { ICON } from '@shared/icons';
  import ShotGrid from '../components/ShotGrid.svelte';
  import { app } from '../lib/store.svelte';

  const shots = $derived(app.favShots);
</script>

<div class="fhead">
  <span class="star">{@html ICON.starY}</span>
  <div><h2>Favoritas</h2><span>{nCapturas(shots.length)} · se marcan en el visor del PC</span></div>
</div>
{#if shots.length}
  <ShotGrid {shots} days={app.prefs.shotsView === 'days'} showGame onopen={(i) => app.openViewer(shots, i, 'Favoritas')} />
{:else}
  <div class="glass panel empty"><span class="face">
    <b>Aún no hay favoritas</b>
    <p>Marca capturas con la estrella (botón X) en el visor del PC y aparecerán aquí.</p>
  </span></div>
{/if}

<style>
  .fhead{display:flex;align-items:center;gap:12px;padding:6px 16px 12px}
  .star{width:42px;height:42px;flex:none}
  .star :global(svg){width:42px;height:42px;display:block}
  h2{margin:0;font-size:21px;font-weight:800;color:var(--ink)}
  .fhead span{font-size:12.5px;font-weight:700;color:var(--dim)}
  .empty{margin:10px 14px}
  .empty .face{padding:24px;text-align:center}
  .empty b{font-size:18px;font-weight:800}
  .empty p{margin:8px 0 0;color:var(--dim);font-weight:600;line-height:1.4}
</style>
