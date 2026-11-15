// Pruebas de las rachas con días libres en goalTrack. Se ejecutan con: node --test tests/readgoals.test.ts
// (Node 22.18+ ejecuta TypeScript sin dependencias; no entra en el tsconfig de la app.)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  EXTRA_MIN_PLAY,
  FREE_MAX,
  FREE_WINDOW,
  goalAt,
  goalHistOf,
  goalTrack,
  localDay,
  type GoalHist,
  type GoalKey,
  type GoalTrack,
} from '../src/lib/readgoals.ts';
import { freeClose, freeDayName, freeLeft } from '../src/lib/readfmt.ts';

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
/** Vela de la última semana para `ago` días (0..6). */
const wday = (t: GoalTrack, ago: number) => t.week[6 - ago];

const vn = (plays: Plays, weeks = 10, hist: GoalHist = []) => goalTrack('vn', 60, lib('vn', plays), null, NOW, weeks, hist);

/** Racha previa de 3 días cumplidos (hace 5, 4 y 3 días) y el domingo (hace 2) a medias con 38 min: faltan 22. */
function base(): Plays {
  const p: Plays = [];
  fill(p, 5, 3, 60);
  day(p, 2, 38);
  return p;
}

test('constantes exportadas de días libres', () => {
  assert.equal(EXTRA_MIN_PLAY, 10);
  assert.equal(FREE_MAX, 2);
  assert.equal(FREE_WINDOW, 7);
});

test('pago simple en un día siguiente: salda la deuda y mantiene la racha', () => {
  const p = base();
  day(p, 1, 85); // lunes: +25 >= 22
  day(p, 0, 60);
  const t = vn(p);
  assert.equal(t.open.length, 0);
  assert.equal(t.free.length, 1);
  assert.equal(t.free[0].done, true);
  assert.equal(t.free[0].owe, 0);
  assert.equal(t.owe, 0);
  assert.equal(t.tokens, 1);
  assert.equal(t.tokenBack, T + 5);
  assert.equal(cell(t, 2).free, true);
  assert.equal(cell(t, 2).owe, false);
  assert.equal(wday(t, 1).extra, 25);
  assert.equal(wday(t, 1).gave, 22);
  assert.equal(t.streak, 6);
  assert.equal(t.paidDay, null); // se pagó ayer, no hoy
});

test('paidDay hoy: avisa el día libre saldado hoy', () => {
  const p = base();
  day(p, 1, 60);
  day(p, 0, 90); // +30 hoy
  const t = vn(p);
  assert.equal(t.open.length, 0);
  assert.equal(t.paidDay, T - 2);
  assert.equal(t.free[0].payDay, T);
  assert.equal(cell(t, 2).free, true);
  assert.equal(cell(t, 2).owe, false);
});

test('día a medias debe solo lo que faltó (y día con 0 min debe toda la meta)', () => {
  const p = base(); // 38 de 60 hace 2 días -> debe 22
  day(p, 1, 60);
  const tPartial = vn(p);
  assert.equal(tPartial.open.length, 1);
  assert.equal(tPartial.open[0].played, 38);
  assert.equal(tPartial.open[0].need, 22);
  assert.equal(tPartial.owe, 22);

  const pZero: Plays = [];
  fill(pZero, 5, 3, 60); // hace 2 días: 0 min
  day(pZero, 1, 60);
  const tZero = vn(pZero);
  assert.equal(tZero.open.length, 1);
  assert.equal(tZero.open[0].played, 0);
  assert.equal(tZero.open[0].need, 60);
  assert.equal(tZero.owe, 60);
  assert.equal(tZero.streak, 5);
});

