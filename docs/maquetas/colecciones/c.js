// Datos de ejemplo y piezas compartidas por las propuestas de «Colecciones» (solo maqueta, nada de esto es código de la app).
// Juego: [nombre, sigla, año, color 1, color 2, estado (t terminado · j jugando · p pendiente), horas, capturas]
const COLS = [
  { name: 'Trails', c: '#3b8fd0', c2: '#19b8a6', games: [
    ['Trails in the Sky', 'Sky', 2004, '#4aa3e0', '#f2d06b', 't', 48, 212], ['Trails in the Sky SC', 'SC', 2006, '#3b6fd0', '#e58a48', 't', 61, 305],
    ['Trails in the Sky the 3rd', '3rd', 2007, '#6E56CF', '#2a3a6b', 't', 39, 140], ['Trails from Zero', 'Zero', 2010, '#19b8a6', '#2a4a7a', 't', 52, 261],
    ['Trails to Azure', 'Azure', 2011, '#2d6fe0', '#7ad0f0', 'j', 23, 118], ['Trails of Cold Steel', 'CS I', 2013, '#b03a2e', '#3a2a2a', 'p', 0, 0],
    ['Trails of Cold Steel II', 'CS II', 2014, '#c94a6a', '#2a2a4a', 'p', 0, 0], ['Trails into Reverie', 'Rev', 2020, '#7a3fd0', '#e0483a', 'p', 0, 0]] },
  { name: 'Kingdom Hearts', c: '#6E56CF', c2: '#e58a48', games: [
    ['Kingdom Hearts', 'KH', 2002, '#3b6fd0', '#f2d06b', 't', 31, 96], ['Chain of Memories', 'CoM', 2004, '#8a2a3a', '#2a1a2a', 't', 18, 41],
    ['Kingdom Hearts II', 'KH II', 2005, '#6E56CF', '#e58a48', 't', 42, 180], ['Birth by Sleep', 'BbS', 2010, '#5ab0e6', '#2a3a6b', 'j', 9, 37],
    ['Dream Drop Distance', 'DDD', 2012, '#e85a9b', '#5b3fb0', 'p', 0, 0], ['Kingdom Hearts III', 'KH III', 2019, '#2d4fb0', '#c9a227', 'p', 0, 0]] },
  { name: 'Yakuza', c: '#e0384a', c2: '#f2b807', games: [
    ['Yakuza 0', 'Y0', 2015, '#e0384a', '#f2b807', 't', 74, 420], ['Yakuza Kiwami', 'K1', 2016, '#b03a2e', '#3a2a2a', 't', 28, 133],
    ['Yakuza Kiwami 2', 'K2', 2017, '#c92a2a', '#2a3a6b', 'j', 12, 58], ['Like a Dragon', 'LaD', 2020, '#e58a48', '#3b6fd0', 'p', 0, 0]] },
  { name: 'Persona', c: '#f2b807', c2: '#e0384a', games: [
    ['Persona 4 Golden', 'P4G', 2012, '#f2b807', '#e58a48', 't', 82, 351], ['Persona 5 Royal', 'P5R', 2019, '#e0384a', '#1a1a1a', 't', 121, 640],
    ['Persona 3 Reload', 'P3R', 2024, '#2d6fe0', '#5cc9f0', 'p', 0, 0]] },
  { name: 'FromSoftware', c: '#c9a227', c2: '#5fbf8f', games: [
    ['Dark Souls', 'DS', 2011, '#8a6a2a', '#2a2a2a', 't', 55, 147], ['Dark Souls III', 'DS III', 2016, '#b0562e', '#2a1a1a', 't', 47, 122],
    ['Sekiro', 'Sek', 2019, '#c94a2a', '#e5b848', 'p', 3, 9], ['Elden Ring', 'ER', 2022, '#c9a227', '#3a4a2a', 'j', 96, 512]] },
];
const ST = { t: ['Terminado', '#3BB54A'], j: ['Jugando', '#00A0D8'], p: ['Pendiente', '#7f8899'] };
const sum = (c, k) => c.games.reduce((n, g) => n + g[k], 0);
const hrs = (c) => sum(c, 6), caps = (c) => sum(c, 7), done = (c) => c.games.filter((g) => g[5] === 't').length;
const sub = (c) => `${c.games.length} juegos · ${hrs(c)} h · ${caps(c)} capturas`;
const cv = (c) => `--c:${c.c};--c2:${c.c2}`;
const cov = (g, w = 54, cls = '', st = '') => `<div class="cov ${cls}" title="${g[0]}" style="--w:${w}px;--g1:${g[3]};--g2:${g[4]};${st}">${g[1]}</div>`;
const shot = (g, k = 0, cls = '', st = '', lab = '') => `<div class="shot ${cls}" style="--g1:${g[3]};--g2:${g[4]};--a:${(k * 67 + 100) % 360}deg;${st}">${lab ? `<span class="chip">${lab}</span>` : ''}</div>`;
const seg = (c) => `<div class="seg" style="--c:${c.c}">${c.games.map((g) => `<i class="${g[5]}"></i>`).join('')}</div>`;
const st = (g) => `<span class="st" style="--s:${ST[g[5]][1]}">${ST[g[5]][0]}</span>`;
const hints = (l) => `<div class="hints">${l.map(([b, t, c = '']) => `<span class="hint pill"><span class="btn ${c}">${b}</span>${t}</span>`).join('')}</div>`;
const tb = (crumbs) => `<div class="crumb pill">${crumbs.map((c, i) => (i === crumbs.length - 1 ? `<b>${c}</b>` : `${c}<i>›</i>`)).join('')}</div><div class="clock">lun 9 nov · 21:14</div>`;
const tabs = (on = 'Colecciones') => `<div class="tabs pill">${['Todos', 'Mi lista', 'Colecciones'].map((t) => `<span class="${t === on ? 'on' : ''}">${t}</span>`).join('')}</div>`;
function mount(id, html) { document.getElementById(id).innerHTML = html; }
// Para las capturas: «fichero.html?solo=a1» deja solo ese marco en la página.
addEventListener('load', () => { const id = new URLSearchParams(location.search).get('solo'); if (!id) return;
  document.body.style.padding = '0'; for (const el of [...document.body.children]) if (el.id !== id) el.style.display = 'none'; });
