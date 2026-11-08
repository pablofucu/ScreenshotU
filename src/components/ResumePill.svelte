<script lang="ts">
  // Tablet: con una partida en marcha, píldora para volver al stream desde cualquier pantalla.
  import { app } from '../lib/store.svelte';
  import { canResume, resumeGame } from '../lib/controller.svelte';
  import { closeRunning, toast } from '../lib/controller.svelte';

  let tick = $state(Date.now());
  $effect(() => { const i = setInterval(() => { tick = Date.now(); }, 15000); return () => clearInterval(i); });

  const name = $derived(app.playing ? app.lib.platforms.flatMap((p) => p.titles).find((t) => t.gameId === app.playing!.id)?.name ?? 'Partida' : '');
  const mins = $derived(Math.max(0, Math.floor((tick - (app.playing?.since ?? tick)) / 60000)));
  // En la ficha del propio juego ya está el botón grande.
  const here = $derived(app.view === 'detail' && app.platform?.titles[app.t]?.gameId === app.playing?.id);
  let confirming = $state(false);
  let force = $state(false);

  async function close() {
    if (!confirming && !force) { confirming = true; setTimeout(() => { confirming = false; }, 4000); toast('Toca otra vez para cerrar el juego (se pierde lo no guardado)', 3000); return; }
    confirming = false;
    try {
      force = await closeRunning(force);
      if (force) { confirming = true; setTimeout(() => { confirming = false; force = false; }, 5000); toast('El juego no se cierra: toca «Forzar» para cerrarlo a la fuerza', 3500); }
    } catch (e) { toast(String(e), 3200); }
  }
</script>

{#if canResume() && app.playing && !here && app.view !== 'viewer'}
  <div class="rp" role="status">
    <span class="t">{name} · {mins} min</span>
    <button class="go" onclick={() => void resumeGame()}>Volver</button>
    <button class="x" onclick={close}>{force ? 'Forzar' : confirming ? '¿Cerrar?' : 'Cerrar'}</button>
  </div>
{/if}

<style>
  .rp { position: absolute; left: 50%; bottom: 14px; transform: translateX(-50%); display: flex; align-items: center; gap: 10px; padding: 6px 8px 6px 18px; border-radius: 999px; background: rgba(40,46,58,.78); -webkit-backdrop-filter: blur(12px); backdrop-filter: blur(12px); box-shadow: inset 0 1px 0 rgba(255,255,255,.25), 0 10px 24px -10px rgba(0,0,0,.5); color: var(--on); font-size: 15px; font-weight: 700; z-index: 48; white-space: nowrap; }
  .t { max-width: 40vw; overflow: hidden; text-overflow: ellipsis; }
  button { border: 0; border-radius: 999px; padding: 8px 16px; font: inherit; color: inherit; background: rgba(255,255,255,.14); }
  .go { background: var(--accent, #4da3ff); color: #fff; }
</style>
