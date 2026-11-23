<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
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
  let prev = $state<{ url: string; dir: number; k: number } | null>(null);
  let prevT: ReturnType<typeof setTimeout> | undefined;
  let slideKey = 0;
  $effect(() => {
    if (!cur) return;
    const url = backend.fullUrl(cur.path);
    const d = app.vdir;
    let dead = false;
    const im = new Image();
    im.src = url;
    (im.decode ? im.decode() : Promise.resolve()).catch(() => {}).then(() => {
      if (dead) return;
      const curShown = untrack(() => shown);
      if (curShown && curShown !== url && d !== 0) {
        clearTimeout(prevT);
        prev = { url: curShown, dir: d, k: ++slideKey };
        prevT = setTimeout(() => { prev = null; }, 230);
      } else {
        clearTimeout(prevT);
        prev = null;
      }
      dir = d;
      shown = url;
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
  });
  onDestroy(() => {
    clearTimeout(prevT);
    while (keep.length) {
      const evicted = keep.pop();
      if (evicted) evicted.src = '';
    }
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
        {#if prev && !zoomed}
          {#key prev.k}
            <img class="vprev {prev.dir > 0 ? 'out-l' : 'out-r'}" alt="" src={prev.url} draggable="false" />
          {/key}
        {/if}
        {#key shown}
          {#if shown}<img class={dir > 0 ? 'in-r' : dir < 0 ? 'in-l' : ''} alt="" src={shown} draggable="false" />{/if}
        {/key}
      </div>
    </div>
  </div>
  <div class="varrow" class:dis={app.s === 0} data-hact="left" style="left:30px">{@html ICON.chevL}</div>
  <div class="varrow" class:dis={app.s >= n - 1} data-hact="right" style="right:30px">{@html ICON.chevR}</div>
  {#if cur}
    <div class="vinfo pill" class:more={app.vinfoMore}>
      <span class="n">{app.s + 1} / {n}</span><span class="vt">{cur.game ? `${app.cur.name} · ${cur.game}` : app.cur.name}</span>
      <span class="dim">{fmt(cur.date)}</span>{#if app.vinfoMore}<span class="dim">{resText(cur.width, cur.height)}</span>{/if}
      <span class="vstar" class:on={app.favs.has(cur.path)} class:burst={app.favBurst.path === cur.path}>{@html ICON.starY}{#if app.favBurst.path === cur.path}<svg class="fsparks" viewBox="0 0 40 40" aria-hidden="true"><line x1="20" y1="2" x2="20" y2="8" /><line x1="35.6" y1="11" x2="30.4" y2="14" /><line x1="35.6" y1="29" x2="30.4" y2="26" /><line x1="20" y1="38" x2="20" y2="32" /><line x1="4.4" y1="29" x2="9.6" y2="26" /><line x1="4.4" y1="11" x2="9.6" y2="14" /></svg>{/if}</span>
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
    : [['LB/RB', '±10', 'rb'], ['X', 'Favorita', 'x'], ['Y', app.vinfoMore ? 'Menos datos' : 'Detalles', 'y'], ['View', app.vbare ? 'Marco' : 'Solo imagen', 'view'], ['Start', 'Menú', 'menu'], ['B', 'Volver', 'b']]} />
</div>
