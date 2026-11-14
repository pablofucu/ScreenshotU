import test from 'node:test';
import assert from 'node:assert/strict';
import { parseCloseStatus, remotePlayingFromSession } from '../src/lib/remote/session_close.ts';

test('parseCloseStatus interpreta asked, forced y juego ya cerrado', () => {
  assert.equal(parseCloseStatus({ closed: 'asked' }), 'asked');
  assert.equal(parseCloseStatus({ closed: 'forced' }), 'forced');
  assert.equal(parseCloseStatus({ closed: 'none' }), 'none');
  assert.equal(parseCloseStatus(null), 'none');
  assert.equal(parseCloseStatus({}), 'none');
});

test('remotePlayingFromSession reconstruye partida activa o limpia al estar idle', () => {
  const now = 1_000_000;
  assert.deepEqual(
    remotePlayingFromSession({ state: 'playing', game: { id: 'g1' }, app: 'Juego actual', elapsedSeconds: 30 }, now),
    { active: true, id: 'g1', since: now - 30_000, app: 'Juego actual' },
  );
  assert.deepEqual(
    remotePlayingFromSession({ state: 'starting', game: { id: 'g2' } }, now),
    { active: true, id: 'g2', since: now, app: null },
  );
  assert.deepEqual(
    remotePlayingFromSession({ state: 'idle', app: 'Desktop' }, now),
    { active: false, app: '' },
  );
});