test('un día libre recuperado poco a poco en varios días', () => {
  const p: Plays = [];
  fill(p, 6, 5, 60);
  day(p, 4, 30); // debe 30
  day(p, 3, 70); // +10 -> quedan 20
  day(p, 2, 72); // +12 -> quedan 8
  day(p, 1, 60); // +0
  const tMid = vn(p);
  assert.equal(tMid.open.length, 1);
  assert.equal(tMid.open[0].paid, 22);
  assert.equal(tMid.open[0].owe, 8);
  assert.equal(tMid.owe, 8);
  assert.equal(wday(tMid, 3).gave, 10);
  assert.equal(wday(tMid, 2).gave, 12);
  assert.equal(cell(tMid, 4).free, true);
  assert.equal(cell(tMid, 4).owe, true);

  day(p, 0, 70); // +10 -> salda los 8 restantes hoy (sobran 2)
  const tDone = vn(p);
  assert.equal(tDone.open.length, 0);
  assert.equal(tDone.owe, 0);
  assert.equal(tDone.paidDay, T - 4);
  assert.equal(wday(tDone, 0).extra, 10);
  assert.equal(wday(tDone, 0).gave, 8);
  assert.equal(cell(tDone, 4).owe, false);
  assert.equal(tDone.streak, 7);
});

test('fin de semana libre (2 seguidos) válido y reparto al más antiguo', () => {
  const p: Plays = [];
  fill(p, 6, 4, 60); // jueves, viernes, sábado cumplidos? No: hace 6..4 días cumplidos
  // Hace 3 días (sábado) y hace 2 días (domingo): 0 min cada uno (deben 60 + 60)
  day(p, 1, 130); // lunes: +70 -> paga 60 al sábado y 10 al domingo
  day(p, 0, 80);  // martes: +20 -> paga 20 más al domingo (quedan 30)
  const t = vn(p);
  assert.equal(t.tokens, 0);
  assert.equal(t.free.length, 2);
  assert.equal(t.free[0].day, T - 3);
  assert.equal(t.free[0].done, true);
  assert.equal(t.free[0].paid, 60);
  assert.equal(t.free[0].payDay, T - 1);
  assert.equal(t.free[1].day, T - 2);
  assert.equal(t.free[1].done, false);
  assert.equal(t.free[1].paid, 30);
  assert.equal(t.free[1].owe, 30);
  assert.equal(t.owe, 30);
  assert.equal(t.open.length, 1);
  assert.equal(t.streak, 7);
});

test('tercer día libre en 7 días rompe la racha', () => {
  const p: Plays = [];
  fill(p, 8, 6, 60);
  // Hace 5 días: 0 min (libre 1)
  day(p, 4, 60);
  // Hace 3 días: 0 min (libre 2)
  day(p, 2, 60);
  // Ayer (hace 1 día): 0 min -> 3er fallo en [hoy-7 .. hoy-2] -> rompe
  day(p, 0, 60);
  const t = vn(p);
  assert.equal(wday(t, 5).free !== null, true);
  assert.equal(wday(t, 3).free !== null, true);
  assert.equal(wday(t, 1).free, null);
  assert.equal(wday(t, 1).broken, true);
  assert.equal(t.streak, 1); // solo hoy tras la ruptura de ayer
  assert.equal(t.tokens, 0); // libre 1 y libre 2 siguen en los últimos 6 días
});

test('exceso antes del día libre NO cuenta (no se guarda por adelantado)', () => {
  const p: Plays = [];
  fill(p, 5, 4, 60);
  day(p, 3, 180); // +120 antes del día libre: se pierde
  day(p, 2, 40);  // debe 20
  day(p, 1, 60);
  day(p, 0, 60);
  const t = vn(p);
  assert.equal(wday(t, 3).extra, 120);
  assert.equal(wday(t, 3).gave, 0);
  assert.equal(t.open.length, 1);
  assert.equal(t.open[0].paid, 0);
  assert.equal(t.open[0].owe, 20);
});

test('plazo vencido rompe y la racha nueva cuenta desde el día siguiente', () => {
  const p = base(); // D = T - 2 (debe 22)
  fill(p, 1, -5, 61); // D+1..D+7: +1 cada día -> +7 en total (insuficiente)
  // En D+7 (domingo 25 a las 22:40) sigue abierto con left = 1
  const day7 = new Date(2026, 9, 25, 22, 40).getTime();
  const tOpen = goalTrack('vn', 60, lib('vn', p), null, day7, 10);
  assert.equal(tOpen.open.length, 1);
  assert.equal(tOpen.open[0].left, 1);
  assert.equal(tOpen.state, 'urgent');
  assert.equal(tOpen.streak, 11);

  // En D+8 (lunes 26 a las 12:00): venció el plazo, D pasa a roto y la racha es solo D+1..D+7 (7 días)
  const day8 = new Date(2026, 9, 26, 12, 0).getTime();
  const t = goalTrack('vn', 60, lib('vn', p), null, day8, 10);
  assert.equal(t.open.length, 0);
  assert.equal(t.free.length, 0);
  assert.equal(t.streak, 7);
  assert.equal(t.weeks.flat().filter((c) => c.free).length, 0);
});

