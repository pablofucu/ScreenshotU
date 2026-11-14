<script lang="ts">
  import { app, accv } from '../lib/store.svelte';
  import { IS_TABLET } from '../lib/backend';
  import { ICON, RAINBOW } from '../lib/icons';
  import TopBar from '../components/TopBar.svelte';
  import { dayKey, fmtAgo, fmtDur, fmtShort, naiveNow, nCapturas, nTitulos } from '../lib/format';
  import Hints from '../components/Hints.svelte';
  import RecentCard from '../components/RecentCard.svelte';
  import ReadCard from '../components/ReadCard.svelte';

  const DOCK = [
    { n: 'Buscar', ic: ICON.search, c: 0 },
    { n: 'Favoritas', ic: ICON.star, c: 1 },
    { n: 'Estadísticas', ic: ICON.chart, c: 3 },
    { n: 'Añadir juego', ic: ICON.plus, c: 4 },
  ];

  const plats = $derived(app.lib.platforms);
  const shown = $derived(plats.map((p, i) => ({ p, i })));
  const size = 190;
  // Con muchas plataformas el paso se encoge para que todas quepan en pantalla.
  const step = $derived(shown.length > 1 ? Math.min(shown.length > 3 ? 232 : 270, (1280 - 80 - size) / (shown.length - 1)) : 270);
  const x0 = $derived((1280 - (shown.length - 1) * step - size) / 2);
  const lw = $derived(Math.min(size + 40, step - 4));
  const totalTitles = $derived(plats.reduce((n, p) => n + p.titles.length, 0));
  const count = (p: (typeof plats)[number]) => p.titles.reduce((s, t) => s + t.shots.length, 0);

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
</script>

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
    {@const a = app.platAcc(i)}
    {@const x = x0 + k * step}
    <div class="glass plat fx" class:f={app.focus === 'plat' + i} data-k="plat{i}" data-act="plat:{i}" style="left:{x}px;top:100px;width:{size}px;height:{size}px;--acc:{a.c};--accv:{accv(a)};--pc:{a.c}">
      <div class="face">{@html ICON[p.icon] ?? ICON.pc}</div>
    </div>
    <div class="plabel hl" class:on={app.focus === 'plat' + i} style="left:{x + (size - lw) / 2}px;top:{112 + size}px;width:{lw}px;--pc:{a.c}">
      {p.name}<small>{nTitulos(p.titles.length)} · {nCapturas(count(p))}</small>
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
    {#if !(IS_TABLET && i === 3)}
    <div class="dpill pill fx" class:f={app.focus === 'dock' + i} data-k="dock{i}" data-act="dock:{i}" style="--dc:{RAINBOW[d.c].c}">
      <i>{@html d.ic}</i>{d.n}
    </div>
    {/if}
  {/each}
</div>

<Hints list={[['A', app.focus === 'read' ? 'Ver racha' : 'Abrir', 'a'], ...(app.focus === 'read' && app.goalKeys.length > 1 ? [['LB/RB', 'Categoría', 'lb'] as [string, string, string]] : []), ...(focusRec?.t.launchable ? [['X', 'Jugar', 'x'] as [string, string, string]] : []), ['Start', 'Menú', 'menu']]} />

<style>
  .hsect small{font-size:var(--t-m);font-weight:600;letter-spacing:0}
  .hsect small :global(b){color:var(--ink);font-weight:800}
  /* Plataformas: el color del foco entra fundido (colores registrados, que sí se interpolan) y la
     escala va sin rebote, para que color y tamaño lleguen a la vez. */
  @property --pt { syntax: '<color>'; inherits: false; initial-value: transparent; }
  @property --pm { syntax: '<color>'; inherits: false; initial-value: transparent; }
  @property --pb { syntax: '<color>'; inherits: false; initial-value: transparent; }
  .plat {
    --pt: var(--glass-top);
    --pm: var(--glass-mid);
    --pb: var(--glass-bot);
    background: linear-gradient(165deg, var(--pt) 0%, var(--pm) 48%, var(--pb) 100%);
    transition: transform 0.28s cubic-bezier(.2, .8, .2, 1), box-shadow 0.28s cubic-bezier(.2, .8, .2, 1),
      --pt 0.28s cubic-bezier(.2, .8, .2, 1), --pm 0.28s cubic-bezier(.2, .8, .2, 1), --pb 0.28s cubic-bezier(.2, .8, .2, 1);
  }
  .plat.f {
    --pt: color-mix(in srgb, var(--g1) 55%, white);
    --pm: var(--f1);
    --pb: var(--f2);
    background: linear-gradient(165deg, var(--pt) 0%, var(--pm) 48%, var(--pb) 100%);
  }
  .plabel,
  .plabel.on {
    transition: color 0.28s cubic-bezier(.2, .8, .2, 1);
  }
  @media (prefers-reduced-motion: reduce) {
    .plat, .plabel, .plabel.on { transition: none; }
  }
</style>
