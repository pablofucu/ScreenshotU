<script lang="ts">
  // Pestaña «Colecciones» de una categoría: lista lateral de colecciones y panel de cristal con aurora. Arriba, nombre,
  // años y progreso; en medio, todas las portadas repartidas en filas que siempre caben (cada fila con su raíl de
  // colores y el año solo cuando cambia); abajo, la ficha del juego enfocado con capturas suyas elegidas al azar.
  import { app, LIST_COLOR, STATUS_LABEL } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtDur, nCapturas, nGames } from '../lib/format';
  import { openTitle, setFocus } from '../lib/controller.svelte';
  import { blip } from '../lib/sound';
  import { COLL_GAME_SHOTS, collLayout, collShotSeed, gameVars, openCollShots, setCollPerRow, shelfAccent, shelfVars } from '../lib/coleccion.svelte';
  import { collGrid, collYearSpan, coverColors, gameYear, pickGameShots } from '../lib/collections';
  import Thumb from '../components/Thumb.svelte';
  import CIcon from '../components/CIcon.svelte';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);
  const shelves = $derived(app.collShelves);
  const onNew = $derived(app.collAt.c >= shelves.length);
  const activeIdx = $derived(shelves.length ? Math.min(app.collAt.c, shelves.length - 1) : 0);
  const activeSh = $derived(shelves[activeIdx]);
  const coverOf = (ti: number) => titles[ti]?.cover ?? titles[ti]?.shots[0]?.path ?? null;

  const sideBars = $derived.by(() =>
    shelves.map((sh) => {
      const games = sh.items.map((ti) => titles[ti]).filter(Boolean);
      const acc = shelfAccent(sh);
      const cols = sh.coll.color == null ? coverColors(games, 0.72, 0.16, 6) : [];
      const stops = cols.length ? cols : [acc.v ?? acc.c];
      const barBg = stops.length === 1 ? stops[0] : `linear-gradient(180deg,${stops.join(',')})`;
      return { acc, barBg };
    }),
  );

  const activeGames = $derived(activeSh ? activeSh.items.map((ti) => titles[ti]).filter(Boolean) : []);
  const activeAcc = $derived(activeSh ? shelfAccent(activeSh) : app.chrome);
  const activeVars = $derived(activeSh ? shelfVars(activeSh, activeAcc) : '');
  const activeLayout = $derived(activeSh ? collLayout(activeSh) : null);
  const activeYears = $derived(activeGames.map((t) => gameYear(t)));
  const activeSpan = $derived(collYearSpan(activeYears));
  const activeDone = $derived(activeGames.filter((t) => t.status === 'finished').length);
  const hasAnyStatus = $derived(activeGames.some((t) => Boolean(t.status)));
  const auroraCols = $derived.by(() => {
    const cols = activeSh?.coll.color == null ? coverColors(activeGames, 0.72, 0.16, 5) : [];
    return cols.length ? cols : [activeAcc.v ?? activeAcc.c];
  });

  const focusedPos = $derived(activeSh?.items.length ? Math.max(0, Math.min(app.collAt.i, activeSh.items.length - 1)) : 0);
  const focusedTi = $derived(activeSh ? activeSh.items[focusedPos] : undefined);
  const focusedGame = $derived(focusedTi !== undefined ? titles[focusedTi] : undefined);
  const focusedYear = $derived(focusedGame ? gameYear(focusedGame) : null);

  // Reparto en filas según el hueco real; el mando lo usa para saltar de fila con arriba y abajo.
  let gw = $state(0);
  let gh = $state(0);
  const G = $derived(collGrid(activeSh?.items.length ?? 0, gw, gh));
  $effect(() => setCollPerRow(G.perRow));
  const rows = $derived.by(() => {
    const items = activeSh?.items ?? [];
    const out: Array<Array<{ ti: number; j: number }>> = [];
    for (let s = 0; s < items.length; s += G.perRow) out.push(items.slice(s, s + G.perRow).map((ti, k) => ({ ti, j: s + k })));
    return out;
  });
  const tintOf = (j: number, ti: number) => activeLayout?.tints[j] ?? app.pairFill(app.titlePair(app.p, ti)).v;
  const railOf = (row: Array<{ ti: number; j: number }>) => {
    const cs = row.map((g) => tintOf(g.j, g.ti));
    return cs.length > 1 ? `linear-gradient(90deg,${cs.join(',')})` : cs[0];
  };

  // Capturas del juego enfocado: una tanda al azar nueva cada vez que se llega a él (y otra con View).
  const gameSeed = $derived.by(() => {
    void focusedTi;
    void activeIdx;
    return Math.floor(Math.random() * 1e9) || 1;
  });
  const gameShots = $derived(focusedGame ? pickGameShots(focusedGame.shots, COLL_GAME_SHOTS, (gameSeed ^ collShotSeed()) >>> 0) : []);
  const moreShots = $derived(focusedGame ? Math.max(0, focusedGame.shots.length - gameShots.length) : 0);

  let gamesEl = $state<HTMLElement>();
  let sideEl = $state<HTMLElement>();

  // Mantiene a la vista el juego enfocado (solo hace falta con muchísimos juegos) y la colección activa en la lista lateral.
  $effect(() => {
    void app.collAt;
    void activeIdx;
    queueMicrotask(() => {
      gamesEl?.querySelector<HTMLElement>('.gi.f')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      sideEl?.querySelector<HTMLElement>('.sitem.on')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  });

  const statusColor = (st?: string) => LIST_COLOR[(st as keyof typeof LIST_COLOR) || 'none']?.c;
</script>

<div class="coll-wrap">
  {#if !shelves.length}
    <div class="coll-empty-wrap">
      <div class="glass panel coll-empty-card">
        <div class="face">
          <div class="empty">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3 3 8l9 5 9-5-9-5z" />
              <path d="M3 13l9 5 9-5" />
              <path d="M3 17.5l9 5 9-5" />
            </svg>
            <h3>Aún no tienes colecciones en {P?.name ?? 'esta categoría'}</h3>
            <p>Agrupa tus juegos en una colección para verlos ordenados en su línea del tiempo y reunir sus capturas y colores sin mover ningún archivo.</p>
            <button
              type="button"
              class="b-main fx"
              class:f={onNew}
              data-k="cnew"
              data-act="cnew"
            >
              {@html ICON.plus} Nueva colección
            </button>
          </div>
        </div>
      </div>
    </div>
  {:else if activeSh}
    <div class="coll-grid">
      <div class="glass panel side-panel">
        <div class="face side-face">
          <span class="side-lab">Colecciones en {P?.name ?? ''}</span>
          <div class="side-list" bind:this={sideEl}>
            {#each shelves as sh, k (sh.coll.id)}
              {@const sb = sideBars[k]}
              {@const isOn = k === activeIdx && !onNew}
              <button
                type="button"
                class="sitem"
                class:on={isOn}
                style={shelfVars(sh, sb.acc)}
                onclick={() => { setFocus(`cl${k}.0`, true); blip(880); }}
              >
                <i class="sitem-bar" style="background:{sb.barBg}"></i>
                <div class="sitem-t">
                  <b>{sh.coll.name}</b>
                  <small>{nGames(sh.items.length)}</small>
                </div>
              </button>
            {/each}
          </div>
          <button
            type="button"
            class="sitem add fx"
            class:f={onNew}
            data-k="cnew"
            data-act="cnew"
          >
            {@html ICON.plus} Nueva colección
          </button>
        </div>
      </div>

      <div class="glass panel main-panel" style={activeVars}>
        <div class="face main-face">
          <div class="aurora" aria-hidden="true">
            {#each auroraCols as hex, k}
              <i style="left:{k * 21 - 4}%;--ac:{hex}"></i>
            {/each}
          </div>

          <div class="mhead">
            <h1>{activeSh.coll.name}</h1>
            {#if activeSpan}
              <span class="tag soft"><CIcon name="cal" class="ic s" /> {activeSpan}</span>
            {/if}
            {#if activeSh.items.length}
              <div class="prog">
                {#if hasAnyStatus}
                  <span><b>{activeDone}</b> de {activeSh.items.length} terminados</span>
                  <div class="prog-bar">
                    {#each activeGames as gm}
                      {@const st = gm.status ?? 'none'}
                      <i style={st !== 'none' ? `background:${statusColor(st)}` : ''}></i>
                    {/each}
                  </div>
                {:else}
                  <span>{nGames(activeSh.items.length)}</span>
                {/if}
              </div>
            {/if}
          </div>

          <div
            class="games"
            class:scroll={G.cw <= 56}
            bind:this={gamesEl}
            bind:clientWidth={gw}
            bind:clientHeight={gh}
            style="--cw:{G.cw}px"
          >
            {#if !activeSh.items.length}
              <div
                class="glass scov addtile g-add fx"
                class:f={!onNew}
                data-k="cl{activeIdx}.0"
                data-act="cgame:{activeIdx}:0"
              >
                <div class="face">
                  <div class="addin"><span>{@html ICON.plus}</span><b>Añadir juegos</b></div>
                </div>
              </div>
            {:else}
              {#each rows as row, r (r)}
                <div class="grow" style="--rail:{railOf(row)}">
                  {#each row as g (g.ti)}
                    {@const t = titles[g.ti]}
                    {@const yr = activeYears[g.j]}
                    {@const isF = !onNew && focusedPos === g.j}
                    {@const showYr = yr !== null && (isF || g.j === 0 || activeYears[g.j - 1] !== yr)}
                    <div class="gi" class:f={isF} class:dim={t.status === 'pending'} style={gameVars(g.ti)}>
                      <div
                        class="glass scov gcov fx"
                        class:f={isF}
                        data-k="cl{activeIdx}.{g.j}"
                        data-act="cgame:{activeIdx}:{g.j}"
                        title={t.name}
                      >
                        <div class="face">
                          {#if coverOf(g.ti)}
                            <Thumb path={coverOf(g.ti)!} />
                          {:else}
                            <div class="noart"><b>{t.name}</b></div>
                          {/if}
                        </div>
                      </div>
                      <span class="gdot"></span>
                      {#if showYr}<span class="gyr">{yr}</span>{/if}
                    </div>
                  {/each}
                </div>
              {/each}
            {/if}
          </div>

          {#if focusedGame && focusedTi !== undefined}
            {@const st = focusedGame.status ?? 'none'}
            {@const ti = focusedTi}
            <div class="glass-in det" style={gameVars(ti)}>
              <div class="det-top">
                <b>{focusedGame.name}</b>
                {#if st !== 'none'}
                  <span class="st-badge">
                    <i style="background:{statusColor(st)}"></i>
                    {STATUS_LABEL[st]}
                  </span>
                {/if}
                <span class="det-meta">
                  {[
                    focusedYear ?? null,
                    focusedGame.playSeconds >= 60 ? `${fmtDur(focusedGame.playSeconds)} jugadas` : null,
                    nCapturas(focusedGame.shots.length),
                  ].filter(Boolean).join(' · ')}
                </span>
                <div class="det-acts">
                  {#if focusedGame.shots.length}
                    <button type="button" class="b-sec" onclick={() => openCollShots(activeIdx, undefined, ti)}>
                      {@html ICON.camera} Capturas
                    </button>
                  {/if}
                  <button type="button" class="b-main" onclick={() => openTitle(ti)}>
                    {@html ICON.play} Abrir juego
                  </button>
                </div>
              </div>
              <div class="shots">
                {#if gameShots.length}
                  {#each gameShots as s (s.path)}
                    <button type="button" class="glass shot" onclick={() => openCollShots(activeIdx, s.path, ti)}>
                      <div class="face"><Thumb path={s.path} /></div>
                    </button>
                  {/each}
                  {#if moreShots > 0}
                    <button type="button" class="glass shot more" onclick={() => openCollShots(activeIdx, undefined, ti)}>
                      <div class="face">
                        <span class="more-in">
                          <CIcon name="cam" class="ic" />
                          {moreShots} más
                          <small>Ver todas</small>
                        </span>
                      </div>
                    </button>
                  {/if}
                {:else}
                  <div class="shots-empty">
                    <CIcon name="cam" class="ic s" />
                    <span>Aún no hay capturas de este juego</span>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .coll-wrap{position:absolute;left:var(--gut);right:var(--gut);top:88px;bottom:calc(var(--band) + var(--s-3));z-index:2}
  .coll-empty-wrap{display:grid;place-items:center;height:100%}
  .coll-empty-card{position:relative;width:560px;max-width:100%;--r:var(--r-l);--bw:8px;cursor:default}
  .coll-empty-card>.face{background:var(--panel)}
  .coll-empty-card .b-main{margin-top:var(--s-2)}

  .coll-grid{display:grid;grid-template-columns:276px 1fr;gap:var(--s-4);height:100%}

  /* Lista lateral izquierda (sin iconos redondos: barra vertical cromática de 4 px) */
  .side-panel{position:relative;height:100%;--r:var(--r-l);--bw:8px;cursor:default}
  .side-panel>.face{background:var(--panel)}
  .side-face{display:flex;flex-direction:column;gap:var(--s-2);padding:var(--s-3);height:100%}
  .side-lab{font:800 var(--t-xs)/1 var(--round);text-transform:uppercase;letter-spacing:.08em;color:var(--ink-2);padding:var(--s-1) var(--s-2)}
  .side-list{display:flex;flex-direction:column;gap:var(--s-2);flex:1;min-height:0;overflow-y:auto;padding:var(--s-1)}
  .side-list::-webkit-scrollbar{width:4px}
  .side-list::-webkit-scrollbar-thumb{background:var(--line-2);border-radius:var(--r-pill)}

  .sitem{position:relative;display:flex;align-items:center;gap:var(--s-3);padding:var(--s-2) var(--s-3) var(--s-2) var(--s-4);
    border-radius:var(--r-m);cursor:pointer;background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line);
    border:0;width:100%;text-align:left;color:var(--ink);flex:none;overflow:hidden;
    transition:transform var(--d-2) var(--e-bounce),box-shadow var(--d-2) var(--e-soft),background var(--d-2) var(--e-soft)}
  .sitem:hover{transform:scale(1.02)}
  .sitem.on{background:var(--glass-chip);box-shadow:inset 0 1px 0 var(--hl),var(--sh-2)}
  .sitem.f{transform:scale(1.04);box-shadow:var(--ring)}

  .sitem-bar{position:absolute;left:0;top:var(--s-1);bottom:var(--s-1);width:4px;border-radius:var(--r-pill);opacity:.78;
    transition:opacity var(--d-2) var(--e-soft),box-shadow var(--d-2) var(--e-soft)}
  .sitem.on .sitem-bar{opacity:1;box-shadow:0 0 10px color-mix(in srgb,var(--v1) 65%,transparent)}

  .sitem-t{min-width:0;flex:1}
  .sitem-t b{display:block;font:800 var(--t-m)/1.2 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sitem.on .sitem-t b{color:var(--f1)}
  :global(#stage[data-theme=dark]) .sitem.on .sitem-t b{color:color-mix(in srgb,var(--v1) 68%,white)}
  .sitem-t small{display:block;font:700 var(--t-s)/1.2 var(--round);color:var(--ink-2);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .sitem.add{justify-content:center;gap:var(--s-2);padding:0 var(--s-3);height:44px;font:700 var(--t-m)/1 var(--round);
    color:var(--ink-2);box-shadow:inset 0 0 0 1.5px var(--line-2)}
  .sitem.add:hover{color:var(--ink)}
  .sitem.add :global(svg){width:var(--i-m);height:var(--i-m)}

  /* Panel derecho principal */
  .main-panel{position:relative;height:100%;min-width:0;--r:var(--r-l);--bw:8px;cursor:default}
  .main-panel>.face{background:var(--panel)}
  .main-face{display:flex;flex-direction:column;padding:var(--s-4) var(--s-5) var(--s-4);gap:var(--s-3);height:100%}

  .aurora{position:absolute;inset:0;pointer-events:none;overflow:hidden;opacity:.34}
  :global(#stage[data-theme=dark]) .aurora{opacity:.28}
  .aurora i{position:absolute;border-radius:50%;width:340px;height:340px;top:-150px;
    background:radial-gradient(circle,var(--ac),transparent 68%)}

  /* Cabecera: nombre, años y progreso. Nada más. */
  .mhead{position:relative;z-index:2;display:flex;align-items:center;gap:var(--s-3);height:44px;flex:none;min-width:0}
  .mhead h1{margin:0;font:800 var(--t-2xl)/1.15 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0}
  .tag{display:inline-flex;align-items:center;gap:var(--s-1);height:28px;padding:0 var(--s-3);border-radius:var(--r-pill);flex:none;
    font:700 var(--t-s)/1 var(--round);color:var(--on);background:var(--gf);white-space:nowrap}
  .tag.soft{color:var(--ink-2);background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line)}
  .prog{margin-left:auto;display:flex;align-items:center;gap:var(--s-3);flex:none;font:700 var(--t-s)/1 var(--round);color:var(--ink-2);white-space:nowrap}
  .prog b{color:var(--ink);font-weight:800}
  .prog-bar{display:flex;gap:3px;width:150px;height:8px;padding:1px;border-radius:var(--r-pill);
    background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line)}
  .prog-bar i{flex:1;border-radius:var(--r-pill);background:var(--line-2)}

  /* Juegos: siempre caben. Se reparten en filas y la portada se encoge (con muchísimos, la zona baja en vertical). */
  .games{position:relative;z-index:2;flex:1;min-height:0;display:flex;flex-direction:column;justify-content:center;gap:14px;overflow:hidden}
  .games.scroll{justify-content:flex-start;overflow-y:auto;scrollbar-width:none;padding-top:var(--s-2)}
  .games.scroll::-webkit-scrollbar{display:none}
  .grow{position:relative;display:flex;justify-content:center;gap:12px;padding-bottom:46px;flex:none}
  .grow::after{content:'';position:absolute;left:0;right:0;bottom:31px;height:6px;border-radius:var(--r-pill);background:var(--rail);opacity:.9;
    box-shadow:inset 0 1px 0 rgba(255,255,255,.45),0 4px 12px -3px color-mix(in srgb,var(--v1) 40%,transparent)}
  .gi{position:relative;display:flex;flex-direction:column;align-items:center;cursor:pointer}
  .gcov{position:relative;width:var(--cw);height:calc(var(--cw) * 4 / 3);--r:var(--r-m);--bw:5px}
  .gcov .face{border-radius:var(--r-s)}
  .gcov :global(.noart){padding:var(--s-1)}
  .gcov :global(.noart b){font-size:var(--t-xs);-webkit-line-clamp:4;line-clamp:4}
  .gi.dim .gcov .face{filter:saturate(.35) brightness(.92)}
  .gi.dim .gcov.f .face{filter:none}
  .gdot{position:absolute;bottom:-21px;width:12px;height:12px;border-radius:50%;background:var(--gv);z-index:3;
    box-shadow:0 0 0 3px var(--panel);transition:transform var(--d-2) var(--e-bounce),box-shadow var(--d-2) var(--e-soft)}
  .gi.f .gdot{transform:scale(1.4);box-shadow:0 0 0 3px var(--panel),0 0 0 6px color-mix(in srgb,var(--v1) 42%,transparent)}
  .gyr{position:absolute;bottom:-44px;font:800 var(--t-xs)/1 var(--round);color:var(--ink-2);padding:3px 7px;border-radius:var(--r-pill);white-space:nowrap}
  .gi.f .gyr{background:var(--gf);color:var(--on);font-size:var(--t-s)}

  .g-add{position:relative;align-self:center;width:112px;height:152px;--r:var(--r-m);--bw:5px}
  .g-add .addin{gap:var(--s-2)}
  .g-add .addin span{width:40px;height:40px;border-radius:var(--r-s);color:var(--on)}
  .g-add .addin b{font-size:var(--t-s);padding:0 var(--s-1)}

  /* Ficha del juego enfocado con capturas suyas al azar debajo */
  .det{position:relative;z-index:2;flex:none;display:flex;flex-direction:column;gap:var(--s-3);padding:var(--s-3) var(--s-4) var(--s-4) calc(var(--s-4) + 5px);overflow:hidden}
  .det::before{content:'';position:absolute;left:0;top:0;bottom:0;width:5px;background:var(--gv)}
  .det-top{display:flex;align-items:center;gap:var(--s-3);height:40px;min-width:0}
  .det-top>b{font:800 var(--t-l)/1.2 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0}
  .st-badge{display:inline-flex;align-items:center;gap:var(--s-2);height:26px;padding:0 var(--s-3);border-radius:var(--r-pill);flex:none;
    background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line);font:700 var(--t-s)/1 var(--round);color:var(--ink);white-space:nowrap}
  .st-badge i{width:10px;height:10px;border-radius:50%;flex:none}
  .det-meta{font:700 var(--t-s)/1 var(--round);color:var(--ink-2);white-space:nowrap;flex:none}
  .det-acts{margin-left:auto;display:flex;align-items:center;gap:var(--s-2);flex:none}
  .det-acts :global(svg){width:var(--i-s);height:var(--i-s)}
  .b-sec{display:inline-flex;align-items:center;gap:var(--s-2);height:40px;padding:0 var(--s-4);border:0;border-radius:var(--r-pill);
    font:700 var(--t-m)/1 var(--round);color:var(--ink);background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line);cursor:pointer;
    transition:transform var(--d-2) var(--e-bounce)}
  .b-sec:hover{transform:scale(1.04)}
  .b-sec:active{transform:scale(.97);transition-duration:var(--d-1)}
  .det-acts .b-main{height:40px}

  .shots{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:var(--s-3);min-width:0}
  .shot{position:relative;width:100%;min-width:0;aspect-ratio:16/9;overflow:hidden;--r:var(--r-m);--bw:4px;border:0;padding:var(--bw);cursor:pointer;
    -webkit-backdrop-filter:none;backdrop-filter:none}
  .shot .face{border-radius:var(--r-s);background:var(--face)}
  .shot:hover{transform:scale(1.04);z-index:4;box-shadow:var(--sh-2)}
  .shot:active{transform:scale(.97);transition-duration:var(--d-1)}
  .shot.more .face{display:grid;place-items:center;background:var(--glass-in)}
  .more-in{display:flex;flex-direction:column;align-items:center;gap:var(--s-1);font:800 var(--t-l)/1 var(--round);color:var(--ink)}
  .more-in small{font:700 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .shots-empty{grid-column:1/-1;aspect-ratio:64/9;display:flex;align-items:center;justify-content:center;gap:var(--s-2);
    border-radius:var(--r-m);background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line);font:700 var(--t-s)/1 var(--round);color:var(--ink-2)}

  :global(.tablet) .coll-wrap{bottom:var(--gut)}
  :global(.tablet) .b-sec{min-height:44px}
</style>
