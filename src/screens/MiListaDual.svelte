<script lang="ts">
  import { app, LIST_COLOR } from '../lib/store.svelte';
  import { ICON } from '../lib/icons';
  import { fmtDur } from '../lib/format';
  import Thumb from '../components/Thumb.svelte';

  const P = $derived(app.platform);
  const titles = $derived(P?.titles ?? []);

  // Juegos clasificados
  const playing = $derived(
    titles
      .map((t, i) => ({ t, i }))
      .filter(({ t }) => !t.hidden && !t.members && t.status === 'playing')
  );

  const dropped = $derived(
    titles
      .map((t, i) => ({ t, i }))
      .filter(({ t }) => !t.hidden && !t.members && t.status === 'dropped')
  );

  const pending = $derived(
    titles
      .map((t, i) => ({ t, i }))
      .filter(({ t }) => !t.hidden && !t.members && t.status === 'pending')
      .sort((a, b) => (a.t.listPos ?? 1e9) - (b.t.listPos ?? 1e9) || a.t.name.localeCompare(b.t.name))
  );


  const coverOf = (i: number) => titles[i]?.cover ?? titles[i]?.shots[0]?.path ?? null;
</script>

<div class="milista-dual">
  <!-- BANDA SUPERIOR DUAL (Jugando a la izquierda + Pausa/Terminados a la derecha) -->
  <div class="top-dual-row">
    
    <!-- Columna Izquierda: JUGANDO AHORA -->
    <div class="dual-col col-playing">
      <div class="col-head">
        <span class="stag playing">
          <span class="sic">{@html ICON.play}</span>
          <b>Jugando ahora</b>
          <span class="cnt">{playing.length}</span>
        </span>
        {#if playing.length}
          <span class="subtext">
            {playing.reduce((s, { t }) => s + (t.playSeconds || 0), 0) >= 60 ? fmtDur(playing.reduce((s, { t }) => s + (t.playSeconds || 0), 0)) : ''}
          </span>
        {/if}
      </div>

      <div class="glass dual-box">
        <div class="face dual-cards">
          {#if !playing.length}
            <div class="empty-msg">
              <p>Ningún juego en curso</p>
              <small>Usa Y en la ficha de un juego para marcarlo como «Jugando»</small>
            </div>
          {:else}
            {#each playing as { t, i } (t.id)}
              {@const acc = app.titleAcc(app.p, i)}
              <div
                class="glass mini-play-card fx"
                class:f={app.focus === 'tit' + i}
                data-k="tit{i}"
                data-act="tit:{i}"
                style="--acc:{acc.c}"
              >
                <div class="face">
                  {#if coverOf(i)}
                    <Thumb path={coverOf(i)!} />
                  {:else}
                    <div class="noart"><b>{t.name}</b></div>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>

    <!-- Columna Derecha: EN PAUSA / ABANDONADOS + BANNER TERMINADOS -->
    <div class="dual-col col-dropped">
      <div class="col-head">
        <span class="stag dropped">
          <span class="sic">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M10 15V9m4 6V9"/></svg>
          </span>
          <b>En pausa</b>
          <span class="cnt">{dropped.length}</span>
        </span>
      </div>

      <div class="glass dual-box">
        <div class="face dual-cards">
          {#if !dropped.length}
            <div class="empty-msg compact">
              <p>Sin juegos en pausa</p>
            </div>
          {:else}
            {#each dropped as { t, i } (t.id)}
              {@const acc = app.titleAcc(app.p, i)}
              <div
                class="glass mini-play-card dropped-card fx"
                class:f={app.focus === 'tit' + i}
                data-k="tit{i}"
                data-act="tit:{i}"
                style="--acc:{acc.c}"
              >
                <div class="face">
                  {#if coverOf(i)}
                    <Thumb path={coverOf(i)!} />
                  {:else}
                    <div class="noart"><b>{t.name}</b></div>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>

  </div>

  <!-- BANDA INFERIOR (Gran Balda de Pendientes con ancho completo) -->
  <div class="pending-section">
    <div class="col-head">
      <span class="stag pending">
        <span class="sic">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 3"/></svg>
        </span>
        <b>Cola de Pendientes</b>
        <span class="cnt">{pending.length}</span>
      </span>
    </div>

    <div class="glass pending-box">
      <div class="face pending-track">
        {#if !pending.length}
          <div class="empty-msg wide">
            <p>No tienes juegos pendientes en {P?.name}</p>
            <small>En la ficha de cualquier juego, pulsa Y › Estado para añadirlo a tu cola de pendientes.</small>
          </div>
        {:else}
          {#each pending as { t, i } (t.id)}
            {@const acc = app.titleAcc(app.p, i)}
            <div
              class="glass pend-cover-card fx"
              class:f={app.focus === 'tit' + i}
              data-k="tit{i}"
              data-act="tit:{i}"
              style="--acc:{acc.c}"
            >
              <div class="face">
                {#if coverOf(i)}
                  <Thumb path={coverOf(i)!} />
                {:else}
                  <div class="noart"><b>{t.name}</b></div>
                {/if}
                <div class="p-name">{t.name}</div>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .milista-dual {
    position: absolute;
    left: 80px;
    right: 80px;
    top: 118px;
    bottom: 56px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    user-select: none;
  }

  /* Filas y columnas */
  .top-dual-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    height: 206px;
  }
  .dual-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
  }

  /* Tags de estado */
  .stag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px 0 8px;
    border-radius: 999px;
    color: #fff;
    font-size: 13px;
    font-weight: 800;
    box-shadow: 0 2px 6px rgba(0,0,0,0.15);
  }
  .stag.playing { background: var(--st-playing, #3BB54A); }
  .stag.dropped { background: var(--st-dropped, #718096); }
  .stag.pending { background: var(--st-pending, #F27A1A); }
  .stag .sic { display: grid; place-items: center; }
  .stag .sic :global(svg) { width: 14px; height: 14px; }
  .stag .cnt {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(0,0,0,0.2);
  }
  .subtext {
    font-size: 13px;
    font-weight: 700;
    color: #718096;
    margin-left: auto;
  }

  /* Cajas de cristal */
  .dual-box {
    position: relative !important;
    left: auto !important;
    top: auto !important;
    width: 100% !important;
    height: 168px !important;
    --r: 20px;
    --bw: 6px;
  }
  .dual-box > .face {
    width: 100% !important;
    height: 100% !important;
  }
  .dual-cards {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    overflow-x: auto;
    background: linear-gradient(180deg, rgba(255,255,255,0.7), rgba(240,244,250,0.55));
  }

  /* Tarjetas pequeñas de jugando / pausa */
  .mini-play-card {
    position: relative !important;
    left: auto !important;
    top: auto !important;
    width: 108px !important;
    height: 144px !important;
    flex: none !important;
    cursor: pointer;
    --r: 15px;
    --bw: 5px;
  }
  .mini-play-card .face {
    position: relative;
    border-radius: 10px;
    overflow: hidden;
    background: #cbd5e1;
  }
  .dropped-card {
    opacity: 0.88;
  }

  /* Gran balda de Pendientes */
  .pending-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .pending-box {
    position: relative !important;
    left: auto !important;
    top: auto !important;
    width: 100% !important;
    flex: 1;
    --r: 22px;
    --bw: 6px;
  }
  .pending-box > .face {
    width: 100% !important;
    height: 100% !important;
  }
  .pending-track {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    overflow-x: auto;
    background: linear-gradient(180deg, rgba(255,255,255,0.75), rgba(238,242,248,0.6));
  }

  .pend-cover-card {
    position: relative !important;
    left: auto !important;
    top: auto !important;
    width: 108px !important;
    height: 146px !important;
    flex: none !important;
    cursor: pointer;
    --r: 16px;
    --bw: 5px;
  }
  .pend-cover-card .face {
    position: relative;
    border-radius: 11px;
    overflow: hidden;
    background: #cbd5e1;
  }
  .pend-cover-card .p-name {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    padding: 6px;
    background: linear-gradient(180deg, transparent 40%, rgba(15,23,42,0.88) 100%);
    color: #fff;
    font-size: 11px;
    font-weight: 800;
    line-height: 1.15;
    text-shadow: 0 1px 2px rgba(0,0,0,0.8);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  /* Mensajes vacíos */
  .empty-msg {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: #64748b;
  }
  .empty-msg p { margin: 0 0 2px; font-weight: 800; font-size: 14px; }
  .empty-msg small { font-size: 12px; }
  .empty-msg.compact p { font-size: 13px; }
  .empty-msg.wide { height: 100%; }

  .noart {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 6px;
    text-align: center;
    color: #475569;
    font-size: 11px;
  }
</style>
