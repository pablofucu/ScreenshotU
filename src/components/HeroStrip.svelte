<script lang="ts">
  // Fondo y logo de una fila de la clasificación (top 3): el hero se funde hacia el blanco y el logo va
  // encima. Sin hero, la captura más reciente o la portada; sin nada, el acento. Sin logo, el nombre.
  import { app } from '../lib/store.svelte';
  import { backend } from '../lib/backend';
  import type { Title } from '../lib/types';

  let { t }: { t: Title } = $props();

  const heroUrl = $derived(t.hero ? (app.artVer, backend.artUrl(t.hero)) : null);
  const logoUrl = $derived(t.logo ? (app.artVer, backend.artUrl(t.logo)) : null);
  let heroBad = $state('');
  let logoBad = $state('');
  const showHero = $derived(!!heroUrl && heroBad !== heroUrl);
  const showLogo = $derived(!!logoUrl && logoBad !== logoUrl);

  const fbPath = $derived(t.shots[0]?.path ?? t.cover ?? null);
  let fb = $state('');
  $effect(() => {
    const src = showHero ? null : fbPath;
    if (!src) { fb = ''; return; }
    const ctl = new AbortController();
    backend.thumb(src, ctl.signal).then((u) => { if (!ctl.signal.aborted) fb = u; }, () => {});
    return () => ctl.abort();
  });
</script>

<span class="hsbg">
  {#if showHero}
    <img alt="" src={heroUrl} draggable="false" onerror={() => (heroBad = heroUrl ?? '')} />
  {:else if fb}
    <img alt="" src={fb} draggable="false" />
  {:else}
    <i></i>
  {/if}
</span>
<span class="hslogo">
  {#if showLogo}
    <img alt={t.name} src={logoUrl} draggable="false" onerror={() => (logoBad = logoUrl ?? '')} />
  {:else}
    <b>{t.name}</b>
  {/if}
</span>
