// Biblioteca simulada para desarrollo en navegador (sin Tauri). El arte procede del mockup aprobado.
import { hash, rng } from './rng';
import type {
  AccentInfo, AudioOutput, Backend, BackendEvent, Config, GameSource, HiddenEntry, Library, Platform, Saga, Shot, SortMode, Swatch, Title,
} from './types';
import { normalizeHex } from './oklch';



function scene(ctx: CanvasRenderingContext2D, w: number, h: number, seed: number, hue: number, vn: boolean, hud: boolean){
  const r=rng(seed), night=r()<.3;
  const g=ctx.createLinearGradient(0,0,0,h);
  g.addColorStop(0, night?`hsl(${hue+220},45%,14%)`:`hsl(${hue},68%,${60+r()*14}%)`);
  g.addColorStop(1, night?`hsl(${hue+260},50%,32%)`:`hsl(${hue+35},85%,87%)`);
  ctx.fillStyle=g;ctx.fillRect(0,0,w,h);
  if(night){ctx.fillStyle='rgba(255,255,255,.8)';for(let i=0;i<70;i++){const s=r()*1.6*w/600+.4;ctx.fillRect(r()*w,r()*h*.55,s,s)}}
  const sx=w*(.15+.7*r()), sy=h*(.16+.22*r()), sr=Math.min(w,h)*(.07+.07*r());
  ctx.save();ctx.shadowColor=night?'rgba(255,250,210,.7)':`hsla(${hue+40},100%,95%,.9)`;ctx.shadowBlur=sr*1.6;
  ctx.fillStyle=night?'#f4f1d0':`hsla(${hue+40},100%,95%,.95)`;ctx.beginPath();ctx.arc(sx,sy,sr,0,7);ctx.fill();ctx.restore();
  const city=r()<.3;
  for(let L=0;L<4;L++){
    const base=h*(.46+.14*L);
    ctx.fillStyle=night?`hsl(${hue+230},28%,${24-L*5}%)`:`hsl(${hue+L*14},${30+L*9}%,${62-L*12}%)`;
    ctx.beginPath();ctx.moveTo(0,h);
    if(city&&L===2){let x=0;while(x<w){const bw=w*(.03+.05*r()),bh=h*(.08+.22*r());ctx.lineTo(x,base-bh);ctx.lineTo(x+bw,base-bh);x+=bw}ctx.lineTo(w,h);ctx.fill();
      ctx.fillStyle=night?'rgba(255,220,140,.75)':'rgba(255,255,255,.35)';for(let i=0;i<120;i++){ctx.fillRect(r()*w,base-r()*h*.25,w/260+1,h/180+1)}continue}
    const steps=5+Math.floor(r()*6);
    for(let i=0;i<=steps;i++){const x=w*i/steps, y=base-r()*h*(.12+.08*(3-L));ctx.lineTo(x,y)}
    ctx.lineTo(w,h);ctx.closePath();ctx.fill();
    if(L<2){const fg=ctx.createLinearGradient(0,base-h*.1,0,base+h*.12);fg.addColorStop(0,'rgba(255,255,255,0)');fg.addColorStop(1,night?'rgba(120,130,190,.12)':'rgba(255,255,255,.28)');ctx.fillStyle=fg;ctx.fillRect(0,base-h*.1,w,h*.25)}
  }
  const u=w/640;
  if(vn){
    ctx.fillStyle=`hsla(${hue+180},30%,${night?70:22}%,.82)`;
    const cx=w*(.35+.3*r());ctx.beginPath();ctx.arc(cx,h*.36,h*.12,0,7);ctx.fill();
    ctx.beginPath();ctx.moveTo(cx-h*.26,h);ctx.quadraticCurveTo(cx-h*.24,h*.52,cx,h*.5);ctx.quadraticCurveTo(cx+h*.24,h*.52,cx+h*.26,h);ctx.fill();
    ctx.fillStyle='rgba(18,20,38,.72)';rr(ctx,w*.04,h*.7,w*.92,h*.26,14*u);ctx.fill();ctx.strokeStyle='rgba(255,255,255,.55)';ctx.lineWidth=2*u;ctx.stroke();
    ctx.fillStyle=`hsl(${hue},65%,58%)`;rr(ctx,w*.06,h*.655,w*.18,h*.075,8*u);ctx.fill();
    ctx.fillStyle='rgba(255,255,255,.82)';[.82,.6].forEach((lw,i)=>{rr(ctx,w*.08,h*(.78+i*.075),w*lw*.82*(0.7+r()*.3),h*.03,6*u);ctx.fill()});
  } else if(hud){
    ctx.fillStyle='rgba(0,0,0,.35)';rr(ctx,18*u,16*u,170*u,16*u,8*u);ctx.fill();
    ctx.fillStyle='#E8384F';rr(ctx,20*u,18*u,(80+r()*80)*u,12*u,6*u);ctx.fill();
    ctx.fillStyle='rgba(0,0,0,.35)';rr(ctx,18*u,38*u,120*u,10*u,5*u);ctx.fill();
    ctx.fillStyle='#3BB54A';rr(ctx,19*u,39*u,(40+r()*70)*u,8*u,4*u);ctx.fill();
    ctx.strokeStyle='rgba(255,255,255,.7)';ctx.lineWidth=3*u;ctx.fillStyle='rgba(0,0,0,.3)';ctx.beginPath();ctx.arc(w-58*u,h-58*u,38*u,0,7);ctx.fill();ctx.stroke();
  }
}
function rr(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number){ctx.beginPath();ctx.moveTo(x+r,y);ctx.arcTo(x+w,y,x+w,y+h,r);ctx.arcTo(x+w,y+h,x,y+h,r);ctx.arcTo(x,y+h,x,y,r);ctx.arcTo(x,y,x+w,y,r);ctx.closePath()}

