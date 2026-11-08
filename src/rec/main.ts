// Indicador de grabacion sobre el juego (ver src-tauri/src/overlay.rs). La app lo actualiza con `window.__rec(estado, segundos, texto)`.
// En el estado `goal` (meta diaria cumplida) el texto llega como `clave|dias|titular` y sale una tarjeta con la llama.
import { GOAL_COL, GOAL_TXT, groupOf, type GoalKey } from '../lib/readgoals';

export {};
const p = document.getElementById('p')!;
const t = document.getElementById('t')!;
const g = document.getElementById('g')!;
const mmss = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;

const TOTAL = 4500;
const OUT = 'cubic-bezier(.16,1,.3,1)';
const IN = 'cubic-bezier(.55,0,1,.45)';
const BACK = 'cubic-bezier(.34,1.56,.64,1)';
const ROLL = 'cubic-bezier(.5,-.4,.3,1.4)';
const SHEEN = 'cubic-bezier(.4,0,.2,1)';

/** Segunda linea de la tarjeta segun el titular. */
function subtitle(title: string, days: number, key: GoalKey): string {
  if (title.includes('recuperada')) return 'Recuperaste el día que faltaba';
  return days > 1 ? 'Hoy ya cuenta para tu racha' : GOAL_TXT[groupOf(key)].sub1;
}

function track(el: Element, frames: [number, Keyframe][]): Animation {
  const kf = frames.map(([ms, f]) => ({ offset: Math.min(1, Math.max(0, ms / TOTAL)), ...f }));
  if (kf[0].offset !== 0) kf.unshift({ ...kf[0], offset: 0, easing: 'linear' });
  if (kf[kf.length - 1].offset !== 1) kf.push({ ...kf[kf.length - 1], offset: 1, easing: 'linear' });
  return el.animate(kf, { duration: TOTAL, fill: 'both' });
}

let running: Animation[] = [];

function playGoal(raw: string) {
  const [key, d, ...rest] = raw.split('|');
  const days = Math.max(1, Number(d) || 1);
  const title = rest.join('|');
  const c = GOAL_COL[key as keyof typeof GOAL_COL] ?? GOAL_COL.vn;
  g.style.setProperty('--a', c.edge);
  g.style.setProperty('--b', c.c);
  document.getElementById('gt')!.textContent = title;
  document.getElementById('gs')!.textContent = subtitle(title, days, (key in GOAL_COL ? key : 'vn') as GoalKey);
  document.getElementById('gc')!.innerHTML = `<span>${days - 1}</span><span>${days}</span>`;
  document.getElementById('gd')!.textContent = days === 1 ? 'día' : 'días';

  running.forEach((a) => a.cancel());
  const q = (s: string) => g.querySelector(s)!;
  const T0 = 'translateY(6px)', T1 = 'translateY(0px)';
  running = [
    track(g, [[0, { opacity: 0, transform: 'scale(.92)', easing: OUT }], [420, { opacity: 1, transform: 'scale(1)' }], [4100, { opacity: 1, transform: 'scale(1)', easing: IN }], [4500, { opacity: 0, transform: 'scale(.95)' }]]),
    track(q('.badge'), [[0, { transform: 'scale(0) rotate(-40deg)' }], [180, { transform: 'scale(0) rotate(-40deg)', easing: BACK }], [560, { transform: 'scale(1) rotate(0deg)' }]]),
    q('.flame').animate(
      [{ transform: 'scale(1,1)' }, { transform: 'scale(1.07,.93)' }, { transform: 'scale(.95,1.07)' }, { transform: 'scale(1,1)' }],
      { duration: 560, delay: 560, iterations: 6, easing: 'ease-in-out' },
    ),
    ...[...g.querySelectorAll('.ember')].map((e, i) =>
      track(e, [[0, { opacity: 0, transform: 'translate(0px,0px) scale(1)' }], [600 + i * 240, { opacity: 0, transform: 'translate(0px,0px) scale(1)', easing: 'ease-out' }], [700 + i * 240, { opacity: 1, transform: `translate(${(i - 1) * 3}px,-4px) scale(1)`, easing: 'ease-out' }], [1500 + i * 240, { opacity: 0, transform: `translate(${(i - 1) * 6}px,-16px) scale(.4)` }]]),
    ),
    track(q('.txt b'), [[0, { opacity: 0, transform: T0 }], [280, { opacity: 0, transform: T0, easing: OUT }], [620, { opacity: 1, transform: T1 }]]),
    track(q('.txt span'), [[0, { opacity: 0, transform: T0 }], [380, { opacity: 0, transform: T0, easing: OUT }], [720, { opacity: 0.85, transform: T1 }]]),
    track(q('.col'), [[0, { transform: 'translateY(0%)' }], [800, { transform: 'translateY(0%)', easing: ROLL }], [1300, { transform: 'translateY(-50%)' }]]),
    track(q('.num'), [[0, { opacity: 0, transform: 'scale(1)' }], [300, { opacity: 0, transform: 'scale(1)', easing: 'ease-out' }], [600, { opacity: 1, transform: 'scale(1)' }], [1300, { opacity: 1, transform: 'scale(1)', easing: 'ease-out' }], [1420, { opacity: 1, transform: 'scale(1.12)', easing: OUT }], [1620, { opacity: 1, transform: 'scale(1)' }]]),
    track(q('.sheen'), [[0, { transform: 'translateX(-110%)' }], [1350, { transform: 'translateX(-110%)', easing: SHEEN }], [2000, { transform: 'translateX(110%)' }]]),
  ];
}

function set(state: string, secs: number, text = '') {
  if (state === 'goal') {
    p.className = 'pill goal';
    g.classList.add('on');
    playGoal(text);
    return;
  }
  running.forEach((a) => a.cancel());
  running = [];
  g.classList.remove('on');
  p.className = `pill ${state}`;
  t.textContent = state === 'recording' ? `REC ${mmss(secs)}` : state === 'done' ? `✓ ${secs} s` : state === 'audio' ? `Audio: ${text}` : 'Sin audio';
}

(window as unknown as { __rec: typeof set }).__rec = set;
const q = new URLSearchParams(location.search);
set(q.get('s') ?? 'recording', Number(q.get('t') ?? 0), q.get('x') ?? '');
