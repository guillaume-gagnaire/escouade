import { describe, expect, it } from 'vitest';
import { readPublicKey, readSignature, verifySigned } from '../../server/utils/minisign';
import { testKey } from '../fake/minisign.mjs';

// The vectors of the `minisign-verify` crate the app uses (src-tauri/Cargo.lock): what the store accepts, the app
// accepts too, and the other way round.
const PUB = 'RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3';
const LEGACY = `untrusted comment: signature from minisign secret key
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=
trusted comment: timestamp:1555779966\tfile:test
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==`;
const PREHASHED = `untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1556193335\tfile:test
y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==`;

describe('minisign', () => {
  const key = readPublicKey(`untrusted comment: minisign public key E7620F1842B4E81F\n${PUB}\n`)!;

  it('reads a public key and its id as minisign prints it', () => {
    expect(key.id).toBe('E7620F1842B4E81F');
    expect(readPublicKey(PUB)!.id).toBe('E7620F1842B4E81F');
    expect(readPublicKey('RWQ')).toBeNull();
    expect(readPublicKey('not a key')).toBeNull();
  });

  it('verifies the crate’s prehashed vector, and refuses another file', () => {
    const sig = readSignature(PREHASHED)!;
    expect(sig.trustedComment).toBe('timestamp:1556193335\tfile:test');
    expect(verifySigned('test', sig, [key])).toBe(key);
    expect(verifySigned('Test', sig, [key])).toBeNull();
  });

  it('takes the legacy form only when asked', () => {
    const sig = readSignature(LEGACY)!;
    expect(verifySigned('test', sig, [key])).toBeNull();
    expect(verifySigned('test', sig, [key], { legacy: true })).toBe(key);
  });

  it('refuses a changed trusted comment, an unknown key, a broken file', () => {
    const changed = PREHASHED.replace('file:test', 'file:else');
    expect(verifySigned('test', readSignature(changed)!, [key])).toBeNull();
    expect(verifySigned('test', readSignature(PREHASHED)!, [readPublicKey(testKey().pub)!])).toBeNull();
    expect(readSignature('untrusted comment: x\nRUQ=\ntrusted comment: y\nzz')).toBeNull();
    expect(readSignature(PREHASHED.split('\n').slice(0, 3).join('\n'))).toBeNull();
  });

  it('verifies what the test signer writes, and the store’s own keys come from it', () => {
    const k = testKey();
    const pub = readPublicKey(k.pub)!;
    expect(pub.id).toBe(k.id);
    const sig = readSignature(k.sign('{"a":1}', 'escouade-revocations-v1 3'))!;
    expect(verifySigned('{"a":1}', sig, [key, pub])).toBe(pub);
    expect(verifySigned('{"a":2}', sig, [pub])).toBeNull();
  });
});
