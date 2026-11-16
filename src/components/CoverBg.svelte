<script lang="ts">
  // Portada como fondo de un bloque (estantería y carriles de Terminados). Prueba portada → hero → captura
  // y no pinta imagen hasta tener una; el bloque conserva su tamaño mientras tanto.
  import { untrack } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { backend, thumbNow } from '../lib/backend';
  import type { Title } from '../lib/types';

  let { t, cls = '', style = '', k, act, title }: { t: Title; cls?: string; style?: string; k?: string; act?: string; title?: string } = $props();

  const srcs = $derived([t.cover, t.hero, t.shots[0]?.path].filter((x): x is string => !!x));
  let src = $state(untrack(() => (srcs[0] ? (thumbNow(srcs[0]) ?? '') : '')));
  $effect(() => {
    const list = srcs;
    void app.artVer;
    const ctl = new AbortController();
    (async () => {
      for (const p of list) {
        try { const u = await backend.thumb(p, ctl.signal); if (ctl.signal.aborted) return; if (u) { src = u; return; } } catch { if (ctl.signal.aborted) return; }
      }
    })();
    return () => ctl.abort();
  });
</script>

<div class={cls} class:f={!!k && app.focus === k} data-k={k} data-act={act} {title} {style} style:background-image={src ? `url("${src}")` : undefined}></div>
