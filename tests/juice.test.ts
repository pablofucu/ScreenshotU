// Pruebas del motor de cálculo sonoro y paralaje (src/lib/juice.ts). Ejecutar con: node --test tests/juice.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { moveTone, panFor, parallaxFor } from '../src/lib/juice.ts';

test('moveTone sube suavemente por la escala con ráfagas rápidas y se acota al máximo', () => {
  const base = moveTone();
  const s1 = moveTone({ streak: 1 });
  const s3 = moveTone({ streak: 3 });
  const s6 = moveTone({ streak: 6 });
  const s99 = moveTone({ streak: 99 });
  assert.equal(base, 784);
  assert.ok(s1 > base);
  assert.ok(s3 > s1);
  assert.ok(s6 > s3);
  assert.equal(s99, s6);
  assert.ok(s6 <= 1250);
});

test('moveTone distingue dirección positiva (derecha/abajo) frente a negativa (izquierda/arriba)', () => {
  const right = moveTone({ dx: 1 });
  const left = moveTone({ dx: -1 });
  const down = moveTone({ dy: 1 });
  const up = moveTone({ dy: -1 });
  assert.ok(right > 784);
  assert.ok(left < 784);
  assert.ok(down > 784);
  assert.ok(up < 784);
});

test('panFor centra en 0, sesga según dx y se acota en [-0.28, 0.28]', () => {
  assert.equal(panFor(0, 0.5), 0);
  assert.ok(panFor(1, 0.5) > 0);
  assert.ok(panFor(-1, 0.5) < 0);
  assert.equal(panFor(1, 1), 0.28);
  assert.equal(panFor(-1, 0), -0.28);
  assert.equal(panFor(1, 99), 0.28);
  assert.equal(panFor(-1, -5), -0.28);
});

test('parallaxFor da 0 en centro sin scroll, opuesto al foco, y bokeh se mueve más que ghosts', () => {
  const c = parallaxFor(0.5, 0.5, 0);
  assert.deepEqual(c, { gx: 0, gy: 0, bx: 0, by: 0 });

  const tr = parallaxFor(1, 0, 0);
  assert.ok(tr.gx < 0 && tr.gy > 0);
  assert.ok(Math.abs(tr.bx) > Math.abs(tr.gx));
  assert.ok(Math.abs(tr.by) > Math.abs(tr.gy));

  const sc = parallaxFor(0.5, 0.5, 900);
  assert.equal(sc.gx, 0);
  assert.equal(sc.gy, -6);
  assert.equal(sc.by, -14);

  const clamped = parallaxFor(5, -3, 50000);
  assert.deepEqual(clamped, { gx: -12, gy: 1, bx: -26, by: 0 });
});
