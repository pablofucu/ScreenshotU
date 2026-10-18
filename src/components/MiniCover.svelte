<script lang="ts">
  // Mini portada vertical (2:3) de un juego para las listas de estadísticas. Si falta algo se usa lo
  // siguiente: portada → hero recortado → captura más reciente → acento con iniciales.
  import { app } from '../lib/store.svelte';
  import { backend } from '../lib/backend';
  import { initials } from '../lib/stats';
  import type { Title } from '../lib/types';

  let { t, acc, small = false }: { t: Title; acc: string; small?: boolean } = $props();

  const srcs = $derived([t.cover, t.hero, t.shots[0]?.path].filter((x): x is string => !!x));
  let src = $state('');
  $effect(() => {
    const list = srcs;
    void app.artVer;
    const ctl = new AbortController();
    src = '';
    (async () => {
      for (const p of list) {
        try {
          const u = await backend.thumb(p, ctl.signal);
          if (ctl.signal.aborted) return;
          if (await loads(u)) { src = u; return; }
        } catch { if (ctl.signal.aborted) return; }
      }
    })();
    return () => ctl.abort();
  });
  const loads = (u: string) => new Promise<boolean>((ok) => { const i = new Image(); i.onload = () => ok(true); i.onerror = () => ok(false); i.src = u; });
</script>

<span class="mcov" class:sm={small} style="--g:{acc}">
  {#if src}<img alt="" {src} draggable="false" />{:else}<b>{initials(t.name)}</b>{/if}
</span>
