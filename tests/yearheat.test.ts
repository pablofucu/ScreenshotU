// Mapa del año y racha de jugar. Se ejecutan con: node --test tests/yearheat.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { yearHeat, playStreak } from '../src/lib/stats.ts';

const H = 3600000;
const at = (y: number, m: number, d: number, h = 0) => new Date(y, m - 1, d, h).getTime();
const play = (y: number, m: number, d: number, h = 18, len = 1): [number, number] => [at(y, m, d, h), at(y, m, d, h + len)];
const title = (name: string, plays: Array<[number, number]>) => ({ name, hidden: false, shots: [], plays, playSeconds: 0, sessions: plays.length });
const lib = {
  platforms: [
    { id: 'pc', name: 'PC', titles: [title('A', [play(2026, 1, 1), play(2026, 3, 10), play(2026, 3, 11), play(2026, 3, 12)])] },
    { id: 'switch', name: 'Switch', titles: [title('B', [play(2026, 1, 1, 20, 2), play(2026, 11, 19), play(2026, 11, 20)])] },
  ],
} as any;

test('yearHeat suma por día y separa por categoría', () => {
  const h = yearHeat(lib, 2026);
  assert.equal(h.total.length, 365);
  assert.equal(h.total[0], 3 * H);
  assert.equal(h.by.get(0)![0], 1 * H);
  assert.equal(h.by.get(1)![0], 2 * H);
  assert.equal(h.total.reduce((a, b) => a + b, 0), 8 * H);
});

test('playStreak: actual hasta hoy o ayer, mejor de siempre', () => {
  const s = playStreak(lib, at(2026, 11, 20, 22));
  assert.equal(s.cur, 2);
  assert.ok(s.playedToday);
  assert.equal(s.best.n, 3);
  assert.equal(s.best.from, at(2026, 3, 10));
  const t = playStreak(lib, at(2026, 11, 21, 10));
  assert.equal(t.cur, 2);
  assert.ok(!t.playedToday);
  assert.equal(playStreak(lib, at(2026, 11, 23, 10)).cur, 0);
  assert.equal(playStreak(lib, at(2026, 11, 20, 22), 1).best.n, 2);
});
