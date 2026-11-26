<script lang="ts">
  import { app, LIST_COLOR, STATUS_LABEL, type MenuItem } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtDur } from '../lib/format';
  import { closeOverlay, closePrompt, MAGPIE_LABEL, MAGPIE_ORDER } from '../lib/controller.svelte';
  import {
    copying, cycleMagpie, gameSetAct, gameSetListAct, gameTabItems,
    otherPlatforms, selectTab, TAB_HELP, visibleTabs,
  } from '../lib/gameset.svelte';
  import { platformCollections } from '../lib/coleccion.svelte';
  import { fmtRelease } from '../lib/release.svelte';
  import Hints from '../components/Hints.svelte';
  import MiniCover from '../components/MiniCover.svelte';
  import { makeMoved } from '../lib/pointer';

  const g = $derived(app.gameSet);
  const t = $derived(app.platform?.titles[g.title]);
  const tabs = $derived(visibleTabs(t).map(([k, name, ico]) => ({ k, name, ico })));
  const tabIdx = $derived(Math.max(0, tabs.findIndex((tb) => tb.k === g.tab)));
  const rows = $derived(gameTabItems(t, g.tab));
  const tabName = $derived(tabs.find((tb) => tb.k === g.tab)?.name ?? '');
  const magpie = $derived.by(() => {
    void app.magpieTick;
    const m = t?.magpieMode ?? 'off';
    return { mode: m, label: MAGPIE_LABEL[m], detail: m !== 'off' ? (t?.magpieSummary ?? 'Se configura al lanzarlo') : 'Sin escalado adicional' };
  });
  const inColls = $derived(t?.id ? platformCollections().filter((c) => c.members.includes(t.id!)).length : 0);
  const curStatus = $derived(t?.status ?? 'none');

  interface RowInfo {
    l: string;
    s?: string;
    v?: string;
    help: string;
    kind?: 'chev' | 'cycle' | 'status' | 'accent';
    danger?: boolean;
  }

  /** Etiqueta, línea secundaria, valor derecho y ayuda contextual de cada fila. */
  function info(it: MenuItem): RowInfo {
    switch (it) {
      case 'status': return {
        l: 'Estado en Mi lista',
        s: 'Organiza el juego dentro de la categoría',
        v: STATUS_LABEL[curStatus],
        kind: 'status',
        help: 'Marca si lo estás jugando, lo tienes pendiente, terminado o aparcado.',
      };
      case 'addcoll': return {
        l: 'Colecciones…',
        s: 'Agrupa juegos relacionados sin mezclar categorías',
        v: inColls ? `En ${inColls}` : 'Ninguna',
        kind: 'chev',
        help: 'Añade o quita este juego de tus colecciones dentro de esta categoría.',
      };
      case 'release': return {
        l: 'Fecha de lanzamiento…',
        s: 'Año y mes para ordenar en la biblioteca',
        v: t?.release ? fmtRelease(t.release) : 'Sin fecha',
        kind: 'chev',
        help: 'Sirve para ordenar cronológicamente la lista y el resumen anual.',
      };
      case 'sgdb': return {
        l: 'Imágenes del juego…',
        s: 'Portada, fondo, logo e icono',
        kind: 'chev',
        help: 'Descarga arte desde SteamGridDB o elige archivos locales de este PC.',
      };
      case 'accent': return {
        l: 'Color del juego…',
        s: 'Tiñe su ficha, paneles y estadísticas',
        v: t?.accentOverride ? 'Manual' : 'Automático',
        kind: 'accent',
        help: 'El cambio de color se refleja al instante en la ficha visible a la izquierda.',
      };
      case 'closegame': return g.confirm
        ? { l: closePrompt.force ? 'Pulsa A otra vez para forzar' : 'Pulsa A otra vez para cerrar', danger: true, help: 'Cierra el proceso del juego abierto actualmente.' }
        : { l: 'Cerrar juego', s: 'Termina la partida en el PC principal', help: 'Detiene la partida en curso desde el visor.' };
      case 'magpie': return {
        l: 'Escalado con Magpie',
        s: magpie.detail,
        v: magpie.label,
        kind: 'cycle',
        help: 'Pulsa ‹ › para cambiar el modo al instante, o A para ver la descripción de cada uno.',
      };
      case 'setexe': return {
        l: 'Ejecutable…',
        s: 'Cambia el archivo que se lanza al pulsar Jugar',
        kind: 'chev',
        help: 'Elige otro ejecutable (.exe) para abrir este juego.',
      };
      case 'setdir': return {
        l: 'Carpeta de trabajo…',
        s: 'Directorio base al iniciar el ejecutable',
        kind: 'chev',
        help: 'Útil si el juego espera ejecutarse desde una subcarpeta concreta.',
      };
      case 'copyhere': return copying.id
        ? { l: 'Cancelar la copia', s: `${copying.name} · ${copying.total ? Math.floor((copying.done / copying.total) * 100) : 0} %`, help: 'Interrumpe la descarga; lo copiado se reanuda más adelante.' }
        : { l: 'Copiar a este equipo…', s: 'Descarga el juego del PC principal para jugarlo aquí', kind: 'chev', help: 'Trae los archivos del juego a este PC cliente.' };
      case 'folder': return {
        l: 'Abrir carpeta de capturas',
        s: 'Muestra la carpeta del juego en el explorador',
        help: 'Abre directamente el directorio donde se guardan las capturas.',
      };
      case 'addshots': return {
        l: 'Añadir capturas…',
        s: 'Importa imágenes sueltas a la galería del juego',
        kind: 'chev',
        help: 'Copia capturas externas dentro de la carpeta del juego.',
      };
      case 'saves': return {
        l: 'Partidas guardadas…',
        s: 'Copias manuales y automáticas de Ludusavi',
        kind: 'chev',
        help: 'Consulta, restaura o crea respaldos de las partidas guardadas.',
      };
      case 'phrases': return {
        l: 'Frases y repaso…',
        s: 'Glosario guardado desde el lector japonés',
        kind: 'chev',
        help: 'Repasa las frases y palabras capturadas durante la lectura.',
      };
      case 'reload': return {
        l: 'Volver a cargar las imágenes',
        s: 'Refresca portada, hero y logo desde el disco',
        help: 'Úsalo si cambiaste archivos de grafismos a mano fuera del visor.',
      };
      case 'category': return {
        l: 'Mover a otra categoría…',
        s: `Actual: ${app.platform?.name ?? ''}`,
        kind: 'chev',
        help: 'Mueve el juego y su carpeta de capturas a otra plataforma o categoría.',
      };
      case 'hide': return {
        l: 'Ocultar de la biblioteca',
        s: 'Lo oculta de las listas sin borrar sus archivos',
        help: 'Puedes volver a mostrarlo en cualquier momento desde Ajustes generales.',
      };
      case 'unhide': return {
        l: 'Volver a mostrar en la biblioteca',
        s: 'Devuelve este juego a sus listas habituales',
        help: 'Quita la marca de oculto de este título.',
      };
      case 'remove': return g.confirm
        ? { l: 'Pulsa A otra vez para quitar', s: 'Las capturas del disco no se borran', danger: true, help: 'Confirma para retirar la ficha de la biblioteca.' }
        : { l: 'Quitar de la biblioteca', s: 'Pide confirmación · las capturas no se borran', danger: true, help: 'Elimina la ficha del visor manteniendo intacta tu carpeta de capturas.' };
      default: return { l: it, help: TAB_HELP[g.tab] };
    }
  }

  const icon = (it: MenuItem) => ({
    sgdb: ICON.search,
    accent: ICON.palette,
    reload: ICON.reload,
    closegame: ICON.power,
    magpie: ICON.scale,
    setexe: ICON.folder,
    setdir: ICON.folder,
    copyhere: ICON.import,
    folder: ICON.folder,
    addshots: ICON.image,
    saves: ICON.import,
    phrases: ICON.book,
    addcoll: ICON.collections,
    status: ICON.checks,
    release: ICON.calendar,
    category: ICON.move,
    hide: ICON.eyeOff,
    unhide: ICON.eye,
    remove: ICON.trash,
  } as Record<string, string>)[it] ?? '';

  const MAGPIE_DESC: Record<string, string> = {
    off: 'Abre el juego con su tamaño normal sin usar Magpie',
    auto: 'Mide la ventana al lanzar y elige 2x o 4x',
    '2x': 'Escala siempre al doble de resolución',
    '4x': 'Escala siempre al cuádruple de resolución',
  };

  const listItems = $derived(g.list === 'magpie'
    ? MAGPIE_ORDER.map((m) => ({ l: MAGPIE_LABEL[m], s: MAGPIE_DESC[m] ?? '', on: m === magpie.mode }))
    : otherPlatforms().map((p) => ({ l: `Mover a ${p.name}`, s: 'Conserva partidas, capturas y estado', on: false })));
  const listTitle = $derived(g.list === 'magpie' ? 'Escalado con Magpie' : 'Mover a otra categoría');

  const focusedItem = $derived(rows[g.row]);
  const focusedInfo = $derived(focusedItem ? info(focusedItem) : null);
  const focusedDanger = $derived(
    !g.list && (focusedItem === 'remove' || (focusedItem === 'closegame' && g.confirm)),
  );
  const hintsList = $derived<Array<[string, string, string?]>>(
    g.list
      ? [['A', 'Elegir', 'a'], ['B', 'Volver', 'b']]
      : focusedItem === 'magpie'
        ? [['←→', 'Modo'], ['LB/RB', 'Pestaña', 'rb'], ['A', 'Opciones', 'a'], ['B', 'Cerrar', 'b']]
        : [['LB/RB', 'Pestaña', 'rb'], ['A', g.confirm ? 'Confirmar' : 'Elegir', 'a'], ['B', 'Cerrar', 'b']],
  );

  const moved = makeMoved();
  function hoverRow(e: MouseEvent, i: number) {
    if (moved(e)) { g.inRows = true; g.row = i; g.confirm = false; }
  }
  function hoverList(e: MouseEvent, i: number) {
    if (moved(e)) g.lrow = i;
  }
