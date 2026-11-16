// Pruebas de los terminados del año. Se ejecutan con: node --test tests/finyear.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { finishedYear } from '../src/lib/stats.ts';

const at = (y: number, m: number, d: number) => new Date(y, m - 1, d, 12).getTime();
const t = (name: string, finishedMs: number | null, extra: Record<string, unknown> = {}) =>
  ({ name, hidden: false, status: 'finished', finishedMs, playSeconds: 3600, plays: [], shots: [], ...extra });
const lib = {
  platforms: [
    { id: 'pc', name: 'Juegos', titles: [t('A', at(2026, 3, 2)), t('B', at(2026, 3, 1)), t('C', at(2025, 7, 1)), t('Oculto', at(2026, 5, 1), { hidden: true })] },
    { id: 'vn', name: 'Novelas visuales', titles: [t('Sin fecha', null), t('Jugando', at(2026, 6, 1), { status: 'playing' })] },
    { id: 'switch', name: 'Switch', titles: [t('D', at(2026, 11, 10)), t('E', at(2025, 2, 2))] },
  ],
} as any;

test('un carril por categoría con terminados, meses ordenados', () => {
  const F = finishedYear(lib, 2026);
  assert.deepEqual(F.lanes.map((l) => l.id), ['pc', 'switch']);
  assert.equal(F.count, 3);
  assert.deepEqual(F.lanes[0].months[2].map((r) => r.t.name), ['B', 'A']);
  assert.equal(F.lanes[1].months[10][0].ms, 3600000);
});

test('año anterior, último terminado y años', () => {
  const F = finishedYear(lib, 2026);
  assert.equal(F.prevCount, 2);
  assert.equal(F.last?.t.name, 'D');
  assert.equal(F.last?.pi, 2);
  assert.deepEqual(F.years, [2025, 2026]);
});

test('año sin terminados', () => {
  const F = finishedYear(lib, 2024);
  assert.equal(F.count, 0);
  assert.equal(F.last, null);
  assert.equal(F.lanes.length, 0);
});
