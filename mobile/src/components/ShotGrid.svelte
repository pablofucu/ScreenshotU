<script lang="ts">
  // Cuadrícula de capturas (3 columnas), continua o agrupada por días con su cabecera. Se pinta por tandas
  // a medida que se baja, así que miles de capturas no cargan el móvil.
  import { dayKey, fmtDay } from '@shared/format';
  import { ICON } from '@shared/icons';
  import type { Shot } from '@shared/types';
  import { nearView } from '../lib/img';
  import { app } from '../lib/store.svelte';
  import Img from './Img.svelte';

  let { shots, days = true, showGame = false, onopen }: { shots: Shot[]; days?: boolean; showGame?: boolean; onopen: (i: number) => void } = $props();

  const STEP = 90;
  let limit = $state(STEP);
  $effect(() => { void shots; limit = STEP; });
  const shown = $derived(shots.slice(0, limit));
  const groups = $derived.by(() => {
    if (!days) return [{ k: 0, label: '', from: 0, items: shown.map((s, i) => ({ s, i })) }];
    const out: Array<{ k: number; label: string; from: number; items: Array<{ s: Shot; i: number }> }> = [];
    shown.forEach((s, i) => {
      const k = dayKey(s.date);
      if (out.at(-1)?.k !== k) out.push({ k, label: fmtDay(s.date), from: i, items: [] });
      out.at(-1)!.items.push({ s, i });
    });
    return out;
  });
  // Total del día aunque aún no se haya pintado entero.
  const dayCount = $derived.by(() => {
    const m = new Map<number, number>();
    for (const s of shots) { const k = dayKey(s.date); m.set(k, (m.get(k) ?? 0) + 1); }
    return m;
  });
</script>

{#each groups as g (g.k + ':' + g.from)}
  {#if days}<div class="dhead"><span class="pill">{g.label}<small>{dayCount.get(g.k)}</small></span></div>{/if}
  <div class="grid">
    {#each g.items as { s, i } (s.path)}
      <button class="glass tap shot" class:fav={app.favs.has(s.path)} onclick={() => onopen(i)}>
        <span class="face">
          <Img paths={[s.path]} />
          {#if app.favs.has(s.path)}<i class="star">{@html ICON.starY}</i>{/if}
          {#if showGame && s.game}<em>{s.game}</em>{/if}
        </span>
      </button>
    {/each}
  </div>
{/each}
{#if limit < shots.length}
  {#key limit}<div class="more" use:nearView={() => (limit += STEP)}>Cargando más capturas…</div>{/key}
{/if}

<style>
  .grid{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;padding:0 12px}
  .shot{--r:14px;--bw:4px;aspect-ratio:16/10;width:100%}
  .shot .face{border-radius:11px}
  .shot::before{display:none}
  .star{position:absolute;z-index:5;right:4px;top:4px;width:18px;height:18px}
  .star :global(svg){width:18px;height:18px;display:block}
  em{position:absolute;z-index:5;left:0;right:0;bottom:0;padding:8px 6px 3px;font-style:normal;font-size:10px;font-weight:800;color:#fff;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;
    background:linear-gradient(transparent,rgba(0,0,0,.6))}
  .dhead{position:sticky;top:calc(var(--top-h) - 2px);z-index:5;padding:10px 14px 8px}
  .dhead .pill{display:inline-flex;align-items:center;gap:8px;height:30px;padding:0 6px 0 14px;border-radius:999px;font-size:13.5px;font-weight:800;color:var(--ink2)}
  .dhead small{min-width:22px;height:22px;padding:0 6px;border-radius:11px;display:grid;place-items:center;font-size:11.5px;font-weight:800;color:var(--acc-ink);background:var(--acc)}
  .more{padding:20px;text-align:center;font-size:13px;font-weight:700;color:var(--dim)}
</style>
