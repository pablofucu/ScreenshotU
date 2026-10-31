// Aviso de logro conseguido sobre el juego (ver src-tauri/src/achpop.rs). La app lo cambia con `window.__ach(datos)`;
// la primera vez los datos van en el fragmento de la URL (JSON en base64).
// Medalla que gira: el icono pasa de gris a color como una moneda mientras se dibuja el anillo de la rareza.
export {};
interface Data {
  name: string;
  desc: string;
  icon: string | null;
  /** 0 Común, 1 Poco común, 2 Raro, 3 Ultra raro, 4 Legendario. */
  tier: number;
  /** % de jugadores que lo tienen; null si no hay datos de rareza. */
  pct: number | null;
}

const TOTAL = 5000;
const OUT = 'cubic-bezier(.16,1,.3,1)';
const IN = 'cubic-bezier(.55,0,1,.45)';
const SHEEN = 'cubic-bezier(.4,0,.2,1)';
const STAR = '<svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true"><path d="M5 0 6 4 10 5 6 6 5 10 4 6 0 5 4 4Z" fill="currentColor"/></svg>';
/** Colores de rareza (los mismos que la tarjeta de logros de la app) y destellos que sueltan. */
const RAR = [
  { label: 'Común', color: '#9fc0dc', spk: 0 },
  { label: 'Poco común', color: '#4fc3a1', spk: 2 },
  { label: 'Raro', color: '#b67cf0', spk: 3 },
  { label: 'Ultra raro', color: '#f2b807', spk: 6 },
  { label: 'Legendario', color: '#ff7a45', spk: 8 },
];
const FALLBACK = 'data:image/svg+xml;utf8,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" fill="#4a5568"/><text x="32" y="43" font-size="30" text-anchor="middle">🏆</text></svg>');

const $ = (id: string) => document.getElementById(id)!;
const card = $('c');
const pctFmt = (p: number) => p.toLocaleString('es-ES', { maximumFractionDigits: 1 }) + ' %';

function track(el: Element, frames: [number, Keyframe][]): Animation {
  const kf = frames.map(([ms, f]) => ({ offset: Math.min(1, Math.max(0, ms / TOTAL)), ...f }));
  if (kf[0].offset !== 0) kf.unshift({ ...kf[0], offset: 0, easing: 'linear' });
  if (kf[kf.length - 1].offset !== 1) kf.push({ ...kf[kf.length - 1], offset: 1, easing: 'linear' });
  return el.animate(kf, { duration: TOTAL, fill: 'both' });
}

let running: Animation[] = [];

function set(d: Data) {
  const r = RAR[Math.min(Math.max(d.tier, 0), RAR.length - 1)];
  card.style.setProperty('--r', r.color);
  $('nm').textContent = d.name;
  $('ds').textContent = d.desc;
  const icon = d.icon || FALLBACK;
  ($('i0') as HTMLImageElement).src = icon;
  ($('i1') as HTMLImageElement).src = icon;

  const hasPct = d.pct != null;
  const meta = $('meta');
  meta.innerHTML = '';
  if (hasPct) {
    const fill = Math.max(0.06, Math.min(1, (d.pct as number) / 100));
    meta.innerHTML = `<span class="meter"><i style="width:${(fill * 100).toFixed(0)}%"></i></span>${r.label} · ${pctFmt(d.pct as number)} de jugadores`;
  }

  // Destellos alrededor de la medalla, solo en rarezas altas.
  const sp = $('sp');
  sp.innerHTML = '';
  const R = 35, cx = 47, cy = 46;
  for (let i = 0; i < r.spk; i++) {
    const a = ((-120 + i * (360 / r.spk)) * Math.PI) / 180;
    const s = document.createElement('span');
    s.className = 'spk';
    s.style.cssText = `left:${(cx + Math.cos(a) * R - 5).toFixed(1)}px;top:${(cy + Math.sin(a) * R - 5).toFixed(1)}px;--r:${r.color}`;
    s.innerHTML = STAR;
    sp.appendChild(s);
  }

  running.forEach((a) => a.cancel());
  const q = (s: string) => card.querySelector(s)!;
  // La ventana va abajo a la derecha: la tarjeta sube desde el borde de la pantalla y se va igual.
  const st = (el: Element, t0: number) => track(el, [[0, { opacity: 0, transform: 'translateY(5px)' }], [t0, { opacity: 0, transform: 'translateY(5px)', easing: OUT }], [t0 + 320, { opacity: 1, transform: 'translateY(0px)' }]]);
  running = [
    track(card, [[0, { opacity: 0, transform: 'translateY(8px)', easing: OUT }], [380, { opacity: 1, transform: 'translateY(0px)' }], [4600, { opacity: 1, transform: 'translateY(0px)', easing: IN }], [5000, { opacity: 0, transform: 'translateY(8px)' }]]),
    track(q('.coin'), [[0, { transform: 'rotateY(0deg) scale(.8)' }], [350, { transform: 'rotateY(0deg) scale(.8)', easing: 'cubic-bezier(.3,1.3,.5,1)' }], [1050, { transform: 'rotateY(180deg) scale(1)' }]]),
    track(q('.rg .a'), [[0, { strokeDashoffset: 100 }], [300, { strokeDashoffset: 100, easing: 'cubic-bezier(.45,0,.2,1)' }], [1100, { strokeDashoffset: 0 }]]),
    st(q('.k'), 400), st(q('.nm'), 480), st(q('.ds'), 560), st(q('.meta'), 640),
    track(q('.sheen'), [[0, { transform: 'translateX(-110%)' }], [1150, { transform: 'translateX(-110%)', easing: SHEEN }], [1750, { transform: 'translateX(110%)' }]]),
  ];
  const meter = card.querySelector('.meter i');
  if (meter) running.push(track(meter, [[0, { transform: 'scaleX(0)' }], [900, { transform: 'scaleX(0)', easing: OUT }], [1500, { transform: 'scaleX(1)' }]]));
  sp.querySelectorAll('.spk').forEach((s, i) => {
    const f: [number, Keyframe][] = [[0, { opacity: 0, transform: 'scale(0) rotate(0deg)' }]];
    for (const w0 of d.tier >= 3 ? [1100, 2500] : [1100]) {
      const s0 = w0 + i * 110;
      f.push([s0, { opacity: 0, transform: 'scale(0) rotate(0deg)', easing: OUT }], [s0 + 220, { opacity: 1, transform: 'scale(1.15) rotate(45deg)', easing: 'ease-in' }], [s0 + 760, { opacity: 0, transform: 'scale(.2) rotate(90deg)' }]);
    }
    f.push([4600, { opacity: 0, transform: 'scale(.2) rotate(90deg)' }]);
    running.push(track(s, f));
  });
}

(window as unknown as { __ach: typeof set }).__ach = set;
const first = location.hash.slice(1);
if (first) {
  try {
    const bytes = Uint8Array.from(atob(decodeURIComponent(first)), (c) => c.charCodeAt(0));
    set(JSON.parse(new TextDecoder().decode(bytes)));
  } catch (e) {
    console.error(e);
  }
}
