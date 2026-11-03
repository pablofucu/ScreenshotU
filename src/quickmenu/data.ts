// Puente de datos y comandos del menú sobre el juego (Tauri y modo navegador).
import { listen } from '@tauri-apps/api/event';
import type { Achievement } from '../lib/types';
import { localDay, dayStart } from '../lib/readgoals';

export interface QuickMenuState {
  open: boolean;
  game: {
    id: string;
    name: string;
    platform: string;
    platformName: string;
    since: number;
    playSeconds: number;
  } | null;
  paused: boolean;
  background: string | null;
  context: null | {
    gameId: string;
    key: 'vn' | 'jvn' | null;
    titles: { gameId: string; name: string; plays: Array<[number, number]> }[];
    title: { playSeconds: number; sessions: number; week: number[]; plays: Array<[number, number]> };
    shots: { path: string; at: number }[];
  };
  goals: { vn: number; jvn: number };
  holdMs: number;
  shots: { path: string; thumb: string; at: number }[];
  audio: AudioState | null;
  perf: PerfState | null;
  pads: number;
}

export interface AudioState {
  volume: number;
  mute: boolean;
  micMute: boolean | null;
  outputs: { id: string; name: string; default: boolean }[];
  inputs: { id: string; name: string; default: boolean }[];
  apps: { pid: number; name: string; volume: number; mute: boolean; isGame: boolean }[];
}

export interface PerfState {
  cpuName: string;
  cpuThreads: number;
  gpuName: string;
  cpu: number[];
  gpu: number[];
  vramUsed: number | null;
  vramTotal: number | null;
  gpuTemp: number | null;
}

