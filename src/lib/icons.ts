// Glifos SVG portados del mockup aprobado.
export const RAINBOW = [
  { n: 'Rojo', c: '#E8384F', dark: false },
  { n: 'Naranja', c: '#F27A1A', dark: false },
  { n: 'Amarillo', c: '#F2B807', dark: true },
  { n: 'Verde', c: '#3BB54A', dark: false },
  { n: 'Azul Wii U', c: '#00A0D8', dark: false },
  { n: 'Añil', c: '#3F63E0', dark: false },
  { n: 'Violeta', c: '#9A4BD8', dark: false },
] as const;
export type Accent = { n: string; c: string; dark: boolean; v?: string };

/** Icono de trazo (§9 de la guía): rejilla 24, trazo 2, extremos redondos, color del texto (currentColor). */
export const ic = (d: string, attrs = '') =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"${attrs}>${d}</svg>`;

export const ICON: Record<string, string> = {
  pc:`<svg viewBox="0 0 120 120"><defs><linearGradient id="gw" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#e9eef5"/></linearGradient></defs><path fill="url(#gw)" fill-rule="evenodd" d="M20 18h66a8 8 0 0 1 8 8v42a8 8 0 0 1-8 8H20a8 8 0 0 1-8-8V26a8 8 0 0 1 8-8zm2 9v38h62V27z"/><path fill="url(#gw)" d="M46 76h14l3 12H43z"/><rect x="32" y="87" width="42" height="7" rx="3.5" fill="url(#gw)"/><path fill="url(#gw)" fill-rule="evenodd" d="M100 26h8a5 5 0 0 1 5 5v58a5 5 0 0 1-5 5h-8a5 5 0 0 1-5-5V31a5 5 0 0 1 5-5zm1 10v4h6v-4zm0 9v4h6v-4z"/><path d="M24 29h28l-28 22z" fill="#fff" opacity=".35"/></svg>`,
  switch:`<svg viewBox="0 0 120 120" fill="none" stroke="#fff" stroke-width="7" stroke-linejoin="round"><rect x="36" y="30" width="48" height="60" rx="5"/><path d="M36 30h-8a14 14 0 0 0-14 14v32a14 14 0 0 0 14 14h8z" fill="#fff"/><path d="M84 30h8a14 14 0 0 1 14 14v32a14 14 0 0 1-14 14h-8z"/><circle cx="25" cy="48" r="5" fill="#8b9" stroke="none" style="fill:var(--pc)"/><circle cx="95" cy="68" r="5" fill="#fff" stroke="none"/></svg>`,
  saga:`<svg viewBox="0 0 120 120" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"><rect x="14" y="30" width="42" height="64" rx="8" transform="rotate(-12 35 62)"/><rect x="64" y="30" width="42" height="64" rx="8" transform="rotate(12 85 62)"/><rect x="39" y="22" width="42" height="68" rx="8" fill="#fff" fill-opacity=".25"/></svg>`,
  vn:`<svg viewBox="0 0 120 120" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"><circle cx="60" cy="30" r="14" fill="#fff" stroke="none"/><path d="M34 62c4-12 14-17 26-17s22 5 26 17" fill="#fff" stroke="none"/><rect x="12" y="64" width="96" height="42" rx="10"/><rect x="20" y="56" width="34" height="14" rx="5" fill="#fff" stroke="none"/><path d="M26 84h50M26 96h32"/><path d="M92 92l6 6 6-6" stroke-width="5"/></svg>`,
  jvn:`<svg viewBox="0 0 120 120" fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"><path d="M60 34c-12-8-28-10-44-8v62c16-2 32 0 44 8 12-8 28-10 44-8V26c-16-2-32 0-44 8z"/><path d="M60 34v62"/><text x="38" y="74" text-anchor="middle" font-size="30" font-weight="800" fill="#fff" stroke="none" font-family="Yu Gothic UI,Meiryo,sans-serif">あ</text><path d="M72 52h20M72 64h20M72 76h14" stroke-width="5"/></svg>`,
  person:`${ic('<circle cx="12" cy="8" r="4"/><path d="M4.5 20.5c1-4.5 3.8-6.5 7.5-6.5s6.5 2 7.5 6.5"/>', ' width="34" height="34"')}`,
  search:`${ic('<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>')}`,
  star:`${ic('<path d="M12 2.5l2.9 6.1 6.6.8-4.9 4.55 1.3 6.55L12 17.25 6.1 20.5l1.3-6.55L2.5 9.4l6.6-.8z"/>')}`,
  import:`${ic('<path d="M12 3v10.5M7.5 9l4.5 4.5L16.5 9"/><path d="M3.5 15v4a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-4"/>')}`,
  gear:`${ic('<path d="M10 2h4l.6 2.8a7.5 7.5 0 0 1 2 1.15l2.7-.9 2 3.45-2.1 1.9a7.5 7.5 0 0 1 0 2.3l2.1 1.9-2 3.45-2.7-.9a7.5 7.5 0 0 1-2 1.15L14 22h-4l-.6-2.8a7.5 7.5 0 0 1-2-1.15l-2.7.9-2-3.45 2.1-1.9a7.5 7.5 0 0 1 0-2.3L2.7 9.4l2-3.45 2.7.9a7.5 7.5 0 0 1 2-1.15z"/><circle cx="12" cy="12" r="3.5"/>')}`,
  trash:`${ic('<path d="M4 6.5h16"/><path d="M9 6.5v-3h6v3"/><path d="M5.5 6.5l1 14h11l1-14"/><path d="M10 10v7M14 10v7"/>')}`,
  power:`${ic('<path d="M12 2.5v9"/><path d="M7 5.5a8 8 0 1 0 10 0"/>')}`,
  starY:`<svg viewBox="0 0 24 24"><path d="M12 2.5l2.9 6.1 6.6.8-4.9 4.55 1.3 6.55L12 17.25 6.1 20.5l1.3-6.55L2.5 9.4l6.6-.8z" fill="#F2B807" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/></svg>`,
  arrowR:`<svg viewBox="0 0 56 110"><path d="M8 6 L52 55 L8 104 Q2 108 2 100 V10 Q2 2 8 6z" fill="rgba(255,255,255,.96)" stroke="rgba(150,160,180,.6)" stroke-width="2"/></svg>`,
  arrowL:`<svg viewBox="0 0 56 110"><path d="M48 6 L4 55 L48 104 Q54 108 54 100 V10 Q54 2 48 6z" fill="rgba(255,255,255,.96)" stroke="rgba(150,160,180,.6)" stroke-width="2"/></svg>`,
  play:`${ic('<path d="M8 5.5v13a1 1 0 0 0 1.5.86l10.5-6.5a1 1 0 0 0 0-1.72L9.5 4.64A1 1 0 0 0 8 5.5z" fill="currentColor"/>')}`,
  reload:`${ic('<path d="M20 12a8 8 0 1 1-2.5-5.75"/><path d="M19 2.5V7h-4.5"/>')}`,
  folder:`${ic('<path d="M2.5 7A1.5 1.5 0 0 1 4 5.5h5l2.5 3H20A1.5 1.5 0 0 1 21.5 10v8.5A1.5 1.5 0 0 1 20 20H4a1.5 1.5 0 0 1-1.5-1.5z"/>')}`,
  plus:`${ic('<path d="M12 4.5v15M4.5 12h15"/>')}`,
  link:`${ic('<path d="M10 14a4 4 0 0 0 5.5 0l3.5-3.5a4 4 0 0 0-5.5-5.5L12 6.5M14 10a4 4 0 0 0-5.5 0L5 13.5a4 4 0 0 0 5.5 5.5l1.5-1.5"/>')}`,
  image:`${ic('<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><circle cx="8.5" cy="10" r="1.5"/><path d="M4 18l5-4.5 3.5 3 3-2.5 5.5 4.5"/>')}`,
  eye:`${ic('<path d="M2 12s4-6.5 10-6.5S22 12 22 12s-4 6.5-10 6.5S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>')}`,
  eyeOff:`${ic('<path d="M2 12s4-6.5 10-6.5S22 12 22 12s-4 6.5-10 6.5S2 12 2 12z"/><path d="M4.5 4l15 16"/>')}`,
  palette:`${ic('<path d="M12 2.5a9.5 9.5 0 1 0 0 19c1.5 0 2-1 1.5-2s0-2.5 2-2.5H18c2 0 3.5-1.5 3.5-3.5C21.5 7 17 2.5 12 2.5z"/><circle cx="7.5" cy="11" r="1"/><circle cx="11" cy="7" r="1"/><circle cx="16" cy="7.5" r="1"/>')}`,
  move:`${ic('<path d="M3.5 12h15M13 6.5l5.5 5.5-5.5 5.5"/>')}`,
  check:`${ic('<path d="M5 12.5l4.5 4.5 9.5-10"/>')}`,
  calendar:`${ic('<rect x="3" y="4.5" width="18" height="16.5" rx="2.5"/><path d="M3 9.5h18M8 2.5v4M16 2.5v4"/><path d="M7.5 13.5h2M11 13.5h2M14.5 13.5h2M7.5 17h2M11 17h2"/>')}`,
  checks:`${ic('<path d="M2.5 12.5l4 4 7-8.5M12 16.5l2 2 7.5-9"/>')}`,
  pad:`${ic('<path d="M7 6h10a4.5 4.5 0 0 1 4.5 4.5V14a3.5 3.5 0 0 1-6 2.5L14 15h-4l-1.5 1.5a3.5 3.5 0 0 1-6-2.5v-3.5A4.5 4.5 0 0 1 7 6z"/><path d="M7.5 10v3M6 11.5h3"/>')}`,
  mouse:`${ic('<rect x="6" y="2.5" width="12" height="19" rx="6"/><path d="M12 6v4"/>')}`,
  key:`${ic('<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6 10h1M10 10h1M14 10h1M18 10h0M7.5 14h9"/>')}`,
  camera:`${ic('<path d="M3 7.5h4l1.5-2.5h7L17 7.5h4v12H3z"/><circle cx="12" cy="13" r="3.5"/>')}`,
  chevL:`${ic('<path d="M15 4l-8 8 8 8"/>')}`,
  phone:`${ic('<rect x="6.5" y="2" width="11" height="20" rx="2.5"/><path d="M10.5 18.5h3"/>')}`,
  chart:`${ic('<rect x="3.5" y="13" width="4" height="7.5" rx="1"/><rect x="10" y="8" width="4" height="12.5" rx="1"/><rect x="16.5" y="3.5" width="4" height="17" rx="1"/>')}`,
  chevR:`${ic('<path d="M9 4l8 8-8 8"/>')}`,
};

