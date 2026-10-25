<script lang="ts">
  import { untrack } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { resetZoom } from '../lib/controller.svelte';
  import { ICON } from '../lib/icons';
  import { backend, decodeImg, thumbNow } from '../lib/backend';
  import { fmt, resText } from '../lib/format';
  import Hints from '../components/Hints.svelte';
  import Thumb from '../components/Thumb.svelte';

  const shots = $derived(app.cur.shots);
  const n = $derived(shots.length);
  const cur = $derived(shots[app.s]);
  const first = $derived(Math.max(0, app.s - 14));
  const last = $derived(Math.min(n, app.s + 15));
  const strip = $derived(Array.from({ length: Math.max(0, last - first) }, (_, k) => first + k));

  // La imagen nueva solo reemplaza a la anterior cuando ya esta decodificada (sin parpadeo).
  let shown = $state('');
  let dir = $state(0);
  $effect(() => {
    if (!cur) return;
    const url = backend.fullUrl(cur.path);
    const d = app.vdir;
    let dead = false;
    const im = new Image();
    im.src = url;
    (im.decode ? im.decode() : Promise.resolve()).catch(() => {}).then(() => {
      if (!dead) { dir = d; shown = url; }
    });
    return () => { dead = true; };
  });

  // Precarga de vecinas (se guardan referencias para que el navegador no las descarte).
  const keep: HTMLImageElement[] = [];
  $effect(() => {
    for (const i of [app.s + 1, app.s - 1, app.s + 2]) {
      if (i >= 0 && i < n) {
        const im = new Image();
        im.src = backend.fullUrl(shots[i].path);
        keep.push(im);
        if (keep.length > 8) {
          const evicted = keep.shift();
          if (evicted) evicted.src = '';
        }
      }
    }
    return () => {
      while (keep.length) {
        const evicted = keep.pop();
        if (evicted) evicted.src = '';
      }
    };
  });

  // Al cambiar de captura el zoom vuelve al 100 %.
  $effect(() => {
    void cur?.path;
    untrack(resetZoom);
  });

  const zoomed = $derived(app.vz.k > 1);

  // Fondo desenfocado: cambia cuando la miniatura nueva ya está decodificada (la anterior se queda hasta entonces).
  let bg = $state(untrack(() => (cur ? (thumbNow(cur.path) ?? '') : '')));
  $effect(() => {
    if (!cur) return;
    const ctl = new AbortController();
    backend.thumb(cur.path, ctl.signal).then(async (u) => { await decodeImg(u); if (!ctl.signal.aborted) bg = u; }, () => {});
    return () => ctl.abort();
  });
</script>

<div class="vw" class:idle={app.idle} class:bare={app.vbare}>
  <div class="vbg" style={bg ? `background-image:url(${bg})` : ''}></div>
  <div class="glass vframe">
    <div class="face">
      <div class="vzoom" style={zoomed ? `transform:translate(${app.vz.x}px,${app.vz.y}px) scale(${app.vz.k})` : ''}>
        {#key shown}
          {#if shown}<img class={dir > 0 ? 'in-r' : dir < 0 ? 'in-l' : ''} alt="" src={shown} draggable="false" />{/if}
        {/key}
      </div>
    </div>
  </div>
  <div class="varrow" class:dis={app.s === 0} data-hact="left" style="left:30px">{@html ICON.chevL}</div>
  <div class="varrow" class:dis={app.s >= n - 1} data-hact="right" style="right:30px">{@html ICON.chevR}</div>
  {#if cur}
    <div class="vinfo pill">
      <span class="n">{app.s + 1} / {n}</span><span class="vt">{cur.game ? `${app.cur.name} · ${cur.game}` : app.cur.name}</span>
      <span class="dim">{fmt(cur.date)}</span><span class="dim">{resText(cur.width, cur.height)}</span>
      <span class="vstar" class:on={app.favs.has(cur.path)}>{@html ICON.starY}</span>
    </div>
  {/if}
  <div class="vstrip">
    <div class="vtrack" style="transform:translateX({640 - 52 - app.s * 118}px)">
      {#each strip as i (shots[i].path)}
        <div class="glass vthumb" class:f={i === app.s} data-vi={i} style="left:{i * 118}px"><div class="face"><Thumb path={shots[i].path} /></div></div>
      {/each}
    </div>
  </div>
  <!-- Con zoom hay menos pistas para no tapar los datos de la captura. -->
  <Hints list={zoomed
    ? [['Stick I', 'Mover'], ['Stick D', `Zoom ${Math.round(app.vz.k * 100)} %`], ['View', 'Solo imagen', 'view'], ['B', '100 %', 'b']]
    : [['LB/RB', '±10', 'rb'], ['Stick D', 'Zoom'], ['X', 'Favorita', 'x'], ['View', 'Solo imagen', 'view'], ['Start', 'Menú', 'menu'], ['B', 'Volver', 'b']]} />
</div>
