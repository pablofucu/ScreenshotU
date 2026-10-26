<script lang="ts">
  // Pantalla de conexión con el visor de escritorio (o modo demo).
  import { ICON } from '@shared/icons';
  import { MI } from '../lib/icons';
  import { DEFAULT_PORT, fmtCode, hello, NetError, AuthError, normCode, parseAddress } from '../lib/source';
  import { app } from '../lib/store.svelte';

  let address = $state('');
  let code = $state('');
  let checking = $state(false);
  let checkResult = $state<{ name: string; api: number } | null>(null);
  let checkError = $state('');
  let connecting = $state(false);
  let connectError = $state('');

  const displayError = $derived(connectError || app.error);

  function onAddressInput() {
    checkResult = null;
    checkError = '';
    connectError = '';
    app.error = '';
  }

  async function handleCheck() {
    checkError = '';
    checkResult = null;
    connectError = '';
    app.error = '';

    const parsed = parseAddress(address);
    if (!parsed) {
      checkError = 'Introduce una dirección válida (ej. 192.168.1.20 o 192.168.1.20:47811)';
      return;
    }

    checking = true;
    try {
      const res = await hello(parsed.host, parsed.port);
      checkResult = res;
    } catch (e) {
      if (e instanceof NetError || e instanceof AuthError || e instanceof Error) {
        checkError = e.message;
      } else {
        checkError = String(e);
      }
    } finally {
      checking = false;
    }
  }

  function onCodeInput(e: Event) {
    const input = e.target as HTMLInputElement;
    let val = input.value;
    if (code.includes('-') && !val.includes('-') && val.length === 8) {
      val = val.slice(0, 3) + val.slice(4);
    }
    const normalized = normCode(val).slice(0, 8);
    code = fmtCode(normalized);
    input.value = code;
    connectError = '';
    app.error = '';
  }

  async function handleConnect() {
    connectError = '';
    app.error = '';

    const parsed = parseAddress(address);
    if (!parsed) {
      connectError = 'Introduce una dirección válida (ej. 192.168.1.20 o 192.168.1.20:47811)';
      return;
    }

    const cleanCode = normCode(code);
    if (cleanCode.length !== 8) {
      connectError = 'El código debe tener 8 caracteres (ej. ABCD-EFGH)';
      return;
    }

    connecting = true;
    try {
      // Firma real de store.svelte.ts: connect(s: ServerCfg | 'demo')
      await app.connect({
        host: parsed.host,
        port: parsed.port,
        code: cleanCode,
        name: checkResult?.name,
      });

      if (!app.snap && app.error) {
        connectError = app.error;
        app.disconnect(app.error);
      }
    } catch (e) {
      if (e instanceof AuthError || e instanceof NetError || e instanceof Error) {
        connectError = e.message;
      } else {
        connectError = String(e);
      }
      app.disconnect(connectError);
    } finally {
      connecting = false;
    }
  }

  async function handleDemo() {
    connectError = '';
    app.error = '';
    connecting = true;
    try {
      await app.connect('demo');
    } catch (e) {
      connectError = e instanceof Error ? e.message : String(e);
      app.disconnect(connectError);
    } finally {
      connecting = false;
    }
  }
</script>

