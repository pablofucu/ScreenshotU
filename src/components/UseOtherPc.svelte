<script lang="ts">
  // Asistente «Usar la biblioteca de otro PC» y aviso «ya no estás emparejado» (maqueta 5-pc-cliente).
  // El mando lo lleva `useCmd` (enganchado en `cmd`); aquí solo se dibuja. Ver lib/usepc.svelte.ts.
  import { onMount } from 'svelte';
  import { IS_TABLET } from '../lib/backend';
  import { closeUsePc, openUsePc, startUsePc, up, useCmd } from '../lib/usepc.svelte';
  import Hints from './Hints.svelte';

  onMount(() => { if (!IS_TABLET) void startUsePc(); });

  const left = $derived(Math.max(0, up.expiresMs - up.now));
  const clock = $derived(`${Math.floor(left / 60000)}:${String(Math.floor((left % 60000) / 1000)).padStart(2, '0')}`);
  const frac = $derived(Math.min(1, left / 120000));
  /** Paso activo del asistente (1-4) para la barra de pasos. */
  const step = $derived(
    up.phase === 'pick' || up.phase === 'manual' || up.phase === 'rejected' || up.phase === 'expired' ? 1
      : up.phase === 'wait' ? 2 : up.phase === 'stream' || up.phase === 'streamerr' ? 3 : 4);
  const STEPS = ['Elegir PC', 'Aprobación', 'Streaming', 'Listo'];
  const wide = $derived(up.phase === 'pick' || up.phase === 'manual');
</script>

