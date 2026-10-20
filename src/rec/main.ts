// Indicador de grabacion sobre el juego (ver src-tauri/src/overlay.rs). La app lo actualiza con `window.__rec(estado, segundos, texto)`.
export {};
const p = document.getElementById('p')!;
const t = document.getElementById('t')!;
const mmss = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;

function set(state: string, secs: number, text = '') {
  p.className = `pill ${state}`;
  t.textContent = state === 'recording' ? `REC ${mmss(secs)}` : state === 'done' ? `✓ ${secs} s` : state === 'audio' ? `Audio: ${text}` : state === 'goal' ? text : 'Sin audio';
}

(window as unknown as { __rec: typeof set }).__rec = set;
const q = new URLSearchParams(location.search);
set(q.get('s') ?? 'recording', Number(q.get('t') ?? 0), q.get('x') ?? '');