const MOCK_PLATS = [
  { id: 'pc', name: 'Juegos PC', folder: 'PC', accent: 4, icon: 'pc', vn: false, res: ['2560×1440', '3840×2160', '1920×1080'],
    titles: ['Elden Ring', 'Cyberpunk 2077', 'Hades II', 'Hollow Knight', "Baldur's Gate 3", 'Stardew Valley', 'Celeste', 'Disco Elysium', 'Red Dead Redemption 2', 'The Witcher 3', 'Outer Wilds', 'Sekiro', 'Hi-Fi Rush', 'Balatro'] },
  { id: 'switch', name: 'Juegos Switch', folder: 'Switch', accent: 0, icon: 'switch', vn: false, res: ['1920×1080', '1280×720'],
    titles: ['Zelda: Tears of the Kingdom', 'Super Mario Odyssey', 'Xenoblade Chronicles 3', 'Animal Crossing', 'Metroid Dread', 'Splatoon 3', 'Fire Emblem Engage', 'Kirby y la tierra olvidada'] },
  { id: 'vn', name: 'Novelas visuales', folder: 'Novelas visuales', accent: 6, icon: 'vn', vn: true, res: ['1920×1080', '1280×720'],
    titles: ['Steins;Gate', 'Clannad', 'Danganronpa', 'Doki Doki Literature Club', 'Ace Attorney', 'Fate/stay night', '999', 'Umineko'] },
];

// Juegos añadidos sin capturas todavía.
const EXTRA_PC = ['Hades', 'Vampire Survivors', 'Dave the Diver'];
const NOT_LAUNCHABLE = new Set(['pc|Sekiro', 'pc|Hi-Fi Rush', 'pc|Balatro', 'switch|Splatoon 3', 'vn|Umineko', 'vn|999', 'vn|Fate/stay night']);
// Casos de arte para probar la ficha: sin hero (se usa la portada borrosa) y sin logo (nombre en texto).
const NO_HERO = new Set(['pc|Hollow Knight', 'pc|Hi-Fi Rush', 'vn|Clannad']);
const NO_LOGO = new Set(['pc|Hades II', 'pc|Hi-Fi Rush', 'switch|Metroid Dread']);
const EMULATOR = 'Ryujinx';
const MANUAL = new Set(['Stardew Valley', 'Steins;Gate', 'Clannad']);
const AGO_H: Record<string, number> = { 'Elden Ring': 3, 'Hades II': 30, 'Zelda: Tears of the Kingdom': 27, 'Steins;Gate': 80, Hades: 200 };
const NOW = Date.UTC(2026, 9, 14, 18, 0);

function gameInfo(pid: string, name: string): { src: GameSource | null } {
  if (NOT_LAUNCHABLE.has(pid + '|' + name)) return { src: null };
  if (pid === 'switch') return { src: 'emulator' };
  if (MANUAL.has(name)) return { src: 'manual' };
  return { src: pid === 'vn' ? null : 'manual' };
}

interface MockTitleMeta { hue: number; vn: boolean }
const meta = new Map<string, MockTitleMeta>(); // "pid|titulo" -> datos de arte

/** Paleta simulada: los tonos con los que `scene()` pinta cielo, horizonte y noche, normalizados como hace el núcleo en Rust. */
function hslHex(h: number, s: number, l: number): string {
  const a = (s / 100) * Math.min(l / 100, 1 - l / 100);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const c = l / 100 - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return Math.round(c * 255).toString(16).padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}
function mockAccent(hue: number): AccentInfo {
  const out: Swatch[] = [];
  const seen: number[] = [];
  for (const h of [hue, hue + 35, hue + 220, hue + 120, hue + 300]) {
    const n = normalizeHex(hslHex(((h % 360) + 360) % 360, 68, 58));
    if (!n) continue;
    const hh = ((h % 360) + 360) % 360;
    if (seen.some((x) => Math.min(Math.abs(x - hh), 360 - Math.abs(x - hh)) < 24)) continue;
    seen.push(hh);
    out.push(n);
  }
  return { accent: out[0], palette: out };
}
/** Juego sin color en su arte: el acento cae al arcoíris. */
const NO_ACCENT = new Set(['pc|Balatro']);

