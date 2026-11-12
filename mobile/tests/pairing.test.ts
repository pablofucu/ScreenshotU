// Pruebas de la lógica pura del emparejamiento. Se ejecutan con: node --test mobile/tests/pairing.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { artemisUrl, migrateServer, orderAddrs, parseQrV2, splitAddr, toStored } from '../src/lib/pairing.ts';

test('splitAddr', () => {
  assert.deepEqual(splitAddr('192.168.1.20:47811'), { host: '192.168.1.20', port: 47811 });
  assert.deepEqual(splitAddr('http://pc.local/'), { host: 'pc.local', port: 47811 });
  assert.equal(splitAddr('a b'), null);
  assert.equal(splitAddr('1.2.3.4:99999'), null);
});

test('orderAddrs quita repetidas y pone la primera delante', () => {
  assert.deepEqual(orderAddrs(['a:1', 'b:1', 'a:1', ' '], 'b:1'), ['b:1', 'a:1']);
  assert.deepEqual(orderAddrs(['a:1', 'b:1']), ['a:1', 'b:1']);
});

test('parseQrV2', () => {
  const ok = JSON.stringify({ v: 2, app: 'visor-capturas', id: 'X', name: 'PC', addrs: ['192.168.1.20:47811', '100.64.0.1'], port: 47811, ticket: 'T1' });
  assert.deepEqual(parseQrV2(ok), { id: 'X', name: 'PC', addrs: ['192.168.1.20:47811', '100.64.0.1:47811'], port: 47811, ticket: 'T1' });
  assert.equal(parseQrV2('visor://pair?h=1.2.3.4'), null);
  assert.equal(parseQrV2(JSON.stringify({ v: 1, addrs: ['a:1'], ticket: 't' })), null);
  assert.equal(parseQrV2(JSON.stringify({ v: 2, addrs: [], ticket: 't' })), null);
  assert.equal(parseQrV2(JSON.stringify({ v: 2, addrs: ['a:1'] })), null);
});

test('migrateServer lee el token guardado como code', () => {
  const tok = 'a'.repeat(32);
  assert.deepEqual(migrateServer({ host: 'h', port: 1, code: tok, name: 'PC', id: 'I' }), { host: 'h', port: 1, token: tok, name: 'PC', id: 'I', addrs: undefined });
  assert.equal(migrateServer({ host: 'h', port: 1, code: 'x', token: tok })?.token, tok);
  assert.equal(migrateServer({ host: 'h', port: 1, code: 'ABCD-EFGH' }), null);
  assert.equal(migrateServer({ host: 'h', port: 0, token: tok }), null);
  assert.equal(migrateServer(null), null);
  assert.deepEqual(migrateServer({ host: 'h', port: 1, token: tok, addrs: ['a:1', 'a:1', 3] })?.addrs, ['a:1']);
});

test('toStored repite el token en code', () => {
  assert.equal(toStored({ host: 'h', port: 1, token: 'T' }).code, 'T');
});

test('artemisUrl', () => {
  assert.equal(artemisUrl('1.2.3.4', { pin: '1234', passphrase: 'a b', port: 47989 }), 'art://1.2.3.4:47989?pin=1234&passphrase=a%20b');
  assert.equal(artemisUrl('h', { pin: '1', passphrase: 'p' }), 'art://h:47989?pin=1&passphrase=p');
});
