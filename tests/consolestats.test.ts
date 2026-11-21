// Días de la consola (Switch) en las cifras por fechas. Se ejecutan con: node --test tests/consolestats.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { rankPeriod, dayHeat, dayPlays, hourHistogram, titleStats, catPeriod, periodRange, yearReview } from '../src/lib/stats.ts';
import { consoleSpans } from '../src/lib/readgoals.ts';

const H = 3600000;
const at = (y: number, m: number, d: number, h = 0) => new Date(y, m - 1, d, h).getTime();
const now = at(2026, 11, 20, 20);
const sw = {
  name: 'Zelda', hidden: false, shots: [], plays: [[at(2026, 11, 3, 18), at(2026, 11, 3, 19)]] as Array<[number, number]>,
  playSeconds: 1 * 3600 + 150 * 60 + 90 * 60, sessions: 1,
  console: { seconds: 240 * 60, firstPlayed: null, lastPlayed: null, source: 'history', days: [[at(2026, 11, 5), 150], [at(2025, 3, 1), 90]] as Array<[number, number]> },
};
const lib = { platforms: [{ id: 'switch', name: 'Switch', titles: [sw] }] } as any;

test('consoleSpans: un tramo dentro del día, del largo de los minutos', () => {
  const [[a, b]] = consoleSpans(sw);
  assert.equal(b - a, 150 * 60000);
  assert.ok(a >= at(2026, 11, 5) && b <= at(2026, 11, 6));
});

test('el mes suma el tiempo de la consola pero no lo cuenta como partida', () => {
  const R = rankPeriod(lib, 'month', 0, now, 0);
  assert.equal(R.ms, 1 * H + 150 * 60000);
  assert.equal(R.n, 1);
  assert.equal(R.days, 2);
  const C = catPeriod(lib, 0, periodRange('month', 0, now));
  assert.equal(C.ms, R.ms);
  assert.equal(C.longest?.ms, 1 * H);
});

test('calendario y día: la consola aparece sin hora', () => {
  assert.equal(dayHeat(lib, 0, 2026)[Math.round((at(2026, 11, 5) - at(2026, 1, 1)) / 864e5)], 150 * 60000);
  const d = dayPlays(lib, at(2026, 11, 5));
  assert.equal(d.length, 1);
  assert.equal(d[0].console, true);
  assert.equal(d[0].ms, 150 * 60000);
});

test('las horas del día no incluyen la consola', () => {
  const h = hourHistogram(lib, 0, periodRange('all', 0, now));
  assert.equal(h.reduce((a, x) => a + x, 0), 1 * H);
});

test('ficha y resumen del año cuentan los días de la consola', () => {
  assert.equal(titleStats(sw as any).days, 3);
  assert.equal(yearReview(lib, 2025, new Set<string>(), now).ms, 90 * 60000);
});