</script>

<div class="scrim gs-scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="glass gs-sheet">
    <div class="face">
      <div class="gs-head">
        {#if t}
          <div class="gs-cov"><MiniCover {t} acc={app.chrome.c} /></div>
        {/if}
        <div class="gs-title-blk">
          <span class="gs-kicker">Ajustes del juego</span>
          <h2>{t?.name ?? ''}</h2>
          <p class="gs-meta">
            <span>{app.platform?.name ?? ''}</span>
            {#if t?.playSeconds}<i></i><span>{fmtDur(t.playSeconds)}</span>{/if}
          </p>
        </div>
        {#if curStatus !== 'none'}
          <span class="gs-stpill" style="--sc:{LIST_COLOR[curStatus].c}">{STATUS_LABEL[curStatus]}</span>
        {/if}
      </div>

      {#if g.list}
        <button type="button" class="gs-back" onclick={() => { g.list = ''; }}>
          {@html ICON.chevL}<span>{tabName}</span>
        </button>
        <h3 class="gs-subtit">{listTitle}</h3>
        <div class="gs-rows">
          <span class="gs-rslide" style="transform:translateY({g.lrow * 64}px)" aria-hidden="true"></span>
          {#each listItems as it, i}
            <button type="button" class="gs-row glass-in" class:f={g.lrow === i}
              onmousemove={(e) => hoverList(e, i)} onclick={() => { g.lrow = i; gameSetListAct(i); }}>
              <span class="gs-ico">{@html it.on ? ICON.check : (g.list === 'magpie' ? ICON.scale : ICON.move)}</span>
              <span class="gs-txt">
                <span class="gs-lbl">{it.l}</span>
                {#if it.s}<span class="gs-sub">{it.s}</span>{/if}
              </span>
              {#if g.list === 'category'}<span class="gs-chev">{@html ICON.chevR}</span>{/if}
            </button>
          {/each}
        </div>
      {:else}
        {#if tabs.length > 1}
          <div class="gs-tabs" style="--n:{tabs.length};--ti:{tabIdx}">
            <span class="gs-tslide" aria-hidden="true"></span>
            {#each tabs as tb (tb.k)}
              <button type="button" class="gs-tab" class:on={g.tab === tb.k}
                onclick={() => selectTab(tb.k)}>
                <span class="gs-tico">{@html tb.ico}</span>
                <span>{tb.name}</span>
              </button>
            {/each}
          </div>
        {/if}

        <div class="gs-rows">
          {#if rows.length}
            <span class="gs-rslide" class:danger={focusedDanger} style="transform:translateY({g.row * 64}px)" aria-hidden="true"></span>
          {/if}
          {#each rows as it, i (it)}
            {@const d = info(it)}
            <button type="button" class="gs-row glass-in" class:f={g.row === i} class:danger={!!d.danger} data-item={it}
              onmousemove={(e) => hoverRow(e, i)} onclick={() => { g.row = i; gameSetAct(it); }}>
              <span class="gs-ico">{@html icon(it)}</span>
              <span class="gs-txt">
                <span class="gs-lbl">{d.l}</span>
                {#if d.s}<span class="gs-sub">{d.s}</span>{/if}
              </span>
              {#if d.kind === 'cycle'}
                <span class="gs-val">
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <span class="gs-arr" onclick={(e) => { e.stopPropagation(); g.row = i; cycleMagpie(-1); }}>‹</span>
                  <span>{d.v}</span>
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <span class="gs-arr" onclick={(e) => { e.stopPropagation(); g.row = i; cycleMagpie(1); }}>›</span>
                </span>
              {:else if d.kind === 'status'}
                <span class="gs-val">
                  <span class="gs-stdot" style="background:{LIST_COLOR[curStatus].c}"></span>
                  <span>{d.v}</span>
                  <span class="gs-chev">{@html ICON.chevR}</span>
                </span>
              {:else if d.kind === 'accent'}
                <span class="gs-val">
                  <span class="gs-swatch"></span>
                  <span>{d.v}</span>
                  <span class="gs-chev">{@html ICON.chevR}</span>
                </span>
              {:else if d.kind === 'chev'}
                <span class="gs-val">
                  {#if d.v}<span>{d.v}</span>{/if}
                  <span class="gs-chev">{@html ICON.chevR}</span>
                </span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}

      <div class="gs-ctx glass-in">
        <b>{g.list ? listTitle : (focusedInfo ? `${tabName} · ${focusedInfo.l.replace('…', '')}` : tabName)}</b>
        <p>{g.list ? (g.list === 'magpie' ? 'Pulsa A para aplicar el modo o B para volver sin cambios.' : 'El juego y sus capturas pasarán a la categoría elegida.') : (focusedInfo?.help ?? TAB_HELP[g.tab])}</p>
      </div>
    </div>
  </div>
  <Hints list={hintsList} />
</div>

<style>
  /* Velo lateral: oscurece detrás de la hoja derecha dejando visible la ficha del juego a la izquierda */
  .gs-scrim {
    background: linear-gradient(90deg, color-mix(in srgb, var(--scrim) 18%, transparent) 0%, color-mix(in srgb, var(--scrim) 64%, transparent) 44%, var(--scrim) 64%);
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
  }
  .gs-sheet {
    right: var(--gut);
    top: var(--gut);
    width: 600px;
    height: calc(var(--content-end) - var(--gut));
    --r: var(--r-l);
    --bw: var(--s-3);
    cursor: default;
    box-shadow: inset 0 1.5px 0 var(--hl), inset 0 0 0 1px var(--hl-ring), inset 0 -10px 14px -8px var(--hl-bot), inset 0 -1.5px 0 var(--line), var(--sh-3);
    animation: gsSheetIn var(--d-3) var(--e-soft);
  }
  @keyframes gsSheetIn {
    from { opacity: 0; transform: translateX(var(--s-6)); }
  }
  .gs-sheet > .face {
    background: var(--panel);
    color: var(--ink);
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    padding: var(--s-5);
  }

  /* Cabecera del juego */
  .gs-head {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    min-width: 0;
  }
  .gs-cov {
    flex: none;
  }
  .gs-cov :global(.mcov) {
    width: 42px;
    height: 60px;
    border-radius: var(--r-s);
  }
  .gs-title-blk {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .gs-kicker {
    font: 800 var(--t-xs)/1.2 var(--round);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .gs-title-blk h2 {
    margin: 0;
    font: 800 var(--t-xl)/1.2 var(--round);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .gs-meta {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    font: 500 var(--t-s)/1.2 var(--round);
    color: var(--ink-2);
  }
  .gs-meta i {
    width: var(--s-1);
    height: var(--s-1);
    border-radius: var(--r-pill);
    background: var(--ink-3);
  }
  .gs-stpill {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 30px;
    padding: 0 var(--s-3);
    border-radius: var(--r-pill);
    font: 700 var(--t-s)/1 var(--round);
    color: var(--on);
    background: linear-gradient(180deg, color-mix(in srgb, var(--sc) 70%, var(--tint-base)), var(--sc));
    box-shadow: inset 0 1px 0 var(--hl-ring), var(--sh-1);
  }

  /* Pestañas segmentadas */
  .gs-tabs {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    padding: var(--s-1);
    border-radius: var(--r-m);
    background: var(--glass-in);
    box-shadow: inset 0 0 0 1px var(--line);
    flex: none;
  }
  .gs-tslide {
    position: absolute;
    left: var(--s-1);
    top: var(--s-1);
    bottom: var(--s-1);
    width: calc((100% - var(--s-2)) / var(--n));
    border-radius: var(--r-s);
    background: var(--gf);
    box-shadow: inset 0 1px 0 var(--hl-ring), var(--sh-1);
    transform: translateX(calc(var(--ti) * 100%));
    transition: transform var(--d-2) var(--e-bounce);
    pointer-events: none;
    z-index: 0;
  }
  .gs-tab {
    position: relative;
    z-index: 1;
    appearance: none;
    border: 0;
    background: transparent;
    height: 44px;
    padding: 0 var(--s-1);
    border-radius: var(--r-s);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    font: 700 var(--t-s)/1 var(--round);
    color: var(--ink-2);
    cursor: pointer;
    transition: color var(--d-1) var(--e-soft);
  }
  .gs-tab.on {
    color: var(--on);
  }
  .gs-tico {
    width: var(--i-s);
    height: var(--i-s);
    display: grid;
    place-items: center;
    flex: none;
  }
  .gs-tico :global(svg) {
    width: 100%;
    height: 100%;
  }

  /* Subvista */
  .gs-back {
    align-self: flex-start;
    appearance: none;
    border: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: 34px;
    padding: 0 var(--s-3);
    border-radius: var(--r-pill);
    background: var(--glass-in);
    box-shadow: inset 0 0 0 1px var(--line);
    font: 700 var(--t-s)/1 var(--round);
    color: var(--ink);
    cursor: pointer;
  }
  .gs-back :global(svg) {
    width: var(--i-s);
    height: var(--i-s);
  }
  .gs-subtit {
    margin: 0;
    font: 800 var(--t-xl)/1.2 var(--round);
    color: var(--ink);
  }

  /* Filas y riel deslizante */
  .gs-rows {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .gs-rslide {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 58px;
    border-radius: var(--r-m);
    background: var(--gf);
    box-shadow: inset 0 1.5px 0 var(--hl-ring), var(--ring);
    pointer-events: none;
    z-index: 0;
    transition: transform var(--d-2) var(--e-bounce), background var(--d-1) var(--e-soft);
  }
  .gs-rslide.danger {
    background: linear-gradient(115deg, var(--bad-f), var(--bad));
    box-shadow: inset 0 1.5px 0 var(--hl-ring), 0 0 0 3px var(--bad);
  }

  .gs-row {
    position: relative;
    z-index: 1;
    appearance: none;
    border: 0;
    width: 100%;
    height: 58px;
    margin-bottom: 6px;
    padding: 0 var(--s-4);
    display: flex;
    align-items: center;
    gap: var(--s-3);
    text-align: left;
    cursor: pointer;
    color: var(--ink);
    transition: color var(--d-1) var(--e-soft);
  }
  .gs-row.f {
    background: transparent;
    box-shadow: none;
    color: var(--on);
  }
  .gs-ico {
    width: 36px;
    height: 36px;
    border-radius: var(--r-s);
    display: grid;
    place-items: center;
    flex: none;
    background: color-mix(in srgb, var(--v1) 14%, var(--tint-base));
    color: var(--f1);
    transition: background var(--d-1) var(--e-soft), color var(--d-1) var(--e-soft);
  }
  .gs-row.danger .gs-ico {
    background: color-mix(in srgb, var(--bad) 14%, var(--tint-base));
    color: var(--bad);
  }
  .gs-row.f .gs-ico {
    background: color-mix(in srgb, var(--on) 22%, transparent);
    color: var(--on);
  }
  .gs-ico :global(svg) {
    width: var(--i-m);
    height: var(--i-m);
  }

  .gs-txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .gs-lbl {
    font: 700 var(--t-l)/1.2 var(--round);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .gs-sub {
    font: 500 var(--t-s)/1.2 var(--round);
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .gs-row.f .gs-sub {
    color: var(--on);
    opacity: 0.9;
  }

  .gs-val {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    flex: none;
    font: 700 var(--t-s)/1 var(--round);
    color: var(--ink-2);
  }
  .gs-row.f .gs-val {
    color: var(--on);
  }
  .gs-arr {
    width: 24px;
    height: 24px;
    border-radius: var(--r-pill);
    display: grid;
    place-items: center;
    background: var(--line);
    font: 800 var(--t-l)/1 var(--round);
    cursor: pointer;
  }
  .gs-row.f .gs-arr {
    background: color-mix(in srgb, var(--on) 22%, transparent);
  }
  .gs-stdot {
    width: 10px;
    height: 10px;
    border-radius: var(--r-pill);
  }
  .gs-swatch {
    width: 20px;
    height: 20px;
    border-radius: var(--r-pill);
    background: var(--gv);
    box-shadow: 0 0 0 2px var(--tint-base);
  }
  .gs-chev {
    display: grid;
    place-items: center;
    opacity: 0.65;
  }
  .gs-chev :global(svg) {
    width: var(--i-s);
    height: var(--i-s);
  }
  .gs-row.f .gs-chev {
    opacity: 1;
  }

  /* Bloque inferior contextual */
  .gs-ctx {
    margin-top: auto;
    padding: var(--s-3) var(--s-4);
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
    flex: none;
  }
  .gs-ctx b {
    font: 800 var(--t-s)/1.2 var(--round);
    color: var(--ink);
  }
  .gs-ctx p {
    margin: 0;
    font: 500 var(--t-s)/1.35 var(--round);
    color: var(--ink-2);
  }
</style>
