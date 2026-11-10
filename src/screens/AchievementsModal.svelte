<script lang="ts">
  import { app } from '../lib/store.svelte';
  import {
    achState,
    getFilteredAchievements,
    toggleRevealSecret,
    formatUnlockDate,
    getRankTier,
    VARIANT_NAMES,
  } from '../lib/achievements.svelte';
  import type { Achievement } from '../lib/types';
  import Hints from '../components/Hints.svelte';
  import TopBar from '../components/TopBar.svelte';

  // El backend (visor-core) ya envía estos campos; el tipo del frontend aún no los declara.
  type Ach = Achievement & {
    rarityPct?: number | null;
    rarityTier?: number;
    statCurrent?: number | null;
    statMax?: number | null;
  };

  // Niveles de rareza, de más raro (arriba) a más común. Los colores viven en el CSS (--rar-N).
  const TIERS = [
    { k: 'Legendario', range: 'menos del 5 %' },
    { k: 'Ultra raro', range: '5–12 %' },
    { k: 'Raro', range: '12–25 %' },
    { k: 'Poco común', range: '25–50 %' },
    { k: 'Común', range: '50 % o más' },
    { k: 'Sin datos de rareza', range: 'sin porcentaje' },
  ];
  const NO_DATA = 5;

  function tierOf(a: Ach): number {
    const p = a.rarityPct;
    if (typeof p === 'number') return p < 5 ? 0 : p < 12 ? 1 : p < 25 ? 2 : p < 50 ? 3 : 4;
    // Sin porcentaje: el backend solo da 0 común, 1 raro, 2 ultra raro.
    const t = a.rarityTier ?? 0;
    if (t >= 1) return t >= 2 ? 1 : 2;
    // Sin dato real nunca es «Común»: los secretos sin conseguir van a «Raro», el resto a «Sin datos».
    return a.hidden && !a.earned ? 2 : NO_DATA;
  }

  const fmtPct = (p: number) => p.toFixed(1).replace('.', ',') + ' %';

  const summary = $derived(achState.summary);
  const total = $derived(summary?.total ?? achState.list.length);
  const unlocked = $derived(summary?.unlocked ?? achState.list.filter((a) => a.earned).length);
  const pct = $derived(total ? Math.round((unlocked / total) * 100) : 0);
  const rank = $derived(getRankTier(pct));

  const filtered = $derived(getFilteredAchievements() as Ach[]);
  // selectedIdx < 0: tarjeta cerrada.
  const selected = $derived(achState.selectedIdx >= 0 ? (filtered[achState.selectedIdx] ?? null) : null);

  // Los logros con porcentaje de jugadores por debajo del 12 % que ya tienes.
  const rareOwned = $derived((achState.list as Ach[]).filter((a) => a.earned && tierOf(a) <= 1).length);
  const hasPct = $derived((achState.list as Ach[]).some((a) => typeof a.rarityPct === 'number'));

  // La lista global se ordena por rareza (luego conseguidos, luego más raros) para que el orden
  // de lectura de las baldas coincida con selectedIdx y el mando (arriba/abajo) recorra lo que se ve.
  $effect(() => {
    const cur = achState.list as Ach[];
    const sorted = [...cur].sort(
      (a, b) =>
        tierOf(a) - tierOf(b) ||
        Number(b.earned) - Number(a.earned) ||
        (a.rarityPct ?? 100) - (b.rarityPct ?? 100)
    );
    if (sorted.some((a, i) => a !== cur[i])) achState.list = sorted;
  });

  const groups = $derived(
    TIERS.map((t, ti) => {
      const items = filtered.map((a, i) => ({ a, i })).filter((x) => tierOf(x.a) === ti);
      const all = (achState.list as Ach[]).filter((a) => tierOf(a) === ti);
      return { ...t, ti, items, ok: all.filter((a) => a.earned).length, count: all.length };
    }).filter((g) => g.items.length > 0)
  );

  const trophySvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M9 17h6"/></svg>`;
  const lockSvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>`;
  const checkSvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`;

  function isSecretHidden(ach: { hidden: boolean; earned: boolean; apiName: string }) {
    return ach.hidden && !ach.earned && !achState.revealedSecrets.has(ach.apiName);
  }

  function frac(a: Ach): number {
    return a.statMax ? Math.min(1, (a.statCurrent ?? 0) / a.statMax) : 0;
  }

  // Ratón: pasar por encima muestra la tarjeta; un clic la fija hasta otro clic o un clic fuera.
  let pinnedIdx = $state(-1);

  $effect(() => {
    if (achState.selectedIdx !== pinnedIdx) pinnedIdx = -1;
  });

  function hoverAch(index: number) {
    if (pinnedIdx < 0) achState.selectedIdx = index;
  }

  function leaveAch() {
    if (pinnedIdx < 0) achState.selectedIdx = -1;
  }

  function clickAch(index: number) {
    if (pinnedIdx === index) {
      pinnedIdx = -1;
    } else {
      achState.selectedIdx = index;
      pinnedIdx = index;
    }
  }

  function onRootClick(e: MouseEvent) {
    if (pinnedIdx >= 0 && !(e.target as Element).closest('.med, .pop')) {
      pinnedIdx = -1;
      achState.selectedIdx = -1;
    }
  }

  function setTab(tab: 'all' | 'unlocked' | 'locked') {
    achState.tab = tab;
    achState.selectedIdx = -1;
  }

  function onToggleSecret() {
    if (selected && selected.hidden && !selected.earned) {
      toggleRevealSecret(selected.apiName);
    }
  }

  let rootEl = $state<HTMLElement | null>(null);
  let shelvesEl = $state<HTMLElement | null>(null);
  let popEl = $state<HTMLElement | null>(null);
  let popPos = $state({ x: 0, y: 0, left: false, ay: 0 });
  // La tarjeta solo se desliza entre medallones; al abrirse aparece ya en su sitio.
  let popAnim = $state(false);

  // Foco del mando: lleva el medallón al área visible y coloca el popover a su lado.
  $effect(() => {
    const idx = achState.selectedIdx;
    void filtered;
    void achState.revealedSecrets;
    if (idx < 0) {
      popAnim = false;
      return;
    }
    if (!rootEl || !shelvesEl) return;
    const el = shelvesEl.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
    if (!el) return;
    el.scrollIntoView({ block: 'nearest' });
    queueMicrotask(() => {
      if (!rootEl || !popEl) return;
      const s = rootEl.getBoundingClientRect();
      const r = el.getBoundingClientRect();
      const K = s.width / rootEl.offsetWidth || 1;
      const L = (r.left - s.left) / K;
      const R = (r.right - s.left) / K;
      const cy = (r.top + r.height / 2 - s.top) / K;
      const h = popEl.offsetHeight;
      const right = R + 18 + 320 <= rootEl.offsetWidth - 22;
      const y = Math.max(76, Math.min(rootEl.offsetHeight - 70 - h, cy - h / 2));
      popPos = { x: right ? R + 18 : L - 18 - 320, y, left: !right, ay: cy - y };
      if (!popAnim) requestAnimationFrame(() => (popAnim = achState.selectedIdx >= 0));
    });
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="root" bind:this={rootEl} onclick={onRootClick}>
  <TopBar crumbs={['Inicio', app.platform?.name ?? '', achState.gameTitle || 'Detalles', 'Logros']} />

  <section class="head glass-in">
    <div class="ring" style="--p:{pct}">
      <u></u><i></i>
      <span><b>{pct} %</b></span>
    </div>
    <div class="sum">
      <div class="t">Logros · {achState.gameTitle || 'Detalles'}</div>
      <div class="s">
        <b>{unlocked}</b> de {total} logros{#if hasPct}
          · <b>{rareOwned}</b> por debajo del 12 % de jugadores{/if}
      </div>
    </div>
    <span class="rank" style="--c:{rank.color}">
      {@html trophySvg}
      <b>{rank.name}</b>
    </span>

    <nav class="chips">
      <button class="chip" class:on={achState.tab === 'all'} type="button" onclick={() => setTab('all')}>
        Todos <small>{total}</small>
      </button>
      <button class="chip" class:on={achState.tab === 'unlocked'} type="button" onclick={() => setTab('unlocked')}>
        Conseguidos <small>{unlocked}</small>
      </button>
      <button class="chip" class:on={achState.tab === 'locked'} type="button" onclick={() => setTab('locked')}>
        Pendientes <small>{total - unlocked}</small>
      </button>
    </nav>
  </section>

  <div class="shelves" bind:this={shelvesEl}>
    {#if filtered.length === 0}
      <div class="empty">
        {#if achState.tab === 'unlocked'}
          Todavía no has conseguido ningún logro en este juego.
        {:else if achState.tab === 'locked'}
          ¡Enhorabuena! Has completado el 100% de los logros.
        {:else}
          No se han encontrado logros registrados para este juego.
        {/if}
      </div>
    {:else}
      {#each groups as g (g.ti)}
        <div class="shelf glass-in" style="--c:var(--rar-{g.ti})">
          <div class="lab">
            <b>{g.k}</b>
            <small><em>{g.ok}/{g.count}</em> · {g.range}</small>
          </div>
          <div class="meds">
            {#each g.items as { a, i } (a.apiName)}
              {@const secret = isSecretHidden(a)}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div
                class="med"
                class:f={i === achState.selectedIdx}
                class:earned={a.earned}
                class:secret
                data-idx={i}
                onmouseenter={() => hoverAch(i)}
                onmouseleave={leaveAch}
                onclick={() => clickAch(i)}
              >
                {#if a.statMax && !a.earned}
                  <svg class="arc" viewBox="0 0 66 66" aria-hidden="true">
                    <rect class="trk" x="2" y="2" width="62" height="62" rx="15" />
                    <rect class="bar" x="2" y="2" width="62" height="62" rx="15" pathLength="100" stroke-dasharray="{frac(a) * 100} 100" />
                  </svg>
                {/if}
                <div class="ico">
                  {#if secret}
                    <b class="qmark">?</b>
                  {:else if a.icon}
                    <img src={a.earned ? a.icon : (a.iconGray ?? a.icon)} alt={a.displayName} />
                  {:else if a.earned}
                    {@html trophySvg}
                  {:else}
                    {@html lockSvg}
                  {/if}
                </div>
              </div>
            {/each}
          </div>
          <span class="rest">{g.ok === g.count ? 'Completo' : g.count - g.ok + ' por conseguir'}</span>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Detalle del logro enfocado, flotando junto a su medallón -->
  <div
    class="pop"
    class:lft={popPos.left}
    class:off={!selected}
    class:mv={popAnim}
    bind:this={popEl}
    style="left:{popPos.x}px;top:{popPos.y}px;--ay:{popPos.ay}px"
  >
    {#if selected}
      {@const isSecret = isSecretHidden(selected)}
      {@const ti = tierOf(selected)}
      <div class="n">{isSecret ? 'Logro secreto' : selected.displayName}</div>
      <div class="d">
        {isSecret
          ? 'Los detalles de este logro están ocultos para no desvelar partes de la trama.'
          : selected.description}
      </div>
      <div class="m">
        {#if ti !== NO_DATA}<span class="tag" style="--c:var(--rar-{ti})">{TIERS[ti].k}</span>{/if}
        {#if typeof selected.rarityPct === 'number'}<span>{fmtPct(selected.rarityPct)}</span>{/if}
        {#if selected.hidden}<span class="tag sec">Secreto</span>{/if}
        <span class="r">
          {#if selected.earned}
            <span class="ok">{@html checkSvg}</span>{formatUnlockDate(selected.unlockedAt)}
          {:else if selected.statMax}
            {selected.statCurrent ?? 0}/{selected.statMax}
          {:else}
            Pendiente
          {/if}
        </span>
      </div>
      <div class="meta">
        <code>{selected.apiName}</code>
        {#if selected.source}<span>{selected.source.toUpperCase()}</span>{/if}
      </div>
      {#if selected.hidden && !selected.earned}
        <button class="b-main reveal" type="button" onclick={onToggleSecret}>
          {isSecret ? 'Revelar logro secreto' : 'Ocultar spoiler'}
        </button>
      {/if}
    {/if}
  </div>

  <div class="hintband"></div>
  <Hints
    list={[
      ['A', 'Seleccionar', 'a'],
      ['LB/RB', 'Cambiar pestaña', 'lb'],
      ...(selected?.hidden && !selected.earned ? ([['X', 'Revelar secreto', 'x']] as [string, string, string][]) : []),
      ['Y', 'Cambiar tarjeta (' + (VARIANT_NAMES[app.config.achievementsCardVariant ?? summary?.variant ?? 1] ?? '') + ')', 'y'],
      ['B', selected ? 'Cerrar tarjeta' : 'Cerrar', 'b'],
    ]}
  />
</div>

<style>
  .root {
    --hint: 64px;
    --mx: var(--s-5);

    position: absolute;
    inset: 0;
    z-index: 100;
    overflow: hidden;
    user-select: none;
    color: var(--ink);
    font-weight: 500;
    background:
      radial-gradient(120% 90% at 50% 0%, color-mix(in srgb, var(--g1, var(--face)) 16%, var(--face)) 0%, var(--face) 70%),
      var(--face);
  }

  /* Cabecera: anillo + resumen + filtros */
  .head {
    position: absolute;
    left: var(--mx);
    right: var(--mx);
    top: 84px;
    height: 100px;
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 0 var(--s-4) 0 var(--s-3);
  }
  .head > * {
    flex: none;
  }
  .ring {
    --sz: 80px;
    --th: 9px;
    --mask: radial-gradient(farthest-side, transparent calc(100% - var(--th)), black calc(100% - var(--th) + 1px));
    width: var(--sz);
    height: var(--sz);
    position: relative;
    border-radius: 50%;
    margin-right: var(--s-1);
  }
  .ring u,
  .ring i {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    -webkit-mask: var(--mask);
    mask: var(--mask);
  }
  .ring u {
    background: var(--line);
  }
  .ring i {
    background: conic-gradient(
      var(--rar-4),
      var(--rar-3),
      var(--rar-2),
      var(--rar-1),
      var(--rar-0) calc(var(--p) * 1%),
      transparent 0
    );
  }
  .ring span {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    line-height: 1;
  }
  .ring b {
    font-size: var(--t-l);
    font-weight: 800;
  }
  .sum {
    min-width: 0;
    flex: 1 1 auto;
  }
  .sum .t {
    font-size: var(--t-2xl);
    font-weight: 800;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sum .s {
    margin-top: var(--s-1);
    font-size: var(--t-m);
    color: var(--ink-2);
  }
  .sum .s b {
    color: var(--ink);
    font-weight: 800;
  }
  .rank {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: 32px;
    padding: 0 var(--s-3);
    border-radius: var(--r-pill);
    font-size: var(--t-m);
    color: color-mix(in srgb, var(--c) 55%, var(--ink));
    background: color-mix(in srgb, var(--c) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 55%, var(--line));
  }
  .rank :global(svg) {
    width: var(--i-s);
    height: var(--i-s);
  }

  /* Selector de pestañas (familia .chip de la guía) */
  .chips {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    padding: var(--s-1);
    margin-left: auto;
    border-radius: var(--r-pill);
    background: var(--glass-chip);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: 36px;
    padding: 0 var(--s-4);
    border: 0;
    border-radius: var(--r-pill);
    cursor: pointer;
    font: 700 var(--t-m) / 1 var(--round);
    color: var(--ink-2);
    background: transparent;
    transition: transform var(--d-2) var(--e-bounce), background var(--d-2) var(--e-soft), color var(--d-2) var(--e-soft);
  }
  .chip small {
    font-size: var(--t-s);
    font-weight: 700;
    opacity: 0.75;
  }
  .chip.on {
    color: var(--on);
    background: var(--gf);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.5), 0 0 0 1px color-mix(in srgb, var(--f2) 75%, black);
  }
  .chip:active {
    transform: scale(0.97);
    transition-duration: var(--d-1);
  }

  /* Baldas por rareza */
  .shelves {
    position: absolute;
    left: var(--mx);
    right: var(--mx);
    top: 192px;
    bottom: calc(var(--hint) + var(--s-2));
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    overflow-y: auto;
    padding: var(--s-2) 0;
    scrollbar-width: none;
  }
  .shelves::-webkit-scrollbar {
    display: none;
  }
  .shelf {
    flex: 1 0 auto;
    min-height: 72px;
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-2) var(--s-4) var(--s-2) 0;
  }
  .lab {
    width: 176px;
    flex: none;
    align-self: stretch;
    display: flex;
    flex-direction: column;
    justify-content: center;
    margin: calc(-1 * var(--s-2)) 0;
    padding: 0 var(--s-4);
    border-radius: var(--r-m) 0 0 var(--r-m);
    border-left: 6px solid var(--c);
    background: linear-gradient(90deg, color-mix(in srgb, var(--c) 22%, transparent), transparent);
  }
  .lab b {
    font-size: var(--t-l);
    font-weight: 800;
    color: color-mix(in srgb, var(--c) 75%, var(--ink));
  }
  .lab small {
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
  }
  .lab em {
    font-style: normal;
    color: var(--ink);
  }
  .meds {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-3);
    align-items: center;
    padding: var(--s-1) 0;
  }
  .rest {
    margin-left: auto;
    align-self: center;
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
    white-space: nowrap;
  }

  .med {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: var(--r-s);
    cursor: pointer;
    transition: transform var(--d-2) var(--e-bounce);
  }
  .med .ico {
    width: 100%;
    height: 100%;
    border-radius: var(--r-s);
    overflow: hidden;
    display: grid;
    place-items: center;
    color: var(--on);
    background: radial-gradient(
      120% 100% at 30% 15%,
      color-mix(in srgb, var(--c) 30%, var(--tint-base)),
      var(--c) 65%,
      color-mix(in srgb, var(--c) 75%, black)
    );
    box-shadow: 0 0 0 3px var(--glass-chip), var(--sh-1);
  }
  .med .ico img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .med .ico :global(svg) {
    width: var(--i-l);
    height: var(--i-l);
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.25));
  }
  .med:not(.earned) .ico {
    color: var(--ink-3);
    background: linear-gradient(160deg, var(--face-hi), color-mix(in srgb, var(--ink-3) 35%, var(--face)));
  }
  .med:not(.earned) .ico img {
    filter: grayscale(1);
    opacity: 0.55;
  }
  .qmark {
    font-size: var(--t-xl);
    font-weight: 800;
  }
  .med.f {
    transform: scale(1.18);
    z-index: 3;
  }
  .med.f .ico {
    box-shadow: 0 0 0 3px var(--glass-chip), 0 0 0 6px var(--c), 0 6px 18px color-mix(in srgb, var(--c) 50%, transparent);
  }
  /* Progreso: rectángulo redondeado alrededor del icono; pathLength=100 hace que el dash sea el porcentaje. */
  .med .arc {
    position: absolute;
    inset: -5px;
    width: 66px;
    height: 66px;
    pointer-events: none;
    overflow: visible;
  }
  .med .arc rect {
    fill: none;
    stroke-width: 3;
  }
  .med .arc .trk {
    stroke: var(--line);
  }
  .med .arc .bar {
    stroke: var(--c);
  }

  /* Ficha flotante del logro (altura flotante) */
  .pop {
    position: absolute;
    z-index: 20;
    width: 320px;
    padding: var(--s-3) var(--s-4);
    border-radius: var(--r-m);
    background: var(--face-hi);
    box-shadow: var(--sh-3);
    -webkit-backdrop-filter: var(--blur-panel);
    backdrop-filter: var(--blur-panel);
  }
  .pop.mv {
    transition: left var(--d-1) var(--e-soft), top var(--d-1) var(--e-soft);
  }
  .pop.off {
    display: none;
  }
  .pop::after {
    content: '';
    position: absolute;
    top: var(--ay, 50%);
    left: -7px;
    width: 14px;
    height: 14px;
    margin-top: -7px;
    background: inherit;
    transform: rotate(45deg);
    box-shadow: -1px 1px 0 var(--line);
  }
  .pop.lft::after {
    left: auto;
    right: -7px;
    box-shadow: 1px -1px 0 var(--line);
  }
  .pop .n {
    font-size: var(--t-l);
    font-weight: 800;
    line-height: 1.2;
  }
  .pop .d {
    font-size: var(--t-s);
    color: var(--ink-2);
    margin: var(--s-1) 0 var(--s-2);
  }
  .pop .m {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2);
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
  }
  .pop .m .r {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
  }
  .ok {
    display: inline-flex;
    color: var(--ok);
  }
  .ok :global(svg) {
    width: var(--i-s);
    height: var(--i-s);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 var(--s-2);
    border-radius: var(--r-xs);
    font-size: var(--t-xs);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--c, var(--ink-3)) 70%, var(--ink));
    background: color-mix(in srgb, var(--c, var(--ink-3)) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c, var(--ink-3)) 35%, transparent);
  }
  .pop .meta {
    display: flex;
    gap: var(--s-2);
    margin-top: var(--s-2);
    font-size: var(--t-xs);
    font-weight: 700;
    color: var(--ink-3);
  }
  .pop .meta code {
    font-family: ui-monospace, Consolas, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .reveal {
    margin-top: var(--s-3);
  }

  .empty {
    margin: auto;
    padding: var(--s-5);
    text-align: center;
    font-size: var(--t-l);
    font-weight: 700;
    color: var(--ink-2);
  }

  .hintband {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: var(--hint);
    z-index: 6;
    pointer-events: none;
    background: linear-gradient(180deg, transparent, color-mix(in srgb, var(--face) 85%, transparent) 45%);
  }
  .root :global(.hints) {
    z-index: 7;
  }
</style>
