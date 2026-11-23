<script lang="ts">
  import { app, accv } from '../lib/store.svelte';
  import { IS_TABLET } from '../lib/backend';
  import { ICON, RAINBOW } from '../lib/icons';
  import { orderedPlatforms, sectionRainbowIdx, type SpaceId } from '../lib/sections';
  import TopBar from '../components/TopBar.svelte';
  import { dayKey, fmtAgo, fmtDur, fmtShort, naiveNow, nCapturas, nTitulos } from '../lib/format';
  import { GOAL_COL, type GoalKey } from '../lib/readgoals';
  import Hints from '../components/Hints.svelte';
  import RecentCard from '../components/RecentCard.svelte';
  import ReadCard from '../components/ReadCard.svelte';
  import WeekNew from '../components/WeekNew.svelte';

  const DOCK = [
    { n: 'Buscar', ic: ICON.search, c: 0 },
    { n: 'Añadir juego', ic: ICON.plus, c: 4 },
  ];

  const SPACES: ReadonlyArray<{ id: SpaceId; label: string; ic: string }> = [
    { id: 'album', label: 'Álbum', ic: ICON.album },
    { id: 'stats', label: 'Estadísticas', ic: ICON.statsTile },
    { id: 'logros', label: 'Logros', ic: ICON.logros },
  ];

  const plats = $derived(app.lib.platforms);
  const shown = $derived(orderedPlatforms(plats));

  // Jerarquía 4+3 en fila: 4 teselas grandes (170 px) + separador vertical + 3 espacios medianos (132 px),
  // alineados abajo en y = 264 px y etiquetas alineadas en top: 280 px.
  const CAT_BASE = 264;
  const SP_S = 132;
  const SP_STEP = 148;
  const SP_TOP = CAT_BASE - SP_S; // 132
  const SP_W = (SPACES.length - 1) * SP_STEP + SP_S; // 428
  const DIV_GAP = 36;

  const geo = $derived.by(() => {
    const n = shown.length;
    if (n <= 4) {
      const catS = 170;
      const catStep = 190;
      const catW = n > 0 ? (n - 1) * catStep + catS : 0;
      const totalW = n > 0 ? catW + DIV_GAP + SP_W : SP_W;
      const x0 = Math.round((1280 - totalW) / 2);
      const spX0 = n > 0 ? x0 + catW + DIV_GAP : x0;
      return { catS, catStep, catTop: CAT_BASE - catS, x0, spX0, divX: x0 + catW + DIV_GAP / 2 };
    }
    const maxCatW = 1280 - 76 - DIV_GAP - SP_W;
    const catStep = Math.floor(maxCatW / n);
    const catS = Math.min(170, Math.max(108, catStep - 14));
    const catW = (n - 1) * catStep + catS;
    const totalW = catW + DIV_GAP + SP_W;
    const x0 = Math.round((1280 - totalW) / 2);
    return { catS, catStep, catTop: CAT_BASE - catS, x0, spX0: x0 + catW + DIV_GAP, divX: x0 + catW + DIV_GAP / 2 };
  });

  const totalTitles = $derived(plats.reduce((n, p) => n + p.titles.length, 0));
  const count = (p: (typeof plats)[number]) => p.titles.reduce((s, t) => s + t.shots.length, 0);
  const totalShots = $derived(plats.reduce((s, p) => s + count(p), 0));
  const totalPlay = $derived(plats.reduce((s, p) => s + p.titles.reduce((a, t) => a + (t.hidden ? 0 : t.playSeconds), 0), 0));

  // «Seguir jugando»
  const CW = 350, CGAP = 34, RW = 440;
  const recent = $derived(app.recent.map((r) => ({ ...r, t: app.lib.platforms[r.pi].titles[r.ti] })));
  // La fila se centra entera: recientes + (si hay) la ficha de metas.
  const rowW = $derived(recent.length * (CW + CGAP) - CGAP + (app.readCard ? CGAP + RW : 0));
  const cx0 = $derived((1280 - rowW) / 2);
  const rx = $derived(cx0 + recent.length * (CW + CGAP));
  const focusRec = $derived(app.focus.startsWith('rec') ? recent[+app.focus.slice(3)] : undefined);
  function meta(t: (typeof recent)[number]['t'], pname: string) {
    const when = t.lastPlayed ? `Jugado ${fmtAgo(t.lastPlayed)}` : t.shots[0] ? `Captura del ${fmtShort(t.shots[0].date)}` : '';
    const rest = [t.playSeconds >= 60 ? fmtDur(t.playSeconds) : pname, t.shots.length ? nCapturas(t.shots.length) : ''].filter(Boolean);
    return { when, rest: rest.join(' · ') };
  }

  // Resumen de los últimos 7 días (minutos de las partidas y capturas con fecha en esos días).
  const week = $derived.by(() => {
    let min = 0, shots = 0;
    const from = dayKey(naiveNow() - 6 * 864e5);
    for (const p of plats) {
      for (const t of p.titles) {
        if (t.hidden) continue;
        min += (t.week ?? []).reduce((a, b) => a + b, 0);
        for (const s of t.shots) { if (dayKey(s.date) >= from) shots++; else break; }
      }
    }
    return { min, shots };
  });

  const achSub = $derived.by(() => {
    const o = app.achOverview;
    if (!o || o.totalAchievements === 0) return 'Vitrina de logros';
    if (o.completedGames > 0) return `${o.totalUnlocked.toLocaleString('es-ES')} · ${o.completedGames} al 100 %`;
    return `${o.totalUnlocked.toLocaleString('es-ES')} de ${o.totalAchievements.toLocaleString('es-ES')}`;
  });

  function spaceSub(id: SpaceId): string {
    if (id === 'album') return app.favs.size > 0 ? `${nCapturas(totalShots)} · ${app.favs.size} fav.` : nCapturas(totalShots);
    if (id === 'stats') return week.min > 0 ? `${fmtDur(week.min * 60)} / sem` : totalPlay > 0 ? fmtDur(totalPlay) : 'Actividad y rachas';
    return achSub;
  }

  const washTarget = $derived.by(() => {
    const f = app.focus;
    if (f.startsWith('plat')) {
      const pi = +f.slice(4);
      if (plats[pi]) return app.pairStyle(app.secPair(pi));
    } else if (f.startsWith('space')) {
      const sp = SPACES[+f.slice(5)];
      if (sp) return app.pairStyle(app.spacePair(sp.id));
    } else if (f.startsWith('rec')) {
      const r = recent[+f.slice(3)];
      if (r) return app.pipStyle(r.pi, r.ti);
    } else if (f.startsWith('dock')) {
      const d = DOCK[+f.slice(4)];
      if (d) return `--p1:${RAINBOW[d.c].c};--p2:${RAINBOW[(d.c + 1) % RAINBOW.length].c}`;
    } else if (f === 'read' || f === 'avatar') {
      const k = (app.homeGoal && app.goalKeys.includes(app.homeGoal) ? app.homeGoal : app.goalKeys[0]) as GoalKey | undefined;
      const g = k ? GOAL_COL[k] : undefined;
      if (g) return `--p1:${g.c};--p2:${g.edge}`;
    }
    return '';
  });
  let homeWash = $state('');
  $effect(() => {
    if (washTarget && !app.overlay) homeWash = washTarget;
  });
