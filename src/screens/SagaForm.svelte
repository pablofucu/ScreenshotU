<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { sagaAct, sagaCandidates, sagaCmd } from '../lib/ops.svelte';
  import Picker from '../components/Picker.svelte';

  const cands = $derived(sagaCandidates());
  const f = $derived(app.sagaForm);
  const rows = $derived([
    { l: 'Nombre', v: f.name || 'Pulsa A para escribirlo' },
    ...cands.map((c) => ({ l: `${f.members.includes(c.id) ? '✓' : '○'}  ${c.name}`, v: c.platform })),
    { l: f.id ? 'Guardar cambios' : 'Crear saga', v: `${f.members.length} ${f.members.length === 1 ? 'juego' : 'juegos'}` },
  ]);
</script>

<Picker
  title={f.id ? 'Editar saga' : 'Crear saga'}
  sub="Marca los juegos con A. La saga solo agrupa: tus capturas no se mueven ni se copian."
  {rows} row={f.row} visible={8} empty=""
  onhover={(i) => (app.sagaForm.row = i)} onpick={(i) => sagaAct(i)} onclose={() => sagaCmd('b')}
  hints={[['A', 'Marcar / elegir', 'a'], ['LB/RB', 'Saltar', 'rb'], ['B', 'Atrás', 'b']]}
/>
