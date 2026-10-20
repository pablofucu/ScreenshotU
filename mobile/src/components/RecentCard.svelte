<script lang="ts">
  // Tarjeta de «Seguir jugando»: el hero del juego con su logo. Sin hero, la captura más reciente
  // (o la portada desenfocada); sin logo, el nombre escrito.
  import { ICON } from '@shared/icons';
  import type { Title } from '@shared/types';
  import Img from './Img.svelte';

  let { t, playing = false }: { t: Title; playing?: boolean } = $props();
  let logoBad = $state(false);
</script>

<span class="face rc">
  <Img paths={[t.hero]} kind="preview" eager>
    {#snippet fallback()}<Img paths={[t.shots[0]?.path, t.cover]} cls={t.shots[0] ? '' : 'blurred'} eager />{/snippet}
  </Img>
  <span class="shade"></span>
  {#if t.logo && !logoBad}
    <img class="logo" alt={t.name} use:logoSrc={t.logo} onerror={() => (logoBad = true)} />
  {:else}
    <span class="name">{t.name}</span>
  {/if}
  {#if playing}<span class="playing"><i>{@html ICON.play}</i>Jugando</span>{/if}
</span>

<script lang="ts" module>
  import { urlOf } from '../lib/img';
  /** El logo va tal cual (PNG con transparencia). */
  export function logoSrc(node: HTMLImageElement, path: string) {
    let cur = path;
    const set = (p: string) => urlOf(p, 'file').then((u) => { if (cur === p) node.src = u; }, () => node.dispatchEvent(new Event('error')));
    set(path);
    return { update(p: string) { cur = p; set(p); } };
  }
</script>

<style>
  .rc{display:block}
  .rc :global(.blurred img){filter:blur(10px) brightness(.85);transform:scale(1.15)}
  .shade{position:absolute;inset:0;z-index:1;background:linear-gradient(180deg,rgba(0,0,0,0) 40%,rgba(0,0,0,.38))}
  .logo{position:absolute;z-index:2;left:8%;right:8%;bottom:10%;top:22%;width:84%;height:68%;object-fit:contain;object-position:center bottom;filter:drop-shadow(0 2px 6px rgba(0,0,0,.5))}
  .name{position:absolute;z-index:2;left:12px;right:12px;bottom:10px;font-size:19px;font-weight:800;color:#fff;line-height:1.1;text-shadow:0 2px 6px rgba(0,0,0,.6);overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical}
  .playing{position:absolute;z-index:3;left:8px;top:8px;display:flex;align-items:center;gap:5px;height:24px;padding:0 10px 0 6px;border-radius:999px;font-size:12px;font-weight:800;color:#fff;
    background:#3BB54A;box-shadow:inset 0 1px 0 rgba(255,255,255,.5),0 0 0 1px #24802f,0 0 14px #3BB54A;animation:pulse 1.6s ease-in-out infinite}
  .playing i{width:14px;height:14px;display:block}
  .playing i :global(svg){width:14px;height:14px;display:block}
</style>
