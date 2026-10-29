import { describe, expect, it } from 'vitest';
import { readPublicKey } from '../../server/utils/minisign';
import { checkRevocations, parseRevocations, revoked, REVOCATIONS_COMMENT } from '../../server/utils/revocation-list';
import { testKey } from '../fake/minisign.mjs';

const entry = {
  id: 'dev.x.ntfy',
  versions: ['1.2.0'],
  action: 'disable',
  reason: { fr: 'Envoie les messages ailleurs.', en: 'Sends the messages elsewhere.' },
  date: '2026-10-29',
};
const list = (serial: number, over: Record<string, unknown> = {}) =>
  JSON.stringify({ serial, issued_at: '2026-10-29T10:00:00Z', entries: [entry], ...over });

describe('the revocation list', () => {
  const k = testKey();
  const key = readPublicKey(k.pub)!;
  const signed = (json: string, serial: number) => k.sign(json, `${REVOCATIONS_COMMENT} ${serial} 2026-10-29T10:00:00Z`);

  it('reads its format, and says what is wrong', () => {
    expect(parseRevocations(list(3))).toMatchObject({ serial: 3, entries: [entry] });
    expect(parseRevocations('{')).toEqual({ error: 'not JSON' });
    expect(parseRevocations(list(0))).toMatchObject({ error: expect.stringContaining('serial') });
    expect(parseRevocations(list(1, { issued_at: 'hier' }))).toMatchObject({ error: expect.stringContaining('issued_at') });
    expect(parseRevocations(list(1, { entries: [{ ...entry, action: 'delete' }] }))).toEqual({
      error: 'entries[0].action: unverify or disable',
    });
    expect(parseRevocations(list(1, { entries: [{ ...entry, versions: [] }] }))).toMatchObject({
      error: expect.stringContaining('versions'),
    });
    expect(parseRevocations(list(1, { entries: [{ ...entry, reason: { fr: 'x' } }] }))).toEqual({
      error: 'entries[0].reason: fr and en',
    });
  });

  it('is taken only signed by a key of the store, with its own comment and serial, above the last', () => {
    const json = list(4);
    expect(checkRevocations(json, signed(json, 4), [key])).toMatchObject({ list: { serial: 4 }, key });
    expect(checkRevocations(json, signed(json, 4), [readPublicKey(testKey().pub)!])).toEqual({
      error: 'not signed by a key of the store',
    });
    expect(checkRevocations(json, signed(list(5), 5), [key])).toEqual({ error: 'not signed by a key of the store' });
    expect(checkRevocations(json, k.sign(json, `${REVOCATIONS_COMMENT} 3`), [key])).toMatchObject({
      error: expect.stringContaining('trusted comment'),
    });
    expect(checkRevocations(json, k.sign(json, 'escouade-verified-v1 dev.x.ntfy@1.2.0'), [key])).toMatchObject({
      error: expect.stringContaining('trusted comment'),
    });
    expect(checkRevocations(json, signed(json, 4), [key], 4)).toEqual({ error: 'serial 4 is not above 4' });
    expect(checkRevocations(json, 'nope', [key])).toEqual({ error: 'the .minisig does not read' });
  });

  it('is refused when it revokes the very key that signed it', () => {
    const json = list(6, { keys_revoked: [k.id] });
    expect(checkRevocations(json, signed(json, 6), [key])).toEqual({ error: 'signed by a key the list itself revokes' });
  });

  it('names the versions it disables or unverifies', () => {
    const l = parseRevocations(list(1, { entries: [entry, { ...entry, id: 'dev.x.other', versions: '*', action: 'unverify' }] }));
    if ('error' in l) throw new Error(l.error);
    expect(revoked(l, 'dev.x.ntfy', '1.2.0', 'disable')).toBeTruthy();
    expect(revoked(l, 'dev.x.ntfy', '1.3.0', 'disable')).toBeNull();
    expect(revoked(l, 'dev.x.ntfy', '1.2.0', 'unverify')).toBeNull();
    expect(revoked(l, 'dev.x.other', '9.9.9', 'unverify')).toBeTruthy();
    expect(revoked(null, 'dev.x.ntfy', '1.2.0', 'disable')).toBeNull();
  });
});
