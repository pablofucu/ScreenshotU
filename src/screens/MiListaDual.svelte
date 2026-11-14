<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { buildBoard } from '../lib/milista.svelte';
  import { setTitlesTab } from '../lib/controller.svelte';
  import { fmtDur } from '../lib/format';
  import Thumb from '../components/Thumb.svelte';

  const titles = $derived(app.platform?.titles ?? []);
  // Las franjas se recalculan al cambiar estados, fijados o la categoría (nunca mezclan categorías).
  const bands = $derived(buildBoard());

  // Solo hay carriles para las franjas con juegos (buildBoard ya descarta las vacías): el espacio se reparte entre ellas.
  const lanes = $derived(bands);

  // Alto del tablero: de debajo de las pestañas (y 182) al final del contenido (644).
  const H = 462, GAP = 6, ROW = 100;
  // Alto de cada fila: 100 px, o menos si hay tantas franjas que no caben.
  const rowH = $derived(lanes.length ? Math.min(ROW, Math.floor((H - GAP * (lanes.length - 1)) / lanes.length)) : ROW);

  // Filas por carril: 1 por carril y el resto al que más lo necesita.
  const rowsOf = $derived.by(() => {
    const need = lanes.map((l: any) => {
      const per = l.key === 'done' ? 6 : 5;
      const n = l.items.length + (l.key === 'done' ? l.items.filter((i: any) => i.yearStart).length : 0);
      return { n, per, want: Math.max(1, Math.ceil(n / per)) };
    });
    const rows: number[] = need.map(() => 1);
    let spare = Math.floor((H - GAP * (lanes.length - 1)) / rowH) - rows.reduce((a, b) => a + b, 0);
    while (spare > 0) {
      let best = -1, bestGap = 0;
      need.forEach((x, i) => {
        const g = x.want - rows[i];
        if (g > bestGap) { best = i; bestGap = g; }
      });
      if (best < 0) break;
      rows[best]++;
      spare--;
    }
    return rows;
  });

  const coverOf = (i: number) => titles[i]?.cover ?? titles[i]?.shots[0]?.path ?? null;

  const hours = (sec: number) => (sec < 3600 ? fmtDur(sec) : `${Math.round(sec / 3600)} h`);
  const ago = (d: number) =>
    d <= 0 ? 'hoy' : d === 1 ? 'ayer' : d < 30 ? `hace ${d} días`
      : d < 45 ? 'hace 1 mes'
      : d < 345 ? `hace ${Math.min(11, Math.round(d / 30))} meses`
      : `hace ${Math.max(1, Math.round(d / 365))} ${Math.max(1, Math.round(d / 365)) === 1 ? 'año' : 'años'}`;

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
      if (e.top < b.top) box.scrollTo({ top: box.scrollTop - (b.top - e.top) / k, behavior: 'smooth' });
      else if (e.bottom > b.bottom) box.scrollTo({ top: box.scrollTop + (e.bottom - b.bottom) / k, behavior: 'smooth' });
    });
  });
</script>

