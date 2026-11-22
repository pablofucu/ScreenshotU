<script lang="ts">
  // Chip de Estadísticas: cristal con cara, icono + título y el contenido que se le pase.
  // Las animaciones corren solo con `.run`: al aparecer en pantalla y cada vez que recibe el foco.
  import type { Snippet } from 'svelte';
  import { onMount, tick } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { toneStyle, type Tone } from '../lib/chiptone';
  import CIcon from './CIcon.svelte';

  let { k, icon, label, wide = false, tone = 4, act, cls = '', children }: {
    k: string; icon?: string; label?: string; wide?: boolean; tone?: Tone; act?: string; cls?: string; children: Snippet;
  } = $props();

  let run = $state(false);
  const restart = async () => { run = false; await tick(); void document.body.offsetWidth; run = true; };
  onMount(() => { const t = setTimeout(() => (run = true), 60); return () => clearTimeout(t); });
  let wasFocus = false;
  $effect(() => { const f = app.focus === k; if (f && !wasFocus) restart(); wasFocus = f; });
</script>

<div class="glass st fx {cls}" class:wide class:f={app.focus === k} class:run data-k={k} data-act={act} style={toneStyle(tone)}>
  <div class="face">
    {#if label}<div class="x-k">{#if icon}<CIcon name={icon} />{/if}{label}</div>{/if}
    {@render children()}
  </div>
</div>
