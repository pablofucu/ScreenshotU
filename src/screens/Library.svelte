<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { fmtBytes } from '../lib/controller.svelte';
  import { goLabel, lb, libForced, libraryAct, libraryCmd, libRows, type LibRow } from '../lib/library.svelte';
  import Hints from '../components/Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const mode = $derived(lb.info?.mode ?? 'library');
  const title = $derived(
    lb.step === 'done' ? 'Biblioteca lista'
      : lb.step === 'copying' ? 'Copiando la biblioteca…'
      : mode === 'legacy' ? 'Elige dónde guardar tu biblioteca'
      : mode === 'missing' ? 'No se encuentra la biblioteca'
      : 'Mover la biblioteca',
  );
  const intro = $derived(
    mode === 'legacy'
      ? 'Desde ahora todo lo del visor vive en una sola carpeta: capturas, copias de partidas, grafismos, glosario, base de datos y ajustes. Puede estar en otro disco. Se copia lo que tienes ahora; los originales no se tocan.'
      : mode === 'missing'
        ? `Estaba en ${lb.info?.dir}. Si es un disco externo, conéctalo y pulsa «Reintentar». Si la has movido, elige su carpeta nueva (la que tiene visor.db).`
        : `Ahora está en ${lb.info?.dir}. Se copia entera a la carpeta nueva y el visor se reinicia allí; la anterior se queda donde está hasta que la borres tú.`,
  );
  const ROWL: Record<LibRow, string> = { dest: 'Carpeta', go: '', retry: 'Reintentar', cancel: 'Cancelar' };
  const rows = $derived(libRows());
  const pct = $derived(lb.progress && lb.progress.totalBytes ? Math.min(100, (lb.progress.doneBytes / lb.progress.totalBytes) * 100) : 0);
  const items = $derived(lb.plan?.items ?? []);
  const H = $derived(lb.step === 'choose' ? Math.min(640, 250 + rows.length * 56 + (items.length ? 40 + items.length * 30 : 34)) : 330);

  const moved = makeMoved();
  function hover(e: MouseEvent, i: number) {
    if (moved(e)) { lb.row = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) libraryCmd('b'); }}>
  <div class="glass formpanel libpanel" style="--acc:{app.chrome.c};height:{H}px;top:{(720 - H) / 2}px">
    <div class="face">
      <h2>{title}</h2>
      {#if lb.step === 'choose'}
        <p class="lintro">{intro}</p>
        {#each rows as r, i (r)}
          <button type="button" class="srow" class:f={lb.row === i} class:save={r === 'go'} onmousemove={(e) => hover(e, i)} onclick={() => { lb.row = i; libraryAct(r); }}>
            {#if r === 'dest'}
              <span class="sl fl">{ROWL.dest}</span><span class="sv lpath">{lb.dest || 'Pulsa A para elegir'}</span>
            {:else}
              <span class="sl">{r === 'go' ? goLabel() : ROWL[r]}</span>
            {/if}
          </button>
        {/each}
        {#if lb.err}
          <p class="lerr">{lb.err}</p>
        {:else if lb.plan?.kind === 'existing'}
          <p class="lnote">Esa carpeta ya es una biblioteca del visor: se usa tal cual{mode === 'legacy' ? ' y no se copia nada de lo de ahora' : ''}.</p>
        {:else if items.length}
          <div class="litems">
            {#each items as it (it.key)}
              <div class="litem"><span>{it.label}</span><span class="lfrom">{it.from}</span><b>{it.key === 'db' ? '' : `${it.files} · `}{fmtBytes(it.bytes)}</b></div>
            {/each}
          </div>
          <p class="lnote">
            {lb.plan?.kind === 'resume' ? 'Había una copia a medias: se salta lo que ya está copiado. ' : ''}En total {fmtBytes(lb.plan?.bytes ?? 0)}{lb.plan?.free != null ? `; libres en ese disco: ${fmtBytes(lb.plan.free)}` : ''}.
          </p>
        {/if}
      {:else if lb.step === 'copying' && lb.progress}
        <p class="lintro">No cierres el visor. Si se corta, vuelve a elegir la misma carpeta y la copia sigue donde se quedó.</p>
        <div class="lbar"><i style="width:{pct}%"></i></div>
        <p class="lnote"><b>{lb.progress.label}</b> · {lb.progress.doneFiles} de {lb.progress.totalFiles} archivos · {fmtBytes(lb.progress.doneBytes)} de {fmtBytes(lb.progress.totalBytes)}</p>
      {:else}
        <p class="lintro">Todo está en {lb.plan?.dest}. Reiniciando el visor…</p>
        <div class="lbar"><i style="width:100%"></i></div>
      {/if}
    </div>
  </div>
  {#if lb.step === 'choose'}
    <Hints list={libForced() ? [['A', 'Seleccionar', 'a']] : [['A', 'Seleccionar', 'a'], ['B', 'Cancelar', 'b']]} />
  {/if}
</div>

<style>
  .libpanel .face { gap: 8px; }
  .lintro { margin: 0 0 6px; font-size: 15px; line-height: 1.4; font-weight: 600; color: #5f6776; }
  .lpath { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; direction: rtl; text-align: right; }
  .lerr { margin: 4px 0 0; font-size: 15px; font-weight: 700; color: #c8283e; }
  .lnote { margin: 4px 0 0; font-size: 14px; line-height: 1.35; color: #5f6776; }
  .litems { display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
  .litem { display: flex; align-items: baseline; gap: 12px; height: 28px; font-size: 15px; font-weight: 700; }
  .litem span:first-child { flex: none; width: 300px; }
  .litem .lfrom { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; font-size: 13px; color: #7a8291; }
  .litem b { flex: none; font-variant-numeric: tabular-nums; }
  .lbar { height: 14px; border-radius: 7px; background: rgba(95, 110, 135, 0.18); overflow: hidden; margin: 18px 0 6px; }
  .lbar i { display: block; height: 100%; border-radius: 7px; background: var(--gf); transition: width 0.25s; }
</style>
