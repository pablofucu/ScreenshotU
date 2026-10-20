<script lang="ts">
  // Imagen del servidor con carga diferida (solo cuando se acerca a la pantalla) y cadena de respaldo:
  // si una ruta falla se prueba la siguiente; si fallan todas, queda el hueco (o `fallback`).
  import type { Snippet } from 'svelte';
  import { nearView, urlOf, type ImgKind } from '../lib/img';
  import { app } from '../lib/store.svelte';

  let { paths, kind = 'thumb', alt = '', fit = 'cover', eager = false, cls = '', fallback }: {
    paths: Array<string | null | undefined>; kind?: ImgKind; alt?: string; fit?: 'cover' | 'contain'; eager?: boolean; cls?: string; fallback?: Snippet;
  } = $props();

  const list = $derived(paths.filter((p): p is string => !!p));
  let seen = $state(false);
  $effect(() => { if (eager) seen = true; });
  let i = $state(0);
  let src = $state('');
  let loaded = $state(false);
  const key = $derived(list.join('|'));
  $effect(() => {
    void key;
    i = 0;
  });
  $effect(() => {
    const p = list[i];
    void app.source;
    if (!seen || !p) { src = ''; return; }
    const ctl = new AbortController();
    loaded = false;
    urlOf(p, kind, ctl.signal).then((u) => { if (!ctl.signal.aborted) src = u; }, () => { if (!ctl.signal.aborted) i++; });
    return () => ctl.abort();
  });
</script>

<span class="im {cls}" use:nearView={() => (seen = true)}>
  {#if src}
    <img {src} {alt} draggable="false" class:in={loaded} style="object-fit:{fit}" onload={() => (loaded = true)} onerror={() => { src = ''; i++; }} />
  {/if}
  {#if (!list.length || i >= list.length) && fallback}{@render fallback()}{/if}
</span>

<style>
  .im{position:absolute;inset:0;display:block;overflow:hidden}
  img{width:100%;height:100%;display:block;opacity:0;transition:opacity .25s}
  img.in{opacity:1}
</style>
