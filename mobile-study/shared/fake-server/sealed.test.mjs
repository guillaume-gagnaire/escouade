// node --test : le vecteur de draft-irtf-cfrg-xchacha-03 §2.2.1 pour HChaCha20, et l'aller-retour.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { hchacha20, seal, open } from './sealed.mjs';

test('HChaCha20, vecteur du draft', () => {
  const key = Buffer.from('000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f', 'hex');
  const nonce = Buffer.from('000000090000004a0000000031415927', 'hex');
  assert.equal(hchacha20(key, nonce).toString('hex'), '82413b4227b27bfed30e42508a877d73a0f9e4d58a74a853c12ec41326d3ecdc');
});

test('seal puis open', () => {
  const k = Buffer.alloc(32, 7).toString('base64url');
  assert.equal(open(k, seal(k, 'é « bonjour »')), 'é « bonjour »');
  assert.throws(() => open(Buffer.alloc(32, 8).toString('base64url'), seal(k, 'x')));
});