export const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function makeSvgThumb(color1: string, color2: string, label: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" viewBox="0 0 320 180">
    <defs>
      <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="${color1}"/>
        <stop offset="100%" stop-color="${color2}"/>
      </linearGradient>
    </defs>
    <rect width="320" height="180" fill="url(#g)"/>
    <circle cx="160" cy="90" r="40" fill="rgba(255,255,255,0.12)"/>
    <text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle" fill="#ffffff" font-family="sans-serif" font-size="14" font-weight="bold">${label}</text>
  </svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

function makeMockBackground(): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720" viewBox="0 0 1280 720">
    <defs>
      <radialGradient id="r1" cx="70%" cy="30%" r="60%">
        <stop offset="0%" stop-color="#ff7a9c" stop-opacity="0.45"/>
        <stop offset="100%" stop-color="#140c2e" stop-opacity="0"/>
      </radialGradient>
      <radialGradient id="r2" cx="20%" cy="80%" r="50%">
        <stop offset="0%" stop-color="#6cc6ff" stop-opacity="0.35"/>
        <stop offset="100%" stop-color="#0a0718" stop-opacity="0"/>
      </radialGradient>
      <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#0e0f1a"/>
        <stop offset="50%" stop-color="#191535"/>
        <stop offset="100%" stop-color="#2a0d33"/>
      </linearGradient>
    </defs>
    <rect width="1280" height="720" fill="url(#bg)"/>
    <rect width="1280" height="720" fill="url(#r1)"/>
    <rect width="1280" height="720" fill="url(#r2)"/>
  </svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

let mockAudio: AudioState = {
  volume: 64,
  mute: false,
  micMute: false,
  outputs: [
    { id: 'out-1', name: 'Auriculares (Arctis 7)', default: true },
    { id: 'out-2', name: 'Altavoces (Realtek Audio)', default: false },
    { id: 'out-3', name: 'TV LG (NVIDIA HDMI)', default: false },
    { id: 'out-4', name: 'Barra de sonido (Bluetooth)', default: false },
  ],
  inputs: [
    { id: 'in-1', name: 'Micrófono (Arctis 7)', default: true },
    { id: 'in-2', name: 'Micrófono (cámara web)', default: false },
  ],
  apps: [
    { pid: 1001, name: 'Fragmentos de Otoño', volume: 80, mute: false, isGame: true },
    { pid: 1002, name: 'Discord', volume: 65, mute: false, isGame: false },
    { pid: 1003, name: 'Spotify', volume: 50, mute: false, isGame: false },
    { pid: 1004, name: 'Chrome', volume: 40, mute: false, isGame: false },
  ],
};

let mockShots: { path: string; thumb: string; at: number }[] = [
  { path: 'C:/shots/shot-1.jpg', thumb: makeSvgThumb('#ff7a9c', '#9d8bff', 'Captura 1'), at: Date.now() - 3 * 60 * 1000 },
  { path: 'C:/shots/shot-2.jpg', thumb: makeSvgThumb('#6cc6ff', '#7be0a8', 'Captura 2'), at: Date.now() - 18 * 60 * 1000 },
  { path: 'C:/shots/shot-3.jpg', thumb: makeSvgThumb('#ffe680', '#ffb36b', 'Captura 3'), at: Date.now() - 41 * 60 * 1000 },
  { path: 'C:/shots/shot-4.jpg', thumb: makeSvgThumb('#9d8bff', '#6cc6ff', 'Captura 4'), at: Date.now() - 65 * 60 * 1000 },
];

let mockPerfHistoryCpu: number[] = Array.from({ length: 60 }, () => Math.round(35 + Math.random() * 15));
let mockPerfHistoryGpu: number[] = Array.from({ length: 60 }, () => Math.round(75 + Math.random() * 15));

export function buildMockState(): QuickMenuState {
  const urlParams = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();
  const caso = (urlParams.get('caso') || 'a').toLowerCase();
  const now = Date.now();
  const today = localDay(now);

  const perf: PerfState = {
    cpuName: 'AMD Ryzen 7 5800X',
    cpuThreads: 16,
    gpuName: 'NVIDIA GeForce RTX 3070',
    cpu: [...mockPerfHistoryCpu],
    gpu: [...mockPerfHistoryGpu],
    vramUsed: 6.1,
    vramTotal: 8.0,
    gpuTemp: 71,
  };

  if (caso === 'c') {
    const playSince = now - 84 * 60 * 1000;
    const titlePlays: Array<[number, number]> = [
      [now - 5 * 864e5, now - 5 * 864e5 + 11520000],
      [now - 3 * 864e5, now - 3 * 864e5 + 7200000],
      [now - 1 * 864e5, now - 1 * 864e5 + 5400000],
    ];
    return {
      open: true,
      game: {
        id: 'game-estrella',
        name: 'Estrella del Norte',
        platform: 'juegos',
        platformName: 'Juegos',
        since: playSince,
        playSeconds: 42 * 3600 + 17 * 60,
      },
      paused: true,
      background: makeMockBackground(),
      context: {
        gameId: 'game-estrella',
        key: null,
        titles: [],
        title: {
          playSeconds: 42 * 3600 + 17 * 60,
          sessions: 31,
          week: [1.2, 0, 2.5, 0.8, 3.1, 1.9, 2.1],
          plays: titlePlays,
        },
        shots: [
          { path: 'C:/shots/ctx-1.jpg', at: now - 120 * 60 * 1000 },
          { path: 'C:/shots/ctx-2.jpg', at: now - 240 * 60 * 1000 },
        ],
      },
      goals: { vn: 60, jvn: 45 },
      holdMs: 1500,
      shots: [...mockShots],
      audio: { ...mockAudio },
      perf,
      pads: 1,
    };
  }

  if (caso === 'b') {
    const playSince = now - 20 * 60 * 1000;
    const titlePlays: Array<[number, number]> = [];
    for (let d = today - 12; d < today; d++) {
      const st = dayStart(d) + 14 * 3600000;
      titlePlays.push([st, st + 65 * 60 * 1000]);
    }
    const todayEarly = dayStart(today) + 10 * 3600000;
    titlePlays.push([todayEarly, todayEarly + 64 * 60 * 1000]);

    return {
      open: true,
      game: {
        id: 'vn-otono',
        name: 'Fragmentos de Otoño',
        platform: 'vn',
        platformName: 'Novelas visuales',
        since: playSince,
        playSeconds: 42 * 3600 + 17 * 60,
      },
      paused: true,
      background: makeMockBackground(),
      context: {
        gameId: 'vn-otono',
        key: 'vn',
        titles: [{ gameId: 'vn-otono', name: 'Fragmentos de Otoño', plays: titlePlays }],
        title: {
          playSeconds: 42 * 3600 + 17 * 60,
          sessions: 31,
          week: [1.0, 1.1, 1.0, 1.2, 1.0, 1.1, 1.0],
          plays: titlePlays,
        },
        shots: [],
      },
      goals: { vn: 60, jvn: 45 },
      holdMs: 1500,
      shots: [...mockShots],
      audio: { ...mockAudio },
      perf,
      pads: 1,
    };
  }

  if (caso === 'd') {
    const dFail = today - 2;
    const titlePlays: Array<[number, number]> = [];
    for (let d = dFail - 3; d < dFail; d++) {
      const st = dayStart(d) + 14 * 3600000;
      titlePlays.push([st, st + 70 * 60 * 1000]);
    }
    const stFail = dayStart(dFail) + 14 * 3600000;
    titlePlays.push([stFail, stFail + 35 * 60 * 1000]);
    const stRec1 = dayStart(dFail + 1) + 14 * 3600000;
    titlePlays.push([stRec1, stRec1 + 74 * 60 * 1000]);
    const stTod = dayStart(today) + 8 * 3600000;
    titlePlays.push([stTod, stTod + 75 * 60 * 1000]);

    const playSince = now - 40 * 60 * 1000;
    return {
      open: true,
      game: {
        id: 'vn-otono',
        name: 'Fragmentos de Otoño',
        platform: 'vn',
        platformName: 'Novelas visuales',
        since: playSince,
        playSeconds: 42 * 3600 + 17 * 60,
      },
      paused: true,
      background: makeMockBackground(),
      context: {
        gameId: 'vn-otono',
        key: 'vn',
        titles: [{ gameId: 'vn-otono', name: 'Fragmentos de Otoño', plays: titlePlays }],
        title: {
          playSeconds: 42 * 3600 + 17 * 60,
          sessions: 31,
          week: [1.2, 0.6, 1.3, 1.2, 0, 1.2, 1.3],
          plays: titlePlays,
        },
        shots: [],
      },
      goals: { vn: 60, jvn: 45 },
      holdMs: 1500,
      shots: [...mockShots],
      audio: { ...mockAudio },
      perf,
      pads: 1,
    };
  }

  // caso 'a' (defecto): rescate pendiente
  const dFail = today - 3;
  const titlePlays: Array<[number, number]> = [];
  for (let d = dFail - 3; d < dFail; d++) {
    const st = dayStart(d) + 14 * 3600000;
    titlePlays.push([st, st + 70 * 60 * 1000]);
  }
  const stFail = dayStart(dFail) + 14 * 3600000;
  titlePlays.push([stFail, stFail + 35 * 60 * 1000]);
  const stP1 = dayStart(dFail + 1) + 14 * 3600000;
  titlePlays.push([stP1, stP1 + 68 * 60 * 1000]);
  const stShort = dayStart(dFail + 1) + 18 * 3600000;
  titlePlays.push([stShort, stShort + 6 * 60 * 1000]);
  const stP2 = dayStart(dFail + 2) + 14 * 3600000;
  titlePlays.push([stP2, stP2 + 66 * 60 * 1000]);

  const playSince = now - 38 * 60 * 1000;
  return {
    open: true,
    game: {
      id: 'vn-otono',
      name: 'Fragmentos de Otoño',
      platform: 'vn',
      platformName: 'Novelas visuales',
      since: playSince,
      playSeconds: 42 * 3600 + 17 * 60,
    },
    paused: true,
    background: makeMockBackground(),
    context: {
      gameId: 'vn-otono',
      key: 'vn',
      titles: [{ gameId: 'vn-otono', name: 'Fragmentos de Otoño', plays: titlePlays }],
      title: {
        playSeconds: 42 * 3600 + 17 * 60,
        sessions: 31,
        week: [1.2, 0.6, 1.1, 1.1, 0.6, 0, 0.6],
        plays: titlePlays,
      },
      shots: [],
    },
    goals: { vn: 60, jvn: 45 },
    holdMs: 1500,
    shots: [...mockShots],
    audio: { ...mockAudio },
    perf,
    pads: 1,
  };
}

export async function fetchQuickmenuState(): Promise<QuickMenuState> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<QuickMenuState>('quickmenu_state');
  }
  return buildMockState();
}

