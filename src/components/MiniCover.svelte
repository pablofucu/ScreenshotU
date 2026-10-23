<script lang="ts">
  // Mini portada vertical (2:3) de un juego para las listas de estadísticas. Si falta algo se usa lo
  // siguiente: portada → hero recortado → captura más reciente → acento con iniciales.
  import { app } from '../lib/store.svelte';
  import { untrack } from 'svelte';
  import { backend, thumbNow } from '../lib/backend';
  import { initials } from '../lib/stats';
  import type { Title } from '../lib/types';

  let { t, acc, small = false }: { t: Title; acc: string; small?: boolean } = $props();

  const srcs = $derived([t.cover, t.hero, t.shots[0]?.path].filter((x): x is string => !!x));
  // La primera ya resuelta se pinta al instante; las demás se prueban en orden sin vaciar la que se ve.
  let src = $state(untrack(() => (srcs[0] ? (thumbNow(srcs[0]) ?? '') : '')));
  $effect(() => {
    const list = srcs;
    void app.artVer;
    const ctl = new AbortController();
    (async () => {
      for (const p of list) {
        try {
          const u = await backend.thumb(p, ctl.signal);
          if (ctl.signal.aborted) return;
          if (await loads(u)) { src = u; return; }
        } catch { if (ctl.signal.aborted) return; }
      }
      if (!ctl.signal.aborted) src = '';
    })();
    return () => ctl.abort();
  });
  const loads = (u: string) => new Promise<boolean>((ok) => { const i = new Image(); i.onload = () => ok(true); i.onerror = () => ok(false); i.src = u; });
</script>

<span class="mcov" class:sm={small} style="--g:{acc}">
  {#if src}<img alt="" {src} draggable="false" />{:else}<b>{initials(t.name)}</b>{/if}
</span>
