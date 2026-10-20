<script lang="ts">
  // Barra superior: avatar con el acento, migas (con botón de volver) y reloj con el estado de la conexión.
  import { ICON } from '@shared/icons';
  import { MI } from '../lib/icons';
  import { app } from '../lib/store.svelte';

  let { title, sub = '' }: { title: string; sub?: string } = $props();
  const d = $derived(new Date(app.now));
  const hh = $derived(`${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`);
  const WD = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
  const MO = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const canBack = $derived(app.stack.length > 1);
  const state = $derived(app.syncing ? 'busy' : app.offline ? 'off' : '');
</script>

<header class="top">
  <button class="glass blur avatar" aria-label="Ajustes" onclick={() => app.setTab('settings')}><span class="face">{@html ICON.person}</span></button>
  <div class="crumb pill">
    {#if canBack}
      <button class="back" aria-label="Volver" onclick={() => app.back()}>{@html MI.back}</button>
    {:else}<span class="home"></span>{/if}
    <span class="cur">{title}</span>
    {#if sub}<span class="muted" style="font-size:13px;white-space:nowrap">{sub}</span>{/if}
  </div>
  <button class="clock" onclick={() => app.sync(true)} aria-label="Sincronizar">
    {hh}
    <small><i class="sync {state}"></i>{WD[d.getDay()]} {d.getDate()} {MO[d.getMonth()]}</small>
  </button>
</header>
