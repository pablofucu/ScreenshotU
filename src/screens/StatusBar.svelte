<script lang="ts">
  import { z } from '../lib/format';
  import { app, LIST_COLOR, STATUS_LABEL, STATUS_ORDER } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { popOverlay } from '../lib/controller.svelte';
  import { statusBarCmd, statusBarPick } from '../lib/milista.svelte';
  import Hints from '../components/Hints.svelte';

  const sb = $derived(app.statusBar);
  const t = $derived(app.platform?.titles[sb.title]);
  const fin = $derived(sb.step === 'finish');
  const MONTHS = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
  const color = $derived(LIST_COLOR[fin ? 'finished' : STATUS_ORDER[sb.row]]);
  const dateBoxes = $derived([[0, z(sb.d)], [1, `${z(sb.m)} · ${MONTHS[sb.m - 1]}`], [2, String(sb.y)]] as Array<[number, string]>);

  function pick(i: number) { sb.row = i; void statusBarPick(); }
  function field(f: number) { sb.field = f; }
  function step(f: number, dir: number) { sb.field = f; statusBarCmd(dir > 0 ? 'up' : 'down'); }
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) { if (fin) sb.step = 'pick'; else popOverlay(); } }}>
  <div class="glass modal menupanel spanel" class:fin style="--acc:{color.c};--acc-ink:white">
    <div class="face">
      {#if !fin}
        <h2>Estado del juego</h2>
        <p class="msub">{t?.name ?? ''}</p>
        <div class="schips">
          {#each STATUS_ORDER as s, i}
            <button type="button" class="schip" class:f={sb.row === i} class:cur={(t?.status ?? 'none') === s} style="--c:{LIST_COLOR[s].c}" onclick={() => pick(i)}>
              <i>{#if s === 'finished'}{@html ICON.check}{/if}</i>
              <b>{STATUS_LABEL[s]}</b>
            </button>
          {/each}
        </div>
        <p class="shint">{(t?.status ?? 'none') === STATUS_ORDER[sb.row] ? 'Es el estado actual.' : 'A para guardar.'}{STATUS_ORDER[sb.row] === 'finished' ? ' Después eliges la fecha y la nota.' : ''}</p>
      {:else}
        <h2>Terminado</h2>
        <p class="msub">{t?.name ?? ''}</p>
        <div class="frow">
          <div class="flab">Fecha</div>
          {#each dateBoxes as [f, v]}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <div class="fbox" class:f={sb.field === f} onclick={() => field(f)}>
              <button class="inc" tabindex="-1" onclick={(e) => { e.stopPropagation(); step(f, 1); }}>▲</button>
              <b>{v}</b>
              <button class="dec" tabindex="-1" onclick={(e) => { e.stopPropagation(); step(f, -1); }}>▼</button>
            </div>
          {/each}
        </div>
        <div class="frow">
          <div class="flab">Nota</div>
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="fbox stars" class:f={sb.field === 3} onclick={() => field(3)}>
            {#each [1, 2, 3, 4, 5] as n}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <span class="rstar" class:on={sb.rating >= n} onclick={(e) => { e.stopPropagation(); sb.field = 3; sb.rating = sb.rating === n ? 0 : n; }}>★</span>
            {/each}
            <small>{sb.rating ? `${sb.rating} de 5` : 'Sin nota'}</small>
          </div>
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="fbtns">
          <button type="button" class="b-main" onclick={() => statusBarCmd('a')}>Guardar como terminado</button>
          <button type="button" class="b-soft pill" onclick={() => statusBarCmd('y')}>Guardar y añadir a una colección</button>
        </div>
      {/if}
    </div>
  </div>
  {#if fin}
    <Hints list={[['←→', 'Campo'], ['↑↓', 'Valor'], ['A', 'Guardar', 'a'], ['Y', 'Guardar y a una colección', 'y'], ['B', 'Atrás', 'b']]} />
  {:else}
    <Hints list={[['←→', 'Elegir'], ['A', 'Confirmar', 'a'], ['B', 'Cerrar', 'b']]} />
  {/if}
</div>

<style>
  .spanel{left:170px;width:940px;top:190px;height:250px;transition:height var(--d-3) var(--e-soft),top var(--d-3) var(--e-soft)}
  .spanel.fin{top:140px;height:376px}
  .schips{display:flex;gap:var(--s-3);margin-top:var(--s-1)}
  .schip{flex:1;height:84px;border-radius:var(--r-m);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:var(--s-2);cursor:pointer;color:var(--ink);font:800 var(--t-l)/1 var(--round);
    background:var(--glass-in);box-shadow:inset 0 1px 0 var(--hl-ring),inset 0 0 0 1px var(--line);transition:transform var(--d-2) var(--e-bounce),box-shadow var(--d-2) var(--e-soft),background var(--d-1) var(--e-soft)}
  .schip i{width:var(--i-m);height:var(--i-m);border-radius:50%;display:grid;place-items:center;background:var(--c);box-shadow:inset 0 1px 0 rgba(255,255,255,.6)}
  .schip i :global(svg){width:70%;height:70%}
  .schip.cur{box-shadow:inset 0 1px 0 var(--hl-ring),0 0 0 2px var(--c)}
  .schip.f{transform:scale(1.06);color:white;text-shadow:0 1px 2px rgba(0,0,0,.28);
    background:linear-gradient(180deg,color-mix(in srgb,var(--c) 70%,var(--tint-base)),var(--c) 60%,color-mix(in srgb,var(--c) 82%,black));
    box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--c) 75%,black),0 8px 18px -8px var(--c)}
  .schip.f i{background:rgba(255,255,255,.9)}
  .shint{margin:var(--s-2) var(--s-1) 0;font:600 var(--t-m)/1.35 var(--round);color:var(--ink-2)}
  .frow{display:flex;align-items:center;gap:var(--s-3);margin-top:var(--s-1)}
  .flab{width:80px;font:800 var(--t-l)/1 var(--round);color:var(--ink-2)}
  .fbox.stars{min-width:360px;padding:0 var(--s-5);gap:var(--s-2)}
  .rstar{font-size:var(--t-3xl);line-height:1;cursor:pointer;color:var(--line-2);transition:color var(--d-1) var(--e-soft)}
  .rstar.on{color:var(--warn);text-shadow:0 1px 2px rgba(120,80,0,.35)}
  .fbox.f .rstar{color:rgba(255,255,255,.5)}
  .fbox.f .rstar.on{color:white}
  .fbox.stars small{margin-left:var(--s-2);font:700 var(--t-l)/1 var(--round);color:var(--ink-2)}
  .fbox.stars.f small{color:var(--acc-ink)}
</style>
