// Teclado en pantalla estilo Wii U: teclas de cristal, foco espacial, A escribe, B borra, X espacio, Y mayúsculas,
// Start acepta. También acepta el teclado físico (escribe tal cual; Intro pulsa la tecla enfocada, Tab acepta, Esc cancela).
import { app } from './store.svelte';
import { blip } from './sound';

export type OskKind = 'char' | 'shift' | 'space' | 'bksp' | 'ok' | 'cancel';
export interface OskKey { kind: OskKind; ch?: string; label: string; span: number }

const row = (chars: string): OskKey[] => [...chars].map((ch) => ({ kind: 'char', ch, label: ch, span: 1 }));
/** Disposición española de 10 columnas; la fila de acentos sustituye a la pulsación larga. */
export const OSK_ROWS: OskKey[][] = [
  row('1234567890'),
  row('qwertyuiop'),
  row('asdfghjklñ'),
  row('zxcvbnm,.-'),
  row('áéíóúü¿¡:/'),
  [
    { kind: 'shift', label: 'Mayús', span: 2 },
    { kind: 'space', label: 'Espacio', span: 3 },
    { kind: 'bksp', label: 'Borrar', span: 1 },
    { kind: 'cancel', label: 'Cancelar', span: 2 },
    { kind: 'ok', label: 'Aceptar', span: 2 },
  ],
];
const start = (r: number, c: number) => OSK_ROWS[r].slice(0, c).reduce((n, k) => n + k.span, 0);
/** Texto que muestra una tecla con o sin mayúsculas. */
export const keyLabel = (k: OskKey, shift: boolean) => (k.kind === 'char' ? (shift ? k.ch!.toUpperCase() : k.ch!) : k.label);

let onDone: ((v: string) => void) | undefined;
let onCancel: (() => void) | undefined;
let onChange: ((v: string) => void) | undefined;
let maxLen = 80;

export interface OskOpts {
  label: string;
  value: string;
  hint?: string;
  maxLen?: number;
  /** Empezar en mayúsculas si el campo está vacío (nombres). */
  capitalize?: boolean;
  onDone: (v: string) => void;
  onCancel?: () => void;
  onChange?: (v: string) => void;
}

/** Ctrl+V con el teclado abierto pega el texto del portapapeles (p. ej. una clave de API). */
let pasteOn = false;
function onPaste(e: ClipboardEvent) {
  if (!app.osk.open) return;
  const txt = (e.clipboardData?.getData('text') ?? '').replace(/[\r\n\t]+/g, ' ').trim();
  if (!txt) return;
  e.preventDefault();
  set([...(app.osk.value + txt)].slice(0, maxLen).join(''));
  blip(1100);
}

export function openOsk(o: OskOpts) {
  if (!pasteOn) { pasteOn = true; window.addEventListener('paste', onPaste); }
  onDone = o.onDone; onCancel = o.onCancel; onChange = o.onChange; maxLen = o.maxLen ?? 80;
  app.osk = { open: true, label: o.label, value: o.value, shift: !!o.capitalize && !o.value, r: 1, c: 0, hint: o.hint ?? '' };
  blip(1320);
}

function close() { app.osk.open = false; onDone = onCancel = onChange = undefined; }

export function oskDone() {
  const v = app.osk.value;
  const cb = onDone;
  close();
  cb?.(v);
  blip(1320);
}
export function oskCancel() {
  const cb = onCancel;
  close();
  cb?.();
  blip(520);
}
function set(v: string) { app.osk.value = v; onChange?.(v); }

export function oskType(ch: string) {
  if ([...app.osk.value].length >= maxLen) return;
  set(app.osk.value + (app.osk.shift ? ch.toUpperCase() : ch));
  app.osk.shift = false; // una mayúscula cada vez
}
export function oskBackspace() { const a = [...app.osk.value]; a.pop(); set(a.join('')); blip(520); }
export function oskSpace() { if (!app.osk.value.endsWith(' ') && app.osk.value) set(app.osk.value + ' '); else if (!app.osk.value) blip(220); blip(700); }
export function oskShift() { app.osk.shift = !app.osk.shift; blip(880); }

export function oskPress(r: number, c: number) {
  const k = OSK_ROWS[r]?.[c];
  if (!k) return;
  if (k.kind === 'char') { oskType(k.ch!); blip(1100); }
  else if (k.kind === 'shift') oskShift();
  else if (k.kind === 'space') oskSpace();
  else if (k.kind === 'bksp') oskBackspace();
  else if (k.kind === 'ok') oskDone();
  else if (k.kind === 'cancel') oskCancel();
}

function moveV(d: number) {
  const { r, c } = app.osk;
  const nr = r + d;
  if (nr < 0 || nr >= OSK_ROWS.length) { blip(220); return; }
  const mid = start(r, c) + OSK_ROWS[r][c].span / 2;
  let acc = 0, nc = OSK_ROWS[nr].length - 1;
  for (let i = 0; i < OSK_ROWS[nr].length; i++) {
    acc += OSK_ROWS[nr][i].span;
    if (mid <= acc) { nc = i; break; }
  }
  app.osk.r = nr; app.osk.c = nc; blip(880);
}
function moveH(d: number) {
  const n = OSK_ROWS[app.osk.r].length;
  const nc = app.osk.c + d;
  if (nc < 0 || nc >= n) { blip(220); return; }
  app.osk.c = nc; blip(880);
}

/** Órdenes del mando o de las flechas / Intro / Retroceso del teclado. */
export function oskCmd(c: string) {
  if (c === 'up') moveV(-1);
  else if (c === 'down') moveV(1);
  else if (c === 'left') moveH(-1);
  else if (c === 'right') moveH(1);
  else if (c === 'a') oskPress(app.osk.r, app.osk.c);
  else if (c === 'b') oskBackspace();
  else if (c === 'x') oskSpace();
  else if (c === 'y') oskShift();
  else if (c === 'menu') oskDone();
}

/** Teclado físico mientras el teclado en pantalla está abierto. Devuelve true si lo ha gestionado. */
export function oskKeydown(e: KeyboardEvent): boolean {
  if (!app.osk.open) return false;
  if (e.ctrlKey || e.altKey || e.metaKey) return false;
  const k = e.key;
  const map: Record<string, string> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down', Enter: 'a', Tab: 'menu' };
  e.preventDefault();
  if (map[k]) oskCmd(map[k]);
  else if (k === 'Escape') oskCancel();
  else if (k === 'Backspace') oskBackspace();
  else if (k.length === 1) { if (k === ' ') oskSpace(); else if ([...app.osk.value].length < maxLen) set(app.osk.value + k); }
  return true;
}
