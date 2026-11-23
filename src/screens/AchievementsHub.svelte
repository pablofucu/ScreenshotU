<script lang="ts">
  // Vitrina global de logros: resumen de toda la biblioteca desde SQLite, agrupación estricta por categoría,
  // últimos desbloqueos y logros más raros. A abre la vitrina del juego; X abre su ficha.
  import { app } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { catIcon } from '../lib/chiptone';
  import { orderedPlatforms, sectionColor } from '../lib/sections';
  import { formatUnlockDate } from '../lib/achievements.svelte';
  import type { GameAchOverview, GlobalAchItem, Title } from '../lib/types';
  import TopBar from '../components/TopBar.svelte';
  import ChipPane from '../components/ChipPane.svelte';
  import Thumb from '../components/Thumb.svelte';
  import CIcon from '../components/CIcon.svelte';
  import Hints from '../components/Hints.svelte';

  type HubTab = 'games' | 'recent' | 'rarest';
  const TABS: Array<[HubTab, string]> = [
    ['games', 'Juegos'],
    ['recent', 'Recientes'],
    ['rarest', 'Más raros'],
  ];

  const TIER_NAME = ['Legendario', 'Ultra raro', 'Raro', 'Poco común', 'Común', 'Sin rareza'];
  const tierOf = (p: number | null | undefined) =>
    typeof p !== 'number' ? 5 : p < 5 ? 0 : p < 12 ? 1 : p < 25 ? 2 : p < 50 ? 3 : 4;
  const fmtPct = (p: number) => p.toFixed(1).replace('.', ',') + ' %';

  type ResolvedRef = { pi: number; ti: number; platId: string; platName: string; title: Title };
  const byGame = $derived.by(() => {
    const m = new Map<string, ResolvedRef>();
    app.lib.platforms.forEach((p, pi) => {
      p.titles.forEach((title, ti) => {
        if (title.hidden) return;
        const ref: ResolvedRef = { pi, ti, platId: p.id, platName: p.name, title };
        if (title.gameId && !m.has(title.gameId)) m.set(title.gameId, ref);
        if (!m.has(title.id)) m.set(title.id, ref);
      });
    });
    return m;
  });

  const ov = $derived(app.achOverview);

  type GameCard = { idx: number; ov: GameAchOverview; ref: ResolvedRef; pct: number; complete: boolean };
  type CatGroup = { pi: number; platId: string; platName: string; color: string; unlocked: number; cards: GameCard[] };

  const catGroups = $derived.by((): CatGroup[] => {
    if (!ov?.games.length) return [];
    const buckets = new Map<number, Array<{ ov: GameAchOverview; ref: ResolvedRef }>>();
    for (const g of ov.games) {
      const ref = byGame.get(g.gameId);
      if (!ref) continue;
      let arr = buckets.get(ref.pi);
      if (!arr) { arr = []; buckets.set(ref.pi, arr); }
      arr.push({ ov: g, ref });
    }
    const groups: CatGroup[] = [];
    let idx = 0;
    for (const { p, i: pi } of orderedPlatforms(app.lib.platforms)) {
      const raw = buckets.get(pi);
      if (!raw?.length) continue;
      const cards: GameCard[] = raw.map(({ ov: g, ref }) => ({
        idx: idx++,
        ov: g,
        ref,
        pct: g.total > 0 ? Math.round((g.unlocked / g.total) * 100) : 0,
        complete: g.total > 0 && g.unlocked >= g.total,
      }));
      groups.push({
        pi,
        platId: p.id,
        platName: p.name,
        color: sectionColor(p.id),
        unlocked: cards.reduce((s, c) => s + c.ov.unlocked, 0),
        cards,
      });
    }
    return groups;
  });

  type AchEntry = { idx: number; item: GlobalAchItem; ref: ResolvedRef; color: string };
  function resolveItems(list: GlobalAchItem[] | undefined): AchEntry[] {
    if (!list?.length) return [];
    const out: AchEntry[] = [];
    for (const item of list) {
      const ref = byGame.get(item.gameId);
      if (!ref) continue;
      out.push({ idx: out.length, item, ref, color: sectionColor(ref.platId) });
    }
    return out;
  }

  const recentItems = $derived(resolveItems(ov?.recent));
  const rarestItems = $derived(resolveItems(ov?.rarest));

  const visibleCount = $derived(
    app.achHubTab === 'games'
      ? catGroups.reduce((s, g) => s + g.cards.length, 0)
      : app.achHubTab === 'recent'
        ? recentItems.length
        : rarestItems.length,
  );
  const pctTotal = $derived(ov && ov.totalAchievements > 0 ? Math.round((ov.totalUnlocked / ov.totalAchievements) * 100) : 0);
  const gamesCount = $derived(ov?.games.length ?? 0);
  const activeFocused = $derived(app.focus.startsWith('ah'));
