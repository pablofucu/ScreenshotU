<script lang="ts">
  // Iconos de trazo para la pantalla de racha (rejilla 24, extremos redondos, sin emojis).
  export type RachaIcon = 'flame' | 'moon' | 'check' | 'clock' | 'warn' | 'lock' | 'back';

  let {
    name,
    size = 20,
    stroke = 'currentColor',
    fill = 'none',
    strokeWidth = 2.3,
    cls = '',
  }: {
    name: RachaIcon;
    size?: number;
    stroke?: string;
    fill?: string;
    strokeWidth?: number;
    cls?: string;
  } = $props();
</script>

<svg
  class="ico {cls}"
  width={size}
  height={name === 'flame' ? Math.round(size * 1.15) : size}
  viewBox="0 0 24 24"
  fill={fill}
  stroke={stroke}
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === 'flame'}
    <path d="M12 10.941c2.333 -3.308 .167 -7.823 -1 -8.941c0 3.395 -2.235 5.299 -3.667 6.706c-1.43 1.408 -2.333 3.621 -2.333 5.588c0 3.704 3.134 6.706 7 6.706s7 -3.002 7 -6.706c0 -1.712 -1.232 -4.403 -2.333 -5.588c-2.084 3.353 -3.257 3.353 -4.667 2.235" />
  {:else if name === 'moon'}
    <path d="M12 3c.132 0 .263 0 .393 0a7.5 7.5 0 0 0 7.92 12.446a9 9 0 1 1 -8.313 -12.454z" />
  {:else if name === 'check'}
    <path d="M5 12l5 5l10 -10" />
  {:else if name === 'clock'}
    <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 3" />
  {:else if name === 'warn'}
    <path d="M10.363 3.591l-8.106 13.534a1.914 1.914 0 0 0 1.636 2.871h16.214a1.914 1.914 0 0 0 1.636 -2.87l-8.106 -13.536a1.914 1.914 0 0 0 -3.274 0z" /><path d="M12 9v4M12 16h.01" />
  {:else if name === 'lock'}
    <rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11v-4a4 4 0 1 1 8 0v4" />
  {:else if name === 'back'}
    <path d="M9 14l-4 -4l4 -4" /><path d="M5 10h11a4 4 0 1 1 0 8h-1" />
  {/if}
</svg>

<style>
  .ico {
    display: block;
    flex: none;
  }
</style>
