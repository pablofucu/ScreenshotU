// Pruebas de las cifras de Panorama (una categoría). Se ejecutan con: node --test tests/pano.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { catPeriod, catWeeks, catMilestone, catShots, hourHistogram, dayHeat, rankPeriod, periodRange } from '../src/lib/stats.ts';

const H = 3600000;
const at = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min).getTime();
const now = at(2026, 11, 12, 20); // jueves
const t = (name: string, plays: Array<[number, number]>, extra: Record<string, unknown> = {}) =>
  ({ name, hidden: false, plays, playSeconds: plays.reduce((a, p) => a + (p[1] - p[0]) / 1000, 0), sessions: plays.length, shots: [], ...extra });
const enc = (ms: number) => ms - new Date(ms).getTimezoneOffset() * 60000; // fecha de captura: hora local como UTC
const lib = {
  platforms: [
    { id: 'pc', name: 'Juegos', titles: [
      t('A', [[at(2026, 11, 10, 22, 30), at(2026, 11, 11, 1)], [at(2026, 11, 3, 10), at(2026, 11, 3, 11)]], {
        shots: [{ path: 'a1', date: enc(at(2026, 11, 10, 23)) }, { path: 'a2', date: enc(at(2026, 11, 11, 0, 30)) }, { path: 'old', date: enc(at(2025, 1, 1)) }],
      }),
      t('B', [[at(2026, 1, 5, 15), at(2026, 1, 5, 20)]], { playSeconds: 98 * 3600 }),
      t('Oculto', [[at(2026, 11, 11, 10), at(2026, 11, 11, 18)]], { hidden: true }),
    ] },
    { id: 'switch', name: 'Switch', titles: [t('S', [[at(2026, 11, 11, 9), at(2026, 11, 11, 19)]])] },
  ],
} as any;

test('catPeriod: tiempo igual a la Clasificación y partida más larga de su categoría', () => {
  const C = catPeriod(lib, 0, 'month', 0, now);
  assert.equal(C.ms, rankPeriod(lib, 'month', 0, now, 0).ms);
  assert.equal(C.ms, 3.5 * H);
  assert.equal(C.games, 1);
  assert.equal(C.longest?.t.name, 'A');
  assert.equal(C.longest?.ms, 2.5 * H);
  assert.equal(catPeriod(lib, 0, 'year', 0, now).longest?.t.name, 'B');
});

test('catWeeks: ocho semanas, días y juegos de esta semana sin mezclar categorías', () => {
  const W = catWeeks(lib, 0, 8, now);
  assert.equal(W.weeks.length, 8);
  assert.equal(W.weeks[7], 2.5 * H);
  assert.equal(W.weeks[6], 1 * H);
  assert.equal(W.days, 2); // martes y miércoles (cruza medianoche)
  assert.equal(W.games, 1);
});

test('catMilestone: el más cerca de su hito', () => {
  const M = catMilestone(lib, 0);
  assert.equal(M?.t.name, 'B');
  assert.equal(M?.h, 100);
  assert.equal(Math.round(M!.left), 2);
});

test('catShots: solo las del periodo, favoritas primero', () => {
  const S = catShots(lib, 0, periodRange('week', 0, now), new Set(['a1']));
  assert.equal(S.count, 2);
  assert.deepEqual(S.pick.map((s) => s.path), ['a1', 'a2']);
});

test('hourHistogram: 24 cubos que suman el tiempo del periodo', () => {
  for (const kind of ['week', 'month', 'year'] as const) {
    const r = periodRange(kind, 0, now);
    const h = hourHistogram(lib, 0, r);
    assert.equal(h.length, 24);
    assert.equal(h.reduce((a, b) => a + b, 0), catPeriod(lib, 0, kind, 0, now).ms);
  }
  const w = hourHistogram(lib, 0, periodRange('week', 0, now));
  assert.equal(w[22], 0.5 * H);
  assert.equal(w[23], H);
  assert.equal(w[0], H);
});

test('dayHeat: un valor por día del año, partido en la medianoche', () => {
  const d = dayHeat(lib, 0, 2026);
  assert.equal(d.length, 365);
  assert.equal(d[4], 5 * H); // 5 ene
  const nov10 = Math.round((at(2026, 11, 10) - at(2026, 1, 1)) / 86400000);
  assert.equal(d[nov10], 1.5 * H);
  assert.equal(d[nov10 + 1], 1 * H);
  assert.equal(dayHeat(lib, 1, 2026).reduce((a, b) => a + b, 0), 10 * H);
});