</script>

<div class="ahub">
  <TopBar crumbs={['Inicio', 'Logros']} />

  <div class="ah-bar sx">
    <span class="cat">Vitrina global</span>
    <span class="gl vsw">
      {#each TABS as [k, label]}
        <span class="o" class:gem={app.achHubTab === k} data-hact="ahtab:{k}">{label}</span>
      {/each}
    </span>
  </div>

  <div class="stsum ah-sum">
    <div class="rbox">
      <div class="k">Desbloqueados</div>
      <div class="v ah-v-acc">{(ov?.totalUnlocked ?? 0).toLocaleString('es-ES')}<small>&nbsp;de {(ov?.totalAchievements ?? 0).toLocaleString('es-ES')} · {pctTotal} %</small></div>
    </div>
    <div class="rbox">
      <div class="k">Al 100 %</div>
      <div class="v">{ov?.completedGames ?? 0}<small>&nbsp;{ov?.completedGames === 1 ? 'juego' : 'juegos'}</small></div>
    </div>
    <div class="rbox">
      <div class="k">Muy raros</div>
      <div class="v">{ov?.rareUnlocked ?? 0}<small>&nbsp;menos del 12 %</small></div>
    </div>
    <div class="rbox">
      <div class="k">Juegos con logros</div>
      <div class="v">{gamesCount}</div>
    </div>
  </div>

  {#if app.achOverviewLoading && !ov}
    <div class="glass note-card ah-empty">
      <div class="face">
        <div class="empty">
          {@html ICON.trophy}
          <h3>Cargando logros…</h3>
          <p>Leyendo el progreso guardado de tu biblioteca.</p>
        </div>
      </div>
    </div>
  {:else if !ov || (gamesCount === 0 && !recentItems.length)}
    <div class="glass note-card ah-empty">
      <div class="face">
        <div class="empty">
          {@html ICON.trophy}
          <h3>Aún no hay logros registrados</h3>
          <p>Cuando juegues desde el visor a títulos con esquema de logros, aparecerán aquí organizados por categoría.</p>
        </div>
      </div>
    </div>
  {:else}
    <ChipPane>
      {#if app.achHubTab === 'games'}
        {#if !catGroups.length}
          <div class="ah-none">Los logros registrados no corresponden a ningún título visible de la biblioteca actual.</div>
        {:else}
          {#each catGroups as grp (grp.platId)}
            <h3 class="sh">
              <CIcon name={catIcon(grp.platId)} />
              <i class="ah-dot" style="background:{grp.color}"></i>
              {grp.platName}
              <span>{grp.cards.length} {grp.cards.length === 1 ? 'juego' : 'juegos'} · {grp.unlocked.toLocaleString('es-ES')} logros</span>
            </h3>
            <div class="cgrid ah-ggrid">
              {#each grp.cards as c (c.ref.title.id)}
                {@const art = c.ref.title.cover ?? c.ref.title.shots[0]?.path}
                {@const acc = app.titleAcc(c.ref.pi, c.ref.ti).c}
                <div
                  class="glass ah-game fx"
                  class:f={app.focus === 'ah' + c.idx}
                  class:done={c.complete}
                  data-k="ah{c.idx}"
                  data-act="ahgame:{c.ref.pi}:{c.ref.ti}"
                  style="--cat:{grp.color};--acc:{acc}"
                >
                  <div class="face ah-gface">
                    <div class="ah-cov">
                      {#if art}<Thumb path={art} />{/if}
                    </div>
                    <div class="ah-gi">
                      <div class="ah-gtop">
                        <b>{c.ref.title.name}</b>
                        {#if c.complete}
                          <span class="ah-plat">100 %</span>
                        {:else if c.ov.rareUnlocked > 0}
                          <span class="ah-rare">{c.ov.rareUnlocked} {c.ov.rareUnlocked === 1 ? 'raro' : 'raros'}</span>
                        {/if}
                      </div>
                      <div class="ah-pbar"><i style="width:{c.pct}%"></i></div>
                      <div class="ah-gm">
                        <span>{c.ov.unlocked} de {c.ov.total}</span>
                        <b>{c.pct} %</b>
                      </div>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          {/each}
        {/if}
      {:else}
        {@const items = app.achHubTab === 'recent' ? recentItems : rarestItems}
        {#if !items.length}
          <div class="ah-none">
            {app.achHubTab === 'rarest'
              ? 'Todavía no hay datos de rareza global. Abre la ficha de logros de un juego con conexión para consultarla.'
              : 'Todavía no hay logros desbloqueados.'}
          </div>
        {:else}
          <h3 class="sh">
            {@html ICON.trophy}
            {app.achHubTab === 'recent' ? 'Últimos logros desbloqueados' : 'Logros más raros conseguidos'}
            <span>{items.length}</span>
          </h3>
          <div class="cgrid ah-ugrid">
            {#each items as entry (`${app.achHubTab}:${entry.idx}:${entry.item.appId}:${entry.item.apiName}`)}
              {@const tier = tierOf(entry.item.pct)}
              <div
                class="glass ah-ach fx"
                class:f={app.focus === 'ah' + entry.idx}
                data-k="ah{entry.idx}"
                data-act="ahgame:{entry.ref.pi}:{entry.ref.ti}"
                style="--cat:{entry.color}"
              >
                <div class="face ah-aface">
                  <div class="ah-ico">
                    {#if entry.item.icon}
                      <img src={entry.item.icon} alt="" draggable="false" />
                    {:else}
                      {@html ICON.trophy}
                    {/if}
                  </div>
                  <div class="ah-ai">
                    <div class="ah-atitle">
                      <b>{entry.item.displayName}</b>
                      {#if typeof entry.item.pct === 'number'}
                        <span class="ah-tier" style="--tc:var(--rar-{tier})">{fmtPct(entry.item.pct)}</span>
                      {/if}
                    </div>
                    {#if entry.item.description}
                      <div class="ah-adesc">{entry.item.description}</div>
                    {/if}
                    <div class="ah-ameta">
                      <span class="ah-pill"><i style="background:{entry.color}"></i>{entry.ref.platName}</span>
                      <span class="ah-gname">{entry.ref.title.name}</span>
                      <span class="ah-when">{tier < 5 && app.achHubTab === 'rarest' ? TIER_NAME[tier] : formatUnlockDate(entry.item.unlockedAt)}</span>
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </ChipPane>
  {/if}

  <Hints list={[
    ...(visibleCount && activeFocused ? [['A', 'Ver logros', 'a'] as [string, string, string], ['X', 'Ficha del juego', 'x'] as [string, string, string]] : []),
    ['LB/RB', 'Pestaña', 'lb'],
    ['B', 'Volver', 'b'],
  ]} />
</div>

<style>
  .ahub {
    position: absolute;
    inset: 0;
  }
  .ah-bar {
    position: absolute;
    left: 48px;
    right: 48px;
    top: 86px;
    height: 44px;
    display: flex;
    align-items: center;
    gap: 14px;
    z-index: 4;
  }
  .ah-bar .vsw {
    margin-left: auto;
    gap: 2px;
    padding: 4px;
  }
  .ah-bar .vsw .o {
    display: grid;
    place-items: center;
    padding: 0 14px;
    height: 36px;
    border-radius: var(--r-pill);
    cursor: pointer;
    white-space: nowrap;
    font-size: var(--t-m);
    font-weight: 800;
  }
  .ah-sum {
    top: 140px;
  }
  .ah-v-acc {
    color: var(--acc);
  }
  :global(#stage[data-theme='dark']) .ah-v-acc {
    color: color-mix(in srgb, var(--v1) 76%, white);
  }
  .ahub :global(.cvp) {
    top: 238px;
    bottom: calc(var(--stage-h) - var(--content-end));
  }
  .ahub :global(.cbar) {
    top: 252px;
    bottom: calc(var(--stage-h) - var(--content-end) + 14px);
  }
  .ahub :global(.cmore) {
    bottom: calc(var(--stage-h) - var(--content-end) + 10px);
  }
  .ahub :global(.cpane) {
    padding-top: 8px;
  }
  .ahub :global(.sh) {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 10px;
    font-size: var(--t-l);
    font-weight: 800;
    color: var(--ink);
  }
  .ahub :global(.sh:first-child) {
    margin-top: 0;
  }
  .ahub :global(.sh > svg) {
    width: 18px;
    height: 18px;
    color: var(--accv);
    flex: none;
  }
  .ahub :global(.sh span) {
    margin-left: auto;
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
  }
  .ah-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
    box-shadow: 0 0 0 2px var(--hl-ring);
  }
  .ah-ggrid {
    grid-template-columns: repeat(3, minmax(0, 1fr)) !important;
    gap: 14px !important;
  }
  .ah-ugrid {
    grid-template-columns: repeat(2, minmax(0, 1fr)) !important;
    gap: 12px !important;
  }
  .ah-game {
    --r: var(--r-m);
    --bw: 5px;
    height: 108px;
  }
  .ah-gface {
    background: var(--face-hi);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px 8px 8px;
    box-sizing: border-box;
  }
  .ah-cov {
    width: 64px;
    height: 86px;
    border-radius: var(--r-s);
    overflow: hidden;
    flex: none;
    background: linear-gradient(155deg, color-mix(in srgb, var(--acc, var(--cat)) 45%, var(--tint-base)), var(--cat));
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .ah-cov :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .ah-gi {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .ah-gtop {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ah-gtop b {
    flex: 1;
    min-width: 0;
    font-size: var(--t-m);
    font-weight: 800;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ah-plat,
  .ah-rare {
    flex: none;
    padding: 2px 8px;
    border-radius: var(--r-pill);
    font-size: var(--t-xs);
    font-weight: 800;
    color: var(--on);
  }
  .ah-plat {
    background: var(--arc-azul);
  }
  .ah-rare {
    background: var(--rar-0);
  }
  .ah-pbar {
    position: relative;
    height: 9px;
    border-radius: var(--r-pill);
    background: color-mix(in srgb, var(--ink-3) 20%, transparent);
    overflow: hidden;
  }
  .ah-pbar i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, color-mix(in srgb, var(--cat) 75%, white), var(--cat));
  }
  .ah-game.done .ah-pbar i {
    background: linear-gradient(90deg, var(--arc-azul), var(--arc-violeta));
  }
  .ah-gm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--t-s);
    font-weight: 700;
    color: var(--ink-2);
  }
  .ah-gm b {
    color: var(--ink);
    font-weight: 800;
  }

  .ah-ach {
    --r: var(--r-m);
    --bw: 5px;
    height: 96px;
  }
  .ah-aface {
    background: var(--face-hi);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px 8px 10px;
    box-sizing: border-box;
  }
  .ah-ico {
    width: 64px;
    height: 64px;
    border-radius: var(--r-s);
    overflow: hidden;
    flex: none;
    display: grid;
    place-items: center;
    background: linear-gradient(150deg, color-mix(in srgb, var(--cat) 26%, var(--tint-base)), color-mix(in srgb, var(--cat) 62%, black));
    color: var(--on);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, .25);
  }
  .ah-ico img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .ah-ico :global(svg) {
    width: 30px;
    height: 30px;
  }
  .ah-ai {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .ah-atitle {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ah-atitle b {
    flex: 1;
    min-width: 0;
    font-size: var(--t-m);
    font-weight: 800;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ah-tier {
    flex: none;
    padding: 2px 8px;
    border-radius: var(--r-pill);
    font-size: var(--t-xs);
    font-weight: 800;
    color: var(--on);
    background: var(--tc);
  }
  .ah-adesc {
    font-size: var(--t-s);
    font-weight: 500;
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ah-ameta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
    font-size: var(--t-xs);
    font-weight: 700;
    color: var(--ink-3);
  }
  .ah-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: var(--r-pill);
    background: var(--glass-in);
    color: var(--ink-2);
    flex: none;
  }
  .ah-pill i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .ah-gname {
    flex: 1;
    min-width: 0;
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ah-when {
    flex: none;
    color: var(--ink-3);
  }

  .ah-empty {
    left: 300px;
    top: 274px;
  }
  .ah-none {
    padding: 40px 24px;
    text-align: center;
    font-size: var(--t-l);
    font-weight: 600;
    color: var(--ink-2);
  }
</style>
