<script lang="ts">
  // Asistente «Añadir dispositivo» (Ajustes › Conexiones): esperando solicitudes y QR de un solo uso con cuenta atrás.
  // Las solicitudes las aprueba el aviso global del visor; aquí solo se explica y se muestran las pendientes.
  // El mando lo lleva `wizardCmd` (A, B, Y), llamado desde el control de Ajustes.
  import { connView, backToWait, closeWizard, newTicket } from '../lib/connections.svelte';
  import { qrSvg } from '../lib/qr';
  import Hints from './Hints.svelte';

  const t = $derived(connView.ticket);
  const left = $derived(t ? Math.max(0, t.expiresMs - connView.now) : 0);
  const expired = $derived(!!t && left <= 0);
  const clock = $derived(`${Math.floor(left / 60000)}:${String(Math.floor((left % 60000) / 1000)).padStart(2, '0')}`);
  const frac = $derived(t ? Math.min(1, left / 300000) : 0);
  const svg = $derived.by(() => {
    if (!t?.qr) return '';
    try { return qrSvg(t.qr); } catch { return ''; }
  });
  const hosts = $derived.by(() => {
    const a = connView.addrs;
    return a.map((x) => ({ x, tail: /^100\./.test(x) }));
  });
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) { if (connView.wizard === 'qr') backToWait(); else closeWizard(); } }}>
  <div class="glass modal adpanel">
    <div class="face">
      {#if connView.wizard === 'wait'}
        <h2>Añadir dispositivo</h2>
        <div class="cols">
          <div class="col">
            <p class="lead">Abre el Visor en la tablet, o «Usar la biblioteca de otro PC» en el otro PC. Este PC aparecerá en su lista y te llegará una solicitud con un número de 4 cifras.</p>
            <div class="how">
              <div class="card">
                <svg class="ic" viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2.5" /><path d="M11 16h2" /></svg>
                <span><b>Tablet</b>Abre el Visor y toca este PC.</span>
              </div>
              <div class="card">
                <svg class="ic" viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="10" rx="2" /><path d="M3 19h18" /></svg>
                <span><b>Otro PC</b>Conexiones › Usar la biblioteca de otro PC.</span>
              </div>
            </div>
            <div class="card alt">
              <svg class="ic" viewBox="0 0 24 24"><rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><path d="M14 14h3v3M21 14v1M14 21h2M21 18v3h-2" /></svg>
              <span><b>¿Este PC no sale en la red?</b>Otra red o Tailscale: usa un QR de un solo uso.</span>
            </div>
          </div>
          <div class="col">
            <div class="tag">Solicitudes pendientes</div>
            {#if connView.pending.length}
              {#each connView.pending as p (p.id)}
                <div class="req">
                  <span><b>{p.name} quiere conectarse</b><small>{p.kind === 'tablet' ? 'Tablet' : 'PC'} · {p.ip}</small></span>
                  <span class="num">{p.digits}</span>
                </div>
              {/each}
              <p class="cap">Aprueba o rechaza en el aviso que ha salido. Comprueba que el número es el mismo que ves en el otro dispositivo.</p>
            {:else}
              <div class="empty">
                <div class="radar"><i></i><i></i><i></i></div>
                <p>Esperando…</p>
              </div>
              <p class="cap">Cuando alguien pida conectarse saldrá un aviso con un número de 4 cifras. Compruébalo con el del otro dispositivo; si no lo has pedido tú, rechaza. Sin respuesta caduca a los 2 minutos.</p>
            {/if}
          </div>
        </div>
      {:else}
        <h2>Escanea el QR</h2>
        <div class="cols">
          <div class="col qrcol">
            {#if svg}
              <div class="qrcard" class:dim={expired}>{@html svg}</div>
              <p class="cap">Escanéalo con el Visor de la tablet. Escanearlo ya cuenta como aprobación.</p>
            {:else}
              <div class="empty"><p>{connView.ticketError || 'Preparando el billete…'}</p></div>
            {/if}
          </div>
          <div class="col">
            <p class="lead">Vale una sola vez. En otro PC, escribe este billete y la dirección.</p>
            <div class="ticket"><span class="code">{t?.ticket ?? '····-····'}</span><small>Billete de un solo uso</small></div>
            <div class="cd" class:bad={expired}>
              <svg class="ic" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>
              {#if t}{expired ? 'Caducado: pide uno nuevo' : `Caduca en ${clock}`}{:else}—{/if}
              <span class="bar"><i style="width:{frac * 100}%"></i></span>
            </div>
            <div class="ads">
              {#each hosts as h (h.x)}
                <div class="card"><span>{h.tail ? 'Tailscale' : 'Casa'}</span><span class="mono">{h.x}</span></div>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
  {#if connView.wizard === 'wait'}
    <Hints list={[['A', 'Mostrar QR', 'a'], ['B', 'Cerrar', 'b']]} />
  {:else}
    <Hints list={[['Y', 'Billete nuevo', 'y'], ['B', 'Volver a esperar', 'b']]} />
  {/if}
</div>

<style>
  .adpanel { left: 140px; top: 50px; width: 1000px; height: 600px; }
  h2 { flex: none; }
  .cols { flex: 1; min-height: 0; display: flex; gap: 28px; margin-top: 10px; }
  .col { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 14px; }
  .lead { margin: 0; font-size: var(--t-m, 18px); line-height: 1.5; color: var(--ink-2); }
  .how { display: flex; flex-direction: column; gap: 10px; }
  .card { display: flex; align-items: center; gap: 14px; padding: 14px 18px; border-radius: var(--r-m); background: var(--glass-in); font-size: var(--t-s); color: var(--ink-2); }
  .card b { display: block; color: var(--ink); font-size: var(--t-l); }
  .card.alt { margin-top: auto; }
  .ic { flex: none; width: 30px; height: 30px; fill: none; stroke: var(--ink-2); stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .tag { font-size: var(--t-s); font-weight: 800; color: var(--ink-3); text-transform: uppercase; letter-spacing: .06em; }
  .req { display: flex; align-items: center; gap: 16px; padding: 16px 20px; border-radius: var(--r-m); background: var(--glass-in); }
  .req span:first-child { flex: 1; min-width: 0; }
  .req b { display: block; font-size: var(--t-l); color: var(--ink); }
  .req small { font-size: var(--t-s); color: var(--ink-2); }
  .num { font: 800 44px/1 var(--mono, ui-monospace, Consolas, monospace); letter-spacing: .12em; color: var(--ink); }
  .cap { margin: 0; font-size: var(--t-s); line-height: 1.5; color: var(--ink-2); }
  .empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--ink-2); font-size: var(--t-l); font-weight: 700; }
  .empty p { margin: 0; text-align: center; }
  .radar { position: relative; width: 120px; height: 120px; }
  .radar i { position: absolute; inset: 0; border-radius: 50%; border: 3px solid var(--acc); opacity: 0; animation: ping 2.4s var(--e-soft, ease-out) infinite; }
  .radar i:nth-child(2) { animation-delay: .8s; }
  .radar i:nth-child(3) { animation-delay: 1.6s; }
  @keyframes ping { from { transform: scale(.2); opacity: .8; } to { transform: scale(1); opacity: 0; } }
  .qrcol { flex: none; width: 380px; align-items: center; }
  .qrcard { width: 340px; height: 340px; padding: 6px; border-radius: var(--r-m); background: #fff; box-sizing: border-box; line-height: 0; transition: opacity .2s; }
  .qrcard.dim { opacity: .25; }
  .qrcard :global(svg) { width: 100%; height: 100%; display: block; }
  .ticket { display: flex; flex-direction: column; gap: 4px; padding: 16px 20px; border-radius: var(--r-m); background: var(--glass-in); }
  .code { font: 800 40px/1 var(--mono, ui-monospace, Consolas, monospace); letter-spacing: .1em; color: var(--ink); }
  .ticket small { font-size: var(--t-s); color: var(--ink-2); }
  .cd { position: relative; display: flex; align-items: center; gap: 10px; padding-bottom: 12px; font-size: var(--t-l); font-weight: 700; color: var(--ink); }
  .cd .ic { width: 24px; height: 24px; }
  .cd.bad { color: var(--danger, #E8384F); }
  .bar { position: absolute; left: 0; right: 0; bottom: 0; height: 5px; border-radius: 3px; background: var(--glass-in); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--grad); transition: width .5s linear; }
  .ads { display: flex; flex-direction: column; gap: 8px; }
  .ads .card { justify-content: space-between; padding: 10px 18px; }
  .mono { font-family: var(--mono, ui-monospace, Consolas, monospace); color: var(--ink); font-weight: 700; }
</style>
