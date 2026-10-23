<script lang="ts">
  // Tarjeta de «Seguir jugando»: el hero del juego con su logo. Sin hero se usa la primera captura
  // (o la portada, ampliada y desenfocada); sin logo, el nombre escrito.
  import { app } from '../lib/store.svelte';
  import { untrack } from 'svelte';
  import { backend, decodeImg, thumbNow } from '../lib/backend';
  import { ICON } from '../lib/icons';
  import type { Title } from '../lib/types';

  let { t, playing = false }: { t: Title; playing?: boolean } = $props();

  const heroUrl = $derived(t.hero ? (app.artVer, backend.artUrl(t.hero)) : null);
  const logoUrl = $derived(t.logo ? (app.artVer, backend.artUrl(t.logo)) : null);
  let heroBad = $state('');
  let logoBad = $state('');
  const showHero = $derived(!!heroUrl && heroBad !== heroUrl);
  const showLogo = $derived(!!logoUrl && logoBad !== logoUrl);

  // Sin hero: la captura más reciente (nítida) o, si no hay, la portada desenfocada.
  const fbPath = $derived(t.shots[0]?.path ?? t.cover ?? null);
  const fbBlur = $derived(!t.shots[0]);
  // Lo ya cargado se pinta al instante; lo nuevo, cuando ya está decodificado (sin hueco entre medias).
  let fb = $state(untrack(() => (showHero || !fbPath ? '' : (thumbNow(fbPath) ?? ''))));
  $effect(() => {
    const src = showHero ? null : fbPath;
    if (!src) { fb = ''; return; }
    const ctl = new AbortController();
    backend.thumb(src, ctl.signal).then(async (u) => { await decodeImg(u); if (!ctl.signal.aborted) fb = u; }, () => {});
    return () => ctl.abort();
  });
</script>

<div class="face">
  {#if showHero}
    <img class="rhero" alt="" src={heroUrl} draggable="false" onerror={() => (heroBad = heroUrl ?? '')} />
  {:else if fb}
    <div class="rfb" class:blur={fbBlur} style="background-image:url({fb})"></div>
  {:else}
    <div class="rfb plain"></div>
  {/if}
  <div class="rshade"></div>
  {#if showLogo}
    <img class="rlogo" alt={t.name} src={logoUrl} draggable="false" onerror={() => (logoBad = logoUrl ?? '')} />
  {:else}
    <div class="rname">{t.name}</div>
  {/if}
  {#if playing}<span class="rplaying"><i>{@html ICON.play}</i>Jugando</span>{/if}
</div>
