import { rng } from './rng';
import { RAINBOW } from './icons';

/** Fondo del mockup: manchas, iconos fantasma y bokeh con semilla fija (42). */
export function buildBackground(): string {
  const r = rng(42);
  let h = '';
  for (let i = 0; i < 9; i++) {
    const s = 180 + r() * 260;
    h += `<div class="blob ${i % 3 === 0 ? 'a' : 'w'}" style="left:${r() * 1180 - 100}px;top:${r() * 640 - 100}px;width:${s}px;height:${s}px;animation-delay:-${(r() * 20).toFixed(1)}s"></div>`;
  }
  for (let i = 0; i < 9; i++) {
    const sc = 0.6 + r() * 0.8;
    h += `<div class="ghost" style="left:${r() * 1200 - 40}px;top:${r() * 660 - 40}px;opacity:${(0.18 + r() * 0.22).toFixed(2)};transform:scale(${sc.toFixed(2)}) rotate(${(r() * 16 - 8).toFixed(1)}deg);--g:${RAINBOW[i % 7].c}"><i></i></div>`;
  }
  for (let i = 0; i < 14; i++) {
    const s = 10 + r() * 34;
    h += `<div class="bokeh" style="left:${r() * 1280}px;top:${r() * 720}px;width:${s}px;height:${s}px;background:radial-gradient(circle at 35% 35%,rgba(255,255,255,.95),rgba(255,255,255,.25) 60%,rgba(255,255,255,0) 72%);opacity:${(0.5 + r() * 0.5).toFixed(2)}"></div>`;
  }
  return h;
}
