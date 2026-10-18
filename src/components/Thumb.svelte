<script lang="ts">
  import { backend } from '../lib/backend';

  let { path }: { path: string } = $props();
  let src = $state('');

  $effect(() => {
    const ctl = new AbortController();
    const p = path;
    src = '';
    backend.thumb(p, ctl.signal).then(
      (u) => { if (!ctl.signal.aborted) src = u; },
      () => {},
    );
    return () => ctl.abort();
  });
</script>

{#if src}<img class="tin" alt="" {src} draggable="false" />{/if}
