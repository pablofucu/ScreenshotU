<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { fmtBytes } from '../lib/controller.svelte';
  import { KIND_LABEL, TABS, originLabel, pickRow, rowHelp, savesCmd, savesRows, savesSummary, setTab, sv, when, type SaveRow } from '../lib/saves.svelte';
  import Hints from '../components/Hints.svelte';

  const ROW_H = 52;
  const VIS = 7;
  const H = 28 + 128 + VIS * ROW_H + 8 + 84;
  const rows = $derived.by(() => { void sv.tab; void sv.confirm; void sv.busy; return savesRows(); });
  const off = $derived(Math.max(0, Math.min(sv.row - 3, rows.length - VIS)));
  const focused = $derived(rows[sv.row]);
  const info = $derived(sv.info);

  function text(r: SaveRow): { l: string; v: string; danger?: boolean; dim?: boolean } {
    const i = info!;
    switch (r.k) {
      case 'backup': return { l: '＋  Copiar ahora', v: sv.busy === 'backup' ? 'Copiando…' : i.copies.length ? `Última: ${when(i.copies[0].createdMs)}` : 'Aún no hay copias' };
      case 'copy': {
        const c = i.copies[r.i];
        return sv.confirm === r.i
          ? { l: 'Pulsa A otra vez para restaurar', v: when(c.createdMs), danger: true }
          : { l: when(c.createdMs), v: `${KIND_LABEL[c.kind]} · ${fmtBytes(c.size)}` };
      }
      case 'download': return { l: '↓  Descargar la base de Ludusavi', v: sv.busy === 'download' ? 'Descargando…' : '18 MB' };
      case 'loc': {
        const l = i.locs[r.i];
        const v = !l.exists ? 'No existe' : l.kind === 'registry' ? originLabel(l) : `${originLabel(l)} · ${fmtBytes(l.size)}`;
        return { l: l.place, v, dim: !l.enabled };
      }
      case 'adddir': return { l: '＋  Añadir carpeta…', v: '' };
      case 'addfile': return { l: '＋  Añadir archivo…', v: '' };
    }
  }

  const hints = $derived.by<Array<[string, string, string?]>>(() => {
    const r = focused;
    const h: Array<[string, string, string?]> = [];
    if (r?.k === 'loc') h.push(['A', info!.locs[r.i].enabled ? 'No copiar' : 'Copiar', 'a']);
    else if (r?.k === 'copy') h.push(['A', 'Restaurar', 'a']);
    else if (r) h.push(['A', 'Elegir', 'a']);
    if (r?.k === 'loc' && info!.locs[r.i].origin === 'manual') h.push(['X', 'Quitar', 'x']);
    h.push(['Y', sv.tab === 'locs' && r?.k === 'loc' ? 'Abrir ubicación' : 'Abrir copias', 'y'], ['LB', 'Pestaña', 'lb'], ['B', 'Atrás', 'b']);
    return h;
  });
  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; pickRow(i); }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) savesCmd('b'); }}>
  <div class="glass setpanel pickpanel savepanel" style="--acc:{app.chrome.c};height:{H}px;top:{(720 - H) / 2}px">
    <div class="face">
      <div class="savehead">
        <h2>Partidas guardadas</h2>
        <div class="savetabs">
          {#each TABS as [id, label]}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <span class="savetab" class:on={sv.tab === id} onclick={() => setTab(id)}>{label}</span>
          {/each}
        </div>
      </div>
      <p class="msub savesum"><b>{info?.name ?? ''}</b> · {savesSummary()}</p>
      <div class="slist" style="height:{VIS * ROW_H + 8}px">
        <div class="strack" style="transform:translateY({-off * ROW_H}px)">
          {#each rows as r, i}
            {@const t = text(r)}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="srow" class:f={sv.row === i} class:danger={t.danger} class:dim={t.dim} data-row={i} onmousemove={(e) => hover(e, i)} onclick={() => { pickRow(i); savesCmd('a'); }}>
              {#if r.k === 'loc'}<span class="tog" class:on={!t.dim}><i></i></span>{/if}
              <span class="sl">{t.l}</span>
              <span class="sv">{t.v}</span>
            </div>
          {/each}
        </div>
      </div>
      {#if rows.length > VIS}
        <div class="sscroll" style="top:128px;height:{VIS * ROW_H}px"><i style="height:{(VIS / rows.length) * 100}%;top:{(off / rows.length) * 100}%"></i></div>
      {/if}
      <p class="savedet">{rowHelp(focused)}</p>
    </div>
  </div>
  <Hints list={hints} />
</div>

<style>
  .savehead { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .savehead h2 { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .savetabs { flex: none; display: flex; gap: 4px; padding: 4px; border-radius: 16px; background: rgba(214, 220, 230, 0.55); }
  .savetab { padding: 6px 14px; border-radius: 12px; font-size: 15px; font-weight: 800; color: #5f6776; cursor: pointer; }
  .savetab.on { background: var(--acc); color: var(--acc-ink); box-shadow: 0 4px 10px -6px color-mix(in srgb, var(--acc) 70%, black); }
  .savesum { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .savesum b { font-weight: 800; color: #3a414e; }
  .savepanel .srow .sl { overflow: hidden; text-overflow: ellipsis; }
  .savepanel .srow .sv { max-width: 280px; }
  .savepanel .srow.dim:not(.f) { color: #9aa1ad; }
  .tog { flex: none; position: relative; width: 38px; height: 22px; border-radius: 11px; background: rgba(120, 130, 150, 0.35); transition: background 0.15s; }
  .tog i { position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #fff; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25); transition: left 0.15s; }
  .tog.on { background: #3fb950; }
  .tog.on i { left: 19px; }
  .savedet {
    margin: 10px 4px 0; height: 62px; overflow: hidden; border-radius: 14px; padding: 8px 14px;
    background: rgba(214, 220, 230, 0.45); font-size: 13.5px; font-weight: 600; line-height: 1.35; color: #4f5766;
    overflow-wrap: anywhere;
  }
</style>