/** Partidas simuladas: rachas de juego por la tarde-noche repartidas en los últimos meses, hasta `last`. */
function mockPlays(r: () => number, played: number, last: number | null): Array<[number, number]> {
  if (played <= 0) return [];
  const out: Array<[number, number]> = [];
  const end = last ?? NOW - Math.floor(r() * 60) * 864e5;
  const span = Math.floor(40 + r() * 200);
  let left = played * 1000;
  for (let d = 0; d < span && left > 0; d++) {
    // Temporadas: unas semanas con mucho juego y otras sin nada.
    if (r() > (Math.sin(d / 9 + played) > 0 ? 0.7 : 0.15)) continue;
    const day = new Date(end - d * 864e5);
    day.setHours(d === 0 ? new Date(end).getHours() : 16 + Math.floor(r() * 6), Math.floor(r() * 60), 0, 0);
    const len = Math.min(left, (20 + Math.floor(r() * 160)) * 60000);
    out.push([day.getTime(), day.getTime() + len]);
    left -= len;
  }
  return out.sort((a, b) => a[0] - b[0]);
}
function weekFromPlays(plays: Array<[number, number]>): number[] {
  const t0 = new Date(NOW); t0.setHours(0, 0, 0, 0);
  return Array.from({ length: 7 }, (_, i) => {
    const a = t0.getTime() - (6 - i) * 864e5, b = a + 864e5;
    return Math.round(plays.reduce((s, p) => s + Math.max(0, Math.min(p[1], b) - Math.max(p[0], a)), 0) / 60000);
  });
}

function mockTitle(p: (typeof MOCK_PLATS)[number], name: string, withShots: boolean): Title {
  const r = rng(hash(p.id + name));
  const n = withShots ? 12 + Math.floor(r() * 44) : 0;
  let d = Date.UTC(2026, 9, 12) - Math.floor(r() * 40) * 864e5;
  const shots: Shot[] = [];
  for (let i = 0; i < n; i++) {
    const res = p.res[Math.floor(r() * p.res.length)];
    const [w, h] = res.split('×').map(Number);
    shots.push({ path: `mock://${p.id}/${name}/${i}`, mtime: d, date: d, width: w, height: h });
    d -= Math.floor((0.2 + r() * 4) * 864e5) + Math.floor(r() * 86400) * 1000;
  }
  meta.set(p.id + '|' + name, { hue: Math.floor(r() * 360), vn: p.vn });
  const { src } = gameInfo(p.id, name);
  const ago = AGO_H[name];
  const played = src && ago !== undefined ? 3600 * (5 + Math.floor(r() * 150)) + Math.floor(r() * 3600) : src && r() < 0.5 ? Math.floor(r() * 40 * 3600) : 0;
  const lastPlayed = src && ago !== undefined ? NOW - ago * 3600e3 : null;
  const plays = mockPlays(r, played, lastPlayed);
  // Algo de historia del año anterior (para probar el resumen anual y su comparación).
  if (played > 0 && r() < 0.7) {
    const r2 = rng(hash(p.id + name + 'prev'));
    plays.unshift(...mockPlays(r2, 3600 * (4 + Math.floor(r2() * 60)), Date.UTC(2025, 10, 28 - Math.floor(r2() * 40), 20)));
  }
  const week = weekFromPlays(plays);
  const key = p.id + '|' + name;
  const hue = meta.get(key)!.hue;
  const gid = src ? `mock:${p.id}:${name}` : null;
  return {
    id: gid ?? `${p.id}/${name.replace(/:/g, ' -').toLowerCase()}`, hidden: false, accentOverride: null, sortMode: null,
    accent: NO_ACCENT.has(key) ? null : mockAccent(hue),
    name, cover: `mock://${p.id}/${name}/cover`, shots,
    hero: NO_HERO.has(key) ? null : `mock://${p.id}/${name}/hero`,
    logo: NO_LOGO.has(key) ? null : `mock://${p.id}/${name}/logo`,
    launcher: src === 'emulator' ? EMULATOR : null,
    sessions: plays.length,
    firstPlayed: plays[0]?.[0] ?? null,
    week, plays,
    folder: name.replace(/:/g, ' -'), dir: `D:\\Capturas\\${p.folder}\\${name.replace(/:/g, ' -')}`, hasFolder: withShots,
    gameId: gid, source: src, launchable: !!src, removable: src === 'manual' || src === 'emulator',
    lastPlayed: lastPlayed ?? plays.at(-1)?.[0] ?? null,
    playSeconds: plays.reduce((a, p) => a + Math.round((p[1] - p[0]) / 1000), 0),
  };
}

const byRecent = (a: Title, b: Title) =>
  (b.lastPlayed ?? -Infinity) - (a.lastPlayed ?? -Infinity) ||
  (b.shots[0]?.date ?? -Infinity) - (a.shots[0]?.date ?? -Infinity) ||
  a.name.localeCompare(b.name);

function buildLibrary(): Library {
  const platforms: Platform[] = MOCK_PLATS.map((p) => {
    const titles: Title[] = p.titles.map((name) => mockTitle(p, name, true));
    if (p.id === 'pc') for (const name of EXTRA_PC) titles.push(mockTitle(p, name, false));
    titles.sort(byRecent);
    return { id: p.id, name: p.name, folder: p.folder, accent: p.accent, icon: p.icon, titles };
  });
  return { platforms, hidden: [] };
}