export async function quickmenuClose(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quickmenu_close');
  }
}

export async function quickmenuCloseGame(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quickmenu_close_game');
  }
}

export async function quickmenuCapture(): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quickmenu_capture');
  } else {
    mockShots.unshift({
      path: `C:/shots/shot-${Date.now()}.jpg`,
      thumb: makeSvgThumb('#ff7a9c', '#6cc6ff', 'Nueva'),
      at: Date.now(),
    });
  }
}

export async function quickmenuTrash(paths: string[]): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quickmenu_trash', { paths });
  } else {
    const set = new Set(paths);
    mockShots = mockShots.filter((s) => !set.has(s.path));
  }
}

export async function quickmenuRumble(port: number): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quickmenu_rumble', { port });
  }
}

export async function getGameAchievements(gameId: string): Promise<Achievement[]> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<Achievement[]>('get_game_achievements', { gameId, appId: null });
  }
  const now = Date.now();
  return [
    { appId: 'demo', apiName: 'ach_1', displayName: 'Primer amanecer', description: 'Llega al faro del norte', hidden: false, earned: true, unlockedAt: now - 12 * 60 * 1000 },
    { appId: 'demo', apiName: 'ach_2', displayName: 'Coleccionista', description: 'Reúne 50 fragmentos de estrella', hidden: false, earned: true, unlockedAt: now - 48 * 60 * 1000 },
    { appId: 'demo', apiName: 'ach_3', displayName: 'Viento a favor', description: 'Planea 2 km sin tocar el suelo', hidden: false, earned: true, unlockedAt: now - 86400000 },
    { appId: 'demo', apiName: 'ach_4', displayName: 'Cartógrafo', description: 'Descubre las 30 zonas del mapa', hidden: false, earned: false, unlockedAt: null },
    { appId: 'demo', apiName: 'ach_5', displayName: 'Sin un rasguño', description: 'Vence al guardián sin recibir daño', hidden: false, earned: false, unlockedAt: null },
    { appId: 'demo', apiName: 'ach_6', displayName: 'Pescador paciente', description: 'Pesca 20 peces raros en el lago', hidden: false, earned: false, unlockedAt: null },
  ];
}

