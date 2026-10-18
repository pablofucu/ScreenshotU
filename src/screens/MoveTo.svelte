<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { moveCmd } from '../lib/ops.svelte';
  import Picker from '../components/Picker.svelte';

  const m = $derived(app.moveTo);
  const n = $derived(m.paths.length);
  const rows = $derived(
    m.targets.map((t, i) =>
      m.confirm && i === m.row
        ? { l: `Pulsa A otra vez: mover ${n === 1 ? 'la captura' : n + ' capturas'} a «${t.name}»`, v: '' }
        : { l: t.name, v: t.platformName },
    ),
  );
</script>

<Picker
  title="Mover a otro juego" sub={n === 1 ? 'Una captura · se mueve dentro de tu biblioteca' : `${n} capturas · se mueven dentro de tu biblioteca`}
  {rows} row={m.row} visible={8}
  onhover={(i) => { m.row = i; m.confirm = false; }} onpick={() => moveCmd('a')} onclose={() => moveCmd('b')}
  hints={[['A', 'Mover', 'a'], ['B', 'Atrás', 'b'], ['LB/RB', 'Saltar', 'rb']]}
/>