function cv(w: number, h: number) { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; }

function drawCover(pid: string, name: string): string {
  const m = meta.get(pid + '|' + name)!;
  const c = cv(300, 400), x = c.getContext('2d')!;
  scene(x, 300, 400, hash(name) + 7, m.hue, m.vn, false);
  const g = x.createLinearGradient(0, 220, 0, 400); g.addColorStop(0, 'rgba(0,0,0,0)'); g.addColorStop(1, 'rgba(0,0,0,.72)'); x.fillStyle = g; x.fillRect(0, 200, 300, 200);
  x.fillStyle = '#fff'; x.textBaseline = 'alphabetic'; let size = 40; x.font = `800 ${size}px "M PLUS Rounded 1c", sans-serif`;
  const words = name.split(' '); let lines: string[] = [], line = '';
  const wrap = () => { lines = []; line = ''; for (const wd of words) { const t = line ? line + ' ' + wd : wd; if (x.measureText(t).width > 260 && line) { lines.push(line); line = wd; } else line = t; } lines.push(line); };
  wrap(); while ((lines.length > 3 || lines.some((l) => x.measureText(l).width > 264)) && size > 20) { size -= 2; x.font = `800 ${size}px "M PLUS Rounded 1c", sans-serif`; wrap(); }
  x.shadowColor = 'rgba(0,0,0,.5)'; x.shadowBlur = 8;
  lines.forEach((l, i) => x.fillText(l, 20, 380 - (lines.length - 1 - i) * size * 1.08));
  return c.toDataURL('image/jpeg', 0.86);
}

function drawHero(pid: string, name: string): string {
  const m = meta.get(pid + '|' + name)!;
  const c = cv(1280, 413);
  scene(c.getContext('2d')!, 1280, 413, hash(name) + 11, m.hue, m.vn, false);
  return c.toDataURL('image/jpeg', 0.86);
}

/** Logo generado: PNG transparente con el nombre en trazo oscuro y relleno claro (no es arte de ningún juego real). */
function drawLogo(pid: string, name: string): string {
  const m = meta.get(pid + '|' + name)!;
  const c = cv(560, 160), x = c.getContext('2d')!;
  let size = 76; x.font = `italic 800 ${size}px "M PLUS Rounded 1c", sans-serif`;
  while (x.measureText(name).width > 520 && size > 24) { size -= 2; x.font = `italic 800 ${size}px "M PLUS Rounded 1c", sans-serif`; }
  x.textBaseline = 'middle'; x.lineJoin = 'round';
  x.strokeStyle = `hsl(${m.hue},60%,16%)`; x.lineWidth = size * 0.2; x.strokeText(name, 20, 80);
  const g = x.createLinearGradient(0, 40, 0, 120); g.addColorStop(0, '#fff'); g.addColorStop(1, `hsl(${m.hue},80%,82%)`);
  x.fillStyle = g; x.fillText(name, 20, 80);
  return c.toDataURL('image/png');
}

function drawShot(pid: string, name: string, idx: number, big: boolean): string {
  const m = meta.get(pid + '|' + name)!;
  const w = big ? 1280 : 384, h = big ? 720 : 216, c = cv(w, h);
  scene(c.getContext('2d')!, w, h, hash(`${pid}|${name}|${idx}`), m.hue + ((idx * 17) % 50) - 25, m.vn, idx % 3 !== 1);
  return c.toDataURL('image/jpeg', big ? 0.9 : 0.82);
}

function parse(path: string): { pid: string; name: string; idx: number | 'cover' | 'hero' | 'logo' } {
  const [pid, ...rest] = path.slice('mock://'.length).split('/');
  const last = rest.pop()!;
  return { pid, name: rest.join('/'), idx: last === 'cover' || last === 'hero' || last === 'logo' ? last : Number(last) };
}

