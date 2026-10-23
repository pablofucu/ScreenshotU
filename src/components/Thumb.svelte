<script lang="ts">
  import { untrack } from 'svelte';
  import { backend, decodeImg, thumbNow, thumbReady } from '../lib/backend';

  let { path }: { path: string } = $props();

  // Lo ya cargado se pinta al instante (sin fundido). Si cambia la ruta, la imagen anterior se queda hasta que la
  // nueva está decodificada: nunca se ve el hueco vacío entre una y otra.
  const first = untrack(() => thumbNow(path)) ?? '';
  const now = !!first && thumbReady(first);
  let src = $state(now ? first : '');
  // Solo hay fundido de entrada cuando la miniatura llega más tarde que la portada (la primera vez que se ve).
  const fade = !now;

  $effect(() => {
    const p = path;
    const hit = thumbNow(p);
    if (hit && hit === untrack(() => src)) return;
    const ctl = new AbortController();
    (hit ? Promise.resolve(hit) : backend.thumb(p, ctl.signal)).then(
      async (u) => {
        await decodeImg(u);
        if (ctl.signal.aborted) return;
        src = u;
      },
      // Sin miniatura (error, no cancelación): fuera la anterior, que sería de otra imagen.
      () => { if (!ctl.signal.aborted) src = ''; },
    );
    return () => ctl.abort();
  });
</script>

{#if src}<img class="tin" class:fade alt="" {src} draggable="false" />{/if}
