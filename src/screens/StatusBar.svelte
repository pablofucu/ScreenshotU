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
  <div class="glass menupanel spanel" class:fin style="--acc:{color.c};--acc-ink:{color.dark ? '#3a2f00' : '#fff'}">
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
          <button type="button" class="fsave" onclick={() => statusBarCmd('a')}><span class="btn a">A</span>Guardar como terminado</button>
          <button type="button" class="fsave alt" onclick={() => statusBarCmd('y')}><span class="btn y">Y</span>Guardar y añadir a una colección</button>
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
  .spanel{left:170px;width:940px;top:190px;height:250px;transition:height .25s cubic-bezier(.25,1,.4,1),top .25s cubic-bezier(.25,1,.4,1)}
  .spanel.fin{top:140px;height:376px}
  .spanel .face{padding:20px 30px 16px;gap:6px}
  .schips{display:flex;gap:12px;margin-top:4px}
  .schip{flex:1;height:84px;border-radius:20px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;cursor:pointer;color:color-mix(in srgb,var(--ink),var(--ink-2));font-size:18px;font-weight:800;
    background:linear-gradient(180deg,rgba(255,255,255,.9),rgba(235,239,245,.8));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.22);transition:transform .18s,box-shadow .18s,background .18s}
  .schip i{width:22px;height:22px;border-radius:50%;display:grid;place-items:center;background:var(--c);box-shadow:inset 0 1px 0 rgba(255,255,255,.6)}
  .schip i :global(svg){width:70%;height:70%}
  .schip.cur{box-shadow:inset 0 1px 0 #fff,0 0 0 2px var(--c)}
  .schip.f{transform:scale(1.05);color:#fff;text-shadow:0 1px 2px rgba(0,0,0,.28);
    background:linear-gradient(180deg,color-mix(in srgb,var(--c) 70%,white),var(--c) 60%,color-mix(in srgb,var(--c) 82%,black));
    box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--c) 75%,black),0 8px 18px -8px var(--c)}
  .schip.f i{background:rgba(255,255,255,.9)}
  .shint{margin:8px 4px 0;font-size:16px;font-weight:600;color:var(--ink-2)}
  .frow{display:flex;align-items:center;gap:14px;margin-top:4px}
  .flab{width:80px;font-size:18px;font-weight:800;color:var(--ink-2)}
  .fbox{position:relative;min-width:150px;height:76px;border-radius:18px;display:flex;align-items:center;justify-content:center;gap:10px;cursor:pointer;
    background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(235,239,245,.85));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.25);transition:background .15s,transform .15s}
  .fbox b{font-size:30px;font-weight:800;color:var(--ink)}
  .fbox button{position:absolute;left:50%;transform:translateX(-50%);border:0;background:none;font:800 12px var(--round);color:var(--ink-3);cursor:pointer;padding:0 14px;line-height:1}
  .fbox .inc{top:6px}
  .fbox .dec{bottom:6px}
  .fbox.f{background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 80%,white),var(--acc));box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--acc) 75%,black),0 8px 16px -8px var(--acc);transform:scale(1.04)}
  .fbox.f b{color:var(--acc-ink);text-shadow:0 1px 2px rgba(0,0,0,.25)}
  .fbox.f button{color:var(--acc-ink)}
  .fbox.stars{min-width:360px;padding:0 20px;gap:8px}
  .rstar{font-size:44px;line-height:1;cursor:pointer;color:rgba(95,110,135,.35);transition:color .15s}
  .rstar.on{color:var(--warn);text-shadow:0 1px 2px rgba(120,80,0,.35)}
  .fbox.f .rstar{color:rgba(255,255,255,.5)}
  .fbox.f .rstar.on{color:#FFD84A}
  .fbox.stars small{margin-left:8px;font-size:17px;font-weight:700;color:var(--ink-2)}
  .fbox.stars.f small{color:var(--acc-ink)}
  .fbtns{margin-top:auto;margin-bottom:2px;align-self:center;display:flex;gap:14px}
  .fsave.alt{background:linear-gradient(180deg,rgba(255,255,255,.95),rgba(235,239,245,.85));color:color-mix(in srgb,var(--ink),var(--ink-2));box-shadow:inset 0 1px 0 #fff,0 0 0 1px rgba(95,110,135,.3)}
  .fsave{display:flex;align-items:center;gap:10px;height:44px;padding:0 22px 0 8px;border-radius:999px;cursor:pointer;font-size:18px;font-weight:800;color:#fff;
    background:linear-gradient(180deg,color-mix(in srgb,var(--acc) 80%,white),var(--acc));box-shadow:inset 0 1.5px 0 rgba(255,255,255,.8),0 0 0 1px color-mix(in srgb,var(--acc) 75%,black)}
</style>
