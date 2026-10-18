// Construye el HTML de una linea (texto real, sin espacios entre palabras: Yomitan escanea el texto contiguo).
import type { GrammarHit, ReaderLine } from '../lib/novel-types';

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const POS = new Set(['n', 'v', 'a', 'adv', 'p', 'aux', 'i', 'x']);

export interface RenderOpts {
  /** Formas de diccionario conocidas: sin furigana ni subrayado. */
  known?: Set<string>;
  /** Resaltado: indice de token o rango [desde, hasta] (fila de gramatica). */
  hl?: number | [number, number];
  /** Subrayado punteado de la gramatica. */
  grammar?: GrammarHit[];
}

/** `<span class="w v" data-i="3">` por palabra, con `<ruby>` en los trozos que tengan furigana si `furigana` es true. */
export function lineHtml(line: ReaderLine, furigana: boolean, o: RenderOpts = {}): string {
  if (!line.toks?.length) return esc(line.text);
  const hl = o.hl ?? -1;
  const inHl = (i: number) => (Array.isArray(hl) ? i >= hl[0] && i <= hl[1] : i === hl);
  const gram = new Set<number>();
  for (const g of o.grammar ?? []) for (let i = g.toks[0]; i <= g.toks[1]; i++) gram.add(i);
  let out = '';
  for (const [i, t] of line.toks.entries()) {
    const surface = t.parts.map((p) => p[0]).join('');
    const known = !!o.known && POS_WORD.has(t.pos) && o.known.has(t.base ?? surface);
    out += `<span class="w ${POS.has(t.pos) ? t.pos : 'x'}${known ? ' known' : ''}${gram.has(i) ? ' g' : ''}${inHl(i) ? ' hl' : ''}" data-i="${i}">`;
    for (const [base, rt] of t.parts) {
      out += furigana && rt && !known ? `<ruby>${esc(base)}<rt>${esc(rt)}</rt></ruby>` : esc(base);
    }
    out += '</span>';
  }
  return out;
}

const POS_WORD = new Set(['n', 'v', 'a', 'adv', 'i']);

/** Texto plano con `**negrita**` -> HTML seguro. */
export function boldHtml(s: string): string {
  return esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>');
}

/** La frase con la palabra (superficie) en negrita, para Anki y su vista previa. */
export function sentenceWithWord(line: ReaderLine, tok: number | [number, number] | null): string {
  if (!line.toks?.length) return esc(line.text);
  const [a, b] = tok === null ? [-1, -1] : Array.isArray(tok) ? tok : [tok, tok];
  return line.toks.map((t, i) => {
    const s = esc(t.parts.map((p) => p[0]).join(''));
    return i >= a && i <= b ? `<b>${s}</b>` : s;
  }).join('');
}