/** Estados de ejemplo para «Mi lista» en el navegador (solo la primera vez; luego manda lo guardado). */
function mockStates(lib: Library): Record<string, { status: string; finishedMs?: number; rating?: number; listPos?: number; statusMs: number }> {
  const out: Record<string, { status: string; finishedMs?: number; rating?: number; listPos?: number; statusMs: number }> = {};
  const idOf = (pid: string, name: string) => lib.platforms.find((p) => p.id === pid)?.titles.find((t) => t.name === name)?.id;
  const put = (pid: string, name: string, v: Omit<(typeof out)[string], 'statusMs'>) => { const id = idOf(pid, name); if (id) out[id] = { ...v, statusMs: NOW - 2 * 864e5 }; };
  const d = (y: number, m: number, day: number) => Date.UTC(y, m - 1, day, 12);
  for (const n of ['Elden Ring', 'Hades II', 'Hollow Knight']) put('pc', n, { status: 'playing' });
  ['Disco Elysium', "Baldur's Gate 3", 'Outer Wilds', 'Sekiro'].forEach((n, i) => put('pc', n, { status: 'pending', listPos: i + 1 }));
  put('pc', 'Celeste', { status: 'finished', finishedMs: d(2026, 8, 14), rating: 5 });
  put('pc', 'Stardew Valley', { status: 'finished', finishedMs: d(2026, 3, 2), rating: 4 });
  put('pc', 'Balatro', { status: 'finished', finishedMs: d(2026, 9, 30), rating: 3 });
  put('pc', 'The Witcher 3', { status: 'finished', finishedMs: d(2025, 11, 20), rating: 5 });
  put('pc', 'Red Dead Redemption 2', { status: 'finished', finishedMs: d(2025, 6, 1) });
  put('pc', 'Cyberpunk 2077', { status: 'dropped' });
  put('switch', 'Zelda: Tears of the Kingdom', { status: 'playing' });
  put('switch', 'Xenoblade Chronicles 3', { status: 'pending', listPos: 5 });
  put('switch', 'Metroid Dread', { status: 'pending', listPos: 6 });
  put('switch', 'Super Mario Odyssey', { status: 'finished', finishedMs: d(2026, 5, 9), rating: 4 });
  return out;
}

const LS = 'visor.mock.config';
let pickN = 0;
function store(): Storage | null { try { return window.localStorage; } catch { return null; } }

