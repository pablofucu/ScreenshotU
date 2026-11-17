<script lang="ts">
  import { IS_TABLET } from '../lib/backend';
  import { ICON } from '../lib/icons';

  let { list }: { list: Array<[string, string, string?]> } = $props();

  // Hints combinados («LB/RB»): el clic en la mitad izquierda del botón lanza la primera acción,
  // el resto (mitad derecha o texto) la segunda. App.svelte lee data-hact al subir el clic.
  function pick(e: MouseEvent, b: string, act?: string) {
    const el = e.currentTarget as HTMLElement;
    if (!act || !b.includes('/')) return;
    const [first, second] = b.split('/').map((s) => s.toLowerCase());
    const r = el.querySelector('.shp, .btn')?.getBoundingClientRect();
    el.dataset.hact = r && e.clientX < r.left + r.width / 2 ? first : second;
  }

  // Tablet: sin botones de mando. Fuera lo que ya hace el dedo (tocar = A, la flecha de arriba = Volver, las
  // flechas de dirección); los pares LB/RB y LT/RT son «‹ etiqueta ›» y el resto, botones con su nombre.
  const TAP = /^(Elegir|Seleccionar|Abrir|Ver juego|Ver racha|Elegir \/ escribir|Escribir|Ver la biblioteca)$/;
  let el = $state<HTMLElement>();
  const inScrim = $derived(!!el?.closest('.scrim, .ovl, .dlg'));
  // Lo que la pantalla ya ofrece en su sitio (flechas del periodo, pestañas, categoría) no se repite abajo.
  let inPlace = $state<Set<string>>(new Set());
  $effect(() => {
    void list;
    const s = new Set<string>();
    // Por acción («lb», «rt»…) y por nombre (la pestaña «Terminados» ya hace lo que «Y Terminados»). «b» no cuenta:
    // la flecha de volver de arriba no sustituye al «Cancelar» de un diálogo.
    for (const n of document.querySelectorAll<HTMLElement>('#screen [data-hact]')) {
      if (n.closest('.hints') || n.dataset.hact === 'b') continue;
      s.add(n.dataset.hact!);
      s.add('@' + (n.textContent ?? '').trim());
    }
    inPlace = s;
  });
  const shown = (b: string, l: string, act: string) =>
    !inPlace.has('@' + l) && (b.includes('/') ? b.toLowerCase().split('/') : [act]).some((a) => !inPlace.has(a));
  const touch = $derived(
    list.filter(([b, l, act]) => {
      if (!act || !shown(b, l, act)) return false;
      if (b === 'A' && TAP.test(l)) return false;
      if (b === 'B' && !inScrim && /^(Volver|Atrás)$/.test(l)) return false;
      return true;
    }),
  );
</script>

{#if IS_TABLET}
  <div class="hints touch" bind:this={el}>
    {#each touch as [b, l, act]}
      {#if b.includes('/')}
        {@const [p, n] = b.split('/').map((s) => s.toLowerCase())}
        <div class="hint pill pair">
          <button type="button" class="tb" data-hact={p} aria-label="{l} anterior">{@html ICON.chevL}</button>
          <span>{l}</span>
          <button type="button" class="tb" data-hact={n} aria-label="{l} siguiente">{@html ICON.chevR}</button>
        </div>
      {:else}
        <button type="button" class="hint pill tap" data-hact={act}>{l}</button>
      {/if}
    {/each}
  </div>
{:else}
  <div class="hints">
    {#each list as [b, l, act]}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="hint pill" role="button" tabindex="-1" aria-label={l} data-hact={act ?? ''} onclick={(e) => pick(e, b, act)}>
        {#if b.length > 1 && /^[LR][BT](\/[LR][BT])?$/.test(b)}<span class="shp">{#each b.split('/') as k}<span class="btn sh {k[1] === 'T' ? 'tr' : 'bu'}">{k}</span>{/each}</span>{:else}<span class="btn {b.length > 1 ? 'sh' : b.toLowerCase()}">{b}</span>{/if}{l}
      </div>
    {/each}
  </div>
{/if}
