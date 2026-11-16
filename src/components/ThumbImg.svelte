<script lang="ts">
  // Miniatura de una captura (o portada) por ruta; no pinta nada hasta tenerla.
  import { app } from '../lib/store.svelte';
  import { backend, thumbNow } from '../lib/backend';
  import { untrack } from 'svelte';

  let { path, style = '' }: { path: string; style?: string } = $props();
  let src = $state(untrack(() => thumbNow(path) ?? ''));
  $effect(() => {
    const p = path;
    void app.artVer;
    const ctl = new AbortController();
    backend.thumb(p, ctl.signal).then((u) => { if (!ctl.signal.aborted) src = u; }).catch(() => {});
    return () => ctl.abort();
  });
</script>

{#if src}<img alt="" {src} {style} draggable="false" />{/if}
