import test from 'node:test';
import assert from 'node:assert/strict';
import { fmtMin } from '../src/lib/readfmt.ts';

test('fmtMin largo', () => {
  assert.equal(fmtMin(45, true), '45 min');
  assert.equal(fmtMin(60, true), '1 h');
  assert.equal(fmtMin(65, true), '1 h 05 min');
  assert.equal(fmtMin(90, true), '1 h 30 min');
});
test('fmtMin corto', () => {
  assert.equal(fmtMin(45), '45m');
  assert.equal(fmtMin(120), '2h');
  assert.equal(fmtMin(65), '1h 05m');
});
