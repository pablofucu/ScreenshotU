<script lang="ts">
  import { ni, importLabels, nintendoImportCmd, fmtMin } from '../lib/nintendo.svelte';
  import Picker from '../components/Picker.svelte';

  const rows = $derived(importLabels());
  const sub = $derived(
    `Mínimo ${fmtMin(ni.minMinutes)} · ${ni.demos} demos y ${ni.ignored} ignorados no salen. A cambia la fila; en «Dudoso», ‹ › elige la ficha.`,
  );
</script>

<Picker
  title="Importar juegos de Nintendo" {sub} {rows} row={ni.row} visible={8}
  empty="No hay juegos que importar."
  onhover={(i) => (ni.row = i)} onpick={() => nintendoImportCmd('a')} onclose={() => nintendoImportCmd('b')}
  hints={[['A', 'Cambiar / Importar', 'a'], ['B', 'Atrás', 'b']]}
/>
