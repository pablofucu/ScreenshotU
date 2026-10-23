<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { OSK_ROWS, keyLabel, oskCmd, oskPress } from '../lib/osk.svelte';
  import Hints from './Hints.svelte';
  import { makeMoved } from '../lib/pointer';

  const moved = makeMoved();
  function hover(e: MouseEvent, r: number, c: number) {
    if (moved(e)) { app.osk.r = r; app.osk.c = c; }
  }
  function click(e: MouseEvent) {
    const h = (e.target as HTMLElement).closest<HTMLElement>('[data-hact]');
    if (h?.dataset.hact) oskCmd(h.dataset.hact);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim oskscrim" onclick={click}>
  <div class="glass oskpanel" style="--acc:{app.chrome.c}">
    <div class="face">
      <div class="oskhead">
        <span class="oskl">{app.osk.label}</span>
        <p class="oskhint">{app.osk.hint}</p>
      </div>
      <div class="oskin" data-testid="osk-value"><span>{app.osk.value}</span><i class="caret"></i></div>
      <div class="oskgrid">
        {#each OSK_ROWS as row, r}
          <div class="oskrow">
            {#each row as k, c}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="key {k.kind}" class:f={app.osk.r === r && app.osk.c === c} class:on={k.kind === 'shift' && app.osk.shift}
                style="grid-column:span {k.span}" data-key={k.kind === 'char' ? k.ch : k.kind}
                onmousemove={(e) => hover(e, r, c)} onclick={(e) => { e.stopPropagation(); app.osk.r = r; app.osk.c = c; oskPress(r, c); }}
              >{keyLabel(k, app.osk.shift)}</div>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  </div>
  <Hints list={[['A', 'Escribir', 'a'], ['B', 'Borrar', 'b'], ['X', 'Espacio', 'x'], ['Y', 'Mayús.', 'y'], ['Start', 'Aceptar', 'menu']]} />
</div>
