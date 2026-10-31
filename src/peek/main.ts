// Miniatura de la captura recien hecha (ver src-tauri/src/peek.rs). La app la cambia con `window.__peek(dataUrl)`.
// Animacion «flash de obturador»: dos laminas negras se abren con un destello blanco que quema la foto, que vuelve a
// su color; luego entra el check. Dura lo mismo que la ventana visible (SHOW_MS en peek.rs).
export {};
const TOTAL = 1700;
/** Si llega otra captura con la anterior aun a la vista, se salta el fundido inicial para no parpadear. */
const REENTRY_SKIP = 60;
const f = document.getElementById('f')!;
const img = document.getElementById('img') as HTMLImageElement;
const q = (s: string) => f.querySelector(s) as HTMLElement;

const OUT = 'cubic-bezier(.16,1,.3,1)';
const IN = 'cubic-bezier(.55,0,1,.45)';
const BACK = 'cubic-bezier(.34,1.56,.64,1)';
const SHUTTER = 'cubic-bezier(.7,0,.2,1)';

/** Linea de tiempo en ms absolutos: cada elemento recorre los TOTAL ms con los fotogramas situados por su `ms`. */
function track(el: Element, frames: [number, Keyframe][]): Animation {
  const kf = frames.map(([ms, p]) => ({ offset: Math.min(1, Math.max(0, ms / TOTAL)), ...p }));
  if (kf[0].offset !== 0) kf.unshift({ ...kf[0], offset: 0, easing: 'linear' });
  if (kf[kf.length - 1].offset !== 1) kf.push({ ...kf[kf.length - 1], offset: 1, easing: 'linear' });
  return el.animate(kf, { duration: TOTAL, fill: 'both' });
}

let running: Animation[] = [];
let startedAt = -Infinity;

function play() {
  const reentry = performance.now() - startedAt < TOTAL;
  running.forEach((a) => a.cancel());
  const SC = 'scale(0)', S1 = 'scale(1)';
  running = [
    track(f, [[0, { opacity: 0, transform: 'scale(1.06)' }], [50, { opacity: 1, transform: 'scale(1.06)', easing: OUT }], [340, { opacity: 1, transform: S1 }], [1380, { opacity: 1, transform: S1, easing: IN }], [1700, { opacity: 0, transform: 'scale(.96)' }]]),
    track(q('.blade.t'), [[0, { transform: 'translateY(0%)' }], [70, { transform: 'translateY(0%)', easing: SHUTTER }], [280, { transform: 'translateY(-101%)' }]]),
    track(q('.blade.b'), [[0, { transform: 'translateY(0%)' }], [70, { transform: 'translateY(0%)', easing: SHUTTER }], [280, { transform: 'translateY(101%)' }]]),
    track(q('.flash'), [[0, { opacity: 0 }], [70, { opacity: 0.95, easing: 'ease-out' }], [420, { opacity: 0 }]]),
    track(img, [[0, { filter: 'brightness(1.9) contrast(.85)' }], [90, { filter: 'brightness(1.9) contrast(.85)', easing: OUT }], [520, { filter: 'brightness(1) contrast(1)' }]]),
    track(q('i'), [[0, { transform: SC, opacity: 0 }], [380, { transform: SC, opacity: 0, easing: BACK }], [560, { transform: S1, opacity: 1 }]]),
  ];
  if (reentry) running.forEach((a) => (a.currentTime = REENTRY_SKIP));
  startedAt = performance.now() - (reentry ? REENTRY_SKIP : 0);
}

function set(src: string) {
  img.src = src;
  play();
}

(window as unknown as { __peek: typeof set }).__peek = set;
const first = location.hash.slice(1);
if (first) set(first);
