// Terminados por año. Se ejecuta con: node --test tests/finyears.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { finishedByYear, finishedYear } from '../src/lib/stats.ts';

const at = (y: number, m: number, d: number) => new Date(y, m - 1, d, 12).getTime();
const t = (name: string, y: number, m: number, d: number, extra: object = {}) =>
  ({ name, hidden: false, shots: [], plays: [], playSeconds: 0, sessions: 0, status: 'finished', finishedMs: at(y, m, d), ...extra });
const lib = {
  platforms: [
    { id: 'pc', name: 'PC', titles: [t('A', 2025, 1, 5), t('B', 2026, 3, 1), t('C', 2026, 3, 9), t('Oculto', 2026, 4, 1, { hidden: true })] },
    { id: 'switch', name: 'Switch', titles: [t('D', 2026, 6, 2), t('E', 2024, 7, 7), t('F', 2026, 1, 1, { status: 'playing' }), t('G', 2026, 2, 2, { finishedMs: 0 })] },
  ],
} as any;

test('finishedByYear: total y reparto por categoría, sin ocultos ni sin fecha', () => {
  const r = finishedByYear(lib);
  assert.deepEqual(r.map((e) => [e.year, e.count]), [[2024, 1], [2025, 1], [2026, 3]]);
  const y26 = r.find((e) => e.year === 2026)!;
  assert.equal(y26.by.get(0), 2);
  assert.equal(y26.by.get(1), 1);
});

test('finishedByYear: con categoría solo cuenta esa', () => {
  const r = finishedByYear(lib, 1);
  assert.deepEqual(r.map((e) => [e.year, e.count]), [[2024, 1], [2026, 1]]);
  assert.equal(r[0].by.size, 1);
});

test('finishedYear filtra por categoría sin cambiar el comportamiento por defecto', () => {
  assert.equal(finishedYear(lib, 2026).count, 3);
  const f = finishedYear(lib, 2026, 0);
  assert.equal(f.count, 2);
  assert.equal(f.lanes.length, 1);
  assert.equal(finishedYear(lib, 2025, 1).prevCount, 1);
});
