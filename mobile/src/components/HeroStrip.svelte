<script lang="ts">
  // Fondo y logo de una fila del podio: el hero se funde hacia el blanco y el logo va encima.
  import type { Title } from '@shared/types';
  import Img from './Img.svelte';
  import { logoSrc } from './RecentCard.svelte';

  let { t }: { t: Title } = $props();
  let logoBad = $state(false);
</script>

<span class="hs">
  <span class="bgi"><Img paths={[t.hero, t.shots[0]?.path, t.cover]} kind="thumb" /></span>
  <span class="lg">
    {#if t.logo && !logoBad}<img alt={t.name} use:logoSrc={t.logo} onerror={() => (logoBad = true)} />{:else}<b>{t.name}</b>{/if}
  </span>
</span>

<style>
  .hs{position:absolute;inset:0;display:block}
  .bgi{position:absolute;right:0;top:0;bottom:0;width:72%;-webkit-mask-image:linear-gradient(90deg,transparent,#000 45%);mask-image:linear-gradient(90deg,transparent,#000 45%)}
  .lg{position:absolute;right:10px;top:8px;bottom:8px;width:46%;display:flex;align-items:center;justify-content:center}
  .lg img{max-width:100%;max-height:100%;object-fit:contain;filter:drop-shadow(0 2px 5px rgba(0,0,0,.55))}
  .lg b{font-size:14px;font-weight:800;color:#fff;text-shadow:0 1px 4px rgba(0,0,0,.7);text-align:right;line-height:1.1}
</style>
