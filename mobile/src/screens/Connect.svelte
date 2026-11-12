<script lang="ts">
  // Asistente de conexión con el visor del PC: elegir PC → confirmar en el PC (número de 4 cifras) →
  // emparejar el streaming con Artemis (solo tablet) → listo. Alternativas: QR del visor o dirección escrita. Y la demo.
  import { ic } from '@shared/icons';
  import { onMount, onDestroy } from 'svelte';
  import { app } from '../lib/store.svelte';
  import { stream, Streaming } from '../lib/streaming.svelte';
  import { takeLostNotice, saveServer } from '../lib/cache';
  import { artemisUrl, orderAddrs, parseQrV2, splitAddr, type ServerCfg } from '../lib/pairing';
  import {
    AuthError, cancelPair, hello, pairWithTicket, probe, requestPair, streamPair, UpdateError, waitPair, DEFAULT_PORT, type Paired,
  } from '../lib/source';
  import QrScan from '../components/QrScan.svelte';

  type Step = 'pick' | 'wait' | 'stream' | 'done';
  interface Pc { id: string; name: string; host: string; port: number; version: string }

  const I = {
    pc: ic('<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>'),
    qr: ic('<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><path d="M14 14h3v3M21 14v1M14 21h2M21 18v3h-2"/>'),
    kbd: ic('<rect x="2" y="6" width="20" height="12" rx="2.5"/><path d="M6 10h1M10 10h1M14 10h1M7 14h10"/>'),
    x: ic('<path d="M6 6l12 12M18 6L6 18"/>'),
    chev: ic('<path d="M9 6l6 6-6 6"/>'),
    check: ic('<path d="M5 12.5l4.5 4.5L19 7.5"/>'),
    alert: ic('<path d="M12 3l10 18H2z"/><path d="M12 10v5M12 18.5v.1"/>'),
    play: ic('<path d="M7 4l13 8-13 8z"/>'),
    pad: ic('<rect x="2" y="7" width="20" height="10" rx="5"/><path d="M7 10v4M5 12h4M16 11h.1M18 13h.1"/>'),
    key: ic('<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M14 9l2 2"/>'),
    lib: ic('<rect x="3" y="4" width="6" height="16" rx="1.5"/><rect x="11" y="4" width="4" height="16" rx="1.5"/><path d="M17 6l4 1-3 13-4-1z"/>'),
    refresh: ic('<path d="M20 11a8 8 0 0 0-14-4L4 9M4 4v5h5M4 13a8 8 0 0 0 14 4l2-2M20 20v-5h-5"/>'),
  };

  let step = $state<Step>('pick');
  let nearby = $state<Pc[]>([]);
  let searching = $state(false);
  let scanning = $state(false);
  let manual = $state(false);
  let address = $state('');
  let ticket = $state('');
  let busy = $state(false);
  /** Aviso arriba de «Elige tu PC»: quitado desde el PC, rechazado, caducado… */
  let notice = $state(takeLostNotice() || app.error);
  let noticeSub = $state('');
  /** Error de versiones: quién tiene que actualizarse. */
  let updateMsg = $state('');

  /* ---- espera de la aprobación ---- */
  let target = $state<{ host: string; port: number; name: string } | null>(null);
  let digits = $state('');
  let requestId = '';
  let ctl: AbortController | null = null;

  /* ---- streaming ---- */
  let cfg = $state.raw<ServerCfg | null>(null);
  let sp = $state<'asking' | 'ready' | 'manual' | 'opened'>('asking');
  let streamErr = $state('');
  let artUrl = '';
  let streamOk = $state(false);

  const steps = $derived(['Elegir PC', 'Confirmar', ...(stream.tablet ? ['Streaming'] : []), 'Listo']);
  const cur = $derived(step === 'pick' ? 1 : step === 'wait' ? 2 : step === 'stream' ? 3 : steps.length);

  async function search() {
    searching = true;
    try {
      const { found } = await Streaming.discover({ timeoutMs: 2000 });
      const list = await Promise.all(found.map(async (f): Promise<Pc> => {
        try { const h = await hello(f.host, f.port, 2000); return { id: f.id, name: h.name || f.name, host: f.host, port: f.port, version: h.version }; }
        catch { return { id: f.id, name: f.name, host: f.host, port: f.port, version: '' }; }
      }));
      nearby = list;
    } catch { /* fuera de Android o sin red */ } finally { searching = false; }
  }
  onMount(() => { void search(); });
  onDestroy(() => { ctl?.abort(); });

  const msgOf = (e: unknown) => (e instanceof Error ? e.message : String(e));
  function fail(e: unknown) {
    if (e instanceof UpdateError) { updateMsg = e.message; notice = ''; }
    else { updateMsg = ''; notice = msgOf(e); noticeSub = ''; }
  }
  function clearMsgs() { notice = ''; noticeSub = ''; updateMsg = ''; app.error = ''; }

  /** Pide entrar a un PC: enseña el número y espera a que el PC lo apruebe. */
  async function request(host: string, port: number, name: string) {
    clearMsgs();
    busy = true;
    try {
      const h = await probe(host, port);
      const r = await requestPair(host, port);
      target = { host, port, name: h.name || name };
      requestId = r.requestId;
      digits = r.digits;
      step = 'wait';
      ctl = new AbortController();
      const res = await waitPair(host, port, requestId, ctl.signal);
      if (res.status === 'approved') { await finish(res.paired, host, port, target.name); return; }
      step = 'pick';
      notice = res.status === 'rejected' ? `${target.name} ha rechazado la solicitud` : 'La solicitud ha caducado';
      noticeSub = 'Vuelve a tocar el PC para pedir entrar otra vez.';
    } catch (e) {
      if (ctl?.signal.aborted) return; // cancelada por el usuario
      step = 'pick';
      fail(e);
    } finally { busy = false; }
  }

  async function cancel() {
    ctl?.abort();
    const t = target;
    step = 'pick';
    if (t) await cancelPair(t.host, t.port, requestId);
  }

  /** Ya hay token: se guarda y se sigue con el streaming (tablet) o con el final. */
  async function finish(p: Paired, host: string, port: number, name: string) {
    cfg = { host, port, token: p.token, id: p.hostId || undefined, name: p.hostName || name, addrs: orderAddrs(p.addrs, `${host}:${port}`) };
    saveServer(cfg); // si se cierra la app a medias, ya está emparejada
    if (stream.tablet) { step = 'stream'; await runStreamPair(); } else { step = 'done'; }
  }

  async function runStreamPair() {
    if (!cfg) return;
    sp = 'asking'; streamErr = ''; artUrl = '';
    await stream.refresh();
    if (!stream.moonlight) { sp = 'manual'; streamErr = 'Artemis no está instalado en esta tablet.'; return; }
    try {
      const d = await streamPair(cfg);
      artUrl = artemisUrl(cfg.host, d);
      sp = 'ready';
    } catch (e) {
      sp = 'manual';
      streamErr = e instanceof AuthError ? '' : msgOf(e);
    }
  }
  async function openArtemis(withUrl: boolean) {
    streamErr = '';
    try {
      await Streaming.openArtemis(withUrl ? { url: artUrl } : {});
      if (withUrl) { sp = 'opened'; streamOk = true; }
    } catch (e) { streamErr = msgOf(e); if (withUrl) sp = 'manual'; }
  }

  /** QR del visor (v2): se prueba cada dirección con el billete hasta que una responda. */
  async function onScan(text: string) {
    scanning = false;
    clearMsgs();
    if (/^visor:\/\/pair/i.test(text.trim())) { updateMsg = 'Ese QR es de una versión antigua. Actualiza el visor del PC y genera otro.'; return; }
    const q = parseQrV2(text);
    if (!q) { notice = 'Ese código QR no es del visor de capturas'; return; }
    busy = true;
    try {
      let last: unknown = new Error('No se puede conectar con el PC');
      for (const a of q.addrs) {
        const p = splitAddr(a);
        if (!p) continue;
        try {
          const paired = await pairWithTicket(p.host, p.port, q.ticket);
          await finish(paired, p.host, p.port, q.name || p.host);
          return;
        } catch (e) {
          last = e;
          if (e instanceof AuthError || e instanceof UpdateError) break; // el PC contestó: no tiene sentido probar otra dirección
        }
      }
      fail(last);
    } finally { busy = false; }
  }

  /** Dirección escrita a mano: con billete entra directo; sin él, pide entrar como con las tarjetas. */
  async function submitManual() {
    clearMsgs();
    const p = splitAddr(address);
    if (!p) { notice = `Escribe una dirección válida (ej. 192.168.1.20 o 192.168.1.20:${DEFAULT_PORT})`; return; }
    const t = ticket.trim();
    if (!t) { await request(p.host, p.port, p.host); return; }
    busy = true;
    try {
      const h = await probe(p.host, p.port);
      const paired = await pairWithTicket(p.host, p.port, t);
      await finish(paired, p.host, p.port, h.name);
    } catch (e) { fail(e); } finally { busy = false; }
  }

  async function openLibrary() {
    if (!cfg) return;
    busy = true;
    try {
      await app.connect(cfg);
      if (!app.snap && app.error) { notice = app.error; step = 'pick'; app.disconnect(app.error); }
    } catch (e) {
      notice = msgOf(e); step = 'pick'; app.disconnect(notice);
    } finally { busy = false; }
  }

  async function handleDemo() {
    clearMsgs();
    busy = true;
    try { await app.connect('demo'); } catch (e) { notice = msgOf(e); app.disconnect(notice); } finally { busy = false; }
  }
