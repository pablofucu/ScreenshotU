// Pruebas del rescate de racha en goalTrack. Se ejecutan con: node --test tests/readgoals.test.ts
// (Node 22.18+ ejecuta TypeScript sin dependencias; no entra en el tsconfig de la app.)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { goalTrack, localDay, type GoalKey, type GoalTrack } from '../src/lib/readgoals.ts';

// «Hoy» es el martes 20 de octubre de 2026, 22:40; las partidas se colocan de día (entre las 10:00 y las 22:00).
const NOW = new Date(2026, 9, 20, 22, 40).getTime();
const T = localDay(NOW);
const at = (ago: number, h: number, m = 0) => new Date(2026, 9, 20 - ago, h, m).getTime();

/** Partidas de un día (hace `ago` días): una por cada duración, separadas por horas para que no se unan. */
type Plays = Array<[number, number]>;
function day(plays: Plays, ago: number, ...durs: number[]) {
  durs.forEach((d, i) => plays.push([at(ago, 10 + i * 3), at(ago, 10 + i * 3) + d * 60000]));
}
/** Días `from`..`to` (hace N días, de más a menos antiguo) con una sola partida de `min` minutos. */
function fill(plays: Plays, from: number, to: number, min: number) {
  for (let a = from; a >= to; a--) day(plays, a, min);
}
function lib(key: GoalKey, plays: Plays) {
  return [{ id: key, titles: [{ name: 'Novela', gameId: 'g-' + key, plays }] }] as any;
}
/** Casilla de un día en el mapa (hace `ago` días). */
function cell(t: GoalTrack, ago: number) {
  const d = T - ago;
  const first = T - ((T + 3) % 7) - (t.weeks.length - 1) * 7;
  return t.weeks[Math.floor((d - first) / 7)][(d - first) % 7];
}

const vn = (plays: Plays, weeks = 10) => goalTrack('vn', 60, lib('vn', plays), null, NOW, weeks);

/** Racha previa de 3 días cumplidos (hace 5, 4 y 3 días) y el domingo (hace 2) fallado con 38 min: faltan 22. */
function base(): Plays {
  const p: Plays = [];
  fill(p, 5, 3, 60);
  day(p, 2, 38);
  return p;
}

test('pago simple: un día de la ventana cubre el déficit', () => {
  const p = base();
  day(p, 1, 85); // lunes: 25 de extra >= 22
  day(p, 0, 60);
  const t = vn(p);
  assert.equal(t.rescue, null);
  assert.equal(cell(t, 2).rescued, true);
  assert.equal(t.streak, 6); // 3 + domingo reparado + lunes + martes
  assert.equal(t.paidDay, null); // se pagó ayer, no hoy
});

test('pago hoy: se avisa una vez con paidDay', () => {
  const p = base();
  day(p, 1, 60);
  day(p, 0, 90); // +30
  const t = vn(p);
  assert.equal(t.rescue, null);
  assert.equal(t.paidDay, T - 2);
  assert.equal(cell(t, 2).rescued, true);
});

test('pago parcial con 2 días: 12 + 6 = 18 de 22 y la racha sigue pendiente', () => {
  const p = base();
  day(p, 1, 72); // +12
  day(p, 0, 66, 7); // +6; la de 7 min no cuenta
  const t = vn(p);
  assert.ok(t.rescue);
  assert.equal(t.rescue!.need, 22);
  assert.equal(t.rescue!.deficit, 22);
  assert.equal(t.rescue!.got, 18);
  assert.equal(t.rescue!.gotBefore, 12);
  assert.equal(t.rescue!.gotToday, 6);
  assert.equal(t.rescue!.next, 64);
  assert.equal(t.rescue!.plays.length, 3);
  assert.deepEqual(t.rescue!.plays.map((x) => x.counts), [true, true, false]);
  assert.equal(t.streak, 6); // pendiente: no se rompe en pantalla
  assert.equal(cell(t, 2).rescued, false);
  assert.equal(t.rescue!.closeAt, new Date(2026, 9, 22, 7, 0).getTime());
});

