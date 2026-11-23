import { rng } from './rng';

/** Fondo del mockup: manchas, iconos fantasma y bokeh en capas con semilla fija (42). */
export function buildBackground(): { blobs: string; ghosts: string; bokeh: string } {
  const r = rng(42);
  let blobs = '';
  let ghosts = '';
  let bokeh = '';
  for (let i = 0; i < 9; i++) {
    const s = 180 + r() * 260;
    const b = (i % 3) + 1;
    blobs += `<div class="blob" style="left:${r() * 1180 - 100}px;top:${r() * 640 - 100}px;width:${s}px;height:${s}px;--b:var(--b${b});animation-delay:-${(r() * 20).toFixed(1)}s"></div>`;
  }
  for (let i = 0; i < 9; i++) {
    const sc = 0.6 + r() * 0.8;
    const g = i % 2 === 0 ? 'var(--b1)' : 'var(--b2)';
    ghosts += `<div class="ghost" style="left:${r() * 1200 - 40}px;top:${r() * 660 - 40}px;opacity:${(0.18 + r() * 0.22).toFixed(2)};transform:scale(${sc.toFixed(2)}) rotate(${(r() * 16 - 8).toFixed(1)}deg);--g:${g}"><i></i></div>`;
  }
  for (let i = 0; i < 14; i++) {
    const s = 10 + r() * 34;
    bokeh += `<div class="bokeh" style="left:${r() * 1280}px;top:${r() * 720}px;width:${s}px;height:${s}px;background:radial-gradient(circle at 35% 35%,rgba(255,255,255,.95),rgba(255,255,255,.25) 60%,rgba(255,255,255,0) 72%);opacity:${(0.5 + r() * 0.5).toFixed(2)}"></div>`;
  }
  return { blobs, ghosts, bokeh };
}
