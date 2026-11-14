<script lang="ts">
  import { app, STATUS_LABEL, type MenuItem } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { closeOverlay, closePrompt, MAGPIE_LABEL, MAGPIE_ORDER } from '../lib/controller.svelte';
  import { GAME_TABS, TAB_HELP, gameSetAct, gameSetListAct, gameTabItems, otherPlatforms, selectTab, copying } from '../lib/gameset.svelte';
  import { platformCollections } from '../lib/coleccion.svelte';
  import { fmtRelease } from '../lib/release.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const g = $derived(app.gameSet);
  const t = $derived(app.platform?.titles[g.title]);
  const tabs = $derived(GAME_TABS.map(([k, name, ico]) => ({ k, name, ico, n: gameTabItems(t, k).length })));
  const rows = $derived(gameTabItems(t, g.tab));
  const tabName = $derived(GAME_TABS.find(([k]) => k === g.tab)?.[1] ?? '');
  const magpie = $derived.by(() => {
    void app.magpieTick;
    const m = t?.magpieMode ?? 'off';
    return { mode: m, label: MAGPIE_LABEL[m], detail: m !== 'off' ? (t?.magpieSummary ?? 'Se configura al lanzarlo') : '' };
  });
  const inColls = $derived(t?.id ? platformCollections().filter((c) => c.members.includes(t.id!)).length : 0);

  /** Etiqueta, línea pequeña y valor (a la derecha) de cada fila. */
  function info(it: MenuItem): { l: string; s?: string; v?: string } {
    switch (it) {
      case 'sgdb': return { l: 'Imágenes del juego…', s: 'Portada, fondo, logo e icono' };
      case 'accent': return { l: 'Color del juego…' };
      case 'reload': return { l: 'Volver a cargar las imágenes' };
      case 'magpie': return { l: 'Escalado con Magpie…', s: magpie.detail, v: magpie.label };
      case 'closegame': return g.confirm
        ? { l: closePrompt.force ? 'Pulsa A otra vez para forzar' : 'Pulsa A otra vez para cerrar' }
        : { l: 'Cerrar juego', s: 'Termina la partida en el PC principal' };
      case 'setexe': return { l: 'Ejecutable…' };
      case 'setdir': return { l: 'Carpeta de trabajo…' };
      case 'copyhere': return copying.id
        ? { l: 'Cancelar la copia', s: `${copying.name} · ${copying.total ? Math.floor((copying.done / copying.total) * 100) : 0} %` }
        : { l: 'Copiar a este equipo…', s: 'Descarga el juego del PC principal para jugarlo aquí' };
      case 'folder': return { l: 'Abrir carpeta de capturas' };
      case 'addshots': return { l: 'Añadir capturas…' };
      case 'saves': return { l: 'Partidas guardadas…' };
      case 'phrases': return { l: 'Frases y repaso…' };
      case 'addcoll': return { l: 'Colecciones…', v: inColls ? `En ${inColls}` : 'Ninguna' };
      case 'status': return { l: 'Estado…', v: STATUS_LABEL[t?.status ?? 'none'] };
      case 'release': return { l: 'Fecha de lanzamiento…', v: t?.release ? fmtRelease(t.release) : '' };
      case 'category': return { l: 'Mover a otra categoría…', v: app.platform?.name };
      case 'hide': return { l: 'Ocultar de la biblioteca' };
      case 'unhide': return { l: 'Volver a mostrar en la biblioteca' };
      case 'remove': return g.confirm
        ? { l: 'Pulsa A otra vez para quitar' }
        : { l: 'Quitar de la biblioteca', s: 'Pide confirmación · las capturas no se borran' };
      default: return { l: it };
    }
  }
  const icon = (it: MenuItem) => ({
    sgdb: ICON.search, accent: ICON.palette, reload: ICON.reload, closegame: ICON.power, magpie: ICON.image, setexe: ICON.folder, setdir: ICON.folder, copyhere: ICON.import,
    folder: ICON.folder, addshots: ICON.image, saves: ICON.import, phrases: ICON.jvn, addcoll: ICON.saga, status: ICON.checks,
    release: ICON.calendar, category: ICON.move, hide: ICON.eyeOff, unhide: ICON.eye, remove: ICON.trash,
  } as Record<string, string>)[it] ?? '';

  const listItems = $derived(g.list === 'magpie'
    ? MAGPIE_ORDER.map((m) => ({ l: MAGPIE_LABEL[m], s: m === 'auto' ? 'Mide la ventana al lanzar y elige' : '', on: m === magpie.mode }))
    : otherPlatforms().map((p) => ({ l: `Mover a ${p.name}`, s: '', on: false })));
  const listTitle = $derived(g.list === 'magpie' ? 'Escalado con Magpie' : 'Mover a otra categoría');

  const moved = makeMoved();
  function hoverTab(e: MouseEvent, k: typeof g.tab, n: number) {
    if (!n || !moved(e)) return;
    selectTab(k); g.inRows = false;
  }
  function hoverRow(e: MouseEvent, i: number) {
    if (moved(e)) { g.inRows = true; g.row = i; g.confirm = false; }
  }
  function hoverList(e: MouseEvent, i: number) {
    if (moved(e)) g.lrow = i;
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) closeOverlay(); }}>
  <div class="gs-wrap">
    <div class="glass gs-menu">
      <div class="face">
        <h2>Ajustes del juego</h2>
        <p class="msub">{t?.name ?? ''}</p>
        {#each tabs as tb (tb.k)}
          <button type="button" class="gs-tab" class:on={g.tab === tb.k} class:f={g.tab === tb.k && !g.inRows && !g.list} class:dis={!tb.n}
            onmousemove={(e) => hoverTab(e, tb.k, tb.n)} onclick={() => { if (tb.n) { selectTab(tb.k); g.inRows = true; g.row = 0; } }}>
            <i>{@html tb.ico}</i><span>{tb.name}</span><em>{tb.n || '—'}</em>
          </button>
        {/each}
      </div>
    </div>
    <div class="glass gs-body">
      <div class="face">
        {#if g.list}
          <h3>{listTitle}</h3>
          <button type="button" class="gs-back" onclick={() => { g.list = ''; }}>‹ {tabName}</button>
          {#each listItems as it, i}
            <button type="button" class="srow gs-row" class:f={g.lrow === i} onmousemove={(e) => hoverList(e, i)} onclick={() => { g.lrow = i; gameSetListAct(i); }}>
              <span class="mic">{@html it.on ? ICON.check : ''}</span>
              <span class="sl">{it.l}{#if it.s}<small>{it.s}</small>{/if}</span>
            </button>
          {/each}
        {:else}
          <h3>{tabName}</h3>
          <p class="gs-help">{TAB_HELP[g.tab]}</p>
          {#each rows as it, i (it)}
            {@const d = info(it)}
            <button type="button" class="srow gs-row" class:f={g.inRows && g.row === i} class:danger={it === 'remove'} class:gap={it === 'category'} data-item={it}
              onmousemove={(e) => hoverRow(e, i)} onclick={() => { g.inRows = true; g.row = i; gameSetAct(it); }}>
              <span class="mic">{@html icon(it)}</span>
              <span class="sl">{d.l}{#if d.s}<small>{d.s}</small>{/if}</span>
              {#if it === 'accent'}<span class="gs-dot"></span>{:else if d.v}<span class="sv">{d.v}</span>{/if}
            </button>
          {/each}
          {#if !rows.length}<p class="gs-help">Este juego lo abre su emulador; aquí no hay nada que ajustar.</p>{/if}
        {/if}
      </div>
    </div>
  </div>
  <Hints list={g.list || g.inRows
    ? [['A', g.confirm ? 'Confirmar' : 'Elegir', 'a'], ['B', g.list ? 'Volver' : 'Pestañas', 'b']]
    : [['A', 'Entrar', 'a'], ['B', 'Cerrar', 'b']]} />
</div>

<style>
  .gs-wrap { position: absolute; inset: 0; }
  .gs-menu { left: 56px; top: 24px; width: 290px; height: 620px; --r: 36px; cursor: default; }
  .gs-body { left: 366px; top: 24px; width: 858px; height: 620px; --r: 36px; cursor: default; }
  .gs-menu .face, .gs-body .face { background: var(--face-hi); color: var(--ink); display: flex; flex-direction: column; padding: 24px 22px; position: relative; }
  .gs-menu h2 { margin: 0 8px 4px; font-size: var(--t-2xl); font-weight: 800; color: var(--ink); }
  .gs-menu .msub { margin: 0 8px 14px; }
  .gs-tab { position: relative; flex: none; display: flex; align-items: center; gap: 12px; width: 100%; height: 58px; margin-bottom: 6px; padding: 0 16px; border: 0; border-radius: var(--r-m); background: transparent; font: inherit; font-size: var(--t-xl); font-weight: 700; color: var(--ink-2); text-align: left; cursor: pointer; transition: background .15s, color .15s; }
  .gs-tab i { flex: none; width: 26px; height: 26px; display: grid; place-items: center; }
  .gs-tab i :global(svg) { width: 100%; height: 100%; }
  .gs-tab span { flex: 1; white-space: nowrap; }
  .gs-tab em { font-style: normal; font-size: var(--t-m); opacity: .7; margin-left: auto; padding-left: 10px; }
  .gs-tab.on { background: var(--gf); color: var(--on); }
  .gs-tab.f { box-shadow: var(--ring); }
  .gs-tab.dis { opacity: .45; cursor: default; }
  .gs-body h3 { margin: 0 0 4px; font-size: var(--t-2xl); font-weight: 800; color: var(--ink); }
  .gs-help { margin: 0 0 14px; font-size: var(--t-m); font-weight: 600; color: var(--ink-2); }
  .gs-row { height: 58px; flex: none; margin-bottom: 4px; }
  .gs-row.gap { margin-top: 14px; }
  .gs-row .sl { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .gs-dot { width: 18px; height: 18px; border-radius: 50%; background: var(--accv); box-shadow: 0 0 0 2px var(--line); }
  .gs-back { align-self: flex-start; margin: 0 0 12px; padding: 0; border: 0; background: transparent; font: inherit; font-size: var(--t-m); font-weight: 700; color: var(--ink-2); cursor: pointer; }
</style>
