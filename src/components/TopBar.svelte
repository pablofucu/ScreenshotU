<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';

  let { crumbs, dots = 0, dot = 0 }: { crumbs: string[]; dots?: number; dot?: number } = $props();

  const z = (n: number) => String(n).padStart(2, '0');
  const DAYS = ['dom', 'lun', 'mar', 'mié', 'jue', 'vie', 'sáb'];
  const clock = $derived({
    d: `${z(app.now.getDate())}/${z(app.now.getMonth() + 1)}`,
    w: DAYS[app.now.getDay()],
    t: `${z(app.now.getHours())}:${z(app.now.getMinutes())}`,
  });
</script>

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
    <span>{clock.d} <small>({clock.w})</small></span><span>{clock.t}</span>
    <span class="batt" title={app.padConnected ? 'Mando conectado' : 'Sin mando'}>
      <svg width="30" height="20" viewBox="0 0 30 20"><rect x="1" y="3" width="28" height="14" rx="7" fill={app.padConnected ? 'var(--acc)' : '#a3abb8'} /><circle cx="8" cy="10" r="3" fill="#fff" /><circle cx="22" cy="10" r="2" fill="#fff" /></svg>
    </span>
  </div>
</div>
