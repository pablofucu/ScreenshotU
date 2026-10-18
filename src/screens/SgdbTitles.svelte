<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { ART_KINDS, KIND_LABEL, sg, sgdbTitleRefs, sgdbTitlesCmd } from '../lib/sgdb.svelte';
  import Picker from '../components/Picker.svelte';

  const rows = $derived(
    sgdbTitleRefs().map(({ pi, ti }) => {
      const p = app.lib.platforms[pi], t = p.titles[ti];
      const miss = ART_KINDS.filter((k) => !t[k]).map((k) => KIND_LABEL[k].toLowerCase());
      return { l: t.name, v: miss.length ? `${p.name} · sin ${miss.join(', ')}` : p.name };
    }),
  );
</script>

<Picker
  title="Grafismos de SteamGridDB" sub="Elige el juego al que quieres cambiarle la portada, el hero o el logo"
  {rows} row={sg.titlesRow} visible={8}
  onhover={(i) => { sg.titlesRow = i; }} onpick={() => sgdbTitlesCmd('a')} onclose={() => sgdbTitlesCmd('b')}
  hints={[['A', 'Elegir', 'a'], ['B', 'Atrás', 'b'], ['LB/RB', 'Saltar', 'rb']]}
/>