</script>

<div class="ambwash" style={homeWash}></div>
<TopBar crumbs={['Inicio']} pad />

{#if !app.config.root}
  <!-- En la tablet no se elige la carpeta de la biblioteca: es la del PC. -->
  {#if !IS_TABLET}
  <div class="glass welcome fx" class:f={app.focus === 'welcome'} data-k="welcome" data-act="welcome" style="left:240px;top:130px">
    <div class="face">
      <div class="wicon">{@html ICON.import}</div>
      <h2>Elige la carpeta de tu biblioteca</h2>
      <p>Todo lo del visor vive en una sola carpeta, en el disco que quieras:</p>
      <pre>Biblioteca/
  Capturas/PC/Elden Ring/captura1.png
  Partidas/  Grafismos/  Glosario/
  visor.db  config.json</pre>
      <p class="wgo"><span class="btn a">A</span> Elegir carpeta</p>
    </div>
  </div>
  {/if}
{:else}
  {#each shown as { p, i }, k (p.id)}
    {@const rbIdx = sectionRainbowIdx(p.id)}
    {@const a = app.platAcc(i)}
    {@const pf = app.pairFill(app.secPair(i))}
    {@const x = geo.x0 + k * geo.catStep}
    <div
      class="glass plat fx"
      class:gold={rbIdx === 2}
      class:f={app.focus === 'plat' + i}
      data-k="plat{i}"
      data-act="plat:{i}"
      style="left:{x}px;top:{geo.catTop}px;width:{geo.catS}px;height:{geo.catS}px;--acc:{a.c};--accv:{accv(a)};--pc:{RAINBOW[rbIdx].c};--pg:{pf.g1};--pf:{pf.c};--g1:{pf.g1};--g2:{pf.g2};--f1:{pf.c};--f2:{pf.c2}"
    >
      <div class="face">{@html ICON[p.icon] ?? ICON.pc}</div>
    </div>
    <div class="plabel" class:on={app.focus === 'plat' + i} style="left:{x - 8}px;top:280px;width:{geo.catS + 16}px;--pc:{RAINBOW[rbIdx].c};--pf:{pf.c}">
      {p.name}<small>{nTitulos(p.titles.length)} · {nCapturas(count(p))}</small>
    </div>
  {/each}

  {#if shown.length > 0}
    <div class="div-v" style="left:{geo.divX}px;top:{Math.max(108, geo.catTop + 22)}px;height:{Math.max(100, CAT_BASE - Math.max(108, geo.catTop + 22) - 12)}px"></div>
  {/if}
  <div class="group-tag" style="left:{geo.spX0}px;top:108px">Espacios</div>

  {#each SPACES as sp, si (sp.id)}
    {@const rbIdx = sectionRainbowIdx(sp.id)}
    {@const a = app.spaceAcc(sp.id)}
    {@const pf = app.pairFill(app.spacePair(sp.id))}
    {@const x = geo.spX0 + si * SP_STEP}
    <div
      class="glass plat space fx"
      class:f={app.focus === 'space' + si}
      data-k="space{si}"
      data-act="space:{sp.id}"
      style="left:{x}px;top:{SP_TOP}px;width:{SP_S}px;height:{SP_S}px;--acc:{a.c};--accv:{accv(a)};--pc:{RAINBOW[rbIdx].c};--pg:{pf.g1};--pf:{pf.c};--g1:{pf.g1};--g2:{pf.g2};--f1:{pf.c};--f2:{pf.c2}"
    >
      <div class="face">{@html sp.ic}</div>
    </div>
    <div class="plabel sp-lbl" class:on={app.focus === 'space' + si} style="left:{x - 6}px;top:280px;width:{SP_S + 12}px;--pc:{RAINBOW[rbIdx].c};--pf:{pf.c}">
      {sp.label}<small>{spaceSub(sp.id)}</small>
    </div>
  {/each}

  {#if totalTitles === 0 && !app.scanning}
    <div class="note pill" style="left:50%;top:452px">No encontré capturas en esta carpeta. Espero <b>Biblioteca/Plataforma/Juego/imagen.png</b>. Cambia la carpeta en Ajustes.</div>
  {/if}

  {#if recent.length}
    <div class="hsect" style="left:{cx0}px">Seguir jugando{#if week.min > 0 || week.shots > 0}<small>&nbsp;· Últimos 7 días: <b>{week.min > 0 ? fmtDur(week.min * 60) : 'sin partidas'}</b> · <b>{nCapturas(week.shots)}</b></small>{/if}</div>
    {#each recent as r, i (r.t.id)}
      {@const x = cx0 + i * (CW + CGAP)}
      {@const m = meta(r.t, app.lib.platforms[r.pi].name)}
      <div class="glass rcard fx" class:f={app.focus === 'rec' + i} data-k="rec{i}" data-act="rec:{i}" style="left:{x}px;--acc:{app.titleAcc(r.pi, r.ti).c};--accv:{accv(app.titleAcc(r.pi, r.ti))};{app.pipStyle(r.pi, r.ti)}">
        <RecentCard t={r.t} playing={!!r.t.gameId && app.playing?.id === r.t.gameId} />
      </div>
      <div class="rmeta" style="left:{x + 4}px">{m.when}{#if m.rest}<span>{m.when ? ' · ' : ''}{m.rest}</span>{/if}</div>
    {/each}
    {#if app.readCard}
      <ReadCard left={rx} w={RW} />
    {/if}
  {:else if totalTitles > 0}
    <div class="hsect" style="left:96px">Seguir jugando</div>
    {#if app.readCard}
      <ReadCard left={1280 - 96 - RW} w={RW} />
    {/if}
    <div class="rnone" style={app.readCard ? `max-width:${1280 - 96 - RW - 96 - 34}px;white-space:normal` : ''}>Aquí aparecerán los juegos a los que juegues desde el visor y los que tengan capturas nuevas.</div>
  {/if}
{/if}

<div class="hdock">
  {#each DOCK as d, i}
    {#if !(IS_TABLET && i === 1)}
    <div class="dpill pill fx" class:f={app.focus === 'dock' + i} data-k="dock{i}" data-act="dock:{i}" style="--dc:{RAINBOW[d.c].c}">
      <i>{@html d.ic}</i>{d.n}
    </div>
    {/if}
  {/each}
</div>

{#if app.config.root}<WeekNew />{/if}

<Hints list={[['A', app.focus === 'read' ? 'Ver racha' : 'Abrir', 'a'], ...(app.focus === 'read' && app.goalKeys.length > 1 ? [['LB/RB', 'Categoría', 'lb'] as [string, string, string]] : []), ...(focusRec?.t.launchable ? [['X', 'Jugar', 'x'] as [string, string, string]] : []), ['Start', 'Menú', 'menu']]} />

<style>
  .hsect small{font-size:var(--t-m);font-weight:600;letter-spacing:0}
  .hsect small :global(b){color:var(--ink);font-weight:800}
</style>