{#snippet steps()}
  <div class="steps">
    {#each STEPS as s, i}
      <span class:on={step === i + 1} class:done={step > i + 1}>
        <i>{#if step > i + 1}<svg class="ic" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>{:else}{i + 1}{/if}</i>{s}
      </span>
    {/each}
  </div>
{/snippet}

{#snippet tile(color: string, ink: string, path: string)}
  <div class="tileic" style="--c:{color};color:{ink}"><svg class="ic" viewBox="0 0 24 24"><path d={path} /></svg></div>
{/snippet}

{#if up.phase === 'unpaired'}
  <div class="scrim upscrim" role="presentation">
    <div class="glass float upmodal" role="alertdialog" aria-modal="true" aria-label="Ya no emparejado">
      {@render tile('#F2B807', '#3a414e', 'M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1M3 3l18 18')}
      <h1>Este PC ya no está emparejado con {up.host}</h1>
      <p class="lead">Ese PC te ha quitado de sus dispositivos o has sido desconectado. Para volver a usar su biblioteca hay que emparejar de nuevo.</p>
      <div class="cbtn">
        <button type="button" class="b-main f" onclick={() => { up.reload = true; openUsePc(); }}><span class="btn a big">A</span>Emparejar de nuevo</button>
        <button type="button" class="b-soft pill" onclick={closeUsePc}><span class="btn b big">B</span>Ahora no</button>
      </div>
    </div>
  </div>
{:else if up.phase}
  <div class="scrim upscrim" role="presentation">
    <div class="crumb pill"><span>Conexiones</span><span class="sep">›</span><span class="cur">Usar la biblioteca de otro PC</span></div>
    <div class="glass wiz">
      <div class="wc" class:mid={!wide}>
        {#if up.phase === 'pick'}
          {@render steps()}
          <h1>Usar la biblioteca de otro PC</h1>
          <p class="lead">Elige el PC cuya biblioteca quieres ver aquí. Tendrá que aprobarte con un número de 4 cifras.</p>
          <div class="gap"></div>
          {#if up.finding}<span class="stat glass-in"><span class="spin"></span>Buscando PCs en la red…</span>{/if}
          <div class="pcs">
            {#each up.found as f, i (f.id + f.addr)}
              <button type="button" class="pcrow" class:f={i === up.sel} class:glass-in={i !== up.sel} onclick={() => { up.sel = i; useCmd('a'); }}>
                <span class="tile"><svg class="ic l" viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" /></svg></span>
                <span class="nm"><b>{f.name}</b><small>{f.addr}</small></span>
                <span class="st">En tu red</span>
                <svg class="ic" viewBox="0 0 24 24"><path d="M9 6l6 6-6 6" /></svg>
              </button>
            {/each}
            {#if !up.finding && !up.found.length}<p class="none">No se ha encontrado ningún PC con el visor en la red.</p>{/if}
          </div>
          <div class="alt2 glass-in">
            <p><b>¿El PC no sale en la lista?</b>Otra red o Tailscale: escribe su dirección y el billete de un solo uso que ese PC te muestra.</p>
            <span class="b-soft pill"><span class="btn x">X</span>Escribir dirección y billete</span>
          </div>
        {:else if up.phase === 'manual'}
          <h1>Escribir dirección y billete</h1>
          <p class="lead">En el otro PC: Conexiones › Añadir dispositivo › Mostrar QR de un solo uso. Copia la dirección y el billete tal como salen allí.</p>
          <div class="mform">
            <div class="fld"><span class="lab">Dirección del PC</span><div class="in mono" class:f={up.field === 0}>{#if up.mAddr}{up.mAddr}{:else}<span class="ph">192.168.1.20:8787</span>{/if}</div><small>La que sale junto al QR, la de casa o la de Tailscale.</small></div>
            <div class="fld"><span class="lab">Billete</span><div class="in mono" class:f={up.field === 1}>{#if up.mTicket}{up.mTicket}{:else}<span class="ph">ABCD-EFGH</span>{/if}</div><small>8 letras y cifras. Vale una vez y caduca a los 5 minutos.</small></div>
            <div class="cbtn left"><span class="b-main" class:f={up.field === 2} style:opacity={up.mAddr && up.mTicket ? 1 : 0.55}>{up.busy ? 'Conectando…' : 'Conectar'}</span></div>
          </div>
        {:else if up.phase === 'wait'}
          {@render steps()}
          <h1>Esperando aprobación en {up.host}</h1>
          <p class="lead">En ese PC ha aparecido una solicitud. Debe aprobarla con A comprobando que ve este mismo número.</p>
          <div class="num mono">{up.digits}</div>
          <div class="cd"><svg class="ic" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>Caduca en {clock}<span class="bar"><i style="width:{frac * 100}%"></i></span></div>
          <div class="cbtn"><button type="button" class="b-soft pill f" onclick={closeUsePc}><span class="btn b big">B</span>Cancelar</button></div>
        {:else if up.phase === 'stream' || up.phase === 'streamerr'}
          {@render steps()}
          <h1>Emparejando el streaming</h1>
          <p class="lead">Moonlight se empareja con {up.host} por sí solo. No tienes que escribir ningún PIN.</p>
          <div class="chk">
            <div class="glass-in ok"><svg class="ic" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg><span>Aprobado por {up.host}</span></div>
            <div class="glass-in ok"><svg class="ic" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg><span>Acceso a la biblioteca guardado</span></div>
            {#if up.phase === 'stream'}
              <div class="glass-in"><span class="spin"></span><span>Emparejando Moonlight</span><small>Unos segundos</small></div>
            {:else}
              <div class="glass-in bad"><svg class="ic" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg><span>No se pudo emparejar Moonlight</span></div>
              <p class="err">{up.err}</p>
            {/if}
          </div>
          {#if up.phase === 'streamerr'}
            <div class="cbtn"><button type="button" class="b-main f" onclick={() => { useCmd('a'); }}><span class="btn a big">A</span>Reintentar</button>
              <button type="button" class="b-soft pill" onclick={() => { useCmd('b'); }}><span class="btn b big">B</span>Seguir sin streaming</button></div>
          {/if}
        {:else if up.phase === 'done'}
          {@render steps()}
          <div class="bigok"><svg class="ic" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg></div>
          <h1>Listo</h1>
          <p class="lead">Este PC ya usa la biblioteca de {up.host}. Se recarga ahora para que veas sus juegos.</p>
        {:else}
          <div class="gap2"></div>
          {#if up.phase === 'rejected'}
            {@render tile('#E8384F', '#fff', 'M6 6l12 12M18 6L6 18')}
            <h1>Solicitud rechazada</h1>
            <p class="lead">{up.host} ha rechazado la solicitud. Si querías entrar, pídele a quien está delante que la apruebe y vuelve a intentarlo.</p>
          {:else}
            {@render tile('#F2B807', '#3a414e', 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2')}
            <h1>Solicitud caducada</h1>
            <p class="lead">Nadie respondió en {up.host} en 2 minutos. Si venías con un billete, puede que también haya caducado: genera uno nuevo en ese PC.</p>
          {/if}
          <div class="cbtn">
            <button type="button" class="b-main f" onclick={openUsePc}><span class="btn a big">A</span>{up.phase === 'rejected' ? 'Intentarlo de nuevo' : 'Pedir de nuevo'}</button>
            <button type="button" class="b-soft pill" onclick={closeUsePc}><span class="btn b big">B</span>Cerrar</button>
          </div>
        {/if}
      </div>
    </div>
    {#if up.phase === 'pick'}
      <Hints list={[['A', 'Elegir', 'a'], ['X', 'Escribir dirección', 'x'], ['Y', 'Buscar de nuevo', 'y'], ['B', 'Cancelar', 'b']]} />
    {:else if up.phase === 'manual'}
      <Hints list={[['A', up.field === 2 ? 'Conectar' : 'Escribir', 'a'], ['B', 'Volver', 'b']]} />
    {:else if up.phase === 'streamerr'}
      <Hints list={[['A', 'Reintentar', 'a'], ['B', 'Seguir sin streaming', 'b']]} />
    {:else if up.phase === 'rejected' || up.phase === 'expired'}
      <Hints list={[['A', up.phase === 'rejected' ? 'Intentarlo de nuevo' : 'Pedir de nuevo', 'a'], ['B', 'Cerrar', 'b']]} />
    {:else if up.phase === 'done'}
      <Hints list={[['A', 'Ver la biblioteca', 'a']]} />
    {:else if up.phase === 'wait'}
      <Hints list={[['B', 'Cancelar', 'b']]} />
    {/if}
  </div>
{/if}

<style>
  .upscrim { z-index: 30; display: block; }
  .crumb { position: absolute; left: 98px; top: 22px; height: 40px; display: flex; align-items: center; gap: 10px; padding: 0 18px; border-radius: var(--r-pill); font-size: var(--t-xl); font-weight: 700; color: var(--ink-2); }
  .crumb .sep { color: var(--ink-3); font-weight: 500; }
  .crumb .cur { color: var(--acc); }
  .wiz { left: 130px; top: 46px; width: 1020px; height: 600px; }
  .wc { position: absolute; inset: 0; padding: 34px 56px 0; display: flex; flex-direction: column; overflow: hidden; }
  .wc.mid { align-items: center; text-align: center; }
  h1 { margin: 0 0 10px; font-size: var(--t-2xl); font-weight: 800; line-height: 1.2; color: var(--ink); }
  .lead { margin: 0; font-size: var(--t-m); font-weight: 500; line-height: 1.4; color: var(--ink-2); max-width: 560px; }
  .gap { height: 18px; flex: none; }
  .gap2 { height: 76px; flex: none; }
  .ic { width: 24px; height: 24px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; flex: none; }
  .ic.l { width: 30px; height: 30px; }
  .steps { display: flex; gap: 8px; margin-bottom: 22px; }
  .steps span { display: inline-flex; align-items: center; gap: 8px; height: 32px; padding: 0 14px 0 6px; border-radius: var(--r-pill); font-size: var(--t-s); font-weight: 800; color: var(--ink-3); background: var(--glass-chip); box-shadow: inset 0 0 0 1px var(--line); }
  .steps i { width: 20px; height: 20px; border-radius: 50%; display: grid; place-items: center; font-style: normal; font-size: var(--t-xs); color: var(--ink-2); background: var(--line); }
  .steps .on { color: var(--on); background: var(--gf); box-shadow: inset 0 1px 0 rgba(255,255,255,.5), 0 0 0 1px color-mix(in srgb, var(--f2) 75%, black); }
  .steps .on i { background: rgba(255,255,255,.3); color: #fff; }
  .steps .done { color: var(--ok-f, var(--ok)); }
  .steps .done i { background: var(--ok); color: #fff; }
  .steps .done i .ic { width: 12px; height: 12px; stroke-width: 3; }
  .tileic { width: 72px; height: 72px; margin: 0 auto 14px; border-radius: 22px; display: grid; place-items: center; color: #fff; background: linear-gradient(180deg, color-mix(in srgb, var(--c, var(--acc)) 40%, #fff), var(--c, var(--acc))); box-shadow: inset 0 1px 0 rgba(255,255,255,.7), 0 0 0 1px color-mix(in srgb, var(--c, var(--acc)) 70%, black), 0 12px 20px -12px color-mix(in srgb, var(--c, var(--acc)) 80%, black); }
  .tileic .ic { width: 38px; height: 38px; }
  .spin { width: 18px; height: 18px; border-radius: 50%; border: 3px solid var(--line-2, var(--line)); border-top-color: var(--acc); animation: upspin 1s linear infinite; flex: none; }
  @keyframes upspin { to { transform: rotate(360deg); } }
  .stat { display: inline-flex; align-items: center; gap: 10px; height: 36px; padding: 0 16px; border-radius: var(--r-pill); font-size: var(--t-m); font-weight: 700; color: var(--ink-2); margin: 0 0 14px; align-self: flex-start; }
  .pcs { display: flex; flex-direction: column; gap: 10px; width: 100%; max-width: 760px; }
  .none { margin: 8px 0; font-size: var(--t-m); color: var(--ink-3); }
  .pcrow { display: flex; align-items: center; gap: 16px; height: 84px; padding: 0 20px 0 18px; border: 0; border-radius: var(--r-m); color: var(--ink-2); text-align: left; cursor: pointer; font: inherit; }
  .pcrow .tile { width: 52px; height: 52px; border-radius: var(--r-s); display: grid; place-items: center; color: var(--ink-2); background: var(--glass-in); box-shadow: inset 0 0 0 1px var(--line); }
  .pcrow .nm { flex: 1; min-width: 0; line-height: 1.25; }
  .pcrow .nm b { display: block; font-size: var(--t-xl); font-weight: 800; color: var(--ink); }
  .pcrow .nm small { font-size: var(--t-s); font-weight: 500; color: var(--ink-2); }
  .pcrow .st { font-size: var(--t-s); font-weight: 700; }
  .pcrow.f { background: linear-gradient(180deg, rgba(255,255,255,.22), rgba(255,255,255,0) 60%), var(--gf); box-shadow: inset 0 1.5px 0 rgba(255,255,255,.8), 0 0 0 1px color-mix(in srgb, var(--f2) 75%, black), 0 0 18px -2px color-mix(in srgb, var(--f1) 45%, transparent), 0 6px 12px -8px color-mix(in srgb, var(--f2) 70%, black); }
  .pcrow.f .nm b, .pcrow.f .nm small, .pcrow.f .st, .pcrow.f { color: #fff; }
  .pcrow.f .nm b { text-shadow: 0 1px 2px rgba(0,0,0,.28); }
  .pcrow.f .tile { background: rgba(255,255,255,.28); color: #fff; box-shadow: inset 0 0 0 1px rgba(255,255,255,.5); }
  .alt2 { margin-top: auto; margin-bottom: 34px; display: flex; align-items: center; gap: 16px; width: 100%; max-width: 760px; padding: 14px 20px; border-radius: var(--r-m); }
  .alt2 p { margin: 0; flex: 1; font-size: var(--t-s); font-weight: 500; line-height: 1.35; color: var(--ink-2); }
  .alt2 p b { display: block; font-size: var(--t-m); font-weight: 800; color: var(--ink); }
  .alt2 .b-soft { display: inline-flex; align-items: center; gap: 8px; height: 44px; padding: 0 18px 0 8px; font-weight: 700; color: var(--ink); }
  .mono { font-family: var(--mono, ui-monospace, Consolas, monospace); }
  .num { margin: 12px auto 0; display: inline-block; padding: 6px 40px 10px 54px; border-radius: var(--r-l); background: var(--face-hi); box-shadow: inset 0 0 0 1px var(--line), inset 0 3px 8px -4px rgba(40,50,70,.25); font-size: 120px; font-weight: 800; line-height: 1.05; letter-spacing: 18px; color: var(--ink); }
  .cd { display: flex; align-items: center; gap: 12px; margin: 22px 0 4px; width: 420px; font-size: var(--t-l); font-weight: 800; color: var(--ink); }
  .cd .bar { flex: 1; height: 8px; border-radius: 4px; background: var(--line); overflow: hidden; }
  .cd .bar i { display: block; height: 100%; background: var(--grad); transition: width .5s linear; }
  .cbtn { display: flex; gap: 16px; justify-content: center; margin-top: 26px; }
  .cbtn.left { justify-content: flex-start; margin-top: 22px; }
  .cbtn :global(.b-main), .cbtn :global(.b-soft) { display: inline-flex; align-items: center; gap: var(--s-2); height: 52px; padding: 0 28px 0 12px; font-size: var(--t-xl); font-weight: 700; border: 0; cursor: pointer; }
  .cbtn.left :global(.b-main) { padding-left: 28px; cursor: default; }
  .cbtn :global(.b-soft) { color: var(--ink); }
  .cbtn :global(.b-main .btn) { background: #fff; color: var(--ok-f, var(--ok)); text-shadow: none; box-shadow: 0 1px 2px rgba(0,0,0,.25); }
  .cbtn :global(.btn.big) { min-width: 36px; height: 36px; font-size: var(--t-l); border-radius: 14px; }
  .chk { display: flex; flex-direction: column; gap: 8px; width: 480px; margin-top: 20px; text-align: left; }
  .chk div { display: flex; align-items: center; gap: 14px; height: 52px; padding: 0 18px; border-radius: var(--r-m); font-size: var(--t-l); font-weight: 700; color: var(--ink-2); }
  .chk div span { flex: 1; }
  .chk div small { font-size: var(--t-s); font-weight: 500; color: var(--ink-3); }
  .chk .ok { color: var(--ok-f, var(--ok)); }
  .chk .bad { color: var(--danger, #E8384F); }
  .err { margin: 0; font-size: var(--t-s); color: var(--ink-2); line-height: 1.4; }
  .bigok { width: 96px; height: 96px; margin: 10px auto 18px; border-radius: 50%; display: grid; place-items: center; color: #fff; background: linear-gradient(180deg, color-mix(in srgb, var(--ok) 40%, #fff), var(--ok)); box-shadow: inset 0 1px 0 rgba(255,255,255,.7), 0 0 0 1px color-mix(in srgb, var(--ok) 70%, black), 0 16px 26px -14px var(--ok); }
  .bigok .ic { width: 52px; height: 52px; stroke-width: 2.6; }
  .mform { margin-top: 28px; width: 560px; }
  .fld { margin-bottom: 14px; }
  .fld .lab { display: block; margin: 0 0 6px 6px; font-size: var(--t-s); font-weight: 800; color: var(--ink-3); letter-spacing: .4px; text-transform: uppercase; }
  .fld .in { display: flex; align-items: center; height: 56px; padding: 0 20px; border-radius: var(--r-m); font-size: var(--t-xl); font-weight: 800; color: var(--ink); background: var(--face-hi); box-shadow: inset 0 0 0 1px var(--line), inset 0 3px 8px -4px rgba(40,50,70,.25); }
  .fld .in.f { box-shadow: var(--ring), inset 0 3px 8px -4px rgba(40,50,70,.25); }
  .fld .ph { color: var(--ink-3); font-weight: 500; }
  .fld small { display: block; margin: 6px 6px 0; font-size: var(--t-s); font-weight: 500; color: var(--ink-3); line-height: 1.35; }
  .upmodal { left: 320px; top: 120px; width: 640px; height: auto; padding: 40px 44px 36px; text-align: center; cursor: default; border-radius: var(--r-l); overflow: hidden; z-index: 60; }
  .upmodal .lead { margin: 8px auto 0; max-width: 500px; }
  .upmodal .cbtn { margin-top: 34px; }
</style>
