// Pruebas de colecciones (muestreo equilibrado de capturas y rango de años). Se ejecutan con: node --test tests/collections.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { collYearSpan, pickCollShots } from '../src/lib/collections.ts';

const mkShot = (path: string, date: number) => ({
  name: path,
  path,
  rel: path,
  date,
  w: 1920,
  h: 1080,
});

const mkGame = (ti: number, name: string, nShots: number, tint: string | null = '#00A0D8') => ({
  ti,
  name,
  tint,
  shots: Array.from({ length: nShots }, (_, k) => mkShot(`${name}/${k}.png`, 1000 + k)),
});

test('pickCollShots reparte 5 capturas entre 5 juegos distintos cuando hay suficientes juegos', () => {
  const games = [0, 1, 2, 3, 4, 5].map((i) => mkGame(i, `Juego ${i}`, 4));
  const picks = pickCollShots(games, 5, 42);
  assert.equal(picks.length, 5);
  assert.equal(new Set(picks.map((p) => p.ti)).size, 5);
  assert.equal(new Set(picks.map((p) => p.path)).size, 5);
});

test('pickCollShots mezcla todos los juegos disponibles antes de repetir juego', () => {
  const games = [mkGame(0, 'A', 3, '#E8384F'), mkGame(1, 'B', 4, '#3BB54A'), mkGame(2, 'C', 0, null)];
  const picks = pickCollShots(games, 5, 17);
  assert.equal(picks.length, 5);
  assert.equal(new Set(picks.map((p) => p.path)).size, 5);
  const fromA = picks.filter((p) => p.ti === 0).length;
  const fromB = picks.filter((p) => p.ti === 1).length;
  assert.ok(fromA >= 2 && fromB >= 2);
});

test('pickCollShots devuelve todas si hay menos que el límite y vacío si no hay capturas', () => {
  assert.deepEqual(pickCollShots([mkGame(0, 'Vacío', 0)], 5, 1), []);
  const few = pickCollShots([mkGame(0, 'Uno', 2)], 5, 1);
  assert.equal(few.length, 2);
});

test('pickCollShots es determinista para la misma semilla y varía al cambiarla', () => {
  const games = [0, 1, 2, 3, 4, 5].map((i) => mkGame(i, `G${i}`, 6));
  const a1 = pickCollShots(games, 5, 123);
  const a2 = pickCollShots(games, 5, 123);
  const b = pickCollShots(games, 5, 987654);
  assert.deepEqual(a1, a2);
  assert.notDeepEqual(
    a1.map((p) => p.path),
    b.map((p) => p.path),
  );
});

test('collYearSpan formatea vacío, un año y rango min–max', () => {
  assert.equal(collYearSpan([]), '');
  assert.equal(collYearSpan([null, null]), '');
  assert.equal(collYearSpan([2015, null, 2015]), '2015');
  assert.equal(collYearSpan([2020, 2004, null, 2011]), '2004 – 2020');
});