export async function quickmenuAudio(): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_audio');
  }
  return { ...mockAudio };
}

export async function quickmenuSetVolume(volume: number): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_set_volume', { volume });
  }
  mockAudio.volume = Math.max(0, Math.min(100, Math.round(volume)));
  mockAudio.mute = false;
  return { ...mockAudio };
}

let volPending: number | null = null;
let volTimer: ReturnType<typeof setTimeout> | null = null;
let lastVolSent = 0;

export function setVolumeThrottled(volume: number, onResult?: (st: AudioState | null) => void) {
  volPending = Math.max(0, Math.min(100, Math.round(volume)));
  if (!volTimer) {
    const wait = Math.max(0, 80 - (Date.now() - lastVolSent));
    volTimer = setTimeout(async () => {
      volTimer = null;
      if (volPending !== null) {
        const v = volPending;
        volPending = null;
        lastVolSent = Date.now();
        const res = await quickmenuSetVolume(v);
        if (onResult) onResult(res);
      }
    }, wait);
  }
}

export async function quickmenuSetMute(mute: boolean): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_set_mute', { mute });
  }
  mockAudio.mute = mute;
  return { ...mockAudio };
}

export async function quickmenuSetMicMute(mute: boolean): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_set_mic_mute', { mute });
  }
  mockAudio.micMute = mute;
  return { ...mockAudio };
}

export async function quickmenuSelectOutput(id: string): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_select_output', { id });
  }
  mockAudio.outputs = mockAudio.outputs.map((o) => ({ ...o, default: o.id === id }));
  return { ...mockAudio };
}

export async function quickmenuSelectInput(id: string): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_select_input', { id });
  }
  mockAudio.inputs = mockAudio.inputs.map((i) => ({ ...i, default: i.id === id }));
  return { ...mockAudio };
}

export async function quickmenuSetAppVolume(pid: number, volume: number): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_set_app_volume', { pid, volume });
  }
  const app = mockAudio.apps.find((a) => a.pid === pid);
  if (app) {
    app.volume = Math.max(0, Math.min(100, Math.round(volume)));
    app.mute = false;
  }
  return { ...mockAudio };
}

export async function quickmenuSetAppMute(pid: number, mute: boolean): Promise<AudioState | null> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AudioState>('quickmenu_set_app_mute', { pid, mute });
  }
  const app = mockAudio.apps.find((a) => a.pid === pid);
  if (app) app.mute = mute;
  return { ...mockAudio };
}

export async function getThumbUrl(path: string): Promise<string> {
  if (isTauri()) {
    const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
    const thumbPath = await invoke<string>('get_thumb', { path });
    return convertFileSrc(thumbPath);
  }
  return makeSvgThumb('#6cc6ff', '#9d8bff', 'Captura');
}

export interface QuickMenuEvents {
  onOpen: (state: QuickMenuState) => void;
  onClose: () => void;
  onPad: (btn: string) => void;
  onHold: (p: number) => void;
  onShot: (shot: { path: string; thumb: string; at: number }) => void;
  onPerf: (perf: PerfState) => void;
}

export function setupQuickMenuListeners(events: QuickMenuEvents): () => void {
  let cleaned = false;
  const unlistens: Array<() => void> = [];

  if (isTauri()) {
    const reg = (p: Promise<() => void>) => {
      p.then((u) => {
        if (cleaned) u();
        else unlistens.push(u);
      });
    };
    reg(listen<QuickMenuState>('qm-open', (e) => events.onOpen(e.payload)));
    reg(listen('qm-close', () => events.onClose()));
    reg(listen<{ btn: string }>('qm-pad', (e) => events.onPad(e.payload.btn)));
    reg(listen<{ p: number }>('qm-hold', (e) => events.onHold(e.payload.p)));
    reg(listen<{ path: string; thumb: string; at: number }>('qm-shot', (e) => events.onShot(e.payload)));
    reg(listen<PerfState>('qm-perf', (e) => events.onPerf(e.payload)));
  }

  return () => {
    cleaned = true;
    for (const u of unlistens) u();
  };
}