<div class="screen pad connect-wrap">
  <div class="glass panel connect-card">
    <div class="face connect-body">
      <div class="hero-icon">
        <i class="pc-svg">{@html ICON.pc}</i>
      </div>

      <h2 class="title">Conectar al PC</h2>
      <p class="muted subtitle">
        Conéctate al visor de tu ordenador para consultar tu biblioteca, capturas y estadísticas.
      </p>

      <form class="form" onsubmit={(e) => { e.preventDefault(); void handleConnect(); }}>
        <!-- Dirección IP:Puerto -->
        <div class="field">
          <label for="server-address" class="label">Dirección del visor (puerto {DEFAULT_PORT})</label>
          <div class="addr-row">
            <div class="input-box pill">
              <input
                id="server-address"
                type="text"
                placeholder="192.168.1.20:{DEFAULT_PORT}"
                bind:value={address}
                oninput={onAddressInput}
                autocomplete="off"
                autocapitalize="off"
                spellcheck="false"
              />
            </div>
            <button
              type="button"
              class="check-btn pill"
              onclick={handleCheck}
              disabled={checking || !address.trim()}
            >
              {checking ? 'Comprobando…' : 'Comprobar'}
            </button>
          </div>

          {#if checkResult}
            <div class="feedback ok">
              <i class="fb-icon">{@html ICON.check}</i>
              <span>Visor detectado: <b>{checkResult.name}</b> (API v{checkResult.api})</span>
            </div>
          {:else if checkError}
            <div class="feedback err">
              <span>{checkError}</span>
            </div>
          {/if}
        </div>

        <!-- Código de emparejamiento -->
        <div class="field">
          <label for="pairing-code" class="label">Código de emparejamiento</label>
          <div class="input-box pill code-box">
            <input
              id="pairing-code"
              type="text"
              placeholder="ABCD-EFGH"
              value={code}
              oninput={onCodeInput}
              maxlength="9"
              autocomplete="off"
              autocapitalize="characters"
              spellcheck="false"
            />
          </div>
          <span class="hint">En el PC: <i>Ajustes → Acceso desde el móvil</i></span>
        </div>

        {#if displayError}
          <div class="feedback err main-err">
            <span>{displayError}</span>
          </div>
        {/if}

        <button
          type="submit"
          class="bigbtn"
          disabled={connecting || !address.trim() || normCode(code).length !== 8}
        >
          {connecting ? 'Conectando…' : 'Conectar'}
        </button>
      </form>

      <div class="sep">
        <span class="sep-line"></span>
        <span class="sep-text">o bien</span>
        <span class="sep-line"></span>
      </div>

      <button
        type="button"
        class="bigbtn ghosty"
        onclick={handleDemo}
        disabled={connecting}
      >
        Demostración
      </button>
      <span class="demo-sub">Explora una biblioteca de prueba sin conectar al PC</span>
    </div>
  </div>
</div>

<style>
  .connect-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - var(--top-h) - var(--tab-h) - 20px);
    padding-top: 14px;
    padding-bottom: 24px;
  }
  .connect-card {
    width: 100%;
    max-width: 360px;
  }
  .connect-body {
    padding: 24px 18px 22px;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    text-align: center;
  }
  .hero-icon {
    width: 60px;
    height: 60px;
    margin: 0 auto 12px;
    border-radius: 18px;
    display: grid;
    place-items: center;
    background: linear-gradient(180deg, color-mix(in srgb, var(--acc) 45%, white), var(--acc));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7), 0 0 0 1px color-mix(in srgb, var(--acc) 70%, black), 0 8px 16px -8px color-mix(in srgb, var(--acc) 80%, black);
  }
  .pc-svg {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
  }
  .pc-svg :global(svg) {
    width: 34px;
    height: 34px;
    display: block;
  }
  .title {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 800;
    color: var(--ink);
  }
  .subtitle {
    margin: 0 0 18px;
    font-size: 13.5px;
    line-height: 1.35;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
    text-align: left;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 12.5px;
    font-weight: 800;
    color: var(--ink2);
    padding-left: 4px;
  }
  .addr-row {
    display: flex;
    gap: 8px;
    align-items: stretch;
  }
  .input-box {
    flex: 1;
    min-width: 0;
    height: 42px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-radius: 999px;
  }
  .input-box input {
    width: 100%;
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 14.5px;
    font-weight: 700;
    color: var(--ink);
  }
  .input-box input::placeholder {
    color: var(--dim2);
    font-weight: 600;
  }
  .code-box {
    text-align: center;
  }
  .code-box input {
    text-align: center;
    font-size: 18px;
    font-weight: 800;
    letter-spacing: 2px;
    font-family: monospace, var(--round);
  }
  .check-btn {
    flex: none;
    height: 42px;
    padding: 0 14px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 800;
    color: var(--acc);
    background: rgba(255, 255, 255, 0.85);
    transition: transform 0.15s, opacity 0.2s;
  }
  .check-btn:active {
    transform: scale(0.96);
  }
  .check-btn:disabled {
    opacity: 0.5;
  }
  .hint {
    font-size: 11.5px;
    font-weight: 700;
    color: var(--dim);
    padding-left: 4px;
  }
  .feedback {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 14px;
    font-size: 12.5px;
    font-weight: 700;
    line-height: 1.3;
    animation: fadeIn 0.2s ease-out;
  }
  .feedback.ok {
    background: rgba(59, 181, 74, 0.12);
    color: #247530;
    border: 1px solid rgba(59, 181, 74, 0.3);
  }
  .fb-icon {
    width: 16px;
    height: 16px;
    flex: none;
  }
  .fb-icon :global(svg) {
    width: 16px;
    height: 16px;
    display: block;
    stroke: #247530;
  }
  .feedback.err {
    background: rgba(232, 56, 79, 0.12);
    color: #a82030;
    border: 1px solid rgba(232, 56, 79, 0.3);
  }
  .main-err {
    text-align: center;
    justify-content: center;
    padding: 10px 14px;
  }
  .sep {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 18px 0 14px;
  }
  .sep-line {
    flex: 1;
    height: 1px;
    background: rgba(110, 125, 150, 0.22);
  }
  .sep-text {
    font-size: 12px;
    font-weight: 800;
    color: var(--dim2);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .demo-sub {
    margin-top: 8px;
    font-size: 11.5px;
    font-weight: 700;
    color: var(--dim);
  }
  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: none; }
  }
</style>