export async function mockBackend(): Promise<Backend> {
  try { await Promise.race([document.fonts.load('800 40px "M PLUS Rounded 1c"'), new Promise((r) => setTimeout(r, 1500))]); } catch {}
  const q = new URLSearchParams(location.search);
  const base: Config = { root: 'mock://biblioteca', titlesView: 'grid', shotsView: 'grid', shotsOrder: 'recent', accentMode: 'art', accentFixed: 4, sound: false,
    platforms: MOCK_PLATS.map((p) => ({ id: p.id, name: p.name, folder: p.folder, accent: p.accent, icon: p.icon })),
    captureHotkey: 'F12', captureCombo: 'back+rb', captureMouse: '', captureMagpie: 'scaled', captureSound: true, capturePeek: true, captureRumble: true, magpieExe: '', showHidden: false,
    sgdbKey: q.has('sgdb') ? 'mock' : '', sgdbAuto: true, deepseekKey: '', deepseekModel: 'deepseek-chat',
    ankiUrl: 'http://127.0.0.1:8765', ankiDeck: 'Japonés::Novelas', ankiModel: '', ankiStyle: 'limpio', ankiAutostart: true, ankiExe: '', recordHotkey: 'F9', audioCombo: 'start+rb', audioHotkey: '', autoBackup: true, openOnCursor: false, savesAuto: true, savesDir: '', goalVn: 60, goalJvn: 30, remoteEnabled: false, remotePort: 47811, remoteToken: '', titlesOrder: {}, titlesZoom: 0, listStalled: true, listStalledDays: 30, listStalledAction: 'ask', listAutoPlay: true, ankiMap: { word: 'Word', reading: 'WordReading', meaning: 'PrimaryDefinition', sentence: 'Sentence', translation: 'SentenceTranslation', shot: 'Picture', audio: 'SentenceAudio' } };
  let cfg: Config = base;
  try { const s = store()?.getItem(LS); if (s) cfg = { ...base, ...JSON.parse(s) }; } catch {}
  if (q.has('empty')) cfg = { ...cfg, root: null };
  if (q.get('view') === 'carousel') cfg = { ...cfg, titlesView: 'carousel' };
  const lib = buildLibrary();
  let mockOuts: AudioOutput[] = [{ id: "a", name: "Altavoces (Realtek Audio)", short: "Altavoces", default: true }, { id: "b", name: "Auriculares (USB Headset)", short: "Auriculares", default: false }, { id: "c", name: "LG ULTRAGEAR (NVIDIA)", short: "LG ULTRAGEAR", default: false }];
  let emit: ((e: BackendEvent) => void) | undefined;
  const cache = new Map<string, string>();
  let favs: string[] = [];
  try { favs = JSON.parse(store()?.getItem('visor.mock.favs') ?? '[]'); } catch {}
  // Datos del usuario (en el navegador, en localStorage): ocultos y preferencias por juego.
  const load = <T,>(k: string, d: T): T => { try { const v = store()?.getItem(k); return v ? JSON.parse(v) : d; } catch { return d; } };
  const save = (k: string, v: unknown) => { try { store()?.setItem(k, JSON.stringify(v)); } catch {} };
  let hidden: HiddenEntry[] = load('visor.mock.hidden', []);
  type MockPref = { accent?: string; sort?: string; status?: string; finishedMs?: number; rating?: number; listPos?: number; statusMs?: number; snoozeMs?: number };
  const prefs: Record<string, MockPref> = load('visor.mock.prefs', mockStates(lib));
  // Capturas movidas: la copia nueva se dibuja igual que la original.
  const alias = new Map<string, string>();
  let seq = 5000;
  const allTitles = () => lib.platforms.flatMap((p) => p.titles.map((t) => ({ p, t })));
  const urlFor = (path: string, big: boolean) => {
    const k = (big ? 'B' : 's') + path;
    let u = cache.get(k);
    if (!u) {
      const { pid, name, idx } = parse(alias.get(path) ?? path);
      u = idx === 'cover' ? drawCover(pid, name) : idx === 'hero' ? drawHero(pid, name) : idx === 'logo' ? drawLogo(pid, name) : drawShot(pid, name, idx, big);
      cache.set(k, u);
    }
    return u;
  };
  return {
    isTauri: false,
    async loadConfig() { return cfg; },
    async saveConfig(c) { cfg = c; try { store()?.setItem(LS, JSON.stringify(c)); } catch {} },
    async learnPad() {},
    async audioOutputs() { return mockOuts; },
    async setAudioOutput(id) {
      mockOuts = mockOuts.map((o) => ({ ...o, default: o.id === id }));
      emit?.({ type: 'audio-outputs', outputs: mockOuts });
      return mockOuts;
    },
    async hotkeySuspend() {},
    async pickFolder(title) { return title ? 'D:\Juegos' : 'mock://biblioteca'; },
    async scan() {
      await new Promise((r) => setTimeout(r, 150));
      if (!cfg.root || q.has('nolib')) return { platforms: lib.platforms.map((p) => ({ ...p, titles: [] })), hidden: [] };
      const ids = new Set(hidden.map((h) => h.id));
      const platforms = lib.platforms.map((p) => ({
        ...p,
        titles: p.titles
          .map((t) => ({ ...t, shots: [...t.shots], hidden: ids.has(t.id), accentOverride: prefs[t.id]?.accent ?? null, sortMode: (prefs[t.id]?.sort as SortMode | undefined) ?? null, status: (prefs[t.id]?.status as Title['status']) ?? null, finishedMs: prefs[t.id]?.finishedMs ?? null, rating: prefs[t.id]?.rating ?? null, listPos: prefs[t.id]?.listPos ?? null, statusMs: prefs[t.id]?.statusMs ?? null, snoozeMs: prefs[t.id]?.snoozeMs ?? null }))
          .filter((t) => cfg.showHidden || !t.hidden),
      }));
      return { platforms, hidden: [...hidden] };
    },
    thumb(path, signal) {
      return new Promise((resolve, reject) => {
        setTimeout(() => (signal?.aborted ? reject(new DOMException('cancelado', 'AbortError')) : resolve(urlFor(path, false))), 0);
      });
    },
    fullUrl: (path) => urlFor(path, true),
    artUrl: (path) => urlFor(path, true),
    bustArt() {},
    prefetch() {},
    async getFavorites() { return favs; },
    async setFavorite(path, on) {
      favs = favs.filter((p) => p !== path); if (on) favs.push(path);
      try { store()?.setItem('visor.mock.favs', JSON.stringify(favs)); } catch {}
    },
    async toggleFullscreen() { if (document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen?.(); },
    async isFullscreen() { return !!document.fullscreenElement; },
    async windowMonitor() { return '1 de 1 · simulado'; },
    async nextMonitor() { return '1 de 1 · simulado'; },
    async exit() { window.close(); },
    async launch(id) {
      // En el navegador solo se simula: se anuncia el inicio y la salida se simula con __visorEmit.
      const t = lib.platforms.flatMap((p) => p.titles).find((x) => x.gameId === id);
      if (t) emit?.({ type: 'game-started', id, name: t.name });
    },
    async goalNotice(_id, at, text) {
      // En el navegador no hay ventana sobre el juego: se deja en la consola.
      setTimeout(() => console.info('[aviso sobre el juego]', text), Math.max(0, at - Date.now()));
    },
    async openFolder() {},
    async addGame(g) {
      await new Promise((r) => setTimeout(r, 60));
      const p = MOCK_PLATS.find((x) => x.id === g.platform) ?? MOCK_PLATS[0];
      const plat = lib.platforms.find((x) => x.id === p.id)!;
      const t = mockTitle(p, g.name, false);
      // Con arte propio se ve; sin él, el juego sale sin hero/logo/portada (como una carpeta vacía).
      Object.assign(t, {
        hero: g.hero ? t.hero : null, logo: g.logo ? t.logo : null, cover: g.cover ? t.cover : null,
        accent: g.hero || g.cover ? t.accent : null, hasFolder: g.kind === 'captures' || !!(g.hero || g.logo || g.cover),
      });
      if (g.kind === 'captures') {
        Object.assign(t, { gameId: null, source: null, launchable: false, removable: true, launcher: null, id: `${p.id}/${g.name.toLowerCase()}`, playSeconds: 0, sessions: 0, week: Array(7).fill(0), plays: [], firstPlayed: null });
      } else {
        const src: GameSource = g.kind === 'emulator' ? 'emulator' : 'manual';
        Object.assign(t, { gameId: `mock:${p.id}:${g.name}`, id: `mock:${p.id}:${g.name}`, source: src, launcher: src === 'emulator' ? EMULATOR : null, launchable: true, removable: true });
      }
      plat.titles.push(t);
      plat.titles.sort(byRecent);
      return { notes: [] };
    },
    async scanGamesDir() {
      await new Promise((r) => setTimeout(r, 200));
      const ex = (f: string, ...n: string[]) => n.map((x) => `D:\Juegos\${f}\${x}`);
      return {
        candidates: [
          { folder: 'Portal 2', name: 'Portal 2', exes: ex('Portal 2', 'portal2.exe', 'bin\SDKLauncher.exe') },
          { folder: 'Left 4 Dead 2', name: 'Left 4 Dead 2', exes: ex('Left 4 Dead 2', 'left4dead2.exe') },
          { folder: 'Rocket League', name: 'Rocket League', exes: ex('Rocket League', 'Binaries\Win64\RocketLeague.exe', 'Launcher.exe') },
        ],
        empty: 1, known: 0,
      };
    },
    async setHidden(e, on) {
      hidden = hidden.filter((h) => h.id !== e.id);
      if (on) hidden.push(e);
      save('visor.mock.hidden', hidden);
    },
    async setAccentOverride(id, color) {
      const p = (prefs[id] ??= {});
      if (color) p.accent = color; else delete p.accent;
      if (!p.accent && !p.sort && !p.status) delete prefs[id];
      save('visor.mock.prefs', prefs);
    },
    async setSortPref(id, mode) {
      const p = (prefs[id] ??= {});
      if (mode === 'recent') delete p.sort; else p.sort = mode;
      if (!p.accent && !p.sort && !p.status) delete prefs[id];
      save('visor.mock.prefs', prefs);
    },
    async setTitleStatus(id, status, finishedMs, rating) {
      const p = (prefs[id] ??= {});
      const nextPos = Math.max(0, ...Object.values(prefs).map((x) => x.listPos ?? 0)) + 1;
      if (status === 'none') { delete p.status; delete p.finishedMs; delete p.rating; delete p.listPos; delete p.snoozeMs; delete p.statusMs; }
      else {
        if (status === 'finished') {
          if (p.status !== 'finished' || !p.finishedMs) p.finishedMs = Date.now();
          if (finishedMs) p.finishedMs = finishedMs;
          if (rating && rating >= 1 && rating <= 5) p.rating = rating; else delete p.rating;
        } else { delete p.finishedMs; delete p.rating; }
        if (status === 'pending') p.listPos ??= nextPos; else delete p.listPos;
        if (p.status !== status) p.statusMs = Date.now();
        delete p.snoozeMs;
        p.status = status;
      }
      if (!p.accent && !p.sort && !p.status) delete prefs[id];
      save('visor.mock.prefs', prefs);
    },
    async setPendingOrder(ids) {
      ids.forEach((id, i) => { const p = prefs[id]; if (p?.status === 'pending') p.listPos = i + 1; });
      save('visor.mock.prefs', prefs);
    },
    async snoozeStalled(ids) {
      for (const id of ids) if (prefs[id]?.status) prefs[id].snoozeMs = Date.now();
      save('visor.mock.prefs', prefs);
    },
    async pickSaveZip(name) { return 'C:/Usuarios/Mock/' + name; },
    async pickZip() { return 'C:/Usuarios/Mock/visor-copia-simulada.zip'; },
    async backupCreate(dest, media) { await new Promise((r) => setTimeout(r, 600)); return { path: dest, size: media ? 48_300_000 : 1_250_000, mediaFiles: media ? 312 : 0 }; },
    async restorePrepare() { await new Promise((r) => setTimeout(r, 400)); return { dbVersion: 2, size: 1_250_000, hasConfig: true, mediaFiles: 0 }; },
    async restoreCancel() {},
    async restoreApply() { throw 'Reiniciar solo funciona en la aplicación de escritorio (simulado)'; },
    async startupNotices() {
      return q.has('dbfail') ? [{ kind: 'error' as const, message: 'No se pudo abrir la base de datos (visor.db): archivo bloqueado. Esta sesión funciona en memoria y NO se guardará nada. Cierra el visor y revisa el archivo, o restaura una copia de seguridad.' }] : [];
    },
    async setMagpieMode(id, mode) {
      const t = lib.platforms.flatMap((p) => p.titles).find((x) => x.id === id);
      if (t) t.magpieMode = mode;
    },
    async remoteInfo() {
      const on = cfg.remoteEnabled;
      if (on && !cfg.remoteToken) cfg = { ...cfg, remoteToken: 'K7QM-4XTR' };
      return { enabled: on, running: on, port: cfg.remotePort, addresses: on ? ['192.168.1.20'] : [], code: cfg.remoteToken, error: null };
    },
    async remoteNewCode() {
      const abc = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';
      const code = Array.from({ length: 8 }, (_, i) => (i === 4 ? '-' : '') + abc[Math.floor(Math.random() * abc.length)]).join('');
      cfg = { ...cfg, remoteToken: code };
      try { store()?.setItem(LS, JSON.stringify(cfg)); } catch {}
      return { enabled: cfg.remoteEnabled, running: cfg.remoteEnabled, port: cfg.remotePort, addresses: cfg.remoteEnabled ? ['192.168.1.20'] : [], code, error: null };
    },
    async magpieStatus() { return { exe: 'C:\\Magpie\\Magpie.exe (simulado)', running: true, cunny: true }; },
    async getSagas() { return load<Saga[]>('visor.mock.sagas', []); },
    async saveSagas(list) {
      const clean = list.filter((s) => s.name.trim() && s.members.length);
      save('visor.mock.sagas', clean);
      return clean;
    },
    async pickImages() {
      pickN++;
      return [
        `C:\\Users\\Mock\\Pictures\\IMG_20260930_19451${pickN}.jpg`,
        `C:\\Users\\Mock\\Pictures\\Pantalla 2026-10-01 12-30-0${pickN}.png`,
        `C:\\Users\\Mock\\Pictures\\sin fecha ${pickN}.webp`,
      ];
    },
    async importShots(dir, paths, onProgress) {
      const hit = allTitles().find(({ t }) => t.dir === dir);
      if (!hit) throw 'Esa carpeta esta fuera de la biblioteca';
      const { p, t } = hit;
      const copied: string[] = [];
      for (let i = 0; i < paths.length; i++) {
        await new Promise((r) => setTimeout(r, 140));
        const nm = paths[i].split(/[\\/]/).pop()!;
        const d = nm.match(/(20\d{2})[-._]?(\d{2})[-._]?(\d{2})/);
        const date = d ? Date.UTC(+d[1], +d[2] - 1, +d[3], 12, 0, 0) : Date.now();
        const path = `mock://${p.id}/${t.name}/${seq++}`;
        t.shots.push({ path, mtime: date, date, width: 1920, height: 1080 });
        copied.push(path);
        onProgress(i + 1, paths.length);
      }
      t.shots.sort((a, b) => b.date - a.date);
      t.hasFolder = true;
      return { copied, skipped: [] };
    },
    async moveShots(paths, targetDir) {
      await new Promise((r) => setTimeout(r, 80));
      const dst = allTitles().find(({ t }) => t.dir === targetDir);
      if (!dst) throw 'Solo se pueden mover archivos dentro de la biblioteca';
      const moved: Array<[string, string]> = [];
      for (const old of paths) {
        const src = allTitles().find(({ t }) => t.shots.some((s) => s.path === old));
        if (!src || src.t === dst.t) continue;
        const sh = src.t.shots.find((s) => s.path === old)!;
        src.t.shots = src.t.shots.filter((s) => s.path !== old);
        const path = `mock://${dst.p.id}/${dst.t.name}/${seq++}`;
        alias.set(path, alias.get(old) ?? old);
        dst.t.shots.push({ ...sh, path });
        dst.t.hasFolder = true;
        if (favs.includes(old)) favs = favs.map((f) => (f === old ? path : f));
        moved.push([old, path]);
      }
      for (const { t } of allTitles()) t.shots.sort((a, b) => b.date - a.date);
      save('visor.mock.favs', favs);
      return { moved, failed: [] };
    },
    async trashShots(paths) {
      await new Promise((r) => setTimeout(r, 80));
      const dead = new Set(paths);
      for (const { t } of allTitles()) t.shots = t.shots.filter((s) => !dead.has(s.path));
      favs = favs.filter((f) => !dead.has(f));
      save('visor.mock.favs', favs);
      return { trashed: paths, failed: [] };
    },
    async reveal() {},
    onFilesDropped(cb) {
      (window as unknown as { __visorDrop?: (p: string[]) => void }).__visorDrop = cb;
    },
    async removeGame(id) {
      for (const p of lib.platforms) p.titles = p.titles.filter((t) => t.gameId !== id);
    },
    async moveGamePlatform(id, platform) {
      const to = lib.platforms.find((p) => p.id === platform);
      for (const p of lib.platforms) {
        const t = p.titles.find((x) => x.gameId === id);
        if (t && to && p !== to) { p.titles = p.titles.filter((x) => x !== t); to.titles.push(t); }
      }
    },
    async pickFile(kind) {
      if (kind === 'image') return 'C:\\Users\\Mock\\Pictures\\arte.png';
      return kind === 'exe' ? 'C:\\Juegos\\Mi_Juego\\mi_juego.exe' : 'D:\\ROMs\\Super Mario Odyssey.nsp';
    },
    onEvent(cb: (e: BackendEvent) => void) {
      emit = cb;
      // __visorEmit({type:'game-exited', id, name, seconds}) también actualiza las estadísticas simuladas.
      (window as unknown as { __visorEmit?: typeof cb }).__visorEmit = (e) => {
        if (e.type === 'game-exited') {
          for (const p of lib.platforms) for (const t of p.titles) {
            if (t.gameId !== e.id) continue;
            t.playSeconds += e.seconds; t.sessions += 1; t.lastPlayed = Date.now(); t.firstPlayed ??= Date.now();
            t.plays = [...t.plays, [Date.now() - e.seconds * 1000, Date.now()]];
            t.week = [...t.week.slice(0, 6), (t.week[6] ?? 0) + Math.round(e.seconds / 60)];
          }
        }
        cb(e);
      };
    },
  };
}