test('la ventana cierra sin pagar: se rompe y la racha vuelve a empezar tras el día fallado', () => {
  const p = base();
  day(p, 1, 61);
  day(p, 0, 61);
  // Cuatro días después (jueves 22 a las 7:00 ya pasó): hoy es el día 4 tras el fallo.
  const later = new Date(2026, 9, 23, 12, 0).getTime();
  const t = goalTrack('vn', 60, lib('vn', p), null, later, 10);
  assert.equal(t.rescue, null);
  assert.equal(t.streak, 0);
  assert.equal(t.weeks.flat().filter((c) => c.rescued).length, 0);
});

test('un fallo de D-1 impide el rescate de D', () => {
  const p: Plays = [];
  fill(p, 6, 4, 60);
  day(p, 3, 40); // D-1 fallado
  day(p, 2, 38); // D
  day(p, 1, 120);
  day(p, 0, 120);
  const t = vn(p);
  assert.equal(t.rescue, null);
  assert.equal(cell(t, 2).rescued, false);
});

test('racha previa de 2 días: no es rescatable', () => {
  const p: Plays = [];
  day(p, 5, 20); // un día flojo antes
  fill(p, 4, 3, 60); // solo 2 días cumplidos antes de D
  day(p, 2, 38);
  day(p, 1, 120);
  day(p, 0, 120);
  const t = vn(p);
  assert.equal(t.rescue, null);
  assert.equal(cell(t, 2).rescued, false);
});

test('una partida de 9 min no suma al extra; una de 10 min sí', () => {
  const a = base();
  day(a, 1, 60, 9); // lunes 69 min, pero solo 9 min en partida corta
  const ta = vn(a);
  assert.ok(ta.rescue);
  assert.equal(ta.rescue!.got, 0);
  const b = base();
  day(b, 1, 60, 10);
  const tb = vn(b);
  assert.equal(tb.rescue!.got, 10);
});

test('tope de 2 rescates en 30 días y el segundo exige 1,5x', () => {
  const p: Plays = [];
  fill(p, 45, 38, 60);
  day(p, 37, 38); // D1
  day(p, 36, 90); // +30 >= 22: pagado
  fill(p, 35, 34, 60);
  day(p, 33, 38); // D2 (segundo en 30 días: exige 33)
  day(p, 32, 100); // +40 >= 33: pagado
  fill(p, 31, 21, 60);
  day(p, 20, 38); // D3: ya hay 2 pagados en los 30 días anteriores
  fill(p, 19, 0, 120);
  const t = vn(p, 12);
  assert.equal(cell(t, 37).rescued, true);
  assert.equal(cell(t, 33).rescued, true);
  assert.equal(cell(t, 20).rescued, false);
  assert.equal(t.rescue, null);
});

test('segundo rescate: necesita el déficit x 1,5', () => {
  const p: Plays = [];
  fill(p, 14, 10, 60);
  day(p, 9, 38); // D1
  day(p, 8, 90); // pagado
  fill(p, 7, 3, 60);
  day(p, 2, 38); // D2: déficit 22, hay que reunir 33
  day(p, 1, 85); // +25: no basta
  const t = vn(p);
  assert.ok(t.rescue);
  assert.equal(t.rescue!.repeat, true);
  assert.equal(t.rescue!.need, 33);
  assert.equal(t.rescue!.got, 25);
  assert.equal(t.rescue!.used, 2);
  assert.equal(cell(t, 9).rescued, true);
  assert.equal(cell(t, 2).rescued, false);
});

test('dos categorías pendientes a la vez, cada una por su cuenta', () => {
  const pv = base();
  day(pv, 1, 72);
  day(pv, 0, 66, 7);
  const pj: Plays = [];
  fill(pj, 5, 3, 30);
  day(pj, 2, 19);
  day(pj, 1, 36); // +6
  day(pj, 0, 33, 6); // +3; la de 6 min no cuenta
  const platforms = [
    { id: 'vn', titles: [{ name: 'A', gameId: 'a', plays: pv }] },
    { id: 'jvn', titles: [{ name: 'B', gameId: 'b', plays: pj }] },
  ] as any;
  const a = goalTrack('vn', 60, platforms, null, NOW);
  const b = goalTrack('jvn', 30, platforms, null, NOW);
  assert.equal(a.rescue!.got, 18);
  assert.equal(a.rescue!.need, 22);
  assert.equal(b.rescue!.got, 9);
  assert.equal(b.rescue!.need, 11);
  assert.equal(b.rescue!.next, 32);
  assert.equal(a.rescue!.key, 'vn');
  assert.equal(b.rescue!.key, 'jvn');
});
