import type { ThreadInfo } from '../lib/novel-types';

export type Reco = { score: number; level: 'good' | 'maybe' | 'bad' | 'none'; label: string; why: string };

const KANA = /[぀-ヿㇰ-ㇿ]/;
const KANJI = /[㐀-䶿一-鿿]/;

/** Puntúa un hilo según su último texto: japonés «normal», sin duplicados tipo 明明日日 ni líneas de 1-2 caracteres. */
export function recommend(t: ThreadInfo): Reco {
  const s = (t.last ?? '').replace(/\s+/g, '');
  if (!s) return { score: 0, level: 'none', label: 'Sin texto', why: 'Todavía no ha enviado texto.' };
  const chars = [...s];
  const n = chars.length;
  const jp = chars.filter(c => KANA.test(c) || KANJI.test(c)).length;
  const kana = chars.filter(c => KANA.test(c)).length;
  let score = (jp / n) * 70;
  if (kana > 0 && kana < n) score += 10; // mezcla kana + kanji: típico de prosa
  if (n >= 8) score += 10;
  if (t.lines >= 3) score += 5;
  const notes: string[] = [];
  if (n <= 2) { score -= 45; notes.push('líneas muy cortas'); }
  if (n >= 4) {
    let pairs = 0;
    for (let i = 0; i + 1 < n; i += 2) if (chars[i] === chars[i + 1]) pairs++;
    let adj = 0;
    for (let i = 0; i + 1 < n; i++) if (chars[i] === chars[i + 1]) adj++;
    if (pairs / Math.floor(n / 2) > 0.5 || adj / (n - 1) > 0.4) { score -= 55; notes.push('caracteres duplicados'); }
  }
  if (jp / n < 0.4) notes.push('poco japonés');
  score = Math.max(0, Math.min(100, Math.round(score)));
  if (score >= 70) return { score, level: 'good', label: 'Recomendado', why: 'Texto japonés limpio.' };
  if (score >= 40) return { score, level: 'maybe', label: 'Posible', why: notes.join(', ') || 'Puede servir.' };
  return { score, level: 'bad', label: 'Poco útil', why: notes.join(', ') || 'No parece diálogo.' };
}
