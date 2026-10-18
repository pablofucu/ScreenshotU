// Utilidades de la pagina de lectura.
import type { WordInfo } from '../lib/novel-types';
import { demoShot } from './demo';

export const demo = new URLSearchParams(location.search).has('demo');

export const POS_NAME: Record<string, string> = { n: 'Sustantivo', v: 'Verbo', a: 'Adjetivo', adv: 'Adverbio', i: 'Interjección' };

/** Etiqueta de frecuencia a partir del puesto (JPDB): común / media / rara. */
export function freqLabel(rank: number): [string, string] {
  return rank < 1500 ? ['común', ''] : rank < 10000 ? ['media', 'm'] : ['rara', 'r'];
}

export const showReading = (term: string, reading: string) => !!reading && reading !== term;

/** Primeras glosas de cada acepcion de la primera entrada, en una linea. */
export function oneLine(w: WordInfo): string {
  const d = w.defs[0];
  return d ? d.senses.map((s) => s.glosses[0]).filter(Boolean).slice(0, 4).join(' · ') : '';
}

/** URL de la captura de una linea (la app la sirve desde memoria o disco). */
export const shotUrl = (lineId: number) => (demo ? demoShot(lineId) : `/shot/${lineId}`);

/** URL del audio de una linea: `rec` = grabado del juego, `synth` = voz sintetica. */
export const audioUrl = (lineId: number, kind: 'rec' | 'synth', v = 0) => (demo ? '' : `/audio/${lineId}/${kind}?v=${v}`);

/** Duracion corta: «45 s», «12 min», «3 h 05 min». */
export function fmtDur(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, '0')} min`;
}

/** Fecha corta de un dia local (dias desde 1970-01-01). */
export const dayLabel = (day: number) => new Date(day * 864e5).toLocaleDateString('es-ES', { day: 'numeric', month: 'short', timeZone: 'UTC' });

export const fmtNum = (n: number) => n.toLocaleString('es-ES');

export const mmss = (secs: number) => `${Math.floor(secs / 60)}:${String(Math.floor(secs % 60)).padStart(2, '0')}`;

export const ICON = {
  check: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>',
  anki: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="3.5" width="16" height="17" rx="3"/><path d="M12 8v8M8 12h8"/></svg>',
  camera: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><rect x="3" y="6" width="18" height="13" rx="3"/><circle cx="12" cy="12.5" r="3.2"/><path d="M8 6l1.5-2h5L16 6"/></svg>',
  help: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.3M12 17h.01"/></svg>',
  star: '<svg viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"><path d="M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.9L12 16.9 6.8 19.7l1-5.9L3.5 9.7l5.9-.8z"/></svg>',
  starO: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linejoin="round"><path d="M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.9L12 16.9 6.8 19.7l1-5.9L3.5 9.7l5.9-.8z"/></svg>',
  mic: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21"/></svg>',
  gear: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1"/></svg>',
  chart: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16M7 17v-5M12 17V6M17 17v-8"/></svg>',
  book: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v15H6.5A2.5 2.5 0 0 0 4 20.5zM4 20.5A2.5 2.5 0 0 0 6.5 23H20"/></svg>',
};
