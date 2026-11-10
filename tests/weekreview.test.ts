// Pruebas del resumen semanal. Se ejecutan con: node --test tests/weekreview.test.ts
// (Node 22.18+ ejecuta TypeScript sin dependencias; no entra en el tsconfig de la app.)
process.env.TZ = 'Europe/Madrid'; // el cambio de hora de octubre depende de la zona
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { weekReview, shotInRange, nextMilestone, MILESTONES_H } from '../src/lib/stats.ts';

const H = 3600000, MIN = 60000;
// «Hoy» es el miércoles 11 de noviembre de 2026, 12:00. Esta semana: lunes 9 a domingo 15.
const NOW = new Date(2026, 10, 11, 12, 0).getTime();
const at = (m: number, d: number, h = 0, min = 0) => new Date(2026, m - 1, d, h, min).getTime();
const noFavs = { has: () => false };

type Plays = Array<[number, number]>;
function title(name: string, plays: Plays, extra: Record<string, unknown> = {}) {
  const ms = plays.reduce((a, p) => a + p[1] - p[0], 0);
  return { name, gameId: 'g-' + name, hidden: false, plays, shots: [], playSeconds: ms / 1000, sessions: plays.length, firstPlayed: plays[0]?.[0] ?? null, ...extra };
}
const plat = (id: string, titles: any[]) => ({ id, name: id.toUpperCase(), titles });
const lib = (...p: any[]) => ({ platforms: p }) as any;
const block = (r: ReturnType<typeof weekReview>, id: string, l: any) => r.blocks.find((b) => l.platforms[b.pi].id === id)!;

test('partida que cruza la medianoche del domingo reparte el tiempo y cuenta en la semana donde empezó', () => {
  // Domingo 8 nov 23:00 a lunes 9 nov 01:00: 1 h en la semana pasada, 1 h en esta.
  const l = lib(plat('pc', [title('A', [[at(11, 8, 23), at(11, 9, 1)]])]));
  const w = weekReview(l, 0, noFavs, null, {}, NOW);
  const b = block(w, 'pc', l);
  assert.equal(b.ms, H);
  assert.equal(b.n, 0);
  assert.equal(b.perDay[0], H);
  const p = weekReview(l, -1, noFavs, null, {}, NOW);
  const bp = block(p, 'pc', l);
  assert.equal(bp.ms, H);
  assert.equal(bp.n, 1);
  assert.equal(bp.perDay[6], H);
  assert.equal(w.prevMs, H);
});

test('cambio de hora de octubre: la semana del 19 al 25 dura 7 días locales y el reparto usa las horas reales', () => {
  // Sábado 24 oct 22:00 a domingo 25 oct 06:00 local: se atrasa 1 h, son 9 h reales.
  const l = lib(plat('pc', [title('A', [[at(10, 24, 22), at(10, 25, 6)]])]));
  const w = weekReview(l, -3, noFavs, null, {}, NOW);
  assert.equal(w.range.from, at(10, 19));
  assert.equal(w.range.to, at(10, 26));
  const b = block(w, 'pc', l);
  assert.equal(b.ms, 9 * H);
  assert.equal(b.perDay[5], 2 * H);
  assert.equal(b.perDay[6], 7 * H);
  assert.equal(w.activeDays, 2);
});

test('Switch: los dias de consola se suman al tiempo, a las barras y a «hasta»', () => {
  const t = title('Zelda', [[at(11, 10, 20), at(11, 10, 21)]], {
    console: { seconds: 0, firstPlayed: null, lastPlayed: null, source: 'history', days: [[at(11, 11), 90], [at(11, 2), 30]] },
  });
  const l = lib(plat('switch', [t]));
  const w = weekReview(l, 0, noFavs, null, {}, NOW);
  const b = block(w, 'switch', l);
  assert.equal(b.consoleMs, 90 * MIN);
  assert.equal(b.ms, H + 90 * MIN);
  assert.equal(b.perDay[1], H);
  assert.equal(b.perDay[2], 90 * MIN);
  assert.equal(b.consoleUntil, at(11, 11));
  assert.equal(w.activeDays, 2);
  // La semana anterior suma los 30 min del día 2.
  assert.equal(w.prevMs, 30 * MIN);
  // Una categoría que no es Switch no tiene campos de consola.
  const l2 = lib(plat('pc', [title('A', [[at(11, 10, 20), at(11, 10, 21)]])]));
  assert.equal(block(weekReview(l2, 0, noFavs, null, {}, NOW), 'pc', l2).consoleMs, undefined);
});

