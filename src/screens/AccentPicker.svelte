<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { RAINBOW } from '../lib/icons';
  import { accentForHue, inkDark } from '../lib/oklch';
  import { accentCmd, accentFocus, accentPickColor, applyAccent, cancelAccent, curTitle } from '../lib/ops.svelte';
  import Hints from '../components/Hints.svelte';

  const a = $derived(app.accentPick);
  const t = $derived(curTitle());
  const stops = Array.from({ length: 13 }, (_, i) => accentForHue(i * 30).color).join(',');
  const cur = $derived(accentPickColor());
  const custom = $derived(accentForHue(a.hue));
  const H = $derived(28 + 78 + (a.palette.length ? 4 : 3) * 68 + 96);

  function setHue(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const f = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width));
    a.hue = Math.round(f * 360) % 360;
    accentFocus(3);
  }
  const autoColor = $derived(t?.accent?.accent.color ?? '');
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) cancelAccent(); }}>
  <div class="glass menupanel accpanel" style="--acc:{app.chrome.c};top:{(720 - H) / 2}px;height:{H}px">
    <div class="face">
      <h2>Color de acento</h2>
      <p class="msub">{t?.name ?? ''}</p>

      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="srow accrow" class:f={a.row === 0} data-acc-row="0" onclick={() => { accentFocus(0); void applyAccent(); }} onmousemove={() => { if (a.row !== 0) accentFocus(0); }}>
        <span class="sl">Automático</span>
        <span class="sv">{autoColor ? 'del arte del juego' : 'arcoíris (sin arte con color)'}</span>
        {#if autoColor}<i class="dot" style="background:{autoColor}"></i>{/if}
      </div>

      {#if a.palette.length}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="srow accrow" class:f={a.row === 1} data-acc-row="1" onmousemove={() => { if (a.row !== 1) accentFocus(1, a.col); }}>
          <span class="sl">Paleta del juego</span>
          <span class="swl">
            {#each a.palette as s, k}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <i class="big" class:sel={a.row === 1 && a.col === k} style="background:{s.color}" data-swatch="p{k}" onclick={() => { accentFocus(1, k); void applyAccent(); }}></i>
            {/each}
          </span>
        </div>
      {/if}

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="srow accrow" class:f={a.row === 2} data-acc-row="2" onmousemove={() => { if (a.row !== 2) accentFocus(2, a.col); }}>
        <span class="sl">Arcoíris</span>
        <span class="swl">
          {#each RAINBOW as s, k}
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
            <i class="big" class:sel={a.row === 2 && a.col === k} style="background:{s.c}" title={s.n} data-swatch="r{k}" onclick={() => { accentFocus(2, k); void applyAccent(); }}></i>
          {/each}
        </span>
      </div>

      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="srow accrow hue" class:f={a.row === 3} data-acc-row="3" onmousemove={() => { if (a.row !== 3) accentFocus(3); }}>
        <span class="sl">Tono a mano</span>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span class="hbar" style="background:linear-gradient(90deg,{stops})" onclick={setHue} data-hue={a.hue}>
          <i class="hthumb" style="left:{(a.hue / 360) * 100}%;background:{custom.color}"></i>
        </span>
      </div>

      <div class="accprev">
        <span class="chip" style="background:{cur ?? (autoColor || app.chrome.c)};color:{inkDark(cur ?? (autoColor || app.chrome.c)) ? '#3a2f00' : '#fff'}" data-testid="acc-chip">
          {a.row === 0 ? 'Automático' : (cur ?? '')}
        </span>
        <span class="fhint">Se ve al instante en la ficha. A lo guarda; B cancela.</span>
      </div>
    </div>
  </div>
  <Hints list={[['A', 'Aplicar', 'a'], ['B', 'Cancelar', 'b']]} />
</div>
