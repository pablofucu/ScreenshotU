<script lang="ts">
  import { app, STATUS_LABEL, TITLE_FILTERS, type MenuItem, type TitleFilter } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { closeOverlay, MAGPIE_LABEL, menuAct, menuLayout } from '../lib/controller.svelte';
  import { fmtRelease } from '../lib/release.svelte';
  import Hints from '../components/Hints.svelte';
  import { tgrid } from '../lib/tgrid';
  import { makeMoved } from '../lib/pointer';

  const t = $derived(app.platform?.titles[app.menu.title]);
  const items = $derived(app.menu.items);
  const shots = $derived(app.menu.kind === 'shots');
  const list = $derived(app.menu.kind === 'titles');
  const fltName = (f: string) => TITLE_FILTERS.find(([k]) => k === f)?.[1] ?? f;
  /** Juegos que deja ver cada filtro (en el submenú «Filtrar»). */
  const fltCount = (f: TitleFilter) => {
    const ts = app.platform?.titles ?? [];
    if (f === 'launch') return ts.filter((x) => x.launchable).length;
    if (f === 'shots') return ts.filter((x) => !x.launchable).length;
    if (f === 'favs') return ts.filter((x) => x.shots.some((sh) => app.favs.has(sh.path))).length;
    return ts.length;
  };
  const audioOut = $derived(app.audioOutputs.find((o) => o.default)?.short ?? '');
  // Alto del panel según las acciones; centrado en el escenario.
  // Con 14 o más acciones el paso se reduce para que el panel (máx. ~704 px) quepa en los 720 px del escenario.
  // La ficha del juego va por secciones (con título) y, si son muchas acciones, en dos columnas; el resto es una lista plana.
  const HH = 34; // alto de un título de sección
  const lay = $derived(menuLayout());
  const cols = $derived(lay.length);
  const grouped = $derived(lay.some((c) => c.some((g) => g.title)));
  const per = $derived(Math.max(...lay.map((c) => c.reduce((n, g) => n + g.items.length, 0))));
  const colRows = $derived(lay.map((c) => c.reduce((n, g) => n + g.items.length, 0)));
  const colHeads = $derived(lay.map((c) => c.filter((g) => g.title).length));
  /** Posición, en la lista plana de acciones, de la primera de la sección gi de la columna ci. */
  const base = (ci: number, gi: number) =>
    colRows.slice(0, ci).reduce((a, b) => a + b, 0) + lay[ci].slice(0, gi).reduce((n, g) => n + g.items.length, 0);
  const pitch = $derived(grouped
    ? Math.min(54, ...colRows.map((r, k) => Math.floor((558 - colHeads[k] * HH) / r)))
    : Math.min(per > 11 ? 44 : per > 9 ? 50 : 58, Math.floor(558 / per)));
  const squeeze = $derived(pitch < 44);
  const H = $derived(28 + 100 + Math.max(...colRows.map((r, k) => r * pitch + colHeads[k] * HH)) + 18);
  const sel = $derived(app.sel.size);
  const label = (it: MenuItem) => {
    switch (it) {
      case 'folder': return 'Abrir carpeta';
      case 'reload': return 'Recargar grafismos';
      case 'addshots': return 'Añadir capturas';
      case 'accent': return 'Color de acento';
      case 'hide': return 'Ocultar juego';
      case 'unhide': return 'Mostrar juego';
      case 'remove': return app.menu.confirm ? 'Pulsa A otra vez para quitar' : 'Quitar de la lista';
      case 'select': return 'Seleccionar';
      case 'unselect': return 'Salir de la selección';
      case 'selall': return 'Marcar todas';
      case 'move': return app.selMode && sel ? `Mover ${sel} a otro juego…` : 'Mover a otro juego…';
      case 'reveal': return 'Mostrar en el Explorador';
      case 'delshot': return app.menu.confirm ? 'Pulsa A otra vez: a la Papelera' : 'Borrar captura';
      case 'delsel': return app.menu.confirm ? `Pulsa A otra vez: ${sel} a la Papelera` : `Borrar las ${sel} seleccionadas`;
      case 'addcoll': return 'Añadir a una colección…';
      case 'sgdb': return 'Grafismos (SteamGridDB o del PC)…';
      case 'phrases': return 'Frases y repaso…';
      case 'saves': return 'Partidas guardadas…';
      case 'setexe': return 'Cambiar ejecutable…';
      case 'setdir': return 'Cambiar carpeta de trabajo…';
      case 'magpie': return magpie.label;
      case 'status': return `Estado: ${STATUS_LABEL[t?.status ?? 'none']}`;
      case 'release': return t?.release ? `Fecha de lanzamiento: ${fmtRelease(t.release)}` : 'Fecha de lanzamiento…';
      case 'category': return 'Cambiar de categoría…';
      case 'filter': return app.titleFilter === 'all' ? 'Filtrar…' : `Filtrar: ${fltName(app.titleFilter)}`;
      case 'audio': return audioOut ? `Salida de audio: ${audioOut}` : 'Salida de audio…';
      case 'settings': return 'Ajustes';
      case 'gstats': return 'Estadísticas de la categoría';
      case 'play': return 'Iniciar juego';
      case 'listopts': return 'Opciones de la lista…';
      case 'copy': return 'Copiar captura';
      case 'tsize': return `Portadas: ${tgrid(app.config.titlesZoom).cols} por fila`;
      default:
        if (it.startsWith('flt:')) return fltName(it.slice(4)); return `Mover a ${app.config.platforms.find((p) => p.id === it.slice(4))?.name ?? it.slice(4)}`;
    }
  };
  /** Fila de Magpie: modo elegido y, debajo, lo ultimo aplicado. */
  const magpie = $derived.by(() => {
    void app.magpieTick;
    const m = t?.magpieMode ?? 'off';
    return { label: `Magpie: ${MAGPIE_LABEL[m]}`, detail: m !== 'off' ? (t?.magpieSummary ?? 'Se configura al lanzarlo') : '' };
  });
  const icon = (it: MenuItem) => it.startsWith('flt:') ? (app.titleFilter === it.slice(4) ? ICON.check : '') : it === 'category' || it.startsWith('cat:') ? ICON.move : ({
    folder: ICON.folder, reload: ICON.reload, remove: ICON.power, addshots: ICON.image, accent: ICON.palette, hide: ICON.eyeOff,
    unhide: ICON.eye, setexe: ICON.folder, setdir: ICON.folder, magpie: ICON.image, select: ICON.check, unselect: ICON.check, selall: ICON.checks, move: ICON.move, reveal: ICON.folder, delshot: ICON.trash, delsel: ICON.trash, addcoll: ICON.saga, sgdb: ICON.search, phrases: ICON.jvn, status: ICON.checks, release: ICON.calendar, saves: ICON.import, filter: ICON.search, tsize: ICON.image, audio: ICON.speaker, settings: ICON.gear, gstats: ICON.checks, play: ICON.play, listopts: ICON.gear, copy: ICON.camera,
  } as Record<string, string>)[it];
  const sub = $derived(
    list ? (app.menu.items[0]?.startsWith('flt:') ? 'Qué juegos se ven en «Todos»' : [app.platform?.name, t?.name].filter(Boolean).join(' · '))
      : shots ? (app.selMode ? `${sel} ${sel === 1 ? 'seleccionada' : 'seleccionadas'}` : (app.cur.name || '')) : (t?.name ?? ''),
  );

  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { app.menu.row = i; app.menu.confirm = false; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="glass menupanel" class:tight={!grouped && per > 9} class:tighter={!grouped && per > 11} class:squeeze={squeeze || grouped} class:two={cols > 1} style="--acc:{app.chrome.c};--mrh:{pitch - 4}px;top:{(720 - H) / 2}px;height:{H}px">
    <div class="face">
      <h2>{list ? (app.menu.items[0]?.startsWith('flt:') ? 'Filtrar' : (t ? t.name : 'Lista de juegos')) : shots ? 'Capturas' : 'Grafismos y opciones'}</h2>
      <p class="msub">{sub}</p>
      <div class="mgrid" style="grid-template-columns:repeat({cols},minmax(0,1fr))">
      {#each lay as col, ci}
        <div class="mcol">
        {#each col as g, gi}
          {#if g.title}<h3 class="mhead">{g.title}</h3>{/if}
          {#each g.items as it, k}
            {@const i = base(ci, gi) + k}
            <button type="button" class="srow mrow" class:f={app.menu.row === i} class:danger={it === 'remove' || it === 'delshot' || it === 'delsel'} class:addrow={it === 'magpie' && !!magpie.detail} data-item={it} onmousemove={(e) => hover(e, i)} onclick={() => { app.menu.row = i; void menuAct(it); }}>
              <span class="mic">{@html icon(it)}</span>
              <span class="sl">{label(it)}{#if it === 'magpie' && magpie.detail}<small>{magpie.detail}</small>{/if}</span>
              {#if it.startsWith('flt:')}<span class="mcount">{fltCount(it.slice(4) as TitleFilter)}</span>{/if}
            </button>
          {/each}
        {/each}
        </div>
      {/each}
      </div>
    </div>
  </div>
  <Hints list={[['A', 'Seleccionar', 'a'], ['B', 'Cerrar', 'b']]} />
</div>

<style>
  /* Con muchas acciones (juego .exe con todo) las filas se aprietan un poco más para que el panel quepa. */
  .menupanel.tighter .mrow{height:40px}
  .menupanel.tighter .mrow.addrow{height:48px}
  .menupanel.squeeze .mrow{height:var(--mrh)}
  .menupanel.squeeze .mrow.addrow{height:calc(var(--mrh) + 8px)}
  .mgrid { display: grid; column-gap: 18px; }
  .mcol { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  /* Dos columnas (ficha del juego): primero la izquierda, de arriba abajo. */
  .menupanel.two { left: 210px; width: 860px; }
  .mhead { margin: 0; height: 26px; line-height: 26px; padding: 0 4px; font-size: 13px; font-weight: 800; letter-spacing: 0.09em; text-transform: uppercase; color: var(--ink-3); border-bottom: 2px solid color-mix(in srgb, var(--acc) 35%, transparent); }
  .mcol > .mhead:not(:first-child) { margin-top: 4px; }
  .mcount { margin-left: auto; font-size: 15px; font-weight: 700; color: var(--ink-3); }
  .f .mcount { color: inherit; opacity: 0.85; }
  /* Textos largos (p. ej. el nombre de una salida de audio) se cortan con puntos suspensivos. */
  .mrow .sl { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
</style>
