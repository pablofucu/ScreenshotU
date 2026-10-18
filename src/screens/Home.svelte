<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ICON, RAINBOW } from '../lib/icons';
  import TopBar from '../components/TopBar.svelte';
  import { dayKey, fmtAgo, fmtDur, fmtShort, naiveNow, nCapturas, nTitulos } from '../lib/format';
  import Hints from '../components/Hints.svelte';
  import RecentCard from '../components/RecentCard.svelte';

  const DOCK = [
    { n: 'Buscar', ic: ICON.search, c: 1 },
    { n: 'Favoritas', ic: ICON.star, c: 2 },
    { n: 'Estadísticas', ic: ICON.chart, c: 3 },
    { n: 'Añadir juego', ic: ICON.plus, c: 4 },
    { n: 'Ajustes', ic: ICON.gear, c: 5 },
    { n: 'Salir', ic: ICON.power, c: 6 },
  ];

  const plats = $derived(app.lib.platforms);
  // Las plataformas se quedan en la mitad superior para dejar sitio a «Seguir jugando».
  const size = 190;
  // Con muchas plataformas el paso se encoge para que todas quepan en pantalla.
  const step = $derived(plats.length > 1 ? Math.min(plats.length > 3 ? 232 : 270, (1280 - 80 - size) / (plats.length - 1)) : 270);
  const x0 = $derived((1280 - (plats.length - 1) * step - size) / 2);
  const lw = $derived(Math.min(size + 40, step - 4));
  const totalTitles = $derived(plats.filter((p) => p.id !== 'sagas').reduce((n, p) => n + p.titles.length, 0));
  const count = (p: (typeof plats)[number]) => p.titles.reduce((s, t) => s + t.shots.length, 0);
  const wacc = $derived(app.acc(4));

  // «Seguir jugando»
  const CW = 350, CGAP = 34;
  const recent = $derived(app.recent.map((r) => ({ ...r, t: app.lib.platforms[r.pi].titles[r.ti] })));
  const cx0 = $derived((1280 - recent.length * CW - (recent.length - 1) * CGAP) / 2);
  const focusRec = $derived(app.focus.startsWith('rec') ? recent[+app.focus.slice(3)] : undefined);
  function meta(t: (typeof recent)[number]['t'], pname: string) {
    const when = t.lastPlayed ? fmtAgo(t.lastPlayed) : t.shots[0] ? `Captura del ${fmtShort(t.shots[0].date)}` : '';
    const rest = [t.playSeconds >= 60 ? fmtDur(t.playSeconds) : pname, t.shots.length ? nCapturas(t.shots.length) : ''].filter(Boolean);
    return { when, rest: rest.join(' · ') };
  }

  // Resumen de los últimos 7 días (minutos de las partidas y capturas con fecha en esos días).
  const week = $derived.by(() => {
    let min = 0, shots = 0;
    const from = dayKey(naiveNow() - 6 * 864e5);
    for (const p of plats) {
      if (p.id === 'sagas') continue;
      for (const t of p.titles) {
        if (t.hidden) continue;
        min += (t.week ?? []).reduce((a, b) => a + b, 0);
        for (const s of t.shots) { if (dayKey(s.date) >= from) shots++; else break; }
      }
    }
    return { min, shots };
  });
</script>

<TopBar crumbs={['Inicio']} />

{#if !app.config.root}
  <div class="glass welcome fx" class:f={app.focus === 'welcome'} data-k="welcome" data-act="welcome" style="left:240px;top:130px;--acc:{wacc.c}">
    <div class="face">
      <div class="wicon">{@html ICON.import}</div>
      <h2>Elige la carpeta de tu biblioteca</h2>
      <p>Indica la carpeta donde guardas tus capturas. Dentro, una carpeta por plataforma y otra por juego:</p>
      <pre>Biblioteca/
  PC/Elden Ring/captura1.png
  Switch/Zelda/captura1.jpg
  Novelas visuales/Steins;Gate/captura1.webp</pre>
      <p class="wgo"><span class="btn a">A</span> Elegir carpeta</p>
    </div>
  </div>
{:else}
  {#each plats as p, i}
    {@const a = app.platAcc(i)}
    {@const x = x0 + i * step}
    <div class="glass plat fx" class:f={app.focus === 'plat' + i} data-k="plat{i}" data-act="plat:{i}" style="left:{x}px;top:100px;width:{size}px;height:{size}px;--acc:{a.c};--pc:{a.c}">
      <div class="face">{@html ICON[p.icon] ?? ICON.pc}</div>
    </div>
    <div class="plabel hl" class:on={app.focus === 'plat' + i} style="left:{x + (size - lw) / 2}px;top:{112 + size}px;width:{lw}px;--pc:{a.c}">
      {p.name}<small>{p.id === 'sagas' ? `${p.titles.length} ${p.titles.length === 1 ? 'saga' : 'sagas'}` : nTitulos(p.titles.length)} · {nCapturas(count(p))}</small>
    </div>
  {/each}
  {#if totalTitles === 0 && !app.scanning}
    <div class="note pill" style="left:50%;top:84px">No encontré capturas en esta carpeta. Espero <b>Biblioteca/Plataforma/Juego/imagen.png</b>. Cambia la carpeta en Ajustes.</div>
  {/if}

  {#if recent.length}
    <div class="hsect" style="left:{cx0}px">Seguir jugando</div>
    {#if week.min > 0 || week.shots > 0}
      <div class="hweek" style="right:{cx0}px">Últimos 7 días: <b>{week.min > 0 ? fmtDur(week.min * 60) : 'sin partidas'}</b> · <b>{nCapturas(week.shots)}</b></div>
    {/if}
    {#each recent as r, i (r.t.id)}
      {@const x = cx0 + i * (CW + CGAP)}
      {@const m = meta(r.t, app.lib.platforms[r.pi].name)}
      <div class="glass rcard fx" class:f={app.focus === 'rec' + i} data-k="rec{i}" data-act="rec:{i}" style="left:{x}px;--acc:{app.titleAcc(r.pi, r.ti).c}">
        <RecentCard t={r.t} playing={!!r.t.gameId && app.playing?.id === r.t.gameId} />
      </div>
      <div class="rmeta" style="left:{x + 4}px">{m.when}{#if m.rest}<span>{m.when ? ' · ' : ''}{m.rest}</span>{/if}</div>
    {/each}
  {:else if totalTitles > 0}
    <div class="hsect" style="left:96px">Seguir jugando</div>
    <div class="rnone">Aquí aparecerán los juegos a los que juegues desde el visor y los que tengan capturas nuevas.</div>
  {/if}
{/if}

<div class="hdock">
  {#each DOCK as d, i}
    <div class="dpill pill fx" class:f={app.focus === 'dock' + i} data-k="dock{i}" data-act="dock:{i}" style="--dc:{RAINBOW[d.c].c}">
      <i>{@html d.ic}</i>{d.n}
    </div>
  {/each}
</div>

<Hints list={[['A', 'Abrir', 'a'], ...(focusRec?.t.launchable ? [['X', 'Jugar', 'x'] as [string, string, string]] : []), ['Start', 'Ajustes', 'menu']]} />