test('hito cruzado durante la semana', () => {
  // Total 26 h: 5 h antes de la semana... antes tenía 21 h, esta semana suma 5 h y cruza las 25 h.
  const old: Plays = [[at(10, 1, 10), at(10, 1, 10) + 21 * H]];
  const cur: Plays = [[at(11, 10, 10), at(11, 10, 15)]];
  const l = lib(plat('pc', [title('A', [...old, ...cur]), title('B', [[at(11, 10, 10), at(11, 10, 12)]])]));
  const w = weekReview(l, 0, noFavs, null, {}, NOW);
  const b = block(w, 'pc', l);
  assert.deepEqual(b.milestones, [{ ti: 0, h: 25 }]);
  assert.ok(w.highlights.some((h) => h.kind === 'milestone' && h.ti === 0 && h.value === 25));
  // Ya estaba por encima: no se cruza.
  const l2 = lib(plat('pc', [title('A', [[at(10, 1, 10), at(10, 1, 10) + 30 * H], ...cur])]));
  assert.deepEqual(block(weekReview(l2, 0, noFavs, null, {}, NOW), 'pc', l2).milestones, []);
});

test('nextMilestone', () => {
  assert.deepEqual(MILESTONES_H, [10, 25, 50, 100, 200]);
  assert.equal(nextMilestone({ playSeconds: 9 * 3600 } as any)!.h, 10);
  assert.equal(nextMilestone({ playSeconds: 10 * 3600 } as any)!.h, 25);
  assert.equal(nextMilestone({ playSeconds: 300 * 3600 } as any), null);
});

test('capturas en el borde del lunes 00:00 (fecha local codificada como UTC)', () => {
  const enc = (ms: number) => ms - new Date(ms).getTimezoneOffset() * 60000;
  const from = at(11, 9), to = at(11, 16);
  assert.equal(shotInRange({ date: enc(from) }, from, to), true);
  assert.equal(shotInRange({ date: enc(from) - 1 }, from, to), false);
  assert.equal(shotInRange({ date: enc(to) - 1 }, from, to), true);
  assert.equal(shotInRange({ date: enc(to) }, from, to), false);
  const shot = (ms: number, path: string) => ({ path, date: enc(ms), mtime: 0, width: 1, height: 1 });
  const t = title('A', [[at(11, 10, 10), at(11, 10, 11)]], { shots: [shot(from, 'lun'), shot(from - 1, 'dom'), shot(to, 'sig'), shot(at(11, 12), 'mie')] });
  const l = lib(plat('pc', [t]));
  const b = block(weekReview(l, 0, noFavs, null, {}, NOW), 'pc', l);
  assert.equal(b.shots, 2);
  assert.equal(b.shotPick!.path, 'mie');
  // Una favorita gana a una más reciente.
  const b2 = block(weekReview(l, 0, { has: (p: string) => p === 'lun' }, null, {}, NOW), 'pc', l);
  assert.equal(b2.shotPick!.path, 'lun');
});

test('los juegos ocultos no cuentan', () => {
  const l = lib(plat('pc', [title('A', [[at(11, 10, 10), at(11, 10, 11)]]), title('Oculto', [[at(11, 10, 12), at(11, 10, 15)]], { hidden: true })]));
  const w = weekReview(l, 0, noFavs, null, {}, NOW);
  const b = block(w, 'pc', l);
  assert.equal(b.ms, H);
  assert.equal(b.games, 1);
  assert.deepEqual(b.top.map((r) => r.ti), [0]);
  assert.equal(w.ms, H);
});