<div class="board" style="--rh:{rowH}px">
  {#each lanes as band, li (band.key)}
    {@const rows = rowsOf[li]}
    <section class="band st-{band.key}" data-band={band.key} style="--c:{band.color}; height:{rows * rowH}px">
      <div class="vt"><span>{band.label}</span></div>
      <div class="its" onwheel={(e) => {
        if (!e.deltaY) return;
        if (rows > 1) e.currentTarget.scrollTop += e.deltaY; else e.currentTarget.scrollLeft += e.deltaY;
        e.preventDefault();
      }}>
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
                  Aparca en {it.warn} d
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
    <button
      type="button"
      class="vacio fx"
      class:f={app.focus === 'list-empty'}
      data-k="list-empty"
      data-act="list-catalog"
      tabindex="-1"
      onclick={() => setTitlesTab('all')}
    >
      <p>Aún no hay juegos en tu lista.</p>
      <span class="btn-cat">
        <span class="btn a">A</span> Ir al Catálogo para añadir juegos
      </span>
    </button>
  {/if}
</div>

<style>
  .board {
    position: absolute;
    left: 40px;
    right: 40px;
    top: 182px;
    bottom: calc(720px - var(--content-end));
    display: flex;
    flex-direction: column;
    gap: 6px;
    user-select: none;
  }
  .band {
    position: relative;
    flex: none;
    border-radius: var(--r-m);
    overflow: hidden;
    background: linear-gradient(180deg,
      color-mix(in srgb, var(--c) 15%, var(--glass-top)) 0%,
      color-mix(in srgb, var(--c) 15%, var(--glass-mid)) 46%,
      color-mix(in srgb, var(--c) 15%, var(--glass-bot)) 100%
    );
    box-shadow:
      inset 0 1.5px 0 var(--hl),
      inset 0 0 0 1px var(--hl-ring),
      inset 0 -10px 14px -8px var(--hl-bot),
      inset 0 -1.5px 0 rgba(110,125,150,.30),
      0 0 0 1px rgba(95,110,135,.24),
      0 2px 3px rgba(40,50,70,.10),
      0 18px 30px -16px rgba(40,50,70,.42);
  }
  /* La franja de «Jugando» lleva algo más de tinte y un brillo de su color. */
  .st-run {
    background: linear-gradient(180deg,
      color-mix(in srgb, var(--c) 28%, var(--glass-top)) 0%,
      color-mix(in srgb, var(--c) 22%, var(--glass-mid)) 46%,
      color-mix(in srgb, var(--c) 18%, var(--glass-bot)) 100%
    );
    box-shadow:
      inset 0 1.5px 0 var(--hl),
      inset 0 0 0 1px color-mix(in srgb, var(--c) 45%, transparent),
      inset 0 -10px 14px -8px var(--hl-bot),
      0 0 0 1px rgba(95,110,135,.24),
      0 18px 30px -16px rgba(40,50,70,.42),
      0 6px 18px -10px var(--c);
  }
  .st-park .it:not(.f) { filter: saturate(0.35); opacity: 0.86; }
  .st-done .it:not(.f) { filter: saturate(0.1); opacity: 0.66; }
  .st-park .it.f, .st-done .it.f { filter: none; opacity: 1; }
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
    font-size: var(--t-s);
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
    flex-wrap: wrap;
    align-content: flex-start;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    box-sizing: border-box;
    overflow: hidden;
  }
  .it {
    display: flex;
    flex: none;
    gap: 9px;
    height: calc(var(--rh, 100px) - 8px);
    padding: 0 10px 0 0;
    border-radius: var(--r-s);
    align-items: center;
    cursor: pointer;
    background: linear-gradient(180deg, var(--glass-top) 0%, var(--glass-mid) 46%, var(--glass-bot) 100%);
    box-shadow:
      inset 0 1.5px 0 var(--hl),
      inset 0 0 0 1px var(--hl-ring),
      inset 0 -10px 14px -8px var(--hl-bot),
      inset 0 -1.5px 0 rgba(110,125,150,.30),
      0 0 0 1px rgba(95,110,135,.24),
      0 2px 3px rgba(40,50,70,.10),
      0 18px 30px -16px rgba(40,50,70,.42);
    transition: background 0.15s, box-shadow 0.15s, filter 0.15s, opacity 0.15s;
  }
  .it.f {
    background: var(--glass-chip);
    box-shadow: var(--ring);
    z-index: 2;
  }
  .cv {
    position: relative;
    flex: none;
    width: 62px;
    height: calc(var(--rh, 100px) - 8px);
    padding: 4px;
    box-sizing: border-box;
    border-radius: var(--r-s);
    transition: transform 0.15s;
  }
  .it.f .cv { transform: scale(1.03); }
  .fc {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: var(--r-s);
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
    font-size: var(--t-xs);
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
  .it.compact .tx { width: 84px; }
  .tx b {
    font-size: var(--t-s);
    font-weight: 800;
    line-height: 1.15;
    color: var(--ink);
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .tx small { font-size: var(--t-s); font-weight: 700; color: var(--ink-2); }
  .wtxt, .ptxt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px;
    font-size: var(--t-xs);
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
    font-size: var(--t-s);
    font-weight: 800;
    color: var(--ink-2);
  }
  .vacio {
    border: none;
    font: inherit;
    margin: auto;
    font-size: var(--t-m);
    color: var(--ink-2);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 28px 36px;
    border-radius: var(--r-m);
    background: linear-gradient(180deg, var(--glass-top) 0%, var(--glass-mid) 46%, var(--glass-bot) 100%);
    box-shadow:
      inset 0 1.5px 0 var(--hl),
      inset 0 0 0 1px var(--hl-ring),
      inset 0 -10px 14px -8px var(--hl-bot),
      0 0 0 1px rgba(95,110,135,.24),
      0 18px 30px -16px rgba(40,50,70,.42);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  .vacio.f {
    background: var(--glass-chip);
    box-shadow: var(--ring);
    transform: scale(1.02);
  }
  .vacio p {
    margin: 0;
    font-weight: 700;
    font-size: var(--t-m);
    color: var(--ink-2);
  }
  .btn-cat {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--t-m);
    font-weight: 800;
    color: var(--ink);
  }
</style>
