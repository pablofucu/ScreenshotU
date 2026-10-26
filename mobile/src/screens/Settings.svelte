<script lang="ts">
  // Pantalla de Ajustes: información del servidor conectado y botón para desconectar.
  import { ICON } from '@shared/icons';
  import { MI } from '../lib/icons';
  import { app } from '../lib/store.svelte';

  const isDemo = $derived(app.server === 'demo');
  const serverName = $derived.by(() => {
    if (app.server === 'demo') return 'Demostración';
    if (app.server) return app.server.name || app.snap?.name || 'Visor PC';
    return 'No conectado';
  });
  const serverAddress = $derived.by(() => {
    if (app.server === 'demo') return 'Biblioteca local simulada';
    if (app.server) return `${app.server.host}:${app.server.port}`;
    return 'Sin dirección';
  });

  const lastSync = $derived.by(() => {
    if (!app.syncedAt) return 'Pendiente';
    const d = new Date(app.syncedAt);
    return `${d.toLocaleDateString('es-ES', { day: 'numeric', month: 'short' })} a las ${d.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' })}`;
  });

  function handleDisconnect() {
    app.disconnect();
  }

  function handleSync() {
    void app.sync(true);
  }
</script>

<div class="pad settings-wrap">
  <!-- Tarjeta de servidor conectado -->
  <div class="glass panel">
    <div class="face settings-card">
      <div class="card-head">
        <div class="srv-badge">
          <i class="srv-icon">{@html isDemo ? ICON.saga : ICON.pc}</i>
        </div>
        <div class="srv-info">
          <span class="srv-type">{isDemo ? 'Modo de demostración' : 'Servidor conectado'}</span>
          <h2 class="srv-title">{serverName}</h2>
          <span class="srv-addr">{serverAddress}</span>
        </div>
      </div>

      <div class="meta-rows">
        <div class="meta-row">
          <span class="meta-label">Estado</span>
          <span class="meta-val status">
            <i class="sync-dot" class:off={app.offline} class:busy={app.syncing}></i>
            {app.syncing ? 'Sincronizando…' : app.offline ? 'Sin conexión (modo local)' : 'Conectado'}
          </span>
        </div>

        <div class="meta-row">
          <span class="meta-label">Última sincronización</span>
          <span class="meta-val">{lastSync}</span>
        </div>

        {#if !isDemo}
          <div class="sync-action">
            <button
              type="button"
              class="sync-btn pill"
              onclick={handleSync}
              disabled={app.syncing}
            >
              <i class="sync-btn-ic">{@html ICON.reload}</i>
              <span>{app.syncing ? 'Sincronizando…' : 'Sincronizar ahora'}</span>
            </button>
          </div>
        {/if}
      </div>

      <div class="actions">
        <button
          type="button"
          class="bigbtn disconnect-btn"
          onclick={handleDisconnect}
        >
          <i class="btn-ic">{@html ICON.power}</i>
          <span>Desconectar</span>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .settings-wrap {
    padding-top: 14px;
    padding-bottom: 24px;
    max-width: 480px;
    margin: 0 auto;
  }
  .settings-card {
    padding: 22px 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .srv-badge {
    width: 52px;
    height: 52px;
    border-radius: 16px;
    display: grid;
    place-items: center;
    flex: none;
    background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 45%, white), var(--acc));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7), 0 0 0 1px color-mix(in srgb, var(--acc) 70%, black), 0 8px 16px -8px color-mix(in srgb, var(--acc) 80%, black);
  }
  .srv-icon {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
  }
  .srv-icon :global(svg) {
    width: 28px;
    height: 28px;
    display: block;
  }
  .srv-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    text-align: left;
  }
  .srv-type {
    font-size: 11.5px;
    font-weight: 800;
    color: var(--dim2);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .srv-title {
    margin: 1px 0 2px;
    font-size: 20px;
    font-weight: 800;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .srv-addr {
    font-size: 13px;
    font-weight: 700;
    color: var(--dim);
    font-family: monospace, var(--round);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta-rows {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: rgba(255, 255, 255, 0.6);
    border-radius: 18px;
    padding: 12px 14px;
    border: 1px solid rgba(110, 125, 150, 0.15);
  }
  .meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
  }
  .meta-label {
    font-weight: 700;
    color: var(--dim);
  }
  .meta-val {
    font-weight: 800;
    color: var(--ink2);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .sync-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #3bb54a;
    flex: none;
  }
  .sync-dot.off {
    background: #e8384f;
  }
  .sync-dot.busy {
    background: var(--acc);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    50% { opacity: 0.3; }
  }
  .sync-action {
    padding-top: 4px;
    border-top: 1px solid rgba(110, 125, 150, 0.12);
  }
  .sync-btn {
    width: 100%;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 800;
    color: var(--acc);
    background: rgba(255, 255, 255, 0.9);
    transition: transform 0.15s, opacity 0.2s;
  }
  .sync-btn:active {
    transform: scale(0.98);
  }
  .sync-btn:disabled {
    opacity: 0.5;
  }
  .sync-btn-ic {
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
  }
  .sync-btn-ic :global(svg) {
    width: 16px;
    height: 16px;
    display: block;
    stroke: var(--acc);
  }
  .actions {
    padding-top: 6px;
  }
  .disconnect-btn {
    background: linear-gradient(180deg, #ff6b7e 0%, #e8384f 55%, #c02035 100%);
    box-shadow: inset 0 1.5px 0 rgba(255, 255, 255, 0.6), 0 0 0 1px #a81c2e, 0 8px 16px -8px rgba(232, 56, 79, 0.6);
  }
  .btn-ic {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
  }
  .btn-ic :global(svg) {
    width: 18px;
    height: 18px;
    display: block;
  }
</style>
