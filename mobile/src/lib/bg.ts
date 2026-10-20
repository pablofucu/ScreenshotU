import { rng } from '@shared/rng';
import { RAINBOW } from '@shared/icons';

/** Fondo del visor (semilla 42) adaptado a vertical: posiciones en % del viewport. */
export function buildBackground(): string {
  const r = rng(42);
  let h = '';
  for (let i = 0; i < 7; i++) {
    const s = 160 + r() * 220;
    h += `<div class="blob ${i % 3 === 0 ? 'a' : 'w'}" style="left:calc(${(r() * 110 - 15).toFixed(1)}% - ${s / 2}px);top:calc(${(r() * 100).toFixed(1)}% - ${s / 2}px);width:${s}px;height:${s}px;animation-delay:-${(r() * 20).toFixed(1)}s"></div>`;
  }
  for (let i = 0; i < 8; i++) {
    const sc = 0.6 + r() * 0.7;
    h += `<div class="ghost" style="left:${(r() * 100 - 10).toFixed(1)}%;top:${(r() * 100 - 5).toFixed(1)}%;opacity:${(0.16 + r() * 0.2).toFixed(2)};transform:scale(${sc.toFixed(2)}) rotate(${(r() * 16 - 8).toFixed(1)}deg);--g:${RAINBOW[i % 7].c}"><i></i></div>`;
  }
  for (let i = 0; i < 12; i++) {
    const s = 8 + r() * 28;
    h += `<div class="bokeh" style="left:${(r() * 100).toFixed(1)}%;top:${(r() * 100).toFixed(1)}%;width:${s}px;height:${s}px;background:radial-gradient(circle at 35% 35%,rgba(255,255,255,.95),rgba(255,255,255,.25) 60%,rgba(255,255,255,0) 72%);opacity:${(0.5 + r() * 0.5).toFixed(2)}"></div>`;
  }
  return h;
}
