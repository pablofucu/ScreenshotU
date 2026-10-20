<script lang="ts">
  // Juegos: las categorías arriba (como los iconos de Inicio), filtros, búsqueda y la cuadrícula de portadas.
  import { fmtAgo, fmtDur, nCapturas, nTitulos } from '@shared/format';
  import { ICON } from '@shared/icons';
  import Cover from '../components/Cover.svelte';
  import { app, sortByName } from '../lib/store.svelte';

  const FILTERS = [['all', 'Todos'], ['played', 'Jugados'], ['shots', 'Con capturas'], ['favs', 'Con favoritas'], ['az', 'A–Z']] as const;
  const P = $derived(app.lib.platforms[app.gamesPlat]);
  const isSaga = $derived(P?.id === 'sagas');
  const list = $derived.by(() => {
    if (!P) return [];
    let all = P.titles.map((t, ti) => ({ t, ti }));
    const q = app.gamesQuery.trim().toLowerCase();
    if (q) all = all.filter(({ t }) => t.name.toLowerCase().includes(q));
    if (isSaga) return all;
    switch (app.gamesFilter) {
      case 'played': return all.filter(({ t }) => (t.plays?.length ?? 0) > 0 || t.playSeconds > 0);
      case 'shots': return all.filter(({ t }) => t.shots.length > 0);
      case 'favs': return all.filter(({ t }) => t.shots.some((s) => app.favs.has(s.path)));
      case 'az': return [...all].sort((a, b) => sortByName(a.t.name, b.t.name));
      default: return all;
    }
  });
  const shots = $derived(P?.titles.reduce((a, t) => a + t.shots.length, 0) ?? 0);
  const hours = $derived(P?.titles.reduce((a, t) => a + t.playSeconds, 0) ?? 0);
  const sub = (t: (typeof list)[number]['t']) => (t.lastPlayed ? fmtAgo(t.lastPlayed) : nCapturas(t.shots.length));
</script>

<div class="plats">
  {#each app.lib.platforms as p, pi (p.id)}
    {@const a = app.platAcc(pi)}
    <button class="glass tap pchip" class:on={pi === app.gamesPlat} style="--pc:{a.c};--acc:{a.c}" onclick={() => { app.gamesPlat = pi; app.gamesQuery = ''; }}>
      <span class="face"><i>{@html ICON[p.icon] ?? ICON.pc}</i>{p.name}</span>
    </button>
  {/each}
</div>

{#if P}
  <div class="thead">
    <h2>{P.name}</h2>
    <span>{isSaga ? `${P.titles.length} sagas` : nTitulos(P.titles.length)} · {nCapturas(shots)}{hours > 3600 ? ` · ${fmtDur(hours)}` : ''}</span>
  </div>
  <div class="tools">
    <label class="search pill"><i>{@html ICON.search}</i><input placeholder="Buscar en {P.name}" bind:value={app.gamesQuery} enterkeyhint="search" /></label>
  </div>
  {#if !isSaga}
    <div class="chips">
      {#each FILTERS as [k, n] (k)}<button class="chip pill" class:on={app.gamesFilter === k} onclick={() => (app.gamesFilter = k)}>{n}</button>{/each}
    </div>
  {/if}
  <div class="grid">
    {#each list as { t, ti } (t.id)}
      {@const a = app.titleAcc(app.gamesPlat, ti)}
      <button class="tile" onclick={() => app.openTitle(t.id)}>
        <span class="glass tap cov" style="--acc:{a.c}"><Cover {t} acc={a.c} />
          {#if t.gameId && app.playing?.id === t.gameId}<b class="live">Jugando</b>{/if}
        </span>
        <span class="tn">{t.name}</span>
        <span class="ts">{sub(t)}</span>
      </button>
    {:else}
      <p class="empty">{app.gamesQuery ? 'Ningún juego coincide con la búsqueda.' : 'No hay juegos en esta categoría con este filtro.'}</p>
    {/each}
  </div>
{/if}

<style>
  .plats{display:flex;gap:8px;overflow-x:auto;padding:6px 14px 4px;scrollbar-width:none}
  .plats::-webkit-scrollbar{display:none}
  .pchip{flex:none;--r:18px;--bw:5px}
  .pchip .face{display:flex;align-items:center;gap:8px;height:38px;padding:0 14px 0 6px;font-size:14px;font-weight:800;color:var(--ink2);background:rgba(247,249,252,.92)}
  .pchip i{width:28px;height:28px;border-radius:9px;display:grid;place-items:center;background:radial-gradient(120% 100% at 30% 10%,color-mix(in srgb,var(--pc) 22%,white) 0%,var(--pc) 70%)}
  .pchip i :global(svg){width:20px;height:20px}
  .pchip.on .face{color:#fff;background:linear-gradient(180deg,color-mix(in srgb,var(--pc) 70%,white),var(--pc))}
  .thead{display:flex;align-items:baseline;gap:10px;margin:14px 18px 8px;flex-wrap:wrap}
  .thead h2{margin:0;font-size:24px;font-weight:800;color:var(--ink)}
  .thead span{font-size:13px;font-weight:700;color:var(--dim)}
  .tools{padding:0 14px 8px}
  .search{display:flex;align-items:center;gap:8px;height:42px;padding:0 14px;border-radius:999px}
  .search i{width:18px;height:18px;flex:none;opacity:.7;filter:invert(.45)}
  .search i :global(svg){width:18px;height:18px;display:block}
  .search input{flex:1;min-width:0;border:0;outline:0;background:transparent;font-size:15px;font-weight:700;color:var(--ink)}
  .grid{display:grid;grid-template-columns:repeat(3,1fr);gap:14px 10px;padding:8px 14px 0}
  .tile{display:flex;flex-direction:column;text-align:left;min-width:0}
  .cov{--r:18px;--bw:6px;aspect-ratio:2/3;width:100%}
  .cov .face{border-radius:13px}
  .live{position:absolute;z-index:5;left:50%;bottom:10px;transform:translateX(-50%);padding:2px 9px;border-radius:999px;font-size:10.5px;font-weight:800;color:#fff;background:#3BB54A;box-shadow:0 0 10px #3BB54A}
  .tn{margin:6px 3px 0;font-size:13px;font-weight:800;color:var(--ink2);line-height:1.2;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
  .ts{margin:1px 3px 0;font-size:11px;font-weight:700;color:var(--dim2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .empty{grid-column:1/-1;text-align:center;color:var(--dim);font-weight:700;padding:30px 10px}
</style>
