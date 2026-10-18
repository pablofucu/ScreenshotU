/* Asignar el botón de captura «esperando el input»: combinación del mando, botón del ratón o tecla. */
import { app, type LearnKind } from './store.svelte';
import { backend } from './backend';
import { blip } from './sound';
import { patchConfig, toast } from './controller.svelte';

/** Botones del mando (ids de `visor_core::capture::PAD_BUTTONS`) y cómo se enseñan. */
const PAD_LABEL: Record<string, string> = {
  back: 'View', start: 'Menú', guide: 'Xbox', lb: 'LB', rb: 'RB', lt: 'LT', rt: 'RT', l3: 'L3', r3: 'R3',
  a: 'A', b: 'B', x: 'X', y: 'Y', up: '↑', down: '↓', left: '←', right: '→',
};
/** Combinaciones de mando que se recorren con izquierda/derecha. */
export const COMBO_PRESETS = ['back+rb', 'back+lb', 'lb+rb', 'back+start', 'l3+r3', 'off'];
export const comboLabel = (id: string) =>
  !id || id === 'off' ? 'Desactivado' : id.split('+').map((b) => PAD_LABEL[b] ?? b).join(' + ');

/** Botones de ratón que pueden capturar (ids de `visor_core::capture::MOUSE_BUTTONS`). */
export const MOUSE_PRESETS = ['', 'middle', 'x1', 'x2'];
const MOUSE_LABEL: Record<string, string> = { '': 'Desactivado', middle: 'Botón central (rueda)', x1: 'Lateral atrás', x2: 'Lateral adelante' };
export const mouseLabel = (id: string) => MOUSE_LABEL[id] ?? id;

/** Atajos de teclado que se recorren con izquierda/derecha ("" = desactivado). */
export const HOTKEY_PRESETS = ['F12', 'F10', 'F9', 'Ctrl+F12', 'Alt+F12', 'Ctrl+Shift+S', ''];
export const hotkeyLabel = (k: string) => k || 'Desactivado';

const SECONDS = 12;
let timer: ReturnType<typeof setInterval> | undefined;

export function startLearn(kind: LearnKind, target: 'capture' | 'record' = 'capture') {
  if (kind === 'pad' && !backend.isTauri) { toast('El mando solo se puede asignar en la aplicación de escritorio', 2600); return; }
  app.learn = { kind, target, combo: 'off', connected: true, left: SECONDS };
  if (kind === 'pad') void backend.learnPad(true);
  if (kind === 'key') void backend.hotkeySuspend(true);
  clearInterval(timer);
  timer = setInterval(() => {
    if (!app.learn) { clearInterval(timer); return; }
    // Mientras se mantiene una combinación no se corta.
    if (app.learn.kind === 'pad' && app.learn.combo !== 'off') return;
    if (--app.learn.left <= 0) cancelLearn('Se acabó el tiempo: no ha cambiado nada');
  }, 1000);
  blip(1320);
}

function close() {
  const k = app.learn?.kind;
  app.learn = null;
  clearInterval(timer);
  if (k === 'pad') void backend.learnPad(false);
  return k;
}

export function cancelLearn(msg?: string) {
  const k = close();
  if (k === 'key') void backend.hotkeySuspend(false);
  if (msg) toast(msg, 2200);
  blip(520);
}

async function finish(p: Parameters<typeof patchConfig>[0], label: string, what = 'Capturar con') {
  const k = close();
  const ok = await patchConfig(p);
  if (k === 'key') await backend.hotkeySuspend(false).catch(() => {});
  if (ok) toast(`${what}: ${label}`, 2200);
  blip(1320);
}

/** Evento del hilo del mando mientras se aprende. */
export function onLearnPad(combo: string, connected: boolean, done: boolean) {
  if (app.learn?.kind !== 'pad') return;
  app.learn.connected = connected;
  app.learn.combo = combo;
  if (done && combo !== 'off') void finish({ captureCombo: combo }, comboLabel(combo));
}

/* ---------- teclado ---------- */
const MOD_CODES = new Set(['ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight']);
/** Teclas que acepta el registro de atajos globales (`global-hotkey`) tal cual las da `KeyboardEvent.code`. */
const PLAIN_CODES = new Set([
  'PrintScreen', 'Pause', 'ScrollLock', 'Insert', 'Delete', 'Home', 'End', 'PageUp', 'PageDown',
  'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space', 'Tab', 'Enter', 'Backspace', 'Minus', 'Equal',
  'BracketLeft', 'BracketRight', 'Backslash', 'Semicolon', 'Quote', 'Backquote', 'Comma', 'Period', 'Slash',
]);
/** Teclas que se pueden usar solas sin estorbar al escribir ni al jugar normalmente. */
const SAFE_ALONE = /^(F\d{1,2}|PrintScreen|Pause|ScrollLock|Insert)$/;

function keyName(code: string): string | null {
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return m[1];
  m = /^Digit(\d)$/.exec(code);
  if (m) return m[1];
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  if (/^Numpad(\d|Add|Subtract|Multiply|Divide|Decimal|Enter)$/.test(code)) return code;
  return PLAIN_CODES.has(code) ? code : null;
}

/** Tecla pulsada mientras se espera un atajo. Devuelve `true` si la consumió. */
export function learnKeydown(e: KeyboardEvent): boolean {
  if (!app.learn) return false;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === 'Escape') { cancelLearn(); return true; }
  if (app.learn.kind !== 'key' || e.repeat) return true;
  // Impr Pant solo llega al soltarla en Windows.
  if (e.type === 'keydown' && e.code === 'PrintScreen') return true;
  if (e.type === 'keyup' && e.code !== 'PrintScreen') return true;
  if (MOD_CODES.has(e.code)) return true;
  const k = keyName(e.code);
  if (!k) { toast('Esa tecla no se puede usar como atajo', 2200); return true; }
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean) as string[];
  if (!mods.length && !SAFE_ALONE.test(k)) {
    toast('Añade Ctrl, Alt o Shift: esa tecla sola dejaría de funcionar al escribir', 3200);
    return true;
  }
  const combo = [...mods, k].join('+');
  if (app.learn.target === 'record') void finish({ recordHotkey: combo }, combo, 'Grabar audio con');
  else void finish({ captureHotkey: combo }, combo);
  return true;
}

/* ---------- ratón ---------- */
/** Botón recién asignado: su `mouseup` tampoco debe llegar a la interfaz (el lateral haría «volver»). */
let swallow = -1;
/** Botón del ratón pulsado mientras se espera. Izquierdo/derecho no se asignan (sirven para Cancelar). */
export function learnMouse(e: MouseEvent): boolean {
  if (e.type !== 'mousedown' && e.button === swallow) {
    if (e.type === 'mouseup') swallow = -1;
    e.preventDefault();
    e.stopPropagation();
    return true;
  }
  if (app.learn?.kind !== 'mouse') return false;
  const id = e.button === 1 ? 'middle' : e.button === 3 ? 'x1' : e.button === 4 ? 'x2' : '';
  if (!id) return false;
  e.preventDefault();
  e.stopPropagation();
  if (e.type === 'mousedown') {
    swallow = e.button;
    void finish({ captureMouse: id }, mouseLabel(id));
  }
  return true;
}
