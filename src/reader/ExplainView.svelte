<script lang="ts">
  // Explicacion de una frase (traduccion, literal, cadena de piezas y notas), igual en la pagina y en el glosario.
  import type { Explanation } from '../lib/novel-types';
  import { boldHtml } from './render';
  let { e }: { e: Explanation } = $props();
</script>

<div class="nat">{e.translation}</div>
{#if e.literal}<div class="lit">{e.literal}</div>{/if}
{#if e.chain.length}
  <div class="chain">{#each e.chain as c}<div class={c.kind}><b class="jp" lang="ja">{c.piece}</b>{c.meaning}</div>{/each}</div>
{/if}
{#if e.notes.length}<ul>{#each e.notes as n}<li>{@html boldHtml(n)}</li>{/each}</ul>{/if}