test('un bloque por categoria y ningun top mezcla pi', () => {
  const l = lib(
    plat('pc', [title('A', [[at(11, 10, 10), at(11, 10, 12)]]), title('B', [[at(11, 11, 10), at(11, 11, 11)]])]),
    plat('switch', [title('C', [[at(11, 10, 10), at(11, 10, 14)]])]),
    plat('vn', [title('D', [[at(11, 12, 10), at(11, 12, 11)]])]),
    plat('jvn', []),
  );
  const w = weekReview(l, 0, noFavs, null, {}, NOW);
  assert.equal(w.blocks.length, 3);
  assert.deepEqual(w.blocks.map((b) => b.pi), [0, 1, 2]);
  for (const b of w.blocks) {
    for (const r of b.top) assert.ok(r.ti < l.platforms[b.pi].titles.length);
    assert.equal(b.kind, b.pi === 2 ? 'read' : 'play');
  }
  assert.deepEqual(block(w, 'pc', l).top.map((r) => r.ti), [0, 1]);
  assert.equal(w.ms, 2 * H + H + 4 * H + H);
  assert.ok(w.highlights.every((h) => w.blocks.some((b) => b.pi === h.pi)));
});

test('metas y racha de la categoria vn; una categoria sin meta da goal indefinido', () => {
  // vn: 40 min lunes a miercoles (semana actual) con meta de 30; se leyó a las 10:00.
  const plays: Plays = [];
  for (let d = 9; d <= 11; d++) plays.push([at(11, d, 10), at(11, d, 10) + 40 * MIN]);
  const l = lib(plat('vn', [title('N', plays)]), plat('pc', [title('A', [[at(11, 10, 10), at(11, 10, 11)]])]));
  const w = weekReview(l, 0, noFavs, null, { vn: 30 }, NOW);
  const v = block(w, 'vn', l);
  assert.equal(v.goal!.met, 3);
  assert.equal(v.goal!.of, 7);
  assert.equal(v.goal!.streak, 3);
  assert.deepEqual(v.goal!.perDay, [1, 1, 1, 0, 0, 0, 0]);
  assert.equal(block(w, 'pc', l).goal, undefined);
  // Sin meta configurada tampoco hay goal en vn.
  assert.equal(block(weekReview(l, 0, noFavs, null, {}, NOW), 'vn', l).goal, undefined);
  // La semana pasada no tiene lecturas: no hay bloque vn.
  assert.equal(weekReview(l, -1, noFavs, null, { vn: 30 }, NOW).blocks.length, 0);
});

test('racha al cerrar una semana pasada y juegos empezados, terminados y récord', () => {
  // Lunes 2 a domingo 8: lectura los 7 días.
  const plays: Plays = [];
  for (let d = 2; d <= 8; d++) plays.push([at(11, d, 10), at(11, d, 10) + 40 * MIN]);
  const a = title('A', [[at(10, 1, 10), at(10, 1, 11)], [at(11, 4, 10), at(11, 4, 13)]], { finishedMs: at(11, 5, 12), firstPlayed: at(10, 1, 10) });
  const b = title('B', [[at(11, 3, 10), at(11, 3, 11)]]);
  const l = lib(plat('vn', [title('N', plays)]), plat('pc', [a, b]));
  const w = weekReview(l, -1, noFavs, null, { vn: 30 }, NOW);
  assert.equal(block(w, 'vn', l).goal!.met, 7);
  assert.equal(block(w, 'vn', l).goal!.streak, 7);
  const pc = block(w, 'pc', l);
  assert.deepEqual(pc.started, [1]);
  assert.deepEqual(pc.finished, [0]);
  assert.equal(pc.longest!.ti, 0);
  const kinds = w.highlights.map((h) => h.kind).sort();
  assert.deepEqual(kinds, ['finished', 'record', 'streak']);
});
