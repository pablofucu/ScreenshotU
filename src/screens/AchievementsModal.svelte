<script lang="ts">
  import { app } from '../lib/store.svelte';
  import {
    achState,
    getFilteredAchievements,
    closeAchievementsModal,
    toggleRevealSecret,
    formatUnlockDate,
    getRankTier,
    cycleCardVariant,
    VARIANT_NAMES,
  } from '../lib/achievements.svelte';
  import type { Achievement } from '../lib/types';
  import Hints from '../components/Hints.svelte';

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
  ];

  function tierOf(a: Ach): number {
    const p = a.rarityPct;
    if (typeof p === 'number') return p < 5 ? 0 : p < 12 ? 1 : p < 25 ? 2 : p < 50 ? 3 : 4;
    // Sin porcentaje: el backend solo da 0 común, 1 raro, 2 ultra raro.
    const t = a.rarityTier ?? 0;
    return t >= 2 ? 1 : t === 1 ? 2 : 4;
  }

  const fmtPct = (p: number) => p.toFixed(1).replace('.', ',') + ' %';

  const summary = $derived(achState.summary);
  const total = $derived(summary?.total ?? achState.list.length);
  const unlocked = $derived(summary?.unlocked ?? achState.list.filter((a) => a.earned).length);
  const pct = $derived(total ? Math.round((unlocked / total) * 100) : 0);
  const rank = $derived(getRankTier(pct));

  const filtered = $derived(getFilteredAchievements() as Ach[]);
  const selected = $derived(filtered[achState.selectedIdx] ?? filtered[0] ?? null);

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

  function selectAch(index: number) {
    achState.selectedIdx = index;
  }

  function setTab(tab: 'all' | 'unlocked' | 'locked') {
    achState.tab = tab;
    achState.selectedIdx = 0;
  }

  function onToggleSecret() {
    if (selected && selected.hidden && !selected.earned) {
      toggleRevealSecret(selected.apiName);
    }
  }

  function onCycleVariant() {
    if (achState.gameId) {
      void cycleCardVariant(achState.gameId);
    }
  }

  let rootEl = $state<HTMLElement | null>(null);
  let shelvesEl = $state<HTMLElement | null>(null);
  let popEl = $state<HTMLElement | null>(null);
  let popPos = $state({ x: 0, y: 0, left: false, ay: 0 });

  // Foco del mando: lleva el medallón al área visible y coloca el popover a su lado.
  $effect(() => {
    const idx = achState.selectedIdx;
    void filtered;
    void achState.revealedSecrets;
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
    });
  });
</script>

<div class="root" bind:this={rootEl}>
  <header class="top">
    <button class="pillbtn" type="button" onclick={closeAchievementsModal} title="Cerrar (B)">
      <span class="chev">‹</span>
      <b>Volver</b>
    </button>
    <div class="crumb">
      <span>{achState.gameTitle || 'Detalles'}</span>
      <span class="sep">›</span>
      <span class="cur">Logros</span>
    </div>
    <button class="pillbtn" type="button" onclick={onCycleVariant} title="Cambiar diseño global de tarjeta (Y)">
      <small>Diseño:</small>
      <b>{VARIANT_NAMES[app.config.achievementsCardVariant ?? summary?.variant ?? 1] ?? 'Variante'}</b>
      <span class="key">Y</span>
    </button>
  </header>

  <section class="head">
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
      <span class="btn sh">LB</span>
      <button class="chip" class:on={achState.tab === 'all'} type="button" onclick={() => setTab('all')}>
        Todos <small>{total}</small>
      </button>
      <button class="chip" class:on={achState.tab === 'unlocked'} type="button" onclick={() => setTab('unlocked')}>
        Conseguidos <small>{unlocked}</small>
      </button>
      <button class="chip" class:on={achState.tab === 'locked'} type="button" onclick={() => setTab('locked')}>
        Pendientes <small>{total - unlocked}</small>
      </button>
      <span class="btn sh">RB</span>
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
        <div class="shelf" style="--c:var(--rar-{g.ti})">
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
                onclick={() => selectAch(i)}
                title={secret ? 'Logro secreto' : a.displayName}
              >
                {#if a.statMax && !a.earned}
                  <span class="arc" style="--q:{frac(a) * 100}"></span>
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
        <span class="tag" style="--c:var(--rar-{ti})">{TIERS[ti].k}</span>
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
        <button class="reveal" type="button" onclick={onToggleSecret}>
          {isSecret ? 'Revelar logro secreto (X)' : 'Ocultar spoiler (X)'}
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
      ['B', 'Cerrar', 'b'],
    ]}
  />
</div>

