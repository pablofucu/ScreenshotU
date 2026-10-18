<script lang="ts">
  let { list }: { list: Array<[string, string, string?]> } = $props();

  // Hints combinados («LB/RB»): el clic en la mitad izquierda del botón lanza la primera acción,
  // el resto (mitad derecha o texto) la segunda. App.svelte lee data-hact al subir el clic.
  function pick(e: MouseEvent, b: string, act?: string) {
    const el = e.currentTarget as HTMLElement;
    if (!act || !b.includes('/')) return;
    const [first, second] = b.split('/').map((s) => s.toLowerCase());
    const r = el.querySelector('.btn')?.getBoundingClientRect();
    el.dataset.hact = r && e.clientX < r.left + r.width / 2 ? first : second;
  }
</script>

<div class="hints">
  {#each list as [b, l, act]}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="hint pill" role="button" tabindex="-1" aria-label={l} data-hact={act ?? ''} onclick={(e) => pick(e, b, act)}>
      <span class="btn {b.length > 1 ? 'sh' : b.toLowerCase()}">{b}</span>{l}
    </div>
  {/each}
</div>
