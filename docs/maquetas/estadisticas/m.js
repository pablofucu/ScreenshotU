// Datos de ejemplo y piezas compartidas por las tres propuestas (solo maqueta, nada de esto es código de la app).
const G = {
  'Celeste': ['#e85a9b', '#5b3fb0'], 'Stardew Valley': ['#6fbf4a', '#2a7a5a'], 'Balatro': ['#e0483a', '#2d4fb0'],
  'The Witcher 3': ['#b03a2e', '#3a2a2a'], 'Red Dead Redemption 2': ['#c92a2a', '#3b1d14'], 'Hi-Fi Rush': ['#f2b807', '#e0384a'],
  'Hades II': ['#7a3fd0', '#e0483a'], 'Metroid Dread': ['#e0384a', '#1b3a6b'], 'Animal Crossing': ['#5cc9a7', '#3b8fd0'],
  'Kirby y la tierra olvidada': ['#ff8fb8', '#5aa9e6'], 'Super Mario Odyssey': ['#e0384a', '#f2b807'], 'Zelda: Tears of the Kingdom': ['#5fbf8f', '#c9a227'],
  'Steins;Gate': ['#c9b27a', '#4a5a3a'], 'Fata Morgana': ['#8a2a3a', '#2a1a2a'], 'Summer Pockets': ['#5ab0e6', '#f2d06b'],
  'Kino no Tabi': ['#6E56CF', '#2a3a6b'], 'Kimi no Na wa': ['#3b6fd0', '#e58a48'], 'Kokoro': ['#E5487A', '#6E56CF'],
};
const CATS = [
  { id: 'pc', name: 'Juegos', c: 'var(--pc)' }, { id: 'sw', name: 'Switch', c: 'var(--sw)' },
  { id: 'vn', name: 'Novelas visuales', c: 'var(--vn)' }, { id: 'jvn', name: 'Novelas japonesas', c: 'var(--jvn)' },
];
// Terminados: [título, categoría, año, mes (1-12), día, nota]
const FIN = [
  ['Celeste', 'pc', 2026, 8, 14, 5], ['Stardew Valley', 'pc', 2026, 3, 2, 4], ['Balatro', 'pc', 2026, 9, 30, 3], ['Hades II', 'pc', 2026, 11, 6, 5],
  ['The Witcher 3', 'pc', 2025, 11, 20, 5], ['Red Dead Redemption 2', 'pc', 2025, 6, 1, 0], ['Hi-Fi Rush', 'pc', 2024, 6, 21, 4],
  ['Super Mario Odyssey', 'sw', 2026, 5, 9, 4], ['Zelda: Tears of the Kingdom', 'sw', 2026, 10, 18, 5], ['Kirby y la tierra olvidada', 'sw', 2025, 4, 12, 4],
  ['Metroid Dread', 'sw', 2024, 10, 15, 5], ['Animal Crossing', 'sw', 2024, 3, 20, 3],
  ['Steins;Gate', 'vn', 2026, 2, 11, 5], ['Summer Pockets', 'vn', 2026, 7, 27, 4], ['Fata Morgana', 'vn', 2025, 9, 3, 5],
  ['Kino no Tabi', 'jvn', 2026, 4, 22, 4], ['Kokoro', 'jvn', 2026, 10, 2, 3], ['Kimi no Na wa', 'jvn', 2025, 12, 28, 4],
];
const MES = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
const DIAS = ['L', 'M', 'X', 'J', 'V', 'S', 'D'];
const ini = (n) => n.split(/[\s:;]+/).filter((w) => /^[A-ZÁÉÍÓÚ0-9]/.test(w)).slice(0, 2).map((w) => w[0]).join('');
const cov = (n, w = 54, cls = '') => `<div class="cov ${cls}" title="${n}" style="--w:${w}px;--g1:${G[n][0]};--g2:${G[n][1]}">${ini(n)}</div>`;
const stars = (r) => (r ? `<u>${'★'.repeat(r)}</u>` : '');
const fin = (f, w = 54, cls = '') => `<div class="fin">${cov(f[0], w, cls)}<span>${f[4]} ${MES[f[3] - 1]}</span>${stars(f[5])}</div>`;
const bars = (a, lab = DIAS, h = 52) => { const m = Math.max(1, ...a); return `<div class="bars" style="--h:${h}px">${a.map((v, i) => `<div><i class="${v ? '' : 'z'}" style="height:${v ? Math.max(6, (v / m) * 100) : 0}%"></i>${lab ? `<span>${lab[i]}</span>` : ''}</div>`).join('')}</div>`; };
const trow = (n, sub, t, pct, w = 30) => `<div class="trow">${cov(n, w)}<span class="n">${n}${sub ? `<small>${sub}</small>` : ''}</span>${pct != null ? `<span class="bar"><i style="width:${pct}%"></i></span>` : ''}<span>${t}</span></div>`;
const hints = (l) => `<div class="hints">${l.map(([b, t, c = '']) => `<span class="hint pill"><span class="btn ${c}">${b}</span>${t}</span>`).join('')}</div>`;
const tb = (crumbs) => `<div class="crumb pill">${crumbs.map((c, i) => (i === crumbs.length - 1 ? `<b>${c}</b>` : `${c}<i>›</i>`)).join('')}</div><div class="clock">lun 9 nov · 21:14</div>`;
const tabs = (l, on, nw = []) => `<div class="tabs pill">${l.map((t) => `<span class="${t === on ? 'on' : ''} ${nw.includes(t) ? 'new' : ''}">${t}</span>`).join('')}</div>`;
const rbox = (k, v, d = '', st = '') => `<div class="rbox grow" style="${st}"><div class="k">${k}</div><div class="v">${v}</div>${d ? `<div class="d">${d}</div>` : ''}</div>`;
const goal7 = (a) => `<div class="goal7">${a.map((x) => `<i class="${x ? 'ok' : ''}"></i>`).join('')}</div>`;
// Semana de ejemplo (2–8 nov), usada en las tres propuestas.
const WEEK = {
  pc: { t: '9 h 20', d: '▲ 1 h 40', up: 1, days: [0, 90, 45, 0, 120, 185, 120], top: [['Hades II', '12 partidas', '6 h 05'], ['Balatro', '5 partidas', '2 h 10'], ['Celeste', '2 partidas', '1 h 05']] },
  sw: { t: '6 h 45', d: '▼ 50 min', up: 0, days: [60, 0, 0, 75, 0, 150, 120], top: [['Zelda: Tears of the Kingdom', 'en la consola', '5 h 30'], ['Animal Crossing', 'en la consola', '1 h 15']] },
  vn: { t: '3 h 10', d: '▲ 40 min', up: 1, goal: [1, 1, 0, 1, 1, 0, 1], top: 'Summer Pockets' },
  jvn: { t: '2 h 25', d: '= igual', up: 1, goal: [1, 1, 1, 1, 0, 1, 1], top: 'Kokoro' },
};
const delta = (w) => `<span class="${w.d[0] === '=' ? 'mut' : w.up ? 'up' : 'dn'}">${w.d}</span>`;
function mount(id, html) { document.getElementById(id).innerHTML = html; }
// Para las capturas: «fichero.html?solo=a1» deja solo ese marco en la página.
addEventListener('load', () => { const id = new URLSearchParams(location.search).get('solo'); if (!id) return;
  document.body.style.padding = '0'; for (const el of [...document.body.children]) if (el.id !== id) el.style.display = 'none'; });
