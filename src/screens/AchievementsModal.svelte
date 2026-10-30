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
  import Hints from '../components/Hints.svelte';

  const summary = $derived(achState.summary);
  const total = $derived(summary?.total ?? achState.list.length);
  const unlocked = $derived(summary?.unlocked ?? achState.list.filter((a) => a.earned).length);
  const pct = $derived(total ? Math.round((unlocked / total) * 100) : 0);
  const rank = $derived(getRankTier(pct));

  const filtered = $derived(getFilteredAchievements());
  const selected = $derived(filtered[achState.selectedIdx] ?? filtered[0] ?? null);

  const trophySvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M9 17h6"/></svg>`;
  const lockSvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>`;
  const checkSvg = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`;

  function isSecretHidden(ach: { hidden: boolean; earned: boolean; apiName: string }) {
    return ach.hidden && !ach.earned && !achState.revealedSecrets.has(ach.apiName);
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

  let listEl = $state<HTMLElement | null>(null);

  // Auto-scroll suave para el foco del mando sin necesidad de barra de scroll
  $effect(() => {
    const idx = achState.selectedIdx;
    if (listEl) {
      const el = listEl.querySelector<HTMLElement>(`[data-idx="${idx}"]`);
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
  });
</script>

<div class="modal-root">
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="overlay-backdrop" onclick={closeAchievementsModal}></div>

  <div class="glass modal-box">
    <div class="face modal-content">
      <!-- Encabezado con información del juego y estadísticas de logros -->
      <header class="modal-header">
        <button class="btn-back" type="button" onclick={closeAchievementsModal} title="Cerrar (B)">
          <span class="btn-circle">‹</span>
          <b>Volver</b>
        </button>

        <div class="header-info">
          <h2>Logros · {achState.gameTitle || 'Detalles'}</h2>
          <div class="header-chips">
            <span class="chip rank" style="--c:{rank.color}">
              {@html trophySvg}
              <b>{rank.name}</b>
            </span>
            <span class="chip count">
              <b>{unlocked}</b> / {total} completados
            </span>
            <span class="chip pct">
              <b>{pct}%</b>
            </span>
          </div>
        </div>

        <button class="btn-variant" type="button" onclick={onCycleVariant} title="Cambiar diseño global de tarjeta (Y)">
          <small>Diseño:</small>
          <b>{VARIANT_NAMES[app.config.achievementsCardVariant ?? summary?.variant ?? 1] ?? 'Variante'}</b>
          <span class="btn-chip-key">Y</span>
        </button>
      </header>

      <!-- Barra de pestañas con atajos LB / RB -->
      <nav class="tabs-bar">
        <div class="tab-scroller">
          <button class="tab-btn" class:active={achState.tab === 'all'} onclick={() => setTab('all')}>
            <span>Todos</span>
            <small>{total}</small>
          </button>
          <button class="tab-btn" class:active={achState.tab === 'unlocked'} onclick={() => setTab('unlocked')}>
            <span>Conseguidos</span>
            <small>{unlocked}</small>
          </button>
          <button class="tab-btn" class:active={achState.tab === 'locked'} onclick={() => setTab('locked')}>
            <span>Pendientes</span>
            <small>{total - unlocked}</small>
          </button>
        </div>
        <div class="tab-hints">
          <span class="bumper">LB</span>
          <span class="bumper">RB</span>
        </div>
      </nav>

      <!-- Cuerpo principal: Dos columnas -->
      <div class="modal-body">
        <!-- Columna izquierda: Lista scrollable de logros (sin barras visibles) -->
        <div class="ach-list" bind:this={listEl}>
          {#if filtered.length === 0}
            <div class="empty-state">
              {#if achState.tab === 'unlocked'}
                Todavía no has conseguido ningún logro en este juego.
              {:else if achState.tab === 'locked'}
                ¡Enhorabuena! Has completado el 100% de los logros.
              {:else}
                No se han encontrado logros registrados para este juego.
              {/if}
            </div>
          {:else}
            {#each filtered as ach, i (ach.apiName)}
              {@const hidden = isSecretHidden(ach)}
              <button
                type="button"
                class="ach-item"
                data-idx={i}
                class:selected={i === achState.selectedIdx}
                class:earned={ach.earned}
                class:is-hidden={hidden}
                onclick={() => selectAch(i)}
                tabindex="-1"
              >
                <div class="ach-ico" class:earned={ach.earned} class:secret={hidden}>
                  {#if hidden}
                    <b class="qmark">?</b>
                  {:else if ach.icon}
                    <img src={ach.icon} alt={ach.displayName} class="ach-ico-img" />
                  {:else if ach.earned}
                    {@html trophySvg}
                  {:else}
                    {@html lockSvg}
                  {/if}
                </div>

                <div class="ach-meta">
                  <div class="ach-name-row">
                    <b>{hidden ? 'Logro secreto' : ach.displayName}</b>
                    {#if ach.earned}
                      <span class="badge-done">{@html checkSvg} Conseguido</span>
                    {/if}
                  </div>
                  <p class="ach-desc">
                    {hidden ? 'Continúa jugando para desvelar este logro' : ach.description}
                  </p>
                  <div class="ach-foot">
                    {#if ach.earned}
                      <small class="ach-date">{formatUnlockDate(ach.unlockedAt)}</small>
                    {:else}
                      <small class="ach-locked">Pendiente</small>
                    {/if}
                  </div>
                </div>
              </button>
            {/each}
          {/if}
        </div>

        <!-- Columna derecha: Detalle extendido e inspección -->
        <div class="ach-detail">
          {#if selected}
            {@const isSecret = isSecretHidden(selected)}
            <div class="detail-card">
              <div class="detail-header">
                <div class="detail-ico" class:earned={selected.earned} class:secret={isSecret}>
                  {#if isSecret}
                    <b class="big-qmark">?</b>
                  {:else if selected.icon}
                    <img src={selected.icon} alt={selected.displayName} class="detail-ico-img" />
                  {:else if selected.earned}
                    {@html trophySvg}
                  {:else}
                    {@html lockSvg}
                  {/if}
                </div>
                <div class="detail-titles">
                  <h3>{isSecret ? 'Logro secreto' : selected.displayName}</h3>
                  <div class="detail-tags">
                    {#if selected.earned}
                      <span class="tag-unlocked">{@html checkSvg} Desbloqueado</span>
                    {:else}
                      <span class="tag-locked">{@html lockSvg} Bloqueado</span>
                    {/if}
                    {#if selected.hidden}
                      <span class="tag-secret">Secreto</span>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="detail-desc-box">
                <p>{isSecret ? 'Los detalles de este logro están ocultos para no desvelar partes de la trama.' : selected.description}</p>
                {#if selected.hidden && !selected.earned}
                  <button class="btn-reveal" type="button" onclick={onToggleSecret}>
                    <b>{isSecret ? 'Revelar logro secreto (X)' : 'Ocultar spoiler'}</b>
                  </button>
                {/if}
              </div>

              <!-- Vista previa de captura automática o estado -->
              <div class="detail-evidence-box">
                <div class="evidence-title">
                  <b>Captura del momento</b>
                  <small>{selected.earned ? 'Guardada automáticamente' : 'Se capturará al desbloquear'}</small>
                </div>
                <div class="evidence-preview">
                  {#if selected.earned}
                    <div class="shot-mock">
                      <div class="shot-hud">
                        <span class="hud-chip">{@html trophySvg} {selected.displayName}</span>
                      </div>
                    </div>
                  {:else}
                    <div class="shot-placeholder">
                      {@html lockSvg}
                      <span>Juega para conseguir este momento</span>
                    </div>
                  {/if}
                </div>
              </div>

              <div class="detail-meta-list">
                <div class="meta-row">
                  <span>Fecha de obtención:</span>
                  <b>{formatUnlockDate(selected.unlockedAt)}</b>
                </div>
                <div class="meta-row">
                  <span>Identificador API:</span>
                  <code>{selected.apiName}</code>
                </div>
                {#if selected.source}
                  <div class="meta-row">
                    <span>Origen de datos:</span>
                    <b>{selected.source.toUpperCase()} (Lector de estado local)</b>
                  </div>
                {/if}
              </div>
            </div>
          {:else}
            <div class="empty-detail">
              Selecciona un logro de la lista para ver sus detalles y su captura.
            </div>
          {/if}
        </div>
      </div>

      <!-- Barra de atajos de mando / teclado inferior -->
      <footer class="modal-footer">
        <Hints
          list={[
            ['A', 'Seleccionar', 'a'],
            ['LB/RB', 'Cambiar pestaña', 'lb'],
            ...(selected?.hidden && !selected.earned ? ([['X', 'Revelar secreto', 'x']] as [string, string, string][]) : []),
            ['Y', 'Cambiar tarjeta (' + (VARIANT_NAMES[app.config.achievementsCardVariant ?? summary?.variant ?? 1] ?? '') + ')', 'y'],
            ['B', 'Cerrar', 'b'],
          ]}
        />
      </footer>
    </div>
  </div>
</div>

<style>
  .modal-root {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    user-select: none;
  }
  .overlay-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(10, 14, 24, 0.72);
    backdrop-filter: blur(8px);
  }

  .modal-box {
    position: relative;
    z-index: 2;
    width: 1140px;
    height: 640px;
    --r: 28px;
    --bw: 10px;
    transform: none !important;
  }
  .modal-content {
    display: flex;
    flex-direction: column;
    padding: 22px 28px 16px;
    background: radial-gradient(120% 90% at 50% 0%, #fbfcfe 0%, #edf0f5 55%, #e2e6ed 100%);
    color: #3a414e;
    overflow: hidden;
  }

  /* Cabecera */
  .modal-header {
    display: flex;
    align-items: center;
    gap: 18px;
    margin-bottom: 14px;
    flex: none;
  }
  .btn-back {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.85);
    border: 0;
    border-radius: 999px;
    height: 42px;
    padding: 0 18px 0 10px;
    font: 800 15px 'M PLUS Rounded 1c', sans-serif;
    color: #4a5260;
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12), inset 0 1px 0 #fff;
    transition: transform 0.1s, background 0.15s;
  }
  .btn-back:hover {
    background: #fff;
    transform: scale(1.03);
  }
  .btn-circle {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: #00A0D8;
    color: #fff;
    display: grid;
    place-items: center;
    font-size: 20px;
    line-height: 1;
    padding-bottom: 2px;
  }

  .header-info {
    flex: 1;
    min-width: 0;
  }
  .header-info h2 {
    margin: 0 0 4px;
    font-size: 24px;
    font-weight: 800;
    color: #2a313d;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .header-chips {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.7);
    box-shadow: inset 0 1px 0 #fff, 0 1px 3px rgba(0, 0, 0, 0.08);
  }
  .chip.rank {
    background: linear-gradient(180deg, #fff, #f4f6fa);
    border: 1px solid rgba(0, 160, 216, 0.3);
  }
  .chip.rank :global(svg) {
    width: 14px;
    height: 14px;
    color: var(--c, #00A0D8);
  }
  .chip.pct {
    background: #00A0D8;
    color: #fff;
    box-shadow: 0 2px 5px rgba(0, 160, 216, 0.35);
  }

  .btn-variant {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 16px;
    border-radius: 999px;
    border: 0;
    background: rgba(255, 255, 255, 0.85);
    color: #4a5260;
    font: 700 13px 'M PLUS Rounded 1c', sans-serif;
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1), inset 0 1px 0 #fff;
    transition: transform 0.1s, background 0.15s;
  }
  .btn-variant:hover {
    background: #fff;
    transform: scale(1.03);
  }
  .btn-chip-key {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #F2B807;
    color: #fff;
    font-size: 11px;
    font-weight: 800;
    display: grid;
    place-items: center;
  }

  /* Pestañas */
  .tabs-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
    flex: none;
    border-bottom: 1px solid rgba(120, 130, 150, 0.2);
    padding-bottom: 8px;
  }
  .tab-scroller {
    display: flex;
    gap: 8px;
  }
  .tab-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 14px;
    border-radius: 999px;
    border: 0;
    background: transparent;
    color: #657082;
    font: 800 13.5px 'M PLUS Rounded 1c', sans-serif;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .tab-btn:hover {
    background: rgba(255, 255, 255, 0.6);
    color: #2a313d;
  }
  .tab-btn.active {
    background: #00A0D8;
    color: #fff;
    box-shadow: 0 2px 6px rgba(0, 160, 216, 0.35);
  }
  .tab-btn small {
    padding: 2px 6px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.1);
    font-size: 11px;
  }
  .tab-btn.active small {
    background: rgba(255, 255, 255, 0.25);
  }
  .tab-hints {
    display: flex;
    gap: 6px;
  }
  .bumper {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.8);
    font-size: 11px;
    font-weight: 800;
    color: #6a7282;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  /* Cuerpo */
  .modal-body {
    flex: 1;
    display: grid;
    grid-template-columns: 580px 1fr;
    gap: 20px;
    min-height: 0;
  }

  /* Lista (scroll sin barra visible, navegación por mando) */
  .ach-list {
    overflow-y: auto;
    padding-right: 4px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .ach-list::-webkit-scrollbar {
    display: none;
    width: 0;
    height: 0;
  }
  .ach-item {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.8);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.04);
    cursor: pointer;
    transition: transform 0.1s, background 0.15s, box-shadow 0.15s;
  }
  .ach-item:hover {
    background: #fff;
    transform: translateY(-1px);
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.08);
  }
  .ach-item.selected {
    background: #fff;
    box-shadow: 0 0 0 2.5px #00A0D8, 0 6px 14px rgba(0, 160, 216, 0.2);
  }

  .ach-ico {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    flex: none;
    display: grid;
    place-items: center;
    background: #dfe4ec;
    color: #7a8291;
    overflow: hidden;
  }
  .ach-ico.earned {
    background: linear-gradient(150deg, #f59e0b, #d97706);
    color: #fff;
    box-shadow: 0 2px 6px rgba(217, 119, 6, 0.35);
  }
  .ach-ico.secret {
    background: #475569;
    color: #fff;
  }
  .ach-ico :global(svg) {
    width: 22px;
    height: 22px;
  }
  .ach-ico-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 12px;
  }
  .ach-item:not(.earned) .ach-ico-img {
    filter: grayscale(0.85) opacity(0.65);
  }
  .qmark {
    font-size: 22px;
    font-weight: 800;
  }

  .ach-meta {
    flex: 1;
    min-width: 0;
  }
  .ach-name-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 3px;
  }
  .ach-name-row b {
    font-size: 14.5px;
    font-weight: 800;
    color: #2a313d;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge-done {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 800;
    color: #10b981;
  }
  .badge-done :global(svg) {
    width: 12px;
    height: 12px;
  }

  .ach-desc {
    margin: 0 0 4px;
    font-size: 12px;
    font-weight: 600;
    color: #6a7282;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ach-foot small {
    font-size: 11px;
    font-weight: 700;
    color: #94a3b8;
  }
  .ach-date {
    color: #059669 !important;
  }

  /* Detalle lateral (scroll sin barra visible) */
  .ach-detail {
    background: rgba(255, 255, 255, 0.85);
    border-radius: 20px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    box-shadow: inset 0 1px 0 #fff, 0 4px 14px rgba(0, 0, 0, 0.05);
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .ach-detail::-webkit-scrollbar {
    display: none;
    width: 0;
    height: 0;
  }
  .detail-card {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .detail-header {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .detail-ico {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    display: grid;
    place-items: center;
    background: #cbd5e1;
    color: #64748b;
    flex: none;
    overflow: hidden;
  }
  .detail-ico.earned {
    background: linear-gradient(150deg, #f59e0b, #d97706);
    color: #fff;
    box-shadow: 0 4px 12px rgba(217, 119, 6, 0.4);
  }
  .detail-ico.secret {
    background: #334155;
    color: #fff;
  }
  .detail-ico :global(svg) {
    width: 34px;
    height: 34px;
  }
  .detail-ico-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 16px;
  }
  .detail-ico:not(.earned) .detail-ico-img {
    filter: grayscale(0.85) opacity(0.65);
  }
  .big-qmark {
    font-size: 32px;
    font-weight: 800;
  }

  .detail-titles h3 {
    margin: 0 0 6px;
    font-size: 20px;
    font-weight: 800;
    color: #1e293b;
  }
  .detail-tags {
    display: flex;
    gap: 6px;
  }
  .tag-unlocked,
  .tag-locked,
  .tag-secret {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 800;
  }
  .tag-unlocked {
    background: #d1fae5;
    color: #065f46;
  }
  .tag-locked {
    background: #f1f5f9;
    color: #475569;
  }
  .tag-secret {
    background: #fef3c7;
    color: #92400e;
  }
  .detail-tags :global(svg) {
    width: 12px;
    height: 12px;
  }

  .detail-desc-box {
    background: #f8fafc;
    border-radius: 14px;
    padding: 14px;
    border: 1px solid rgba(0, 0, 0, 0.05);
  }
  .detail-desc-box p {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.5;
    color: #475569;
    font-weight: 600;
  }
  .btn-reveal {
    margin-top: 10px;
    height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    border: 0;
    background: #00A0D8;
    color: #fff;
    font: 800 12px 'M PLUS Rounded 1c', sans-serif;
    cursor: pointer;
  }

  /* Evidencia / Captura automática */
  .detail-evidence-box {
    border-radius: 14px;
    overflow: hidden;
    background: #f8fafc;
    border: 1px solid rgba(0, 0, 0, 0.05);
    padding: 12px;
  }
  .evidence-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  .evidence-title b {
    font-size: 13px;
    font-weight: 800;
    color: #334155;
  }
  .evidence-title small {
    font-size: 11px;
    font-weight: 700;
    color: #64748b;
  }
  .evidence-preview {
    height: 120px;
    border-radius: 10px;
    overflow: hidden;
    position: relative;
    background: #0f172a;
    display: grid;
    place-items: center;
  }
  .shot-mock {
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, #1e3a8a, #0f172a);
    display: flex;
    align-items: flex-end;
    padding: 10px;
  }
  .shot-hud {
    z-index: 2;
  }
  .hud-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(15, 23, 42, 0.75);
    color: #fbbf24;
    font-size: 11px;
    font-weight: 800;
    backdrop-filter: blur(6px);
  }
  .hud-chip :global(svg) {
    width: 13px;
    height: 13px;
  }
  .shot-placeholder {
    color: #64748b;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 700;
  }
  .shot-placeholder :global(svg) {
    width: 24px;
    height: 24px;
    opacity: 0.6;
  }

  .detail-meta-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }
  .meta-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
    border-bottom: 1px dashed rgba(0, 0, 0, 0.08);
  }
  .meta-row span {
    color: #64748b;
    font-weight: 600;
  }
  .meta-row b {
    color: #1e293b;
    font-weight: 800;
  }
  .meta-row code {
    background: #f1f5f9;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 11px;
    color: #0f172a;
  }

  .empty-state,
  .empty-detail {
    display: grid;
    place-items: center;
    height: 100%;
    color: #8b95a5;
    font-size: 14px;
    font-weight: 700;
    text-align: center;
    padding: 20px;
  }

  .modal-footer {
    margin-top: 14px;
    flex: none;
    position: relative;
    height: 36px;
  }
  .modal-footer :global(.hints) {
    position: static;
  }
</style>
