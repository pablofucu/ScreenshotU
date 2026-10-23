<script lang="ts">
  // «Nueva colección» / «Editar colección»: nombre, juegos, color, orden entre colecciones y borrar.
  import { app } from '../lib/store.svelte';
  import { RAINBOW } from '../lib/icons';
  import { collFormAct, collFormCmd, collFormRows, collSuggestPick, nGames, platformGames, setFormColor, type CollRow } from '../lib/coleccion.svelte';
  import { autoHex } from '../lib/collections';
  import Hints from '../components/Hints.svelte';
  import Picker from '../components/Picker.svelte';

  const f = $derived(app.collForm);
  const rows = $derived(collFormRows());
  const plat = $derived(app.lib.platforms.find((p) => p.id === f.platform));
  const sub = $derived(`${plat?.name ?? ''} · no se mueve ni se copia ninguna captura`);
  // Color automático: el que saldría de las portadas de los juegos elegidos.
  const auto = $derived(autoHex(platformGames(f.platform).filter((t) => f.members.includes(t.id))));
  const H = $derived(28 + 118 + rows.length * 58 + 30);
  const label = (r: CollRow) => {
    switch (r) {
      case 'name': return 'Nombre';
      case 'games': return 'Juegos';
      case 'color': return 'Color';
      case 'up': return 'Mover arriba';
      case 'down': return 'Mover abajo';
      case 'del': return f.confirm ? 'Pulsa A otra vez para borrarla' : 'Borrar colección';
      case 'save': return f.id ? 'Guardar cambios' : 'Crear colección';
    }
  };
  const value = (r: CollRow) => {
    switch (r) {
      case 'name': return f.name || 'Pulsa A para escribirlo';
      case 'games': return f.members.length ? nGames(f.members.length) : 'Pulsa A para elegirlos';
      case 'up': return 'entre las de esta categoría';
      case 'down': return 'entre las de esta categoría';
      default: return '';
    }
  };

  let lastXY = '';
  function hover(e: MouseEvent, i: number) {
    const xy = e.clientX + ',' + e.clientY;
    if (xy !== lastXY) { lastXY = xy; f.row = i; f.confirm = false; }
  }
</script>

{#if f.step === 'suggest'}
  <Picker
    title="Nueva colección" sub="Juegos de {plat?.name ?? ''} con un comienzo en común. Elige uno o empieza en blanco."
    rows={[...f.sugg.map((s) => ({ l: s.name, v: nGames(s.ids.length) })), { l: 'En blanco', v: 'Elegir los juegos a mano' }]}
    row={f.row} visible={Math.min(8, f.sugg.length + 1)}
    onhover={(i) => (f.row = i)} onpick={(i) => collSuggestPick(i)} onclose={() => collFormCmd('b')}
    hints={[['A', 'Elegir', 'a'], ['B', 'Cancelar', 'b']]}
  />
{:else}
  <div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) collFormCmd('b'); }}>
    <div class="glass setpanel cfpanel" style="--acc:{app.chrome.c};height:{H}px;top:{(720 - H) / 2}px">
      <div class="face">
        <h2>{f.id ? 'Editar colección' : 'Nueva colección'}</h2>
        <p class="msub">{sub}</p>
        {#each rows as r, i}
          {#if r === 'color'}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="srow accrow" class:f={f.row === i} onmousemove={(e) => hover(e, i)}>
              <span class="sl">Color</span>
              <span class="swl">
                <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                <span class="cauto" class:sel={f.color === null} title="Automático: sale de las portadas" onclick={() => { f.row = i; setFormColor(null); }}>
                  <i class="dot" style="background:{auto ?? app.chrome.c}"></i>Automático
                </span>
                {#each RAINBOW as s, k}
                  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                  <i class="big" class:sel={f.color === k} style="background:{s.c}" title={s.n} onclick={() => { f.row = i; setFormColor(k); }}></i>
                {/each}
              </span>
            </div>
          {:else}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="srow" class:f={f.row === i} class:danger={r === 'del'} onmousemove={(e) => hover(e, i)} onclick={() => collFormAct(i)}>
              <span class="sl">{label(r)}</span>
              <span class="sv">{value(r)}</span>
            </div>
          {/if}
        {/each}
      </div>
    </div>
    <Hints list={[
      ['A', 'Elegir', 'a'],
      ...(rows[f.row] === 'color' ? [['←→', 'Color'] as [string, string]] : []),
      ['Start', 'Guardar', 'menu'], ['B', 'Atrás', 'b'],
    ]} />
  </div>
{/if}

<style>
  .cfpanel .face{gap:4px}
  .cfpanel :global(.msub){flex:none}
  .cfpanel .srow{height:54px}
  .cfpanel .srow.accrow{height:60px}
  .cauto{display:flex;align-items:center;gap:8px;height:34px;padding:0 12px 0 8px;border-radius:11px;font-size:15px;font-weight:800;cursor:pointer;color:#4a5260;
    background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(235,239,245,.85));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.3)}
  .cauto .dot{width:18px;height:18px;border-radius:6px}
  .cauto.sel{outline:4px solid #3a414e;outline-offset:2px}
  .srow.f .cauto.sel{outline-color:#fff}
</style>
