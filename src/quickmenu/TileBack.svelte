<script lang="ts">
  // Baldosa solo de la tablet: vuelve al visor (la partida sigue en marcha en el PC).
  let { onBack }: { onBack: () => void } = $props();
</script>

<div class="glass tile t-back" style="--i:5">
  <div class="lbl">
    <svg class="i" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="14" rx="3"/><path d="M8 21h8M12 18v3"/></svg>
    Visor
    <em>la partida sigue en el PC</em>
  </div>
  <div class="powrow">
    <div
      class="ctl pbtn"
      data-f
      data-id="back"
      onclick={onBack}
      onkeydown={(e) => { if (e.key === 'Enter') onBack(); }}
      role="button"
      tabindex="0"
    >
      <span class="powi">
        <svg class="i" viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7"/></svg>
      </span>
      <span>
        <b>Volver a la app</b>
        <small>Abre el visor y deja el juego</small>
      </span>
    </div>
  </div>
</div>
