<script lang="ts">
  // Mini portada (2:3) para las listas de estadísticas.
  import { initials } from '@shared/stats';
  import type { Title } from '@shared/types';
  import Img from './Img.svelte';

  let { t, acc, size = 40 }: { t: Title; acc: string; size?: number } = $props();
</script>

<span class="mc" style="--g:{acc};width:{size}px;height:{Math.round(size * 1.5)}px">
  <Img paths={[t.cover, t.hero, t.shots[0]?.path]}>
    {#snippet fallback()}<b>{initials(t.name)}</b>{/snippet}
  </Img>
</span>

<style>
  .mc{position:relative;display:block;flex:none;border-radius:8px;overflow:hidden;background:linear-gradient(160deg,color-mix(in srgb,var(--g) 35%,white),var(--g));
    box-shadow:0 0 0 1px rgba(20,30,50,.18),0 3px 6px -3px rgba(0,0,0,.35)}
  b{position:absolute;inset:0;display:grid;place-items:center;font-size:13px;font-weight:800;color:#fff}
</style>
