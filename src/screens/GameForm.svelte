<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { formAct, formRows, fileLabel, isArtRow, type FormRow } from '../lib/ops.svelte';
  import { cmd } from '../lib/controller.svelte';
  import Hints from '../components/Hints.svelte';

  const rows = $derived(formRows());
  const RH = $derived(rows.length > 7 ? 42 : 52);
  const H = $derived(28 + 44 + 48 + rows.length * (RH + 6) + 52);
  const plat = $derived(app.config.platforms.find((p) => p.id === app.form.platform)?.name ?? app.form.platform);
  const TITLE = { exe: 'Añadir juego (.exe)', emu: 'Añadir emulador + ROM', captures: 'Solo capturas' } as const;
  const L: Record<FormRow, string> = {
    exe: '', rom: 'Juego o ROM', name: 'Nombre', args: 'Argumentos', platform: 'Plataforma',
    hero: 'Hero (opcional)', logo: 'Logo (opcional)', cover: 'Portada (opcional)', save: 'Guardar',
  };
  const label = (r: FormRow) => (r === 'exe' ? (app.form.kind === 'emu' ? 'Emulador (.exe)' : 'Ejecutable (.exe)') : L[r]);
  const short = (p: string, empty = 'Elegir…') => (p ? (p.length > 34 ? '…' + p.slice(-33) : p) : empty);
  const text = (r: FormRow): string => {
    const f = app.form;
    if (r === 'exe') return short(f.exe);
    if (r === 'rom') return short(f.rom);
    if (r === 'name') return f.name || 'Escribir…';
    if (r === 'args') return f.args || (f.kind === 'emu' ? '"{rom}" -f' : '(opcional)');
    if (r === 'platform') return `◀ ${plat} ▶`;
    if (isArtRow(r)) return f[r as 'hero' | 'logo' | 'cover'] ? short(fileLabel(f[r as 'hero' | 'logo' | 'cover'])) : 'Ninguno';
    return '';
  };
  const empty = (r: FormRow) => {
    const f = app.form;
    return (r === 'name' && !f.name) || (r === 'args' && !f.args) || (isArtRow(r) && !f[r as 'hero' | 'logo' | 'cover']);
  };

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; app.formRow = i; }
  }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) cmd('b'); }}>
  <div class="glass formpanel" style="--acc:{app.chrome.c};height:{H}px;top:{(720 - H) / 2}px">
    <div class="face">
      <h2>{TITLE[app.form.kind]}</h2>
      {#each rows as r, i}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="srow frow" class:f={app.formRow === i} class:save={r === 'save'} style="height:{RH}px" data-frow={r}
          onmousemove={(e) => hover(e, i)} onclick={() => { app.formRow = i; void formAct(i, 0); }}>
          <span class="sl fl">{label(r)}</span>
          {#if r !== 'save'}<span class="sv" class:ph={empty(r)}>{text(r)}</span>{/if}
        </div>
      {/each}
      <p class="fhint">
        {#if app.form.kind === 'emu'}En los argumentos, <b>{'{rom}'}</b> se sustituye por la ruta del juego.
        {:else if app.form.kind === 'captures'}Crea la carpeta Plataforma/Nombre para poder añadir capturas; no hay botón Jugar.
        {:else}El nombre decide la carpeta de capturas (Plataforma/Nombre).{/if}
        <br />Los grafismos se <b>copian</b> a la carpeta del juego; los originales no se tocan.
      </p>
    </div>
  </div>
  <Hints list={[['A', 'Elegir / escribir', 'a'], ['X', 'Quitar', 'x'], ['B', 'Volver', 'b']]} />
</div>
