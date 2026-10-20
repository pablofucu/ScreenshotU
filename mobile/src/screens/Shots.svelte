<script lang="ts">
  // Todas las capturas de un juego (o de una saga, con filtro por juego), por días o en cuadrícula continua.
  import { nCapturas } from '@shared/format';
  import type { Shot } from '@shared/types';
  import MiniCover from '../components/MiniCover.svelte';
  import ShotGrid from '../components/ShotGrid.svelte';
  import { app } from '../lib/store.svelte';

  let { id }: { id: string } = $props();
  const ref = $derived(app.index.get(id));
  const T = $derived(ref?.t);
  const acc = $derived(app.accById(id));
  let member = $state(-1);

  const shots = $derived.by((): Shot[] => {
    if (!T) return [];
    let list = T.shots;
    if (T.members && member >= 0) list = list.filter((s) => s.gid === T.members![member].id);
    if (app.shotsSort === 'oldest') return [...list].reverse();
    if (app.shotsSort === 'favs') return [...list.filter((s) => app.favs.has(s.path)), ...list.filter((s) => !app.favs.has(s.path))];
    return list;
  });
  const byDay = $derived(app.prefs.shotsView === 'days' && app.shotsSort !== 'favs');
  const SORTS = [['recent', 'Recientes'], ['oldest', 'Antiguas'], ['favs', 'Favoritas primero']] as const;
</script>

{#if T}
  <div style="--acc:{acc.c};--acc-ink:{acc.dark ? '#3a2f00' : '#fff'}">
    <div class="ghead">
      <MiniCover t={T} acc={acc.c} size={42} />
      <div><h2>{T.name}</h2><span>{nCapturas(shots.length)}</span></div>
      <div class="seg pill mini">
        <button class:on={app.prefs.shotsView === 'days'} onclick={() => app.setPrefs({ shotsView: 'days' })}>Días</button>
        <button class:on={app.prefs.shotsView === 'grid'} onclick={() => app.setPrefs({ shotsView: 'grid' })}>Todo</button>
      </div>
    </div>
    <div class="chips">
      {#each SORTS as [k, n] (k)}<button class="chip pill" class:on={app.shotsSort === k} onclick={() => (app.shotsSort = k)}>{n}</button>{/each}
    </div>
    {#if T.members}
      <div class="chips">
        <button class="chip pill" class:on={member < 0} onclick={() => (member = -1)}>Toda la saga</button>
        {#each T.members as m, i (m.id)}<button class="chip pill" class:on={member === i} onclick={() => (member = i)}>{m.name}</button>{/each}
      </div>
    {/if}
    <ShotGrid {shots} days={byDay} showGame={!!T.members && member < 0} onopen={(i) => app.openViewer(shots, i, T.name)} />
  </div>
{/if}

<style>
  .ghead{display:flex;align-items:center;gap:12px;padding:6px 16px 10px}
  .ghead>div:nth-child(2){flex:1;min-width:0}
  h2{margin:0;font-size:19px;font-weight:800;color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .ghead span{font-size:12.5px;font-weight:700;color:var(--dim)}
  .mini button{height:28px;padding:0 10px;font-size:12.5px}
</style>
