// Pruebas del reparto fijo de colores arcoíris y orden visual de secciones. Se ejecutan con: node --test tests/sections.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RAINBOW } from '../src/lib/icons.ts';
import { GOAL_COL, GOAL_KEYS } from '../src/lib/readgoals.ts';
import {
  SPACE_IDS,
  firstHomePlat,
  orderedPlatforms,
  sectionColor,
  sectionRainbowIdx,
  sectionRainbowT,
} from '../src/lib/sections.ts';

test('los siete accesos principales ocupan las 7 ranuras del arcoíris de la guía en orden exacto', () => {
  const order = ['switch', 'pc', 'vn', 'jvn', ...SPACE_IDS];
  assert.deepEqual(order.map(sectionRainbowIdx), [0, 1, 2, 3, 4, 5, 6]);
  assert.equal(sectionColor('switch'), '#E8384F');
  assert.equal(sectionColor('pc'), '#F27A1A');
  assert.equal(sectionColor('vn'), '#F2B807');
  assert.equal(sectionColor('jvn'), '#3BB54A');
  assert.equal(sectionColor('album'), '#00A0D8');
  assert.equal(sectionColor('stats'), '#3F63E0');
  assert.equal(sectionColor('logros'), '#9A4BD8');
  assert.equal(sectionRainbowT('switch'), 0);
  assert.equal(sectionRainbowT('logros'), 1);
});

test('GOAL_COL y GOAL_KEYS coinciden exactamente con el color y orden arcoíris de cada categoría', () => {
  assert.deepEqual(GOAL_KEYS, ['switch', 'pc', 'vn', 'jvn']);
  for (const k of GOAL_KEYS) {
    assert.equal(GOAL_COL[k].c, RAINBOW[sectionRainbowIdx(k)].c);
  }
});

test('una carpeta personalizada recibe una ranura arcoíris válida y determinista', () => {
  const a = sectionRainbowIdx('ps2');
  const b = sectionRainbowIdx('ps2');
  assert.equal(a, b);
  assert.ok(a >= 0 && a < RAINBOW.length);
  assert.equal(sectionColor('ps2'), RAINBOW[a].c);
});

test('orderedPlatforms ordena Inicio sin alterar los índices reales ni omitir carpetas extra', () => {
  const libPlats = [
    { id: 'pc', name: 'PC' },
    { id: 'switch', name: 'Switch' },
    { id: 'vn', name: 'VN' },
    { id: 'jvn', name: 'JVN' },
    { id: 'wii', name: 'Wii' },
  ];
  const ordered = orderedPlatforms(libPlats);
  assert.deepEqual(ordered.map((x) => [x.p.id, x.i]), [
    ['switch', 1],
    ['pc', 0],
    ['vn', 2],
    ['jvn', 3],
    ['wii', 4],
  ]);
  assert.equal(firstHomePlat(libPlats), 1);
  assert.equal(firstHomePlat([]), 0);
});
