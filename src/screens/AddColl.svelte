<script lang="ts">
  // «Añadir a una colección»: las colecciones de la categoría del juego y «Nueva colección».
  import { app } from '../lib/store.svelte';
  import { addCollAct, addCollCmd, platformCollections } from '../lib/coleccion.svelte';
  import { nGames } from '../lib/format';
  import Picker from '../components/Picker.svelte';

  const t = $derived(app.platform?.titles[app.addColl.ti]);
  const list = $derived(platformCollections());
  const rows = $derived([
    ...list.map((c) => ({ l: c.name, v: t && c.members.includes(t.id) ? 'Ya está' : nGames(c.members.length) })),
    { l: 'Nueva colección…', v: t ? `con «${t.name}»` : '' },
  ]);
</script>

<Picker
  title="Añadir a una colección" sub={t?.name ?? ''} {rows} row={app.addColl.row} visible={Math.min(8, rows.length)} empty=""
  onhover={(i) => (app.addColl.row = i)} onpick={(i) => void addCollAct(i)} onclose={() => addCollCmd('b')}
  hints={[['A', 'Añadir', 'a'], ['B', 'Atrás', 'b']]}
/>