/* Iconos de salida de audio (altavoz, auriculares, monitor) para la barra superior y Ajustes. */
export type AudioKind = 'speaker' | 'headphones' | 'monitor';
const AUDIO_PATHS: Record<AudioKind, string> = {
  speaker: '<path d="M4 9.5h3.5L12 6v12l-4.5-3.5H4z"/><path d="M15.5 8.5a4.5 4.5 0 0 1 0 7M18 6a8 8 0 0 1 0 12"/>',
  headphones: '<path d="M4 15v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="14" width="4.5" height="6.5" rx="1.5"/><rect x="16.5" y="14" width="4.5" height="6.5" rx="1.5"/>',
  monitor: '<rect x="2.5" y="4.5" width="19" height="12.5" rx="2"/><path d="M8.5 20.5h7M12 17v3.5"/>',
};
export const audioSvg = (kind: AudioKind) => ic(AUDIO_PATHS[kind]);
/** Adivina el tipo de dispositivo por su nombre de Windows. */
export const audioKind = (name: string): AudioKind =>
  /auricular|headphone|headset|cascos|airpods|buds/i.test(name) ? 'headphones'
    : /hdmi|displayport|nvidia|amd high|monitor|ultragear|tv|display/i.test(name) ? 'monitor' : 'speaker';
ICON.speaker = audioSvg('speaker');