</script>

<div class="screen pad wrap">
  <div class="glass panel card-main">
    <div class="face body">
      <div class="steps">
        {#each steps as l, i (l)}
          {@const n = i + 1}
          <span class="step" class:on={n === cur && step !== 'done'} class:done={n < cur || step === 'done'}>
            <i>{#if n < cur || step === 'done'}{@html I.check}{:else}{n}{/if}</i>{l}
          </span>
        {/each}
      </div>

      {#if step === 'pick'}
        <h1>Elige tu PC</h1>
        <p class="lead">Estos PC con el Visor están en tu red. Toca el tuyo para pedir entrar.</p>

        {#if notice}
          <div class="warn" role="alert">
            <i class="wi">{@html I.alert}</i>
            <div><b>{notice}</b>{#if noticeSub}<small>{noticeSub}</small>{:else if app.error && notice === app.error}<small>Vuelve a conectarte para seguir.</small>{/if}</div>
          </div>
        {/if}
        {#if updateMsg}
          <div class="warn" role="alert"><i class="wi">{@html I.alert}</i><div><b>{updateMsg}</b></div></div>
        {/if}

        <div class="pcs">
          {#each nearby as f, i (f.id + f.host)}
            <button type="button" class="pc" style="--c:{app.acc(4 + i).c}" disabled={busy} onclick={() => request(f.host, f.port, f.name)}>
              <span class="tile">{@html I.pc}</span>
              <span class="nm"><b>{f.name}</b><small>{f.host}{f.version ? ` · Visor ${f.version}` : ''}</small></span>
              <i class="chev">{@html I.chev}</i>
            </button>
          {/each}
          {#if searching && !nearby.length}
            <div class="skel"><span class="a"></span><span class="b"><i></i><i></i></span></div>
          {/if}
        </div>
        {#if searching}
          <div class="search"><i class="si spin">{@html I.refresh}</i>Buscando más PC…</div>
        {:else}
          <button type="button" class="link search-btn" onclick={search}><i class="si">{@html I.refresh}</i>{nearby.length ? 'Buscar de nuevo' : 'No se ha encontrado ningún PC. Buscar de nuevo'}</button>
        {/if}

        <div class="foot">
          <button type="button" class="bigbtn main" onclick={() => { clearMsgs(); scanning = true; }} disabled={busy}><i class="bi">{@html I.qr}</i>Escanear QR</button>
          <div class="links">
            <button type="button" class="link" onclick={() => (manual = !manual)}><i class="si">{@html I.kbd}</i>Escribir dirección y billete</button>
            <button type="button" class="link q" onclick={handleDemo} disabled={busy}><i class="si">{@html I.play}</i>Probar la demostración</button>
          </div>
          {#if manual}
            <form class="form" onsubmit={(e) => { e.preventDefault(); void submitManual(); }}>
              <label class="label" for="srv-address">Dirección del visor (puerto {DEFAULT_PORT})</label>
              <div class="input-box pill"><input id="srv-address" type="text" placeholder="192.168.1.20:{DEFAULT_PORT}" bind:value={address} autocomplete="off" autocapitalize="off" spellcheck="false" /></div>
              <label class="label" for="srv-ticket">Billete (opcional: si lo dejas vacío, el PC te pedirá confirmar)</label>
              <div class="input-box pill"><input id="srv-ticket" type="text" placeholder="Billete del visor" bind:value={ticket} autocomplete="off" autocapitalize="off" spellcheck="false" /></div>
              <button type="submit" class="bigbtn main" disabled={busy || !address.trim()}>{busy ? 'Conectando…' : 'Conectar'}</button>
            </form>
          {/if}
        </div>

      {:else if step === 'wait'}
        <div class="center">
          <h1>Confirma en el PC</h1>
          <p class="lead">Pulsa A en {target?.name} cuando veas este número.</p>
          <div class="num">{digits}</div>
          <div class="dots"><i></i><i></i><i></i></div>
          <span class="who pill"><i class="si">{@html I.pc}</i>{target?.name} · {target?.host}</span>
        </div>
        <div class="foot"><button type="button" class="bigbtn ghosty" onclick={cancel}><i class="bi">{@html I.x}</i>Cancelar</button></div>

      {:else if step === 'stream'}
        <h1>Empareja el streaming</h1>
        <p class="lead">Para jugar desde la tablet, Artemis tiene que conocer el PC. Es un paso y se hace una vez.</p>
        <div class="todo glass-in">
          <div class="ok"><i class="si">{@html I.check}</i>Conectada a {cfg?.name}</div>
          <div class:wait={sp === 'manual'}><i class="si" style="color:var(--acc)">{@html I.pad}</i>{sp === 'opened' ? 'Artemis abierto' : sp === 'manual' ? 'Streaming sin emparejar' : 'Emparejando streaming con Artemis…'}</div>
          <div class="wait"><i class="si">{@html I.play}</i>Listo para jugar</div>
        </div>
        {#if sp === 'manual'}
          <div class="tipcard glass-in plan-b">
            <span class="tile"><i class="bi">{@html I.key}</i></span>
            <div>
              <b>Empareja Artemis a mano</b>
              <p>{#if streamErr}{streamErr} {/if}Abre Artemis, añade el PC con la dirección {cfg?.host} y sigue sus pasos; el PIN lo acepta el PC desde Apollo. Puedes hacerlo más tarde.</p>
            </div>
          </div>
        {:else}
          <div class="tipcard glass-in">
            <span class="tile"><i class="bi">{@html I.key}</i></span>
            <div><b>Artemis te mostrará un PIN</b><p>No hace falta escribirlo: el PC lo acepta solo. Abre Artemis y vuelve aquí.</p></div>
          </div>
        {/if}
        {#if streamErr && sp !== 'manual'}<div class="warn"><i class="wi">{@html I.alert}</i><div><b>{streamErr}</b></div></div>{/if}
        <div class="foot">
          {#if sp === 'ready'}
            <button type="button" class="bigbtn main" onclick={() => openArtemis(true)}><i class="bi">{@html I.play}</i>Abrir Artemis</button>
          {:else if sp === 'opened'}
            <button type="button" class="bigbtn main" onclick={() => (step = 'done')}>Continuar</button>
          {:else if sp === 'manual'}
            <button type="button" class="bigbtn main" onclick={runStreamPair}><i class="bi">{@html I.refresh}</i>Reintentar</button>
            {#if stream.moonlight}<button type="button" class="link" onclick={() => openArtemis(false)}>Abrir Artemis</button>{/if}
          {/if}
          <button type="button" class="link q" onclick={() => (step = 'done')}>Hacerlo después</button>
        </div>

      {:else}
        <div class="center">
          <div class="big-ok"><i class="bi">{@html I.check}</i></div>
          <h1>Listo</h1>
          <p class="lead">Esta {stream.tablet ? 'tablet' : 'app'} ya está conectada a {cfg?.name}.</p>
          <div class="sums">
            <span class="glass-in"><i class="si" style="color:var(--ok-f,#27833a)">{@html I.lib}</i>Biblioteca</span>
            {#if stream.tablet && streamOk}<span class="glass-in"><i class="si" style="color:var(--ok-f,#27833a)">{@html I.pad}</i>Streaming emparejado</span>{/if}
          </div>
        </div>
        <div class="foot"><button type="button" class="bigbtn main" onclick={openLibrary} disabled={busy}>{busy ? 'Abriendo…' : 'Abrir la biblioteca'}</button></div>
      {/if}
    </div>
  </div>
</div>

{#if scanning}<QrScan onresult={onScan} onclose={() => (scanning = false)} />{/if}

<style>
  .wrap { display: flex; align-items: center; justify-content: center; min-height: calc(100vh - var(--top-h) - var(--tab-h) - 20px); padding-top: 14px; padding-bottom: 24px; }
  .card-main { width: 100%; max-width: 640px; }
  .body { padding: 20px 20px 18px; display: flex; flex-direction: column; text-align: left; min-height: 420px; }
  h1 { margin: 0; font-size: 24px; font-weight: 800; line-height: 1.2; color: var(--ink); }
  .lead { margin: 6px 0 0; font-size: 14.5px; font-weight: 500; line-height: 1.4; color: var(--ink2); }
  .steps { display: flex; justify-content: center; flex-wrap: wrap; gap: 6px; margin-bottom: 16px; }
  .step { display: inline-flex; align-items: center; gap: 8px; height: 34px; padding: 0 12px 0 6px; border-radius: 999px; font-size: 13px; font-weight: 800; color: var(--dim2); }
  .step i { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; font-style: normal; font-size: 12px; color: var(--ink2); background: rgba(255, 255, 255, 0.6); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24); }
  .step i :global(svg) { width: 14px; height: 14px; stroke-width: 3; }
  .step.on { color: var(--ink); }
  .step.on i { color: var(--acc-ink); background: var(--acc); box-shadow: 0 0 0 1px color-mix(in srgb, var(--acc) 70%, black); }
  .step.done { color: var(--ink2); }
  .step.done i { color: #fff; background: #3bb54a; box-shadow: none; }
  .bi, .si, .wi, .chev { display: inline-grid; place-items: center; flex: none; }
  .bi { width: 22px; height: 22px; } .si { width: 18px; height: 18px; } .wi { width: 28px; height: 28px; color: #d92c44; } .chev { width: 28px; height: 28px; color: var(--dim2); }
  .bi :global(svg), .si :global(svg), .wi :global(svg), .chev :global(svg) { width: 100%; height: 100%; display: block; }
  .pcs { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 12px; margin-top: 16px; }
  .pc { display: flex; align-items: center; gap: 14px; min-height: 84px; padding: 0 14px; border-radius: 18px; text-align: left; background: rgba(247, 249, 252, 0.94); box-shadow: 0 0 0 1px rgba(95, 110, 135, 0.18), 0 10px 18px -12px rgba(40, 50, 70, 0.4); transition: transform 0.15s; }
  .pc:active { transform: scale(0.97); }
  .pc:disabled { opacity: 0.6; }
  .tile { flex: none; width: 54px; height: 54px; border-radius: 16px; display: grid; place-items: center; color: #fff; background: linear-gradient(180deg, color-mix(in srgb, var(--c, var(--acc)) 35%, #fff), var(--c, var(--acc))); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7), 0 0 0 1px color-mix(in srgb, var(--c, var(--acc)) 70%, black); }
  .tile :global(svg) { width: 28px; height: 28px; }
  .nm { flex: 1; min-width: 0; line-height: 1.25; }
  .nm b { display: block; font-size: 17px; font-weight: 800; color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm small { font-size: 12.5px; font-weight: 500; color: var(--ink2); }
  .skel { height: 84px; border-radius: 18px; display: flex; align-items: center; gap: 14px; padding: 0 14px; background: rgba(255, 255, 255, 0.55); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24); }
  .skel .a { width: 54px; height: 54px; border-radius: 16px; background: rgba(95, 110, 135, 0.2); }
  .skel .b { flex: 1; display: flex; flex-direction: column; gap: 10px; } .skel .b i { height: 12px; width: 60%; border-radius: 6px; background: rgba(95, 110, 135, 0.2); } .skel .b i + i { width: 38%; }
  .search { display: flex; align-items: center; gap: 8px; margin-top: 12px; font-size: 13px; font-weight: 700; color: var(--dim2); }
  .search-btn { margin-top: 4px; justify-content: flex-start; }
  .spin { animation: spin 1.2s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .foot { margin-top: auto; padding-top: 18px; display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .foot .bigbtn { max-width: 380px; }
  .links { display: flex; flex-wrap: wrap; justify-content: center; gap: 4px 22px; }
  .link { display: inline-flex; align-items: center; gap: 8px; min-height: 44px; padding: 0 6px; font-size: 14.5px; font-weight: 800; color: var(--acc); background: none; }
  .link.q { color: var(--ink2); }
  .link:disabled { opacity: 0.5; }
  .form { width: 100%; max-width: 380px; display: flex; flex-direction: column; gap: 8px; margin-top: 4px; }
  .label { font-size: 12.5px; font-weight: 800; color: var(--ink2); padding-left: 4px; }
  .input-box { height: 46px; display: flex; align-items: center; padding: 0 14px; border-radius: 999px; }
  .input-box input { width: 100%; border: 0; outline: 0; background: transparent; font-size: 15px; font-weight: 700; color: var(--ink); }
  .input-box input::placeholder { color: var(--dim2); font-weight: 600; }
  .center { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
  .num { display: inline-block; padding: 4px 18px 8px 34px; border-radius: 24px; margin: 16px 0 12px; font-size: 76px; line-height: 1.05; font-weight: 800; letter-spacing: 14px; font-family: ui-monospace, Consolas, monospace; color: var(--ink); background: rgba(247, 249, 252, 0.94); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24), inset 0 3px 8px -4px rgba(40, 50, 70, 0.25); }
  .dots { display: flex; gap: 8px; justify-content: center; margin: 2px 0 14px; }
  .dots i { width: 10px; height: 10px; border-radius: 50%; background: var(--acc); opacity: 0.25; animation: dd 1.2s infinite; }
  .dots i:nth-child(2) { animation-delay: 0.2s; } .dots i:nth-child(3) { animation-delay: 0.4s; }
  @keyframes dd { 50% { opacity: 1; } }
  .who { display: inline-flex; align-items: center; gap: 10px; min-height: 44px; padding: 0 18px 0 12px; border-radius: 999px; font-size: 14px; font-weight: 800; color: var(--ink2); }
  .who .si { color: var(--acc); }
  .todo { display: flex; flex-direction: column; gap: 4px; margin-top: 16px; padding: 6px; border-radius: 18px; background: rgba(255, 255, 255, 0.55); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24); }
  .todo div { display: flex; align-items: center; gap: 12px; min-height: 46px; padding: 0 14px; font-size: 15.5px; font-weight: 700; color: var(--ink); }
  .todo .ok { color: #27833a; } .todo .wait { color: var(--dim2); }
  .tipcard { display: flex; gap: 14px; align-items: flex-start; padding: 16px 18px; margin-top: 14px; border-radius: 18px; background: rgba(255, 255, 255, 0.55); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24); }
  .tipcard .tile { --c: #f27a1a; width: 48px; height: 48px; }
  .tipcard .tile .bi { width: 26px; height: 26px; }
  .tipcard b { display: block; font-size: 17px; font-weight: 800; color: var(--ink); margin-bottom: 4px; }
  .tipcard p { margin: 0; font-size: 14px; font-weight: 500; line-height: 1.4; color: var(--ink2); }
  .big-ok { width: 108px; height: 108px; border-radius: 50%; display: grid; place-items: center; color: #fff; margin-bottom: 16px; background: linear-gradient(180deg, color-mix(in srgb, #3bb54a 40%, #fff), #3bb54a); box-shadow: inset 0 2px 0 rgba(255, 255, 255, 0.7), 0 0 0 6px rgba(255, 255, 255, 0.7), 0 0 0 8px color-mix(in srgb, #3bb54a 60%, transparent), 0 20px 30px -14px color-mix(in srgb, #3bb54a 80%, black); }
  .big-ok .bi { width: 56px; height: 56px; } .big-ok :global(svg) { stroke-width: 3; }
  .sums { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 16px; justify-content: center; }
  .sums span { display: inline-flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 16px; border-radius: 999px; font-size: 14px; font-weight: 800; color: var(--ink2); background: rgba(255, 255, 255, 0.55); box-shadow: inset 0 0 0 1px rgba(95, 110, 135, 0.24); }
  .warn { display: flex; align-items: center; gap: 12px; margin-top: 14px; padding: 12px 16px; border-radius: 18px; color: #8f1a2b; background: color-mix(in srgb, #e8384f 11%, #fff); box-shadow: inset 0 0 0 1px color-mix(in srgb, #e8384f 40%, transparent); }
  .warn b { display: block; font-size: 15px; font-weight: 800; }
  .warn small { font-size: 12.5px; font-weight: 500; color: var(--ink2); }
  .bigbtn.main { color: var(--acc-ink); }
</style>
