// Niveles Categoría y Juego de Estadísticas. Se ejecutan con: node --test tests/gamestats.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  catSummary, dayPart, fmtDaySpan, fmtHHMM, fmtSpan, hourHistogram, nextRoundHours, nextRoundMilestone, peakHour,
  periodRange, titleFavHour, titleRank, titleSeries, titleStreak,
} from '../src/lib/stats.ts';

const H = 3600000;
const at = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min).getTime();
const play = (y: number, m: number, d: number, h = 18, len = 1): [number, number] => [at(y, m, d, h), at(y, m, d, h) + len * H];
const title = (name: string, plays: Array<[number, number]>, extra: Record<string, unknown> = {}) =>
  ({ name, hidden: false, shots: [], plays, playSeconds: 0, sessions: plays.length, ...extra });
const NOW = at(2026, 11, 20, 12);

const A = title('A', [play(2026, 1, 1, 19, 3), play(2026, 3, 10, 20), play(2026, 3, 11, 20), play(2026, 3, 12, 21, 2)]);
const B = title('B', [play(2025, 5, 5, 9), play(2025, 5, 6, 10)], { status: 'finished', finishedMs: at(2026, 2, 1), shots: [{ path: 'x', date: at(2026, 4, 1) - new Date(2026, 3, 1).getTimezoneOffset() * 60000 }] });
const C = title('C', [play(2026, 6, 1, 15, 4)]);
const lib = {
  platforms: [
    { id: 'pc', name: 'PC', titles: [A, B] },
    { id: 'switch', name: 'Switch', titles: [C] },
  ],
} as any;

test('hora del día: reparte las partidas que cruzan horas y da el pico', () => {
  const t = title('X', [[at(2026, 1, 1, 22, 30), at(2026, 1, 2, 0, 30)]]);
  const h = hourHistogram({ platforms: [{ id: 'pc', name: 'PC', titles: [t] }] } as any, 0, periodRange('all', 0));
  assert.equal(h[22], H / 2 + 0);
  assert.equal(h[23], H);
  assert.equal(h[0], H / 2);
  assert.equal(peakHour(h), 23);
  assert.equal(peakHour(new Array(24).fill(0)), -1);
});

test('próximo hito: cifras redondas', () => {
  assert.equal(nextRoundHours(0), 50);
  assert.equal(nextRoundHours(50), 100);
  assert.equal(nextRoundHours(248), 250);
  assert.equal(nextRoundHours(250), 500);
  assert.equal(nextRoundHours(999), 1000);
  assert.equal(nextRoundHours(5000), 10000);
  const m = nextRoundMilestone(248 * H);
  assert.equal(m.h, 250);
  assert.equal(m.leftMs, 2 * H);
});

test('catSummary: solo cuenta la categoría pedida y reparte el porcentaje', () => {
  const all = catSummary(lib, 0, periodRange('all', 0, NOW), NOW);
  assert.equal(all.ms, 9 * H);
  assert.equal(all.share, Math.round((9 / 13) * 100));
  assert.equal(all.days, 6);
  assert.equal(all.finished, 1);
  assert.equal(all.shots, 1);
  assert.equal(all.longest!.ms, 3 * H);
  assert.equal(all.longest!.t.name, 'A');
  const y25 = catSummary(lib, 0, periodRange('year', -1, NOW), NOW);
  assert.equal(y25.ms, 2 * H);
  assert.equal(y25.days, 2);
  assert.equal(y25.ofDays, 365);
  assert.equal(y25.finished, 0);
  assert.equal(y25.allMs, 9 * H);
  const y26 = catSummary(lib, 1, periodRange('year', 0, NOW), NOW);
  assert.equal(y26.ms, 4 * H);
  assert.equal(y26.share, Math.round((4 / 11) * 100));
  assert.equal(y26.ofDays, 324); // del 1 de enero al 20 de noviembre incluido
});

test('titleRank: puesto dentro de su categoría', () => {
  assert.deepEqual(titleRank(lib, 0, 0), { pos: 1, of: 2 });
  assert.deepEqual(titleRank(lib, 0, 1), { pos: 2, of: 2 });
  assert.deepEqual(titleRank(lib, 1, 0), { pos: 1, of: 1 });
});

test('titleSeries: meses del año, semanas y años', () => {
  const m = titleSeries(A, 'month', NOW);
  assert.equal(m.bars.length, 12);
  assert.equal(m.bars[0].ms, 3 * H);
  assert.equal(m.bars[2].ms, 4 * H);
  assert.ok(m.bars[10].cur);
  assert.match(m.caption, /2026/);
  // Sin partidas este año: el último año con partidas.
  const old = titleSeries(B, 'month', NOW);
  assert.match(old.caption, /2025/);
  assert.equal(old.bars[4].ms, 2 * H);
  const y = titleSeries(A, 'year', NOW);
  assert.deepEqual(y.bars.map((b) => b.label), ['2026']);
  assert.equal(y.bars[0].ms, 7 * H);
  const w = titleSeries(title('W', [play(2026, 11, 18)]), 'week', NOW);
  assert.equal(w.bars.length, 12);
  assert.equal(w.bars[11].ms, H);
  assert.ok(w.bars[11].cur);
  assert.equal(w.bars.slice(0, 11).reduce((a, b) => a + b.ms, 0), 0);
});

test('titleStreak: racha máxima de días seguidos', () => {
  const s = titleStreak(A)!;
  assert.equal(s.n, 3);
  assert.equal(s.from, at(2026, 3, 10));
  assert.equal(s.to, at(2026, 3, 12));
  assert.equal(titleStreak(title('E', [])), null);
  assert.equal(fmtDaySpan(s.from, s.to, NOW), 'del 10 al 12 de marzo');
  assert.equal(fmtDaySpan(at(2025, 3, 10), at(2025, 3, 10), NOW), 'el 10 de marzo de 2025');
});

test('titleFavHour: hora de inicio más frecuente y tramo del día', () => {
  const f = titleFavHour(A)!;
  assert.equal(f.hour, 20);
  assert.equal(f.part, 'tarde');
  assert.equal(titleFavHour(title('N', [play(2026, 1, 1, 23), play(2026, 1, 2, 23), play(2026, 1, 3, 3)]))!.part, 'noche');
  assert.equal(titleFavHour(title('N', [])), null);
  assert.equal(dayPart(7), 'mañana');
  assert.equal(dayPart(14), 'tarde');
  assert.equal(dayPart(21), 'noche');
});

test('formatos: horas y tramos de tiempo', () => {
  assert.equal(fmtHHMM(at(2026, 1, 1, 7, 5)), '07:05');
  assert.equal(fmtSpan(at(2026, 1, 3), at(2026, 1, 4)), '1 día');
  assert.equal(fmtSpan(at(2026, 1, 3), at(2026, 1, 24)), '3 semanas');
  assert.equal(fmtSpan(at(2026, 1, 3), at(2026, 11, 14)), '10 meses');
  assert.equal(fmtSpan(at(2023, 1, 3), at(2026, 1, 3)), '3 años');
});

test('periodos de categoría: años con partidas y rangos', async () => {
  const { catYears, catPerList, catPerRange } = await import('../src/lib/stats.ts');
  assert.deepEqual(catYears(lib, 0), [2025, 2026]);
  assert.deepEqual(catPerList(lib, 1), ['all', 'd30', 2026]);
  const d = catPerRange('d30', NOW);
  assert.equal(Math.round((d.to - d.from) / 3600000 / 24), 30);
  assert.equal(catPerRange(2025, NOW).label, '2025');
});