test('punto fijo: al vencer un día libre antiguo, su exceso pasa al siguiente', () => {
  const p: Plays = [];
  fill(p, 12, 10, 60);
  day(p, 9, 0);   // D1 = T-9 (debe 60, vence en T-2)
  day(p, 8, 60);
  day(p, 7, 30);  // D2 = T-7 (debe 30, vence en T)
  fill(p, 6, 2, 66); // D1+3..D1+7: +30 en total; primero iban a D1
  fill(p, 1, 0, 60);
  // Al vencer D1 (T-2 < T), D1 pasa a roto y los +30 de T-6..T-2 pagan D2 por completo
  const t = vn(p);
  assert.equal(cell(t, 9).free, false);
  assert.equal(cell(t, 7).free, true);
  assert.equal(cell(t, 7).owe, false);
  assert.equal(t.open.length, 0);
  assert.equal(t.streak, 9); // T-8 .. T
});

test('ficha que vuelve a los 7 días', () => {
  const p: Plays = [];
  fill(p, 10, 8, 60);
  day(p, 7, 40); // D = T - 7 (falta pagar 20)
  fill(p, 6, 0, 60);
  // En D + 6 (= T - 1): la ficha de D aún está gastada
  const atDay6 = new Date(2026, 9, 19, 22, 40).getTime();
  const t6 = goalTrack('vn', 60, lib('vn', p), null, atDay6);
  assert.equal(t6.tokens, 1);
  assert.equal(t6.tokenBack, T); // vuelve hoy (T = D + 7)

  // En D + 7 (= T): la ficha ya volvió (tokens = 2), aunque D siga abierto como último día (left = 1)
  const t7 = vn(p);
  assert.equal(t7.tokens, 2);
  assert.equal(Number.isNaN(t7.tokenBack), true);
  assert.equal(t7.open.length, 1);
  assert.equal(t7.open[0].left, 1);
  assert.equal(t7.state, 'urgent');
});

test('partidas de menos de 10 min no pagan; 10 min sí', () => {
  const a = base();
  day(a, 1, 60, 9); // 69 min, pero el exceso es una partida de 9 min
  const ta = vn(a);
  assert.equal(ta.open[0].paid, 0);
  assert.equal(wday(ta, 1).extra, 0);

  const b = base();
  day(b, 1, 60, 10);
  const tb = vn(b);
  assert.equal(tb.open[0].paid, 10);
  assert.equal(wday(tb, 1).extra, 10);
  assert.equal(wday(tb, 1).gave, 10);
});

test('state: ok, lit, debt, tope y urgent (con prioridad urgent > tope > debt > lit/ok)', () => {
  // ok: sin deuda, hoy sin cumplir
  const pOk: Plays = [];
  fill(pOk, 3, 1, 60);
  assert.equal(vn(pOk).state, 'ok');

  // lit: sin deuda, hoy cumplido
  day(pOk, 0, 60);
  assert.equal(vn(pOk).state, 'lit');

  // debt: 1 día libre reciente (left > 2), quedan fichas
  const pDebt: Plays = [];
  fill(pDebt, 4, 2, 60);
  day(pDebt, 1, 30); // debe 30, left = 7
  day(pDebt, 0, 60);
  const tDebt = vn(pDebt);
  assert.equal(tDebt.tokens, 1);
  assert.equal(tDebt.state, 'debt');

  // tope: 2 días libres en 7 días (tokens = 0), ninguno con left <= 2
  const pTope: Plays = [];
  fill(pTope, 5, 3, 60);
  day(pTope, 2, 40); // left = 6
  day(pTope, 1, 40); // left = 7
  day(pTope, 0, 60);
  const tTope = vn(pTope);
  assert.equal(tTope.tokens, 0);
  assert.equal(tTope.state, 'tope');

  // urgent: supera a tope aunque tokens === 0
  const pUrgent: Plays = [];
  fill(pUrgent, 9, 7, 60);
  day(pUrgent, 6, 40); // deadline = T + 1 -> left = 2 (urgente)
  fill(pUrgent, 5, 2, 60);
  day(pUrgent, 1, 40); // segunda ficha gastada -> tokens = 0
  day(pUrgent, 0, 60);
  const tUrgent = vn(pUrgent);
  assert.equal(tUrgent.tokens, 0);
  assert.ok(tUrgent.urgent);
  assert.equal(tUrgent.urgent!.left, 2);
  assert.equal(tUrgent.state, 'urgent');
});