<style>
  .root {
    /* Colores semánticos de rareza (de más raro a más común); lo demás sale de los tokens. */
    --rar-0: #9a4bd8;
    --rar-1: #e8384f;
    --rar-2: #f27a1a;
    --rar-3: #3bb54a;
    --rar-4: #00a0d8;
    --hint: 64px;

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

  /* Barra superior */
  .top {
    position: absolute;
    left: 22px;
    right: 22px;
    top: 14px;
    height: 44px;
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .pillbtn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 var(--t-m, 15px);
    border: 0;
    border-radius: var(--r-pill);
    cursor: pointer;
    font: 700 var(--t-m) inherit;
    font-family: inherit;
    color: var(--ink);
    background: var(--glass-chip);
    box-shadow: 0 0 0 1px var(--line), inset 0 1px 0 var(--glass-top);
  }
  .pillbtn small {
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-3);
  }
  .chev {
    font-size: 22px;
    line-height: 1;
    margin-top: -2px;
  }
  .key {
    min-width: 24px;
    height: 24px;
    display: inline-grid;
    place-items: center;
    border-radius: 12px;
    font-size: var(--t-s);
    font-weight: 800;
    color: var(--on, var(--ink));
    background: var(--f1, var(--ink-3));
  }
  .crumb {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--t-xl);
    font-weight: 700;
    color: var(--ink-2);
    min-width: 0;
  }
  .crumb span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .crumb .sep {
    color: var(--ink-3);
    font-weight: 500;
  }
  .crumb .cur {
    color: var(--f1, var(--ink));
  }

  /* Cabecera: anillo + resumen + filtros */
  .head {
    position: absolute;
    left: 22px;
    right: 22px;
    top: 66px;
    height: 100px;
    display: flex;
    align-items: center;
    gap: var(--t-xs, 11px);
    padding: 0 20px 0 14px;
    border-radius: var(--r-m);
    background: var(--glass-in);
    box-shadow: 0 0 0 1px var(--line), inset 0 1px 0 var(--glass-top);
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
    margin-right: 6px;
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
    margin-top: 2px;
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
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: var(--r-pill);
    font-size: var(--t-m);
    color: color-mix(in srgb, var(--c) 70%, var(--ink));
    background: color-mix(in srgb, var(--c) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 45%, transparent);
  }
  .rank :global(svg) {
    width: 16px;
    height: 16px;
  }
  .chips {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px;
    margin-left: auto;
    border-radius: var(--r-pill);
    background: var(--glass-chip);
    box-shadow: 0 0 0 1px var(--line);
  }
  .chips .btn {
    margin: 0 2px;
  }
  .chip {
    height: 32px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--r-pill);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font: 700 var(--t-m) inherit;
    font-family: inherit;
    color: var(--ink-2);
    background: transparent;
    transition: background 0.2s, color 0.2s;
  }
  .chip small {
    font-size: var(--t-s);
    font-weight: 700;
    opacity: 0.7;
  }
  .chip.on {
    color: var(--on, var(--ink));
    background: linear-gradient(180deg, var(--f1, var(--ink-3)), var(--f2, var(--f1, var(--ink-3))));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--f1, var(--ink-3)) 75%, black);
  }

  /* Baldas por rareza */
  .shelves {
    position: absolute;
    left: 22px;
    right: 22px;
    top: 176px;
    bottom: calc(var(--hint) + 6px);
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding: 6px 0;
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
    gap: 16px;
    padding: 6px 16px 6px 0;
    border-radius: var(--r-m);
    background: var(--glass-in);
    box-shadow: 0 0 0 1px var(--line), inset 0 1px 0 var(--glass-top);
  }
  .lab {
    width: 176px;
    flex: none;
    align-self: stretch;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0 16px;
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
    color: var(--ink-3);
  }
  .lab em {
    font-style: normal;
    color: var(--ink-2);
  }
  .meds {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    padding: 6px 0;
  }
  .rest {
    margin-left: auto;
    align-self: center;
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-3);
    white-space: nowrap;
  }

  .med {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    cursor: pointer;
    transition: transform 0.18s;
  }
  .med .ico {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    overflow: hidden;
    display: grid;
    place-items: center;
    color: white;
    background: radial-gradient(
      120% 100% at 30% 15%,
      color-mix(in srgb, var(--c) 30%, white),
      var(--c) 65%,
      color-mix(in srgb, var(--c) 75%, black)
    );
    box-shadow: 0 0 0 3px var(--glass-chip), 0 2px 6px var(--line);
  }
  .med .ico img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .med .ico :global(svg) {
    width: 56%;
    height: 56%;
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
    font-size: 24px;
    font-weight: 800;
  }
  .med.f {
    transform: scale(1.18);
    z-index: 3;
  }
  .med.f .ico {
    box-shadow: 0 0 0 3px var(--glass-chip), 0 0 0 6px var(--c), 0 6px 18px color-mix(in srgb, var(--c) 50%, transparent);
  }
  .med .arc {
    position: absolute;
    inset: -5px;
    border-radius: 50%;
    background: conic-gradient(var(--c) calc(var(--q) * 1%), transparent 0);
    -webkit-mask: radial-gradient(circle, transparent 31px, black 32px);
    mask: radial-gradient(circle, transparent 31px, black 32px);
  }

  /* Popover flotante */
  .pop {
    position: absolute;
    z-index: 20;
    width: 320px;
    padding: 12px 16px;
    border-radius: var(--r-m);
    background: var(--face-hi);
    box-shadow: 0 10px 30px var(--scrim), 0 0 0 1px var(--line);
    backdrop-filter: blur(8px);
    transition: left 0.15s, top 0.15s;
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
    margin: 2px 0 8px;
  }
  .pop .m {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
  }
  .pop .m .r {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .ok {
    display: inline-flex;
    color: var(--ok);
  }
  .ok :global(svg) {
    width: 15px;
    height: 15px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: var(--r-xs);
    font-size: var(--t-xs);
    font-weight: 800;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--c, var(--ink-3)) 80%, var(--ink));
    background: color-mix(in srgb, var(--c, var(--ink-3)) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c, var(--ink-3)) 35%, transparent);
  }
  .pop .meta {
    display: flex;
    gap: 8px;
    margin-top: 8px;
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
    margin-top: 10px;
    height: 32px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-pill);
    cursor: pointer;
    font: 800 var(--t-s) inherit;
    font-family: inherit;
    color: var(--on, var(--ink));
    background: linear-gradient(180deg, var(--f1, var(--ink-3)), var(--f2, var(--f1, var(--ink-3))));
  }

  .empty {
    margin: auto;
    padding: 24px;
    text-align: center;
    font-size: var(--t-l);
    font-weight: 700;
    color: var(--ink-3);
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
