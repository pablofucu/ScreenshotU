<script lang="ts">
  import { app, STATUS_LABEL, type MenuItem } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { closeOverlay, MAGPIE_LABEL, menuAct } from '../lib/controller.svelte';
  import Hints from '../components/Hints.svelte';

  const t = $derived(app.platform?.titles[app.menu.title]);
  const items = $derived(app.menu.items);
  const shots = $derived(app.menu.kind === 'shots');
  // Alto del panel según las acciones; centrado en el escenario.
  const pitch = $derived(items.length > 9 ? 50 : 58);
  const H = $derived(28 + 100 + items.length * pitch + 18);
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
      case 'editsaga': return 'Editar saga';
      case 'delsaga': return app.menu.confirm ? 'Pulsa A otra vez para disolver' : 'Disolver saga';
      case 'sgdb': return 'Grafismos (SteamGridDB o del PC)…';
      case 'phrases': return 'Frases y repaso…';
      case 'saves': return 'Partidas guardadas…';
      case 'magpie': return magpie.label;
      case 'status': return `Estado: ${STATUS_LABEL[t?.status ?? 'none']}`;
      case 'category': return 'Cambiar de categoría…';
      default: return `Mover a ${app.config.platforms.find((p) => p.id === it.slice(4))?.name ?? it.slice(4)}`;
    }
  };
  /** Fila de Magpie: modo elegido y, debajo, lo ultimo aplicado. */
  const magpie = $derived.by(() => {
    void app.magpieTick;
    const m = t?.magpieMode ?? 'off';
    return { label: `Magpie: ${MAGPIE_LABEL[m]}`, detail: m !== 'off' ? (t?.magpieSummary ?? 'Se configura al lanzarlo') : '' };
  });
  const icon = (it: MenuItem) => it === 'category' || it.startsWith('cat:') ? ICON.move : ({
    folder: ICON.folder, reload: ICON.reload, remove: ICON.power, addshots: ICON.image, accent: ICON.palette, hide: ICON.eyeOff,
    unhide: ICON.eye, magpie: ICON.image, select: ICON.check, unselect: ICON.check, selall: ICON.checks, move: ICON.move, reveal: ICON.folder, editsaga: ICON.palette, delsaga: ICON.power, sgdb: ICON.search, phrases: ICON.jvn, status: ICON.checks, saves: ICON.import,
  } as Record<string, string>)[it];
  const sub = $derived(
    shots ? (app.selMode ? `${sel} ${sel === 1 ? 'seleccionada' : 'seleccionadas'}` : (app.cur.name || '')) : (t?.name ?? ''),
  );

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; app.menu.row = i; app.menu.confirm = false; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="glass menupanel" class:tight={items.length > 9} style="--acc:{app.chrome.c};top:{(720 - H) / 2}px;height:{H}px">
    <div class="face">
      <h2>{shots ? 'Capturas' : t?.members ? 'Saga' : 'Grafismos y opciones'}</h2>
      <p class="msub">{sub}</p>
      {#each items as it, i}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="srow mrow" class:f={app.menu.row === i} class:danger={it === 'remove' || it === 'delsaga'} class:addrow={it === 'magpie' && !!magpie.detail} data-item={it} onmousemove={(e) => hover(e, i)} onclick={() => { app.menu.row = i; void menuAct(it); }}>
          <span class="mic">{@html icon(it)}</span>
          <span class="sl">{label(it)}{#if it === 'magpie' && magpie.detail}<small>{magpie.detail}</small>{/if}</span>
        </div>
      {/each}
    </div>
  </div>
  <Hints list={[['A', 'Seleccionar', 'a'], ['B', 'Cerrar', 'b']]} />
</div>
