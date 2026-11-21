<script lang="ts">
  // Pestaña «Colecciones» de una categoría: lista lateral de colecciones con barra de espectro vivo y panel de
  // cristal con aurora, 5 capturas aleatorias de todos los juegos, línea del tiempo cromática y ficha inferior.
  import { app, LIST_COLOR, STATUS_LABEL } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtDur, nCapturas, nGames } from '../lib/format';
  import { openTitle, setFocus } from '../lib/controller.svelte';
  import { blip } from '../lib/sound';
  import { hash } from '../lib/rng';
  import { collLayout, collShotSeed, gameVars, openCollShots, shelfAccent, shelfVars, shuffleCollShots } from '../lib/coleccion.svelte';
  import { collYearSpan, coverColors, gameYear, pickCollShots } from '../lib/collections';
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
      const span = collYearSpan(games.map((t) => gameYear(t)));
      const hours = Math.round(games.reduce((n, t) => n + (t.playSeconds ?? 0), 0) / 3600);
      return { acc, barBg, span, hours };
    }),
  );

  const activeGames = $derived(activeSh ? activeSh.items.map((ti) => titles[ti]).filter(Boolean) : []);
  const activeAcc = $derived(activeSh ? shelfAccent(activeSh) : app.chrome);
  const activeVars = $derived(activeSh ? shelfVars(activeSh, activeAcc) : '');
  const activeLayout = $derived(activeSh ? collLayout(activeSh) : null);
  const activeYears = $derived(activeGames.map((t) => gameYear(t)));
  const activeSpan = $derived(collYearSpan(activeYears));
  const activeHoursSec = $derived(activeGames.reduce((n, t) => n + (t.playSeconds ?? 0), 0));
  const activeTotalShots = $derived(activeGames.reduce((n, t) => n + t.shots.length, 0));
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

  const randomShots = $derived.by(() => {
    if (!activeSh || !activeLayout) return [];
    const sources = activeSh.items.map((ti, j) => ({
      ti,
      name: titles[ti].name,
      shots: titles[ti].shots,
      tint: activeLayout.tints[j] ?? app.pairFill(app.titlePair(app.p, ti)).v,
    }));
    return pickCollShots(sources, 5, (hash(activeSh.coll.id) ^ collShotSeed()) >>> 0);
  });

  let tlEl = $state<HTMLElement>();
  let sideEl = $state<HTMLElement>();

  // Mantiene a la vista el juego enfocado en la línea del tiempo y la colección activa en la lista lateral.
  $effect(() => {
    void app.collAt;
    void activeIdx;
    queueMicrotask(() => {
      const cov = tlEl?.querySelector<HTMLElement>('.tl-it.f');
      if (cov && tlEl) {
        const left = cov.offsetLeft - (tlEl.clientWidth - cov.offsetWidth) / 2;
        tlEl.scrollTo({ left: Math.max(0, left), behavior: 'smooth' });
      }
      const row = sideEl?.querySelector<HTMLElement>('.sitem.on');
      row?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    });
  });

  const statusColor = (st?: string) => LIST_COLOR[(st as keyof typeof LIST_COLOR) || 'none']?.c;
  const gameHoursText = (sec: number) => (sec >= 3600 ? `${Math.round(sec / 3600)} h` : '—');
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
                  <small>{sb.span ? `${sb.span} · ` : ''}{nGames(sh.items.length)}{sb.hours >= 1 ? ` · ${sb.hours} h` : ''}</small>
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
            <div class="mhead-t">
              <div class="mhead-row">
                <h1>{activeSh.coll.name}</h1>
                {#if activeSpan}
                  <span class="tag soft"><CIcon name="cal" class="ic s" /> {activeSpan}</span>
                {/if}
                <span class="tag">
                  {activeDone > 0 ? `${activeDone} / ${activeSh.items.length} terminados` : nGames(activeSh.items.length)}
                </span>
              </div>
              <div class="mhead-sub">
                {#if activeHoursSec >= 60}
                  <span><CIcon name="clock" class="ic s" /> {fmtDur(activeHoursSec)}</span>
                {/if}
                <span><CIcon name="cam" class="ic s" /> {nCapturas(activeTotalShots)}</span>
                {#if activeSh.items.length}
                  <span>Juego {focusedPos + 1} de {activeSh.items.length}</span>
                {/if}
              </div>
            </div>
            {#if hasAnyStatus && activeSh.items.length}
              <div class="st-seg">
                <div class="st-seg-bar">
                  {#each activeGames as gm}
                    {@const st = gm.status ?? 'none'}
                    <i style={st !== 'none' ? `background:${statusColor(st)}` : ''}></i>
                  {/each}
                </div>
                {#if activeDone > 0}
                  <small>{Math.round((activeDone / activeSh.items.length) * 100)} % completado</small>
                {/if}
              </div>
            {/if}
          </div>

          <div class="cshots-sec">
            {#if randomShots.length}
              <div class="cshots-top">
                <span>Capturas al azar de la colección</span>
                <div class="cshots-acts">
                  {#if activeTotalShots > 5}
                    <button type="button" class="cshots-btn" onclick={shuffleCollShots}>
                      {@html ICON.reload} Otras
                    </button>
                  {/if}
                  <button type="button" class="cshots-btn" onclick={() => openCollShots(activeIdx)}>
                    {@html ICON.camera} Ver todas ({activeTotalShots})
                  </button>
                </div>
              </div>
              <div class="cshots-row">
                {#each randomShots as s (s.path)}
                  <button
                    type="button"
                    class="glass cshot"
                    style={s.tint ? `--sc:${s.tint}` : ''}
                    onclick={() => openCollShots(activeIdx, s.path)}
                  >
                    <div class="face">
                      <Thumb path={s.path} />
                      <span class="shot-cap">
                        <i style={s.tint ? `background:${s.tint}` : ''}></i>
                        <b>{s.game}</b>
                      </span>
                    </div>
                  </button>
                {/each}
              </div>
            {:else}
              <div class="glass-in cshots-empty">
                <CIcon name="cam" class="ic s" />
                <span>Aún no hay capturas en esta colección</span>
              </div>
            {/if}
          </div>

          <div class="tl-zone">
            {#if !activeSh.items.length}
              <div class="tl-empty-act">
                <div
                  class="glass scov addtile tl-add fx"
                  class:f={!onNew}
                  data-k="cl{activeIdx}.0"
                  data-act="cgame:{activeIdx}:0"
                >
                  <div class="face">
                    <div class="addin"><span>{@html ICON.plus}</span><b>Añadir juegos</b></div>
                  </div>
                </div>
              </div>
            {:else if activeLayout}
              <div
                class="tl-scroll"
                bind:this={tlEl}
                onwheel={(e) => {
                  const dy = e.deltaY || e.deltaX;
                  if (!dy) return;
                  e.currentTarget.scrollLeft += dy;
                  e.preventDefault();
                }}
              >
                <div class="tl-track">
                  <div class="tl-rail" style="background:{activeLayout.rail ?? 'var(--gv)'}"></div>
                  {#each activeSh.items as ti, j (ti)}
                    {@const t = titles[ti]}
                    {@const yr = activeYears[j]}
                    {@const prevYr = j > 0 ? activeYears[j - 1] : null}
                    {@const isF = !onNew && app.collAt.i === j}
                    {#if yr !== null && prevYr !== null && yr - prevYr >= 3}
                      <span class="tl-gap">+{yr - prevYr} a</span>
                    {/if}
                    <div class="tl-it" class:f={isF} style={gameVars(ti)}>
                      <div
                        class="glass scov tl-cov fx"
                        class:f={isF}
                        data-k="cl{activeIdx}.{j}"
                        data-act="cgame:{activeIdx}:{j}"
                      >
                        <div class="face">
                          {#if coverOf(ti)}
                            <Thumb path={coverOf(ti)!} />
                          {:else}
                            <div class="noart"><b>{t.name}</b></div>
                          {/if}
                          <span class="cov-cap">{t.name}</span>
                        </div>
                      </div>
                      <div class="tl-stem"></div>
                      <div class="tl-dot"></div>
                      <div class="tl-meta">
                        <span class="tl-yr">{yr ?? '–'}</span>
                        <span class="tl-hr">{gameHoursText(t.playSeconds ?? 0)}</span>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          {#if focusedGame && focusedTi !== undefined}
            {@const st = focusedGame.status ?? 'none'}
            <div class="glass-in det-bar" style={gameVars(focusedTi)}>
              <div class="det-info">
                <div class="det-top">
                  <b>{focusedGame.name}</b>
                  {#if st !== 'none'}
                    <span class="st-badge">
                      <i style="background:{statusColor(st)}"></i>
                      {STATUS_LABEL[st]}
                    </span>
                  {/if}
                </div>
                <div class="det-nums">
                  <span>Año <strong>{focusedYear ?? '–'}</strong></span>
                  {#if focusedGame.playSeconds >= 60}
                    <span>Jugado <strong>{fmtDur(focusedGame.playSeconds)}</strong></span>
                  {/if}
                  <span><strong>{nCapturas(focusedGame.shots.length)}</strong></span>
                </div>
              </div>
              <button type="button" class="b-main" onclick={() => openTitle(focusedTi)}>
                {@html ICON.play} Abrir juego
              </button>
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
  .main-face{display:flex;flex-direction:column;padding:var(--s-4) var(--s-5) var(--s-3);gap:var(--s-3);height:100%}

  .aurora{position:absolute;inset:0;pointer-events:none;overflow:hidden;opacity:.34}
  :global(#stage[data-theme=dark]) .aurora{opacity:.28}
  .aurora i{position:absolute;border-radius:50%;width:340px;height:340px;top:-150px;
    background:radial-gradient(circle,var(--ac),transparent 68%)}

  .mhead{position:relative;z-index:2;display:flex;align-items:center;gap:var(--s-4);flex:none}
  .mhead-t{display:flex;flex-direction:column;gap:var(--s-1);min-width:0;flex:1}
  .mhead-row{display:flex;align-items:center;gap:var(--s-3);flex-wrap:wrap}
  .mhead-row h1{margin:0;font:800 var(--t-2xl)/1.15 var(--round);color:var(--ink)}
  .mhead-sub{display:flex;align-items:center;gap:var(--s-4);font:700 var(--t-s)/1.2 var(--round);color:var(--ink-2)}
  .mhead-sub span{display:inline-flex;align-items:center;gap:var(--s-1)}

  .tag{display:inline-flex;align-items:center;gap:var(--s-1);height:28px;padding:0 var(--s-3);border-radius:var(--r-pill);
    font:700 var(--t-s)/1 var(--round);color:var(--on);background:var(--gf);white-space:nowrap}
  .tag.soft{color:var(--ink-2);background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line)}

  .st-seg{display:flex;flex-direction:column;align-items:flex-end;gap:var(--s-1);flex:none}
  .st-seg-bar{display:flex;gap:3px;width:196px;height:8px;padding:1px;border-radius:var(--r-pill);
    background:var(--glass-in);box-shadow:inset 0 0 0 1px var(--line)}
  .st-seg-bar i{flex:1;border-radius:var(--r-pill);background:var(--line-2)}
  .st-seg small{font:700 var(--t-s)/1 var(--round);color:var(--ink-2)}

  /* Franja de 5 capturas aleatorias de todos los juegos */
  .cshots-sec{position:relative;z-index:2;display:flex;flex-direction:column;gap:var(--s-1);flex:none}
  .cshots-top{display:flex;align-items:center;justify-content:space-between;gap:var(--s-3)}
  .cshots-top>span{font:800 var(--t-xs)/1 var(--round);text-transform:uppercase;letter-spacing:.08em;color:var(--ink-2)}
  .cshots-acts{display:flex;align-items:center;gap:var(--s-2)}
  .cshots-btn{display:inline-flex;align-items:center;gap:var(--s-1);height:26px;padding:0 var(--s-3);border:0;
    border-radius:var(--r-pill);background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line);
    font:700 var(--t-s)/1 var(--round);color:var(--ink-2);cursor:pointer;
    transition:transform var(--d-2) var(--e-bounce),color var(--d-1) var(--e-soft),background var(--d-1) var(--e-soft)}
  .cshots-btn:hover{transform:scale(1.04);color:var(--ink);background:var(--panel)}
  .cshots-btn:active{transform:scale(.97);transition-duration:var(--d-1)}
  .cshots-btn :global(svg){width:var(--i-s);height:var(--i-s)}

  .cshots-row{display:grid;grid-template-columns:repeat(5,1fr);gap:var(--s-2)}
  .cshot{position:relative;width:100%;height:96px;--r:var(--r-m);--bw:4px;border:0;padding:var(--bw);text-align:left;
    -webkit-backdrop-filter:none;backdrop-filter:none;cursor:pointer}
  .cshot .face{border-radius:var(--r-s);background:var(--face)}
  .cshot:hover{transform:scale(1.04);z-index:4;box-shadow:var(--sh-2)}
  .cshot:active{transform:scale(.97);transition-duration:var(--d-1)}
  .shot-cap{position:absolute;left:0;right:0;bottom:0;display:flex;align-items:center;gap:var(--s-1);padding:var(--s-2) var(--s-2) var(--s-1);
    background:linear-gradient(180deg,transparent 0%,color-mix(in srgb,var(--scrim) 45%,black) 100%);
    color:var(--on);font:700 var(--t-xs)/1.15 var(--round)}
  .shot-cap i{width:7px;height:7px;border-radius:50%;background:var(--sc,var(--v1));flex:none}
  .shot-cap b{font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .cshots-empty{display:flex;align-items:center;gap:var(--s-2);height:36px;padding:0 var(--s-3);
    font:500 var(--t-s)/1 var(--round);color:var(--ink-2)}

  /* Línea del tiempo cromática */
  .tl-zone{position:relative;z-index:2;flex:1;min-height:0;display:flex;flex-direction:column}
  .tl-empty-act{flex:1;display:grid;place-items:center}
  .tl-add{position:relative;width:112px;height:152px;--r:var(--r-m);--bw:5px}
  .tl-add .addin{gap:var(--s-2)}
  .tl-add .addin span{width:40px;height:40px;border-radius:var(--r-s);color:var(--on)}
  .tl-add .addin b{font-size:var(--t-s);padding:0 var(--s-1)}

  .tl-scroll{flex:1;min-height:0;overflow-x:auto;overflow-y:hidden;display:flex;align-items:flex-end;scrollbar-width:none}
  .tl-scroll::-webkit-scrollbar{display:none}
  .tl-track{position:relative;display:inline-flex;min-width:100%;align-items:flex-end;justify-content:space-between;
    gap:var(--s-3);padding:12px var(--s-3) 60px}
  .tl-rail{position:absolute;left:var(--s-5);right:var(--s-5);bottom:46px;height:8px;border-radius:var(--r-pill);
    box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 4px 14px -2px color-mix(in srgb,var(--v1) 48%,transparent)}

  .tl-it{position:relative;display:flex;flex-direction:column;align-items:center;flex:1 0 auto;min-width:86px;cursor:pointer}
  .tl-cov{position:relative;width:82px;height:106px;--r:var(--r-m);--bw:5px}
  .tl-cov .face{border-radius:var(--r-s)}
  .tl-cov :global(.noart){padding:var(--s-1)}
  .tl-cov :global(.noart b){font-size:var(--t-s);-webkit-line-clamp:4;line-clamp:4}
  .cov-cap{position:absolute;left:0;right:0;bottom:0;padding:var(--s-1) var(--s-2);
    background:linear-gradient(180deg,transparent 0%,color-mix(in srgb,var(--scrim) 45%,black) 100%);
    color:var(--on);font:800 var(--t-xs)/1.15 var(--round);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}

  .tl-stem{width:3px;height:10px;border-radius:var(--r-xs);background:var(--gv);opacity:.55;margin-top:var(--s-1);
    transition:opacity var(--d-2) var(--e-soft)}
  .tl-it.f .tl-stem{opacity:1}
  .tl-dot{position:absolute;bottom:-17px;width:14px;height:14px;border-radius:50%;background:var(--gv);
    box-shadow:0 0 0 3px var(--panel),0 0 12px color-mix(in srgb,var(--v1) 65%,transparent);
    transition:transform var(--d-2) var(--e-bounce),box-shadow var(--d-2) var(--e-soft)}
  .tl-it.f .tl-dot{transform:scale(1.36);
    box-shadow:0 0 0 3px var(--panel),0 0 0 6px color-mix(in srgb,var(--v1) 42%,transparent)}
  .tl-meta{position:absolute;top:calc(100% + 22px);display:flex;flex-direction:column;align-items:center;gap:2px;white-space:nowrap}
  .tl-yr{font:800 var(--t-s)/1 var(--round);color:var(--ink);padding:2px var(--s-2);border-radius:var(--r-pill)}
  .tl-it.f .tl-yr{background:var(--gf);color:var(--on)}
  .tl-hr{font:700 var(--t-s)/1 var(--round);color:var(--ink-2)}
  .tl-gap{flex:0 0 auto;align-self:flex-end;margin-bottom:-15px;font:800 var(--t-xs)/1 var(--round);color:var(--ink-2)}

  /* Ficha inferior del juego enfocado */
  .det-bar{position:relative;z-index:2;flex:none;display:flex;align-items:center;justify-content:space-between;gap:var(--s-4);
    padding:var(--s-3) var(--s-4);overflow:hidden}
  .det-bar::before{content:'';position:absolute;left:0;top:0;bottom:0;width:5px;background:var(--gv)}
  .det-info{display:flex;flex-direction:column;gap:var(--s-1);min-width:0;flex:1}
  .det-top{display:flex;align-items:center;gap:var(--s-3);min-width:0}
  .det-top b{font:800 var(--t-l)/1.2 var(--round);color:var(--ink);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .st-badge{display:inline-flex;align-items:center;gap:var(--s-2);height:26px;padding:0 var(--s-3);border-radius:var(--r-pill);
    background:var(--glass-chip);box-shadow:inset 0 0 0 1px var(--line);font:700 var(--t-s)/1 var(--round);color:var(--ink);white-space:nowrap}
  .st-badge i{width:10px;height:10px;border-radius:50%;flex:none}
  .det-nums{display:flex;align-items:center;gap:var(--s-4);font:700 var(--t-s)/1.2 var(--round);color:var(--ink-2)}
  .det-nums strong{color:var(--ink);font-weight:800}
  .det-bar .b-main{flex:none}
  .det-bar .b-main :global(svg){width:var(--i-s);height:var(--i-s)}

  :global(.tablet) .coll-wrap{bottom:var(--gut)}
  :global(.tablet) .cshots-btn{min-height:44px;padding:0 var(--s-4)}
</style>
