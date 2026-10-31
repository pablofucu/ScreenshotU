<script lang="ts">
  // Partidas guardadas a pantalla completa («Explorador»): a la izquierda el espacio que ocupan las copias y la lista por
  // días; a la derecha la copia elegida y la comparación archivo a archivo con las partidas de ahora. «Dónde guarda»
  // es un cajón que se abre desde la cabecera. Ratón: clic, clic derecho = menú ⋯, «Restaurar este» al pasar por una fila.
  import { app } from '../lib/store.svelte';
  import { fmtBytes } from '../lib/controller.svelte';
  import { fmtHM } from '../lib/stats';
  import {
    KIND_LABEL, KIND_SHORT, STATE_LABEL, backupNow, closeMenu, dayLabel, focusTable, hhmm, locsClick, menuItems, openLocs, openMenu, originLabel, pickRow,
    restoreCopy, restoreFile, rowHelp, runMenu, savesCmd, savesRows, savesSummary, selectCopy, setFilter, sv, togglePin, visibleFiles, when,
    type CopyFile, type FileFilter, type SaveCopy, type SaveRow,
  } from '../lib/saves.svelte';
  import Hints from '../components/Hints.svelte';
  import TopBar from '../components/TopBar.svelte';
  import Thumb from '../components/Thumb.svelte';

  const info = $derived(sv.info);
  const copies = $derived(info?.copies ?? []);
  const copy = $derived(copies[sv.sel]);
  const drawer = $derived(sv.focus === 'locs');
  const files = $derived(visibleFiles());

  /* ---------- el juego en la biblioteca: capturas y partidas jugadas ---------- */
  const title = $derived(info ? app.lib.platforms.flatMap((p) => p.titles).find((t) => t.gameId === info.gameId) : undefined);
  const DAY = 864e5;
  const dayStartMs = (ms: number) => { const d = new Date(ms); d.setHours(0, 0, 0, 0); return d.getTime(); };
  /** Minutos jugados (en ms) ese día local. */
  function playedOn(ms: number): number {
    const a = dayStartMs(ms), b = a + DAY;
    return (title?.plays ?? []).reduce((n, p) => n + Math.max(0, Math.min(p[1], b) - Math.max(p[0], a)), 0);
  }
  /** Duración de la partida a la que pertenece la copia (la copia «al cerrar» llega justo al terminar). */
  function sessionMs(c: SaveCopy): number {
    const p = (title?.plays ?? []).find((x) => c.createdMs >= x[0] && c.createdMs <= x[1] + 10 * 60000);
    return p ? Math.min(c.createdMs, p[1]) - p[0] : 0;
  }
  /** Última captura del juego anterior a la copia (sus fechas son la hora local codificada como UTC). */
  function shotBefore(c: SaveCopy): string | null {
    const lim = c.createdMs - new Date(c.createdMs).getTimezoneOffset() * 60000;
    let best: { path: string; date: number } | null = null;
    for (const s of title?.shots ?? []) if (s.date <= lim && (!best || s.date > best.date)) best = s;
    return best?.path ?? null;
  }
  const shot = $derived(copy ? shotBefore(copy) : null);
  const session = $derived(copy ? sessionMs(copy) : 0);

  /* ---------- espacio y grupos por día ---------- */
  const total = $derived(copies.reduce((a, c) => a + c.size, 0));
  const byKind = (k: SaveCopy['kind']) => copies.filter((c) => c.kind === k).reduce((a, c) => a + c.size, 0);
  const KCOL = { auto: 'var(--acc)', manual: 'var(--ok)', antes: 'var(--warn)' };
  const groups = $derived.by(() => {
    const out: Array<{ key: number; label: string; played: number; items: Array<{ c: SaveCopy; i: number }> }> = [];
    copies.forEach((c, i) => {
      const key = dayStartMs(c.createdMs);
      let g = out[out.length - 1];
      if (!g || g.key !== key) { g = { key, label: dayLabel(c.createdMs), played: playedOn(c.createdMs), items: [] }; out.push(g); }
      g.items.push({ c, i });
    });
    return out;
  });
  const heading = (c: SaveCopy) => c.note || KIND_LABEL[c.kind];

  /* ---------- tabla ---------- */
  const cmpFiles = $derived(sv.cmp?.files ?? []);
  const cnt = $derived({
    all: cmpFiles.length,
    changed: cmpFiles.filter((f) => f.state === 'changed').length,
    onlyCopy: cmpFiles.filter((f) => f.state === 'onlyCopy').length,
    newNow: cmpFiles.filter((f) => f.state === 'newNow').length,
    same: cmpFiles.filter((f) => f.state === 'same').length,
  });
  const CHIPS: Array<[FileFilter, string, string]> = [
    ['all', 'Todos', ''], ['changed', 'cambian', 'chg'], ['onlyCopy', 'solo en la copia', 'del'], ['newNow', 'nuevos ahora', 'add'], ['same', 'iguales', ''],
  ];
  const place = (f: CopyFile) => (f.registry ? 'Registro' : sv.cmp?.locs[f.loc]?.place.split(' › ').slice(-2).join(' › ') ?? '');
  const shortWhen = (ms: number) => (ms > 0 ? when(ms) : '—');
  const fileMod = (f: CopyFile) => (f.state === 'onlyCopy' ? f.copyMs : f.state === 'changed' ? f.nowMs : f.nowMs || f.copyMs);
  const fileSize = (f: CopyFile) => fmtBytes(f.state === 'newNow' ? f.nowSize : f.size);

  /* ---------- cajón «Dónde guarda» ---------- */
  const rows = $derived.by(() => { void sv.focus; void sv.busy; return savesRows(); });
  const focusedRow = $derived(rows[sv.row]);
  function locText(r: SaveRow): { l: string; v: string; dim?: boolean } {
    const i = info!;
    switch (r.k) {
      case 'download': return { l: '↓  Descargar la base de Ludusavi', v: sv.busy === 'download' ? 'Descargando…' : '18 MB' };
      case 'loc': {
        const l = i.locs[r.i];
        return { l: l.place, v: !l.exists ? 'No existe' : l.kind === 'registry' ? originLabel(l) : `${originLabel(l)} · ${l.files} archivos · ${fmtBytes(l.size)}`, dim: !l.enabled };
      }
      case 'adddir': return { l: '＋  Añadir carpeta…', v: '' };
      case 'addfile': return { l: '＋  Añadir archivo…', v: '' };
    }
  }

  /* ---------- pistas de mando ---------- */
  const hints = $derived.by<Array<[string, string, string?]>>(() => {
    const armed = sv.confirm === 'all';
    if (sv.focus === 'locs') {
      const r = focusedRow;
      const h: Array<[string, string, string?]> = [['A', r?.k === 'loc' ? (info!.locs[r.i].enabled ? 'No copiar' : 'Copiar') : 'Elegir', 'a']];
      if (r?.k === 'loc' && info!.locs[r.i].origin === 'manual') h.push(['X', 'Quitar', 'x']);
      h.push(['Y', r?.k === 'loc' ? 'Abrir ubicación' : 'Abrir copias', 'y'], ['B', 'Volver', 'b']);
      return h;
    }
    if (sv.focus === 'menu') return [['↑↓', 'Elegir'], ['A', 'Aceptar', 'a'], ['B', 'Cerrar', 'b']];
    if (sv.focus === 'table') {
      const f = files[sv.fileRow];
      return [['↑↓', 'Archivo'], ['←', 'Copias', 'left'], ['A', sv.confirm === f?.entry ? '¿Seguro?' : 'Restaurar este', 'a'], ['LB/RB', 'Filtro', 'lb'], ['Y', 'Más', 'y'], ['B', 'Volver', 'b']];
    }
    return [
      ['↑↓', 'Copia'], ['→', 'Archivos', 'right'], ['A', copy ? (armed ? '¿Seguro?' : 'Restaurar') : 'Copiar ahora', 'a'],
      ['X', copy?.pinned ? 'No fijar' : 'Fijar', 'x'], ['Y', 'Más', 'y'], ['LB', 'Dónde guarda', 'lb'], ['B', 'Atrás', 'b'],
    ];
  });

  /* ---------- ratón ---------- */
  let root: HTMLDivElement | undefined = $state();
  function onScrim(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t === e.currentTarget) { savesCmd('b'); return; }
    if (sv.focus === 'menu' && !t.closest('.smenu, .hints')) closeMenu();
  }
  /** Clic derecho en una copia: la elige y abre el menú ⋯ donde está el cursor (el escenario va escalado). */
  function ctxCopy(e: MouseEvent, i: number) {
    e.preventDefault(); e.stopPropagation();
    selectCopy(i);
    const r = root!.getBoundingClientRect(), k = r.width / 1280 || 1;
    openMenu([Math.min((e.clientX - r.left) / k, 1280 - 290), Math.min((e.clientY - r.top) / k, 720 - 260)]);
  }
  function clickFile(j: number) { sv.fileRow = j; if (sv.focus !== 'table') sv.focus = 'table'; }

  // Mantiene a la vista la copia o el archivo elegidos.
  $effect(() => {
    void sv.sel; void copies.length;
    root?.querySelector(`[data-ci="${sv.sel}"]`)?.scrollIntoView({ block: 'nearest' });
  });
  $effect(() => {
    void sv.fileRow; void sv.focus;
    if (sv.focus === 'table') root?.querySelector(`[data-fj="${sv.fileRow}"]`)?.scrollIntoView({ block: 'nearest' });
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim spage" role="presentation" bind:this={root} onclick={onScrim}>
  <TopBar crumbs={[info?.name ?? '', 'Partidas guardadas']} />

  <!-- Izquierda: espacio y lista por días -->
  <div class="panel lst">
    <div class="stor">
      <div class="top">{copies.length} {copies.length === 1 ? 'copia' : 'copias'}<small>{fmtBytes(total)} en disco</small></div>
      <div class="ubar">
        {#each ['auto', 'manual', 'antes'] as const as k}
          {#if total > 0 && byKind(k) > 0}<i style="width:{(byKind(k) / total) * 100}%;background:{KCOL[k]}"></i>{/if}
        {/each}
      </div>
      <div class="sub">Al cerrar {fmtBytes(byKind('auto'))} · manuales {fmtBytes(byKind('manual'))} · conserva {info?.keep ?? 10} + fijadas</div>
      <div class="lbtns">
        <button class="bt sm" disabled={!!sv.busy || sv.loading} onclick={() => backupNow()}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>{sv.busy === 'backup' ? 'Copiando…' : 'Copiar ahora'}
        </button>
        <button class="bt sm" onclick={() => openLocs()}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>Dónde guarda
          <span class="cbadge">{info?.locs.filter((l) => l.enabled).length ?? 0}</span>
        </button>
      </div>
    </div>
    <div class="lwrap">
      {#if !copies.length}
        <p class="empty">{sv.loading ? 'Buscando las copias…' : 'Aún no hay copias de este juego.'}</p>
      {/if}
      {#each groups as g (g.key)}
        <div class="lgh"><span>{g.label}</span><span>{g.played >= 30000 ? fmtHM(g.played) + ' jugado' : ''}</span></div>
        {#each g.items as { c, i } (c.file)}
          <div class="li k-{c.kind}" class:s={i === sv.sel} class:fo={i === sv.sel && sv.focus === 'list'} data-ci={i} onclick={() => { selectCopy(i); if (sv.focus !== 'list') { sv.focus = 'list'; } }} oncontextmenu={(e) => ctxCopy(e, i)}>
            <span class="kdot"></span><span class="t">{hhmm(c.createdMs)}</span>
            <span class="n" class:plain={!c.note}>{c.note || KIND_SHORT[c.kind]}</span>
            {#if c.pinned}<span class="pin"><svg viewBox="0 0 24 24"><path fill="currentColor" d="M12 2.5l2.9 6 6.6.8-4.9 4.6 1.3 6.5L12 17.2l-5.9 3.2 1.3-6.5L2.5 9.3l6.6-.8z" /></svg></span>{/if}
            <span class="z">{fmtBytes(c.size)}</span>
          </div>
        {/each}
      {/each}
    </div>
  </div>

  <!-- Derecha: la copia elegida y su contenido -->
  <div class="panel rp">
    {#if copy}
      <div class="head">
        <div class="thumb">{#if shot}{#key shot}<Thumb path={shot} />{/key}{:else}<span class="nothumb"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 8h3l2-3h6l2 3h3v11H4z" /><circle cx="12" cy="13" r="3.5" /></svg></span>{/if}</div>
        <div class="hinfo">
          <h3><span class="ht">{heading(copy)}</span>{#if copy.pinned}<span class="pin"><svg viewBox="0 0 24 24"><path fill="currentColor" d="M12 2.5l2.9 6 6.6.8-4.9 4.6 1.3 6.5L12 17.2l-5.9 3.2 1.3-6.5L2.5 9.3l6.6-.8z" /></svg></span>{/if}</h3>
          <div class="when"><span class="badge k-{copy.kind}">{KIND_SHORT[copy.kind]}</span>{when(copy.createdMs)}{session >= 60000 ? ` · ${fmtHM(session)} de juego` : ''}</div>
          <div class="meta">{copy.files} {copy.files === 1 ? 'archivo' : 'archivos'} · {fmtBytes(copy.size)} · {sv.cmp ? sv.cmp.locs.length : '…'} {sv.cmp?.locs.length === 1 ? 'ubicación' : 'ubicaciones'}</div>
        </div>
        <div class="brow">
          {#if sv.confirm === 'all'}
            <button class="bt dan sm" onclick={() => restoreCopy()}>¿Seguro? Otra vez <span class="btn mini a">A</span></button>
          {:else}
            <button class="bt pri sm" disabled={!!sv.busy} onclick={() => restoreCopy()}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7L3 8" /><path d="M3 3v5h5" /><path d="M12 7v5l3 2" /></svg>Restaurar <span class="btn mini a">A</span>
            </button>
          {/if}
          <div class="brow2">
            <button class="bt sm" style="flex:1" onclick={() => togglePin()}>
              <svg viewBox="0 0 24 24" fill={copy.pinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2" stroke-linejoin="round" style={copy.pinned ? 'color:var(--warn)' : ''}><path d="M12 2.5l2.9 6 6.6.8-4.9 4.6 1.3 6.5L12 17.2l-5.9 3.2 1.3-6.5L2.5 9.3l6.6-.8z" /></svg>{copy.pinned ? 'Fijada' : 'Fijar'}
            </button>
            <button class="bt sm ic" title="Más: nota, exportar, abrir, borrar" aria-label="Más opciones" onclick={() => (sv.focus === 'menu' ? closeMenu() : openMenu())}>
              <svg viewBox="0 0 24 24"><circle cx="5" cy="12" r="1.5" fill="currentColor" /><circle cx="12" cy="12" r="1.5" fill="currentColor" /><circle cx="19" cy="12" r="1.5" fill="currentColor" /></svg>
            </button>
          </div>
        </div>
      </div>

      <div class="diff"><span class="lbl">Comparada con ahora:</span>
        {#each CHIPS as [k, t, cl]}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="dchip" class:on={sv.filter === k} onclick={() => setFilter(k)}>{#if k === 'all'}Todos <b>{cnt.all}</b>{:else}<b class={cl}>{cnt[k]}</b> {t}{/if}</span>
        {/each}
      </div>

      <div class="ftab" class:fo={sv.focus === 'table'}>
        <div class="fh"><span></span><span>Archivo</span><span>Ubicación</span><span style="text-align:right">Tamaño</span><span>Modificado</span><span>Al restaurar</span></div>
        <div class="fbody">
          {#if sv.cmpLoading && !sv.cmp}
            <div class="fmsg">Comparando con las partidas de ahora…</div>
          {:else if !files.length}
            <div class="fmsg">{cmpFiles.length ? 'Ningún archivo con ese filtro.' : 'Esta copia no tiene archivos.'}</div>
          {/if}
          {#each files as f, j (f.entry)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="fr" class:s={sv.focus === 'table' && sv.fileRow === j} class:arm={sv.confirm === f.entry} data-fj={j} onclick={() => clickFile(j)}>
              <span class="ico"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{#if f.registry}<rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><path d="M14 17.5h7M17.5 14v7" />{:else}<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" /><path d="M14 3v5h5" />{/if}</svg></span>
              <span class="fn" title={f.name}>{f.name}</span>
              <span class="loc" title={sv.cmp?.locs[f.loc]?.path}>{place(f)}</span>
              <span class="zz">{fileSize(f)}</span>
              <span class="md">{shortWhen(fileMod(f))}</span>
              <span class="fst {f.state}">{STATE_LABEL[f.state]}</span>
              {#if f.state !== 'newNow'}
                <button class="bt sm one" class:dan={sv.confirm === f.entry} onclick={(e) => { e.stopPropagation(); sv.fileRow = j; void restoreFile(f); }}>{sv.confirm === f.entry ? '¿Seguro? Otra vez' : 'Restaurar este'}</button>
              {/if}
            </div>
          {/each}
        </div>
      </div>
      <div class="foot">Lo que hay ahora se guarda antes en una copia «Antes de restaurar».</div>
    {:else}
      <div class="blank">
        <h3>{sv.loading ? 'Buscando…' : 'Sin copias todavía'}</h3>
        <p>{savesSummary()}</p>
        {#if !sv.loading}
          <div class="brow3">
            <button class="bt pri" disabled={!!sv.busy} onclick={() => backupNow()}>Copiar ahora</button>
            <button class="bt" onclick={() => openLocs()}>Dónde guarda</button>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Menú ⋯ -->
  {#if sv.focus === 'menu' && copy}
    {@const xy = sv.menuXY ?? [930, 150]}
    <div class="smenu" style="left:{xy[0]}px;top:{xy[1]}px">
      {#each menuItems() as m, n (m.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class:f={sv.menuRow === n} class:red={m.id === 'del'} class:arm={m.id === 'del' && sv.confirm === 'del'} onclick={() => { sv.menuRow = n; runMenu(m.id); }}>{m.label}</div>
        {#if m.id === 'open'}<hr />{/if}
      {/each}
    </div>
  {/if}

  <!-- Cajón «Dónde guarda» -->
  {#if drawer && info}
    <div class="sdrawer">
      <h2>Dónde guarda</h2>
      <p class="dsum"><b>{info.name}</b> · {savesSummary()}</p>
      <div class="dlist">
        {#each rows as r, i}
          {@const t = locText(r)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="srow" class:f={sv.row === i} class:dim={t.dim} onmousemove={() => { if (sv.row !== i) pickRow(i); }} onclick={() => locsClick(i)}>
            {#if r.k === 'loc'}<span class="tog" class:on={!t.dim}><i></i></span>{/if}
            <span class="sl">{t.l}</span>
            <span class="sv">{t.v}</span>
          </div>
        {/each}
      </div>
      <p class="savedet">{rowHelp(focusedRow)}</p>
      <button class="bt sm" onclick={() => savesCmd('b')}>Cerrar</button>
    </div>
  {/if}
  <Hints list={hints} />
</div>

<style>
  .spage :global(.hints) { z-index: 40; }
  .spage { background: var(--face); -webkit-backdrop-filter: none; backdrop-filter: none; color: var(--ink); }
  .panel { position: absolute; border-radius: 24px; background: linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(250, 251, 253, 0.86)); box-shadow: inset 0 1.5px 0 var(--glass-top), 0 0 0 1px var(--line), 0 18px 30px -18px rgba(40, 50, 70, 0.45); }
  .lst { left: 40px; top: 92px; width: 392px; height: 560px; padding: 16px 14px; display: flex; flex-direction: column; }
  .rp { left: 448px; top: 92px; width: 792px; height: 560px; padding: 18px 22px; display: flex; flex-direction: column; }
  .k-auto { --k: var(--acc); } .k-manual { --k: var(--ok); } .k-antes { --k: var(--warn); }

  /* espacio ocupado */
  .stor { flex: none; padding: 0 8px 12px; border-bottom: 1px solid var(--line); margin-bottom: 6px; }
  .stor .top { display: flex; justify-content: space-between; align-items: baseline; font-size: 15px; font-weight: 800; }
  .stor .top small { font-size: 13px; font-weight: 700; color: var(--ink-3); }
  .ubar { display: flex; height: 10px; border-radius: 5px; overflow: hidden; margin: 8px 0 6px; background: var(--line); }
  .stor .sub { font-size: 12.5px; font-weight: 700; color: var(--ink-3); }
  .lbtns { display: flex; gap: 8px; margin-top: 10px; }
  .cbadge { display: inline-grid; place-items: center; min-width: 20px; height: 20px; padding: 0 6px; border-radius: 10px; background: var(--ok); color: var(--on); font-size: 12px; }

  /* lista por días */
  .lwrap { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; scrollbar-width: none; margin: 0 -8px; padding: 0 8px 8px; }
  .lwrap::-webkit-scrollbar { display: none; }
  .empty { margin: 24px 10px; font-size: 14px; font-weight: 700; color: var(--ink-3); }
  .lgh { height: 28px; padding: 8px 10px 0; font-size: 12px; font-weight: 800; color: var(--ink-3); text-transform: uppercase; letter-spacing: 0.06em; display: flex; justify-content: space-between; }
  .lgh span:last-child { text-transform: none; letter-spacing: 0; }
  .li { display: flex; align-items: center; gap: 10px; height: 46px; padding: 0 10px; border-radius: 14px; cursor: pointer; margin-bottom: 2px; }
  .li:hover { background: var(--glass-in); }
  .li .kdot { width: 10px; height: 10px; border-radius: 50%; background: var(--k); flex: none; }
  .li .t { width: 52px; flex: none; font-size: 15px; font-weight: 800; }
  .li .n { flex: 1; min-width: 0; font-size: 14.5px; font-weight: 700; color: var(--ink-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .li .n.plain { color: var(--ink-3); }
  .li .z { font-size: 12.5px; font-weight: 700; color: var(--ink-3); flex: none; }
  .pin { color: var(--warn); display: inline-flex; flex: none; }
  .pin :global(svg) { width: 15px; height: 15px; }
  .li.s { background: var(--face); }
  .li.s.fo { background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 30%, rgba(255, 255, 255, 0.85)), var(--acc)); box-shadow: inset 0 1.5px 0 rgba(255, 255, 255, 0.8), 0 0 0 1px color-mix(in srgb, var(--acc) 75%, black), 0 8px 16px -8px color-mix(in srgb, var(--acc) 70%, black); }
  .li.s.fo .t, .li.s.fo .n, .li.s.fo .z { color: var(--on); }
  .li.s.fo .kdot { box-shadow: 0 0 0 2px var(--on); }

  /* botones y piezas */
  .bt { height: 44px; padding: 0 18px; border-radius: 999px; display: inline-flex; align-items: center; justify-content: center; gap: 8px; font: 800 16px var(--round, inherit); color: var(--ink-2); white-space: nowrap; flex: none; background: linear-gradient(180deg, var(--face-hi), var(--face)); box-shadow: inset 0 1px 0 var(--glass-top), 0 0 0 1px var(--line), 0 4px 10px -6px rgba(40, 50, 70, 0.4); border: 0; cursor: pointer; transition: transform 0.12s, filter 0.12s; }
  .bt:hover { filter: brightness(1.04); transform: translateY(-1px); }
  .bt:active { transform: translateY(1px); }
  .bt:disabled { opacity: 0.55; cursor: default; transform: none; }
  .bt.pri { color: var(--acc-ink); background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 45%, white), var(--acc) 60%); box-shadow: inset 0 1.5px 0 rgba(255, 255, 255, 0.7), 0 0 0 1px color-mix(in srgb, var(--acc) 75%, black), 0 8px 16px -8px var(--acc); }
  .bt.dan { color: var(--on); background: var(--bad); box-shadow: inset 0 1.5px 0 rgba(255, 255, 255, 0.6), 0 0 0 1px color-mix(in srgb, var(--bad) 75%, black), 0 8px 16px -8px var(--bad); }
  .bt :global(svg) { width: 19px; height: 19px; flex: none; }
  .bt.sm { height: 36px; font-size: 14px; padding: 0 13px; gap: 6px; }
  .bt.sm :global(svg) { width: 16px; height: 16px; }
  .bt.ic { width: 36px; padding: 0; }
  .btn.mini { min-width: 20px; height: 20px; font-size: 11px; border-radius: 10px; padding: 0 5px; }
  .badge { display: inline-flex; align-items: center; gap: 6px; height: 24px; padding: 0 10px; border-radius: 999px; font-size: 12.5px; font-weight: 800; white-space: nowrap; background: color-mix(in srgb, var(--k) 16%, white); color: color-mix(in srgb, var(--k) 62%, black); }
  .badge::before { content: ''; width: 8px; height: 8px; border-radius: 50%; background: var(--k); }

  /* cabecera de la copia */
  .head { flex: none; display: flex; gap: 16px; align-items: center; }
  .thumb { position: relative; width: 176px; height: 99px; flex: none; border-radius: 14px; overflow: hidden; background: var(--line); box-shadow: inset 0 0 0 1px rgba(20, 30, 50, 0.18); }
  .thumb :global(img.tin) { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .nothumb { position: absolute; inset: 0; display: grid; place-items: center; color: var(--ink-3); background: repeating-linear-gradient(135deg, var(--face-hi) 0 10px, var(--face) 10px 20px); }
  .nothumb :global(svg) { width: 34px; height: 34px; }
  .hinfo { min-width: 0; flex: 1; }
  .head h3 { margin: 0; font-size: 22px; font-weight: 800; display: flex; gap: 8px; align-items: center; min-width: 0; }
  .head h3 .ht { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .head h3 .pin :global(svg) { width: 18px; height: 18px; }
  .when { font-size: 14.5px; font-weight: 700; color: var(--ink-2); margin: 4px 0 8px; display: flex; gap: 8px; align-items: center; white-space: nowrap; }
  .meta { font-size: 13.5px; font-weight: 700; color: var(--ink-3); }
  .brow { margin-left: auto; align-self: flex-start; display: flex; flex-direction: column; gap: 6px; flex: none; width: 190px; }
  .brow > .bt { width: 100%; }
  .brow2 { display: flex; gap: 6px; }

  /* comparación */
  .diff { flex: none; display: flex; gap: 8px; margin: 14px 0 10px; align-items: center; flex-wrap: wrap; row-gap: 6px; }
  .dchip { height: 30px; padding: 0 12px; border-radius: 999px; font-size: 13.5px; font-weight: 800; display: flex; align-items: center; gap: 6px; background: var(--glass-in); color: var(--ink-2); cursor: pointer; white-space: nowrap; }
  .dchip.on { box-shadow: 0 0 0 2px var(--acc); }
  .dchip b { font-size: 14px; }
  .lbl { font-size: 13.5px; font-weight: 800; color: var(--ink-3); margin-right: 2px; white-space: nowrap; }
  .chg { color: var(--warn); } .del { color: var(--bad); } .add { color: var(--ok); }
  .ftab { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; border-radius: 16px; box-shadow: 0 0 0 1px var(--line); overflow: hidden; }
  .ftab.fo { box-shadow: 0 0 0 2px var(--acc); }
  .fh, .fr { display: grid; grid-template-columns: 28px minmax(0, 1fr) 170px 76px 112px 116px; align-items: center; gap: 10px; padding: 0 14px; }
  .fh { flex: none; height: 32px; background: var(--face); font-size: 12px; font-weight: 800; color: var(--ink-3); text-transform: uppercase; letter-spacing: 0.05em; }
  .fbody { flex: 1; min-height: 0; overflow-y: auto; scrollbar-width: none; }
  .fbody::-webkit-scrollbar { display: none; }
  .fmsg { padding: 22px; text-align: center; font-size: 14px; font-weight: 700; color: var(--ink-3); }
  .fr { height: 38px; border-top: 1px solid var(--line); font-size: 14px; font-weight: 700; color: var(--ink-2); cursor: pointer; position: relative; }
  .fr:first-child { border-top: 0; }
  .fr:hover { background: var(--glass-in); }
  .fr.s { background: color-mix(in srgb, var(--acc) 12%, white); box-shadow: inset 3px 0 0 var(--acc); }
  .fr .ico { width: 20px; height: 20px; color: var(--ink-3); display: grid; place-items: center; }
  .fr .ico :global(svg) { width: 100%; height: 100%; }
  .fr .fn, .fr .loc, .fr .md { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .fr .loc, .fr .md { font-size: 12.5px; color: var(--ink-3); }
  .fr .zz { font-size: 13px; color: var(--ink-3); text-align: right; }
  .fst { font-size: 12px; font-weight: 800; height: 22px; border-radius: 999px; display: inline-flex; align-items: center; padding: 0 9px; justify-self: start; white-space: nowrap; }
  .fst.same { background: var(--face); color: var(--ink-3); } .fst.changed { background: color-mix(in srgb, var(--warn) 20%, white); color: var(--warn); }
  .fst.onlyCopy { background: color-mix(in srgb, var(--bad) 18%, white); color: var(--bad); } .fst.newNow { background: color-mix(in srgb, var(--ok) 18%, white); color: var(--ok); }
  .fr .one { position: absolute; right: 10px; top: 1px; display: none; }
  .fr:hover .one, .fr.s .one, .fr.arm .one { display: inline-flex; }
  .fr:hover .fst, .fr.s .fst, .fr.arm .fst { visibility: hidden; }
  .foot { flex: none; margin-top: 10px; font-size: 13.5px; font-weight: 700; color: var(--ink-2); }

  .blank { margin: auto; text-align: center; max-width: 440px; }
  .blank h3 { margin: 0 0 6px; font-size: 24px; font-weight: 800; }
  .blank p { margin: 0 0 18px; font-size: 15px; font-weight: 600; color: var(--ink-2); line-height: 1.4; }
  .brow3 { display: flex; gap: 10px; justify-content: center; }

  /* menú ⋯ */
  .smenu { position: absolute; z-index: 28; width: 250px; padding: 8px; border-radius: 20px; background: var(--face-hi); box-shadow: 0 0 0 1px rgba(95, 110, 135, 0.25), 0 24px 40px -16px rgba(30, 40, 60, 0.55); }
  .smenu div { display: flex; align-items: center; height: 40px; padding: 0 14px; border-radius: 12px; font-size: 15.5px; font-weight: 700; color: var(--ink-2); cursor: pointer; }
  .smenu div:hover, .smenu div.f { background: var(--acc); color: var(--acc-ink); }
  .smenu div.red { color: var(--bad); }
  .smenu div.red:hover, .smenu div.red.f { background: var(--bad); color: var(--on); }
  .smenu div.arm { background: var(--bad); color: var(--on); }
  .smenu hr { border: 0; border-top: 1px solid var(--line); margin: 6px 4px; }

  /* cajón «Dónde guarda» */
  .sdrawer { position: absolute; z-index: 30; right: 0; top: 0; bottom: 0; width: 520px; padding: 80px 26px 70px; background: var(--face-hi); box-shadow: -20px 0 40px -20px rgba(30, 40, 60, 0.45), inset 1px 0 0 rgba(95, 110, 135, 0.2); display: flex; flex-direction: column; gap: 8px; }
  .sdrawer h2 { margin: 0; font-size: 24px; font-weight: 800; }
  .dsum { margin: 0 0 6px; font-size: 14px; font-weight: 600; color: var(--ink-2); line-height: 1.4; }
  .dsum b { font-weight: 800; color: var(--ink); }
  .dlist { flex: 1; min-height: 0; overflow-y: auto; scrollbar-width: none; margin: -2px -8px -6px; padding: 2px 8px 6px; }
  .dlist::-webkit-scrollbar { display: none; }
  .sdrawer .srow { height: 50px; padding: 0 16px; font-size: 16px; }
  .sdrawer .srow .sl { overflow: hidden; text-overflow: ellipsis; }
  .sdrawer .srow .sv { max-width: 200px; font-size: 13px; }
  .sdrawer .srow.dim:not(.f) { color: var(--ink-3); }
  .sdrawer .bt { align-self: flex-start; }
  .tog { flex: none; position: relative; width: 38px; height: 22px; border-radius: 11px; background: rgba(120, 130, 150, 0.35); transition: background 0.15s; }
  .tog i { position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: var(--on); box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25); transition: left 0.15s; }
  .tog.on { background: var(--ok); }
  .tog.on i { left: 19px; }
  .savedet { margin: 6px 0 4px; height: 62px; overflow: hidden; border-radius: 14px; padding: 8px 14px; background: rgba(214, 220, 230, 0.45); font-size: 13.5px; font-weight: 600; line-height: 1.35; color: var(--ink-2); overflow-wrap: anywhere; }
</style>
