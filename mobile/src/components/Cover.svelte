<script lang="ts">
  // Portada vertical (2:3) de un juego en las cuadrículas. Sin portada: hero → captura → iniciales.
  import { initials } from '@shared/stats';
  import type { Title } from '@shared/types';
  import Img from './Img.svelte';

  let { t, acc }: { t: Title; acc: string } = $props();
</script>

<span class="face cv" style="--g:{acc}">
  <Img paths={[t.cover, t.hero, t.shots[0]?.path]}>
    {#snippet fallback()}<b class="ini">{initials(t.name)}</b>{/snippet}
  </Img>
</span>

<style>
  .cv{display:block;background:linear-gradient(160deg,color-mix(in srgb,var(--g) 35%,white),var(--g))}
  .ini{position:absolute;inset:0;display:grid;place-items:center;font-size:30px;font-weight:800;color:#fff;text-shadow:0 2px 4px rgba(0,0,0,.25)}
</style>
