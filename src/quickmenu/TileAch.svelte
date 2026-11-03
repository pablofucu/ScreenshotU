<script lang="ts">
  // Baldosa compacta y hoja completa de logros de Steam / emuladores.
  import type { Achievement } from '../lib/types';
  import { fmtShort } from '../lib/format';

  let {
    achievements = null,
    loading = false,
    view = 'tile',
    onOpenSheet,
  }: {
    achievements: Achievement[] | null;
    loading?: boolean;
    view?: 'tile' | 'sheet';
    onOpenSheet?: () => void;
  } = $props();

  let activeTab = $state<'recent' | 'locked' | 'all'>('recent');

  const total = $derived(achievements?.length ?? 0);
  const unlocked = $derived(
    achievements ? achievements.filter((a) => a.earned).sort((a, b) => (b.unlockedAt ?? 0) - (a.unlockedAt ?? 0)) : []
  );
  const locked = $derived(achievements ? achievements.filter((a) => !a.earned) : []);
  const pct = $derived(total > 0 ? Math.round((unlocked.length / total) * 100) : 0);
  const lastUnlocked = $derived(unlocked[0] ?? null);
  const nextTarget = $derived(locked[0] ?? null);

  const displayedList = $derived(
    activeTab === 'recent' ? unlocked : activeTab === 'locked' ? locked : (achievements ?? [])
  );

  function relTime(at: number | null | undefined): string {
    if (!at) return '';
    const diff = Math.max(0, Date.now() - at);
    if (diff < 60000) return 'hace un momento';
    if (diff < 3600000) return `hace ${Math.floor(diff / 60000)} min`;
    if (diff < 86400000) return `hace ${Math.floor(diff / 3600000)} h`;
    if (diff < 172800000) return 'ayer';
    return fmtShort(at);
  }

  export function cycleTab(dir: number) {
    const tabs: Array<'recent' | 'locked' | 'all'> = ['recent', 'locked', 'all'];
    const idx = tabs.indexOf(activeTab);
    activeTab = tabs[(idx + dir + tabs.length) % tabs.length];
  }
</script>

{#if view === 'tile'}
  {#if loading}
    <div class="glass tile t-ach dim" style="--i:4">
      <div class="lbl">
        <svg class="i" viewBox="0 0 24 24"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M10 17h4"/></svg>
        Logros
      </div>
      <div class="ach-empty">Cargando…</div>
    </div>
  {:else if !achievements || achievements.length === 0}
    <div class="glass tile t-ach dim" style="--i:4">
      <div class="lbl">
        <svg class="i" viewBox="0 0 24 24"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M10 17h4"/></svg>
        Logros
      </div>
      <div class="ach-empty">Este juego no tiene logros</div>
    </div>
  {:else}
    <div
      class="glass tile t-ach"
      style="--i:4"
      data-f
      data-id="ach"
      onclick={onOpenSheet}
      onkeydown={(e) => { if (e.key === 'Enter') onOpenSheet?.(); }}
      role="button"
      tabindex="0"
    >
      <div class="lbl">
        <svg class="i" viewBox="0 0 24 24"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M10 17h4"/></svg>
        Logros
        <em>{pct}%</em>
      </div>
      <div class="achc">
        <div class="ring" style="--p:{pct};--s:62px;--t:8px">
          <b style="font-size:17px">{unlocked.length}<small style="display:block;font-size:10px;opacity:.75">de {total}</small></b>
        </div>
        <div class="at">
          {#if lastUnlocked}
            <b>{lastUnlocked.displayName}</b>
            <small>{relTime(lastUnlocked.unlockedAt)}</small>
          {/if}
          {#if nextTarget}
            <small>Cerca: {nextTarget.displayName}</small>
          {:else}
            <small>¡Todos conseguidos!</small>
          {/if}
        </div>
      </div>
    </div>
  {/if}
{:else}
  {#if !achievements || achievements.length === 0}
    <div class="ach-empty" style="font-size:18px;text-align:center;padding:40px">Este juego no tiene logros</div>
  {:else}
    <div class="vhead">
      <div class="ring" style="--p:{pct};--s:96px;--t:10px">
        <b style="font-size:30px">{unlocked.length}<small style="display:block;font-size:13px;opacity:.7">de {total}</small></b>
      </div>
      <div>
        <div class="big2">{pct} % completado</div>
        <div class="sub">
          {unlocked.length} de {total} conseguidos
          {#if nextTarget}
            · Próximo: {nextTarget.displayName}
          {/if}
        </div>
        <div class="tabs">
          <i class="k">LB</i>
          <span
            class="tab"
            class:on={activeTab === 'recent'}
            data-f
            data-id="tab-recent"
            onclick={() => { activeTab = 'recent'; }}
            onkeydown={(e) => { if (e.key === 'Enter') activeTab = 'recent'; }}
            role="button"
            tabindex="0"
          >
            Conseguidos ({unlocked.length})
          </span>
          <span
            class="tab"
            class:on={activeTab === 'locked'}
            data-f
            data-id="tab-locked"
            onclick={() => { activeTab = 'locked'; }}
            onkeydown={(e) => { if (e.key === 'Enter') activeTab = 'locked'; }}
            role="button"
            tabindex="0"
          >
            Por conseguir ({locked.length})
          </span>
          <span
            class="tab"
            class:on={activeTab === 'all'}
            data-f
            data-id="tab-all"
            onclick={() => { activeTab = 'all'; }}
            onkeydown={(e) => { if (e.key === 'Enter') activeTab = 'all'; }}
            role="button"
            tabindex="0"
          >
            Todos ({total})
          </span>
          <i class="k">RB</i>
        </div>
      </div>
    </div>

    <div class="alist">
      {#each displayedList as a, i}
        <div class="arow" data-f data-id="ach-{a.apiName}">
          <div
            class="ai"
            class:lk={!a.earned}
            style="--g:linear-gradient(135deg, var(--r{((i % 6) + 1)}), var(--r{(((i + 2) % 6) + 1)}))"
          >
            {#if a.earned}
              <svg viewBox="0 0 24 24"><path d="M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z"/></svg>
            {:else}
              <svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>
            {/if}
          </div>
          <div class="at">
            <b>{a.displayName}</b>
            <small>{a.description || 'Logro oculto'}</small>
          </div>
          <div class="am">
            {#if a.earned}
              <span>{relTime(a.unlockedAt)}</span>
            {:else}
              <span style="opacity:0.6">Bloqueado</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
{/if}
