// The spaces given to the phone's page (MO-1): well formed, or dropped.
import { describe, expect, it } from 'vitest';
import { pairedSpaces, parseSpaces } from './paired';

const ok = { id: 'travail', name: ' Travail ', url: 'ws://127.0.0.1:7002/remote', token: 't' };

describe('the spaces given to the page', () => {
  it('keeps the well-formed ones, from a list or its JSON', () => {
    expect(parseSpaces([ok])).toEqual([{ id: 'travail', name: 'Travail', here: false, url: ok.url, token: 't' }]);
    expect(parseSpaces(JSON.stringify([{ ...ok, here: true }]))[0].here).toBe(true);
  });

  it('drops what is not a space: no token, an address that is not a WebSocket, a name twice, broken JSON', () => {
    expect(
      parseSpaces([
        { ...ok, token: '' },
        { ...ok, id: 'b', url: 'http://127.0.0.1/remote' },
        { ...ok, id: 'c', name: '  ' },
        ok,
        { ...ok, name: 'Encore' },
        null,
        'x',
      ]).map((s) => s.id),
    ).toEqual(['travail']);
    expect(parseSpaces('{not json')).toEqual([]);
    expect(parseSpaces({ id: 'x' })).toEqual([]);
  });

  it('takes the page’s spaces first, a build of development’s else, none on a phone', () => {
    expect(pairedSpaces({ __ESCOUADE_SPACES__: [ok] }, JSON.stringify([{ ...ok, id: 'dev' }]))[0].id).toBe('travail');
    expect(pairedSpaces({}, JSON.stringify([{ ...ok, id: 'dev' }]))[0].id).toBe('dev');
    expect(pairedSpaces({}, undefined)).toEqual([]);
  });
});
