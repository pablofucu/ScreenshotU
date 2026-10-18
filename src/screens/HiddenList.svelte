<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { hiddenCmd } from '../lib/ops.svelte';
  import Picker from '../components/Picker.svelte';

  const rows = $derived(
    app.lib.hidden.map((h) => ({ l: h.name, v: `${app.lib.platforms.find((p) => p.id === h.platform)?.name ?? h.platform} · Mostrar` })),
  );
</script>

<Picker
  title="Juegos ocultos" sub="Pulsa A para mostrar un juego otra vez. Sus archivos nunca se tocan."
  {rows} row={app.hiddenRow} empty="No hay juegos ocultos. Se ocultan desde la ficha del juego (Y › Ocultar juego)."
  onhover={(i) => (app.hiddenRow = i)} onpick={() => hiddenCmd('a')} onclose={() => hiddenCmd('b')}
  hints={[['A', 'Mostrar', 'a'], ['B', 'Atrás', 'b']]}
/>