test('goalAt con historial y meta diferida (hoy la vieja, mañana la nueva)', () => {
  const hist: GoalHist = [[0, 60], [T + 1, 45]];
  assert.equal(goalAt(undefined, T, 60), 60);
  assert.equal(goalAt([], T, 60), 60);
  assert.equal(goalAt(hist, T - 5, 45), 60);
  assert.equal(goalAt(hist, T, 45), 60);
  assert.equal(goalAt(hist, T + 1, 45), 45);
  assert.deepEqual(goalHistOf({ goalHist: { vn: hist } }, 'vn'), hist);
  assert.deepEqual(goalHistOf({ goalHist: {} }, 'pc'), []);

  // Hoy (T): la meta sigue siendo 60 aunque la config ya guarde 45; con 50 min NO cumple hoy
  const p: Plays = [];
  fill(p, 3, 1, 60);
  day(p, 0, 50);
  const tToday = goalTrack('vn', 45, lib('vn', p), null, NOW, 10, hist);
  assert.equal(tToday.goal, 60);
  assert.equal(tToday.nextGoal, 45);
  assert.equal(tToday.done, false);

  // Mañana (T + 1): hoy la meta es 45 (cumple con 45), pero ayer (T) se evalúa con 60 (día libre con need = 10)
  const tomorrow = new Date(2026, 9, 21, 22, 40).getTime();
  day(p, -1, 45); // T + 1: 45 min
  const tTomorrow = goalTrack('vn', 45, lib('vn', p), null, tomorrow, 10, hist);
  assert.equal(tTomorrow.goal, 45);
  assert.equal(tTomorrow.nextGoal, 45);
  assert.equal(tTomorrow.done, true);
  assert.equal(tTomorrow.open.length, 1);
  assert.equal(tTomorrow.open[0].day, T);
  assert.equal(tTomorrow.open[0].goal, 60);
  assert.equal(tTomorrow.open[0].need, 10);
});

test('dos categorías abiertas a la vez sin mezclarse', () => {
  const pv = base();
  day(pv, 1, 72); // +12
  day(pv, 0, 66, 7); // +6 (la de 7 min no cuenta)
  const pj: Plays = [];
  fill(pj, 5, 3, 30);
  day(pj, 2, 19); // debe 11
  day(pj, 1, 36); // +6
  day(pj, 0, 33, 6); // +3 (la de 6 min no cuenta)
  const platforms = [
    { id: 'vn', titles: [{ name: 'A', gameId: 'a', plays: pv }] },
    { id: 'jvn', titles: [{ name: 'B', gameId: 'b', plays: pj }] },
  ] as any;
  const a = goalTrack('vn', 60, platforms, null, NOW);
  const b = goalTrack('jvn', 30, platforms, null, NOW);
  assert.equal(a.open[0].paid, 18);
  assert.equal(a.open[0].owe, 4);
  assert.equal(b.open[0].paid, 9);
  assert.equal(b.open[0].owe, 2);
});

test('readfmt: freeDayName, freeLeft y freeClose', () => {
  const p = base();
  day(p, 1, 60);
  const t = vn(p);
  const f = t.open[0]; // T - 2 = domingo 18 oct 2026, deadline = T + 5 = domingo 25 oct, left = 6
  assert.equal(freeDayName(f.day), 'dom 18');
  assert.equal(freeDayName(f.day, true), 'domingo 18');
  assert.equal(freeLeft(f), 'quedan 6 días');
  assert.equal(freeLeft({ ...f, left: 2 }), 'hoy y mañana');
  assert.equal(freeLeft({ ...f, left: 1 }), 'hoy');
  assert.equal(freeClose(f), 'lun 7:00');
  assert.equal(freeClose(f, true), 'lunes 26 a las 7:00');
});
