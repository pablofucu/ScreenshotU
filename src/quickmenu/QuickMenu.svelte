<svelte:options runes={true} />

<script lang="ts">
  // Raíz del menú sobre el juego: cuadrícula de baldosas, hojas de detalle, navegación por halo y atajos.
  import { onMount, onDestroy, tick } from 'svelte';
  import type { Achievement } from '../lib/types';
  import { isTauri,
    fetchQuickmenuState,
    setupQuickMenuListeners,
    markPainted,
    quickmenuClose,
    quickmenuCloseGame,
    quickmenuCapture,
    quickmenuTrash,
    quickmenuRumble,
    getGameAchievements,
    quickmenuSetMute,
    quickmenuSetMicMute,
    quickmenuSelectOutput,
    quickmenuSelectInput,
    quickmenuSetAppVolume,
    quickmenuSetAppMute,
    setVolumeThrottled,
    type QuickMenuState,
    type PerfState,
  } from './data';
  import TilePerf from './TilePerf.svelte';
  import TileTime from './TileTime.svelte';
  import TileShots from './TileShots.svelte';
  import TileAudio from './TileAudio.svelte';
  import TileAch from './TileAch.svelte';
  import TilePad from './TilePad.svelte';
  import TileExit from './TileExit.svelte';

  let qmState = $state<QuickMenuState | null>(null);
  let sheet = $state<null | 'rendimiento' | 'tiempo' | 'capturas' | 'audio' | 'logros' | 'mando'>(null);
  let holdProgress = $state<number>(-1);
  let stageScale = $state<number>(1);
  let stageEl = $state<HTMLElement | null>(null);
  let closing = $state<boolean>(false);
  let flashActive = $state<boolean>(false);

  // Foco
  let focusedEl = $state<HTMLElement | null>(null);
  let lastTileId = $state<string>('time');

  // Logros en caché por apertura
  let achievements = $state<Achievement[] | null>(null);
  let achievementsLoading = $state<boolean>(false);

  // Capturas y borrado diferido
  let selShotIdx = $state<number>(0);
  interface PendingDelete {
    shot: { path: string; thumb: string; at: number };
    timer: ReturnType<typeof setTimeout>;
  }
  let pendingDeletes = $state<PendingDelete[]>([]);
  let toastMsg = $state<string | null>(null);

  // Reloj
  let clockTime = $state<string>('');
  let clockDate = $state<string>('');

  function updateClock() {
    const d = new Date();
    clockTime = d.toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' });
    clockDate = d.toLocaleDateString('es-ES', { weekday: 'long', day: 'numeric', month: 'long' });
  }

  function fitStage() {
    if (typeof window === 'undefined') return;
    stageScale = Math.min(window.innerWidth / 1280, window.innerHeight / 720);
  }

  function setFocus(el: HTMLElement | null) {
    if (focusedEl === el) return;
    if (focusedEl) focusedEl.classList.remove('fo');
    focusedEl = el;
    if (el) {
      el.classList.add('fo');
      el.focus();
    }
  }

  function pickSpatial(dx: number, dy: number): HTMLElement | null {
    if (!stageEl) return null;
    const rawItems = Array.from(stageEl.querySelectorAll('[data-f]:not([disabled])')) as HTMLElement[];
    const items = rawItems.filter((el) => el.offsetParent !== null);
    if (!items.length) return null;
    if (!focusedEl || !items.includes(focusedEl)) return items[0];

    const r = focusedEl.getBoundingClientRect();
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;

    let best: HTMLElement | null = null;
    let bestDist = 1e9;

    for (const it of items) {
      if (it === focusedEl) continue;
      const q = it.getBoundingClientRect();
      const x = q.left + q.width / 2 - cx;
      const y = q.top + q.height / 2 - cy;
      const align = dx !== 0 ? x * dx : y * dy;
      if (align <= 2) continue;
      if (dx !== 0 && (q.bottom <= r.top || q.top >= r.bottom)) continue;
      const dist = align + (dx !== 0 ? Math.abs(y) : Math.abs(x)) * 2.2;
      if (dist < bestDist) {
        bestDist = dist;
        best = it;
      }
    }
    return best;
  }

  function moveSpatial(dx: number, dy: number) {
    const next = pickSpatial(dx, dy);
    if (next) setFocus(next);
  }

  async function openSheet(sh: 'rendimiento' | 'tiempo' | 'capturas' | 'audio' | 'logros' | 'mando') {
    if (focusedEl?.dataset.id) lastTileId = focusedEl.dataset.id;
    sheet = sh;
    await tick();
    if (stageEl) {
      const firstSheetItem = stageEl.querySelector('.sheetA [data-f]') as HTMLElement | null;
      if (firstSheetItem) setFocus(firstSheetItem);
    }
  }

  async function backAction() {
    if (sheet !== null) {
      const prev = lastTileId;
      sheet = null;
      await tick();
      if (stageEl) {
        const tile = (stageEl.querySelector(`[data-id="${prev}"]`) || stageEl.querySelector('[data-f]')) as HTMLElement | null;
        setFocus(tile);
      }
      return;
    }
    await triggerClose();
  }

  function flushPendingTrash() {
    for (const pd of pendingDeletes) {
      clearTimeout(pd.timer);
      quickmenuTrash([pd.shot.path]);
    }
    pendingDeletes = [];
    toastMsg = null;
  }

  async function triggerClose() {
    if (closing) return;
    await quickmenuClose();
    if (!isTauri()) onCloseEvent();
  }

  function triggerShutterFlash() {
    flashActive = true;
    setTimeout(() => { flashActive = false; }, 360);
  }

  async function handleCapture() {
    triggerShutterFlash();
    await quickmenuCapture();
  }

  function deleteShotDeferred(index: number) {
    if (!qmState?.shots[index]) return;
    const shot = qmState.shots[index];
    qmState.shots = qmState.shots.filter((_, i) => i !== index);
    if (selShotIdx >= qmState.shots.length) {
      selShotIdx = Math.max(0, qmState.shots.length - 1);
    }
    toastMsg = 'Captura borrada · LB para deshacer';

    const timer = setTimeout(() => {
      quickmenuTrash([shot.path]);
      pendingDeletes = pendingDeletes.filter((p) => p.shot.path !== shot.path);
      if (pendingDeletes.length === 0) toastMsg = null;
    }, 4000);

    pendingDeletes.push({ shot, timer });
  }

  function undoDeleteShot() {
    const last = pendingDeletes.pop();
    if (!last || !qmState) return;
    clearTimeout(last.timer);
    qmState.shots.unshift(last.shot);
    selShotIdx = 0;
    if (pendingDeletes.length === 0) toastMsg = null;
  }

  function loadAchievementsOnce() {
    if (achievements !== null || !qmState?.game?.id) return;
    achievementsLoading = true;
    getGameAchievements(qmState.game.id).then((res) => {
      achievements = res;
      achievementsLoading = false;
    });
  }

  // Eventos recibidos
  function onOpenState(newState: QuickMenuState) {
    flushPendingTrash();
    qmState = withContextShots(newState);
    selShotIdx = 0;
    sheet = null;
    closing = false;
    holdProgress = -1;
    achievements = null;
    loadAchievementsOnce();
    tick().then(() => {
      if (stageEl) {
        const initEl = (stageEl.querySelector('[data-id="time"]') || stageEl.querySelector('[data-f]')) as HTMLElement | null;
        setFocus(initEl);
      }
    });
    // Dos frames: el primero deja que el DOM pinte la nueva hoja, el segundo confirma que ya salio en pantalla.
    // Sin este aviso el backend nunca sabe que puede pausar el juego (ver quickmenu.rs::confirm).
    requestAnimationFrame(() => requestAnimationFrame(() => markPainted(newState.gen)));
  }

  function onPausedEvent(paused: boolean) {
    if (qmState) qmState.paused = paused;
  }

  function onCloseEvent() {
    flushPendingTrash();
    closing = true;
    // La foto del juego pesa: no se queda en memoria con el menú oculto.
    setTimeout(() => { if (closing && qmState) qmState.background = null; }, 240);
  }

  /** Une las capturas de esta partida con las anteriores del juego, que no traen imagen (se piden al verlas). */
  function withContextShots(st: QuickMenuState): QuickMenuState {
    const seen = new Set(st.shots.map((x) => x.path));
    const old = (st.context?.shots ?? []).filter((x) => !seen.has(x.path)).map((x) => ({ path: x.path, thumb: '', at: x.at }));
    return { ...st, shots: [...st.shots, ...old].sort((x, y) => y.at - x.at) };
  }

  /** Paso de volumen general: la interfaz responde ya y el envío va limitado. */
  function stepVolume(d: number) {
    const au = qmState?.audio;
    if (!au) return;
    au.volume = Math.max(0, Math.min(100, au.volume + d));
    au.mute = false;
    setVolumeThrottled(au.volume, (st) => {
      if (st && qmState) qmState.audio = { ...st, volume: qmState.audio?.volume ?? st.volume };
    });
  }

  /** Mover la barra enfocada: la de una aplicación o, si no, la general. */
  function stepSlider(el: HTMLElement, d: number) {
    const pid = el.dataset.id?.startsWith('app-vol-') ? Number(el.dataset.id.slice(8)) : null;
    if (pid === null) { stepVolume(d); return; }
    const row = qmState?.audio?.apps.find((x) => x.pid === pid);
    if (!row) return;
    row.volume = Math.max(0, Math.min(100, row.volume + d));
    row.mute = false;
    void quickmenuSetAppVolume(pid, row.volume);
  }

  function onShotEvent(shot: { path: string; thumb: string; at: number }) {
    if (!qmState) return;
    qmState.shots.unshift(shot);
    triggerShutterFlash();
  }

  function onPerfEvent(perf: PerfState) {
    if (qmState) qmState.perf = perf;
  }

  function onHoldEvent(p: number) {
    holdProgress = p;
  }

  function onPadBtn(btn: string) {
    const b = btn.toLowerCase();
    if (b === 'b') {
      backAction();
      return;
    }
    if (b === 'x') {
      handleCapture();
      return;
    }
    if (b === 'y') {
      if (sheet === 'capturas') {
        deleteShotDeferred(selShotIdx);
      } else if (focusedEl && focusedEl.dataset.id?.startsWith('s')) {
        const idx = parseInt(focusedEl.dataset.id.slice(1), 10);
        if (!isNaN(idx)) deleteShotDeferred(idx);
      }
      return;
    }
    if (b === 'lt') {
      stepVolume(-5);
      return;
    }
    if (b === 'rt') {
      stepVolume(5);
      return;
    }
    if (b === 'lb' && pendingDeletes.length) {
      undoDeleteShot();
      return;
    }
    if (b === 'lb' || b === 'rb') {
      if (sheet === 'logros') {
        const tabRecent = stageEl?.querySelector('[data-id="tab-recent"]') as HTMLElement | null;
        const tabLocked = stageEl?.querySelector('[data-id="tab-locked"]') as HTMLElement | null;
        const tabAll = stageEl?.querySelector('[data-id="tab-all"]') as HTMLElement | null;
        const tabs = [tabRecent, tabLocked, tabAll].filter(Boolean) as HTMLElement[];
        const curTabIdx = tabs.findIndex((t) => t.classList.contains('on'));
        const nextIdx = (curTabIdx + (b === 'rb' ? 1 : -1) + tabs.length) % tabs.length;
        tabs[nextIdx]?.click();
      }
      return;
    }

    // Navegación direccional
    if (focusedEl && 'slider' in focusedEl.dataset) {
      if (b === 'left' || b === 'right') { stepSlider(focusedEl, b === 'left' ? -5 : 5); return; }
      // A no hace nada aquí: un clic simulado no tiene posición y dejaría la barra a cero.
      if (b === 'a') return;
    }

    if (b === 'up') moveSpatial(0, -1);
    else if (b === 'down') moveSpatial(0, 1);
    else if (b === 'left') moveSpatial(-1, 0);
    else if (b === 'right') moveSpatial(1, 0);
    else if (b === 'a') {
      if (focusedEl) focusedEl.click();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (isTauri()) return;
    if (e.key === ' ' && !e.repeat) {
      e.preventDefault();
      if (closing) return;
      backAction();
      return;
    }

    const keyMap: Record<string, string> = {
      ArrowUp: 'up',
      ArrowDown: 'down',
      ArrowLeft: 'left',
      ArrowRight: 'right',
      Enter: 'a',
      Escape: 'b',
      Backspace: 'b',
      x: 'x',
      X: 'x',
      y: 'y',
      Y: 'y',
      q: 'lb',
      Q: 'lb',
      PageDown: 'lb',
      e: 'rb',
      E: 'rb',
      PageUp: 'rb',
      z: 'lt',
      Z: 'lt',
      '-': 'lt',
      c: 'rt',
      C: 'rt',
      '+': 'rt',
    };

    const action = keyMap[e.key];
    if (action) {
      e.preventDefault();
      onPadBtn(action);
    }
  }

  let cleanupListeners: (() => void) | null = null;
  let clockTimer: ReturnType<typeof setInterval> | null = null;

  function syncIfOpen() {
    fetchQuickmenuState().then((init) => {
      if (init.open) onOpenState(init);
    });
  }

  onMount(() => {
    fitStage();
    updateClock();
    clockTimer = setInterval(updateClock, 1000);
    window.addEventListener('resize', fitStage);
    window.addEventListener('keydown', handleKeydown);
    window.addEventListener('focus', syncIfOpen);

    cleanupListeners = setupQuickMenuListeners({
      onOpen: onOpenState,
      onClose: onCloseEvent,
      onPad: onPadBtn,
      onHold: onHoldEvent,
      onShot: onShotEvent,
      onPerf: onPerfEvent,
      onPaused: onPausedEvent,
    });

    // La ventana se precarga oculta al empezar la partida: sólo se monta el menú si ya está abierto.
    syncIfOpen();
  });

  onDestroy(() => {
    if (clockTimer) clearInterval(clockTimer);
    if (cleanupListeners) cleanupListeners();
    flushPendingTrash();
    if (typeof window !== 'undefined') {
      window.removeEventListener('resize', fitStage);
      window.removeEventListener('keydown', handleKeydown);
      window.removeEventListener('focus', syncIfOpen);
    }
  });

  // Pista de ayuda dinámica
  const hintsHtml = $derived.by(() => {
    const fid = focusedEl?.dataset.id || '';
    if (sheet !== null) {
      return [
        { k: '✚', t: 'Mover' },
        { k: 'A', t: 'Elegir', c: 'a' },
        { k: 'B', t: 'Atrás', c: 'b' },
        ...(sheet === 'logros' ? [{ k: 'LB RB', t: 'Pestañas' }] : []),
        ...(sheet === 'capturas' ? [{ k: 'Y', t: 'Borrar', c: 'y' }] : []),
      ];
    }
    if (fid === 'vol') {
      return [
        { k: '← →', t: 'Volumen' },
        { k: '↑ ↓', t: 'Salir de la barra' },
        { k: 'X', t: 'Captura', c: 'x' },
        { k: 'B', t: 'Volver', c: 'b' },
      ];
    }
    if (fid === 'close') {
      return [
        { k: 'A', t: 'Cerrar juego', c: 'a' },
        { k: 'B', t: 'Volver', c: 'b' },
      ];
    }
    if (fid === 'cap') {
      return [
        { k: 'A', t: 'Capturar', c: 'a' },
        { k: 'LT RT', t: 'Volumen' },
        { k: 'B', t: 'Volver', c: 'b' },
      ];
    }
    if (fid.startsWith('s') && !fid.startsWith('sh-')) {
      return [
        { k: 'A', t: 'Ampliar', c: 'a' },
        { k: 'Y', t: 'Borrar', c: 'y' },
        { k: 'X', t: 'Captura', c: 'x' },
        { k: 'B', t: 'Volver', c: 'b' },
      ];
    }
    return [
      { k: '✚', t: 'Mover' },
      { k: 'A', t: 'Abrir', c: 'a' },
      { k: 'X', t: 'Captura', c: 'x' },
      { k: 'LT RT', t: 'Volumen' },
      { k: 'B', t: 'Volver al juego', c: 'b' },
    ];
  });
</script>

<div class="qm-wrap">
  <div
    class="stage"
    bind:this={stageEl}
    style="transform: scale({stageScale})"
    onpointerover={(e) => {
      const target = (e.target as HTMLElement)?.closest<HTMLElement>('[data-f]');
      if (target) setFocus(target);
    }}
    oncontextmenu={(e) => {
      e.preventDefault();
      backAction();
    }}
    role="region"
    aria-label="Menú del juego"
  >
    {#if qmState?.background}
      <div class="qm-bg" style={`background-image:url("${qmState.background}")`}></div>
    {/if}

    <div class="A" class:anim={!closing} class:out={closing} class:det={sheet !== null}>
      <div
        class="scrim"
        onclick={() => { if (sheet !== null) backAction(); }}
        onkeydown={(e) => { if (e.key === 'Enter' && sheet !== null) backAction(); }}
        role="button"
        tabindex="-1"
      ></div>

      <header class="topA">
        <div class="glass gname">
          <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
          <b>{qmState?.game?.name || 'Juego'}</b>
          <small>{qmState?.game?.platformName || 'Plataforma'}</small>
        </div>

        {#if qmState?.paused}
          <span class="paused"><i></i>Juego en pausa</span>
        {/if}

        <div class="sp"></div>

        <div class="clock">
          <b>{clockTime}</b>
          <small>{clockDate}</small>
        </div>
      </header>

      <main class="gridA">
        <TilePerf
          perf={qmState?.perf ?? null}
          paused={qmState?.paused ?? false}
          view="tile"
          onOpenSheet={() => openSheet('rendimiento')}
        />

        <TileTime
          context={qmState?.context ?? null}
          game={qmState?.game ?? null}
          goals={qmState?.goals ?? { vn: 60, jvn: 45, pc: 60, switch: 30 }}
          view="tile"
          onOpenSheet={() => openSheet('tiempo')}
        />

        <TileShots
          shots={qmState?.shots ?? []}
          view="tile"
          onOpenSheet={(i) => {
            selShotIdx = i;
            openSheet('capturas');
          }}
          onCapture={handleCapture}
          onDeleteShot={deleteShotDeferred}
        />

        <TileAudio
          audio={qmState?.audio ?? null}
          view="tile"
          onOpenSheet={() => openSheet('audio')}
          onSetVolume={(v) => {
            if (qmState?.audio) qmState.audio.volume = v;
            setVolumeThrottled(v, (st) => { if (st && qmState) qmState.audio = st; });
          }}
          onToggleMute={async () => {
            if (qmState?.audio) {
              const res = await quickmenuSetMute(!qmState.audio.mute);
              if (res) qmState.audio = res;
            }
          }}
          onToggleMicMute={async () => {
            if (qmState?.audio) {
              const res = await quickmenuSetMicMute(!qmState.audio.micMute);
              if (res) qmState.audio = res;
            }
          }}
          onSelectOutput={async (id) => {
            const res = await quickmenuSelectOutput(id);
            if (res && qmState) qmState.audio = res;
          }}
        />

        <TileAch
          achievements={achievements}
          loading={achievementsLoading}
          view="tile"
          onOpenSheet={() => openSheet('logros')}
        />

        <TilePad
          pads={qmState?.pads ?? 0}
          view="tile"
          onOpenSheet={() => openSheet('mando')}
        />

        <TileExit
          onClose={() => quickmenuCloseGame()}
        />
      </main>

      {#if holdProgress >= 0}
        <div class="glass holdban">
          <i class="hf" style="width:{Math.min(100, Math.max(0, holdProgress * 100))}%"></i>
          <svg class="i" viewBox="0 0 24 24"><path d="M12 3v8"/><path d="M6.3 6.8a8 8 0 1 0 11.4 0"/></svg>
          <span>Forzando el cierre…<small>Suelta para cancelar</small></span>
        </div>
      {/if}

      {#if toastMsg}
        <div class="glass toast toptoast">
          <svg class="i" viewBox="0 0 24 24"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M10 11v6M14 11v6"/></svg>
          <div>
            {toastMsg}
          </div>
        </div>
      {/if}

      <div class="hints">
        {#each hintsHtml as h}
          <span>
            <i class="k {h.c || ''}">{h.k}</i>
            {h.t}
          </span>
        {/each}
      </div>
    </div>

    {#if sheet !== null}
      <section class="glass sheetA">
        <div class="sh-h">
          <div class="lbl">
            {#if sheet === 'rendimiento'}
              <svg class="i" viewBox="0 0 24 24"><path d="M13 2L4 14h7l-1 8 9-12h-7z"/></svg>
              Rendimiento
            {:else if sheet === 'tiempo'}
              <svg class="i" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>
              Tiempo de juego y racha
            {:else if sheet === 'capturas'}
              <svg class="i" viewBox="0 0 24 24"><rect x="3" y="7" width="18" height="13" rx="3"/><path d="M8 7l2-3h4l2 3"/><circle cx="12" cy="13.5" r="3.5"/></svg>
              Capturas de pantalla
            {:else if sheet === 'audio'}
              <svg class="i" viewBox="0 0 24 24"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>
              Audio · Dispositivos y mezclador
            {:else if sheet === 'logros'}
              <svg class="i" viewBox="0 0 24 24"><path d="M8 4h8v5a4 4 0 0 1-8 0z"/><path d="M8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 13v4M8 20h8M10 17h4"/></svg>
              Logros conseguidos
            {:else if sheet === 'mando'}
              <svg class="i" viewBox="0 0 24 24"><path d="M7 7h10a5 5 0 0 1 4.8 6.3l-1 3.7a2.6 2.6 0 0 1-4.4 1.1L14 16h-4l-2.4 2.1a2.6 2.6 0 0 1-4.4-1.1l-1-3.7A5 5 0 0 1 7 7z"/><path d="M8 10v4M6 12h4"/><circle cx="16" cy="11" r=".8"/><circle cx="17.5" cy="13" r=".8"/></svg>
              Mandos conectados
            {/if}
          </div>
          <div class="sp"></div>
          <div
            class="btn sm"
            style="height:36px;flex:none;font-size:var(--t-s);padding:0 14px"
            data-f
            data-id="sh-close"
            onclick={() => backAction()}
            onkeydown={(e) => { if (e.key === 'Enter') backAction(); }}
            role="button"
            tabindex="0"
          >
            Volver (B)
          </div>
        </div>

        <div class="sh-b">
          {#if sheet === 'rendimiento'}
            <TilePerf
              perf={qmState?.perf ?? null}
              paused={qmState?.paused ?? false}
              view="sheet"
            />
          {:else if sheet === 'tiempo'}
            <TileTime
              context={qmState?.context ?? null}
              game={qmState?.game ?? null}
              goals={qmState?.goals ?? { vn: 60, jvn: 45, pc: 60, switch: 30 }}
              view="sheet"
            />
          {:else if sheet === 'capturas'}
            <TileShots
              shots={qmState?.shots ?? []}
              view="sheet"
              selectedShot={selShotIdx}
              hasUndo={pendingDeletes.length > 0}
              onSelectShot={(i) => { selShotIdx = i; }}
              onDeleteShot={deleteShotDeferred}
              onUndoDelete={undoDeleteShot}
            />
          {:else if sheet === 'audio'}
            <TileAudio
              audio={qmState?.audio ?? null}
              view="sheet"
              onToggleMicMute={async () => {
                if (qmState?.audio) {
                  const res = await quickmenuSetMicMute(!qmState.audio.micMute);
                  if (res) qmState.audio = res;
                }
              }}
              onSelectOutput={async (id) => {
                const res = await quickmenuSelectOutput(id);
                if (res && qmState) qmState.audio = res;
              }}
              onSelectInput={async (id) => {
                const res = await quickmenuSelectInput(id);
                if (res && qmState) qmState.audio = res;
              }}
              onSetAppVolume={async (pid, v) => {
                const res = await quickmenuSetAppVolume(pid, v);
                if (res && qmState) qmState.audio = res;
              }}
              onSetAppMute={async (pid, m) => {
                const res = await quickmenuSetAppMute(pid, m);
                if (res && qmState) qmState.audio = res;
              }}
            />
          {:else if sheet === 'logros'}
            <TileAch
              achievements={achievements}
              loading={achievementsLoading}
              view="sheet"
            />
          {:else if sheet === 'mando'}
            <TilePad
              pads={qmState?.pads ?? 0}
              view="sheet"
              onRumble={(p) => quickmenuRumble(p)}
            />
          {/if}
        </div>
      </section>
    {/if}

    {#if flashActive}
      <div class="flash"></div>
    {/if}
  </div>
</div>
