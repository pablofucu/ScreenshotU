<script lang="ts">
  // Configuracion del texthook (hilos, filtros, personajes, codigos H...) dentro de la pagina de lectura: es la misma
  // interfaz que la ventana «Texthook» de la app, montada en un Shadow DOM para que sus estilos no choquen con los de aqui.
  import { mount, unmount } from 'svelte';
  import HookApp from '../hook/App.svelte';
  import hookCss from '../hook/hook.css?inline';
  import tokensCss from '../tokens.css?inline';
  import type { HookApi } from '../hook/api';

  let { api, onclose }: { api: HookApi | null; onclose: () => void } = $props();

  // `:root`/`html,body` de la ventana propia pasan al anfitrion del Shadow DOM. Los tokens van delante:
  // el `div#app` de dentro los recibe igual que en la ventana propia.
  const css = tokensCss + hookCss.replace(/:root\s*\{/g, ':host{').replace(/html,body\s*\{[^}]*\}/g, '');

  function shadow(host: HTMLElement) {
    const root = host.attachShadow({ mode: 'open' });
    const style = document.createElement('style');
    style.textContent = css;
    const target = document.createElement('div');
    target.id = 'app';
    root.append(style, target);
    const app = mount(HookApp, { target, props: { api, onclose } });
    return { destroy: () => void unmount(app) };
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="hookpanel" use:shadow></div>

<style>
  .hookpanel {
    position: fixed;
    inset: 0;
    z-index: 200;
  }
</style>
