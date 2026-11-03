<script lang="ts">
  // Baldosa de salida: un único botón directo para forzar el cierre del juego de inmediato.
  let { onClose }: { onClose: () => void } = $props();
</script>

<div class="glass tile t-pow" style="--i:6">
  <div class="lbl">
    <svg class="i" viewBox="0 0 24 24"><path d="M12 3v8"/><path d="M6.3 6.8a8 8 0 1 0 11.4 0"/></svg>
    Salir del juego
    <em>la sesión se guarda igual</em>
  </div>
  <div class="powrow">
    <div
      class="ctl pbtn close"
      data-f
      data-id="close"
      onclick={onClose}
      onkeydown={(e) => { if (e.key === 'Enter') onClose(); }}
      role="button"
      tabindex="0"
    >
      <span class="powi">
        <svg class="i" viewBox="0 0 24 24"><path d="M12 3v8"/><path d="M6.3 6.8a8 8 0 1 0 11.4 0"/></svg>
      </span>
      <span>
        <b>Cerrar juego</b>
        <small>Cierre inmediato del juego</small>
      </span>
    </div>
  </div>
</div>
