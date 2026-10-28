<script lang="ts">
  import { app, LIST_COLOR } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { inkFor } from '../lib/ops.svelte';
  import Shelf, { type ShelfItem } from '../components/Shelf.svelte';
  import { computeMonthPositions, finShelfSub, LIST_VIS, SHELF_STEP } from '../lib/milista.svelte';
  import { openTitle } from '../lib/controller.svelte';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const shelves = $derived(app.finShelves);

  let focusShelf = $state(0);
  let focusItem = $state(0);

  const coverOf = (ti: number) => titles[ti]?.cover ?? titles[ti]?.shots[0]?.path ?? null;

  const finShelfItems = (items: number[]): ShelfItem[] =>
    items.map((ti, k) => ({
      k: 'fin' + ti,
      act: 'tit:' + ti,
      path: coverOf(ti),
      name: titles[ti]?.name ?? '',
      n: k + 1,
      stars: titles[ti]?.rating ?? 0,
      label: titles[ti]?.finishedMs ? new Date(titles[ti].finishedMs!).toLocaleDateString('es', { month: 'short' }) : '',
    }));

  const monthCounts = (items: number[]) => {
    const c = Array<number>(12).fill(0);
    for (const ti of items) {
      if (titles[ti]?.finishedMs) c[new Date(titles[ti].finishedMs!).getMonth()]++;
    }
    return c;
  };
</script>

<div class="finstats">
  {#if !shelves.length}
    <div class="glass none-card" style="--acc:{app.chrome.c}">
      <div class="face">
        <h2>Aún no hay juegos terminados</h2>
        <p>Cuando termines un juego, márcalo como <b>Terminado</b> desde su ficha (Y › Estado) para registrar tu fecha y valoración de estrellas.</p>
      </div>
    </div>
  {:else}
    <div class="shelves-scroll">
      {#each shelves as sh, k (sh.key)}
        {@const isFoc = k === focusShelf}
        {@const pos = sh.year > 0 ? computeMonthPositions(sh.items, titles, isFoc ? focusItem : -1) : null}
        {@const mc = monthCounts(sh.items)}
        <div class="fin-shelf-wrap">
          <Shelf
            acc={LIST_COLOR.finished.c}
            ink={inkFor(LIST_COLOR.finished.c)}
            title={sh.label}
            icon={ICON.check}
            sub={`${sh.items.length} ${sh.items.length === 1 ? 'juego completado' : 'juegos completados'}`}
            chip={String(sh.items.length)}
            items={finShelfItems(sh.items)}
            positions={pos?.xs}
            dividers={pos?.dividers}
            monthGroups={pos?.groups}
            focus={isFoc ? focusItem : -1}
            from={0}
            visible={LIST_VIS}
            top={0}
            dim={!isFoc}
          >
            {#snippet extra()}
              {#if sh.year > 0}
                <span class="mbars" title="Terminados por mes">
                  {#each mc as c, m}
                    <i class:z={!c} style="height:{c ? 6 + Math.round((c / Math.max(...mc, 1)) * 16) : 3}px"></i>
                  {/each}
                </span>
              {/if}
            {/snippet}
          </Shelf>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .finstats {
    position: absolute;
    left: 0;
    width: 1280px;
    top: 130px;
    bottom: 60px;
    overflow-y: auto;
  }
  .shelves-scroll {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-bottom: 30px;
  }
  .fin-shelf-wrap {
    position: relative;
    height: 240px;
  }
  .none-card {
    position: absolute;
    left: 50%;
    top: 40%;
    transform: translate(-50%, -50%);
    width: 600px;
  }
  .none-card .face {
    padding: 32px;
    text-align: center;
  }
  .none-card h2 {
    margin: 0 0 10px;
    font-size: 24px;
    font-weight: 800;
  }
  .none-card p {
    margin: 0;
    font-size: 16px;
    color: #64748b;
    line-height: 1.5;
  }
  .mbars {
    display: inline-flex;
    align-items: flex-end;
    gap: 3px;
    height: 22px;
    margin-left: 12px;
    vertical-align: middle;
  }
  .mbars i {
    width: 4px;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.85);
  }
  .mbars i.z {
    background: rgba(255, 255, 255, 0.25);
  }
</style>
