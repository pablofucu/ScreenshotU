<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { buildBoard } from '../lib/milista.svelte';
  import { fmtDur } from '../lib/format';
  import Thumb from '../components/Thumb.svelte';

  const titles = $derived(app.platform?.titles ?? []);
  // Las franjas se recalculan al cambiar estados, fijados o la categoría (nunca mezclan categorías).
  const bands = $derived(buildBoard());

  const coverOf = (i: number) => titles[i]?.cover ?? titles[i]?.shots[0]?.path ?? null;

  const hours = (sec: number) => (sec < 3600 ? fmtDur(sec) : `${Math.round(sec / 3600)} h`);
  const ago = (d: number) =>
    d <= 0 ? 'hoy' : d === 1 ? 'ayer' : d < 30 ? `hace ${d} días` : d < 365 ? `hace ${Math.round(d / 30)} meses` : `hace ${Math.round(d / 365)} ${Math.round(d / 365) === 1 ? 'año' : 'años'}`;

  /** Mantiene a la vista la portada enfocada dentro de su franja (cada franja se desplaza sola). */
  $effect(() => {
    void app.focus;
    queueMicrotask(() => {
      const el = document.querySelector<HTMLElement>('#screen .it.f');
      const box = el?.closest<HTMLElement>('.its');
      if (!el || !box) return;
      const e = el.getBoundingClientRect(), b = box.getBoundingClientRect();
      const k = e.width / el.offsetWidth || 1;
      const pad = 24 * k;
      if (e.left < b.left + pad) box.scrollTo({ left: box.scrollLeft - (b.left + pad - e.left) / k, behavior: 'smooth' });
      else if (e.right > b.right - pad) box.scrollTo({ left: box.scrollLeft + (e.right - (b.right - pad)) / k, behavior: 'smooth' });
    });
  });
</script>

<div class="board">
  {#each bands as band (band.key)}
    <section class="band st-{band.key}" data-band={band.key} style="--c:{band.color}">
      <div class="vt"><span>{band.label}</span></div>
      <div class="its">
        {#each band.items as it (it.ti)}
          {@const t = titles[it.ti]}
          {#if band.key === 'done' && it.yearStart}
            <div class="ysep">{it.year || 'Sin fecha'}</div>
          {/if}
          <div
            class="it fx"
            class:f={app.focus === 'tit' + it.ti}
            class:compact={band.compact}
            data-k="tit{it.ti}"
            data-act="tit:{it.ti}"
          >
            <div class="cv" style={app.pipStyle(app.p, it.ti)}>
              <i class="pip"></i>
              <div class="fc">
                {#if coverOf(it.ti)}
                  <Thumb path={coverOf(it.ti)!} />
                {:else}
                  <div class="noart"><b>{t.name}</b></div>
                {/if}
              </div>
              {#if it.pin}
                <span class="bd pin" title="Fijado">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 3h6l-1 6 3 3v2h-4v6l-1 1-1-1v-6H7v-2l3-3z" fill="currentColor" /></svg>
                </span>
              {:else if band.key === 'done'}
                <span class="bd chk" title="Terminado">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" /></svg>
                </span>
              {/if}
            </div>
            <div class="tx">
              <b>{t.name}</b>
              {#if band.key === 'pend'}
                <small>sin empezar</small>
              {:else if band.key === 'done'}
                <small>{hours(t.playSeconds ?? 0)}</small>
              {:else if t.status === 'dropped'}
                <small>abandonado</small>
              {:else}
                <small>{hours(t.playSeconds ?? 0)} · {ago(it.days)}</small>
              {/if}
              {#if it.warn}
                <span class="wtxt">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2.4" /><path d="M12 7v5l3 2" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" /></svg>
                  Se aparca en {it.warn} d
                </span>
              {:else if it.pin}
                <span class="ptxt">Fijado</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
  {#if !bands.length}
    <div class="vacio">Aún no hay juegos en tu lista. Marca el estado de un juego con Y.</div>
  {/if}
</div>

<style>
  .board {
    position: absolute;
    left: 40px;
    right: 40px;
    top: 112px;
    bottom: 44px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    user-select: none;
  }
  .band {
    position: relative;
    flex: none;
    height: 108px;
    border-radius: var(--r-m);
    overflow: hidden;
    background: linear-gradient(90deg, color-mix(in srgb, var(--c) 16%, var(--glass-top)), var(--glass-mid) 60%);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .st-run { background: linear-gradient(90deg, color-mix(in srgb, var(--c) 30%, var(--glass-top)), var(--glass-mid) 65%); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 45%, transparent), 0 6px 18px -10px var(--c); }
  .st-park .its { filter: saturate(0.35); opacity: 0.86; }
  .st-done .its { filter: saturate(0.1); opacity: 0.66; }
  .st-done .its :global(.it.f), .st-park .its :global(.it.f) { filter: none; }
  .vt {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--c);
    color: white;
  }
  .vt span {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .its {
    position: absolute;
    left: 52px;
    right: 10px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }
  .it {
    display: flex;
    flex: none;
    gap: 9px;
    height: 100px;
    padding: 0 10px 0 0;
    border-radius: 14px;
    align-items: center;
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s;
  }
  .it.f {
    background: var(--glass-chip);
    box-shadow: var(--ring);
    z-index: 2;
  }
  .cv {
    position: relative;
    flex: none;
    width: 68px;
    height: 100px;
    padding: 4px;
    box-sizing: border-box;
    border-radius: 14px;
    transition: transform 0.15s;
  }
  .it.f .cv { transform: scale(1.03); }
  .fc {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 10px;
    overflow: hidden;
    background: var(--face);
    box-shadow: 0 0 0 1px var(--line);
  }
  .fc :global(.tin) { width: 100%; height: 100%; object-fit: cover; display: block; }
  .noart {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    box-sizing: border-box;
    text-align: center;
    font-size: 10px;
    color: var(--ink-2);
    overflow: hidden;
  }
  .bd {
    position: absolute;
    right: -2px;
    top: -2px;
    width: 18px;
    height: 18px;
    border-radius: var(--r-pill);
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    box-shadow: 0 0 0 2px var(--glass-chip);
  }
  .bd svg { width: 12px; height: 12px; }
  .bd.pin { background: var(--g2); }
  .bd.chk { background: var(--c); }
  .tx {
    width: 104px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    min-width: 0;
  }
  .it.compact .tx { width: 72px; }
  .tx b {
    font-size: 14px;
    font-weight: 800;
    line-height: 1.15;
    color: var(--ink);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .tx small { font-size: 12px; font-weight: 700; color: var(--ink-2); }
  .wtxt, .ptxt {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 11.5px;
    font-weight: 800;
    line-height: 1.1;
  }
  .wtxt { color: var(--warn); filter: brightness(0.8); }
  .wtxt svg { width: 12px; height: 12px; flex: none; }
  .ptxt { color: var(--g2); }
  .ysep {
    flex: none;
    align-self: stretch;
    display: flex;
    align-items: center;
    padding: 0 6px 0 10px;
    margin-left: 4px;
    border-left: 2px solid color-mix(in srgb, var(--c) 30%, transparent);
    font-size: 13px;
    font-weight: 800;
    color: var(--ink-2);
  }
  .vacio {
    margin: auto;
    font-size: var(--t-m);
    color: var(--ink-3);
  }
</style>
