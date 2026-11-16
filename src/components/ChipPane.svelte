<script lang="ts">
  // Zona desplazable de chips: el contenido sube y baja con `app.goff` (lo fija el controlador al mover el foco),
  // con barra lateral y el aviso «N chips más abajo».
  import type { Snippet } from 'svelte';
  import { onDestroy, onMount } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { resetCounts } from '../lib/countup';
  import CIcon from './CIcon.svelte';

  let { children }: { children: Snippet } = $props();
  let vp = $state<HTMLElement>(), pane = $state<HTMLElement>();
  let H = $state(0), full = $state(0), below = $state(0);

  const max = $derived(Math.max(0, full - H));
  const thumb = $derived(Math.max(40, (H ? H - 34 : 0) * (full ? H / full : 1)));
  const track = $derived(Math.max(0, H - 34));

  function measure() {
    if (!vp || !pane) return;
    H = vp.clientHeight; full = pane.scrollHeight;
  }
  $effect(() => {
    void app.goff; void full; void H;
    if (!pane) return;
    const edge = app.goff + H - 40;
    below = [...pane.querySelectorAll<HTMLElement>('.st')].filter((c) => c.offsetTop > edge).length;
  });
  onMount(() => {
    measure();
    const ro = new ResizeObserver(measure);
    if (pane) ro.observe(pane);
    if (vp) ro.observe(vp);
    return () => ro.disconnect();
  });
  onDestroy(() => { app.goff = 0; resetCounts(); });
</script>

<div class="cvp" bind:this={vp}>
  <div class="cpane" bind:this={pane} style="transform:translateY({-app.goff}px)">{@render children()}</div>
</div>
<div class="cbar" class:off={max === 0}><i style="height:{thumb}px;top:{max ? (track - thumb) * Math.min(1, app.goff / max) : 0}px"></i></div>
<div class="cmore" class:off={max === 0 || app.goff >= max - 4}><CIcon name="down" class="" /><span>{below ? `${below} ${below === 1 ? 'chip' : 'chips'} más abajo` : 'Más abajo'}</span></div>
