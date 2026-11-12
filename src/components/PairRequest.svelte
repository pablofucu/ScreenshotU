<script lang="ts">
  // Modal «<equipo> quiere conectarse»: 4 cifras para comparar con las de la tablet/PC. A aprueba, B rechaza
  // (el mando lo recoge `pairCmd`, enganchado al principio de `cmd`). La notificación de Windows va aparte, en Rust.
  import { onMount } from 'svelte';
  import { IS_TABLET } from '../lib/backend';
  import { answerPair, current, pr, startPairs } from '../lib/pair.svelte';

  onMount(() => { if (!IS_TABLET) void startPairs(); });

  const r = $derived(current());
  const left = $derived(r ? Math.max(0, Math.round((r.expiresMs - pr.now) / 1000)) : 0);
  const clock = $derived(`${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`);
  const KIND: Record<string, string> = { tablet: 'Tablet', pc: 'PC' };
</script>

{#if r}
  <div class="scrim prscrim" role="presentation">
    <div class="glass float prmodal" role="alertdialog" aria-modal="true" aria-label="Solicitud de emparejamiento">
      <div class="prtile">
        <svg class="ic" viewBox="0 0 24 24" aria-hidden="true">
          {#if r.kind === 'tablet'}<rect x="3" y="5" width="18" height="14" rx="2.5" /><path d="M11 16h2" />
          {:else}<rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" />{/if}
        </svg>
      </div>
      <h1>{r.name} quiere conectarse</h1>
      <p class="sub">Comprueba que este número es el mismo que ves en el otro dispositivo.</p>
      <div class="num">{r.digits}</div>
      <div class="meta">
        <span class="pill">{KIND[r.kind] ?? r.kind}</span>
        {#if r.ip}<span class="pill">{r.ip}</span>{/if}
      </div>
      <div class="exp">Si no lo has pedido tú, rechaza. Caduca en {clock}.{pr.list.length > 1 ? ` Hay ${pr.list.length - 1} más en espera.` : ''}</div>
      <div class="mb">
        <button type="button" class="b-main f" disabled={pr.busy} onclick={() => answerPair(true)}><span class="btn a big">A</span>Aprobar</button>
        <button type="button" class="b-soft pill" disabled={pr.busy} onclick={() => answerPair(false)}><span class="btn b big">B</span>Rechazar</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .prscrim { z-index: 60; display: block; }
  .prmodal { left: 320px; top: 90px; width: 640px; height: auto; padding: 34px 40px 32px; text-align: center; cursor: default; border-radius: var(--r-l); overflow: hidden; }
  .prmodal::before { content: ''; position: absolute; left: 0; right: 0; top: 0; height: 6px; background: var(--grad); z-index: 4; }
  .prtile { position: relative; z-index: 1; width: 64px; height: 64px; margin: 0 auto 14px; border-radius: 20px; display: grid; place-items: center; color: var(--on);
    background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 40%, #fff), var(--acc));
    box-shadow: inset 0 1px 0 rgba(255,255,255,.7), 0 0 0 1px color-mix(in srgb, var(--acc) 70%, black), 0 12px 20px -12px color-mix(in srgb, var(--acc) 80%, black); }
  .ic { width: 34px; height: 34px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  h1 { position: relative; z-index: 1; margin: 0; font-size: var(--t-2xl); font-weight: 800; line-height: 1.2; color: var(--ink); overflow-wrap: anywhere; }
  .sub { position: relative; z-index: 1; margin: 8px auto 0; max-width: 460px; font-size: var(--t-m); font-weight: 500; line-height: 1.35; color: var(--ink-2); }
  .num { position: relative; z-index: 1; margin: 20px auto 0; display: inline-block; padding: 6px 40px 10px 54px; border-radius: var(--r-l); background: var(--face-hi);
    box-shadow: inset 0 0 0 1px var(--line), inset 0 3px 8px -4px rgba(40,50,70,.25);
    font: 800 88px/1.05 ui-monospace, Consolas, monospace; letter-spacing: 14px; color: var(--ink); }
  .meta { position: relative; z-index: 1; display: flex; justify-content: center; gap: 10px; margin-top: 18px; }
  .meta span { display: inline-flex; align-items: center; height: 34px; padding: 0 14px; border-radius: var(--r-pill); font-size: var(--t-s); font-weight: 700; color: var(--ink-2); }
  .exp { position: relative; z-index: 1; margin-top: 14px; font-size: var(--t-s); font-weight: 700; color: var(--ink-3); }
  .mb { position: relative; z-index: 1; display: flex; gap: 16px; justify-content: center; margin-top: 22px; }
  .mb :global(.b-main), .mb :global(.b-soft) { height: 52px; padding: 0 28px 0 12px; font-size: var(--t-xl); cursor: pointer; }
  .mb :global(.b-soft) { display: inline-flex; align-items: center; gap: var(--s-2); border: 0; font-weight: 700; color: var(--ink); }
  .mb :global(.b-main .btn) { background: #fff; color: var(--ok-f, var(--ok)); text-shadow: none; box-shadow: 0 1px 2px rgba(0,0,0,.25); }
  .mb :global(.btn.big) { min-width: 36px; height: 36px; font-size: var(--t-l); border-radius: 14px; }
  button:disabled { opacity: .6; }
</style>
